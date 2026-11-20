//! Milestone 3: matching in linear time, and the limits that make it
//! possible (SPEC 4.5 and section 6).
//!
//! Nothing here measures speed. Each test gives the engine work that is
//! small when one search costs (pattern size) x (haystack length), and
//! hopeless when it costs more: a matcher that tries the possibilities one
//! after the other needs longer than the age of the universe for some of
//! these, and one that starts again at every position needs minutes. Every
//! test must finish within `common::LIBRARY_LIMIT` (30 seconds); the
//! reference solution needs less than two seconds for each in a debug build.
//!
//! This file installs `common::Watchful` as the allocator. If the test
//! program ends with "memory allocation of N bytes failed", a pattern made
//! your compiler ask for more than a gigabyte: see SPEC 6.1.

// Miri would need hours for inputs of this size, and smaller inputs prove
// nothing here.
#![cfg(not(miri))]

mod common;

use common::{bounded, expect_find, regex, rejection};
use rgx::Regex;

#[global_allocator]
static ALLOCATOR: common::Watchful = common::Watchful;

#[test]
fn nested_quantifiers_do_not_explode() {
    bounded(|| {
        // The classic: every way to split the run of `a` among the rounds
        // of the two loops fails only at the end.
        let run = "a".repeat(100_000);
        expect_find("(a+)*b", &run, None);
        expect_find("(a+)+b", &run, None);
        expect_find("(a|aa)+b", &run, None);
        expect_find("(a|a)*b", &run, None);
        expect_find("(a?a)*b", &run, None);
        expect_find("((a+)+)+b", &run, None);
        expect_find("^(a+)+$", &format!("{run}!"), None);
        // ...and the same patterns when there is a match.
        let with_b = format!("{run}b");
        expect_find("(a+)*b", &with_b, Some(0..100_001));
        expect_find("(a|aa)+b", &with_b, Some(0..100_001));
        expect_find("^(a+)+$", &run, Some(0..100_000));
    });
}

#[test]
fn optional_copies_do_not_explode() {
    bounded(|| {
        // `a?` thirty times and then `a` thirty times, against thirty `a`:
        // a matcher that tries "take it" before "skip it" for each `a?`
        // needs 2^30 attempts. Here it is three hundred times.
        let n = 300;
        let haystack = "a".repeat(n);
        expect_find(&format!("(a?){{{n}}}a{{{n}}}"), &haystack, Some(0..n));
        expect_find(&format!("^(a?){{{n}}}a{{{n}}}$"), &haystack, Some(0..n));
        expect_find(&format!("(a?){{{n}}}a{{{n}}}b"), &haystack, None);
        // The same spelled out, without counts.
        let spelled = format!("{}{}", "a?".repeat(n), "a".repeat(n));
        expect_find(&spelled, &haystack, Some(0..n));
        expect_find(&format!("a{{0,{n}}}a{{{n}}}"), &haystack, Some(0..n));
    });
}

#[test]
fn many_wildcards_do_not_explode() {
    bounded(|| {
        // Each `.*` can stop anywhere: a matcher that tries the stops one
        // by one needs (length)^8 steps to find out that there is no `!`.
        let haystack = "x".repeat(5_000);
        expect_find(".*.*.*.*.*.*.*.*!", &haystack, None);
        expect_find("(.*)(.*)(.*)(.*)(.*)(.*)(.*)(.*)!", &haystack, None);
        expect_find("(x+x+)+y", &haystack, None);
        expect_find("(.*,){12}z", &",".repeat(2_000), None);
        expect_find(r"^(\w+\s?)*$", &format!("{}!", "word ".repeat(2_000)), None);
        expect_find(r"\b(\w|\w\w)+\b!", &"w".repeat(5_000), None);
        expect_find(".*.*.*.*.*.*.*.*!", &format!("{haystack}!"), Some(0..5_001));
    });
}

