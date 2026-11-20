//! CODE REVIEW 01 — A teammate's pull request
//! Run: cargo test --test chapter15 c01_
//! Then: cargo clippy --test chapter15
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and some behavior tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every intermediate collection, and every loop an
//!    iterator adapter would say more clearly.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. You MAY change parameter and return types where the doc comment
//!    says so; the tests are written to accept the better signature.
//! 4. Run clippy and resolve its warnings for this file.

use std::collections::HashSet;

/// Average of the tokens that parse as f64 after trimming; other tokens are
/// ignored. None when no token parses. Keep this signature.
fn average(tokens: &[&str]) -> Option<f64> {
    // One pass carries the sum and the count, so no vector is needed.
    let (total, count) = tokens
        .iter()
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .fold((0.0, 0u32), |(total, count), value| {
            (total + value, count + 1)
        });
    // Without this check, empty input would give 0.0 / 0.0, which is NaN.
    (count > 0).then(|| total / f64::from(count))
}

/// The first `n` distinct words, in order of first appearance. Stop reading
/// once `n` are found. You may change the parameter and return types.
fn first_distinct(words: &[String], n: usize) -> Vec<&str> {
    // `insert` returns false for a word that is already in the set. `take`
    // stops pulling once `n` words have passed the filter.
    let mut seen = HashSet::new();
    words
        .iter()
        .map(String::as_str)
        .filter(|word| seen.insert(*word))
        .take(n)
        .collect()
}

/// "name: score" for each name paired with its score, for as many pairs as
/// both slices provide. Keep this signature.
fn scoreboard(names: &[&str], scores: &[u32]) -> Vec<String> {
    // zip ends with the shorter slice, so no index can be out of bounds.
    names
        .iter()
        .zip(scores)
        .map(|(name, score)| format!("{name}: {score}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_average_ignores_bad_tokens() {
        assert_eq!(average(&["1", " x", " 2 "]), Some(1.5));
        assert_eq!(average(&["-4"]), Some(-4.0));
    }

    #[test]
    fn a_average_of_nothing_is_none() {
        assert_eq!(average(&[]), None);
        assert_eq!(average(&["x", ""]), None);
    }

    #[test]
    fn a_average_needs_no_intermediate_vector() {
        let tokens: Vec<String> = (1..=100).map(|i| i.to_string()).collect();
        let tokens: Vec<&str> = tokens.iter().map(String::as_str).collect();
        let (mean, allocations) = count(|| average(&tokens));
        assert_eq!(mean, Some(50.5));
        assert_eq!(
            allocations, 0,
            "average made {allocations} heap allocation(s); one pass can carry the \
             sum and the count"
        );
    }

    #[test]
    fn b_first_distinct_keeps_first_appearance_order() {
        let words = strings(&["b", "a", "b", "c", "a"]);
        assert_eq!(first_distinct(&words, 2), ["b", "a"]);
        assert_eq!(first_distinct(&words, 9), ["b", "a", "c"]);
        assert!(first_distinct(&words, 0).is_empty());
    }

    #[test]
    fn b_first_distinct_stops_early_without_copying_everything() {
        let words: Vec<String> = (0..1000).map(|i| format!("w{i}")).collect();
        let (firsts, allocations) = count(|| first_distinct(&words, 2).len());
        assert_eq!(firsts, 2);
        assert!(
            allocations <= 6,
            "finding 2 words cost {allocations} allocations"
        );
    }

    #[test]
    fn c_scoreboard_pairs_names_with_scores() {
        assert_eq!(scoreboard(&["ada", "bo"], &[3, 10]), ["ada: 3", "bo: 10"]);
        assert_eq!(scoreboard(&["ada"], &[3, 10]), ["ada: 3"]);
        assert!(scoreboard(&[], &[1]).is_empty());
    }

    #[test]
    fn c_scoreboard_stops_at_the_shorter_slice() {
        assert_eq!(scoreboard(&["ada", "bo", "cy"], &[3]), ["ada: 3"]);
        assert!(scoreboard(&["ada"], &[]).is_empty());
    }
}
