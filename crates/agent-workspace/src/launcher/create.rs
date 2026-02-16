use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::EXIT_RUNTIME;

use super::{
    PRIMARY_COMMAND_NAME, RepoSpec, WORKSPACE_META_FILE, command_exists, ensure_workspace_root,
    generate_workspace_name, normalize_workspace_name_for_create, parse_repo_spec, slugify_name,
    trimmed_nonempty, workspace_repo_destination,
};

#[derive(Debug, Default, Clone)]
pub(super) struct ParsedCreate {
    show_help: bool,
    no_extras: bool,
    no_work_repos: bool,
    private_repo: Option<String>,
    workspace_name: Option<String>,
    primary_repo: Option<String>,
    extra_repos: Vec<String>,
    ignored_options: Vec<String>,
}

pub(super) fn parse_create_args(args: &[OsString]) -> Result<ParsedCreate, String> {
    let mut parsed = ParsedCreate::default();
    let mut idx = 0usize;
    let mut positional_only = false;

    while idx < args.len() {
        let current = args[idx].clone();
        let text = current.to_string_lossy().into_owned();

        if !positional_only {
            match text.as_str() {
                "-h" | "--help" => {
                    parsed.show_help = true;
                    idx += 1;
                    continue;
                }
                "--no-work-repos" => {
                    parsed.no_work_repos = true;
                    idx += 1;
                    continue;
                }
                "--no-extras" => {
                    parsed.no_extras = true;
                    idx += 1;
                    continue;
                }
                "--private-repo" => {
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --private-repo"));
                    }
                    parsed.private_repo = trimmed_nonempty(args[idx].to_string_lossy().as_ref());
                    idx += 1;
                    continue;
                }
                "--name" => {
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --name"));
                    }
                    let value = args[idx].to_string_lossy().into_owned();
                    let normalized = normalize_workspace_name_for_create(&value);
                    parsed.workspace_name = trimmed_nonempty(&normalized);
                    idx += 1;
                    continue;
                }
                "--" => {
                    positional_only = true;
                    idx += 1;
                    continue;
                }
                _ if text.starts_with("--private-repo=") => {
                    parsed.private_repo = trimmed_nonempty(text["--private-repo=".len()..].trim());
                    idx += 1;
                    continue;
                }
                _ if text.starts_with("--name=") => {
                    let value = text["--name=".len()..].trim();
                    let normalized = normalize_workspace_name_for_create(value);
                    parsed.workspace_name = trimmed_nonempty(&normalized);
                    idx += 1;
                    continue;
                }
                _ if text.starts_with('-') => {
                    parsed.ignored_options.push(text);
                    idx += 1;
                    continue;
                }
                _ => {}
            }
        }

        if parsed.primary_repo.is_none() {
            parsed.primary_repo = Some(text);
        } else {
            parsed.extra_repos.push(text);
        }
        idx += 1;
    }

    if parsed.no_work_repos && (parsed.primary_repo.is_some() || !parsed.extra_repos.is_empty()) {
        return Err(String::from("--no-work-repos does not accept repo args"));
    }

    Ok(parsed)
}

