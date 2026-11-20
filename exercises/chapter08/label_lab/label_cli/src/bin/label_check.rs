//! label_check: a second binary in the same package.
//!
//! Cargo builds every file in `src/bin/` as its own binary crate, named after
//! the file. This one and `src/main.rs` are separate crates: they cannot share
//! a module, but both can use the library `label_core`.
//!
//! Contract (step 4): exit with status 0 when every argument is a valid label
//! (also when there are no arguments), and with status 1 otherwise. Print
//! nothing.

use std::process::ExitCode;

fn main() -> ExitCode {
    todo!("4: report through the exit status only")
}
