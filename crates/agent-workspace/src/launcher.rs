use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::EXIT_RUNTIME;

pub const DEFAULT_LAUNCHER_PATH: &str = "/opt/agent-kit/docker/agent-env/bin/agent-workspace";
pub const LEGACY_LAUNCHER_PATH: &str = "/opt/agent-kit/docker/codex-env/bin/codex-workspace";
const LAUNCHER_ENV: &str = "AGENT_WORKSPACE_LAUNCHER";

pub fn dispatch(subcommand: &str, args: &[OsString]) -> i32 {
    match subcommand {
        "create" => {
            let translated = translate_create_args(args);
            forward("create", &translated)
        }
        "exec" => run_exec(args),
        "rm" => run_rm(args),
        _ => forward(subcommand, args),
    }
}

pub fn forward(subcommand: &str, args: &[OsString]) -> i32 {
    let launcher = resolve_launcher_path();
    forward_with_launcher_and_env(&launcher, subcommand, args, &[])
}

fn translate_create_args(args: &[OsString]) -> Vec<OsString> {
    args.iter()
        .map(|arg| {
            if arg.to_string_lossy() == "--no-work-repos" {
                OsString::from("--no-clone")
            } else {
                arg.clone()
            }
        })
        .collect()
}

#[derive(Debug, Default, Clone)]
struct ParsedExec {
    show_help: bool,
    user: Option<OsString>,
    workspace: Option<OsString>,
    command: Vec<OsString>,
}

fn parse_exec_args(args: &[OsString]) -> Result<ParsedExec, String> {
    let mut parsed = ParsedExec::default();
    let mut idx = 0usize;

    while idx < args.len() {
        if parsed.workspace.is_some() {
            parsed.command.extend(args[idx..].iter().cloned());
            break;
        }

        let current = &args[idx];
        let text = current.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => {
                parsed.show_help = true;
                return Ok(parsed);
            }
            "--root" => {
                if parsed.user.is_none() {
                    parsed.user = Some(OsString::from("0"));
                }
            }
            "--user" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --user"));
                }
                parsed.user = Some(args[idx].clone());
            }
            _ if text.starts_with("--user=") => {
                parsed.user = Some(OsString::from(&text["--user=".len()..]));
            }
            _ if text.starts_with('-') => {
                return Err(format!("unknown option for exec: {text}"));
            }
            _ => {
                parsed.workspace = Some(current.clone());
            }
        }
        idx += 1;
    }

    if parsed.workspace.is_none() {
        return Err(String::from("missing workspace name"));
    }
    Ok(parsed)
}

fn run_exec(args: &[OsString]) -> i32 {
    let parsed = match parse_exec_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_exec_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_exec_usage();
        return 0;
    }

    let workspace = parsed.workspace.expect("workspace checked");
    if parsed.command.is_empty() && parsed.user.is_none() {
        return forward("shell", &[workspace]);
    }

    let workspace = resolve_workspace_container_name(&workspace);

    let mut cmd = Command::new("docker");
    cmd.arg("exec");

    if parsed.command.is_empty() {
        let stdin_tty = std::io::stdin().is_terminal();
        let stdout_tty = std::io::stdout().is_terminal();
        if stdin_tty && stdout_tty {
            cmd.arg("-it");
        } else if stdin_tty {
            cmd.arg("-i");
        }
        cmd.args(["-w", "/work"]);
    }

    if let Some(user) = parsed.user {
        cmd.arg("-u");
        cmd.arg(user);
    }
    cmd.arg(workspace);

    if parsed.command.is_empty() {
        cmd.args(["zsh", "-l"]);
    } else {
        cmd.args(parsed.command);
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(EXIT_RUNTIME),
        Err(err) => {
            eprintln!("error: failed to run docker exec: {err}");
            EXIT_RUNTIME
        }
    }
}

#[derive(Debug, Default, Clone)]
struct ParsedRm {
    show_help: bool,
    all: bool,
    workspace: Option<OsString>,
}

fn parse_rm_args(args: &[OsString]) -> Result<ParsedRm, String> {
    let mut parsed = ParsedRm::default();

    for arg in args {
        let text = arg.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--all" => parsed.all = true,
            "--yes" => {}
            _ if text.starts_with('-') => {
                return Err(format!("unknown option for rm: {text}"));
            }
            _ => {
                if parsed.workspace.is_some() {
                    return Err(String::from("rm accepts at most one workspace name"));
                }
                parsed.workspace = Some(arg.clone());
            }
        }
    }

    Ok(parsed)
}

