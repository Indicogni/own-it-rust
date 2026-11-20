//! LESSON 09 — Drop order: unwinding and temporaries
//! Run: cargo test --test chapter13 l09_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: When a panic unwinds, which values are
//! dropped, and in what order? How long does a temporary such as `cell.borrow_mut()` live
//! in a `match`, in an `if let`, and at the end of a block?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why does a guard type restore state reliably when plain code after
//! the call does not? Where does a temporary end in a `let` statement, in a `match`, in an
//! `if let` with an `else`, and in the last expression of a block?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch13_scratch.rs):
//! Write a Drop impl that panics, create one value of it, and panic while it is alive.
//! Read how the process ends and why no later destructor runs. Then compile the `if let`
//! and tail-expression predictions below with `rustc --edition 2021` and compare the logs.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete when its tests, predictions, and explanation are done. The lab is optional.
//!
//! Some tests panic on purpose and catch the panic. Lines such as "deliberate test panic"
//! in the output of a failing test are expected.

use std::cell::{Cell, RefCell};

type Log = RefCell<Vec<&'static str>>;

/// Provided. Records its label in the log when it is dropped. Do not edit.
struct Tracer<'a>(&'static str, &'a Log);
impl Drop for Tracer<'_> {
    fn drop(&mut self) {
        // try_borrow_mut never panics, so this destructor is safe during unwinding.
        if let Ok(mut log) = self.1.try_borrow_mut() {
            log.push(self.0);
        }
    }
}

/// A. Set `level` to `temporary`, run `work`, then put the old value back and return
/// what `work` returned. The old value must also come back when `work` panics, and the
/// panic must continue to the caller: do not catch it. Write a small guard type with a
/// `Drop` impl (you may declare it inside this function).
fn with_level<R>(level: &Cell<u8>, temporary: u8, work: impl FnOnce() -> R) -> R {
    /// Puts the saved level back when dropped: on a normal return and during unwinding.
    struct Restore<'a> {
        level: &'a Cell<u8>,
        old: u8,
    }
    impl Drop for Restore<'_> {
        fn drop(&mut self) {
            self.level.set(self.old);
        }
    }

    // A named binding keeps the guard alive until the function ends. `let _ = ...`
    // would drop it at once.
    let _restore = Restore {
        level,
        old: level.replace(temporary),
    };
    work()
}

/// B. Run `work`. When this function ends, the log must gain "flush" and then "unlock",
/// in that order, after everything `work` logged. This must hold when `work` returns and
/// when `work` panics (do not catch the panic). Use two `Tracer`s; do not push to the
/// log yourself.
fn flush_then_unlock(log: &Log, work: impl FnOnce()) {
    // Locals drop in reverse declaration order, so the tracer declared last ("flush")
    // is dropped first. The same order applies when `work` panics.
    let _unlock = Tracer("unlock", log);
    let _flush = Tracer("flush", log);
    work();
}

/// C. Pop and return the top value. If the stack is empty, return `on_empty()` instead.
/// `on_empty` is allowed to borrow the stack itself, so no guard of yours may be alive
/// while it runs.
fn pop_or_else(stack: &RefCell<Vec<u32>>, on_empty: impl FnOnce() -> u32) -> u32 {
    // The guard from `borrow_mut()` is a temporary of this `let` statement and is
    // dropped at its semicolon, before `on_empty` can run.
    let top = stack.borrow_mut().pop();
    top.unwrap_or_else(on_empty)
}

