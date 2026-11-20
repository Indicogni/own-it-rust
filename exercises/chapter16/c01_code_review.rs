//! CODE REVIEW 01 — A teammate's pull request
//! Run: cargo test --test chapter16 c01_
//! Then: cargo clippy --test chapter16
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and some behavior tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every needless clone or allocation, and every place a
//!    different collection or std API would be clearer or faster.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. Change parameter types, return types and trait bounds only where
//!    the doc comment says so; the tests are written to accept the better
//!    signature.
//! 4. Run clippy and resolve its warnings for this file.

use std::collections::HashMap;

/// Count whitespace-separated words (case-sensitive). Return (word, count)
/// pairs sorted by count descending, then by word ascending.
/// You may change the return type to borrow the words from `text`.
fn word_counts(text: &str) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for word in text.split_whitespace() {
        if counts.contains_key(&word.to_string()) {
            *counts.get_mut(&word.to_string()).unwrap() += 1;
        } else {
            counts.insert(word.to_string(), 1);
        }
    }
    let mut out: Vec<(String, usize)> = counts.iter().map(|(w, c)| (w.clone(), *c)).collect();
    out.sort_by(|a, b| b.1.cmp(&a.1));
    out
}

/// Return the last `limit` distinct events, most recent first.
/// The result must borrow from `events`: change the return type to `Vec<&T>`
/// (a technique test fails while events are copied). You may also change the
/// parameter type and add the bounds `Eq + Hash` to `T`.
fn recent_unique<T: Clone + PartialEq>(events: &Vec<T>, limit: usize) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    for event in events.iter().rev() {
        if !out.contains(event) {
            out.push(event.clone());
        }
        if out.len() > limit {
            break;
        }
    }
    out
}

/// Index users by id; when an id repeats, the later user wins.
/// Keep this signature: the caller hands the vector over.
fn index_by_id(users: Vec<(u32, String)>) -> HashMap<u32, String> {
    let mut map = HashMap::new();
    for user in users.iter() {
        map.insert(user.0, user.1.clone());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::cell::Cell;
    use std::hash::{Hash, Hasher};

    fn pairs<S: AsRef<str>>(v: &[(S, usize)]) -> Vec<(&str, usize)> {
        v.iter().map(|(w, c)| (w.as_ref(), *c)).collect()
    }

    fn strs<S: AsRef<str>>(v: &[S]) -> Vec<&str> {
        v.iter().map(AsRef::as_ref).collect()
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_word_counts_sorts_by_count() {
        assert_eq!(pairs(&word_counts("b a b")), [("b", 2), ("a", 1)]);
        assert!(word_counts("  ").is_empty());
    }

    #[test]
    fn a_word_counts_breaks_ties_by_word() {
        let text = "q w e r t y u i o p a s d f g h j k l z x c v b n m m";
        let out = word_counts(text);
        let words: Vec<&str> = pairs(&out).into_iter().map(|(w, _)| w).collect();
        assert_eq!(
            words.concat(),
            "mabcdefghijklnopqrstuvwxyz",
            "equal counts must not come out in hash order"
        );
    }

    #[test]
    fn a_word_counts_does_not_allocate_per_word() {
        let text = "to be or not to be ".repeat(100);
        let (out, allocations) = count(|| pairs(&word_counts(&text)).len());
        assert_eq!(out, 4);
        assert!(
            allocations <= 12,
            "600 words cost {allocations} allocations; a lookup must not build \
             a String, and the map can borrow its keys from `text`"
        );
    }

    #[test]
    fn b_recent_unique_respects_the_limit() {
        let events = strings(&["a", "b", "a", "c", "b"]);
        assert_eq!(strs(&recent_unique(&events, 9)), ["b", "c", "a"]);
        assert_eq!(
            strs(&recent_unique(&events, 2)),
            ["b", "c"],
            "a limit of 2 means at most 2 events"
        );
        assert!(recent_unique(&events, 0).is_empty());
        assert!(recent_unique(&Vec::<String>::new(), 3).is_empty());
    }

    #[test]
    fn b_recent_unique_does_not_copy_events() {
        let events: Vec<String> = (0..300).map(|i| format!("event{}", i % 100)).collect();
        let (out, allocations) = count(|| strs(&recent_unique(&events, 100)).len());
        assert_eq!(out, 100);
        assert!(
            allocations <= 20,
            "100 results cost {allocations} allocations. The result must borrow \
             the events (`Vec<&T>`), not clone each one into a new String"
        );
    }

    thread_local! {
        static COMPARISONS: Cell<usize> = const { Cell::new(0) };
    }
    /// An event that counts how often it is compared with `==`.
    #[derive(Clone, Debug)]
    struct Probe(u32);
    impl PartialEq for Probe {
        fn eq(&self, other: &Self) -> bool {
            COMPARISONS.set(COMPARISONS.get() + 1);
            self.0 == other.0
        }
    }
    impl Eq for Probe {}
    impl Hash for Probe {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.0.hash(state);
        }
    }

    #[test]
    fn b_recent_unique_does_not_rescan_its_output() {
        let events: Vec<Probe> = (0..2000).map(Probe).collect();
        COMPARISONS.set(0);
        let len = recent_unique(&events, 2000).len();
        let comparisons = COMPARISONS.get();
        assert_eq!(len, 2000);
        assert!(
            comparisons <= 20_000,
            "2000 distinct events needed {comparisons} `==` comparisons. Searching \
             the output vector for every event costs about 2 000 000; remember the \
             events you have seen in a HashSet instead"
        );
    }

    #[test]
    fn c_index_by_id_keeps_the_later_user() {
        let users = vec![(1, "ada".to_string()), (2, "bo".into()), (1, "cy".into())];
        let map = index_by_id(users);
        assert_eq!(map.len(), 2);
        assert_eq!((map[&1].as_str(), map[&2].as_str()), ("cy", "bo"));
    }

    #[test]
    fn c_index_by_id_moves_the_names() {
        let users: Vec<(u32, String)> = (0..50).map(|i| (i, format!("user{i}"))).collect();
        let first = users[0].1.as_ptr();
        let (map, allocations) = count(|| index_by_id(users));
        assert_eq!(
            map[&0].as_ptr(),
            first,
            "the map must hold the caller's String (same text buffer), not a clone of it"
        );
        assert!(
            allocations <= 8,
            "50 users cost {allocations} allocations; only the map's table should \
             be allocated, not one new String per user"
        );
    }
}
