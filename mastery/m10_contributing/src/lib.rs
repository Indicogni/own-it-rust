//! Mastery module M10: working in real codebases and contributing.
//!
//! Each `lNN_*` module is one lesson. The lessons work on `fixture/`, a small project
//! called tasklane that you did not write. Start with `README.md` and read the
//! matching section of `CONCEPTS.md` before each lesson.

// Starter bodies are `todo!()`, so their parameters are unused until you implement them.
#![allow(dead_code, unused_variables)]

pub mod check;
pub mod history;
pub mod l01_orienting;
pub mod l02_reproduce;
pub mod l03_bisect;
pub mod l04_change;
pub mod l05_review;
pub mod l06_writing;
pub mod l07_soundness;
pub mod l08_dependencies;
pub mod l09_ecosystem;
pub mod l10_capstone;

/// Placeholder for a prediction. Replace the call with your answer before
/// running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

/// Runs `f` on a helper thread and returns its result, or fails the test with a clear
/// message if it has not finished after `secs` seconds. Tests use this around code
/// that would otherwise run for hours when a solution is too slow.
#[track_caller]
pub fn within<R: Send + 'static>(
    secs: u64,
    what: &str,
    f: impl FnOnce() -> R + Send + 'static,
) -> R {
    use std::sync::mpsc::{self, RecvTimeoutError};
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(std::time::Duration::from_secs(secs)) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "{what} was still running after {secs} s. The work it does grows much \
             faster than its input; look for the same sub-problem being solved again \
             and again."
        ),
        Err(RecvTimeoutError::Disconnected) => {
            panic!("{what} panicked before finishing; see the panic message above")
        }
    }
}
