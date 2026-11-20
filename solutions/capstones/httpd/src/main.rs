//! `httpd`: a small HTTP/1.1 server for static files and a JSON echo
//! endpoint, written with the standard library only. SPEC.md is the
//! contract; DESIGN.md explains why the code is shaped this way.
//!
//! The modules, from the outside in:
//!
//! - [`config`]: the command line.
//! - [`server`]: the listener, admission control and the shutdown sequence.
//! - [`pool`]: a bounded pool of worker threads.
//! - [`conn`]: one connection: timeouts, keep-alive, the order of the checks.
//! - [`request`], [`response`], [`date`]: the wire format.
//! - [`files`], [`json`]: the two things the server can answer with.

mod config;
mod conn;
mod date;
mod files;
mod json;
mod pool;
mod request;
mod response;
mod server;

use std::process::ExitCode;

use config::Cli;

/// Exit status for a command line the server cannot understand.
const EXIT_USAGE: u8 = 2;
/// Exit status for a start-up failure (bad root, address in use).
const EXIT_START: u8 = 1;

fn main() -> ExitCode {
    let config = match config::parse(std::env::args_os().skip(1)) {
        Ok(Cli::Run(config)) => config,
        Ok(Cli::Help) => {
            println!("{}", config::USAGE);
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("httpd: {error}");
            eprintln!("Try `httpd --help`.");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    match server::run(config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("httpd: {error}");
            ExitCode::from(EXIT_START)
        }
    }
}
