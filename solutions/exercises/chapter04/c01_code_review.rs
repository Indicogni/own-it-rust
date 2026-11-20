//! CODE REVIEW 01 — A teammate's pull request
//! Run: cargo test --test chapter04 c01_
//! Then: cargo clippy --test chapter04
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! it looks reasonable, and one of its tests already passes. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug and every unnecessary allocation or ownership transfer.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. You MAY change parameter and return types where the doc comment
//!    says so; the tests are written to accept the better signature.
//! 4. Run clippy and resolve its warnings for this file.

/// Return the name with the most Unicode scalar values (`char`s); ties go to
/// the earliest. None for an empty list. You may change the parameter and
/// return types.
fn longest(names: &[String]) -> Option<&str> {
    // `max_by_key` returns the LAST of several equal maxima. Walking the
    // names in reverse turns that into the earliest one.
    names
        .iter()
        .rev()
        .max_by_key(|name| name.chars().count())
        .map(String::as_str)
}

/// Join labels with ", " (no trailing separator). Keep this signature.
fn join_labels(labels: &[String]) -> String {
    // `join` puts the separator only between items and sizes the result once.
    labels.join(", ")
}

/// Move every job starting with "urgent:" to the front, keeping the relative
/// order inside both groups. Move the Strings; do not clone them. You may
/// change the parameter type.
fn urgent_first(jobs: &mut [String]) {
    // `false` sorts before `true`, and `sort_by_key` is stable, so urgent jobs
    // come first and both groups keep their order. Sorting moves the String
    // headers; no text is copied.
    jobs.sort_by_key(|job| !job.starts_with("urgent:"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    // Accept both the original Option<String> and an improved Option<&str>.
    fn text(value: Option<impl AsRef<str>>) -> Option<String> {
        value.map(|v| v.as_ref().to_string())
    }

    fn length(value: Option<impl AsRef<str>>) -> Option<usize> {
        value.map(|v| v.as_ref().len())
    }

    #[test]
    fn a_longest_counts_scalars_and_keeps_the_first_tie() {
        let names = strings(&["ééé", "abcd", "wxyz"]);
        assert_eq!(text(longest(&names)).as_deref(), Some("abcd"));
        assert_eq!(
            text(longest(&strings(&["ab", "cd"]))).as_deref(),
            Some("ab")
        );
        assert_eq!(text(longest(&Vec::new())), None);
    }

    #[test]
    fn a_longest_borrows_instead_of_copying() {
        let names = strings(&["one", "three", "five"]);
        let (best, allocations) = count(|| length(longest(&names)));
        assert_eq!(best, Some(5));
        assert_eq!(
            allocations, 0,
            "longest made {allocations} heap allocation(s); a search can return a borrow \
             into its input and allocate nothing"
        );
    }

    #[test]
    fn b_join_has_no_trailing_separator() {
        assert_eq!(join_labels(&strings(&["a", "b", "c"])), "a, b, c");
        assert_eq!(join_labels(&strings(&["solo"])), "solo");
        assert_eq!(join_labels(&[]), "");
    }

    #[test]
    fn b_join_does_not_reallocate_per_label() {
        let labels: Vec<String> = (0..100).map(|i| format!("label{i}")).collect();
        let (joined, allocations) = count(|| join_labels(&labels));
        assert!(joined.starts_with("label0, label1"));
        assert!(
            allocations <= 10,
            "join_labels made {allocations} heap allocations for 100 labels; build the result \
             in one growing String (at most 10 allocations) and do not copy it per label"
        );
    }

    #[test]
    fn c_urgent_jobs_move_to_the_front_in_order() {
        let mut jobs = strings(&["a", "urgent:1", "b", "urgent:2"]);
        urgent_first(&mut jobs);
        assert_eq!(jobs, ["urgent:1", "urgent:2", "a", "b"]);
        let mut none: Vec<String> = Vec::new();
        urgent_first(&mut none);
        assert!(none.is_empty());
    }

    #[test]
    fn c_urgent_first_moves_strings_instead_of_cloning_them() {
        let mut jobs: Vec<String> = (0..50)
            .map(|i| {
                if i % 3 == 0 {
                    format!("urgent:{i}")
                } else {
                    format!("job{i}")
                }
            })
            .collect();
        let first_urgent = jobs[0].as_ptr();
        let (_, allocations) = count(|| urgent_first(&mut jobs));
        assert_eq!(
            jobs[0].as_ptr(),
            first_urgent,
            "the first urgent job must still be the original String (same heap buffer), \
             not a copy"
        );
        assert!(
            allocations < 20,
            "urgent_first made {allocations} heap allocations for 50 jobs; reordering by moving \
             needs only a few (for temporary vectors or a sort buffer), so Strings were cloned"
        );
    }
}
