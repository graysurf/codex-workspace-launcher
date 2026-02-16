use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::EXIT_RUNTIME;

use super::{
    PRIMARY_COMMAND_NAME, Workspace, command_exists, default_gpg_signing_key,
    list_workspaces_on_disk, map_workspace_internal_path, push_unique_path,
    resolve_codex_auth_file, resolve_codex_profile_auth_files, resolve_workspace, trimmed_nonempty,
    write_file_secure,
};

#[derive(Debug, Default, Clone)]
struct ParsedAuth {
    show_help: bool,
    provider: Option<String>,
    workspace: Option<String>,
    profile: Option<String>,
    host: Option<String>,
    key: Option<String>,
}

fn parse_auth_args(args: &[OsString]) -> Result<ParsedAuth, String> {
    let mut parsed = ParsedAuth::default();
    let mut idx = 0usize;

    while idx < args.len() {
        let current = args[idx].to_string_lossy();
        match current.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--container" | "--workspace" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(format!("missing value for {}", current));
                }
                parsed.workspace = Some(args[idx].to_string_lossy().into_owned());
            }
            "--profile" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --profile"));
                }
                parsed.profile = Some(args[idx].to_string_lossy().into_owned());
            }
            "--host" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --host"));
                }
                parsed.host = Some(args[idx].to_string_lossy().into_owned());
            }
            "--key" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --key"));
                }
                parsed.key = Some(args[idx].to_string_lossy().into_owned());
            }
            _ if current.starts_with("--container=") => {
                parsed.workspace = Some(current["--container=".len()..].to_string());
            }
            _ if current.starts_with("--workspace=") => {
                parsed.workspace = Some(current["--workspace=".len()..].to_string());
            }
            _ if current.starts_with("--profile=") => {
                parsed.profile = Some(current["--profile=".len()..].to_string());
            }
            _ if current.starts_with("--host=") => {
                parsed.host = Some(current["--host=".len()..].to_string());
            }
            _ if current.starts_with("--key=") => {
                parsed.key = Some(current["--key=".len()..].to_string());
            }
            "--" => {
                idx += 1;
                while idx < args.len() {
                    let text = args[idx].to_string_lossy().into_owned();
                    if parsed.provider.is_none() {
                        parsed.provider = Some(text);
                    } else if parsed.workspace.is_none() {
                        parsed.workspace = Some(text);
                    } else {
                        return Err(format!("unexpected arg: {}", args[idx].to_string_lossy()));
                    }
                    idx += 1;
                }
                break;
            }
            _ if current.starts_with('-') => {
                return Err(format!("unknown option for auth: {current}"));
            }
            _ => {
                let text = current.to_string();
                if parsed.provider.is_none() {
                    parsed.provider = Some(text);
                } else if parsed.workspace.is_none() {
                    parsed.workspace = Some(text);
                } else {
                    return Err(format!("unexpected arg: {current}"));
                }
            }
        }
        idx += 1;
    }

    Ok(parsed)
}

