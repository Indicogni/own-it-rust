//! LESSON 06 — const and unsafe expression blocks
//! Run: cargo test --test chapter06 l06_
//! Read first: CONCEPTS.md, section "Lesson 06".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: When is an inline const block
//! evaluated? Does putting safe code inside unsafe make it faster?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain which operations require unsafe and what facts a
//! caller must prove. Describe why these tasks need no unchecked operations.
//! COMPILER / TRANSFER LAB: Write `const { 6 * 7 }` as an array length, then try
//! referring to a runtime local inside the const block. Separately obtain a raw
//! pointer from a live i32 and read it in one small unsafe block; state validity,
//! alignment, initialization, and aliasing facts. Never dereference a null or
//! dangling pointer, even as an experiment.
//! Use examples/ch06_scratch.rs; restore it to compiling code afterwards.

/// A. Produce an all-zero 12-byte frame. Express its length using an inline
/// const block whose calculation is 3 fields * 4 bytes, rather than literal 12.
fn zero_frame() -> [u8; 12] {
    // The compiler evaluates the const block, so the length is part of the type.
    [0; const { 3 * 4 }]
}
/// B. Return a copied byte at index if it exists. Do not use unsafe: safe
/// checked indexing is the right tool when no special invariant is provided.
fn checked_byte(bytes: &[u8], index: usize) -> Option<u8> {
    bytes.get(index).copied()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::source;
    #[test]
    fn a_const_expression_sets_array_length() {
        assert_eq!(zero_frame(), [0; 12]);
    }
    #[test]
    fn a_source_uses_an_inline_const_block() {
        let body = source::body(include_str!("l06_const_unsafe.rs"), "zero_frame");
        assert!(
            source::has(&body, "const {"),
            "task A asks for an inline const block as the array length inside zero_frame: \
             `[0; const {{ 3 * 4 }}]` (this test reads your source)"
        );
    }
    #[test]
    fn b_source_has_no_unsafe() {
        assert_eq!(checked_byte(&[7], 0), Some(7));
        let body = source::body(include_str!("l06_const_unsafe.rs"), "checked_byte");
        assert!(
            !source::has_word(&body, "unsafe"),
            "task B says not to use unsafe: `get` already checks the index \
             (this test reads your source)"
        );
    }
    #[test]
    fn b_safe_lookup_handles_boundaries() {
        assert_eq!(checked_byte(&[4, 8], 1), Some(8));
        assert_eq!(checked_byte(&[4, 8], 0), Some(4));
        assert_eq!(checked_byte(&[4, 8], 2), None);
        assert_eq!(checked_byte(&[], 0), None);
        assert_eq!(checked_byte(&[4, 8], usize::MAX), None);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Size in bytes of an array whose length is an inline const block.
        let table = [0u16; const { 3 * 4 }];
        let p1: usize = 24;
        assert_eq!(p1, size_of_val(&table), "P1: const length, u16 elements");

        // P2. Writing through a raw pointer to a live local. `&raw mut x` makes
        // a `*mut i32` that points to `x` without creating a reference first.
        let mut x = 5;
        let p = &raw mut x;
        // SAFETY: `p` comes from a live, aligned, initialized local, and no
        // reference to `x` is used while we write through it.
        unsafe {
            *p += 1;
        }
        let p2: i32 = 6;
        assert_eq!(p2, x, "P2: the write is visible through the binding");

        // P3. Pointer arithmetic: does add(1) move one byte or one element?
        let values = [10u32, 20, 30];
        let first = values.as_ptr();
        // SAFETY: index 1 is inside `values`, which outlives this read.
        let second = unsafe { *first.add(1) };
        let p3: u32 = 20;
        assert_eq!(p3, second, "P3: add counts elements");
    }
}
