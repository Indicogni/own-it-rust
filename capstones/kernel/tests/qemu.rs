//! Acceptance tests that boot your kernel in QEMU (SPEC.md section 4).
//!
//! ```text
//! cargo test --test qemu                 # all of them
//! cargo test --test qemu m1              # one milestone
//! cargo test --test qemu -- --nocapture  # also show the SKIPPED reason again
//! ```
//!
//! The kernel is built and booted once; every milestone test then looks at
//! the lines of the serial output that carry its tag. `panic_selftest`
//! builds and boots a second kernel with the `selftest-panic` feature.
//!
//! Without `qemu-system-aarch64` or the `aarch64-unknown-none` target the
//! tests print a line that starts with `SKIPPED` and pass. Set
//! `KERNEL_REQUIRE_QEMU=1` to turn such a skip into a failure.

// Miri cannot start other programs.
#![cfg(not(miri))]

use std::sync::OnceLock;

use harness::{Config, End, Outcome, Run, run_kernel, tagged, transcript};

/// The one boot of the normal kernel that all milestone tests share.
fn normal_run() -> Option<&'static Run> {
    static RUN: OnceLock<Outcome> = OnceLock::new();
    let outcome = RUN.get_or_init(|| {
        run_kernel(&Config {
            scratch_dir: env!("CARGO_TARGET_TMPDIR"),
            features: &[],
        })
    });
    ran(outcome)
}

/// The one boot of the kernel built with `--features selftest-panic`.
fn panic_run() -> Option<&'static Run> {
    static RUN: OnceLock<Outcome> = OnceLock::new();
    let outcome = RUN.get_or_init(|| {
        run_kernel(&Config {
            scratch_dir: env!("CARGO_TARGET_TMPDIR"),
            features: &["selftest-panic"],
        })
    });
    ran(outcome)
}

fn ran(outcome: &'static Outcome) -> Option<&'static Run> {
    match outcome {
        Outcome::Ran(run) => Some(run),
        Outcome::Skipped(message) => {
            // Shown per test with `--nocapture`; the harness has already
            // printed the same line once, uncaptured.
            println!("{message}");
            None
        }
        // The build ran once; every test that needs the image reports it.
        Outcome::BuildFailed(message) => panic!("{message}"),
    }
}

/// Compares the lines tagged `[mN] ` with the transcript for milestone N.
fn check_milestone(number: u32) {
    let Some(run) = normal_run() else { return };
    let tag = format!("[m{number}] ");
    let expected = tagged(transcript(), &tag);
    let actual = tagged(run.lines(), &tag);
    assert!(
        actual == expected,
        "milestone {number}: the lines tagged `{}` differ from SPEC 4.1.\n\
         --- expected ---\n{}\n--- your kernel printed ---\n{}\n{}",
        tag.trim_end(),
        expected.join("\n"),
        if actual.is_empty() {
            "(no such line)".to_string()
        } else {
            actual.join("\n")
        },
        run.report()
    );
}

#[test]
fn m1_boot() {
    check_milestone(1);
}

#[test]
fn m2_drivers() {
    check_milestone(2);
}

#[test]
fn m3_heap() {
    check_milestone(3);
}

#[test]
fn m4_interrupts() {
    check_milestone(4);
}

#[test]
fn m5_tasks() {
    check_milestone(5);
}

#[test]
fn m6_ipc() {
    check_milestone(6);
}

/// SPEC 3.5: the kernel ends every run by switching the machine off.
#[test]
fn powers_off_cleanly() {
    let Some(run) = normal_run() else { return };
    assert!(
        run.end == End::Exited(Some(0)),
        "the kernel must switch the machine off, so that QEMU exits with status 0 by itself.{}",
        run.report()
    );
    assert!(
        tagged(run.lines(), "[panic] ").is_empty(),
        "the kernel panicked.{}",
        run.report()
    );
    assert!(
        run.lines().last() == Some(&"[m1] power off"),
        "the last line of every run must be `[m1] power off`.{}",
        run.report()
    );
}

/// SPEC 4.2: with all six milestones done, the output is exactly the
/// transcript: every line, in order, and nothing else.
#[test]
fn full_transcript() {
    let Some(run) = normal_run() else { return };
    let expected = transcript();
    let actual = run.lines();
    if actual == expected {
        return;
    }
    let first_difference = expected
        .iter()
        .zip(&actual)
        .position(|(want, got)| want != got)
        .unwrap_or(expected.len().min(actual.len()));
    panic!(
        "the serial output is not the transcript of SPEC 4.1 (tests/transcript.txt).\n\
         First difference at line {}:\n  expected: {}\n  printed:  {}\n{}",
        first_difference + 1,
        expected
            .get(first_difference)
            .unwrap_or(&"(end of transcript)"),
        actual.get(first_difference).unwrap_or(&"(end of output)"),
        run.report()
    );
}

/// SPEC 4.3: a panic prints one `[panic]` line and switches the machine off.
#[test]
fn panic_selftest() {
    let Some(run) = panic_run() else { return };
    let lines = run.lines();
    assert!(
        lines.first() == Some(&"[m1] boot: EL1"),
        "selftest-panic: the first line must still be `[m1] boot: EL1`.{}",
        run.report()
    );
    assert!(
        lines.len() == 2 && lines[1].starts_with("[panic] ") && lines[1].contains("selftest"),
        "selftest-panic: the second and last line must start with `[panic] ` \
         and contain the panic message `selftest`.{}",
        run.report()
    );
    assert!(
        run.end == End::Exited(Some(0)),
        "selftest-panic: after printing, the panic handler must switch the machine off.{}",
        run.report()
    );
}
