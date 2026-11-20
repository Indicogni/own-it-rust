//! LESSON 06 — Copy and explicit cloning
//! Run: cargo test --test chapter04 l06_
//! Read first: "Lesson 06" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Does assigning a `[u8; 4]` move it? Does assigning a `[String; 2]`?
//! 2. Does Rust ever call `clone` for you without you writing it?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain which types are copied on assignment, why a type
//! must ask for `Copy`, and why a cloned String is independent of the
//! original.
//!
//! COMPILER / TRANSFER LAB (optional): Derive `Copy` and `Clone` on a struct of
//! two integers, then add a String field and read the error. Try to copy a
//! tuple that contains one String. Explain why a type with a `Drop`
//! implementation cannot be `Copy`.
//! Use examples/ch04_scratch.rs for this, and leave it compiling afterward.

/// A. Copy the fixed-size integer array, overwrite the copy's first byte, and
/// return (original, changed). Do not use clone.
fn copied_header(header: [u8; 4], first: u8) -> ([u8; 4], [u8; 4]) {
    // `[u8; 4]` is Copy, so this assignment copies and `header` stays usable.
    let mut changed = header;
    changed[0] = first;
    (header, changed)
}
/// B. Return an independent owned copy with a suffix; input remains unchanged.
/// Explicitly clone the input String before modifying it.
// `&String` is on purpose here: the task is an explicit clone of a String.
// In your own code a read-only text parameter should be `&str`.
#[allow(clippy::ptr_arg)]
fn cloned_label(input: &String, suffix: &str) -> String {
    let mut label = input.clone();
    label.push_str(suffix);
    label
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::hint::black_box;
    #[test]
    fn a_copy_leaves_source_usable() {
        assert_eq!(copied_header([1, 2, 3, 4], 9), ([1, 2, 3, 4], [9, 2, 3, 4]));
        assert_eq!(copied_header([0; 4], 0), ([0; 4], [0; 4]));
    }
    #[test]
    fn b_clone_owns_independent_text() {
        let s = String::from("港");
        let out = cloned_label(&s, "!");
        assert_eq!(s, "港");
        assert_eq!(out, "港!");
        assert_ne!(
            s.as_ptr(),
            out.as_ptr(),
            "the result must own its own heap buffer, separate from the input's buffer"
        );
        assert_eq!(cloned_label(&String::new(), ""), "");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Copying an array of integers, then changing the copy.
        let a = [1u8, 2, 3];
        let mut b = a;
        b[0] = 9;
        let p1: u8 = 1;
        assert_eq!(
            p1, a[0],
            "P1: a[0] after changing the copy (left: your prediction, right: actual)"
        );
        assert_eq!(b, [9, 2, 3]);

        // P2. Heap allocations made by cloning a Vec holding two non-empty Strings.
        let v = vec!["ab".to_owned(), "cd".to_owned()];
        let (copy, allocations) = count(|| black_box(&v).clone());
        let p2: usize = 3;
        assert_eq!(
            p2, allocations,
            "P2: allocations made by cloning a Vec<String> (left: your prediction, right: actual)"
        );
        assert_eq!(copy, v);

        // P3. Heap allocations made by copying a [u64; 1000] array.
        let big = [7u64; 1000];
        let (copy, allocations) = count(|| black_box(big));
        let p3: usize = 0;
        assert_eq!(
            p3, allocations,
            "P3: allocations made by copying an array (left: your prediction, right: actual)"
        );
        assert_eq!(copy[999], 7);
    }
}
