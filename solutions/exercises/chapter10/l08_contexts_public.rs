//! LESSON 08 — Pattern contexts and public compatibility
//! Run: cargo test --test chapter10 l08_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Can `let` bind a refutable `Some(..)` pattern without an `else`?
//! 2. Why must a match on another crate's `#[non_exhaustive]` enum include a
//!    wildcard arm?
//!
//! IMPLEMENT the tasks below. Keep the tests and add one edge case.
//!
//! EXPLAIN afterward: List pattern contexts: let, function parameters, for,
//! match, if let, while let, and let else. Explain when each permits refutable
//! patterns.
//!
//! COMPILER / TRANSFER LAB (optional, not checked by tests): Follow the
//! two-package `#[non_exhaustive]` lab ("Optional lab") in this lesson's
//! section of CONCEPTS.md. It needs two packages, so it is not in this suite.
//! Afterwards explain why you should not add a wildcard arm to a closed enum
//! of your own "just in case".
//! Use examples/ch10_scratch.rs for single-file experiments, and restore it to
//! compiling code afterwards.

/// A. Pop all owned strings using while let Some(value), returning them in
/// reverse input order, in a new vector. Do not clone, index, or call reverse.
fn drain_back(mut values: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(values.len());
    while let Some(value) = values.pop() {
        out.push(value);
    }
    out
}
/// B. Use let else with tuple destructuring. Require a '=' and a nonempty
/// left half; return the borrowed halves unchanged, with no trimming.
#[allow(clippy::question_mark)] // `?` would also work; this task practices let else
fn assignment(text: &str) -> Option<(&str, &str)> {
    let Some((key, value)) = text.split_once('=') else {
        return None;
    };
    if key.is_empty() {
        return None;
    }
    Some((key, value))
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, within};
    #[test]
    fn a_repeated_pattern_moves_payloads() {
        // A loop that never removes an element would run forever, so every
        // call runs under a time limit and fails with a message instead.
        let small = within(3, "drain_back on three strings", || {
            drain_back(vec!["a".into(), "b".into(), "c".into()])
        });
        assert_eq!(small, ["c", "b", "a"]);
        let none = within(3, "drain_back on an empty vector", || drain_back(vec![]));
        assert!(none.is_empty());
    }
    #[test]
    fn a_strings_move_into_a_new_vector() {
        let values: Vec<String> = (0..100).map(|i| format!("v{i}")).collect();
        // Addresses are kept as plain numbers so they can be compared after
        // `values` has moved to the helper thread.
        let first_text = values[0].as_ptr() as usize;
        let input_buffer = values.as_ptr() as usize;
        let (out, allocations) = within(3, "drain_back on 100 strings", move || {
            count(|| drain_back(values))
        });
        assert_eq!((out[0].as_str(), out[99].as_str()), ("v99", "v0"));
        assert_eq!(
            out[99].as_ptr() as usize,
            first_text,
            "each String must move to the output with its buffer; it must not be cloned"
        );
        assert_ne!(
            out.as_ptr() as usize,
            input_buffer,
            "the result is the input vector itself, reordered in place; \
             this task asks you to pop each value into a new vector"
        );
        assert!(
            allocations <= 10,
            "moving 100 Strings cost {allocations} allocations; only the output \
             vector may allocate while it grows"
        );
    }
    #[test]
    fn b_refutable_let_has_a_diverging_else() {
        assert_eq!(assignment("x=y=z"), Some(("x", "y=z")));
        assert_eq!(assignment("x="), Some(("x", "")));
        assert_eq!(assignment("=y"), None);
        assert_eq!(assignment("none"), None);
        assert_eq!(assignment(" x = y "), Some((" x ", " y ")));
        assert_eq!(assignment("="), None);
        assert_eq!(assignment(""), None);
        assert_eq!(assignment("ключ=значение"), Some(("ключ", "значение")));
        let text = String::from("k=v");
        let (k, v) = assignment(&text).unwrap();
        assert_eq!(
            (k.as_ptr(), v.as_ptr()),
            (text.as_ptr(), text[2..].as_ptr()),
            "both halves must be views into the input text, not new strings"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. let else inside a loop: what is collected?
        let mut parsed = Vec::new();
        for line in ["a=1", "b", "=2", "c=x"] {
            let Some((_, value)) = line.split_once('=') else {
                continue;
            };
            let Ok(n) = value.parse::<i32>() else {
                parsed.push(-1);
                continue;
            };
            parsed.push(n);
        }
        let p1: Vec<i32> = vec![1, 2, -1];
        assert_eq!(p1, parsed, "P1: let else diverges");

        // P2. A let chain (Rust 2024): which branch runs?
        let a = Some(5);
        let b: Result<i32, ()> = Err(());
        let r = if let Some(x) = a
            && x > 3
            && let Ok(y) = b
        {
            x + y
        } else if let Some(x) = a {
            x * 100
        } else {
            0
        };
        let p2: i32 = 500;
        assert_eq!(p2, r, "P2: every condition in a chain must hold");
    }
}
