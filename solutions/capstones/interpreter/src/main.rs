//! The `tansy` binary: runs the interpreter on a thread with a large stack
//! and turns its result into the exit status.

use std::ffi::OsString;
use std::io::{self, BufWriter};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let status = tansy::with_large_stack(move || {
        // `print` in a loop must not make one system call per line, so
        // standard output is block buffered; `cli` flushes it wherever the
        // specification says output must be visible.
        let mut out = BufWriter::new(io::stdout().lock());
        tansy::cli::run(
            &args,
            &mut io::stdin().lock(),
            &mut out,
            &mut io::stderr().lock(),
        )
    });
    match status {
        Ok(status) => ExitCode::from(status),
        Err(e) => {
            eprintln!("error: cannot start the interpreter thread: {e}");
            ExitCode::from(tansy::cli::EXIT_USAGE_OR_IO)
        }
    }
}
