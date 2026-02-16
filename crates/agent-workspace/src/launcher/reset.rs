use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::EXIT_RUNTIME;

use super::{PRIMARY_COMMAND_NAME, Workspace, confirm_or_abort, resolve_workspace};

const DEFAULT_REF: &str = "origin/main";

const RESET_REPO_SCRIPT: &str = r#"
set -euo pipefail

repo_dir="${1:?missing repo_dir}"
ref="${2:-origin/main}"

if [[ ! -e "$repo_dir/.git" ]]; then
  echo "error: not a git repo: $repo_dir" >&2
  exit 1
fi

cd "$repo_dir"

remote="${ref%%/*}"
branch="${ref#*/}"
if [[ "$remote" == "$ref" || -z "$remote" || -z "$branch" ]]; then
  echo "error: invalid ref (expected remote/branch): $ref" >&2
  exit 2
fi

git fetch --prune -- "$remote" >/dev/null 2>&1 || git fetch --prune -- "$remote"

resolved="$remote/$branch"
if ! git show-ref --verify --quiet "refs/remotes/$resolved"; then
  default_ref="$(git symbolic-ref -q --short "refs/remotes/$remote/HEAD" 2>/dev/null || true)"
  if [[ -n "$default_ref" ]] && git show-ref --verify --quiet "refs/remotes/$default_ref"; then
    echo "warn: $resolved not found; using $default_ref (from $remote/HEAD)" >&2
    resolved="$default_ref"
  elif git show-ref --verify --quiet "refs/remotes/$remote/master"; then
    echo "warn: $resolved not found; using $remote/master" >&2
    resolved="$remote/master"
  else
    echo "error: remote branch not found: $resolved" >&2
    exit 1
  fi
fi

target_branch="${resolved#*/}"
echo "+ reset $repo_dir -> $resolved"

if git show-ref --verify --quiet "refs/heads/$target_branch"; then
  git checkout --force "$target_branch" >/dev/null 2>&1 || {
    git clean -fd >/dev/null 2>&1 || true
    git checkout --force "$target_branch"
  }
else
  git checkout --force -B "$target_branch" "$resolved" >/dev/null 2>&1 || {
    git clean -fd >/dev/null 2>&1 || true
    git checkout --force -B "$target_branch" "$resolved"
  }
fi

if command -v git-reset-remote >/dev/null 2>&1; then
  git-reset-remote --ref "$resolved" --no-fetch --clean --yes
else
  git reset --hard "$resolved"
  git clean -fd
  echo "✅ Done. '$target_branch' now matches '$resolved'."
fi
"#;

const LIST_GIT_REPOS_SCRIPT: &str = r#"
set -euo pipefail

root="${1:?missing root}"
depth="${2:?missing depth}"

if ! [[ "$depth" =~ ^[0-9]+$ ]] || [[ "$depth" -le 0 ]]; then
  echo "error: --depth must be a positive integer (got: $depth)" >&2
  exit 2
fi

if [[ ! -d "$root" ]]; then
  exit 0
fi

git_depth=$((depth + 1))
find -L "$root" -maxdepth "$git_depth" -mindepth 2 \( -type d -o -type f \) -name .git -print0 2>/dev/null \
  | while IFS= read -r -d '' git_entry; do
      printf '%s\n' "${git_entry%/.git}"
    done \
  | sort -u
"#;

pub(super) fn run(args: &[OsString]) -> i32 {
    if args.is_empty() {
        print_reset_usage();
        return 0;
    }

    let subcommand = args[0].to_string_lossy();
    if matches!(subcommand.as_ref(), "-h" | "--help") {
        print_reset_usage();
        return 0;
    }

    match subcommand.as_ref() {
        "repo" => run_reset_repo(&args[1..]),
        "work-repos" => run_reset_work_repos(&args[1..]),
        "opt-repos" => run_reset_opt_repos(&args[1..]),
        "private-repo" => run_reset_private_repo(&args[1..]),
        _ => {
            eprintln!("error: unknown reset subcommand: {subcommand}");
            eprintln!("hint: {PRIMARY_COMMAND_NAME} reset --help");
            EXIT_RUNTIME
        }
    }
}

