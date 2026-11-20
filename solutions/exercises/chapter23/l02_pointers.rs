//! LESSON 02 — Raw pointer reads with contracts
//! Run: cargo test --test chapter23 l02_
//! Read first: "Lesson 02" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Creating a raw pointer is safe. Which operations on it are unsafe?
//! 2. Is a non-null pointer enough to permit a read?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: List every caller obligation of `read_u32` and show how
//! the safe wrapper `at` discharges each one.
//!
//! COMPILER / TRANSFER LAB (optional): Create raw pointers from valid
//! references and print their addresses with `{:p}`. Do not dereference an
//! address that you made up. Explain in one sentence why knowing an address
//! does not prove that an allocation lives there.
//! Use examples/ch23_scratch.rs for this, and leave it compiling afterward.

/// A. Read and copy one u32.
/// # Safety
/// p must be non-null, aligned for u32, and point to an initialized u32 in a
/// live allocation readable for this operation. There must be no concurrent
/// mutation or exclusive reference conflicting with this read. Do not free it.
unsafe fn read_u32(p: *const u32) -> u32 {
    // SAFETY: the caller promises that `p` is non-null, aligned, and points
    // to an initialized u32 that nobody is writing during this call. That is
    // exactly what a read through a raw pointer requires.
    unsafe { *p }
}
/// B. Safe boundary: return None for an invalid index. Otherwise derive a raw
/// pointer from this slice and call read_u32; do not form a pointer out of range.
fn at(values: &[u32], index: usize) -> Option<u32> {
    if index >= values.len() {
        return None;
    }
    // SAFETY: two unsafe operations.
    // `add`: `index < values.len()`, so the offset stays inside the
    // allocation that `values.as_ptr()` points into.
    // `read_u32`: the pointer is derived from a live `&[u32]`, so it is
    // non-null, aligned, and points to an initialized element; the shared
    // borrow keeps the memory alive and excludes writers during the call.
    Some(unsafe { read_u32(values.as_ptr().add(index)) })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_raw_read_does_not_take_ownership() {
        let value = 37_u32;
        // SAFETY: the pointer comes from a reference to a live, initialized u32.
        let observed = unsafe { read_u32(&value) };
        assert_eq!(observed, 37);
        assert_eq!(value, 37, "a read copies the u32; the source is unchanged");
        let max = u32::MAX;
        // SAFETY: as above.
        assert_eq!(unsafe { read_u32(&max) }, u32::MAX);
    }
    #[test]
    fn b_safe_boundary_checks_bounds() {
        assert_eq!(at(&[4, 9], 1), Some(9));
        assert_eq!(at(&[4, 9], 0), Some(4));
        assert_eq!(at(&[4, 9], 2), None, "index == len is out of range");
        assert_eq!(at(&[4, 9], usize::MAX), None);
        assert_eq!(at(&[], 0), None, "an empty slice has no element 0");
        assert_eq!(at(&[], usize::MAX), None);
    }
    #[test]
    fn b_reads_from_a_subslice() {
        let values = [1, 2, 3, 4, 5];
        assert_eq!(at(&values[2..], 1), Some(4));
        assert_eq!(
            at(&values[2..4], 2),
            None,
            "bounds come from the slice, not the array"
        );
    }
    #[test]
    fn b_technique_at_calls_read_u32() {
        let body = crate::source::body_of(include_str!("l02_pointers.rs"), "fn at");
        assert!(
            body.contains("read_u32"),
            "task B is the safe wrapper around task A: check the index, then call `read_u32` \
             with a pointer derived from the slice. `values.get(index)` gives the same result \
             but does not practise the contract"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// `left` in a failure message is your prediction; `right` is the real value.
    #[test]
    fn p_predictions() {
        // P1. Sizes on a 64-bit target: Option<&u32>, Option<*const u32>, *const [u32].
        let p1: (usize, usize, usize) = (8, 16, 16);
        assert_eq!(
            p1,
            (
                size_of::<Option<&u32>>(),
                size_of::<Option<*const u32>>(),
                size_of::<*const [u32]>()
            ),
            "P1: niches and fat pointers"
        );

        // P2. `add` counts elements. How many BYTES apart are these pointers?
        let values = [1_u32, 2, 3];
        let start = values.as_ptr();
        let p2: usize = 8;
        assert_eq!(
            p2,
            start.wrapping_add(2).addr() - start.addr(),
            "P2: pointer arithmetic"
        );

        // P3. This pointer outlived the variable it pointed to (creating it is
        // safe; it is never dereferenced). Is it null?
        let dangling = {
            let local = 5_u32;
            &local as *const u32
        };
        let p3: bool = false;
        assert_eq!(p3, dangling.is_null(), "P3: dangling pointers");
    }
}