fn run_rm(args: &[OsString]) -> i32 {
    let parsed = match parse_rm_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_rm_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_rm_usage();
        return 0;
    }

    if parsed.all {
        let workspaces = match list_workspaces() {
            Ok(items) => items,
            Err(err) => {
                eprintln!("error: {err}");
                return EXIT_RUNTIME;
            }
        };
        for workspace in workspaces {
            let code = forward("rm", &[OsString::from(workspace)]);
            if code != 0 {
                return code;
            }
        }
        return 0;
    }

    if let Some(workspace) = parsed.workspace {
        return forward("rm", &[workspace]);
    }

    eprintln!("error: missing workspace name or --all");
    print_rm_usage();
    EXIT_RUNTIME
}

fn list_workspaces() -> Result<Vec<String>, String> {
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            "label=agent-kit.workspace=1",
            "--format",
            "{{.Names}}",
        ])
        .output()
        .map_err(|err| format!("failed to list workspaces via docker: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "docker ps failed (exit {}): {stderr}",
            output.status.code().unwrap_or(EXIT_RUNTIME)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

fn print_exec_usage() {
    eprintln!("usage: agent-workspace exec [--root|--user <user>] <workspace> [command ...]");
}

fn print_rm_usage() {
    eprintln!("usage: agent-workspace rm [--all] [--yes] <workspace>");
}

fn resolve_workspace_container_name(workspace: &OsString) -> OsString {
    let workspace_name = workspace.to_string_lossy();
    if docker_container_exists(&workspace_name) {
        return workspace.clone();
    }

    let prefix = std::env::var("AGENT_WORKSPACE_PREFIX")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| String::from("codex-ws"));

    let prefixed = if workspace_name.starts_with(&(prefix.clone() + "-")) {
        workspace_name.to_string()
    } else {
        format!("{prefix}-{workspace_name}")
    };

    if docker_container_exists(&prefixed) {
        return OsString::from(prefixed);
    }

    workspace.clone()
}

fn docker_container_exists(name: &str) -> bool {
    Command::new("docker")
        .args(["container", "inspect", name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub(crate) fn forward_with_launcher_and_env(
    launcher: &Path,
    subcommand: &str,
    args: &[OsString],
    env_overrides: &[(&str, &str)],
) -> i32 {
    if !launcher.is_file() {
        eprintln!("error: launcher not found: {}", launcher.display());
        eprintln!("hint: set {LAUNCHER_ENV} to the low-level launcher path");
        return EXIT_RUNTIME;
    }

    let mut cmd = Command::new(launcher);
    cmd.arg(subcommand);
    cmd.args(args.iter().cloned());
    for (k, v) in env_overrides {
        cmd.env(k, v);
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(EXIT_RUNTIME),
        Err(err) => {
            eprintln!(
                "error: failed to run launcher {}: {err}",
                launcher.display()
            );
            EXIT_RUNTIME
        }
    }
}

fn resolve_launcher_path() -> PathBuf {
    launcher_path_from_env(std::env::var_os(LAUNCHER_ENV))
}

fn launcher_path_from_env(value: Option<OsString>) -> PathBuf {
    match value {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => auto_detect_launcher_path(),
    }
}

fn auto_detect_launcher_path() -> PathBuf {
    if Path::new(DEFAULT_LAUNCHER_PATH).is_file() {
        return PathBuf::from(DEFAULT_LAUNCHER_PATH);
    }
    if Path::new(LEGACY_LAUNCHER_PATH).is_file() {
        return PathBuf::from(LEGACY_LAUNCHER_PATH);
    }
    PathBuf::from(DEFAULT_LAUNCHER_PATH)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;

    use super::{
        DEFAULT_LAUNCHER_PATH, forward_with_launcher_and_env, launcher_path_from_env,
        parse_exec_args, parse_rm_args, translate_create_args,
    };
    use crate::EXIT_RUNTIME;

    #[test]
    fn launcher_path_defaults_when_env_absent() {
        let path = launcher_path_from_env(None);
        assert_eq!(path, PathBuf::from(DEFAULT_LAUNCHER_PATH));
    }

    #[test]
    fn launcher_path_uses_env_when_present() {
        let path = launcher_path_from_env(Some(OsString::from("/tmp/custom-launcher")));
        assert_eq!(path, PathBuf::from("/tmp/custom-launcher"));
    }

    #[test]
    fn launcher_path_treats_empty_env_as_default() {
        let path = launcher_path_from_env(Some(OsString::from("")));
        assert_eq!(path, PathBuf::from(DEFAULT_LAUNCHER_PATH));
    }

    #[test]
    fn forwarding_passes_subcommand_args_and_codex_env() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let launcher = write_stub_launcher(temp.path());
        let log_path = temp.path().join("launcher.log");
        let log_path_str = log_path.to_string_lossy().to_string();

        let args = vec![
            OsString::from("github"),
            OsString::from("ws-test"),
            OsString::from("--host"),
            OsString::from("github.com"),
        ];

        let exit_code = forward_with_launcher_and_env(
            &launcher,
            "auth",
            &args,
            &[
                ("AW_TEST_LOG", &log_path_str),
                ("CODEX_SECRET_DIR", "/tmp/codex-secrets"),
                ("CODEX_AUTH_FILE", "/tmp/codex-auth.json"),
            ],
        );
        assert_eq!(exit_code, 0);

        let log = fs::read_to_string(log_path).expect("read log");
        for expected in [
            "subcommand=auth",
            "arg0=github",
            "arg1=ws-test",
            "arg2=--host",
            "arg3=github.com",
            "codex_secret_dir=/tmp/codex-secrets",
            "codex_auth_file=/tmp/codex-auth.json",
        ] {
            assert!(log.contains(expected), "missing line: {expected}\n{log}");
        }
    }

    #[test]
    fn forwarding_returns_child_exit_code() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let launcher = write_stub_launcher(temp.path());

        let exit_code =
            forward_with_launcher_and_env(&launcher, "ls", &[], &[("AW_TEST_EXIT_CODE", "17")]);
        assert_eq!(exit_code, 17);
    }

    #[test]
    fn forwarding_fails_when_launcher_is_missing() {
        let path = PathBuf::from("/tmp/agent-workspace-tests/missing-launcher");
        let exit_code = forward_with_launcher_and_env(&path, "ls", &[], &[]);
        assert_eq!(exit_code, EXIT_RUNTIME);
    }

    #[test]
    fn create_translation_maps_no_work_repos_to_no_clone() {
        let translated = translate_create_args(&[
            OsString::from("--no-work-repos"),
            OsString::from("--name"),
            OsString::from("ws-test"),
        ]);
        let values: Vec<String> = translated
            .into_iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect();
        assert_eq!(values, vec!["--no-clone", "--name", "ws-test"]);
    }

    #[test]
    fn exec_parser_extracts_user_workspace_and_command() {
        let parsed = parse_exec_args(&[
            OsString::from("--user"),
            OsString::from("codex"),
            OsString::from("ws-test"),
            OsString::from("id"),
            OsString::from("-u"),
        ])
        .expect("parse");

        assert!(!parsed.show_help);
        assert_eq!(parsed.user, Some(OsString::from("codex")));
        assert_eq!(parsed.workspace, Some(OsString::from("ws-test")));
        let command: Vec<String> = parsed
            .command
            .into_iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect();
        assert_eq!(command, vec!["id", "-u"]);
    }

    #[test]
    fn exec_parser_supports_root_flag() {
        let parsed = parse_exec_args(&[
            OsString::from("--root"),
            OsString::from("ws-test"),
            OsString::from("id"),
            OsString::from("-u"),
        ])
        .expect("parse");
        assert_eq!(parsed.user, Some(OsString::from("0")));
        assert_eq!(parsed.workspace, Some(OsString::from("ws-test")));
    }

    #[test]
    fn rm_parser_accepts_yes_and_workspace() {
        let parsed =
            parse_rm_args(&[OsString::from("ws-test"), OsString::from("--yes")]).expect("parse");

        assert!(!parsed.all);
        assert_eq!(parsed.workspace, Some(OsString::from("ws-test")));
    }

    #[test]
    fn rm_parser_accepts_all() {
        let parsed =
            parse_rm_args(&[OsString::from("--all"), OsString::from("--yes")]).expect("parse");

        assert!(parsed.all);
        assert_eq!(parsed.workspace, None);
    }

    fn write_stub_launcher(dir: &std::path::Path) -> PathBuf {
        let path = dir.join("launcher-stub.sh");
        fs::write(&path, launcher_script()).expect("write launcher stub");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).expect("chmod");
        }

        path
    }

    fn launcher_script() -> &'static str {
        r#"#!/usr/bin/env bash
set -euo pipefail

log="${AW_TEST_LOG:-/dev/null}"
printf 'subcommand=%s\n' "$1" >"$log"
shift

i=0
for arg in "$@"; do
  printf 'arg%s=%s\n' "$i" "$arg" >>"$log"
  i=$((i + 1))
done

printf 'codex_secret_dir=%s\n' "${CODEX_SECRET_DIR:-}" >>"$log"
printf 'codex_auth_file=%s\n' "${CODEX_AUTH_FILE:-}" >>"$log"
exit "${AW_TEST_EXIT_CODE:-0}"
"#
    }
}
