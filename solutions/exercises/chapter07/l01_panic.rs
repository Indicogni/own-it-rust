//! LESSON 01 — Panics and invariants
//! Run: cargo test --test chapter07 l01_
//!
//! Read first: CONCEPTS.md, section "Lesson 01".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Do the destructors of local values run while a panic unwinds?
//! 2. Do they run when the program is built with panic=abort?
//! 3. Can catch_unwind stop an abort?
//!
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain why panic is appropriate for the stated
//! invariant, and why ordinary missing input should use Result instead.
//!
//! OPTIONAL LABS: LABS.md, lab 1 (unwinding versus abort) and lab 2 (panic
//! hooks and the other panicking macros). Never change the panic hook or the
//! abort setting inside the chapter tests: they run in parallel.

/// A. Require a strictly positive capacity, panicking with EXACT message
/// "capacity must be positive" when zero. Return the input otherwise.
fn require_capacity(capacity: usize) -> usize {
    // A zero capacity is a caller bug, not an input to recover from.
    assert!(capacity > 0, "capacity must be positive");
    capacity
}
/// B. Expected invalid input should return Err("empty label"). Trim text and
/// return the borrowed label on success; never panic for an empty label.
fn label(text: &str) -> Result<&str, &'static str> {
    let label = text.trim();
    if label.is_empty() {
        Err("empty label")
    } else {
        Ok(label)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    #[test]
    fn a_positive_capacity_is_accepted() {
        assert_eq!(require_capacity(1), 1);
        assert_eq!(require_capacity(99), 99);
        assert_eq!(require_capacity(usize::MAX), usize::MAX);
    }
    #[test]
    #[should_panic(expected = "capacity must be positive")]
    fn a_zero_violates_the_invariant() {
        require_capacity(0);
    }
    #[test]
    fn b_empty_label_is_a_recoverable_error() {
        assert_eq!(label(" 港 "), Ok("港"));
        assert_eq!(label(" \n"), Err("empty label"));
        assert_eq!(label(""), Err("empty label"));
        assert_eq!(label("\u{3000}"), Err("empty label"));
    }
    #[test]
    fn b_label_borrows_from_the_input() {
        let text = String::from("  route  ");
        let out = label(&text).unwrap();
        assert_eq!(
            out.as_ptr() as usize,
            text.as_ptr() as usize + 2,
            "the label must point into the caller's text (2 bytes in), not at a copy"
        );
    }

    struct Noisy<'a>(&'static str, &'a RefCell<Vec<&'static str>>);
    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(self.0);
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// The panics below are caught; their messages in the test output are expected.
    #[test]
    fn p_predictions() {
        // P1. Do destructors run while a panic unwinds? In what order?
        let log = RefCell::new(Vec::new());
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _first = Noisy("first", &log);
            let _second = Noisy("second", &log);
            panic!("boom");
        }));
        assert!(
            result.is_err(),
            "catch_unwind returns Err for a caught panic"
        );
        let p1: Vec<&str> = vec!["second", "first"];
        assert_eq!(p1, *log.borrow(), "P1: unwinding drops locals");

        // P2. Is each panic payload a String? (A literal message, and a message
        // that formats the run-time value `n`.)
        let n = 7;
        let literal = catch_unwind(|| panic!("boom")).unwrap_err();
        let formatted = catch_unwind(|| panic!("code {n}")).unwrap_err();
        let p2: (bool, bool) = (false, true);
        assert_eq!(
            p2,
            (literal.is::<String>(), formatted.is::<String>()),
            "P2: panic payload types"
        );
    }
}
