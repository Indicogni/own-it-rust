//! LESSON 05 — Leaving a replacement behind
//! Run: cargo test --test chapter04 l05_
//! Read first: "Lesson 05" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. What remains in a String after `mem::take`?
//! 2. Why does putting a new value into a place let the old value leave it?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Describe the rule that `replace` and `take` keep true,
//! and say what the caller sees in the place afterward.
//!
//! COMPILER / TRANSFER LAB (optional): Try to move a String out through a
//! mutable reference with `let old = *slot;` and read the error. Repair it with
//! `mem::replace`, then with `mem::take`. Compare both with
//! `Option<String>::take`, including what a second `take` returns.
//! Use examples/ch04_scratch.rs for this, and leave it compiling afterward.

/// A. Install the owned replacement and return the old String; do not clone.
fn exchange(slot: &mut String, replacement: String) -> String {
    std::mem::replace(slot, replacement)
}
/// B. Take the optional message, leaving None. Repeated calls return None.
fn deliver(slot: &mut Option<String>) -> Option<String> {
    slot.take()
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_replacement_is_observable() {
        let mut s = "old".to_owned();
        let p = s.as_ptr();
        let replacement = "new".to_owned();
        let q = replacement.as_ptr();
        let old = exchange(&mut s, replacement);
        assert_eq!(
            old.as_ptr(),
            p,
            "exchange must return the old String itself (same heap buffer), not a copy"
        );
        assert_eq!(old, "old");
        assert_eq!(s, "new");
        assert_eq!(
            s.as_ptr(),
            q,
            "the slot must now own the replacement String itself (same heap buffer), not a copy"
        );
        assert_eq!(exchange(&mut s, String::new()), "new");
        assert_eq!(s, "");
    }
    #[test]
    fn b_message_is_delivered_once() {
        let mut slot = Some("ready".into());
        assert_eq!(deliver(&mut slot), Some("ready".into()));
        assert_eq!(slot, None);
        assert_eq!(deliver(&mut slot), None);
    }

    #[test]
    fn a_b_replacement_does_not_clone() {
        let mut slot = "old".to_owned();
        let replacement = "new".to_owned();
        let (_, allocations) = count(|| exchange(&mut slot, replacement));
        assert_eq!(
            allocations, 0,
            "exchange made {allocations} heap allocation(s); swapping two owned Strings needs none"
        );
        let mut message = Some("ready".to_owned());
        let (_, allocations) = count(|| deliver(&mut message));
        assert_eq!(
            allocations, 0,
            "deliver made {allocations} heap allocation(s); moving the message out needs none"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Length and capacity left behind in `s` by mem::take.
        let mut s = String::with_capacity(64);
        s.push_str("payload");
        let taken = std::mem::take(&mut s);
        let p1: (usize, usize) = (0, 0);
        assert_eq!(
            p1,
            (s.len(), s.capacity()),
            "P1: (len, capacity) of `s` after take (left: your prediction, right: actual)"
        );

        // P2. Is the capacity of `taken` at least 64? Answer true or false.
        let p2: bool = true;
        assert_eq!(
            p2,
            taken.capacity() >= 64,
            "P2: capacity of `taken` at least 64? (left: your prediction, right: actual)"
        );

        // P3. How many heap allocations does mem::take make here?
        let mut v = vec![1, 2, 3];
        let (_, allocations) = count(|| std::mem::take(&mut v));
        let p3: usize = 0;
        assert_eq!(
            p3, allocations,
            "P3: allocations made by mem::take on a Vec (left: your prediction, right: actual)"
        );
    }
}
