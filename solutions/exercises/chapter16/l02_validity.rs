//! LESSON 02 — Growth, removal, and borrow validity
//! Run: cargo test --test chapter16 l02_
//! Read first: "Lesson 02" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Can a reference to v[0] survive v.push(...) while used
//! afterward? Is unused reserved capacity enough to convince the borrow checker?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why does Rust reject potential invalidation without proving whether
//! this particular push reallocates? How does splitting prove disjoint mutable access?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch16_scratch.rs):
//! Try let first=&v[0]; v.push(4); println!("{first}"); even after reserve. Repair by
//! copying the element, by ending the borrow, and by retaining an index then reborrowing.
//! Try holding an iterator while inserting. Compare all diagnostics. Then rewrite
//! task A with get_disjoint_mut and compare the two versions.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, lab, and explanation are done.

/// A. Borrow two different elements mutably at the same time. Return them in
/// the order (values[a], values[b]). Return None when a == b or when either
/// index is out of range. No unsafe and no cloning: split the slice with
/// split_at_mut, so that the compiler can see that the two references cannot
/// overlap. (std also has a shortcut, get_disjoint_mut; P4 and the lab use it.)
fn pair_mut<T>(values: &mut [T], a: usize, b: usize) -> Option<(&mut T, &mut T)> {
    if a == b || a >= values.len() || b >= values.len() {
        return None;
    }
    // Split just before the larger index. The smaller index is then in the
    // left part and the larger one is element 0 of the right part.
    if a < b {
        let (left, right) = values.split_at_mut(b);
        Some((&mut left[a], &mut right[0]))
    } else {
        let (left, right) = values.split_at_mut(a);
        Some((&mut right[0], &mut left[b]))
    }
}
/// B. Retain only nonnegative entries, keeping their relative order.
/// Edit in place: no new vector.
fn retain_nonnegative(values: &mut Vec<i32>) {
    values.retain(|&value| value >= 0);
}
/// C. Remove a valid index without preserving order, returning removed value.
fn fast_remove(values: &mut Vec<i32>, index: usize) -> i32 {
    values.swap_remove(index)
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_split_mutability_is_disjoint() {
        let mut a = [1, 2, 3, 4];
        let (x, y) = pair_mut(&mut a, 3, 1).expect("3 and 1 are different, valid indices");
        assert_eq!(
            (*x, *y),
            (4, 2),
            "the pair is (values[a], values[b]) in that order, also when a > b"
        );
        std::mem::swap(x, y);
        assert_eq!(a, [1, 4, 3, 2]);
        let (x, y) = pair_mut(&mut a, 0, 3).expect("0 and 3 are different, valid indices");
        *x += *y;
        *y = 0;
        assert_eq!(a, [3, 4, 3, 0]);
        let (x, y) = pair_mut(&mut a, 1, 2).expect("neighbours are still two elements");
        assert_eq!((*x, *y), (4, 3));
    }
    #[test]
    fn a_pair_mut_rejects_equal_and_invalid_indices() {
        let mut a = [1, 2, 3, 4];
        assert!(
            pair_mut(&mut a, 2, 2).is_none(),
            "one element cannot be borrowed mutably twice"
        );
        assert!(pair_mut(&mut a, 1, 4).is_none(), "index 4 is past the end");
        assert!(pair_mut(&mut a, 4, 1).is_none(), "index 4 is past the end");
        assert!(pair_mut(&mut a, 9, 9).is_none());
        assert!(pair_mut::<i32>(&mut [], 0, 1).is_none());
        assert_eq!(a, [1, 2, 3, 4], "a refused request changes nothing");
    }
    #[test]
    fn a_pair_mut_lends_the_elements_themselves() {
        let mut words = vec!["ab".to_string(), "cd".to_string(), "ef".to_string()];
        // Addresses of the elements inside the vector's buffer, for comparison only.
        let (slot0, slot2) = (&raw const words[0], &raw const words[2]);
        let (same_places, allocations) = count(|| {
            let (x, y) = pair_mut(&mut words, 2, 0).expect("2 and 0 are different, valid indices");
            let same_places = (std::ptr::eq(x, slot2), std::ptr::eq(y, slot0));
            std::mem::swap(x, y);
            same_places
        });
        assert_eq!(
            same_places,
            (true, true),
            "the references must point at values[2] and values[0] inside the slice, \
             not at copies and not at other elements"
        );
        assert_eq!(words, ["ef", "cd", "ab"]);
        assert_eq!(
            allocations, 0,
            "lending two elements needs no clone and no new collection"
        );
    }
    #[test]
    fn b_retain_is_stable() {
        let mut a = vec![-1, 3, 0, -2, 3];
        retain_nonnegative(&mut a);
        assert_eq!(a, [3, 0, 3]);
        let mut none = vec![-5, -1];
        retain_nonnegative(&mut none);
        assert!(none.is_empty());
    }
    #[test]
    fn b_retain_edits_in_place() {
        let mut a: Vec<i32> = (-50..50).collect();
        let p = a.as_ptr();
        let (_, allocations) = count(|| retain_nonnegative(&mut a));
        assert_eq!((a.len(), a[0]), (50, 0));
        assert_eq!(
            a.as_ptr(),
            p,
            "the vector must still use its old buffer; a new address means \
             the survivors were collected into a second vector"
        );
        assert_eq!(
            allocations, 0,
            "removing in place needs no allocation; got {allocations}"
        );
    }
    #[test]
    fn c_swap_remove_fills_the_hole_with_last() {
        let mut a = vec![10, 20, 30, 40];
        assert_eq!(fast_remove(&mut a, 1), 20);
        assert_eq!(
            a,
            [10, 40, 30],
            "the last element moves into the hole; the others do not shift"
        );
        assert_eq!(fast_remove(&mut a, 2), 30);
        assert_eq!(a, [10, 40]);
        assert_eq!(fast_remove(&mut a, 0), 10);
        assert_eq!(a, [40]);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. drain removes a range and hands it back. What is drained, and what stays?
        let mut v = vec![1, 2, 3, 4, 5];
        let drained: Vec<i32> = v.drain(1..3).collect();
        let p1: (Vec<i32>, Vec<i32>) = (vec![2, 3], vec![1, 4, 5]);
        assert_eq!(p1, (drained, v), "P1: drain");

        // P2. Pushing within reserved capacity: does the buffer move?
        let mut v = Vec::with_capacity(4);
        v.push(1);
        let before = v.as_ptr();
        v.push(2);
        let p2: bool = true; // true if the buffer stayed put
        assert_eq!(p2, v.as_ptr() == before, "P2: push within capacity");

        // P3. clear() on 100 elements: the length, and whether capacity >= 100 remains.
        let mut v = vec![0u8; 100];
        v.clear();
        let p3: (usize, bool) = (0, true);
        assert_eq!(
            p3,
            (v.len(), v.capacity() >= 100),
            "P3: clear keeps the buffer"
        );

        // P4. get_disjoint_mut asks for several `&mut` elements in one call and
        // checks the indices at run time. Which of these three calls return Ok?
        let mut v = [10, 20, 30];
        let p4: (bool, bool, bool) = (true, false, false);
        assert_eq!(
            p4,
            (
                v.get_disjoint_mut([0, 2]).is_ok(),
                v.get_disjoint_mut([1, 1]).is_ok(),
                v.get_disjoint_mut([2, 3]).is_ok()
            ),
            "P4: get_disjoint_mut refuses overlapping and out-of-range indices"
        );
    }
}
