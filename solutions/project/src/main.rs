//! The `loglens` binary. Milestone m18 (Chapter 18).
//!
//! m18 G. Connect the process to `loglens::cli::main_with`:
//!
//! - arguments: `std::env::args_os()`, without the program name (`args()`
//!   would panic on a file name that is not UTF-8);
//! - stdin: `io::stdin().lock()` (a `BufRead`);
//! - stdout: the locked stdout inside a `BufWriter`, so printing many lines
//!   does not make one system call per line;
//! - stderr: `io::stderr().lock()` (unbuffered, so messages appear at once);
//! - the returned status becomes the process exit status via
//!   `ExitCode::from`.

use std::io::{self, BufWriter};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut stdin = io::stdin().lock();
    let mut stdout = BufWriter::new(io::stdout().lock());
    let mut stderr = io::stderr().lock();
    let status = loglens::cli::main_with(
        std::env::args_os().skip(1),
        &mut stdin,
        &mut stdout,
        &mut stderr,
    );
    ExitCode::from(status)
}
