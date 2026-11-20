//! LESSON 06 — Hash maps, tree maps, and entries
//! Run: cargo test --test chapter16 l06_
//! Read first: "Lesson 06" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does HashMap iteration order belong in a test assertion? Does
//! entry(key).or_insert_with(f) invoke f for occupied entries?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: When does a BTreeMap range query justify ordered keys? How can
//! borrowed lookup avoid temporary String allocation?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch16_scratch.rs):
//! Write an Entry match that increments an occupied count and inserts one for a vacant
//! entry. Query a String-keyed map with &str. Mutate values through values_mut without
//! changing keys.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, lab, and explanation are done.

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
/// A. Count how often each item occurs. Compare items exactly: do not normalize
/// text and do not discard empty strings. Look each item up once, with the
/// entry API. `contains_key` followed by `get_mut` or `insert` finds the same
/// place twice; a technique test counts how often your code hashes a key.
fn frequencies<K: Eq + Hash + Clone>(items: &[K]) -> HashMap<K, usize> {
    todo!("Task 1: implement the documented contract")
}
/// B. Return entries sorted by key, without relying on hash iteration order.
fn ordered(map: &HashMap<String, usize>) -> Vec<(String, usize)> {
    todo!("Task 2: implement the documented contract")
}
/// C. Return owned copies of tree-map entries with start<=key<end, in key order.
/// Callers guarantee start<=end (BTreeMap::range panics otherwise).
fn in_range(map: &BTreeMap<i32, String>, start: i32, end: i32) -> Vec<(i32, String)> {
    todo!("Task 3: implement the documented contract")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, guess};
    use std::cell::Cell;
    use std::hash::Hasher;
    #[test]
    fn a_entry_counts_duplicates_and_empty_keys() {
        let map = frequencies(&["b", "a", "b", ""]);
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("b"), Some(&2));
        assert_eq!(map.get(""), Some(&1));
        let exact = frequencies(&["Go", "go", "go", "é", "e\u{301}"]);
        assert_eq!(exact.len(), 4, "case and Unicode forms stay distinct");
        assert_eq!((exact["go"], exact["Go"]), (2, 1));
        assert!(frequencies::<&str>(&[]).is_empty());
        let numbers = frequencies(&[3_u8, 1, 3, 3]);
        assert_eq!((numbers[&3], numbers[&1], numbers.len()), (3, 1, 2));
    }

    thread_local! {
        static HASHES: Cell<usize> = const { Cell::new(0) };
    }
    /// A key that counts how often it is hashed.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Probe(u32);
    impl Hash for Probe {
        fn hash<H: Hasher>(&self, state: &mut H) {
            HASHES.set(HASHES.get() + 1);
            self.0.hash(state);
        }
    }
    #[test]
    fn a_each_item_is_looked_up_once() {
        let items: Vec<Probe> = (0..1000).map(|i| Probe(i % 4)).collect();
        HASHES.set(0);
        let map = frequencies(&items);
        let hashes = HASHES.get();
        assert_eq!(map.len(), 4);
        assert_eq!(map[&Probe(3)], 250);
        assert!(
            hashes <= 1024,
            "1000 items need about 1000 hash computations, one lookup per item, \
             but keys were hashed {hashes} times. `contains_key` followed by \
             `get_mut` or `insert` hashes the key twice; `entry` hashes it once"
        );
    }
    #[test]
    fn b_output_order_is_explicit() {
        assert_eq!(
            ordered(&HashMap::from([("z".into(), 1), ("a".into(), 2)])),
            [("a".into(), 2), ("z".into(), 1)]
        );
        let many: HashMap<String, usize> = (0..30).map(|i| (format!("k{i:02}"), i)).collect();
        let out = ordered(&many);
        assert_eq!(out.len(), 30);
        assert!(
            out.windows(2).all(|w| w[0].0 < w[1].0),
            "the output must be sorted by key; a HashMap iterates in an unspecified order"
        );
        assert_eq!(out[7], ("k07".into(), 7));
    }
    #[test]
    fn c_tree_range_is_ordered_and_half_open() {
        let map = BTreeMap::from([(3, "c".into()), (1, "a".into()), (2, "b".into())]);
        assert_eq!(in_range(&map, 1, 3), [(1, "a".into()), (2, "b".into())]);
        assert!(
            in_range(&map, 2, 2).is_empty(),
            "start == end is an empty range"
        );
        assert_eq!(in_range(&map, -10, 10).len(), 3);
        assert_eq!(in_range(&map, 3, 4), [(3, "c".into())]);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. How many times does the closure run?
        let mut map = HashMap::new();
        let mut calls = 0;
        for word in ["a", "b", "a", "a"] {
            *map.entry(word).or_insert_with(|| {
                calls += 1;
                0
            }) += 1;
        }
        let p1: usize = guess();
        assert_eq!(p1, calls, "P1: or_insert_with runs only for vacant entries");

        // P2. insert on an existing key: what does it return, and what is stored?
        let mut map = HashMap::new();
        map.insert("k", 1);
        let old = map.insert("k", 2);
        let p2: (Option<i32>, i32) = guess();
        assert_eq!(p2, (old, map["k"]), "P2: insert replaces");

        // P3. BTreeMap iterates in key order. In what order are these keys?
        let tree = BTreeMap::from([("b", 1), ("a", 2), ("C", 3)]);
        let p3: Vec<&str> = guess();
        assert_eq!(
            p3,
            tree.keys().copied().collect::<Vec<_>>(),
            "P3: str ordering"
        );

        // P4. Allocations made by looking up a String key with a &str.
        let owned = HashMap::from([("key".to_string(), 1)]);
        let (found, allocations) = count(|| owned.get("key").copied());
        let p4: usize = guess();
        assert_eq!(p4, allocations, "P4: borrowed lookup");
        assert_eq!(found, Some(1));
    }
}
