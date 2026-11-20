//! Milestone 4: all the matches, matching lines, and what they may cost
//! (SPEC 2.1, 4.4 and section 8).
//!
//! This file installs `common::Watchful` as the allocator, to count the
//! allocations that the two iterators make.

// The expectations of `find_iter` are lists of ranges. A list with one
// range in it means exactly that.
#![allow(clippy::single_range_in_vec_init)]

mod common;

use std::ops::Range;

use common::{MIRI, assert_counting_works, bounded, count_allocations, expect_all, regex};
use rgx::Regex;

#[global_allocator]
static ALLOCATOR: common::Watchful = common::Watchful;

// ---------------------------------------------------------------------------
// find_iter
// ---------------------------------------------------------------------------

#[test]
fn matches_come_from_left_to_right_and_do_not_overlap() {
    bounded(|| {
        expect_all("a", "banana", &[1..2, 3..4, 5..6]);
        expect_all("aa", "aaaaa", &[0..2, 2..4]);
        expect_all("ana", "banana", &[1..4]);
        expect_all(r"\d+", "a1b22c333", &[1..2, 3..5, 6..9]);
        expect_all("a|ab", "abab", &[0..1, 2..3]);
        expect_all("ab|a", "abaab", &[0..2, 2..3, 3..5]);
        expect_all("x", "banana", &[]);
        expect_all("x", "", &[]);
        expect_all(r"\w+", "one, two;three", &[0..3, 5..8, 9..14]);
        expect_all("[^,]+", "a,,bc,", &[0..1, 3..5]);
        expect_all(".", "ab\ncd", &[0..1, 1..2, 3..4, 4..5]);
        expect_all("é", "été", &[0..2, 3..5]);
        expect_all(".", "aé日😀", &[0..1, 1..3, 3..6, 6..10]);
    });
}

#[test]
fn the_first_match_of_find_iter_is_the_match_of_find() {
    bounded(|| {
        for (pattern, haystack) in [
            ("b+", "abbbc"),
            ("a|ab", "xab"),
            ("x*", "aaa"),
            ("$", "abc"),
            ("q", "abc"),
            (r"\b", "  hi"),
            ("", ""),
        ] {
            let regex = regex(pattern);
            assert_eq!(
                regex.find_iter(haystack).next(),
                regex.find(haystack),
                "pattern {pattern:?}, haystack {haystack:?}"
            );
        }
    });
}

#[test]
fn empty_matches_follow_the_two_rules() {
    bounded(|| {
        // SPEC 4.4, rule 1: after an empty match the search goes on one
        // character further, so the iterator always moves on.
        expect_all("", "", &[0..0]);
        expect_all("", "ab", &[0..0, 1..1, 2..2]);
        expect_all("x*", "ab", &[0..0, 1..1, 2..2]);
        expect_all("$", "ab", &[2..2]);
        expect_all("^", "ab", &[0..0]);
        expect_all(r"\b", "ab cd", &[0..0, 2..2, 3..3, 5..5]);
        expect_all(r"\B", "ab cd", &[1..1, 4..4]);
        expect_all("()", "a", &[0..0, 1..1]);
        // Rule 2: an empty match directly after the match before it is
        // not reported.
        expect_all("a*", "aaa", &[0..3]);
        expect_all("a*", "baaa", &[0..0, 1..4]);
        expect_all("a*", "aab", &[0..2, 3..3]);
        expect_all("a*", "aabaa", &[0..2, 3..5]);
        expect_all("a|", "ba", &[0..0, 1..2]);
        expect_all("a|", "ab", &[0..1, 2..2]);
        expect_all("a?", "aba", &[0..1, 2..3]);
        expect_all("b*", "abba", &[0..0, 1..3, 4..4]);
        expect_all(r"\d*", "12 3", &[0..2, 3..4]);
        expect_all("a*|b", "ab", &[0..1, 2..2]);
        expect_all("(|a){2}", "aa", &[0..0, 1..1, 2..2]);
        expect_all("(a|){2}", "aaa", &[0..2, 2..3]);
    });
}

