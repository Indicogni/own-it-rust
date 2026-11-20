//! LESSON 03 — Sorting, binary search, and slices
//! Run: cargo test --test chapter16 l03_
//! Read first: "Lesson 03" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does binary_search choose the first duplicate? Can
//! binary_search be used on an unsorted slice?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why does stable sort preserve equal-key ordering? When can an
//! unstable sort be preferable?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch16_scratch.rs):
//! Compare sort_by_key and sort_unstable_by_key on equal keys: describe the guarantee
//! rather than assuming one observed permutation. Test slice equality and lexical
//! comparison on unequal lengths. Add binary_search_by_key for records.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, lab, and explanation are done.

/// A. Stable ascending score order; retain original order for equal scores.
fn rank(records: &mut [(i32, String)]) {
    // `sort_by_key` is stable. The key is the score only, so records with
    // equal scores are never compared by label and keep their input order.
    records.sort_by_key(|record| record.0);
}
/// B. On sorted input, return the half-open range containing all target matches.
/// If absent, return the empty range at the insertion point.
fn equal_range(sorted: &[i32], target: i32) -> std::ops::Range<usize> {
    let start = sorted.partition_point(|&value| value < target);
    let end = sorted.partition_point(|&value| value <= target);
    start..end
}
/// C. Sort and remove duplicates in place, preserving ascending order.
fn unique_sorted(values: &mut Vec<i32>) {
    // Equal integers cannot be told apart, so the faster unstable sort is safe.
    values.sort_unstable();
    values.dedup();
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, within};
    #[test]
    fn a_stable_sort_preserves_equal_score_input_order() {
        let mut r = vec![(2, "x".into()), (1, "b".into()), (1, "a".into())];
        rank(&mut r);
        assert_eq!(r, [(1, "b".into()), (1, "a".into()), (2, "x".into())]);
    }
    #[test]
    fn a_many_equal_scores_keep_input_order() {
        let mut r: Vec<(i32, String)> = (0..60).map(|i| (i % 3, format!("{i:02}"))).collect();
        rank(&mut r);
        let expected: Vec<(i32, String)> = (0..3)
            .flat_map(|s| {
                (0..60)
                    .filter(move |i| i % 3 == s)
                    .map(move |i| (s, format!("{i:02}")))
            })
            .collect();
        assert_eq!(
            r, expected,
            "records with the same score must stay in input order (a stable sort by score only)"
        );
    }
    #[test]
    fn b_duplicate_boundaries_are_deterministic() {
        // `within` fails the test with a message if a hand-written search loop never ends.
        within(10, "the search in equal_range", || {
            let a = [1, 2, 2, 2, 4];
            assert_eq!(
                equal_range(&a, 2),
                1..4,
                "all three 2s, not just one of them"
            );
            assert_eq!(
                equal_range(&a, 3),
                4..4,
                "absent: the empty range at the place where 3 would be inserted"
            );
            assert_eq!(equal_range(&[], 9), 0..0);
            assert_eq!(equal_range(&a, 0), 0..0);
            assert_eq!(equal_range(&a, 9), 5..5);
            assert_eq!(equal_range(&a, 1), 0..1);
            assert_eq!(equal_range(&[7, 7, 7], 7), 0..3);
        });
    }
    #[test]
    fn b_long_runs_of_duplicates_have_exact_bounds() {
        within(10, "the search in equal_range", || {
            let sorted: Vec<i32> = (0..300).map(|i| i / 100).collect();
            assert_eq!(equal_range(&sorted, 0), 0..100);
            assert_eq!(equal_range(&sorted, 1), 100..200);
            assert_eq!(equal_range(&sorted, 2), 200..300);
        });
    }
    #[test]
    fn c_dedup_requires_sorting_for_global_uniqueness() {
        let mut a = vec![3, 1, 3, 2, 1];
        unique_sorted(&mut a);
        assert_eq!(a, [1, 2, 3]);
        let mut empty: Vec<i32> = vec![];
        unique_sorted(&mut empty);
        assert!(empty.is_empty());
    }
    #[test]
    fn c_unique_sorted_works_in_place() {
        let mut a: Vec<i32> = (0..100).map(|i| (i * 37) % 50).collect();
        let p = a.as_ptr();
        let (_, allocations) = count(|| unique_sorted(&mut a));
        assert_eq!(a, (0..50).collect::<Vec<_>>());
        assert_eq!(
            a.as_ptr(),
            p,
            "sort and dedup must work in the vector's own buffer; \
             a new address means the result was built somewhere else"
        );
        assert!(
            allocations <= 1,
            "no set and no second vector are needed (a stable sort may use one \
             scratch buffer); got {allocations} allocations"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. binary_search for an absent and a present value.
        let sorted = [1, 3, 5];
        let p1: (Result<usize, usize>, Result<usize, usize>) = (Err(2), Ok(2));
        assert_eq!(
            p1,
            (sorted.binary_search(&4), sorted.binary_search(&5)),
            "P1: Ok(index) or Err(insertion point)"
        );

        // P2. Slices compare lexicographically, element by element.
        let p2: (bool, bool, bool) = (true, true, false);
        assert_eq!(
            p2,
            (
                [1, 2, 3][..] < [1, 3][..],
                [2][..] > [1, 9, 9][..],
                [1, 2, 0][..] < [1, 2][..]
            ),
            "P2: slice ordering"
        );

        // P3. dedup on unsorted input.
        let mut v = vec![1, 1, 2, 1, 1];
        v.dedup();
        let p3: Vec<i32> = vec![1, 2, 1];
        assert_eq!(p3, v, "P3: dedup removes consecutive repeats");
    }
}
