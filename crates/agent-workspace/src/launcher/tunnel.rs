use std::ffi::OsString;
use std::process::{Command, Stdio};

use crate::EXIT_RUNTIME;

use super::{
    PRIMARY_COMMAND_NAME, command_exists, json_escape, resolve_workspace, trimmed_nonempty,
};

#[derive(Debug, Default, Clone)]
pub(super) struct ParsedTunnel {
    pub(super) show_help: bool,
    pub(super) workspace: Option<String>,
    pub(super) tunnel_name: Option<String>,
    pub(super) detach: bool,
    pub(super) output_json: bool,
}

pub(super) fn parse_tunnel_args(args: &[OsString]) -> Result<ParsedTunnel, String> {
    let mut parsed = ParsedTunnel::default();
    let mut idx = 0usize;

    while idx < args.len() {
        let current = args[idx].to_string_lossy();
        match current.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--detach" => parsed.detach = true,
            "--name" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --name"));
                }
                parsed.tunnel_name = trimmed_nonempty(args[idx].to_string_lossy().as_ref());
            }
            "--output" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --output"));
                }
                let value = args[idx].to_string_lossy();
                if value != "json" {
                    return Err(format!("unsupported --output value: {value}"));
                }
                parsed.output_json = true;
            }
            _ if current.starts_with("--name=") => {
                parsed.tunnel_name = trimmed_nonempty(&current["--name=".len()..]);
            }
            _ if current.starts_with("--output=") => {
                let value = &current["--output=".len()..];
                if value != "json" {
                    return Err(format!("unsupported --output value: {value}"));
                }
                parsed.output_json = true;
            }
            _ if current.starts_with('-') => {
                return Err(format!("unknown option for tunnel: {current}"));
            }
            _ => {
                if parsed.workspace.is_some() {
                    return Err(format!("unexpected arg for tunnel: {current}"));
                }
                parsed.workspace = Some(current.to_string());
            }
        }
        idx += 1;
    }

    Ok(parsed)
}

pub(super) fn run(args: &[OsString]) -> i32 {
    let parsed = match parse_tunnel_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_tunnel_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_tunnel_usage();
        return 0;
    }

    let workspace_name = if let Some(workspace_name) = parsed.workspace.as_deref() {
        workspace_name
    } else {
        eprintln!("error: missing workspace name");
        print_tunnel_usage();
        return EXIT_RUNTIME;
    };

    let workspace = match resolve_workspace(workspace_name) {
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

    if !command_exists("code") {
        eprintln!("error: 'code' command not found in PATH (required for tunnel)");
        return EXIT_RUNTIME;
    }

    let mut cmd = Command::new("code");
    cmd.arg("tunnel");
    cmd.arg("--accept-server-license-terms");
    if let Some(tunnel_name) = parsed.tunnel_name.as_deref() {
        cmd.args(["--name", tunnel_name]);
    }
    cmd.current_dir(&workspace.path);

    if parsed.detach {
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        return match cmd.spawn() {
            Ok(child) => {
                if parsed.output_json {
                    println!(
                        "{{\"workspace\":\"{}\",\"detached\":true,\"pid\":{}}}",
                        json_escape(&workspace.name),
                        child.id()
                    );
                } else {
                    println!("tunnel: {} detached (pid={})", workspace.name, child.id());
                }
                0
            }
            Err(err) => {
                eprintln!("error: failed to launch tunnel: {err}");
                EXIT_RUNTIME
            }
        };
    }

    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    match cmd.status() {
        Ok(status) => {
            let code = status.code().unwrap_or(EXIT_RUNTIME);
            if parsed.output_json {
                println!(
                    "{{\"workspace\":\"{}\",\"detached\":false,\"exit_code\":{}}}",
                    json_escape(&workspace.name),
                    code
                );
            }
            code
        }
        Err(err) => {
            eprintln!("error: failed to run tunnel command: {err}");
            EXIT_RUNTIME
        }
    }
}

fn print_tunnel_usage() {
    println!("usage:");
    println!(
        "  {PRIMARY_COMMAND_NAME} tunnel <workspace> [--name <tunnel_name>] [--detach] [--output json]"
    );
}
