//! LESSON 04 — A flagged borrow
//! Run: cargo test --test chapter23 l04_
//! Read first: "Lesson 04" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Why is the lowest address bit of a pointer to an aligned u32 always 0?
//! 2. A pointer has two parts: an address, and the permission to use one
//!    allocation (its provenance). Which part does a tag bit change?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain how PhantomData lets the borrow checker track
//! the lifetime, and how `map_addr` keeps the pointer's provenance.
//!
//! COMPILER / TRANSFER LAB (optional): Write a function that creates a local
//! u32 and tries to return a `Flagged` that refers to it. Keep the lifetime
//! error and read it; do not silence it with `'static`.
//! Use examples/ch23_scratch.rs for this, and leave it compiling afterward.

use std::marker::PhantomData;
struct Flagged<'a> {
    tagged: *const u32,
    borrow: PhantomData<&'a u32>,
}
impl<'a> Flagged<'a> {
    /// A. Borrow value and store flag in its pointer's low bit using map_addr.
    /// Check align_of::<u32>() >= 2. Never dereference the tagged pointer.
    fn new(value: &'a u32, flag: bool) -> Self {
        todo!("A: implement the contract")
    }
    fn flag(&self) -> bool {
        self.tagged.addr() & 1 != 0
    }
    /// B. Clear the tag with map_addr, then return the original shared borrow.
    /// SAFETY invariant: pointer comes from a live &'a u32, with only low tag bit
    /// changed, and PhantomData retains that borrow. No mutation is permitted.
    fn value(&self) -> &'a u32 {
        todo!("B: implement the contract")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;
    #[test]
    fn a_b_tag_and_borrow_are_independent() {
        let value = 123;
        for flag in [false, true] {
            let f = Flagged::new(&value, flag);
            assert_eq!(f.flag(), flag, "the flag must survive in the low bit");
            assert_eq!(*f.value(), 123);
            assert!(
                std::ptr::eq(f.value(), &value),
                "value() must return the original borrow: clear the tag bit first"
            );
        }
    }
    #[test]
    fn a_only_the_low_bit_changes() {
        let value = 7_u32;
        let original = (&value as *const u32).addr();
        assert_eq!(
            Flagged::new(&value, false).tagged.addr(),
            original,
            "flag == false must leave the address unchanged"
        );
        assert_eq!(
            Flagged::new(&value, true).tagged.addr(),
            original | 1,
            "flag == true must set bit 0 of the address and nothing else"
        );
    }
    #[test]
    fn b_each_element_keeps_its_own_address() {
        let values = [10_u32, 20, 30, 40];
        let flags: Vec<Flagged<'_>> = values
            .iter()
            .enumerate()
            .map(|(i, v)| Flagged::new(v, i % 2 == 1))
            .collect();
        for (i, f) in flags.iter().enumerate() {
            assert_eq!(f.flag(), i % 2 == 1);
            assert!(
                std::ptr::eq(f.value(), &values[i]),
                "element {i}: value() must point at the element that was borrowed"
            );
        }
    }
    #[test]
    fn a_b_technique_tag_keeps_provenance() {
        for item in ["fn new", "fn value"] {
            let body = crate::source::body_of(include_str!("l04_tagged.rs"), item);
            assert!(
                body.contains("map_addr") || body.contains("with_addr"),
                "`{item}` should change the address with `map_addr`, which keeps the pointer's \
                 provenance. A round trip through `as usize` and back also passes the other \
                 tests, but it gives up the link between the pointer and its allocation"
            );
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// `left` in a failure message is your prediction; `right` is the real value.
    #[test]
    fn p_predictions() {
        // P1. Alignment of u32 and of u8 (how many low address bits are free?).
        let p1: (usize, usize) = guess();
        assert_eq!(p1, (align_of::<u32>(), align_of::<u8>()), "P1: alignment");

        // P2. Tag a pointer, then clear the tag. Low bit while tagged, and is
        // the cleared pointer equal to the original?
        let value = 5_u32;
        let original = &value as *const u32;
        let tagged = original.map_addr(|a| a | 1);
        let cleared = tagged.map_addr(|a| a & !1);
        let p2: (usize, bool) = guess();
        assert_eq!(p2, (tagged.addr() & 1, cleared == original), "P2: map_addr");

        // P3. Size of a Flagged, and of its PhantomData field, on a 64-bit target.
        let p3: (usize, usize) = guess();
        assert_eq!(
            p3,
            (size_of::<Flagged<'_>>(), size_of::<PhantomData<&u32>>()),
            "P3: what PhantomData costs"
        );
    }
}
