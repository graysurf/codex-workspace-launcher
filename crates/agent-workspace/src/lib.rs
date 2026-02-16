mod cli;
mod launcher;

use clap::Parser;
use cli::Cli;

pub const EXIT_RUNTIME: i32 = 1;

pub fn run() -> i32 {
    run_with_args(std::env::args_os())
}

pub fn run_with_args<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            let code = err.exit_code();
            let _ = err.print();
            return code;
        }
    };

    let request = cli.command.into_forward_request();
    launcher::dispatch(request.subcommand, &request.args)
}
