use std::ffi::OsString;
use std::fs;

use crate::EXIT_RUNTIME;

use super::{PRIMARY_COMMAND_NAME, confirm_or_abort, list_workspaces_on_disk, resolve_workspace};

#[derive(Debug, Default, Clone)]
struct ParsedRm {
    show_help: bool,
    all: bool,
    yes: bool,
    workspace: Option<String>,
}

fn parse_rm_args(args: &[OsString]) -> Result<ParsedRm, String> {
    let mut parsed = ParsedRm::default();

    for arg in args {
        let text = arg.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--all" => parsed.all = true,
            "-y" | "--yes" => parsed.yes = true,
            _ if text.starts_with('-') => {
                return Err(format!("unknown option for rm: {text}"));
            }
            _ => {
                if parsed.workspace.is_some() {
                    return Err(String::from("rm accepts at most one workspace name"));
                }
                parsed.workspace = Some(text.to_string());
            }
        }
    }

    Ok(parsed)
}

pub(super) fn run(args: &[OsString]) -> i32 {
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

    if parsed.all && parsed.workspace.is_some() {
        eprintln!("error: rm --all does not accept a workspace name");
        print_rm_usage();
        return EXIT_RUNTIME;
    }

    let targets = if parsed.all {
        match list_workspaces_on_disk() {
            Ok(items) => items,
            Err(err) => {
                eprintln!("error: {err}");
                return EXIT_RUNTIME;
            }
        }
    } else if let Some(workspace_name) = parsed.workspace.as_deref() {
        match resolve_workspace(workspace_name) {
            Ok(Some(workspace)) => vec![workspace],
            Ok(None) => {
                eprintln!("error: workspace not found: {workspace_name}");
                return EXIT_RUNTIME;
            }
            Err(err) => {
                eprintln!("error: {err}");
                return EXIT_RUNTIME;
            }
        }
    } else {
        eprintln!("error: missing workspace name or --all");
        print_rm_usage();
        return EXIT_RUNTIME;
    };

    if targets.is_empty() {
        return 0;
    }

    if !parsed.yes {
        if parsed.all {
            println!("This will remove {} workspace(s):", targets.len());
        } else {
            println!("This will remove workspace:");
        }
        for target in &targets {
            println!("  - {}", target.name);
        }
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    for target in targets {
        if let Err(err) = fs::remove_dir_all(&target.path) {
            eprintln!(
                "error: failed to remove workspace {} ({}): {err}",
                target.name,
                target.path.display()
            );
            return EXIT_RUNTIME;
        }
        println!("removed: {}", target.name);
    }

    0
}

fn print_rm_usage() {
    eprintln!("usage: {PRIMARY_COMMAND_NAME} rm [--all] [--yes] <workspace>");
}