#[test]
fn empty_matches_step_over_whole_characters() {
    bounded(|| {
        expect_all("", "é", &[0..0, 2..2]);
        expect_all("", "aé日😀", &[0..0, 1..1, 3..3, 6..6, 10..10]);
        expect_all("x*", "日本", &[0..0, 3..3, 6..6]);
        expect_all("é?", "éaé", &[0..2, 3..5]);
        expect_all("é*", "aéé", &[0..0, 1..5]);
        expect_all(r"\B", "éé", &[0..0, 2..2, 4..4]);
        // Every range that `find_iter` returns must be usable as a slice.
        let haystack = "añ😀 ñ日x";
        for pattern in ["", "x*", r"\b", r"\B", "ñ?", "[^ñ]*", ".?"] {
            for range in regex(pattern).find_iter(haystack).take(haystack.len() + 2) {
                assert!(
                    haystack.get(range.clone()).is_some(),
                    "pattern {pattern:?}, haystack {haystack:?}: {range:?} is not on character boundaries"
                );
            }
        }
    });
}

#[test]
fn later_searches_still_see_the_whole_haystack() {
    bounded(|| {
        // SPEC 4.4: the assertions of a later search look at the text
        // before the place where that search starts.
        expect_all("^a", "aaa", &[0..1]);
        expect_all("^", "aaa", &[0..0]);
        expect_all("a$", "aaa", &[2..3]);
        expect_all(r"\ba", "a aa", &[0..1, 2..3]);
        expect_all(r"\Ba", "aaa", &[1..2, 2..3]);
        expect_all(r"\b\w", "ab cd", &[0..1, 3..4]);
        expect_all(r"^\w|,\w", "a,b,c", &[0..1, 1..3, 3..5]);
        expect_all(r"\Bb", "abb", &[1..2, 2..3]);
        expect_all("(^|,)[a-z]*", "ab,,c", &[0..2, 2..3, 3..5]);
    });
}

#[test]
fn an_exhausted_iterator_stays_exhausted() {
    bounded(|| {
        let regex = regex("a*");
        let mut matches = regex.find_iter("baa");
        assert_eq!(matches.next(), Some(0..0));
        assert_eq!(matches.next(), Some(1..3));
        for _ in 0..3 {
            assert_eq!(
                matches.next(),
                None,
                "`next` after the end must keep returning None (SPEC 4.4)"
            );
        }
    });
}

#[test]
fn find_iter_searches_only_as_far_as_it_is_asked() {
    bounded(|| {
        // Every single search here has to read to the end of the haystack:
        // the preferred branch `a*b` stays possible until the last `a`. A
        // `find_iter` that looks for all the matches before it returns the
        // first needs (length)^2 / 2 steps and will not finish in time.
        // Taking three matches needs three searches (SPEC 4.4).
        let length = if MIRI { 500 } else { 300_000 };
        let haystack = "a".repeat(length);
        let regex = regex("a*b|a");
        let first_three: Vec<Range<usize>> = regex.find_iter(&haystack).take(3).collect();
        assert_eq!(first_three, [0..1, 1..2, 2..3]);
    });
}

// ---------------------------------------------------------------------------
// matching_lines
// ---------------------------------------------------------------------------

/// The matching lines as owned values, to compare with an expectation.
fn lines_of(pattern: &str, text: &str) -> Vec<(usize, String)> {
    let limit = text.len() + 2; // more lines than any text has
    regex(pattern)
        .matching_lines(text)
        .take(limit)
        .map(|(number, line)| (number, line.to_string()))
        .collect()
}

fn numbered(lines: &[(usize, &str)]) -> Vec<(usize, String)> {
    lines
        .iter()
        .map(|&(number, line)| (number, line.to_string()))
        .collect()
}

#[test]
fn matching_lines_have_numbers_and_no_terminator() {
    bounded(|| {
        let text = "alpha\nbeta\ngamma\ndelta\n";
        assert_eq!(
            lines_of("a$", text),
            numbered(&[(1, "alpha"), (2, "beta"), (3, "gamma"), (4, "delta")])
        );
        assert_eq!(
            lines_of("^[bd]", text),
            numbered(&[(2, "beta"), (4, "delta")])
        );
        assert_eq!(lines_of("mm", text), numbered(&[(3, "gamma")]));
        assert_eq!(lines_of("x", text), numbered(&[]));
        // Each line is a haystack of its own: `^` and `$` are its ends.
        assert_eq!(lines_of("^beta$", text), numbered(&[(2, "beta")]));
        assert_eq!(lines_of("alpha.beta", text), numbered(&[]));
        assert_eq!(lines_of(r"a\nb", text), numbered(&[]));
    });
}