pub(super) fn run(args: &[OsString]) -> i32 {
    let parsed = match parse_auth_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_auth_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help || parsed.provider.is_none() {
        print_auth_usage();
        return 0;
    }

    let workspace = match resolve_workspace_for_auth(parsed.workspace.as_deref()) {
        Ok(workspace) => workspace,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let provider = parsed
        .provider
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    match provider.as_str() {
        "github" => run_auth_github(&workspace, parsed.host.as_deref()),
        "codex" => run_auth_codex(&workspace, parsed.profile.as_deref()),
        "gpg" => run_auth_gpg(&workspace, parsed.key.as_deref()),
        _ => {
            eprintln!("error: unknown auth provider: {provider}");
            eprintln!("hint: expected: codex|github|gpg");
            EXIT_RUNTIME
        }
    }
}

fn run_auth_github(workspace: &Workspace, host: Option<&str>) -> i32 {
    let gh_host = host
        .and_then(trimmed_nonempty)
        .or_else(|| std::env::var("GITHUB_HOST").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| String::from("github.com"));

    let auth_mode = std::env::var("AGENT_WORKSPACE_AUTH")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("CODEX_WORKSPACE_AUTH").ok())
        .unwrap_or_else(|| String::from("auto"));

    let env_token = std::env::var("GH_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("GITHUB_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });

    let keyring_token = gh_keyring_token(&gh_host);

    let (chosen_token, chosen_source) = match auth_mode.as_str() {
        "none" => (None, "none"),
        "env" => (env_token, "env"),
        "gh" | "keyring" => {
            if let Some(token) = keyring_token {
                (Some(token), "gh")
            } else {
                eprintln!(
                    "warn: AGENT_WORKSPACE_AUTH={auth_mode} but no gh keyring token found; falling back to GH_TOKEN/GITHUB_TOKEN"
                );
                (env_token, "env")
            }
        }
        "auto" | "" => {
            if let Some(token) = keyring_token {
                (Some(token), "gh")
            } else {
                (env_token, "env")
            }
        }
        _ => {
            eprintln!(
                "error: unknown AGENT_WORKSPACE_AUTH={auth_mode} (expected: auto|gh|env|none)"
            );
            return EXIT_RUNTIME;
        }
    };

    let token = if let Some(token) = chosen_token {
        token
    } else {
        if auth_mode == "none" {
            eprintln!("error: AGENT_WORKSPACE_AUTH=none; no token to apply");
        } else {
            eprintln!("error: no GitHub token found (gh keyring or GH_TOKEN/GITHUB_TOKEN)");
        }
        eprintln!("hint: run 'gh auth login' or export GH_TOKEN/GITHUB_TOKEN");
        return EXIT_RUNTIME;
    };

    let content = format!("host={gh_host}\ntoken={token}\n");
    let target = workspace.path.join("auth").join("github.env");
    if let Err(err) = write_file_secure(&target, content.as_bytes()) {
        eprintln!(
            "error: failed to write GitHub auth file {}: {err}",
            target.display()
        );
        return EXIT_RUNTIME;
    }

    println!(
        "auth: github -> {} ({gh_host}; source={chosen_source})",
        workspace.name
    );
    0
}

fn run_auth_codex(workspace: &Workspace, profile_arg: Option<&str>) -> i32 {
    let profile = profile_arg
        .and_then(trimmed_nonempty)
        .or_else(|| {
            std::env::var("AGENT_WORKSPACE_CODEX_PROFILE")
                .ok()
                .and_then(|value| trimmed_nonempty(&value))
        })
        .or_else(|| {
            std::env::var("CODEX_WORKSPACE_CODEX_PROFILE")
                .ok()
                .and_then(|value| trimmed_nonempty(&value))
        });

    let mut candidate_files: Vec<PathBuf> = Vec::new();
    if let Some(profile) = profile.as_deref() {
        if profile.contains('/')
            || profile.contains("..")
            || profile.chars().any(char::is_whitespace)
        {
            eprintln!("error: invalid codex profile name: {profile}");
            return EXIT_RUNTIME;
        }

        for candidate in resolve_codex_profile_auth_files(profile) {
            push_unique_path(&mut candidate_files, PathBuf::from(candidate));
        }
    }
    push_unique_path(
        &mut candidate_files,
        PathBuf::from(resolve_codex_auth_file()),
    );

    for candidate in candidate_files {
        if !candidate.is_file() {
            continue;
        }

        let auth_data = match fs::read(&candidate) {
            Ok(data) => data,
            Err(err) => {
                eprintln!(
                    "warn: failed to read codex auth candidate {}: {err}",
                    candidate.display()
                );
                continue;
            }
        };

        if let Err(err) = sync_codex_auth_into_workspace(workspace, &auth_data) {
            eprintln!(
                "warn: failed to sync codex auth from {}: {err}",
                candidate.display()
            );
            continue;
        }

        if let Some(profile) = profile.as_deref() {
            println!(
                "auth: codex -> {} (profile={profile}; source={})",
                workspace.name,
                candidate.display()
            );
        } else {
            println!(
                "auth: codex -> {} (source={})",
                workspace.name,
                candidate.display()
            );
        }
        return 0;
    }

    eprintln!("error: unable to resolve codex auth file");
    eprintln!("hint: set CODEX_AUTH_FILE or pass --profile <name>");
    EXIT_RUNTIME
}

