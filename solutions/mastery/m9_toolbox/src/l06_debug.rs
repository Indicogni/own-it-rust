//! LESSON 06 — Debugging: panics, backtraces, hooks, and a debugger
//! Run: cargo test --manifest-path mastery/m9_toolbox/Cargo.toml l06_
//! Read first: CONCEPTS.md, "Lesson 06 · Debugging".
//!
//! PREDICT before running: first answer `l06_p_predictions` in `tests/l06_process.rs`
//! (replace each `guess()` with your answer). Then consider: a program panics with
//! "index out of bounds". Is the line in the panic message the line with the bug?
//! IMPLEMENT `capture_panic` below (task A), and repair `src/bin/stockroom.rs`
//! (task C, described in that file; its tests are in `tests/l06_process.rs`).
//! Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain the difference between the place where a program
//! fails and the place where it went wrong, using the stockroom bug.
//! JUDGMENT: Read the panic message and the backtrace first; they are free. Reach
//! for `dbg!` when you have one question, for a debugger when you want to look
//! around a stopped program, and for logging when the failure happens on a machine
//! you cannot stop.

use std::panic::UnwindSafe;

/// What a panic hook can learn about a panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanicReport {
    /// The panic message. `"<non-string payload>"` if the payload is neither a
    /// `&str` nor a `String` (for example after `std::panic::panic_any(42)`).
    pub message: String,
    /// The source file of the `panic!` call, as the compiler recorded it.
    pub file: String,
    /// The line of the `panic!` call.
    pub line: u32,
    /// The name of the panicking thread, if it has one.
    pub thread: Option<String>,
}

/// A. Runs `f`. If it panics, returns a report of the panic instead of letting the
/// panic travel on.
///
/// The information in [`PanicReport`] is only available inside a panic hook
/// (`std::panic::set_hook`), and the hook is one global setting for the whole
/// process. Tests run on many threads at once, so:
///
/// - Calls on different threads at the same time must not disturb each other: each
///   call gets the report of its own panic.
/// - A panic on a thread that is not inside `capture_panic` must still be reported
///   the normal way (through the hook that was installed before yours).
/// - A panic that is captured prints nothing.
/// - `capture_panic` may be called inside `f` (nesting). The inner call captures the
///   inner panic; the outer call still works afterwards.
pub fn capture_panic<R>(f: impl FnOnce() -> R + UnwindSafe) -> Result<R, PanicReport> {
    use std::cell::RefCell;
    use std::sync::Once;

    thread_local! {
        /// `Some` while this thread is inside `capture_panic`. The inner Option is
        /// the report, filled in by the hook.
        static SLOT: RefCell<Option<Option<PanicReport>>> = const { RefCell::new(None) };
    }
    static INSTALL: Once = Once::new();

    // One hook for the whole process, installed once and never removed. It decides
    // per thread: record the panic, or pass it on to the hook that was there before.
    INSTALL.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let recording = SLOT.with(|slot| slot.borrow().is_some());
            if !recording {
                previous(info);
                return;
            }
            let report = PanicReport {
                message: info
                    .payload_as_str()
                    .unwrap_or("<non-string payload>")
                    .to_owned(),
                file: info.location().map_or("", |l| l.file()).to_owned(),
                line: info.location().map_or(0, |l| l.line()),
                thread: std::thread::current().name().map(str::to_owned),
            };
            SLOT.with(|slot| *slot.borrow_mut() = Some(Some(report)));
        }));
    });

    // Remember the outer call's state, so nesting works.
    let outer = SLOT.with(|slot| slot.borrow_mut().replace(None));
    let result = std::panic::catch_unwind(f);
    let report = SLOT.with(|slot| std::mem::replace(&mut *slot.borrow_mut(), outer));
    match result {
        Ok(value) => Ok(value),
        Err(_) => Err(report.flatten().unwrap_or(PanicReport {
            message: "<panic without a hook call>".to_owned(),
            file: String::new(),
            line: 0,
            thread: None,
        })),
    }
}

