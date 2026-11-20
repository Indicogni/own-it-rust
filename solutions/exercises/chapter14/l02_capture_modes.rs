//! LESSON 02 — Mutation, move, and consuming captures
//! Run: cargo test --test chapter14 l02_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does move || text.len() consume text on
//! every call? What changes when the body returns text itself?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Which closures implement Fn, FnMut, and FnOnce? Explain why every Fn
//! closure also supports the weaker call traits.
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch14_scratch.rs):
//! Write move || owned_string and call it twice; record the moved-closure error. Change the
//! body to owned_string.len() and call twice. Capture a mutable counter without move and
//! inspect access while the closure is still used later.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete when its tests, predictions, and explanation are done. The lab is optional.

use supplied::Ledger;

mod supplied {
    //! Supplied code. Do not change it.

    /// A ledger that hides its entries. The only way to read them is to pass a
    /// closure to [`Ledger::each`].
    pub struct Ledger {
        entries: Vec<i32>,
    }

    impl Ledger {
        pub fn new(entries: &[i32]) -> Self {
            Ledger {
                entries: entries.to_vec(),
            }
        }

        /// Calls `visit` once per entry, in order.
        pub fn each(&self, mut visit: impl FnMut(i32)) {
            for &entry in &self.entries {
                visit(entry);
            }
        }
    }
}

/// A. Return the running total after each ledger entry, starting at initial.
/// `Ledger` hides its entries, so the only way in is `Ledger::each`: pass it a closure
/// that mutably borrows your running total and your output vector.
/// Inputs are small enough to fit i64.
fn running_totals(initial: i64, ledger: &Ledger) -> Vec<i64> {
    let mut total = initial;
    let mut totals = Vec::new();
    // No `move`: the closure mutably borrows `total` and `totals`. Both are
    // free again as soon as `each` returns, so `totals` can be returned.
    ledger.each(|entry| {
        total += i64::from(entry);
        totals.push(total);
    });
    totals
}
/// B. Return an owning, reusable closure whose result is prefix + argument.
fn owned_prefix(prefix: String) -> impl Fn(&str) -> String {
    // `move` puts the String inside the closure. The body only reads it, so
    // the closure is still `Fn` and can be called many times.
    move |suffix: &str| -> String { format!("{prefix}{suffix}") }
}
/// C. Return a closure that owns `text` and gives it back when called. The body
/// consumes the capture, so the closure can run only once (FnOnce). The String's
/// buffer moves into the closure and out again; nothing is copied.
fn give_once(text: String) -> impl FnOnce() -> String {
    // Returning `text` moves it out of the closure, so a call uses the closure
    // up: it is `FnOnce` only. (`move` is optional here, because a capture that
    // the body moves is always taken by value.)
    move || text
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::cell::RefCell;
    use std::mem::size_of_val;
    #[test]
    fn a_mutable_capture_tracks_state() {
        assert_eq!(running_totals(10, &Ledger::new(&[2, -5, 0])), [12, 7, 7]);
        assert!(running_totals(0, &Ledger::new(&[])).is_empty());
        assert_eq!(
            running_totals(0, &Ledger::new(&[i32::MAX, i32::MAX, i32::MIN])),
            [2_147_483_647, 4_294_967_294, 2_147_483_646]
        );
    }
    #[test]
    fn b_move_capture_can_implement_fn() {
        let f = owned_prefix("api/".into());
        assert_eq!(f("a"), "api/a");
        assert_eq!(f("b"), "api/b");
        assert_eq!(f(""), "api/");
        assert_eq!(owned_prefix(String::new())("🦀"), "🦀");
    }
    #[test]
    fn c_consuming_capture_transfers_the_allocation() {
        let text = String::from("owned");
        let ptr = text.as_ptr();
        let ((out, closure_size), allocations) = count(|| {
            let give = give_once(text);
            let closure_size = size_of_val(&give);
            (give(), closure_size)
        });
        assert_eq!(out, "owned");
        assert_eq!(
            closure_size,
            size_of::<String>(),
            "the closure's only state should be the String it owns"
        );
        assert_eq!(
            out.as_ptr(),
            ptr,
            "the closure must hand back the same buffer it was given, not a copy"
        );
        assert_eq!(
            allocations, 0,
            "moving a String into a closure and out again copies no text"
        );
    }

    struct Noisy<'a>(&'static str, &'a RefCell<Vec<&'static str>>);
    impl Noisy<'_> {
        fn label(&self) -> &'static str {
            self.0
        }
    }
    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(self.0);
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Without `move`, the closure mutably borrows `hits`. Value afterwards?
        let mut hits = 0;
        {
            let mut bump = || hits += 10;
            bump();
            bump();
        }
        let p1: i32 = 20;
        assert_eq!(p1, hits, "P1: a mutable borrow capture edits the original");

        // P2. With `move`, an i32 is copied into the closure. (third call's result, hits)?
        let mut hits = 0;
        let mut bump = move || {
            hits += 10;
            hits
        };
        bump();
        bump();
        let third = bump();
        let p2: (i32, i32) = (30, 0);
        assert_eq!(p2, (third, hits), "P2: move copies Copy values");

        // P3. A move closure that only reads its capture. When is the capture dropped?
        let log = RefCell::new(Vec::new());
        let token = Noisy("token dropped", &log);
        let read = move || token.label().len();
        read();
        read();
        log.borrow_mut().push("after calls");
        drop(read);
        log.borrow_mut().push("after drop(read)");
        let p3: Vec<&str> = vec!["after calls", "token dropped", "after drop(read)"];
        assert_eq!(
            p3,
            *log.borrow(),
            "P3: a capture lives as long as the closure"
        );

        // P4. Now the body consumes the capture. When is it dropped this time?
        let log = RefCell::new(Vec::new());
        let token = Noisy("token dropped", &log);
        let consume = move || drop(token);
        log.borrow_mut().push("before call");
        consume();
        log.borrow_mut().push("after call");
        let p4: Vec<&str> = vec!["before call", "token dropped", "after call"];
        assert_eq!(p4, *log.borrow(), "P4: an FnOnce body can drop its capture");
    }
}
