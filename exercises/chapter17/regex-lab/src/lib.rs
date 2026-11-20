#![allow(dead_code, unused_variables, unused_imports)]
use regex::Regex;
use std::sync::{
    LazyLock,
    atomic::{AtomicUsize, Ordering},
};
/// Supplied: how many times the initializer of `PAIRS` has run.
static BUILDS: AtomicUsize = AtomicUsize::new(0);
/// Supplied: how many times `build_pattern` has been called.
static COMPILES: AtomicUsize = AtomicUsize::new(0);
/// Supplied: the one shared regex. It is compiled on first use.
static PAIRS: LazyLock<Regex> = LazyLock::new(|| {
    BUILDS.fetch_add(1, Ordering::SeqCst);
    build_pattern()
});
/// A. ASCII lowercase key=[a-z]+, value=[0-9]+, separated by '='.
/// Name groups key and value. Search substrings, with no whole-text anchors.
fn build_pattern() -> Regex {
    // Supplied: counts compilations for test f. Keep this line.
    COMPILES.fetch_add(1, Ordering::SeqCst);
    todo!("Task 1: implement the documented contract")
}
/// B. Use the shared PAIRS regex. Return (start_byte,end_byte,whole_match).
fn matches(text: &str) -> Vec<(usize, usize, String)> {
    todo!("Task 2: implement the documented contract")
}
/// C. Return (key,parsed_u32) for each match in order; skip numeric overflow.
fn values(text: &str) -> Vec<(String, u32)> {
    todo!("Task 3: implement the documented contract")
}
/// D. Rewrite every match as key:value using named capture replacement.
fn colons(text: &str) -> String {
    todo!("Task 4: implement the documented contract")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_pattern_has_named_ascii_groups() {
        let re = build_pattern();
        let c = re.captures("cpu=12").unwrap();
        assert_eq!(&c["key"], "cpu");
        assert_eq!(&c["value"], "12");
        assert!(!re.is_match("CPU=12"));
        assert!(!re.is_match("cpu=٧"));
        assert!(!re.is_match("cpu="));
    }
    #[test]
    fn b_find_reports_byte_ranges_and_whole_matches() {
        assert_eq!(
            matches("é cpu=12 mem=3"),
            [(3, 9, "cpu=12".into()), (10, 15, "mem=3".into())]
        );
        assert!(matches("no pairs").is_empty());
    }
    #[test]
    fn c_capture_parsing_separates_shape_from_value_range() {
        assert_eq!(
            values("cpu=12 big=4294967296 mem=0"),
            [("cpu".into(), 12), ("mem".into(), 0)]
        );
        assert_eq!(matches("big=4294967296").len(), 1);
    }
    #[test]
    fn d_replacement_preserves_surrounding_unicode_text() {
        assert_eq!(colons("é cpu=12, mem=3!"), "é cpu:12, mem:3!");
        assert_eq!(colons("unchanged"), "unchanged");
    }
    #[test]
    fn e_lazylock_builds_shared_regex_once_even_with_threads() {
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| assert!(PAIRS.is_match("cpu=1")));
            }
        });
        assert_eq!(
            BUILDS.load(Ordering::SeqCst),
            1,
            "the LazyLock initializer must run exactly once, whichever thread gets there first"
        );
    }
    #[test]
    fn f_tasks_reuse_the_shared_regex() {
        for _ in 0..20 {
            assert_eq!(matches("cpu=1").len(), 1);
            assert_eq!(values("cpu=1").len(), 1);
            assert_eq!(colons("cpu=1"), "cpu:1");
        }
        let compiles = COMPILES.load(Ordering::SeqCst);
        assert!(
            compiles >= 1,
            "keep the supplied COMPILES line at the top of build_pattern"
        );
        // One compilation belongs to PAIRS and one to test a, which calls
        // build_pattern directly. Sixty task calls must not add sixty more.
        assert!(
            compiles <= 4,
            "the pattern was compiled {compiles} times; matches, values and colons \
             must use the shared PAIRS regex instead of calling build_pattern again"
        );
    }
}
