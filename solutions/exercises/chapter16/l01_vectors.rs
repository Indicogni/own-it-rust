//! LESSON 01 — Vector access and structural editing
//! Run: cargo test --test chapter16 l01_
//! Read first: "Lesson 01" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does reserve add elements? Which of remove and swap_remove
//! preserves order? What does split_off leave behind?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Explain length versus capacity and how each selected operation
//! changes both ownership and ordering.
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch16_scratch.rs):
//! Build Vec::with_capacity(8), record len/capacity, push three values, reserve(20),
//! shrink_to_fit, and record observations. Assert only capacity>=len; do not assume a
//! growth factor or that shrinking is exact.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, lab, and explanation are done.

/// A. Checked copied access; no panic on an invalid index.
fn at(values: &[i32], index: usize) -> Option<i32> {
    values.get(index).copied()
}
/// B. Split at a valid index, reverse the owned tail, return (head,tail).
/// Use split_off: the head keeps the original buffer; only the tail is new.
fn split_tail(mut values: Vec<i32>, index: usize) -> (Vec<i32>, Vec<i32>) {
    // `split_off` moves the tail into a new vector; `values` keeps its buffer.
    let mut tail = values.split_off(index);
    tail.reverse();
    (values, tail)
}
/// C. On nonempty data swap first and last, then fill the interior with zero.
/// Lengths zero, one, and two have no interior.
fn border_swap(values: &mut [i32]) {
    // The pattern matches only when there are at least two elements, so the
    // lengths zero and one need no special case.
    if let [first, interior @ .., last] = values {
        std::mem::swap(first, last);
        interior.fill(0);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_checked_access_handles_end_and_empty() {
        assert_eq!(at(&[2, 4], 1), Some(4));
        assert_eq!(at(&[2, 4], 2), None, "index == len is one past the end");
        assert_eq!(at(&[], usize::MAX), None);
    }
    #[test]
    fn b_split_off_moves_an_independent_tail() {
        assert_eq!(split_tail(vec![1, 2, 3, 4], 2), (vec![1, 2], vec![4, 3]));
        assert_eq!(split_tail(vec![1], 0), (vec![], vec![1]));
        assert_eq!(split_tail(vec![1, 2], 2), (vec![1, 2], vec![]));
    }
    #[test]
    fn b_head_keeps_the_original_buffer() {
        let v: Vec<i32> = (0..100).collect();
        let p = v.as_ptr();
        let ((head, tail), allocations) = count(|| split_tail(v, 40));
        assert_eq!(
            head.as_ptr(),
            p,
            "the head must stay in the buffer the vector already had; \
             a different address means the head was copied"
        );
        assert_eq!(
            (head.len(), tail[0]),
            (40, 99),
            "40 elements stay in the head, and the reversed tail starts with 99"
        );
        assert!(
            allocations <= 1,
            "only the tail needs a new buffer, so at most 1 allocation; got {allocations}"
        );
    }
    #[test]
    fn c_swapping_and_filling_leave_length_unchanged() {
        let mut a = [1, 2, 3, 4, 5];
        border_swap(&mut a);
        assert_eq!(a, [5, 0, 0, 0, 1]);
        let mut b = [9];
        border_swap(&mut b);
        assert_eq!(b, [9]);
        let mut c = [1, 2];
        border_swap(&mut c);
        assert_eq!(c, [2, 1]);
        let mut d = [1, 2, 3];
        border_swap(&mut d);
        assert_eq!(d, [3, 0, 1]);
        border_swap(&mut []);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. After reserve(20) on a 3-element vector: the length, and whether
        // the capacity is now at least 23.
        let mut v = vec![1, 2, 3];
        v.reserve(20);
        let p1: (usize, bool) = (3, true);
        assert_eq!(p1, (v.len(), v.capacity() >= 23), "P1: reserve vs length");

        // P2. What does split_off(2) leave behind, and what does it return?
        let mut v = vec![1, 2, 3, 4, 5];
        let tail = v.split_off(2);
        let p2: (Vec<i32>, Vec<i32>) = (vec![1, 2], vec![3, 4, 5]);
        assert_eq!(p2, (v, tail), "P2: split_off");

        // P3. Range access with get: in bounds, and running past the end.
        let v = [10, 20, 30];
        let p3: (Option<&[i32]>, Option<&[i32]>) = (Some(&[20, 30][..]), None);
        assert_eq!(p3, (v.get(1..3), v.get(2..5)), "P3: checked slicing");
    }
}