#[test]
fn a_failing_search_does_not_start_again_at_every_position() {
    bounded(|| {
        // No `b` anywhere. An attempt that starts at any position runs to
        // the end of the haystack before it fails, so "try position 0,
        // then position 1, ..." costs (length)^2 / 2 steps: 10^11 here.
        let run = "a".repeat(500_000);
        expect_find("a*b", &run, None);
        expect_find(r"a[a-z]*\d", &run, None);
        // `find` must not pay that price either when the only match is at
        // the very end.
        expect_find("a{3}b", &format!("{run}b"), Some(499_997..500_001));
        expect_find("a$", &run, Some(499_999..500_000));
    });
}

#[test]
fn long_haystacks_with_long_matches() {
    bounded(|| {
        let line = "the quick brown fox jumps over the lazy dog, ".repeat(10_000);
        let length = line.len();
        expect_find(".*", &line, Some(0..length));
        expect_find("^.*$", &line, Some(0..length));
        expect_find(r"(\w+[ ,]+)+", &line, Some(0..length));
        expect_find(r"(\w+[ ,]+)+!", &line, None);
        expect_find(r"[a-z ,]*\d", &line, None);
        let wide = "日本語のテキスト。".repeat(20_000);
        expect_find("[^a-z]*", &wide, Some(0..wide.len()));
        expect_find("(.)*x", &wide, None);
    });
}

#[test]
fn large_patterns_within_the_limit_work() {
    bounded(|| {
        // 3000 words as one alternation: cost 3000 * 6 + 2999, well below
        // the limit. A tree or a walk that goes one level deeper per branch
        // is 3000 levels deep here, and 100 000 levels deep in two later
        // tests of this file.
        let words: Vec<String> = (0..3_000).map(|n| format!("w{n:04}x")).collect();
        let any_word = regex(&words.join("|"));
        assert_eq!(any_word.find("...w2999x..."), Some(3..9));
        assert_eq!(any_word.find("...w0000x..."), Some(3..9));
        assert_eq!(any_word.find("w123x w3000x w12345"), None);
        assert!(any_word.is_match(&format!("{}w1500x", "w15 ".repeat(50))));

        // One long literal, and a long chain of small groups. In each
        // search below, (cost of the pattern) x (length of the haystack) is
        // at most 10^8: SPEC 4.5 allows an engine to do some work for every
        // state at every position, and such an engine must pass too.
        let literal: String = (0..2_500).map(|n| format!("{n:04}")).collect();
        let long_literal = regex(&literal);
        assert_eq!(long_literal.find(&format!("00{literal}")), Some(2..10_002));
        assert_eq!(long_literal.find(&literal[1..]), None);
        // Four times as long, on a short haystack.
        let longer: String = (0..10_000).map(|n| format!("{n:04}")).collect();
        let longer_literal = regex(&longer);
        assert_eq!(longer_literal.find(&longer[..2_000]), None);
        let groups = regex(&format!("^{}$", "(a|b)".repeat(5_000)));
        assert_eq!(groups.find(&"ab".repeat(2_500)), Some(0..5_000));
        assert_eq!(groups.find(&"ab".repeat(2_499)), None);

        // A class with thousands of members, and a long run of quantified
        // atoms.
        let class: String = (0x4E00..0x9000_u32).filter_map(char::from_u32).collect();
        let kanji = regex(&format!("[{class}]+"));
        assert_eq!(kanji.find("abc 日本語 def"), Some(4..13));
        // ...and one with as many members that do not touch each other.
        let scattered: String = (0..20_000_u32)
            .filter_map(|n| char::from_u32(0x4E00 + 2 * n))
            .collect();
        let every_other = regex(&format!("[{scattered}]+"));
        assert_eq!(
            every_other.find("\u{4E01}\u{4E00}\u{4E02}\u{4E03}"),
            Some(3..9)
        );
        let optional = regex(&"a?".repeat(20_000));
        assert_eq!(optional.find(&"a".repeat(100)), Some(0..100));

        // The largest count, alone and nested within the cost limit.
        let thousand = "a".repeat(1_000);
        expect_find("a{1000}", &thousand, Some(0..1_000));
        expect_find("a{1000}", &thousand[1..], None);
        expect_find("^(a{10}){100}$", &thousand, Some(0..1_000));
        expect_find("^(a{0,10}){0,100}$", &thousand, Some(0..1_000));
        expect_find("^(a{1,}){1000}$", &thousand, Some(0..1_000));
    });
}

