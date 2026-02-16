use std::ffi::OsString;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "agent-workspace",
    version,
    about = "Rust CLI that forwards workspace commands to the low-level launcher",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {
    #[command(disable_help_flag = true)]
    Auth(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Create(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Ls(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Rm(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Exec(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Reset(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Tunnel(PassthroughArgs),
}

#[derive(Debug, Args)]
#[command(trailing_var_arg = true)]
pub struct PassthroughArgs {
    #[arg(
        value_name = "ARG",
        num_args = 0..,
        allow_hyphen_values = true,
        value_parser = clap::builder::OsStringValueParser::new()
    )]
    pub args: Vec<OsString>,
}

pub struct ForwardRequest {
    pub subcommand: &'static str,
    pub args: Vec<OsString>,
}

impl CliCommand {
    pub fn into_forward_request(self) -> ForwardRequest {
        match self {
            Self::Auth(args) => ForwardRequest {
                subcommand: "auth",
                args: args.args,
            },
            Self::Create(args) => ForwardRequest {
                subcommand: "create",
                args: args.args,
            },
            Self::Ls(args) => ForwardRequest {
                subcommand: "ls",
                args: args.args,
            },
            Self::Rm(args) => ForwardRequest {
                subcommand: "rm",
                args: args.args,
            },
            Self::Exec(args) => ForwardRequest {
                subcommand: "exec",
                args: args.args,
            },
            Self::Reset(args) => ForwardRequest {
                subcommand: "reset",
                args: args.args,
            },
            Self::Tunnel(args) => ForwardRequest {
                subcommand: "tunnel",
                args: args.args,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{Cli, CliCommand};

    #[test]
    fn help_lists_core_subcommands() {
        let mut cmd = Cli::command();
        let mut out = Vec::new();
        cmd.write_long_help(&mut out).expect("write help");

        let help = String::from_utf8(out).expect("utf8");
        for subcommand in ["auth", "create", "ls", "rm", "exec", "reset", "tunnel"] {
            assert!(
                help.contains(subcommand),
                "help should include subcommand {subcommand}"
            );
        }
    }

    #[test]
    fn parse_reset_keeps_trailing_args() {
        let cli = Cli::try_parse_from([
            "agent-workspace",
            "reset",
            "repo",
            "ws-test",
            "--yes",
            "--ref",
            "origin/main",
        ])
        .expect("parse args");

        let CliCommand::Reset(args) = cli.command else {
            panic!("expected reset command");
        };
        let collected: Vec<String> = args
            .args
            .into_iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            collected,
            vec!["repo", "ws-test", "--yes", "--ref", "origin/main"]
        );
    }
}
