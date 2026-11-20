//! The `tansy` binary.
//!
//! This starter compiles and does nothing useful. Replace it with your
//! interpreter: SPEC.md says what the binary must do, and README.md says how
//! to work through the milestones.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("tansy: nothing is implemented yet. Start with SPEC.md.");
    // Not one of the exit statuses in SPEC 2.2, so no test passes by accident.
    ExitCode::from(100)
}
