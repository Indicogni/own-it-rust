//! Project automation written in Rust: `cargo xtask <task>`.
//!
//! An xtask is an ordinary binary crate in the workspace. A Cargo alias makes
//! `cargo xtask ...` run it. Contributors need nothing but Cargo: no make, no shell.

use std::process::{Command, ExitCode};

/// The checks a pull request must pass, in the order CI runs them.
const CI_STEPS: &[&[&str]] = &[
    &["fmt", "--all", "--check"],
    &[
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--",
        "-D",
        "warnings",
    ],
    &["test", "--workspace", "--locked"],
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["ci"] => run_ci(false),
        ["ci", "--dry-run"] => run_ci(true),
        _ => {
            eprintln!("usage: cargo xtask ci [--dry-run]");
            ExitCode::from(2)
        }
    }
}

fn run_ci(dry_run: bool) -> ExitCode {
    // Cargo sets CARGO for everything it runs, so the xtask uses the same Cargo.
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    for step in CI_STEPS {
        println!("cargo {}", step.join(" "));
        if dry_run {
            continue;
        }
        let status = Command::new(&cargo).args(*step).status();
        if !status.is_ok_and(|status| status.success()) {
            eprintln!("step failed: cargo {}", step.join(" "));
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
