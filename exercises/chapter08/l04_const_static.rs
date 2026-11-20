//! LESSON 04 — Constants and statics
//! Run: cargo test --test chapter08 l04_
//!
//! Read first: CONCEPTS.md, section "Lesson 04".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider: Does every use of a const
//! refer to one stable address? How does the lifetime of a static item differ
//! from the lifetime of a local binding?
//!
//! IMPLEMENT the tasks below. Keep the tests and add one edge case.
//!
//! EXPLAIN afterward: Explain shared static storage and why a const is a
//! value definition, without assuming identity for references to a const.
//!
//! LAB (optional): LABS.md, Lab 3, "Const and static in the scratch file".

const HEADER_BYTES: usize = 4;
static MAGIC: [u8; 4] = *b"RUST";
/// A. Return bytes for n fixed-size headers as Option, rejecting usize overflow.
/// Use HEADER_BYTES rather than repeating a literal.
fn header_storage(n: usize) -> Option<usize> {
    todo!("Implement this task")
}
/// B. Return a static borrow of the single MAGIC array, not an equal copy.
fn magic() -> &'static [u8; 4] {
    todo!("Implement this task")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;
    use std::sync::atomic::{AtomicU32, Ordering};
    #[test]
    fn a_const_drives_checked_size() {
        assert_eq!(header_storage(0), Some(0));
        assert_eq!(header_storage(3), Some(12));
        assert_eq!(header_storage(usize::MAX), None);
        assert_eq!(header_storage(usize::MAX / 4), Some(usize::MAX / 4 * 4));
        assert_eq!(header_storage(usize::MAX / 4 + 1), None);
    }
    #[test]
    fn b_static_has_stable_identity() {
        assert_eq!(magic(), b"RUST");
        assert!(
            std::ptr::eq(magic(), &MAGIC),
            "magic() must borrow the MAGIC static itself, not an equal copy"
        );
        assert!(
            std::ptr::eq(magic(), magic()),
            "every call must return the same address: a static has one location"
        );
    }

    fn hit() -> u32 {
        static HITS: AtomicU32 = AtomicU32::new(0);
        HITS.fetch_add(1, Ordering::Relaxed) + 1
    }

    static ALIAS: &[u8] = &MAGIC;

    const fn kib(n: usize) -> usize {
        n * 1024
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. A static declared inside a function body: three calls.
        let p1: (u32, u32, u32) = guess();
        assert_eq!(
            p1,
            (hit(), hit(), hit()),
            "P1: one storage location for every call"
        );

        // P2. A static that borrows another static: same address?
        let p2: bool = guess();
        assert_eq!(
            p2,
            std::ptr::eq(ALIAS.as_ptr(), MAGIC.as_ptr()),
            "P2: statics can borrow statics"
        );

        // P3. A const fn can size an array at compile time.
        let buffer = [0u8; kib(2)];
        let p3: usize = guess();
        assert_eq!(p3, buffer.len(), "P3: const fn in an array length");
    }
}
