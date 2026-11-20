//! A test of the reference solution's own, not an acceptance test.
//!
//! SPEC 7.7 asks that the program ends quietly when its standard *output*
//! is closed. This solution holds itself to the same for standard *error*:
//! a message that nobody can read must not turn "status 2" into a panic.
//! `eprintln!` panics when its write fails, which is why `cli::report`
//! exists.

// The test starts a process, which Miri cannot do.
#![cfg(not(miri))]

mod common;

use std::io::Write;

use common::{FAILED, RUN_LIMIT, TempDir, spawn, wait_with_limit};

#[test]
fn a_closed_standard_error_does_not_turn_an_error_into_a_panic() {
    let dir = TempDir::new();
    // Without a path the program reads standard input to its end before it
    // has anything to report. That gives the test the time to close the
    // read end of the pipe first, so nothing here depends on timing.
    let mut child = spawn(dir.path(), &["x"]);
    drop(child.0.stderr.take());
    let mut stdin = child.0.stdin.take().expect("stdin is piped");
    // Not valid UTF-8: the program must report `invalid UTF-8 at byte 10`.
    stdin
        .write_all(b"not text: \xFF\n")
        .expect("rgx reads its standard input");
    drop(stdin);

    let Some(status) = wait_with_limit(&mut child.0, RUN_LIMIT) else {
        panic!("TIMEOUT: rgx did not end after its standard input was closed");
    };
    assert_eq!(
        status.code(),
        Some(FAILED),
        "an input that fails is status 2 (SPEC 7.6), also when the message could not be written; \
         101 would be a panic"
    );
}