fn sync_codex_auth_into_workspace(workspace: &Workspace, auth_data: &[u8]) -> Result<(), String> {
    let targets = codex_auth_targets(workspace);
    for target in targets {
        write_file_secure(&target, auth_data)?;
    }
    Ok(())
}

fn run_auth_gpg(workspace: &Workspace, key_arg: Option<&str>) -> i32 {
    let key = key_arg
        .and_then(trimmed_nonempty)
        .or_else(default_gpg_signing_key);

    let key = if let Some(key) = key {
        key
    } else {
        eprintln!("error: missing gpg signing key");
        eprintln!("hint: pass --key <fingerprint> or set AGENT_WORKSPACE_GPG_KEY");
        eprintln!("hint: or set: git config --global user.signingkey <keyid>");
        return EXIT_RUNTIME;
    };

    if command_exists("gpg") {
        let status = Command::new("gpg")
            .args(["--batch", "--list-secret-keys", &key])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match status {
            Ok(result) if result.success() => {}
            Ok(_) => {
                eprintln!("error: gpg key not found in host keyring: {key}");
                return EXIT_RUNTIME;
            }
            Err(err) => {
                eprintln!("error: failed to run gpg for key lookup: {err}");
                return EXIT_RUNTIME;
            }
        }
    } else {
        eprintln!("warn: gpg not found in PATH; writing key id only");
    }

    let target = workspace.path.join("auth").join("gpg-key.txt");
    if let Err(err) = write_file_secure(&target, format!("{key}\n").as_bytes()) {
        eprintln!(
            "error: failed to write gpg auth file {}: {err}",
            target.display()
        );
        return EXIT_RUNTIME;
    }

    println!("auth: gpg -> {} (key={key})", workspace.name);
    0
}

pub(super) fn codex_auth_targets(workspace: &Workspace) -> Vec<PathBuf> {
    let mut targets = vec![workspace.path.join(".codex").join("auth.json")];

    if let Ok(value) = std::env::var("CODEX_AUTH_FILE")
        && let Some(cleaned) = trimmed_nonempty(&value)
    {
        let mapped = map_workspace_internal_path(workspace, &cleaned);
        push_unique_path(&mut targets, mapped);
    }

    targets
}

fn gh_keyring_token(host: &str) -> Option<String> {
    if !command_exists("gh") {
        return None;
    }

    let output = Command::new("gh")
        .args(["auth", "token", "-h", host])
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    trimmed_nonempty(String::from_utf8_lossy(&output.stdout).as_ref())
}

pub(super) fn resolve_workspace_for_auth(name: Option<&str>) -> Result<Workspace, String> {
    if let Some(name) = name.and_then(trimmed_nonempty) {
        return match resolve_workspace(&name)? {
            Some(workspace) => Ok(workspace),
            None => Err(format!("workspace not found: {name}")),
        };
    }

    let workspaces = list_workspaces_on_disk()?;
    match workspaces.as_slice() {
        [] => Err(String::from("no workspaces found")),
        [single] => Ok(single.clone()),
        _ => Err(format!(
            "multiple workspaces found; specify one: {}",
            workspaces
                .iter()
                .map(|workspace| workspace.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn print_auth_usage() {
    eprintln!("usage:");
    eprintln!("  {PRIMARY_COMMAND_NAME} auth codex [--profile <name>] [--container <workspace>]");
    eprintln!("  {PRIMARY_COMMAND_NAME} auth github [--host <host>] [--container <workspace>]");
    eprintln!(
        "  {PRIMARY_COMMAND_NAME} auth gpg [--key <keyid|fingerprint>] [--container <workspace>]"
    );
}
