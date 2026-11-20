//! LESSON 01 — Borrowed captures and sorting
//! Run: cargo test --test chapter14 l01_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Can a closure read `prefix` while the
//! caller still holds it? Does passing the closure to another function move `prefix`?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why is the captured prefix shared rather than mutable? When is the
//! borrow released?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch14_scratch.rs):
//! Create a closure borrowing a String; try mutating the String before a later closure
//! call. Remove that later call and see that the borrow ends at the closure's last use.
//! Explain why a closure that reads one struct field leaves the other fields free.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete when its tests, predictions, and explanation are done. The lab is optional.

use supplied::Shelf;

mod supplied {
    //! Supplied code. Do not change it.

    /// A word list that hides its storage. The only way to look at the words is
    /// to pass a closure to [`Shelf::select`].
    pub struct Shelf {
        words: Vec<String>,
    }

    impl Shelf {
        pub fn new(words: &[&str]) -> Self {
            Shelf {
                words: words.iter().map(|w| w.to_string()).collect(),
            }
        }

        /// Calls `keep` once per word, in order, and returns the accepted words.
        pub fn select(&self, keep: impl Fn(&str) -> bool) -> Vec<&str> {
            let mut kept = Vec::new();
            for word in &self.words {
                if keep(word) {
                    kept.push(word.as_str());
                }
            }
            kept
        }
    }
}

/// A. Keep words starting with prefix, preserving order and duplicates.
/// `Shelf` hides its words, so the only way in is `Shelf::select`: pass it a closure
/// that captures `prefix` by shared borrow (no copy of the prefix). Return owned
/// copies of the accepted words.
fn matching(shelf: &Shelf, prefix: &str) -> Vec<String> {
    // The closure only reads `prefix`, so it captures a shared borrow of it.
    // The borrow ends when `select` returns.
    shelf
        .select(|word| word.starts_with(prefix))
        .into_iter()
        .map(str::to_owned)
        .collect()
}
/// B. Sort by UTF-8 byte length, then lexical text. Mutate only the vector order:
/// the comparison closure borrows the strings and never clones them.
fn shortest_first(words: &mut [String]) {
    // `sort_by` lends both strings to the closure. `sort_by_key` would need an
    // owned key here, which means cloning a String for every comparison.
    words.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::mem::size_of_val;
    #[test]
    fn a_closure_borrows_the_prefix() {
        let shelf = Shelf::new(&["air", "sea", "air", "α"]);
        assert_eq!(matching(&shelf, "a"), ["air", "air"]);
        assert_eq!(matching(&Shelf::new(&["a", ""]), ""), ["a", ""]);
        assert!(matching(&Shelf::new(&[]), "x").is_empty());
        assert!(matching(&shelf, "αβ").is_empty());
    }
    #[test]
    fn a_closure_does_not_copy_the_prefix() {
        let words: Vec<String> = (0..100).map(|i| format!("w{i:02}")).collect();
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        let shelf = Shelf::new(&words);
        let prefix = String::from("w4");
        let (out, allocations) = count(|| matching(&shelf, &prefix));
        assert_eq!(out.len(), 10, "w40 to w49 start with w4");
        assert!(
            allocations <= 24,
            "{allocations} allocations for 10 results: the closure should borrow the \
             prefix, not build a new String for every word it tests"
        );
    }
    #[test]
    fn b_sorting_closure_has_a_deterministic_tie_break() {
        let mut v = vec!["zz".into(), "é".into(), "a".into(), "bb".into()];
        shortest_first(&mut v);
        assert_eq!(v, ["a", "bb", "zz", "é"]);
        let mut empty: Vec<String> = vec![];
        shortest_first(&mut empty);
        assert!(empty.is_empty());
    }

    #[test]
    fn b_sorting_moves_strings_without_cloning() {
        let mut v: Vec<String> = (0..100).rev().map(|i| format!("w{i}")).collect();
        let first = v[99].as_ptr(); // "w0" sorts first
        let (_, allocations) = count(|| shortest_first(&mut v));
        assert_eq!(v[0], "w0");
        assert_eq!(
            v[0].as_ptr(),
            first,
            "the first String must still own its original buffer: sorting moves the \
             String values and never copies their text"
        );
        assert!(
            allocations <= 2,
            "sorting 100 words cost {allocations} allocations; the comparison closure \
             should borrow both strings instead of cloning them"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. A closure is a value holding its captures. How many machine words
        // (one word = the size of a usize or of a thin pointer) does each take?
        // Answer as (captures_nothing, reads_table).
        let table = String::from("lookup");
        let reads_table = || table.len();
        let captures_nothing = |n: usize| n + 1;
        let p1: (usize, usize) = (0, 1);
        assert_eq!(
            (p1.0 * size_of::<usize>(), p1.1 * size_of::<usize>()),
            (size_of_val(&captures_nothing), size_of_val(&reads_table)),
            "P1: sizes in bytes (your words x word size, actual); a shared-borrow \
             capture is one pointer"
        );

        // P2. This closure reads two fields of `point`. Machine words of captured state?
        let point = (3i32, 4i32);
        let sum_fields = || point.0 + point.1;
        let p2: usize = 2;
        assert_eq!(
            p2 * size_of::<usize>(),
            size_of_val(&sum_fields),
            "P2: sizes in bytes (your words x word size, actual); closures capture \
             places (fields), not whole variables"
        );
        assert_eq!(sum_fields(), 7);

        // P3. sort_by_key is stable. What order results?
        let mut words = vec!["bb", "a", "cc", "d"];
        words.sort_by_key(|w| w.len());
        let p3: Vec<&str> = vec!["a", "d", "bb", "cc"];
        assert_eq!(p3, words, "P3: equal keys keep their input order");
    }
}
