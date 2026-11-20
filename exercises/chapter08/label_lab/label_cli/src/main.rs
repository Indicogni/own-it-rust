//! label_cli: print one tag for every label given on the command line.
//!
//! Contract (step 4), where LABEL arguments are numbered from 1:
//!
//! - `label_cli LABEL...` prints `label_core::tag` of every label, one per
//!   line, to standard output and exits with status 0.
//! - If a label is invalid, print NOTHING to standard output. Print
//!   `label_cli: argument N: REASON` to standard error, where N is the
//!   position of the first invalid label and REASON is the error's `Display`
//!   text, and exit with status 1.
//! - Without any LABEL, print `usage: label_cli [--slug] LABEL...` to
//!   standard error and exit with status 2.
//! - If the first argument is exactly `--slug`, print
//!   `label_cli: built without the slug feature` to standard error and exit
//!   with status 2.
//!
//! Contract (step 6), which changes the last rule for a build WITH this
//! package's `slug` feature:
//!
//! - A first argument `--slug` selects slug mode: print `label_core::slug`
//!   of every label instead of its tag. `--slug` is not a LABEL and has no
//!   number. With `--slug` and no LABEL, print the usage line as above.
//! - A build without the feature keeps the step 4 behaviour.
//!
//! Keep this file thin: parsing arguments and choosing exit codes belongs
//! here; every rule about labels belongs in the library.

use std::process::ExitCode;

fn main() -> ExitCode {
    todo!("4: follow the contract at the top of this file")
}