// B. Predictions about backtraces, exit codes, `dbg!` and hooks are in
//    `tests/l06_process.rs`. They run the supplied program `src/bin/panicker.rs`.
//
// C. Repair `src/bin/stockroom.rs`. Its tests are in `tests/l06_process.rs` too.

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    /// Runs the body of one test in a process of its own (this test executable,
    /// started again for exactly that test). `capture_panic` changes the panic hook
    /// of the whole process, so a mistake in it must not silence or block the
    /// other tests. Run one of these tests alone to see its output directly.
    fn isolated(name: &str, body: impl FnOnce()) {
        crate::support::in_own_process(&format!("l06_debug::tests::{name}"), 90, body);
    }

    #[test]
    fn a_returns_the_value_when_nothing_panics() {
        isolated("a_returns_the_value_when_nothing_panics", || {
            assert_eq!(capture_panic(|| 6 * 7), Ok(42));
        });
    }

    #[test]
    fn a_reports_message_and_location() {
        isolated("a_reports_message_and_location", || {
            let expected_line = line!() + 1;
            let report = capture_panic(|| panic!("no stock for {}", "gear")).unwrap_err();
            assert_eq!(report.message, "no stock for gear");
            assert!(
                report.file.ends_with("l06_debug.rs"),
                "the report must name the file of the panic! call, got {:?}",
                report.file
            );
            assert_eq!(
                report.line, expected_line,
                "the report must name the line of the panic! call"
            );

            let literal = capture_panic(|| panic!("plain text")).unwrap_err();
            assert_eq!(
                literal.message, "plain text",
                "a literal message has a &'static str payload, a formatted one a String"
            );

            let index = capture_panic(|| {
                let items = [1, 2, 3];
                items[std::hint::black_box(7)]
            })
            .unwrap_err();
            assert!(
                index.message.contains("index out of bounds"),
                "panics raised by the standard library are captured too: {:?}",
                index.message
            );
        });
    }

    #[test]
    fn a_reports_other_payloads_and_thread_names() {
        isolated("a_reports_other_payloads_and_thread_names", || {
            let report = capture_panic(|| std::panic::panic_any(42_i32)).unwrap_err();
            assert_eq!(report.message, "<non-string payload>");

            let named = std::thread::Builder::new()
                .name("worker-7".to_owned())
                .spawn(|| capture_panic(|| panic!("boom")).unwrap_err())
                .unwrap()
                .join()
                .unwrap();
            assert_eq!(named.thread.as_deref(), Some("worker-7"));
            assert_eq!(named.message, "boom");
        });
    }

    #[test]
    fn a_parallel_calls_do_not_disturb_each_other() {
        isolated("a_parallel_calls_do_not_disturb_each_other", || {
            // Technique: eight threads panic at the same moment, fifty times. With one
            // shared "last report" variable, or with hooks that replace each other,
            // some thread gets another thread's report or none at all.
            let reports = crate::support::within(60, "eight threads of capture_panic", || {
                let barrier = Arc::new(Barrier::new(8));
                let workers: Vec<_> = (0..8)
                    .map(|id| {
                        let barrier = Arc::clone(&barrier);
                        std::thread::spawn(move || {
                            let mut messages = Vec::new();
                            for round in 0..50 {
                                barrier.wait();
                                let report = capture_panic(|| panic!("thread {id} round {round}"));
                                messages.push(report.unwrap_err().message);
                            }
                            messages
                        })
                    })
                    .collect();
                workers
                    .into_iter()
                    .map(|worker| worker.join().expect("a worker thread died"))
                    .collect::<Vec<_>>()
            });
            for (id, messages) in reports.iter().enumerate() {
                for (round, message) in messages.iter().enumerate() {
                    assert_eq!(
                        message,
                        &format!("thread {id} round {round}"),
                        "thread {id} received the report of a different panic"
                    );
                }
            }
        });
    }

    #[test]
    fn a_hook_passes_on_what_it_does_not_capture() {
        isolated("a_hook_passes_on_what_it_does_not_capture", || {
            use std::sync::Mutex;
            // Technique: this process has not called `capture_panic` yet, so the hook
            // set here is "the hook that was installed before yours". It records
            // every panic that reaches it.
            static REACHED: Mutex<Vec<String>> = Mutex::new(Vec::new());
            let reached = || REACHED.lock().unwrap().clone();
            std::panic::set_hook(Box::new(|info| {
                let message = info.payload_as_str().unwrap_or("<no text>").to_owned();
                REACHED.lock().unwrap().push(message);
            }));

            let report = capture_panic(|| panic!("captured")).unwrap_err();
            assert_eq!(report.message, "captured");
            let seen = reached();
            assert!(
                seen.is_empty(),
                "a captured panic must not be passed on to the earlier hook (that hook \
                 would print it), but the earlier hook saw {seen:?}"
            );

            let outside = std::thread::spawn(|| panic!("outside")).join();
            assert!(outside.is_err());
            let seen = reached();
            assert_eq!(
                seen,
                ["outside"],
                "a panic on a thread that is not inside capture_panic must be passed on \
                 to the hook that was installed before yours"
            );

            // A helper thread started inside `f` is not inside capture_panic either.
            let helper = capture_panic(|| std::thread::spawn(|| panic!("helper")).join().is_err());
            assert_eq!(helper, Ok(true));
            let seen = reached();
            assert_eq!(
                seen,
                ["outside", "helper"],
                "the decision is made per thread, at the moment of the panic"
            );
        });
    }

    #[test]
    fn a_other_threads_and_nested_calls() {
        isolated("a_other_threads_and_nested_calls", || {
            // A panic on a thread that `f` spawns is not `f` panicking: the spawned
            // thread is not inside capture_panic. (Its message is printed the normal
            // way, so you will see "expected: a helper thread panics" in the test output.)
            let outcome = capture_panic(|| {
                std::thread::spawn(|| panic!("expected: a helper thread panics"))
                    .join()
                    .is_err()
            });
            assert_eq!(
                outcome,
                Ok(true),
                "only panics of the calling thread are captured"
            );

            let nested = capture_panic(|| {
                let inner = capture_panic(|| panic!("inner")).unwrap_err();
                assert_eq!(inner.message, "inner");
                panic!("outer");
            })
            .unwrap_err();
            assert_eq!(
                nested.message, "outer",
                "after a nested call, the outer call must still capture its own panic"
            );

            // And afterwards the thread is back to normal.
            assert_eq!(capture_panic(|| 1), Ok(1));
        });
    }
}
