//! A test of the reference solution's own, not an acceptance test: it uses
//! `Regex::matcher`, which is this solution's design and not part of the
//! facade of SPEC section 2.
//!
//! The allocation budget of SPEC 8.2 allows 100 allocations per pass. The
//! solution's own rule is stricter and easier to keep: **making a matcher
//! allocates a fixed number of times, and searching with it allocates
//! nothing**, whatever the haystack is.

mod common;

use common::{MIRI, assert_counting_works, bounded, count_allocations};
use rgx::Regex;

#[global_allocator]
static ALLOCATOR: common::Watchful = common::Watchful;

const PATTERNS: [&str; 6] = [
    "",
    "needle",
    r"\b(w|x)[0-9]{1,6}\b",
    "(a|ab)(c|bcd)*$",
    r"[^\s,]+@[a-zé-ω]+\.(org|net)",
    "(x+)*y|.{0,40}z",
];

/// Sixty groups inside each other, each one a loop around a choice. Walking
/// into such a pattern is where a work list grows deepest.
fn deeply_nested() -> String {
    let mut pattern = String::from("w");
    for level in 0..60 {
        pattern = format!("({pattern}|{})+", level % 10);
    }
    pattern
}

#[test]
fn a_search_allocates_nothing_after_the_matcher_is_made() {
    bounded(|| {
        assert_counting_works();
        let words = if MIRI { 40 } else { 1_000 };
        let haystack: String = (0..words).map(|n| format!("w{n} é,x@a.org ")).collect();
        let nested = deeply_nested();
        for pattern in PATTERNS.into_iter().chain([nested.as_str()]) {
            let regex = Regex::new(pattern).expect("the patterns above are valid");
            let mut matcher = regex.matcher();
            let ((), allocations) = count_allocations(|| {
                std::hint::black_box(matcher.is_match(&haystack));
                std::hint::black_box(matcher.find(&haystack));
                std::hint::black_box(matcher.find_iter(&haystack).count());
                std::hint::black_box(matcher.find_at("", 0));
            });
            assert_eq!(
                allocations, 0,
                "searching with a matcher for {pattern:?} must not allocate"
            );
        }
    });
}

#[test]
fn making_a_matcher_allocates_a_fixed_number_of_times() {
    bounded(|| {
        assert_counting_works();
        let small = Regex::new("a").expect("valid");
        let copies = if MIRI { 200 } else { 10_000 };
        let large = Regex::new(&"(a|b)?".repeat(copies)).expect("valid");
        let (_, for_small) = count_allocations(|| std::hint::black_box(small.matcher()));
        let (_, for_large) = count_allocations(|| std::hint::black_box(large.matcher()));
        assert_eq!(
            for_small, for_large,
            "the size of the scratch memory depends on the pattern, the number of its allocations must not"
        );
        assert!(
            for_small <= 8,
            "a matcher is two state sets and a stack; it took {for_small} allocations"
        );
    });
}
