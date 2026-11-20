//! Milestone 2: what a pattern matches (SPEC sections 3 and 4).
//!
//! Every case gives a pattern, a haystack and the byte range that `find`
//! must return (`None`: no match). `is_match` is checked against the same
//! expectation.

mod common;

use std::ops::Range;

use common::{Generator, bounded, expect_find, regex};

type Case = (&'static str, &'static str, Option<Range<usize>>);

#[track_caller]
fn expect_cases(cases: Vec<Case>) {
    bounded(move || {
        for (pattern, haystack, expected) in cases {
            expect_find(pattern, haystack, expected);
        }
    });
}

#[test]
fn literals() {
    expect_cases(vec![
        ("abc", "abc", Some(0..3)),
        ("abc", "xxabcxx", Some(2..5)),
        ("abc", "ab", None),
        ("abc", "", None),
        ("abc", "ABC", None),
        ("b", "abcabc", Some(1..2)),
        ("aab", "aaab", Some(1..4)),
        ("abab", "abaabab", Some(3..7)),
        // `]` and `}` are ordinary characters outside a class or a count.
        ("a]b}", "xa]b}", Some(1..5)),
        // A pattern is not a line: spaces and punctuation are literals.
        ("a b", "a b", Some(0..3)),
        ("a-b", "a-b", Some(0..3)),
        ("a,b:c", "a,b:c", Some(0..5)),
    ]);
}

#[test]
fn the_empty_pattern_matches_the_empty_string_at_the_start() {
    expect_cases(vec![
        ("", "", Some(0..0)),
        ("", "abc", Some(0..0)),
        ("()", "abc", Some(0..0)),
        ("|", "abc", Some(0..0)),
        ("(|)", "", Some(0..0)),
    ]);
}

#[test]
fn escapes() {
    expect_cases(vec![
        (r"a\.b", "axb a.b", Some(4..7)),
        (r"\\", r"a\b", Some(1..2)),
        (r"\(\)", "f()", Some(1..3)),
        (r"\[a\]", "[a]", Some(0..3)),
        (r"\{2\}", "a{2}", Some(1..4)),
        (r"a\*", "aaa*", Some(2..4)),
        (r"\^\$", "^$", Some(0..2)),
        (r"a\|b", "a|b", Some(0..3)),
        (r"1\+1\?", "1+1?", Some(0..4)),
        (r"\-\/", "-/", Some(0..2)),
        (r"a\nb", "a\nb", Some(0..3)),
        (r"\t", "a\tb", Some(1..2)),
        (r"\r\n", "a\r\n", Some(1..3)),
        (r"\u{41}", "xA", Some(1..2)),
        (r"\u{e9}", "café", Some(3..5)),
        (r"\u{00E9}", "café", Some(3..5)),
        (r"\u{1F600}", "a😀", Some(1..5)),
        (r"\u{0}", "a\0", Some(1..2)),
    ]);
}

#[test]
fn the_dot_matches_one_character_but_not_a_line_feed() {
    expect_cases(vec![
        (".", "a", Some(0..1)),
        (".", "", None),
        (".", "\n", None),
        (".", "\na", Some(1..2)),
        (".", "\r", Some(0..1)),
        ("a.c", "abc", Some(0..3)),
        ("a.c", "a\nc", None),
        ("a.c", "ac", None),
        ("...", "ab\ncdef", Some(3..6)),
        // One dot is one character, however many bytes it has.
        ("a.c", "aéc", Some(0..4)),
        ("a.c", "a日c", Some(0..5)),
        ("a.c", "a😀c", Some(0..6)),
        ("^.$", "😀", Some(0..4)),
        ("^..$", "😀", None),
        // A character is a scalar value, not what a reader sees as one
        // letter: `e` followed by a combining accent is two characters.
        ("^..$", "e\u{301}", Some(0..3)),
        ("^.$", "e\u{301}", None),
    ]);
}

