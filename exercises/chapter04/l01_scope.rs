//! LESSON 01 — Ownership scopes
//! Run: cargo test --test chapter04 l01_
//! Read first: "Lesson 01" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Does dropping a `Vec<String>` also drop its elements?
//! 2. Does a move allocate a new string buffer?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Draw the owner tree for a vector of strings and explain
//! when each part of it is destroyed.
//!
//! COMPILER / TRANSFER LAB (optional): Create a String inside a nested block
//! and try to use it after the block. Read the error. Repair the program by
//! moving the String into a binding that is declared before the block, then
//! draw the owner tree of the repaired program.
//! Use examples/ch04_scratch.rs for this, and leave it compiling afterward.

/// A. Build two owned labels, "north" and "south", inside a nested block;
/// return their vector so its ownership leaves the block.
fn labels() -> Vec<String> {
    todo!("Implement this task")
}
/// B. Consume the String and return it unchanged. Do not clone it.
fn handoff(value: String) -> String {
    todo!("Implement this task")
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::droplog::DropLog;
    use practice::guess;
    #[test]
    fn a_nested_scope_returns_owners() {
        assert_eq!(labels(), ["north", "south"]);
    }
    #[test]
    fn b_handoff_preserves_allocation() {
        let s = String::from("🦀 route");
        let p = s.as_ptr();
        let out = handoff(s);
        assert_eq!(out, "🦀 route");
        assert_eq!(
            out.as_ptr(),
            p,
            "handoff must return the same heap buffer it was given, not a copy: \
             move `value` out, do not clone or rebuild it"
        );
        assert_eq!(handoff(String::new()), "");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    ///
    /// `log.noisy("x")` makes a value that writes "x" into `log` at the moment
    /// it is dropped. `log.entries()` returns what has been written so far.
    #[test]
    fn p_predictions() {
        // P1. In what order are these two owners dropped?
        let log = DropLog::new();
        {
            let _first = log.noisy("first");
            let _second = log.noisy("second");
        }
        let p1: Vec<&str> = guess();
        assert_eq!(
            p1,
            log.entries(),
            "P1: drop order at the end of a block (left: your prediction, right: actual)"
        );

        // P2. Dropping a vector of two noisy values: what does the log hold?
        let log = DropLog::new();
        drop(vec![log.noisy("x"), log.noisy("y")]);
        let p2: Vec<&str> = guess();
        assert_eq!(
            p2,
            log.entries(),
            "P2: log after dropping a vector (left: your prediction, right: actual)"
        );

        // P3. After `let w = v;`, does w's first String use the same heap buffer
        // that v's first String used? Answer true or false.
        let v = vec![String::from("north")];
        let before = v[0].as_ptr();
        let w = v;
        let p3: bool = guess();
        assert_eq!(
            p3,
            w[0].as_ptr() == before,
            "P3: same buffer after moving the vector? (left: your prediction, right: actual)"
        );
    }
}