/// D. If the stack has a top value, pass a copy of it to `visit` and return true. The
/// stack keeps the value. Return false for an empty stack. `visit` is allowed to borrow
/// the stack mutably, so no guard of yours may be alive while it runs.
fn visit_top(stack: &RefCell<Vec<u32>>, visit: impl FnOnce(u32)) -> bool {
    // `copied()` turns the `Option<&u32>` into a plain `Option<u32>`, so nothing
    // borrows from the guard after this statement.
    let top = stack.borrow().last().copied();
    let Some(top) = top else {
        return false;
    };
    visit(top);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{self, AssertUnwindSafe};

    #[test]
    fn a_level_is_temporary_while_work_runs() {
        let level = Cell::new(1);
        assert_eq!(with_level(&level, 7, || level.get() * 2), 14);
        assert_eq!(level.get(), 1, "the old level must be back after the call");
        let nested = with_level(&level, 5, || {
            with_level(&level, 6, || level.get()) + level.get()
        });
        assert_eq!(
            nested, 11,
            "the inner call must restore 5 before the outer work reads it"
        );
        assert_eq!(
            level.get(),
            1,
            "the old level must be back after nested calls"
        );
    }

    /// Runs `f`, which must end with the tests' own "deliberate test panic". Any other
    /// panic, such as an unfinished `todo!()`, continues unchanged.
    fn expect_test_panic(f: impl FnOnce()) {
        let payload = panic::catch_unwind(AssertUnwindSafe(f))
            .expect_err("the panic must continue to the caller: do not catch it");
        if payload.downcast_ref::<&str>() != Some(&"deliberate test panic") {
            panic::resume_unwind(payload);
        }
    }

    #[test]
    fn a_level_is_restored_while_a_panic_unwinds() {
        let level = Cell::new(1);
        expect_test_panic(|| {
            with_level(&level, 7, || -> u8 { panic!("deliberate test panic") });
        });
        assert_eq!(
            level.get(),
            1,
            "the old level must be restored during unwinding: a panic skips the code \
             after `work()`, but it still runs destructors"
        );
    }

    #[test]
    fn b_cleanup_runs_in_order_after_normal_work() {
        let log = Log::default();
        flush_then_unlock(&log, || log.borrow_mut().push("work"));
        assert_eq!(
            *log.borrow(),
            ["work", "flush", "unlock"],
            "locals drop in reverse declaration order"
        );
    }

    #[test]
    fn b_cleanup_runs_in_order_while_a_panic_unwinds() {
        let log = Log::default();
        expect_test_panic(|| {
            flush_then_unlock(&log, || {
                let _local = Tracer("work local", &log);
                panic!("deliberate test panic");
            });
        });
        assert_eq!(
            *log.borrow(),
            ["work local", "flush", "unlock"],
            "unwinding drops the inner call's values first, then this function's \
             tracers in reverse declaration order; pushes written after `work()` never run"
        );
    }

    /// Runs a task. A panic caused by borrowing a RefCell twice becomes an explanation.
    fn explain_double_borrow<R>(task: &str, f: impl FnOnce() -> R) -> R {
        match panic::catch_unwind(AssertUnwindSafe(f)) {
            Ok(value) => value,
            Err(payload) => {
                let double_borrow = payload
                    .downcast_ref::<String>()
                    .is_some_and(|message| message.contains("already"));
                if double_borrow {
                    panic!(
                        "{task}: a guard from borrow() or borrow_mut() was still alive when \
                         the closure borrowed the stack. A temporary guard lives to the end \
                         of its statement, and in a `match` or `if let` to the end of the \
                         arm or block. Store the plain value with `let` first."
                    );
                }
                panic::resume_unwind(payload)
            }
        }
    }

    #[test]
    fn c_pop_returns_the_top_and_keeps_the_rest() {
        let stack = RefCell::new(vec![1, 2]);
        assert_eq!(pop_or_else(&stack, || 99), 2);
        assert_eq!(*stack.borrow(), [1]);
        assert_eq!(pop_or_else(&stack, || 99), 1);
        assert_eq!(pop_or_else(&stack, || 99), 99);
        assert!(stack.borrow().is_empty());
    }

    #[test]
    fn c_no_guard_is_alive_while_on_empty_runs() {
        let stack = RefCell::new(Vec::new());
        let got = explain_double_borrow("pop_or_else", || {
            pop_or_else(&stack, || {
                stack.borrow_mut().extend([3, 2, 1]);
                0
            })
        });
        assert_eq!(got, 0);
        assert_eq!(*stack.borrow(), [3, 2, 1]);
    }

    #[test]
    fn d_visit_sees_a_copy_of_the_top() {
        let stack = RefCell::new(vec![4, 9]);
        let seen = Cell::new(0);
        assert!(visit_top(&stack, |top| seen.set(top)));
        assert_eq!(seen.get(), 9);
        assert_eq!(
            *stack.borrow(),
            [4, 9],
            "visit_top must not remove the value"
        );
        let empty = RefCell::new(Vec::new());
        assert!(!visit_top(&empty, |_| panic!(
            "an empty stack has no top to visit"
        )));
    }

    #[test]
    fn d_no_guard_is_alive_while_visit_runs() {
        let stack = RefCell::new(vec![4, 9]);
        let visited = explain_double_borrow("visit_top", || {
            visit_top(&stack, |top| stack.borrow_mut().push(top * 2))
        });
        assert!(visited);
        assert_eq!(*stack.borrow(), [4, 9, 18]);
    }

    impl Tracer<'_> {
        /// Returns `answer` unchanged. Only the lifetime of `self` matters.
        fn ask(&self, answer: Option<&'static str>) -> Option<&'static str> {
            answer
        }
    }

    struct Pair<'a> {
        first: Tracer<'a>,
        second: Tracer<'a>,
    }

    fn make<'a>(label: &'static str, log: &'a Log, explode: bool) -> Tracer<'a> {
        if explode {
            panic!("deliberate test panic");
        }
        Tracer(label, log)
    }

    fn deep(log: &Log, explode: bool) {
        let _c = Tracer("c", log);
        let _d = Tracer("d", log);
        if explode {
            panic!("deliberate test panic");
        }
    }

    fn tail_len(log: &Log) -> usize {
        let _local = Tracer("local", log);
        Tracer("tail", log).0.len()
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. A panic two calls deep. Which labels are logged, in which order?
        let log = Log::default();
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let _a = Tracer("a", &log);
            let _b = Tracer("b", &log);
            deep(&log, true);
            let _never = Tracer("never", &log);
        }));
        assert!(result.is_err());
        log.borrow_mut().push("caught");
        let p1: Vec<&str> = vec!["d", "c", "b", "a", "caught"];
        assert_eq!(
            p1,
            *log.borrow(),
            "P1: unwinding across two frames (left: your prediction, right: the real log)"
        );

        // P2. The panic happens while the second field of `Pair` is being built. One
        // tracer was moved into `kept` earlier.
        let log = Log::default();
        let mut kept = Vec::new();
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let moved = Tracer("moved", &log);
            let _stays = Tracer("stays", &log);
            kept.push(moved);
            let _pair = Pair {
                first: make("first", &log, false),
                second: make("second", &log, true),
            };
        }));
        assert!(result.is_err());
        log.borrow_mut().push("caught");
        drop(kept);
        let p2: Vec<&str> = vec!["first", "stays", "caught", "moved"];
        assert_eq!(
            p2,
            *log.borrow(),
            "P2: unwinding drops only what exists and was not moved away \
             (left: your prediction, right: the real log)"
        );

        // P3. Temporaries: in a plain statement, behind `let x = &...`, and in the last
        // expression of a function body (see `tail_len` above; edition 2024).
        let log = Log::default();
        {
            let _n = Tracer("statement", &log).0.len();
            log.borrow_mut().push("next line");
            let extended = &Tracer("extended", &log);
            let _m = tail_len(&log);
            log.borrow_mut().push("block end");
            let _label = extended.0;
        }
        let p3: Vec<&str> = vec![
            "statement",
            "next line",
            "tail",
            "local",
            "block end",
            "extended",
        ];
        assert_eq!(
            p3,
            *log.borrow(),
            "P3: where temporaries end (left: your prediction, right: the real log)"
        );

        // P4. A temporary in a `match` scrutinee and in two `if let` conditions
        // (edition 2024).
        let log = Log::default();
        match Tracer("match temp", &log).ask(Some("match arm")) {
            Some(arm) => log.borrow_mut().push(arm),
            None => log.borrow_mut().push("no arm"),
        }
        if let Some(text) = Tracer("first if-let temp", &log).ask(Some("then block")) {
            log.borrow_mut().push(text);
        }
        if let Some(text) = Tracer("second if-let temp", &log).ask(None) {
            log.borrow_mut().push(text);
        } else {
            log.borrow_mut().push("else block");
        }
        let p4: Vec<&str> = vec![
            "match arm",
            "match temp",
            "then block",
            "first if-let temp",
            "second if-let temp",
            "else block",
        ];
        assert_eq!(
            p4,
            *log.borrow(),
            "P4: scrutinee temporaries (left: your prediction, right: the real log)"
        );
    }
}
