//! LESSON 03 — Raw slice reconstruction
//! Run: cargo test --test chapter23 l03_
//! Read first: "Lesson 03" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Two `&mut` slices overlap, but nobody reads or writes through them yet.
//!    Is that already undefined behavior?
//! 2. Which properties must the pointer of a zero-length slice still have?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Prove bounds, provenance, alignment, initialization,
//! lifetime, and non-overlap for `split_checked`, one at a time.
//!
//! COMPILER / TRANSFER LAB (optional): Write the split with two ordinary index
//! expressions, `(&mut values[..mid], &mut values[mid..])`, and read the
//! borrow error. The compiler cannot see that the two ranges are disjoint;
//! your proof replaces that check, so do not bypass it without one.
//! Use examples/ch23_scratch.rs for this, and leave it compiling afterward.

/// A. Safe checked split, implemented with raw pointers and from_raw_parts_mut.
/// For mid > len return None. Otherwise return exactly the two disjoint regions.
/// Input's unique borrow lasts for both outputs; no allocation and no cloning.
fn split_checked<T>(values: &mut [T], mid: usize) -> Option<(&mut [T], &mut [T])> {
    let len = values.len();
    if mid > len {
        return None;
    }
    let start = values.as_mut_ptr();
    // SAFETY:
    // - bounds: `mid <= len`, so `[0, mid)` and `[mid, len)` lie inside the
    //   slice, and `start.add(mid)` is at most one past its end;
    // - provenance: both pointers come from `values.as_mut_ptr()`, which may
    //   access the whole slice;
    // - alignment and initialization: inherited from the `&mut [T]`;
    // - lifetime: both results borrow from `values` (see the signature);
    // - aliasing: the two ranges do not overlap, and `values` itself is not
    //   used again, so each element is reachable through one `&mut` only.
    unsafe {
        Some((
            std::slice::from_raw_parts_mut(start, mid),
            std::slice::from_raw_parts_mut(start.add(mid), len - mid),
        ))
    }
}
/// B. Swap two elements via raw pointer operations after checking BOTH indices.
/// Equal indices are valid. Return false with no mutation for any invalid index.
fn swap_checked<T>(values: &mut [T], a: usize, b: usize) -> bool {
    let len = values.len();
    if a >= len || b >= len {
        return false;
    }
    let start = values.as_mut_ptr();
    // SAFETY: both indices are in bounds, so both pointers point to
    // initialized elements of the uniquely borrowed slice. `ptr::swap`
    // allows its two arguments to be the same place (`a == b`).
    unsafe { std::ptr::swap(start.add(a), start.add(b)) };
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_disjoint_regions_mutate_original() {
        let mut a = [1, 2, 3, 4];
        let (l, r) = split_checked(&mut a, 2).unwrap();
        l[0] = 9;
        r[0] = 8;
        assert_eq!(a, [9, 2, 8, 4], "both halves must write to the original");
        assert!(split_checked(&mut a, 5).is_none(), "mid > len is rejected");
        assert_eq!(
            split_checked::<u8>(&mut [], 0).unwrap(),
            (&mut [][..], &mut [][..])
        );
    }
    #[test]
    fn a_endpoints_and_exact_regions() {
        let mut a = [1, 2, 3];
        let base = a.as_ptr();
        let (l, r) = split_checked(&mut a, 0).unwrap();
        assert_eq!((l.len(), r.len()), (0, 3));
        assert_eq!(
            r.as_ptr(),
            base,
            "with mid == 0 the right half starts at the first element"
        );
        let (l, r) = split_checked(&mut a, 3).unwrap();
        assert_eq!((l.len(), r.len()), (3, 0));
        assert_eq!(
            l.as_ptr(),
            base,
            "the left half starts at the first element"
        );
        let (l, r) = split_checked(&mut a, 1).unwrap();
        assert_eq!((l, r), (&mut [1][..], &mut [2, 3][..]));
        assert!(split_checked(&mut a, usize::MAX).is_none());
    }
    #[test]
    fn a_split_does_not_copy() {
        let mut labels = vec![String::from("a"), String::from("b")];
        let first = labels[0].as_ptr();
        let (parts, allocations) =
            count(|| split_checked(&mut labels, 1).map(|(l, r)| (l.len(), r.len())));
        assert_eq!(
            (parts, allocations),
            (Some((1, 1)), 0),
            "(lengths, allocations): a split only makes two views; it never allocates"
        );
        assert_eq!(
            labels[0].as_ptr(),
            first,
            "the Strings must stay where they were"
        );
    }
    #[test]
    fn a_technique_split_builds_slices_from_raw_parts() {
        let body = crate::source::body_of(include_str!("l03_raw_slices.rs"), "fn split_checked");
        assert!(
            body.contains("from_raw_parts_mut") && !body.contains("split_at_mut"),
            "task A asks you to rebuild both halves with `slice::from_raw_parts_mut` and to \
             write the proof yourself; `split_at_mut` (and its checked form) is the std \
             function you are re-creating"
        );
    }
    #[test]
    fn b_raw_swap_preserves_noncopy_ownership() {
        let mut a = [String::from("a"), String::from("b")];
        assert!(swap_checked(&mut a, 0, 1));
        assert_eq!(a, ["b", "a"]);
        assert!(swap_checked(&mut a, 1, 1), "equal indices are valid");
        assert!(!swap_checked(&mut a, 0, 2));
        assert_eq!(a, ["b", "a"], "a rejected swap changes nothing");
    }
    #[test]
    fn b_either_index_may_be_invalid() {
        let mut a = [String::from("x"), String::from("y")];
        let px = a[0].as_ptr();
        assert!(!swap_checked(&mut a, 2, 0), "first index out of range");
        assert!(!swap_checked(&mut a, usize::MAX, usize::MAX));
        assert_eq!(a, ["x", "y"]);
        let (swapped, allocations) = count(|| swap_checked(&mut a, 1, 0));
        assert_eq!(
            (swapped, allocations),
            (true, 0),
            "(swapped, allocations): a swap moves two values; it never clones them"
        );
        assert_eq!(a[1].as_ptr(), px, "swap moves the Strings, not their text");
        let mut empty: [u8; 0] = [];
        assert!(!swap_checked(&mut empty, 0, 0));
    }
    #[test]
    fn b_technique_swap_uses_raw_pointers() {
        let body = crate::source::body_of(include_str!("l03_raw_slices.rs"), "fn swap_checked");
        assert!(
            body.contains("unsafe") && !body.contains("values.swap("),
            "task B asks for a raw-pointer swap (for example `ptr::swap`) after your own \
             bounds check; `values.swap(a, b)` is the safe method you are re-creating"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// `left` in a failure message is your prediction; `right` is the real value.
    #[test]
    fn p_predictions() {
        // P1. An empty Vec has not allocated. Is its pointer's address zero,
        // and what is that address modulo the alignment of u64?
        let v: Vec<u64> = Vec::new();
        let p1: (bool, usize) = (false, 0);
        assert_eq!(
            p1,
            (
                v.as_ptr().addr() == 0,
                v.as_ptr().addr() % align_of::<u64>()
            ),
            "P1: the pointer of an empty Vec"
        );

        // P2. How many BYTES separate the two halves of this split?
        let mut words = [0_u16; 5];
        let (left, right) = words.split_at_mut(2);
        let p2: usize = 4;
        assert_eq!(
            p2,
            right.as_ptr().addr() - left.as_ptr().addr(),
            "P2: split offset"
        );

        // P3. ptr::swap with the SAME pointer twice. What does the array hold?
        let mut pair = [1, 2];
        let p = pair.as_mut_ptr();
        // SAFETY: p is valid for reads and writes; ptr::swap allows overlap.
        unsafe { std::ptr::swap(p, p) };
        let p3: [i32; 2] = [1, 2];
        assert_eq!(p3, pair, "P3: swapping a place with itself");
    }
}