#[test]
fn the_line_rules_of_the_spec() {
    bounded(|| {
        // SPEC 7.2. No lines at all in an empty text.
        assert_eq!(lines_of("", ""), numbered(&[]));
        assert_eq!(lines_of("^$", ""), numbered(&[]));
        // A text that ends without a line feed still ends with a line...
        assert_eq!(lines_of("b", "a\nb"), numbered(&[(2, "b")]));
        assert_eq!(lines_of("", "abc"), numbered(&[(1, "abc")]));
        // ...and one that ends with a line feed has no empty line after it.
        assert_eq!(lines_of("", "a\nb\n"), numbered(&[(1, "a"), (2, "b")]));
        assert_eq!(lines_of("^$", "a\nb\n"), numbered(&[]));
        // Empty lines are lines.
        assert_eq!(lines_of("^$", "\n"), numbered(&[(1, "")]));
        assert_eq!(
            lines_of("^$", "a\n\n\nb\n\n"),
            numbered(&[(2, ""), (3, ""), (5, "")])
        );
        assert_eq!(lines_of("", "\n\n"), numbered(&[(1, ""), (2, "")]));
        // A carriage return is an ordinary character.
        assert_eq!(lines_of("b$", "ab\r\nab\n"), numbered(&[(2, "ab")]));
        assert_eq!(lines_of(r"b\r$", "ab\r\nab\n"), numbered(&[(1, "ab\r")]));
        assert_eq!(lines_of("b", "a\rb\rc"), numbered(&[(1, "a\rb\rc")]));
        // Lines are text: characters of any length.
        assert_eq!(
            lines_of("^.{2}$", "日本\n日本語\nab\n"),
            numbered(&[(1, "日本"), (3, "ab")])
        );
    });
}

#[test]
fn the_lines_are_slices_of_the_text() {
    bounded(|| {
        let text = "first line\nsecond\n\nfourth line\nlast, without an end";
        let regex = regex("");
        let expected: Vec<&str> = text.split('\n').collect();
        let found: Vec<(usize, &str)> = regex.matching_lines(text).take(10).collect();
        assert_eq!(found.len(), expected.len());
        for ((number, line), expected) in found.into_iter().zip(expected) {
            assert_eq!(line, expected, "line {number}");
            assert!(
                std::ptr::eq(line.as_ptr(), expected.as_ptr()),
                "line {number} has the right content, but it must be a slice of the text that was passed in, \
                 not a copy of it (SPEC 8.1)"
            );
        }
    });
}

#[test]
fn the_lines_outlive_the_regex() {
    bounded(|| {
        // The items borrow from the text and not from the `Regex`: this
        // does not compile otherwise (SPEC 2.1).
        let text = String::from("keep this\ndrop this\nkeep that\n");
        let kept: Vec<&str> = {
            let regex = regex("keep");
            regex
                .matching_lines(&text)
                .take(5)
                .map(|(_, line)| line)
                .collect()
        };
        assert_eq!(kept, ["keep this", "keep that"]);
        // A text that lives for ever gives lines that live for ever.
        let forever: Vec<&'static str> = regex("o")
            .matching_lines("one\ntwo\nthree")
            .take(5)
            .map(|(_, l)| l)
            .collect();
        assert_eq!(forever, ["one", "two"]);
    });
}

