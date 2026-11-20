//! The `rgx` binary: SPEC section 7.
//!
//! The engine is the library crate `rgx`. Everything about the command
//! line, files and threads is in the `cli` module of this binary, so that
//! the library stays a small, pure piece that could be published alone.

#![forbid(unsafe_code)]

mod cli;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run(std::env::args_os().skip(1))
}
