//! LESSON 01 — Borrowing and dereferencing
//! Run: cargo test --test chapter05 l01_
//! Read first: CONCEPTS.md, section "Lesson 01 · Borrowing and dereferencing".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does passing &String move the String? Does calling len on &&String need explicit stars?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain who owns storage and why a method receiver can be dereferenced automatically while assignment through &mut needs *.
//! COMPILER / TRANSFER LAB (optional; tests do not check it): Try mutating through &String, then through &mut String. Compare `(*reference).len()` with `reference.len()` and with indexing a borrowed vector.
//! Use examples/ch05_scratch.rs; repair intentional failures afterward.

/// A. Return the string byte length without taking ownership or cloning.
// `&String` on purpose: this task is about borrowing from an owner. In your own
// code, prefer `&str` for a parameter that only reads text (see C01).
#[allow(clippy::ptr_arg)]
fn borrowed_length(text: &String) -> usize {
    text.len()
}
/// B. Replace an integer through its mutable reference, returning its old value.
fn overwrite(slot: &mut i32, value: i32) -> i32 {
    // `replace` writes the new value and hands back the old one in one step.
    std::mem::replace(slot, value)
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::hint::black_box;
    #[test]
    fn a_owner_remains_usable() {
        let text = String::from("é🦀");
        assert_eq!(
            borrowed_length(&text),
            6,
            "len() counts bytes, not characters"
        );
        assert_eq!(text, "é🦀", "the owner is still usable after lending it");
        assert_eq!(borrowed_length(&String::new()), 0);
    }
    #[test]
    fn b_exclusive_borrow_updates_referent() {
        let mut n = -8;
        assert_eq!(
            overwrite(&mut n, 4),
            -8,
            "return the value that was replaced"
        );
        assert_eq!(n, 4, "the write must reach the caller's variable");
        assert_eq!(overwrite(&mut n, 0), 4);
        assert_eq!(n, 0);
    }

    #[test]
    fn a_borrowed_length_does_not_allocate() {
        let text = String::from("borrowed");
        let (n, allocations) = count(|| borrowed_length(&text));
        assert_eq!(n, 8);
        assert_eq!(
            allocations, 0,
            "reading through a borrow needs no copy; did you clone the String?"
        );
    }

    fn by_reference(text: &str) -> usize {
        text.len()
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Two writes through one mutable reference.
        let mut n = 5;
        let r = &mut n;
        *r *= 3;
        *r -= 1;
        let p1: i32 = 14;
        assert_eq!(p1, n, "P1: writing through &mut changes the referent");

        // P2. Indexing and len() through three layers of references.
        let v = vec![10, 20, 30];
        let r = &&&v;
        let p2: i32 = 23;
        assert_eq!(p2, r[1] + r.len() as i32, "P2: automatic dereferencing");

        // P3. Heap allocations: passing a borrow, versus cloning the String.
        let s = String::from("owner");
        let (_, borrowed) = count(|| by_reference(&s));
        let (_, cloned) = count(|| black_box(s.clone()));
        let p3: (usize, usize) = (0, 1);
        assert_eq!(p3, (borrowed, cloned), "P3: a borrow copies nothing");
    }
}
