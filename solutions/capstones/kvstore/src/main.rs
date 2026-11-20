//! The `kvstore` binary: arguments in, exit code out. Everything else is in
//! the library.

use std::process::ExitCode;

use kvstore::config::{Config, USAGE};

fn main() -> ExitCode {
    let config = match Config::parse(std::env::args_os().skip(1)) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("kvstore: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match kvstore::run(&config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("kvstore: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}
