//! LESSON 01 — Destruction and resource lifetimes
//! Run: cargo test --test chapter13 l01_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: In what order are local guards destroyed?
//! Does moving a guard create another destructor call? What does drop(guard) consume?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Explain why calling .drop() directly is prohibited and why
//! std::mem::drop is an ordinary generic function. When does forgetting a value leak
//! resources?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch13_scratch.rs):
//! Create two guards in a nested scope and print their drop order. Move one into a new
//! binding and check that only one line is printed for it. Then move one field out of a
//! two-field struct and watch which field is still dropped at the end of the scope.
//! Lesson 09 tests drop order during a panic. The provided exercise Guard skips its
//! unfinished helper during unwinding so that a `todo!()` cannot cause a second panic;
//! that check is scaffold protection, not general Drop behavior.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete when its tests, predictions, and explanation are done. The lab is optional.

use std::cell::RefCell;
use std::rc::Rc;
type Log = Rc<RefCell<Vec<String>>>;
/// A. Record exactly one label. The Drop scaffold below calls this helper.
fn record_drop(log: &Log, label: &str) {
    todo!("Task 1: implement the documented contract")
}
struct Guard {
    label: &'static str,
    log: Log,
}
impl Drop for Guard {
    fn drop(&mut self) {
        // Keeps unfinished TODO helpers from causing a second panic during unwinding.
        if !std::thread::panicking() {
            record_drop(&self.log, self.label);
        }
    }
}
/// B. Build three guards that share `log` (clone the Rc for each; never push to the
/// log yourself):
/// 1. create "outer";
/// 2. in a nested block, create "early" and then "inner";
/// 3. still inside that block, drop "early" explicitly and then call `checkpoint()`;
/// 4. let both scopes end.
///
/// Afterwards the log reads ["early", whatever `checkpoint` logged, "inner", "outer"].
fn scoped_order(log: &Log, checkpoint: impl FnOnce()) {
    todo!("Task 2: implement the documented contract")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;
    #[test]
    fn a_destructor_records_one_event_per_owner() {
        let log = Rc::new(RefCell::new(vec![]));
        {
            let first = Guard {
                label: "one",
                log: Rc::clone(&log),
            };
            let moved = first;
            drop(moved);
        }
        assert_eq!(
            *log.borrow(),
            ["one"],
            "a move transfers the one owner; the destructor runs once, for the final owner"
        );
    }
    #[test]
    fn b_scope_and_explicit_drop_define_event_order() {
        let log: Log = Rc::new(RefCell::new(vec![]));
        let seen = Rc::clone(&log);
        scoped_order(&log, move || {
            // Owners of the log right now: the test, this closure, "inner", and "outer".
            assert_eq!(
                Rc::strong_count(&seen),
                4,
                "at the checkpoint exactly two guards (inner and outer) must be alive: \
                 build real Guard values, and drop `early` before calling checkpoint()"
            );
            seen.borrow_mut().push("checkpoint".to_string());
        });
        assert_eq!(
            *log.borrow(),
            ["early", "checkpoint", "inner", "outer"],
            "early is dropped explicitly, then inner and outer when their scopes end"
        );
        assert_eq!(
            Rc::strong_count(&log),
            1,
            "every guard must be dropped by the time scoped_order returns"
        );
    }

    struct Noisy<'a>(&'static str, &'a RefCell<Vec<&'static str>>);
    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(self.0);
        }
    }

    struct Pair<'a> {
        first: Noisy<'a>,
        second: Noisy<'a>,
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Locals drop in reverse order. In what order do a struct's fields drop?
        let log = RefCell::new(Vec::new());
        drop(Pair {
            first: Noisy("first", &log),
            second: Noisy("second", &log),
        });
        let p1: Vec<&str> = guess();
        assert_eq!(
            p1,
            *log.borrow(),
            "P1: field drop order (left: your prediction, right: the real log)"
        );

        // P2. `let _ = ...` vs `let _name = ...`.
        let log = RefCell::new(Vec::new());
        {
            let _ = Noisy("underscore", &log);
            log.borrow_mut().push("middle");
            let _named = Noisy("named", &log);
            log.borrow_mut().push("end");
        }
        let p2: Vec<&str> = guess();
        assert_eq!(
            p2,
            *log.borrow(),
            "P2: `_` does not bind (left: your prediction, right: the real log)"
        );

        // P3. Assigning to an initialized variable. What is logged BEFORE the block ends?
        let log = RefCell::new(Vec::new());
        let mut slot = Noisy("old", &log);
        assert_eq!(slot.0, "old");
        slot = Noisy("new", &log);
        let p3: Vec<&str> = guess();
        assert_eq!(
            p3,
            *log.borrow(),
            "P3: assignment drops the old value (left: your prediction, right: the real log)"
        );
        drop(slot);

        // P4. mem::forget: how many destructor calls are logged?
        let log = RefCell::new(Vec::new());
        std::mem::forget(Noisy("forgotten", &log));
        let p4: usize = guess();
        assert_eq!(
            p4,
            log.borrow().len(),
            "P4: forget skips Drop (left: your prediction, right: the real count)"
        );
    }
}
