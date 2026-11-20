//! LESSON 03 — Moves and control flow
//! Run: cargo test --test chapter04 l03_
//! Read first: "Lesson 03" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Can a value that is moved in only one branch be used after the `if`?
//! 2. What changes if every branch gives the variable a new value?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain how the rule "a variable must hold a value when
//! it is used" applies to every path through an `if` and to every loop
//! iteration.
//!
//! COMPILER / TRANSFER LAB (optional): Inside a loop, move a String into
//! another binding without giving the variable a new value, and read the error
//! about the next iteration. Repair it by assigning a returned owner back on
//! every iteration. Then compare an `if` branch that assigns a new value with
//! one that does not.
//! Use examples/ch04_scratch.rs for this, and leave it compiling afterward.

/// A. Consume both alternatives and return exactly one without cloning.
fn choose(first: String, second: String, use_first: bool) -> String {
    // Each branch moves a different input out. The other one is dropped when
    // the function returns.
    if use_first { first } else { second }
}
/// B. Pass ownership through a helper once per suffix; append suffixes in order.
/// The helper must take and return String. Use a loop and reassign `value`.
/// Keep using the buffer you were given: do not clone or rebuild the String.
fn pipeline(mut value: String, suffixes: &[&str]) -> String {
    for suffix in suffixes {
        // `append` takes ownership of `value`, so `value` is empty (moved)
        // until the result is assigned back to it.
        value = append(value, suffix);
    }
    value
}

/// Takes ownership of `text`, appends `suffix`, and hands `text` back.
fn append(mut text: String, suffix: &str) -> String {
    text.push_str(suffix);
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::droplog::DropLog;
    use practice::alloc::count;
    #[test]
    fn a_branch_returns_one_owner() {
        assert_eq!(choose("left".into(), "right".into(), true), "left");
        assert_eq!(choose("left".into(), "right".into(), false), "right");
    }
    #[test]
    fn b_loop_reinitializes_owner() {
        assert_eq!(pipeline("go".into(), &["-", "🦀", "!"]), "go-🦀!");
        assert_eq!(pipeline("keep".into(), &[]), "keep");
    }

    #[test]
    fn a_choose_does_not_clone() {
        let (left, right) = ("left".to_owned(), "right".to_owned());
        let chosen = right.as_ptr();
        let (out, allocations) = count(|| choose(left, right, false));
        assert_eq!(
            out.as_ptr(),
            chosen,
            "choose must return the String it was given (same heap buffer), not a copy"
        );
        assert_eq!(
            allocations, 0,
            "choose made {allocations} heap allocation(s); returning one of its inputs needs none"
        );
    }

    #[test]
    fn b_pipeline_keeps_one_buffer() {
        let mut value = String::with_capacity(32);
        value.push_str("go");
        let start = value.as_ptr();
        let (out, allocations) = count(|| pipeline(value, &["-", "fast", "!"]));
        assert_eq!(out, "go-fast!");
        assert_eq!(
            out.as_ptr(),
            start,
            "pipeline must keep appending to the buffer it was given: move `value` \
             through the helper and back, do not clone it or build a new String"
        );
        assert_eq!(
            allocations, 0,
            "pipeline made {allocations} heap allocation(s); the buffer already has room \
             for every suffix, so moving it around needs none"
        );
    }

    fn pick(log: &DropLog, take_a: bool) {
        let a = log.noisy("a");
        let b = log.noisy("b");
        let kept = if take_a { a } else { b };
        log.note("end");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// `log.noisy("x")` makes a value that writes "x" into `log` at the moment
    /// it is dropped, and `log.note("end")` writes "end" immediately. Read
    /// `pick` above, then predict the whole log for each call.
    #[test]
    fn p_predictions() {
        // P1. The log after `pick` moved `a` into `kept`.
        let log = DropLog::new();
        pick(&log, true);
        let p1: Vec<&str> = vec!["end", "a", "b"];
        assert_eq!(
            p1,
            log.entries(),
            "P1: log when `a` is the one moved (left: your prediction, right: actual)"
        );

        // P2. The log after `pick` moved `b` into `kept` instead.
        let log = DropLog::new();
        pick(&log, false);
        let p2: Vec<&str> = vec!["end", "b", "a"];
        assert_eq!(
            p2,
            log.entries(),
            "P2: log when `b` is the one moved (left: your prediction, right: actual)"
        );
    }
}