fn run_reset_repo(args: &[OsString]) -> i32 {
    let parsed = match parse_reset_repo_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_reset_repo_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_reset_repo_usage();
        return 0;
    }

    let workspace_name = if let Some(workspace) = parsed.workspace {
        workspace
    } else {
        eprintln!("error: missing workspace");
        print_reset_repo_usage();
        return EXIT_RUNTIME;
    };

    let repo_dir = if let Some(repo_dir) = parsed.repo_dir {
        repo_dir
    } else {
        eprintln!("error: missing repo_dir");
        print_reset_repo_usage();
        return EXIT_RUNTIME;
    };

    let workspace = match resolve_workspace(&workspace_name) {
        Ok(Some(workspace)) => workspace,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let target_repo = map_workspace_repo_path(&workspace, &repo_dir);
    if !target_repo.join(".git").exists() {
        eprintln!("error: not a git repo: {}", target_repo.display());
        return EXIT_RUNTIME;
    }

    if !parsed.yes {
        println!("This will reset a repo in workspace: {}", workspace.name);
        println!("  - {}", target_repo.display());
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    match reset_repo_on_host(&target_repo, &parsed.refspec) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("error: {err}");
            EXIT_RUNTIME
        }
    }
}

fn run_reset_work_repos(args: &[OsString]) -> i32 {
    let parsed = match parse_reset_work_repos_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_reset_work_repos_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_reset_work_repos_usage();
        return 0;
    }

    let workspace_name = if let Some(workspace) = parsed.workspace {
        workspace
    } else {
        eprintln!("error: missing workspace");
        print_reset_work_repos_usage();
        return EXIT_RUNTIME;
    };

    let workspace = match resolve_workspace(&workspace_name) {
        Ok(Some(workspace)) => workspace,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let root = map_workspace_repo_path(&workspace, &parsed.root);
    let repos = match list_git_repos_on_host(&root, parsed.depth) {
        Ok(repos) => repos,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if repos.is_empty() {
        eprintln!(
            "warn: no git repos found under {} (depth={}) in {}",
            root.display(),
            parsed.depth,
            workspace.name
        );
        return 0;
    }

    if !parsed.yes {
        println!(
            "This will reset {} repo(s) inside workspace: {}",
            repos.len(),
            workspace.name
        );
        for repo in &repos {
            println!("  - {}", repo.display());
        }
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    let mut failed = 0usize;
    for repo in repos {
        if let Err(err) = reset_repo_on_host(&repo, &parsed.refspec) {
            eprintln!("error: {err}");
            failed += 1;
        }
    }
    if failed > 0 {
        eprintln!("error: failed to reset {failed} repo(s)");
        return EXIT_RUNTIME;
    }

    0
}

fn run_reset_opt_repos(args: &[OsString]) -> i32 {
    let parsed = match parse_reset_opt_repos_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_reset_opt_repos_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_reset_opt_repos_usage();
        return 0;
    }

    let workspace_name = if let Some(workspace) = parsed.workspace {
        workspace
    } else {
        eprintln!("error: missing workspace");
        print_reset_opt_repos_usage();
        return EXIT_RUNTIME;
    };

    let workspace = match resolve_workspace(&workspace_name) {
        Ok(Some(workspace)) => workspace,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let opt_root = workspace.path.join("opt");
    let repos = match list_git_repos_on_host(&opt_root, 4) {
        Ok(repos) => repos,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if repos.is_empty() {
        eprintln!("warn: no git repos found under {}", opt_root.display());
        return 0;
    }

    if !parsed.yes {
        println!(
            "This will reset /opt-style repos in workspace: {}",
            workspace.name
        );
        for repo in &repos {
            println!("  - {}", repo.display());
        }
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    for repo in repos {
        if let Err(err) = reset_repo_on_host(&repo, DEFAULT_REF) {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    }

    0
}

fn run_reset_private_repo(args: &[OsString]) -> i32 {
    let parsed = match parse_reset_private_repo_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_reset_private_repo_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_reset_private_repo_usage();
        return 0;
    }

    let workspace_name = if let Some(workspace) = parsed.workspace {
        workspace
    } else {
        eprintln!("error: missing workspace");
        print_reset_private_repo_usage();
        return EXIT_RUNTIME;
    };

    let workspace = match resolve_workspace(&workspace_name) {
        Ok(Some(workspace)) => workspace,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let private_repo = match detect_private_repo_dir(&workspace) {
        Ok(Some(path)) => path,
        Ok(None) => {
            eprintln!(
                "warn: no private git repo found in workspace: {}",
                workspace.name
            );
            eprintln!(
                "hint: seed it with: AGENT_WORKSPACE_PRIVATE_REPO=OWNER/REPO {PRIMARY_COMMAND_NAME} create ..."
            );
            return 0;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if !parsed.yes {
        println!(
            "This will reset private repo in workspace: {}",
            workspace.name
        );
        println!("  - {}", private_repo.display());
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    match reset_repo_on_host(&private_repo, &parsed.refspec) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("error: {err}");
            EXIT_RUNTIME
        }
    }
}

fn detect_private_repo_dir(workspace: &Workspace) -> Result<Option<PathBuf>, String> {
    let private_root = workspace.path.join("private");
    if !private_root.exists() {
        return Ok(None);
    }

    let repos = list_git_repos_on_host(&private_root, 4)?;
    Ok(repos.into_iter().next())
}

fn reset_repo_on_host(repo_dir: &Path, refspec: &str) -> Result<(), String> {
    let status = Command::new("bash")
        .args([
            "-c",
            RESET_REPO_SCRIPT,
            "--",
            repo_dir.to_string_lossy().as_ref(),
            refspec,
        ])
        .status()
        .map_err(|err| format!("failed to reset repo {}: {err}", repo_dir.display()))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to reset repo {} (exit {})",
            repo_dir.display(),
            status.code().unwrap_or(EXIT_RUNTIME)
        ))
    }
}

fn list_git_repos_on_host(root: &Path, depth: u32) -> Result<Vec<PathBuf>, String> {
    if depth == 0 {
        return Err(String::from("--depth must be a positive integer"));
    }

    let output = Command::new("bash")
        .args([
            "-c",
            LIST_GIT_REPOS_SCRIPT,
            "--",
            root.to_string_lossy().as_ref(),
            &depth.to_string(),
        ])
        .output()
        .map_err(|err| format!("failed to list git repos under {}: {err}", root.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "failed to list git repos under {} (exit {}): {stderr}",
            root.display(),
            output.status.code().unwrap_or(EXIT_RUNTIME)
        ));
    }

    let mut repos: Vec<PathBuf> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect();
    repos.sort();
    repos.dedup();
    Ok(repos)
}

fn map_workspace_repo_path(workspace: &Workspace, raw: &str) -> PathBuf {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return workspace.path.clone();
    }

    if cleaned == "/work" {
        return workspace.path.join("work");
    }
    if let Some(rest) = cleaned.strip_prefix("/work/") {
        return workspace.path.join("work").join(rest);
    }

    if cleaned == "/opt" {
        return workspace.path.join("opt");
    }
    if let Some(rest) = cleaned.strip_prefix("/opt/") {
        return workspace.path.join("opt").join(rest);
    }

    if cleaned == "~/.private" {
        return workspace.path.join("private");
    }
    if let Some(rest) = cleaned.strip_prefix("~/.private/") {
        return workspace.path.join("private").join(rest);
    }

    let path = Path::new(cleaned);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.path.join(path)
    }
}