#[test]
fn classes() {
    expect_cases(vec![
        ("[abc]", "xxbxx", Some(2..3)),
        ("[abc]", "xyz", None),
        ("[a-c]", "xxbxx", Some(2..3)),
        ("[a-c]", "d", None),
        ("[a-cx-z]", "my", Some(1..2)),
        ("[0-9][0-9]", "a1b22", Some(3..5)),
        ("x[ab]y", "xay xcy xby", Some(0..3)),
        ("x[ab]y", "xcy xby", Some(4..7)),
        // A range of one character, and ranges that overlap.
        ("[a-a]", "ba", Some(1..2)),
        ("[a-mh-z]", "#q", Some(1..2)),
        // Inside a class only `\`, `]`, `[`, a leading `^` and a `-`
        // between two elements are special.
        ("[.]", "a.", Some(1..2)),
        ("[*+?]", "a?", Some(1..2)),
        ("[(|)]", "a|", Some(1..2)),
        ("[{}]", "a}", Some(1..2)),
        ("[$]", "a$", Some(1..2)),
        ("[a^]", "^", Some(0..1)),
        (r"[\]]", "a]", Some(1..2)),
        (r"[\[]", "a[", Some(1..2)),
        (r"[\\]", r"a\", Some(1..2)),
        (r"[\^a]", "^", Some(0..1)),
        (r"[\n\t]", "a\tb", Some(1..2)),
        (r"[\u{e9}]", "é", Some(0..2)),
        // SPEC 3.5: where a `-` is a character.
        ("[-a]", "x-", Some(1..2)),
        ("[a-]", "x-", Some(1..2)),
        ("[a-]", "b", None),
        (r"[a\-z]", "m", None),
        (r"[a\-z]", "-", Some(0..1)),
        ("[a-c-e]", "d", None),
        ("[a-c-e]", "-", Some(0..1)),
        ("[a-c-e]", "e", Some(0..1)),
        // ...and that `-`, like any single character, can begin a range:
        // here `a` to `c`, then `-` to `0`, which is the four characters -./0
        ("[a-c--0]", "/", Some(0..1)),
        ("[a-c--0]", "d1", None),
        ("[+--]", ",", Some(0..1)),
        ("[+--]", ".", None),
        (r"[\d-z]", "-", Some(0..1)),
        (r"[\d-z]", "m", None),
        (r"[\d-z]", "z7", Some(0..1)),
    ]);
}

#[test]
fn negated_classes() {
    expect_cases(vec![
        ("[^abc]", "abcd", Some(3..4)),
        ("[^abc]", "cab", None),
        ("[^a-y]", "az", Some(1..2)),
        ("[^^]", "^a", Some(1..2)),
        ("[^-]", "-x", Some(1..2)),
        // Unlike the dot, a negated class matches a line feed.
        ("[^a]", "a\n", Some(1..2)),
        ("a[^b]c", "a\nc", Some(0..3)),
        // It matches any character that is not listed, of any length.
        ("[^a]", "aé", Some(1..3)),
        ("[^a]", "a😀", Some(1..5)),
        ("[^é]", "éa", Some(2..3)),
        ("[^α-ω]", "λx", Some(2..3)),
        ("x[^x]*x", "xaébx", Some(0..6)),
        // The largest and the smallest character.
        ("[^a]", "\u{10FFFF}", Some(0..4)),
        ("[^a]", "\0", Some(0..1)),
        // A class that leaves nothing over is valid and matches nothing.
        (r"[^\u{0}-\u{10FFFF}]", "abc", None),
        (r"[^\d\D]", "a1", None),
        (r"[\d\D]", "\n", Some(0..1)),
    ]);
}

#[test]
fn classes_hold_characters_not_bytes() {
    expect_cases(vec![
        ("[é]", "café", Some(3..5)),
        ("[éè]", "è", Some(0..2)),
        ("[α-ω]", "abc λ", Some(4..6)),
        ("[α-ω]+", "λόγος", Some(0..2)), // ό is outside α-ω
        ("[α-ωό]+", "λόγος", Some(0..10)),
        ("[日本]", "en 本", Some(3..6)),
        ("[😀-😏]", "a😎", Some(1..5)),
        ("[😀-😏]", "😐", None),
        ("[a-é]", "z", Some(0..1)),
        ("[a-é]", "ê", None),
        // The first byte of `é` is 0xC3. A class must not match a byte.
        ("[\u{c3}]", "é", None),
        ("[^\u{c3}]", "é", Some(0..2)),
        (r"[\u{80}-\u{10FFFF}]", "abc", None),
        (r"[\u{80}-\u{10FFFF}]+", "abcéλ日😀z", Some(3..14)),
    ]);
}

#[test]
fn predefined_classes_are_ascii_only() {
    expect_cases(vec![
        (r"\d", "a7", Some(1..2)),
        (r"\d+", "abc 2024!", Some(4..8)),
        (r"\D", "12a", Some(2..3)),
        (r"\w+", "hello_World42!", Some(0..13)),
        (r"\W", "ab_9 x", Some(4..5)),
        (r"\s", "ab cd", Some(2..3)),
        (r"\s+", "a \t\n\r\u{b}\u{c}b", Some(1..7)),
        (r"\S+", "  word  ", Some(2..6)),
        // SPEC 3.5: the sets are defined on ASCII. Other digits, letters
        // and spaces are not in them, so they are in the negated sets.
        (r"\d", "٣", None), // ARABIC-INDIC DIGIT THREE
        (r"\D", "٣", Some(0..2)),
        (r"\w", "é", None),
        (r"\W", "é", Some(0..2)),
        (r"\w+", "naïve", Some(0..2)),
        (r"\s", "\u{a0}", None), // NO-BREAK SPACE
        (r"\S", "\u{a0}", Some(0..2)),
        (r"\s", "\u{2003}", None), // EM SPACE
        // In classes.
        (r"[\d.]+", "v1.25b", Some(1..5)),
        (r"[\w-]+", "--a-b c", Some(0..5)),
        (r"[^\s]+", " ab ", Some(1..3)),
        (r"[\D]", "1a", Some(1..2)),
        (r"[\W\d]", "ab3", Some(2..3)),
        (r"[^\W]", " _", Some(1..2)),
        (r"[^\d\s]", "1 2 x", Some(4..5)),
    ]);
}

#[test]
fn alternation_prefers_the_earlier_branch() {
    expect_cases(vec![
        ("a|b", "b", Some(0..1)),
        ("a|b", "c", None),
        ("cat|dog", "hotdog", Some(3..6)),
        // SPEC 4.2: the first branch that leads to a match wins, even if a
        // later branch would match more.
        ("a|ab", "ab", Some(0..1)),
        ("ab|a", "ab", Some(0..2)),
        ("a|ab|abc", "abc", Some(0..1)),
        ("abc|ab|a", "abc", Some(0..3)),
        ("foo|foobar", "foobar", Some(0..3)),
        // ...but a branch that leads nowhere is given up.
        ("(a|ab)c", "abc", Some(0..3)),
        ("(a|ab)(c|bcd)", "abcd", Some(0..4)),
        ("(ab|a)(bc|c)", "abc", Some(0..3)),
        // An empty branch matches the empty string.
        ("a|", "a", Some(0..1)),
        ("a|", "b", Some(0..0)),
        ("|a", "a", Some(0..0)),
        ("x(a|)y", "xy xay", Some(0..2)),
        ("x(|a)y", "xay", Some(0..3)),
        // Alternation binds less tightly than concatenation.
        ("ab|cd", "ad", None),
        ("ab|cd", "acd", Some(1..3)),
        ("a(b|c)d", "acd", Some(0..3)),
    ]);
}

#[test]
fn an_earlier_start_beats_a_preferred_branch() {
    expect_cases(vec![
        // SPEC 4.3: first the leftmost start, then the preference.
        ("b|ab", "ab", Some(0..2)),
        ("b+|a", "abbb", Some(0..1)),
        ("c|bc|abc", "abc", Some(0..3)),
        ("y|xy*", "xyyy", Some(0..4)),
        ("b|^a", "ab", Some(0..1)),
        ("(b|c)+|a", "abc", Some(0..1)),
    ]);
}

#[test]
fn quantifiers_are_greedy() {
    expect_cases(vec![
        ("a*", "aaa", Some(0..3)),
        ("a*", "baaa", Some(0..0)),
        ("a*", "", Some(0..0)),
        ("a+", "baaa", Some(1..4)),
        ("a+", "bbb", None),
        ("a?", "aa", Some(0..1)),
        ("a?", "b", Some(0..0)),
        ("ab*", "abbbc", Some(0..4)),
        ("ab+c", "ac abc", Some(3..6)),
        ("colou?r", "color colour", Some(0..5)),
        ("a.*c", "abcabc", Some(0..6)),
        ("a.+c", "acac", Some(0..4)),
        ("<.*>", "<a> <b>", Some(0..7)),
        ("<[^>]*>", "<a> <b>", Some(0..3)),
        // Greedy means "prefer more", not "insist on more".
        ("a*a", "aaa", Some(0..3)),
        ("a*ab", "aaab", Some(0..4)),
        ("a?a", "a", Some(0..1)),
        ("a+a+", "aaa", Some(0..3)),
        (".*b", "abab", Some(0..4)),
        ("(a|b)*b", "abab", Some(0..4)),
        ("x*", "y", Some(0..0)),
        // A quantifier applies to one atom.
        ("ab+", "abab", Some(0..2)),
        ("(ab)+", "ababa", Some(0..4)),
        ("[ab]+", "cabbac", Some(1..5)),
        (r"\d+", "a123b", Some(1..4)),
        (".?b", "ab", Some(0..2)),
    ]);
}

#[test]
fn counted_repetition() {
    expect_cases(vec![
        ("a{3}", "aaaa", Some(0..3)),
        ("a{3}", "aa", None),
        ("a{2,}", "aaaa", Some(0..4)),
        ("a{2,}", "a", None),
        ("a{2,3}", "aaaa", Some(0..3)),
        ("a{2,3}", "a", None),
        ("a{2,3}", "baa", Some(1..3)),
        ("a{0,2}", "aaa", Some(0..2)),
        ("a{0,}", "baa", Some(0..0)),
        ("a{1,}", "baa", Some(1..3)),
        ("a{1}", "aa", Some(0..1)),
        ("a{0}", "aa", Some(0..0)),
        ("a{0,0}b", "ab", Some(1..2)),
        ("xa{0}y", "xy", Some(0..2)),
        ("(ab){2}", "ababab", Some(0..4)),
        ("(ab){2,}c", "abababc", Some(0..7)),
        ("[0-9]{4}-[0-9]{2}", "on 2024-05-17", Some(3..10)),
        (r"\w{3,5}", "ab abcdefg", Some(3..8)),
        ("(a|b){3}", "xabbay", Some(1..4)),
        ("xa{2}", "xaxaa", Some(2..5)),
        // A count gives back what the rest of the pattern needs.
        ("a{2,4}a", "aaaa", Some(0..4)),
        ("a{2,4}aaa", "aaaa", None),
        ("a{2,4}aa", "aaaa", Some(0..4)),
        ("(a{1,2}){2}", "aaaaa", Some(0..4)),
        ("a{0001}", "aa", Some(0..1)),
    ]);
}

#[test]
fn groups() {
    expect_cases(vec![
        ("(a)", "a", Some(0..1)),
        ("((a))", "a", Some(0..1)),
        ("(a)(b)(c)", "abc", Some(0..3)),
        ("a(b(c|d)e)*f", "abcebdef", Some(0..8)),
        ("(a|b)(c|d)", "xbd", Some(1..3)),
        ("((a|b)c)+", "acbcab", Some(0..4)),
        ("(a(b)?)+", "aba", Some(0..3)),
        ("x()y", "xy", Some(0..2)),
        ("x(()|())y", "xy", Some(0..2)),
    ]);
}

#[test]
fn start_and_end_anchors() {
    expect_cases(vec![
        ("^a", "ab", Some(0..1)),
        ("^a", "ba", None),
        ("a$", "ba", Some(1..2)),
        ("a$", "ab", None),
        ("^abc$", "abc", Some(0..3)),
        ("^abc$", "abcd", None),
        ("^$", "", Some(0..0)),
        ("^$", "a", None),
        ("^", "abc", Some(0..0)),
        ("$", "abc", Some(3..3)),
        ("$", "", Some(0..0)),
        ("a*$", "baa", Some(1..3)),
        ("b*$", "baa", Some(3..3)),
        // SPEC 3.8: they are the two ends of the haystack. A line feed
        // inside the haystack does not make a new start or end.
        ("^b", "a\nb", None),
        ("a$", "a\nb", None),
        ("a$", "a\n", None),
        ("^a\n", "a\n", Some(0..2)),
        // They are ordinary atoms and may stand anywhere.
        ("a^b", "ab", None),
        ("a$b", "ab", None),
        ("(^a|b)c", "bc", Some(0..2)),
        ("(^a|b)c", "xac", None),
        ("(^a|b)c", "ac", Some(0..2)),
        ("a(b$|c)", "abc ab", Some(4..6)),
        ("x*^a", "xa", None),
        ("x*^a", "ab", Some(0..1)),
        ("^^a$$", "a", Some(0..1)),
        ("(^)?a", "ba", Some(1..2)),
        ("^?a", "ba", Some(1..2)),
        ("^{2}a", "ba", None),
        ("^{2}a", "ab", Some(0..1)),
        ("^{0}a", "ba", Some(1..2)),
        ("a$?", "ab", Some(0..1)),
        ("(^a)*", "aa", Some(0..1)),
        ("(a$)+", "aa", Some(1..2)),
        ("$^", "", Some(0..0)),
        ("$^", "a", None),
        ("é$", "éé", Some(2..4)),
        ("^é", "éé", Some(0..2)),
    ]);
}

#[test]
fn word_boundaries() {
    expect_cases(vec![
        (r"\bcat\b", "cat", Some(0..3)),
        (r"\bcat\b", "a cat sat", Some(2..5)),
        (r"\bcat\b", "concat", None),
        (r"\bcat\b", "cats", None),
        (r"\bcat\b", "(cat)", Some(1..4)),
        (r"\bcat", "wildcat cat_flap", Some(8..11)),
        (r"cat\b", "cats cat", Some(5..8)),
        (r"\b", "ab", Some(0..0)),
        (r"\b", "  ab", Some(2..2)),
        (r"\b", "", None),
        (r"\b", "  ", None),
        (r"\B", "", Some(0..0)),
        (r"\B", "ab", Some(1..1)),
        (r"\B", " a", Some(0..0)),
        (r"\Bcat", "cat concat", Some(7..10)),
        (r"a\B", "a ab", Some(2..3)),
        (r"\b\w+\b", "  hello world", Some(2..7)),
        (r"\b\d+\b", "a1 22 3c", Some(3..5)),
        (r"a\b", "a_b a", Some(4..5)), // `_` is a word character
        (r"\b\b\ba", "a", Some(0..1)),
        (r"a\b\B", "a", None),
        (r"(\b){2}a", " a", Some(1..2)),
        (r"(\ba)+", "aa a", Some(0..1)),
        (r"(a\b)+", "aa a", Some(1..2)),
        // SPEC 3.8: `\b` uses the ASCII word characters of `\w`, so `é`
        // counts as a non-word character on both sides.
        (r"\bb", "éb", Some(2..3)),
        (r"a\b", "aé", Some(0..1)),
        (r"\bé", "é", None),
        (r"\Bé", "é", Some(0..2)),
        (r"é\B", "é", Some(0..2)),
        (r"\b", "éaé", Some(2..2)),
    ]);
}

#[test]
fn loops_give_back_what_the_rest_needs() {
    // SPEC 4.2: after each round a loop prefers one more round to stopping,
    // and the rounds are tried in the order of the body's own preferences.
    expect_cases(vec![
        // The loop prefers `a`; after `a` no round can follow, and stopping
        // there is preferred to everything that starts with `ab`.
        ("(a|ab)*", "abab", Some(0..1)),
        ("(ab|a)*", "abab", Some(0..4)),
        // ...unless the rest of the pattern cannot match there.
        ("(a|ab)*c", "ababc", Some(0..5)),
        ("(a|ab)*$", "abab", Some(0..4)),
        ("(a|ab)+b", "abab", Some(0..2)),
        ("(a+)+b", "aaab", Some(0..4)),
        ("(a+)*a", "aaa", Some(0..3)),
        ("(a+|b)*", "aabab!", Some(0..5)),
        ("(a*b)*", "aabbab!", Some(0..6)),
        ("(a*b)*a", "aabba", Some(0..5)),
    ]);
}

#[test]
fn counted_copies_may_be_empty() {
    // SPEC 3.7 and 4.2: a count writes out copies. A copy is not a round
    // of a loop, and it may match the empty string.
    expect_cases(vec![
        ("(|a){2}", "aa", Some(0..0)),
        ("(|a){2}b", "ab", Some(0..2)),
        ("(|a){2}$", "aa", Some(0..2)),
        ("(a|){3}", "aa", Some(0..2)),
        ("(a*){3}b", "aab", Some(0..3)),
        ("(a*){2,3}", "aaa", Some(0..3)),
        ("(a?){2}a", "aa", Some(0..2)),
        ("(a?){3}", "aa", Some(0..2)),
        ("(a?){0,3}b", "aab", Some(0..3)),
        ("(a*)?b", "aab", Some(0..3)),
        ("(){3}a", "ba", Some(1..2)),
        ("x(){0,5}y", "xy", Some(0..2)),
        // Assertions under a count.
        (r"(\b|a){2}", "aa", Some(0..0)),
        (r"(a|\b){2}", "aa", Some(0..2)),
        (r"(^|\b|a){2}b", "ab", Some(0..2)),
        ("(^|a){3}", "aa", Some(0..0)),
        ("(a|^){3}", "aa", Some(0..2)),
        ("($|a){2}", "a", Some(0..1)),
    ]);
}

#[test]
fn offsets_are_bytes_and_matches_never_split_a_character() {
    expect_cases(vec![
        ("é", "café au lait", Some(3..5)),
        ("é+", "ééé", Some(0..6)),
        ("é*", "ééa", Some(0..4)),
        ("a", "éa", Some(2..3)),
        ("日本", "こんにちは日本", Some(15..21)),
        ("😀", "a😀b", Some(1..5)),
        ("b", "a😀b", Some(5..6)),
        ("é|e", "été", Some(0..2)),
        ("t.", "été", Some(2..5)),
        ("(é|ab){2}", "abéab", Some(0..4)),
        // `é` is the bytes C3 A9, `ã` is C3 A3 and `©` is C2 A9: sharing a
        // byte is not sharing a character.
        ("é", "ã©", None),
        ("[é]", "ã©", None),
        ("é?", "ã", Some(0..0)),
        // U+00E9 and `e` + U+0301 look the same and are different texts.
        ("é", "e\u{301}", None),
        ("e\u{301}", "e\u{301}", Some(0..3)),
        ("x*", "é", Some(0..0)),
        ("$", "é", Some(2..2)),
    ]);
    bounded(|| {
        // An empty match, too, lies on a character boundary.
        for (pattern, haystack) in [
            ("x*$", "日本語"),
            ("b*", "é"),
            (r"\B", "😀😀"),
            ("(a|)$", "ñ"),
            ("[^a]*", "aλ"),
        ] {
            let range = regex(pattern).find(haystack).expect("these patterns match");
            assert!(
                haystack.is_char_boundary(range.start) && haystack.is_char_boundary(range.end),
                "pattern {pattern:?}, haystack {haystack:?}: {range:?} splits a character"
            );
        }
    });
}

#[test]
fn realistic_patterns() {
    expect_cases(vec![
        (
            r"[a-z0-9._]+@[a-z0-9.]+\.[a-z]{2,}",
            "mail ada.l@example.org now",
            Some(5..22),
        ),
        (
            r"\b[0-9]{1,3}(\.[0-9]{1,3}){3}\b",
            "host 192.168.0.12 up",
            Some(5..17),
        ),
        (r"^\s*(#.*)?$", "   # a comment", Some(0..14)),
        (r"^\s*(#.*)?$", "x = 1 # no", None),
        (
            r"(TODO|FIXME)(\([a-z]+\))?:",
            "// FIXME(ada): later",
            Some(3..14),
        ),
        (r"fn \w+\(", "pub fn find_iter(&self)", Some(4..17)),
        (r#""([^"\\]|\\.)*""#, r#"say "a \"b\" c" now"#, Some(4..15)),
        (
            r"[+-]?\d+(\.\d+)?(e[+-]?\d+)?",
            "x = -12.5e+3;",
            Some(4..12),
        ),
        (r"(\w+)\s*=\s*(\w+)", "  key =  value ", Some(2..14)),
        (
            r"https?://[^\s/]+(/\S*)?",
            "see http://example.org/a/b?c=1 ok",
            Some(4..30),
        ),
    ]);
}

#[test]
fn every_position_of_a_long_haystack_is_searched() {
    bounded(|| {
        let length = if common::MIRI { 300 } else { 50_000 };
        // The only match is at the very end.
        let mut haystack = "ab ".repeat(length);
        haystack.push_str("abc");
        let start = haystack.len() - 3;
        expect_find("abc", &haystack, Some(start..start + 3));
        expect_find(r"\bab(c|d)$", &haystack, Some(start..start + 3));
        expect_find("abd", &haystack, None);
        // The same with multi-byte characters before the match. This run is
        // short on purpose: a simple matcher that calls itself once for
        // every round of a loop must be able to pass this file. The long
        // runs are in M3.
        let wide = if common::MIRI { 100 } else { 1_000 };
        let mut haystack = "é".repeat(wide);
        haystack.push('!');
        expect_find("!", &haystack, Some(2 * wide..2 * wide + 1));
        expect_find("é+!", &haystack, Some(0..2 * wide + 1));
    });
}

#[test]
fn no_search_panics_and_every_match_is_a_slice() {
    bounded(|| {
        // SPEC 2.1: the searching methods never panic. SPEC 4.1: a match is
        // a pair of positions, so it can be used to slice the haystack.
        //
        // The patterns are generated, with a fixed seed, and valid by
        // construction: only what must consume a character gets a loop
        // (SPEC 3.7).
        const CONSUMING: [&str; 12] = [
            "a", "b", "é", "日", "😀", ".", "[ab]", "[^a]", "[é-日]", r"\w", r"\s", r"\D",
        ];
        const ANY_QUANTIFIER: [&str; 8] = ["", "", "?", "*", "+", "{2}", "{0,2}", "{1,}"];
        const ASSERTIONS: [&str; 4] = ["^", "$", r"\b", r"\B"];
        const NO_LOOP: [&str; 4] = ["", "?", "{2}", "{0,3}"];
        const HAYSTACK: [&str; 10] = ["a", "a", "b", " ", "é", "日", "😀", "\n", "1", "_"];

        let mut generator = Generator::new(0x5EED_0002);
        let rounds = if common::MIRI { 60 } else { 4_000 };
        for _ in 0..rounds {
            let mut pattern = String::new();
            for _ in 0..generator.below(5) {
                match generator.below(6) {
                    // A group whose two branches both consume.
                    0 => {
                        let first = generator.below(3) + 1;
                        let second = generator.below(2) + 1;
                        pattern.push('(');
                        pattern.push_str(&generator.text(&CONSUMING, first));
                        pattern.push('|');
                        pattern.push_str(&generator.text(&CONSUMING, second));
                        pattern.push(')');
                        pattern.push_str(generator.pick(&ANY_QUANTIFIER));
                    }
                    // What can be empty takes `?` or a count, never a loop.
                    1 => {
                        pattern.push_str(generator.pick(&ASSERTIONS));
                        pattern.push_str(generator.pick(&NO_LOOP));
                    }
                    2 => {
                        pattern.push('(');
                        pattern.push_str(generator.pick(&CONSUMING));
                        pattern.push_str("|)");
                        pattern.push_str(generator.pick(&NO_LOOP));
                    }
                    _ => {
                        pattern.push_str(generator.pick(&CONSUMING));
                        pattern.push_str(generator.pick(&ANY_QUANTIFIER));
                    }
                }
            }
            let regex = regex(&pattern);
            for _ in 0..3 {
                let length = generator.below(8);
                let haystack = generator.text(&HAYSTACK, length);
                let found = regex.find(&haystack);
                assert_eq!(
                    regex.is_match(&haystack),
                    found.is_some(),
                    "pattern {pattern:?}, haystack {haystack:?}: `is_match` must agree with `find`"
                );
                if let Some(range) = found {
                    assert!(
                        range.start <= range.end && haystack.get(range.clone()).is_some(),
                        "pattern {pattern:?}, haystack {haystack:?}: {range:?} is not a pair of \
                         positions of the haystack (SPEC 4.1)"
                    );
                }
            }
        }
    });
}
