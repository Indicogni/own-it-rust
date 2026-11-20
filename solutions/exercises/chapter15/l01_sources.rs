//! LESSON 01 — Borrowed, mutable, owned, and draining sources
//! Run: cargo test --test chapter15 l01_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: What are the Item types of slice.iter(),
//! iter_mut(), and Vec::into_iter()? When are elements removed by drain?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why can reading and editing use different iterator types over the
//! same container? Who owns elements returned by drain?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch15_scratch.rs):
//! Call into_iter on a Vec<String>, then try to use the vector. Compare .iter(). For drain,
//! create a drain over 1..3, consume one item, then drop it; inspect the remaining vector.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, and explanation are done.

/// A. Sum borrowed i32 values into i64 without consuming the source.
fn borrowed_sum(values: &[i32]) -> i64 {
    values.iter().map(|&v| i64::from(v)).sum()
}
/// B. Negate each value through iter_mut. Inputs exclude i32::MIN.
fn negate(values: &mut [i32]) {
    values.iter_mut().for_each(|v| *v = -*v);
}
/// C. Drain valid start..end from an owned vector; return (removed,remaining).
fn drain_range(mut values: Vec<String>, start: usize, end: usize) -> (Vec<String>, Vec<String>) {
    // The Drain borrows `values`; it is dropped at the end of this statement,
    // and only then is `values` free to be moved into the result.
    let removed: Vec<String> = values.drain(start..end).collect();
    (removed, values)
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_borrowed_source_widens_before_sum() {
        let v = vec![i32::MAX, i32::MAX, -1];
        assert_eq!(borrowed_sum(&v), 4_294_967_293);
        assert_eq!(v.len(), 3, "the caller still owns all three values");
        assert_eq!(borrowed_sum(&[]), 0);
        assert_eq!(borrowed_sum(&[i32::MIN, i32::MIN]), -4_294_967_296);
    }
    #[test]
    fn a_borrowed_sum_does_not_allocate() {
        let v: Vec<i32> = (0..1000).collect();
        let (total, allocations) = count(|| borrowed_sum(&v));
        assert_eq!(total, 499_500);
        assert_eq!(
            allocations, 0,
            "borrowed_sum made {allocations} heap allocation(s); sum straight from \
             the iterator instead of collecting into a Vec first"
        );
    }
    #[test]
    fn b_mutable_source_edits_elements() {
        let mut v = [-2, 0, 7];
        negate(&mut v);
        assert_eq!(v, [2, 0, -7]);
    }
    #[test]
    fn c_drain_transfers_only_selected_elements() {
        assert_eq!(
            drain_range(vec!["a".into(), "b".into(), "c".into(), "d".into()], 1, 3),
            (vec!["b".into(), "c".into()], vec!["a".into(), "d".into()])
        );
        assert_eq!(drain_range(vec![], 0, 0), (vec![], vec![]));
    }
    #[test]
    fn c_drained_strings_move_without_cloning() {
        let v: Vec<String> = vec!["keep".into(), "move".into(), "keep".into()];
        let moved = v[1].as_ptr();
        let (removed, remaining) = drain_range(v, 1, 2);
        assert_eq!(
            removed[0].as_ptr(),
            moved,
            "drain_range must move the original String out (same heap buffer), not clone it"
        );
        assert_eq!(remaining, ["keep", "keep"]);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. A Drain dropped after one item: what is left in the vector?
        let mut v = vec![1, 2, 3, 4, 5];
        {
            let mut drain = v.drain(1..4);
            drain.next();
        }
        let p1: Vec<i32> = vec![1, 5];
        assert_eq!(p1, v, "P1: contents of v after the Drain is dropped");

        // P2. Does each item keep its original heap buffer?
        // (via into_iter(), via to_vec())
        let words = vec![String::from("one")];
        let original = words[0].as_ptr();
        let cloned: Vec<String> = words.to_vec();
        let moved: Vec<String> = words.into_iter().collect();
        let p2: (bool, bool) = (true, false);
        assert_eq!(
            p2,
            (
                moved[0].as_ptr() == original,
                cloned[0].as_ptr() == original
            ),
            "P2: (moved still uses the original buffer, cloned still uses the original buffer)"
        );

        // P3. Heap allocations made by a map-then-sum pipeline over 1000 values.
        let v: Vec<i32> = (0..1000).collect();
        let (total, allocations) = count(|| v.iter().map(|x| x * 2).sum::<i32>());
        let p3: usize = 0;
        assert_eq!(p3, allocations, "P3: heap allocations made by map + sum");
        assert_eq!(total, 999_000);
    }
}