#[derive(Debug, Clone)]
struct ParsedResetRepo {
    show_help: bool,
    workspace: Option<String>,
    repo_dir: Option<String>,
    refspec: String,
    yes: bool,
}

impl Default for ParsedResetRepo {
    fn default() -> Self {
        Self {
            show_help: false,
            workspace: None,
            repo_dir: None,
            refspec: String::from(DEFAULT_REF),
            yes: false,
        }
    }
}

fn parse_reset_repo_args(args: &[OsString]) -> Result<ParsedResetRepo, String> {
    let mut parsed = ParsedResetRepo::default();
    let mut idx = 0usize;

    while idx < args.len() {
        let text = args[idx].to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--ref" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --ref"));
                }
                parsed.refspec = args[idx].to_string_lossy().into_owned();
            }
            "-y" | "--yes" => parsed.yes = true,
            _ if text.starts_with("--ref=") => {
                parsed.refspec = text["--ref=".len()..].to_string();
            }
            _ if text.starts_with('-') => return Err(format!("unknown arg: {text}")),
            _ => {
                if parsed.workspace.is_none() {
                    parsed.workspace = Some(text.to_string());
                } else if parsed.repo_dir.is_none() {
                    parsed.repo_dir = Some(text.to_string());
                } else {
                    return Err(format!("unexpected arg: {text}"));
                }
            }
        }
        idx += 1;
    }

    Ok(parsed)
}

#[derive(Debug, Clone)]
pub(super) struct ParsedResetWorkRepos {
    show_help: bool,
    workspace: Option<String>,
    root: String,
    depth: u32,
    refspec: String,
    yes: bool,
}

impl Default for ParsedResetWorkRepos {
    fn default() -> Self {
        Self {
            show_help: false,
            workspace: None,
            root: String::from("/work"),
            depth: 3,
            refspec: String::from(DEFAULT_REF),
            yes: false,
        }
    }
}

