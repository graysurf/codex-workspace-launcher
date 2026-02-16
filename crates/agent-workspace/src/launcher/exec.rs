use std::ffi::OsString;
use std::io::IsTerminal;
use std::process::{Command, Stdio};

use crate::EXIT_RUNTIME;

use super::{PRIMARY_COMMAND_NAME, resolve_workspace};

#[derive(Debug, Default, Clone)]
pub(super) struct ParsedExec {
    pub(super) show_help: bool,
    pub(super) user: Option<OsString>,
    pub(super) workspace: Option<OsString>,
    pub(super) command: Vec<OsString>,
}

pub(super) fn parse_exec_args(args: &[OsString]) -> Result<ParsedExec, String> {
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

pub(super) fn run(args: &[OsString]) -> i32 {
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

    let workspace_name = parsed
        .workspace
        .as_ref()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
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

    if parsed.user.is_some() {
        eprintln!("warn: --root/--user is ignored in host-native exec mode");
    }

    let mut command = if parsed.command.is_empty() {
        let shell = std::env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/bash"));
        let mut cmd = Command::new(shell);
        if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
            cmd.arg("-l");
        }
        cmd
    } else {
        let mut cmd = Command::new(&parsed.command[0]);
        if parsed.command.len() > 1 {
            cmd.args(&parsed.command[1..]);
        }
        cmd
    };

    command.current_dir(&workspace.path);
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    match command.status() {
        Ok(status) => status.code().unwrap_or(EXIT_RUNTIME),
        Err(err) => {
            eprintln!(
                "error: failed to run command in {}: {err}",
                workspace.path.display()
            );
            EXIT_RUNTIME
        }
    }
}

fn print_exec_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} exec [--root|--user <user>] <workspace> [command ...]"
    );
}
