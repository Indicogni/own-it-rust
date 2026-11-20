//! LESSON 07 — Impossible errors and deliberate recovery
//! Run: cargo test --test chapter07 l07_
//! Read first: CONCEPTS.md, section "Lesson 07".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. When is expect justified?
//! 2. Does assigning a Result to _ mean the operation succeeded?
//! 3. Why does an unused Result produce a warning?
//!
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain the documented proof behind expect and the
//! information intentionally discarded by a recovery policy.
//!
//! COMPILER LAB (scratch example): call a fallible function as a bare
//! statement and read the must_use warning. Then write `let _ =` in front of
//! it: the warning goes away, the failure does not. Drill d03 is the same
//! experiment.

/// A. `literal` is a limit written in this program's own source code (the
/// tests pass entries of a built-in table), never text from a user. Parse it
/// as u16 with `expect`, using EXACTLY the message
/// "built-in limit must be a u16 literal". A bad entry is a bug in the
/// table, so the right response is a panic that names the broken promise;
/// returning a default would hide the bug.
fn built_in_limit(literal: &'static str) -> u16 {
    todo!("Implement this task")
}
/// B. Parse every trimmed i16 input; return (successful values, error count).
/// This API deliberately recovers from bad rows instead of stopping early.
fn recover_rows(rows: &[&str]) -> (Vec<i16>, usize) {
    todo!("Implement this task")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;
    use std::cell::RefCell;
    use std::panic::catch_unwind;
    #[test]
    fn a_built_in_literals_parse() {
        assert_eq!(built_in_limit("64"), 64);
        assert_eq!(built_in_limit("0"), 0);
        assert_eq!(built_in_limit("65535"), u16::MAX);
    }
    /// The panic message this test prints is expected: the panic is caught.
    #[test]
    fn a_broken_literal_panics_and_says_why() {
        let payload = catch_unwind(|| built_in_limit("sixty-four"))
            .expect_err("a bad built-in literal is a bug: it must panic, not return a default");
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("");
        assert!(
            message.contains("built-in limit must be a u16 literal"),
            "the panic message must state the broken promise; got {message:?}"
        );
        assert!(
            message.contains("InvalidDigit"),
            "expect adds the underlying error to your message; got {message:?}"
        );
    }
    #[test]
    fn b_recovery_reports_information_loss() {
        assert_eq!(
            recover_rows(&["1", "bad", " -2 ", "40000"]),
            (vec![1, -2], 2)
        );
        assert_eq!(recover_rows(&[]), (vec![], 0));
        assert_eq!(recover_rows(&["bad"]), (vec![], 1));
        assert_eq!(
            recover_rows(&["3", "", "1", "-32768"]),
            (vec![3, 1, -32768], 1)
        );
    }

    struct Noisy<'a>(&'static str, &'a RefCell<Vec<&'static str>>);
    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(self.0);
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. `let _ =` does not bind; `let _b =` does. When does each value drop?
        let log = RefCell::new(Vec::new());
        {
            let _ = Noisy("a", &log);
            log.borrow_mut().push("mid");
            let _b = Noisy("b", &log);
            log.borrow_mut().push("end");
        }
        let p1: Vec<&str> = guess();
        assert_eq!(p1, *log.borrow(), "P1: _ discards immediately");

        // P2. ok() silently discards every error, including overflow.
        let rows = ["5", "300", "-2", "x"];
        let sum: i8 = rows.iter().filter_map(|r| r.parse::<i8>().ok()).sum();
        let p2: i8 = guess();
        assert_eq!(p2, sum, "P2: filter_map with ok()");
    }
}
