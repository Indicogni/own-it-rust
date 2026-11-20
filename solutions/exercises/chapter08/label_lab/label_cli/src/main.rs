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

use label_core::LabelError;

/// The slug of one label. Two definitions, and `#[cfg]` keeps exactly one:
/// `label_core::slug` does not exist without the feature, so a call to it
/// must be removed before the compiler resolves names. `if cfg!(..)` could
/// not do that, because both of its branches are compiled.
#[cfg(feature = "slug")]
fn slug(raw: &str) -> Result<String, LabelError> {
    label_core::slug(raw)
}

#[cfg(not(feature = "slug"))]
fn slug(_raw: &str) -> Result<String, LabelError> {
    unreachable!("main rejects --slug in a build without the feature")
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (slug_mode, labels) = match args.split_first() {
        Some((flag, rest)) if flag == "--slug" => (true, rest),
        _ => (false, args.as_slice()),
    };
    if slug_mode && !cfg!(feature = "slug") {
        eprintln!("label_cli: built without the slug feature");
        return ExitCode::from(2);
    }
    if labels.is_empty() {
        eprintln!("usage: label_cli [--slug] LABEL...");
        return ExitCode::from(2);
    }

    // Convert everything before printing anything, so that an invalid label
    // leaves standard output empty.
    let mut lines = Vec::with_capacity(labels.len());
    for (index, raw) in labels.iter().enumerate() {
        let line = if slug_mode {
            slug(raw)
        } else {
            label_core::tag(raw)
        };
        match line {
            Ok(line) => lines.push(line),
            Err(error) => {
                eprintln!("label_cli: argument {}: {error}", index + 1);
                return ExitCode::FAILURE;
            }
        }
    }
    for line in lines {
        println!("{line}");
    }
    ExitCode::SUCCESS
}
