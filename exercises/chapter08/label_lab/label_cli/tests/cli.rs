//! Runs the two binaries the way a user would: as separate processes.
//!
//! When Cargo builds an integration test of a package that has binaries, it
//! builds the binaries first and passes each path in a compile-time variable
//! named `CARGO_BIN_EXE_<name>`.

use std::process::{Command, Output};

const LABEL_CLI: &str = env!("CARGO_BIN_EXE_label_cli");
const LABEL_CHECK: &str = env!("CARGO_BIN_EXE_label_check");

fn run(exe: &str, args: &[&str]) -> Output {
    Command::new(exe)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("could not start {exe}: {error}"))
}

/// Checks the exit status first, with the program's own error output in the
/// failure message, and returns (stdout, stderr) as text.
fn expect_status(output: &Output, status: i32, what: &str) -> (String, String) {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(
        output.status.code(),
        Some(status),
        "{what}: wrong exit status.\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    (stdout, stderr)
}

#[test]
fn step4_cli_prints_one_tag_per_label() {
    let output = run(LABEL_CLI, &[" North Dock ", "Bay 7", "港"]);
    let (stdout, stderr) = expect_status(&output, 0, "three valid labels");
    assert_eq!(stdout, "tag:North Dock\ntag:Bay 7\ntag:港\n");
    assert_eq!(stderr, "", "a successful run prints no diagnostics");
}

#[test]
fn step4_cli_stops_at_the_first_invalid_label() {
    let too_long = "x".repeat(30);
    let output = run(LABEL_CLI, &["ok", too_long.as_str(), "   "]);
    let (stdout, stderr) = expect_status(&output, 1, "an invalid label");
    assert_eq!(stdout, "", "no tag is printed when any label is invalid");
    assert_eq!(
        stderr.trim_end(),
        "label_cli: argument 2: label has 30 characters; the limit is 24",
        "report the FIRST invalid label, numbered from 1, with the library's message"
    );

    let output = run(LABEL_CLI, &[""]);
    let (_, stderr) = expect_status(&output, 1, "an empty label");
    assert_eq!(stderr.trim_end(), "label_cli: argument 1: label is empty");
}

#[test]
fn step4_cli_without_labels_prints_usage() {
    let output = run(LABEL_CLI, &[]);
    let (stdout, stderr) = expect_status(&output, 2, "no arguments");
    assert_eq!(stdout, "");
    assert_eq!(stderr.trim_end(), "usage: label_cli [--slug] LABEL...");
}

#[test]
fn step4_label_check_answers_with_its_exit_status() {
    let output = run(LABEL_CHECK, &["Bay 7", " padded "]);
    let (stdout, stderr) = expect_status(&output, 0, "two valid labels");
    assert_eq!(
        (stdout.as_str(), stderr.as_str()),
        ("", ""),
        "print nothing"
    );

    let output = run(LABEL_CHECK, &["Bay 7", " "]);
    let (stdout, stderr) = expect_status(&output, 1, "one invalid label");
    assert_eq!(
        (stdout.as_str(), stderr.as_str()),
        ("", ""),
        "print nothing"
    );

    let output = run(LABEL_CHECK, &[]);
    expect_status(&output, 0, "nothing to check");
}

/// Runs only in a build WITHOUT this package's `slug` feature (the default).
#[cfg(not(feature = "slug"))]
#[test]
fn step4_slug_flag_needs_the_feature() {
    let output = run(LABEL_CLI, &["--slug", "North Dock"]);
    let (stdout, stderr) = expect_status(&output, 2, "--slug without the feature");
    assert_eq!(stdout, "");
    assert_eq!(
        stderr.trim_end(),
        "label_cli: built without the slug feature"
    );
}

/// Runs only in a build WITH this package's `slug` feature:
/// `cargo test -p label_cli --features slug`.
#[cfg(feature = "slug")]
mod with_slug {
    use super::{LABEL_CLI, expect_status, run};

    #[test]
    fn step6_slug_flag_prints_slugs() {
        let output = run(LABEL_CLI, &["--slug", " North Dock #7 ", "Bay 7"]);
        let (stdout, stderr) = expect_status(&output, 0, "--slug with the feature");
        assert_eq!(stdout, "north-dock-7\nbay-7\n");
        assert_eq!(stderr, "");
    }

    #[test]
    fn step6_slug_flag_is_not_a_label() {
        let output = run(LABEL_CLI, &["--slug", "ok", ""]);
        let (stdout, stderr) = expect_status(&output, 1, "--slug with an invalid label");
        assert_eq!(stdout, "");
        assert_eq!(
            stderr.trim_end(),
            "label_cli: argument 2: label is empty",
            "labels are numbered from 1; --slug has no number"
        );

        let output = run(LABEL_CLI, &["--slug"]);
        let (_, stderr) = expect_status(&output, 2, "--slug without labels");
        assert_eq!(stderr.trim_end(), "usage: label_cli [--slug] LABEL...");
    }

    #[test]
    fn step6_tags_still_work() {
        let output = run(LABEL_CLI, &["Bay 7"]);
        let (stdout, _) = expect_status(&output, 0, "a tag in a slug build");
        assert_eq!(stdout, "tag:Bay 7\n");
    }
}
