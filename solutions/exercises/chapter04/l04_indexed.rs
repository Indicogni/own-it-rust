//! LESSON 04 — Moving from indexed collections
//! Run: cargo test --test chapter04 l04_
//! Read first: "Lesson 04" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Why can you copy `vec[0]` for integers but not move `vec[0]` for Strings?
//! 2. How does `remove` differ from `swap_remove`?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain why every slot of a vector must always hold a
//! value, and compare the order and the cost of `pop`, `remove` and
//! `swap_remove`.
//!
//! COMPILER / TRANSFER LAB (optional): Try
//! `let value = vec![String::from("x")][0];` and read the error. Repair it in
//! three separate experiments with `pop`, `remove` and `swap_remove`. Draw how
//! each one changes the indices of the other elements.
//! Use examples/ch04_scratch.rs for this, and leave it compiling afterward.

/// A. Remove a valid index while preserving order. Return None and change
/// nothing for an invalid index. Do not clone the removed String.
fn ordered_take(values: &mut Vec<String>, index: usize) -> Option<String> {
    // `remove` panics for an index that is out of range, so check first.
    (index < values.len()).then(|| values.remove(index))
}
/// B. Remove a valid index by filling its slot with the last element.
/// Return None without mutation for an invalid index. Use swap_remove.
fn unordered_take(values: &mut Vec<String>, index: usize) -> Option<String> {
    (index < values.len()).then(|| values.swap_remove(index))
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_order_is_preserved() {
        let mut v = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(ordered_take(&mut v, 1), Some("b".into()));
        assert_eq!(v, ["a", "c"]);
        assert_eq!(ordered_take(&mut v, 2), None);
        assert_eq!(v, ["a", "c"]);
        assert_eq!(ordered_take(&mut vec![], 0), None);
    }
    #[test]
    fn b_last_value_fills_the_hole() {
        let mut v = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        assert_eq!(unordered_take(&mut v, 1), Some("b".into()));
        assert_eq!(v, ["a", "d", "c"]);
        assert_eq!(unordered_take(&mut v, 9), None);
        assert_eq!(v, ["a", "d", "c"]);
        assert_eq!(unordered_take(&mut vec![], 0), None);
    }

    #[test]
    fn a_b_extraction_does_not_clone() {
        let mut v: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        let first = v[0].as_ptr();
        let (taken, allocations) = count(|| ordered_take(&mut v, 0));
        assert_eq!(
            taken.map(|s| s.as_ptr()),
            Some(first),
            "ordered_take must hand out the element itself (same heap buffer), not a copy"
        );
        assert_eq!(
            allocations, 0,
            "ordered_take made {allocations} heap allocation(s); moving an element out needs none"
        );
        let first = v[0].as_ptr();
        let (taken, allocations) = count(|| unordered_take(&mut v, 0));
        assert_eq!(
            taken.map(|s| s.as_ptr()),
            Some(first),
            "unordered_take must hand out the element itself (same heap buffer), not a copy"
        );
        assert_eq!(
            allocations, 0,
            "unordered_take made {allocations} heap allocation(s); moving an element out needs none"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Contents after swap_remove(0).
        let mut v = vec!["a", "b", "c", "d"];
        v.swap_remove(0);
        let p1: Vec<&str> = vec!["d", "b", "c"];
        assert_eq!(
            p1, v,
            "P1: swap_remove(0) (left: your prediction, right: actual)"
        );

        // P2. Contents after remove(0).
        let mut v = vec!["a", "b", "c", "d"];
        v.remove(0);
        let p2: Vec<&str> = vec!["b", "c", "d"];
        assert_eq!(
            p2, v,
            "P2: remove(0) (left: your prediction, right: actual)"
        );

        // P3. swap_remove on the LAST index: what is returned, and what is left?
        let mut v = vec!["a", "b", "c"];
        let last = v.swap_remove(2);
        let p3: (&str, Vec<&str>) = ("c", vec!["a", "b"]);
        assert_eq!(
            p3,
            (last, v),
            "P3: swap_remove of the last element (left: your prediction, right: actual)"
        );
    }
}