/// `inner`, wrapped in `levels` groups that are each repeated 1000 times.
fn thousandfold(inner: &str, levels: usize) -> String {
    let mut pattern = inner.to_string();
    for _ in 0..levels {
        pattern = format!("({pattern}){{1000}}");
    }
    pattern
}

#[test]
fn the_cost_limit_is_exact() {
    bounded(|| {
        // SPEC 6.1 defines the cost. Each pair below is a pattern that
        // costs exactly 100 000, which is allowed, and one that costs a
        // little more.
        #[track_caller]
        fn at_and_over(at_limit: &str, over: &str) {
            regex(at_limit);
            assert_eq!(
                rejection(over),
                ("pattern too large".to_string(), 0),
                "a pattern with a cost above 100 000"
            );
        }
        // Characters cost 1 each.
        let literal = "a".repeat(100_000);
        at_and_over(&literal, &format!("{literal}b"));
        at_and_over(
            &format!("{}{}", "é".repeat(50_000), ".".repeat(50_000)),
            &format!("{literal}."),
        );
        at_and_over(
            &format!("^{}$", "[a-z]".repeat(99_998)),
            &format!("^{}$", r"\d".repeat(99_999)),
        );
        // `e{n}` costs n times e.
        let hundred = "a{1000}".repeat(100);
        at_and_over(&hundred, &format!("{hundred}a"));
        at_and_over("(a{100}){1000}", "(a{100}b){1000}");
        at_and_over("((a{10}){10}){1000}", "(b(a{10}){10}){1000}");
        // `e?` and `e*` and `e+` cost one more than e.
        at_and_over(&"a?".repeat(50_000), &format!("{}a", "a?".repeat(50_000)));
        at_and_over("(a{98}b?){1000}", "(a{99}b?){1000}");
        at_and_over("(a{98}b*){1000}", "(a{98}b*c){1000}");
        at_and_over("((ab)+c{97}){1000}", "((ab)+c{98}){1000}");
        // `e{n,}` costs n times e, plus 1; `e{0,}` is `e*`.
        at_and_over("(a{99,}){1000}", "(a{100,}){1000}");
        at_and_over(
            "(a{98}b{0,}){500}(a{98}b{1,}){500}",
            "(a{98}b{0,}){500}(a{98}b{2,}){500}",
        );
        // `e{n,m}` costs m times e, plus m - n.
        at_and_over("(a{0,50}){1000}", "(a{0,50}b){1000}");
        at_and_over("(a{50,75}){1000}", "(a{49,75}){1000}");
        at_and_over("((ab){20,40}){1000}", "((ab){19,40}){1000}");
        // An alternation costs its branches, plus 1 for each `|`.
        let fifty_thousand: Vec<&str> = vec!["a"; 50_000];
        at_and_over(
            &format!("{}b", fifty_thousand.join("|")),
            &format!("{}bc", fifty_thousand.join("|")),
        );
        at_and_over(&"|".repeat(100_000), &"|".repeat(100_001));
        at_and_over("((a|b|c){20}){1000}", "((a|b|c|){20}){1000}");
        // Groups cost nothing; assertions cost 1.
        at_and_over(
            &format!("{}a{{1000}}", "(a{1000})".repeat(99)),
            &format!("(){}", "(a{1000}b{0}|)".repeat(100)),
        );
        at_and_over(r"(^\b$\Ba{96}){1000}", r"(^\b$\B\ba{96}){1000}");
    });
}

