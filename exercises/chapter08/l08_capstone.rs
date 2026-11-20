//! LESSON 08 — Capstone: library facade and workspace
//! Run: cargo test --test chapter08 l08_
//!
//! Read first: CONCEPTS.md, section "Lesson 08".
//!
//! PREDICT before running: Which code belongs in a reusable library and which
//! belongs in a command-line binary? How can a private implementation module
//! stay free to change?
//!
//! IMPLEMENT the tasks below. Keep the tests and add one edge case.
//!
//! EXPLAIN afterward: Explain the public facade and the kind of test that
//! proves its privacy. Say exactly what a package archive contains.
//!
//! LAB: label_lab/ is this capstone as a real workspace: a library, two
//! binaries, an integration test, a feature and a build script. Start with
//! label_lab/README.md.

mod labels {
    fn validate(name: &str) -> Option<&str> {
        let name = name.trim();
        if name.is_empty() { None } else { Some(name) }
    }
    /// A. Return "tag:NAME" after private validation; reject empty labels.
    pub fn tag(name: &str) -> Option<String> {
        todo!("Implement this task")
    }
}
pub use labels::tag;
/// B. Apply the public API to all inputs in order. Return Err(zero_based_index)
/// at the first invalid name; no partial successful vector on error. Stop at
/// that name: the names after it are never tagged.
fn tag_batch(names: &[&str]) -> Result<Vec<String>, usize> {
    todo!("Implement this task")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_facade_exposes_behavior() {
        assert_eq!(tag(" 港 "), Some("tag:港".into()));
        assert_eq!(tag(" \n"), None);
        assert_eq!(tag("a:b"), Some("tag:a:b".into()));
        assert_eq!(tag(""), None);
    }
    #[test]
    fn b_library_api_is_independent_of_cli() {
        assert_eq!(
            tag_batch(&[" a ", "b"]),
            Ok(vec!["tag:a".into(), "tag:b".into()])
        );
        assert_eq!(tag_batch(&["a", "", "b"]), Err(1));
        assert_eq!(tag_batch(&[]), Ok(vec![]));
        assert_eq!(tag_batch(&["a", "b", "\t"]), Err(2));
        assert_eq!(tag_batch(&[" ", ""]), Err(0), "the FIRST invalid index");
    }
    #[test]
    fn b_batch_stops_at_the_first_invalid_name() {
        let mut names = vec![""];
        names.extend(std::iter::repeat_n("valid", 100));
        let (result, allocations) = count(|| tag_batch(&names));
        assert_eq!(result, Err(0));
        assert!(
            allocations <= 1,
            "names after the error should not be tagged; got {allocations} allocations"
        );
    }
}