#[test]
fn one_regex_serves_many_threads() {
    bounded(|| {
        // SPEC 2.1: `Regex` is `Send + Sync`, and any number of threads may
        // search with the same value at the same time.
        fn shareable<T: Send + Sync>() {}
        shareable::<Regex>();

        let regex = regex(r": \d*[02468]$");
        let per_thread = if MIRI { 40 } else { 2_000 };
        let texts: Vec<String> = (0..8)
            .map(|thread| {
                (0..per_thread)
                    .map(|n| format!("t{thread}: {}\n", n * (thread + 1)))
                    .collect()
            })
            .collect();
        let alone: Vec<Vec<usize>> = texts
            .iter()
            .map(|text| {
                regex
                    .matching_lines(text)
                    .map(|(number, _)| number)
                    .collect()
            })
            .collect();
        let together: Vec<Vec<usize>> = std::thread::scope(|scope| {
            let handles: Vec<_> = texts
                .iter()
                .map(|text| {
                    let regex = &regex;
                    scope.spawn(move || {
                        let mut numbers = Vec::new();
                        // Mix the three kinds of search on one shared value.
                        for (number, line) in regex.matching_lines(text) {
                            assert!(regex.is_match(line));
                            assert!(regex.find_iter(line).count() >= 1);
                            numbers.push(number);
                        }
                        numbers
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("a searching thread panicked"))
                .collect()
        });
        assert_eq!(
            together, alone,
            "searching from several threads at once changed the results"
        );
        assert_eq!(
            alone[0].len(),
            per_thread / 2,
            "every other line of the first text ends in an even number"
        );
    });
}

// ---------------------------------------------------------------------------
// Allocation budgets (SPEC 8.2)
// ---------------------------------------------------------------------------

/// SPEC 8.2: the most allocations that one whole pass of `find_iter` or of
/// `matching_lines` may make for a pattern of cost 50 or less.
const BUDGET: usize = 100;

#[test]
fn matching_lines_allocates_per_search_not_per_line() {
    bounded(|| {
        assert_counting_works();
        let lines = if MIRI { 400 } else { 50_000 };
        let text: String = (0..lines)
            .map(|n| format!("entry {n}: {} units\n", n % 7))
            .collect();
        // Costs 12 and 36 (SPEC 6.1), below 50.
        for (pattern, expected) in [
            (r"needle|: [36] u", 2 * lines / 7),
            (r"^entry \d*[05]: [0-6]{1,8} u(nit|x)s$", lines / 5),
        ] {
            let regex = regex(pattern);
            let (selected, allocations) = count_allocations(|| regex.matching_lines(&text).count());
            assert!(
                selected.abs_diff(expected) <= 1,
                "{pattern:?} selected {selected} lines, about {expected} expected"
            );
            assert!(
                allocations <= BUDGET,
                "one pass over {lines} lines with {pattern:?} made {allocations} allocations. \
                 The limit is {BUDGET}, whatever the number of lines (SPEC 8.2). \
                 Something allocates for every line, or for every search."
            );
        }
    });
}

#[test]
fn find_iter_allocates_per_haystack_not_per_match() {
    bounded(|| {
        assert_counting_works();
        let words = if MIRI { 400 } else { 50_000 };
        let haystack: String = (0..words).map(|n| format!("w{n} ")).collect();
        for pattern in [r"\w+", r"\b(w|x)[0-9]{1,6}\b", " *", "0|1|2|3|4|5|6|7|8|9"] {
            let regex = regex(pattern);
            let (found, allocations) = count_allocations(|| regex.find_iter(&haystack).count());
            assert!(
                found >= words,
                "{pattern:?} has at least {words} matches, {found} were found"
            );
            assert!(
                allocations <= BUDGET,
                "one pass over {found} matches of {pattern:?} made {allocations} allocations. \
                 The limit is {BUDGET}, whatever the number of matches (SPEC 8.2)."
            );
        }
    });
}

#[test]
fn the_budget_holds_for_long_lines_and_wide_characters() {
    bounded(|| {
        assert_counting_works();
        // Scratch memory must depend on the pattern, not on the haystack:
        // one very long line, then many short ones, with multi-byte text.
        let long = if MIRI { 2_000 } else { 400_000 };
        let short = if MIRI { 200 } else { 20_000 };
        let mut text = "é日x ".repeat(long / 4);
        text.push('\n');
        for n in 0..short {
            text.push_str(if n % 2 == 0 {
                "日本語 text\n"
            } else {
                "plain\n"
            });
        }
        let regex = regex(r"x é|[語-龍]\s\w+$");
        let (selected, allocations) = count_allocations(|| regex.matching_lines(&text).count());
        assert_eq!(selected, 1 + short / 2);
        assert!(
            allocations <= BUDGET,
            "a text with one line of {long} characters and {short} short lines needed {allocations} \
             allocations; the limit is {BUDGET} (SPEC 8.2)"
        );
    });
}