#[test]
fn a_pattern_over_the_cost_limit_is_refused_before_anything_is_built() {
    bounded(|| {
        let too_large = ("pattern too large".to_string(), 0);
        // 10^6: small enough to build by mistake. It must still be refused.
        assert_eq!(rejection(&thousandfold("a", 2)), too_large);
        // 10^9 and 10^12: building these needs more memory than the test
        // program may use.
        assert_eq!(rejection(&thousandfold("a", 3)), too_large);
        assert_eq!(rejection(&thousandfold("a", 4)), too_large);
        // 10^18 fits in a u64; 10^21 and 10^30 do not. A cost that is
        // computed with arithmetic that wraps around can come out small.
        assert_eq!(rejection(&thousandfold("a", 6)), too_large);
        assert_eq!(rejection(&thousandfold("a", 7)), too_large);
        assert_eq!(rejection(&thousandfold("a", 10)), too_large);
        assert_eq!(rejection(&thousandfold("(a|b)*c{2,1000}", 40)), too_large);
        // 2^64 exactly, and 2^64 + 1000: wrapped to 64 bits they are 0 and
        // 1000.
        let two_to_the_64 = format!("{}a{}", "(".repeat(8), "){256}".repeat(8));
        assert_eq!(rejection(&two_to_the_64), too_large);
        assert_eq!(rejection(&format!("{two_to_the_64}b{{1000}}")), too_large);
        // Open-ended and optional forms count too.
        assert_eq!(rejection(&thousandfold("a*", 2)), too_large);
        assert_eq!(rejection("((a{1000,}){1000,}){1000,}"), too_large);
        assert_eq!(rejection("((a{0,1000}){0,1000}){0,1000}"), too_large);
        // An error in the syntax is reported even if the pattern is also
        // too large (SPEC 5.1).
        assert_eq!(
            rejection(&format!("{}(", thousandfold("a", 5))).0,
            "missing closing parenthesis"
        );
    });
}

#[test]
fn what_costs_nothing_takes_no_time() {
    bounded(|| {
        // Zero times anything is zero: these patterns are within the limit,
        // and all of them match exactly the empty string. Compiling them
        // must not do 1000^20 steps of anything.
        for inner in ["", "()", "a{0}", "()()", "(a{1000}){0}"] {
            let pattern = thousandfold(inner, 20);
            expect_find(&pattern, "aaa", Some(0..0));
            expect_find(&format!("x{pattern}y"), "axy", Some(1..3));
        }
        // Cost 1001: one choice for each of the 1000 outer copies.
        expect_find("((){999,1000}){1000}b", "ab", Some(1..2));
        // Cost exactly 100 000, almost all of it choices between nothing
        // and nothing.
        expect_find("(((){999,1000}){1000}){100}", "ab", Some(0..0));
    });
}

#[test]
fn compiling_at_the_limit_and_matching_stay_fast() {
    bounded(|| {
        // Patterns of the largest cost, on haystacks that are short enough
        // for (cost of the pattern) x (length of the haystack) to stay at
        // 10^8 or below.
        let at_limit = regex(&"a{1000}".repeat(100));
        assert_eq!(at_limit.find(&"a".repeat(1_000)), None);
        assert!(!at_limit.is_match(&"ab".repeat(500)));
        let optional = regex(&"a?".repeat(50_000));
        assert_eq!(optional.find("aaab"), Some(0..3));
        let empties = regex(&"|".repeat(100_000));
        assert_eq!(empties.find("abc"), Some(0..0));
        // Compiling is not allowed to be slow either: many medium patterns.
        for n in 1..=200 {
            let pattern = format!("(x{n}|y[a-z]{{2,40}}|z{{{n},}})+\\b.{{0,300}}$");
            assert!(Regex::new(&pattern).is_ok(), "{pattern:?} is valid");
        }
    });
}
