//! The `kvstore` binary. SPEC.md says what it must do: section 3 for the
//! command line and the exit codes, section 8 for the protocol.
//!
//! This starter exits with code 1, which SPEC.md keeps for run-time
//! failures. Every acceptance test therefore fails with a message that
//! shows the line below, and none of them waits for a timeout.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("kvstore: not implemented yet. Start with SPEC.md, milestone M1.");
    ExitCode::FAILURE
}
