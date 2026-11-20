//! The `rgx` binary.
//!
//! This starter compiles and does nothing useful. Replace it with your
//! program: SPEC.md section 7 says what the binary must do.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("rgx: nothing is implemented yet. Start with SPEC.md.");
    // Not one of the exit statuses of SPEC 7.6, so no test passes by accident.
    ExitCode::from(100)
}
