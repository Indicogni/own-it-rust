//! LESSON 09 — Capstone: borrowed roster queries
//! Run: cargo test --test chapter05 l09_
//! Read first: CONCEPTS.md, section "Lesson 09 · Capstone: borrowed roster queries".
//!
//! PREDICT before running: Why might an index be easier to store than a reference when a collection must later change?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Compare returning a borrow, returning an index, and cloning an owner. Explain invalidation of indices on removal and the caller obligations of each API.
//! COMPILER / TRANSFER LAB (optional; tests do not check it): Design an editable roster with selection. First try storing a reference to a vector element beside the vector; then store an index. Explain how deletion and sorting affect the index and propose a stable-ID alternative.
//! Use examples/ch05_scratch.rs; repair intentional failures afterward.

/// A. Find the FIRST longest label by Unicode scalar count. Return its index,
/// or None when empty. Allocate no copies of the labels.
fn longest_index(labels: &[String]) -> Option<usize> {
    // Track (index, scalar count) and replace it only for a strictly greater
    // count, so the first of several equal labels wins.
    let mut best: Option<(usize, usize)> = None;
    for (index, label) in labels.iter().enumerate() {
        let scalars = label.chars().count();
        if best.is_none_or(|(_, most)| scalars > most) {
            best = Some((index, scalars));
        }
    }
    best.map(|(index, _)| index)
}
/// B. Return the trimmed borrowed label at an index, or None if invalid.
/// Do not allocate. The output must not be tied to any unrelated argument.
fn label_at(labels: &[String], index: usize) -> Option<&str> {
    labels.get(index).map(|label| label.trim())
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_selection_uses_scalar_count_and_first_tie() {
        let a = vec!["🦀".into(), "ab".into(), "cd".into()];
        assert_eq!(longest_index(&a), Some(1));
        assert_eq!(longest_index(&[]), None);
        assert_eq!(longest_index(&["".into(), "".into()]), Some(0));
    }
    #[test]
    fn b_labels_are_borrowed_and_checked() {
        let a = vec![" 港 ".into()];
        assert_eq!(label_at(&a, 0), Some("港"));
        assert_eq!(label_at(&a, 1), None);
        assert_eq!(label_at(&[], 0), None);
        assert_eq!(
            label_at(&a, 0).unwrap().as_ptr(),
            a[0][1..].as_ptr(),
            "the label must point into the roster's own String (after the leading space)"
        );
    }

    #[test]
    fn a_b_queries_never_copy_labels() {
        let labels: Vec<String> = (0..50).map(|i| format!(" label {i} ")).collect();
        let (index, allocations) = count(|| longest_index(&labels));
        assert_eq!(index, Some(10));
        assert_eq!(allocations, 0, "compare lengths through borrows");
        let (label, allocations) = count(|| label_at(&labels, 7));
        assert_eq!(label, Some("label 7"));
        assert_eq!(allocations, 0, "trim returns a borrowed subslice");
    }
}