pub(super) fn run(args: &[OsString]) -> i32 {
    let parsed = match parse_create_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_create_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_create_usage();
        return 0;
    }

    if !parsed.ignored_options.is_empty() {
        eprintln!(
            "warn: ignoring unsupported create options in host-native mode: {}",
            parsed.ignored_options.join(" ")
        );
    }

    let default_host = std::env::var("GITHUB_HOST").unwrap_or_else(|_| String::from("github.com"));

    let primary_spec = if let Some(primary_repo) = parsed.primary_repo.as_deref() {
        match parse_repo_spec(primary_repo, &default_host) {
            Some(spec) => Some(spec),
            None => {
                eprintln!(
                    "error: invalid primary repo (expected OWNER/REPO or URL): {primary_repo}"
                );
                return EXIT_RUNTIME;
            }
        }
    } else {
        None
    };

    let mut workspace_name = parsed
        .workspace_name
        .clone()
        .or_else(|| {
            primary_spec
                .as_ref()
                .map(|spec| format!("ws-{}", slugify_name(&spec.repo)))
        })
        .unwrap_or_else(generate_workspace_name);
    workspace_name = normalize_workspace_name_for_create(&workspace_name);
    if workspace_name.is_empty() {
        workspace_name = generate_workspace_name();
    }

    let root = match ensure_workspace_root() {
        Ok(root) => root,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let workspace_path = root.join(&workspace_name);
    if workspace_path.exists() {
        eprintln!("error: workspace already exists: {workspace_name}");
        return EXIT_RUNTIME;
    }

    if let Err(err) =
        create_workspace_skeleton(&workspace_path, &workspace_name, primary_spec.as_ref())
    {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    if !parsed.no_work_repos
        && let Some(spec) = primary_spec.as_ref()
    {
        let destination = workspace_repo_destination(&workspace_path.join("work"), spec);
        if let Err(err) = clone_repo_into(spec, &destination) {
            eprintln!(
                "error: failed to clone primary repo {}: {err}",
                spec.owner_repo
            );
            return EXIT_RUNTIME;
        }
    }

    if !parsed.no_extras {
        if let Some(private_repo_raw) = parsed.private_repo.as_deref() {
            if let Some(spec) = parse_repo_spec(private_repo_raw, &default_host) {
                let destination =
                    workspace_repo_destination(&workspace_path.join("private"), &spec);
                if let Err(err) = clone_repo_into(&spec, &destination) {
                    eprintln!(
                        "warn: failed to clone private repo {}: {err}",
                        spec.owner_repo
                    );
                }
            } else {
                eprintln!(
                    "warn: invalid private repo (expected OWNER/REPO or URL): {private_repo_raw}"
                );
            }
        }

        for extra_repo_raw in &parsed.extra_repos {
            if let Some(spec) = parse_repo_spec(extra_repo_raw, &default_host) {
                let destination = workspace_repo_destination(&workspace_path.join("work"), &spec);
                if let Err(err) = clone_repo_into(&spec, &destination) {
                    eprintln!(
                        "warn: failed to clone extra repo {}: {err}",
                        spec.owner_repo
                    );
                }
            } else {
                eprintln!("warn: invalid repo (expected OWNER/REPO or URL): {extra_repo_raw}");
            }
        }
    }

    println!("workspace: {workspace_name}");
    println!("path: {}", workspace_path.display());
    0
}

fn create_workspace_skeleton(
    workspace_path: &Path,
    workspace_name: &str,
    primary_repo: Option<&RepoSpec>,
) -> Result<(), String> {
    fs::create_dir_all(workspace_path).map_err(|err| {
        format!(
            "failed to create workspace directory {}: {err}",
            workspace_path.display()
        )
    })?;

    for subdir in ["work", "opt", "private", "auth", ".codex"] {
        fs::create_dir_all(workspace_path.join(subdir)).map_err(|err| {
            format!(
                "failed to create workspace subdir {}: {err}",
                workspace_path.join(subdir).display()
            )
        })?;
    }

    let created_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    let metadata = format!(
        "name={workspace_name}\ncreated_unix={created_unix}\nprimary_repo={}\n",
        primary_repo
            .map(|repo| repo.owner_repo.as_str())
            .unwrap_or("none")
    );
    fs::write(workspace_path.join(WORKSPACE_META_FILE), metadata).map_err(|err| {
        format!(
            "failed to write workspace metadata {}: {err}",
            workspace_path.join(WORKSPACE_META_FILE).display()
        )
    })?;

    Ok(())
}

fn clone_repo_into(repo: &RepoSpec, destination: &Path) -> Result<(), String> {
    if destination.join(".git").is_dir() {
        return Ok(());
    }

    if destination.exists() {
        return Err(format!(
            "destination exists but is not a git repo: {}",
            destination.display()
        ));
    }

    if !command_exists("git") {
        return Err(String::from("git not found in PATH"));
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create clone parent {}: {err}", parent.display()))?;
    }

    let status = Command::new("git")
        .arg("clone")
        .arg("--progress")
        .arg(&repo.clone_url)
        .arg(destination)
        .status()
        .map_err(|err| format!("failed to run git clone for {}: {err}", repo.owner_repo))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "git clone failed for {} (exit {})",
            repo.owner_repo,
            status.code().unwrap_or(EXIT_RUNTIME)
        ))
    }
}

fn print_create_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} create [--name <workspace>] [--private-repo OWNER/REPO] [--no-work-repos] [--no-extras] [repo] [extra_repos...]"
    );
}