pub(super) fn parse_reset_work_repos_args(
    args: &[OsString],
) -> Result<ParsedResetWorkRepos, String> {
    let mut parsed = ParsedResetWorkRepos::default();
    let mut idx = 0usize;

    while idx < args.len() {
        let text = args[idx].to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--root" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --root"));
                }
                parsed.root = args[idx].to_string_lossy().into_owned();
            }
            "--depth" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --depth"));
                }
                parsed.depth = args[idx]
                    .to_string_lossy()
                    .parse::<u32>()
                    .map_err(|_| String::from("--depth must be a positive integer"))?;
            }
            "--ref" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --ref"));
                }
                parsed.refspec = args[idx].to_string_lossy().into_owned();
            }
            "-y" | "--yes" => parsed.yes = true,
            _ if text.starts_with("--root=") => {
                parsed.root = text["--root=".len()..].to_string();
            }
            _ if text.starts_with("--depth=") => {
                parsed.depth = text["--depth=".len()..]
                    .parse::<u32>()
                    .map_err(|_| String::from("--depth must be a positive integer"))?;
            }
            _ if text.starts_with("--ref=") => {
                parsed.refspec = text["--ref=".len()..].to_string();
            }
            _ if text.starts_with('-') => return Err(format!("unknown arg: {text}")),
            _ => {
                if parsed.workspace.is_none() {
                    parsed.workspace = Some(text.to_string());
                } else {
                    return Err(format!("unexpected arg: {text}"));
                }
            }
        }

        idx += 1;
    }

    if parsed.depth == 0 {
        return Err(String::from("--depth must be a positive integer"));
    }

    Ok(parsed)
}

#[derive(Debug, Default, Clone)]
struct ParsedResetSimple {
    show_help: bool,
    workspace: Option<String>,
    yes: bool,
}

fn parse_reset_opt_repos_args(args: &[OsString]) -> Result<ParsedResetSimple, String> {
    let mut parsed = ParsedResetSimple::default();

    for arg in args {
        let text = arg.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "-y" | "--yes" => parsed.yes = true,
            _ if text.starts_with('-') => return Err(format!("unknown arg: {text}")),
            _ => {
                if parsed.workspace.is_none() {
                    parsed.workspace = Some(text.to_string());
                } else {
                    return Err(format!("unexpected arg: {text}"));
                }
            }
        }
    }

    Ok(parsed)
}

#[derive(Debug, Clone)]
struct ParsedResetPrivate {
    show_help: bool,
    workspace: Option<String>,
    refspec: String,
    yes: bool,
}

impl Default for ParsedResetPrivate {
    fn default() -> Self {
        Self {
            show_help: false,
            workspace: None,
            refspec: String::from(DEFAULT_REF),
            yes: false,
        }
    }
}

fn parse_reset_private_repo_args(args: &[OsString]) -> Result<ParsedResetPrivate, String> {
    let mut parsed = ParsedResetPrivate::default();
    let mut idx = 0usize;

    while idx < args.len() {
        let text = args[idx].to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--ref" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --ref"));
                }
                parsed.refspec = args[idx].to_string_lossy().into_owned();
            }
            "-y" | "--yes" => parsed.yes = true,
            _ if text.starts_with("--ref=") => {
                parsed.refspec = text["--ref=".len()..].to_string();
            }
            _ if text.starts_with('-') => return Err(format!("unknown arg: {text}")),
            _ => {
                if parsed.workspace.is_none() {
                    parsed.workspace = Some(text.to_string());
                } else {
                    return Err(format!("unexpected arg: {text}"));
                }
            }
        }
        idx += 1;
    }

    Ok(parsed)
}

fn print_reset_usage() {
    eprintln!("usage:");
    eprintln!(
        "  {PRIMARY_COMMAND_NAME} reset repo <workspace> <repo_dir> [--ref <remote/branch>] [--yes]"
    );
    eprintln!(
        "  {PRIMARY_COMMAND_NAME} reset work-repos <workspace> [--root <dir>] [--depth <N>] [--ref <remote/branch>] [--yes]"
    );
    eprintln!("  {PRIMARY_COMMAND_NAME} reset opt-repos <workspace> [--yes]");
    eprintln!(
        "  {PRIMARY_COMMAND_NAME} reset private-repo <workspace> [--ref <remote/branch>] [--yes]"
    );
}

fn print_reset_repo_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} reset repo <workspace> <repo_dir> [--ref <remote/branch>] [--yes]"
    );
}

fn print_reset_work_repos_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} reset work-repos <workspace> [--root <dir>] [--depth <N>] [--ref <remote/branch>] [--yes]"
    );
}

fn print_reset_opt_repos_usage() {
    eprintln!("usage: {PRIMARY_COMMAND_NAME} reset opt-repos <workspace> [--yes]");
}

fn print_reset_private_repo_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} reset private-repo <workspace> [--ref <remote/branch>] [--yes]"
    );
}
