//! Milestone 1: the pattern language and its errors (SPEC sections 3, 5
//! and the syntax limits of section 6).
//!
//! These tests only compile patterns. Nothing is matched yet.

mod common;

use common::{Generator, bounded, regex, rejection};
use rgx::Regex;

/// Checks a table of (pattern, message, offset).
#[track_caller]
fn expect_errors(cases: &'static [(&'static str, &'static str, usize)]) {
    bounded(move || {
        for &(pattern, message, offset) in cases {
            let (actual_message, actual_offset) = rejection(pattern);
            assert_eq!(
                (actual_message.as_str(), actual_offset),
                (message, offset),
                "the error for the pattern {pattern:?}: (message, byte offset) differ from SPEC section 5"
            );
        }
    });
}

#[test]
fn valid_patterns_compile() {
    bounded(|| {
        let patterns = [
            // SPEC 3.2: literals, and the two closing characters that are
            // ordinary outside their construct.
            "",
            "abc",
            "héllo, 世界 😀",
            "a]b}c",
            "-,:;<=>!\"#%&'/@_`~",
            // SPEC 3.3: escapes.
            r"\\\.\^\$\|\(\)\[\]\{\}\*\+\?",
            r"\-\/\!\@\_\~\,",
            r"\n\t\r",
            r"\u{41}\u{e9}\u{1F600}\u{10FFFF}\u{0}",
            r"\d\D\w\W\s\S",
            // SPEC 3.4: the dot. SPEC 3.5: classes.
            ".",
            "[abc]",
            "[^abc]",
            "[a-z0-9_]",
            "[α-ω]",
            "[-a]",
            "[a-]",
            "[^-a]",
            "[a-c-e]",
            "[+--]",
            r"[\d-z]",
            r"[\]\[\\\-\^]",
            "[a^]",
            "[^^]",
            r"[\d\s\w]",
            r"[\D\S\W]",
            r"[^\D]",
            r"[\n\t\u{e9}-\u{ff}]",
            "[.*+?(){}|$]",
            // SPEC 3.6: groups and alternation, with empty parts.
            "(a)",
            "()",
            "a|b|c",
            "a|",
            "|a",
            "|",
            "(|)",
            "(a|b)(c|d)",
            "((a))",
            // SPEC 3.7: quantifiers.
            "a*",
            "a+",
            "a?",
            "a{0}",
            "a{3}",
            "a{3,}",
            "a{0,}",
            "a{3,5}",
            "a{5,5}",
            "a{0,0}",
            "a{1000}",
            "a{0,1000}",
            "(ab)*",
            "[ab]+",
            ".?",
            r"\d{2,4}",
            "(a|b)*",
            "(a+)+",
            "(a*b)+",
            "a*b+c?",
            // SPEC 3.7: a count and `?` are not loops; what they repeat may
            // be empty.
            "()?",
            "(){3}",
            "(a*)?",
            "(a*){3}",
            "(a|){0,5}",
            "(a?b?){2,7}",
            // SPEC 3.8: assertions, also under `?` or a count.
            "^a$",
            r"\bword\b",
            r"\B",
            "^?",
            "${2}",
            r"\b?",
            "(^a)*",
            r"(\ba)+",
            "(^|x)a",
            "a$|b",
            "^^$$",
        ];
        for pattern in patterns {
            regex(pattern);
        }
    });
}

#[test]
fn group_errors() {
    expect_errors(&[
        ("(", "missing closing parenthesis", 0),
        ("a(b", "missing closing parenthesis", 1),
        ("a(b(c)", "missing closing parenthesis", 1),
        ("((a)", "missing closing parenthesis", 0),
        // Several groups are open at the end: the last one opened.
        ("((a", "missing closing parenthesis", 1),
        ("(a(b)(c|(d", "missing closing parenthesis", 8),
        ("é(a|b", "missing closing parenthesis", 2),
        (")", "unmatched closing parenthesis", 0),
        ("ab)", "unmatched closing parenthesis", 2),
        ("(a))", "unmatched closing parenthesis", 3),
        ("a|)", "unmatched closing parenthesis", 2),
        ("(?:a)", "unsupported group syntax", 0),
        ("a(?i)b", "unsupported group syntax", 1),
        ("(?<name>a)", "unsupported group syntax", 0),
        ("(?", "unsupported group syntax", 0),
    ]);
}

#[test]
fn class_errors() {
    expect_errors(&[
        ("[", "missing closing bracket", 0),
        ("[a", "missing closing bracket", 0),
        ("x[^ab", "missing closing bracket", 1),
        (r"[a\]", "missing closing bracket", 0),
        ("[a-", "missing closing bracket", 0),
        ("日[本", "missing closing bracket", 3),
        ("[]", "empty character class", 0),
        ("[^]", "empty character class", 0),
        ("ab[]c", "empty character class", 2),
        // SPEC 3.5: `]` ends the class wherever it stands.
        ("[]a]", "empty character class", 0),
        ("[z-a]", "invalid class range", 1),
        ("[ab-a]", "invalid class range", 2),
        (r"[a-\d]", "invalid class range", 1),
        (r"[\u{ff}-\u{fe}]", "invalid class range", 1),
        ("[ω-α]", "invalid class range", 1),
        // `[+--]` is the range from `+` to `-`; this is the range from `a`
        // to `-`, which runs backwards.
        ("[a--]", "invalid class range", 1),
        // SPEC 5.2: if the second end of a range cannot be read at all, the
        // error is the one for that character.
        (r"[a-\q]", "unknown escape", 3),
        ("[a-[]", "unsupported class syntax", 3),
        (r"[a-\u{110000}]", "invalid Unicode escape", 3),
        ("[a-\\", "incomplete escape", 3),
        ("[[]", "unsupported class syntax", 1),
        ("[a[b]]", "unsupported class syntax", 2),
        ("[[:alpha:]]", "unsupported class syntax", 1),
        ("[^[:digit:]]", "unsupported class syntax", 2),
    ]);
}

#[test]
fn repetition_errors() {
    expect_errors(&[
        ("*", "nothing to repeat", 0),
        ("+a", "nothing to repeat", 0),
        ("?", "nothing to repeat", 0),
        ("{2}", "nothing to repeat", 0),
        ("a|*", "nothing to repeat", 2),
        ("(+a)", "nothing to repeat", 1),
        ("a(|?)", "nothing to repeat", 3),
        // A quantifier cannot follow a quantifier.
        ("a**", "nothing to repeat", 2),
        ("a+?", "nothing to repeat", 2),
        ("a*+", "nothing to repeat", 2),
        ("a?{2}", "nothing to repeat", 2),
        ("a{2}{3}", "nothing to repeat", 4),
        ("a{2,3}*", "nothing to repeat", 6),
        ("é{2}?", "nothing to repeat", 5),
        // A `{` after an atom must begin a well-formed count.
        ("a{", "invalid repetition", 1),
        ("a{}", "invalid repetition", 1),
        ("a{2", "invalid repetition", 1),
        ("a{2,", "invalid repetition", 1),
        ("a{2,3", "invalid repetition", 1),
        ("a{,3}", "invalid repetition", 1),
        ("a{x}", "invalid repetition", 1),
        ("a{2,x}", "invalid repetition", 1),
        ("a{ 2}", "invalid repetition", 1),
        ("a{2 }", "invalid repetition", 1),
        ("a{-1}", "invalid repetition", 1),
        ("ab{1,2,3}", "invalid repetition", 2),
        ("(a){3,2}", "invalid repetition", 3),
        ("a{1,0}", "invalid repetition", 1),
    ]);
}

#[test]
fn a_loop_needs_something_to_consume() {
    // SPEC 3.7: `*`, `+` and `{n,}` after an expression that can match the
    // empty string. The offset is that of the quantifier.
    expect_errors(&[
        ("()*", "repeated expression can be empty", 2),
        ("()+", "repeated expression can be empty", 2),
        ("(){2,}", "repeated expression can be empty", 2),
        ("(a*)*", "repeated expression can be empty", 4),
        ("(a*)+", "repeated expression can be empty", 4),
        ("(a?)*", "repeated expression can be empty", 4),
        ("(a|)*", "repeated expression can be empty", 4),
        ("(|a)+", "repeated expression can be empty", 4),
        ("(a|)+b", "repeated expression can be empty", 4),
        ("(a?){2,}", "repeated expression can be empty", 4),
        ("(a*b*)*", "repeated expression can be empty", 6),
        ("(a?a?)*b", "repeated expression can be empty", 6),
        ("((a*)*)*", "repeated expression can be empty", 5),
        ("((a|b)*)*", "repeated expression can be empty", 8),
        ("x(y|(z?))+", "repeated expression can be empty", 9),
        // Zero copies are nothing.
        ("(a{0})*", "repeated expression can be empty", 6),
        ("(a{0,5})+", "repeated expression can be empty", 8),
        ("(a{0}|c)*", "repeated expression can be empty", 8),
        // An assertion matches the empty string.
        ("^*", "repeated expression can be empty", 1),
        ("$+", "repeated expression can be empty", 1),
        (r"\b*", "repeated expression can be empty", 2),
        (r"(\b)+a", "repeated expression can be empty", 4),
        ("(^|a)+", "repeated expression can be empty", 5),
        (r"(^$\B){1,}", "repeated expression can be empty", 6),
        ("é(ñ?)*", "repeated expression can be empty", 7),
        // In reading order: the first loop is refused before anything
        // after it is read.
        ("()**", "repeated expression can be empty", 2),
        ("(a*)*)", "repeated expression can be empty", 4),
        (r"(a?)+\q", "repeated expression can be empty", 4),
        // SPEC 5.2: a count is judged by form, size and order first.
        ("(a*){2,x}", "invalid repetition", 4),
        ("(a*){2000,}", "repetition count too large", 4),
        ("(a*){1000,}", "repeated expression can be empty", 4),
        ("(a*){0,}", "repeated expression can be empty", 4),
    ]);
    bounded(|| {
        // What must consume may be repeated, however it is built.
        for pattern in [
            "(a*b)*",
            "(a|b)+",
            "(a?b|c)*",
            "(^a)+",
            r"(a\b)*",
            "(a{1,5}){2,}",
            "(ab{0}|c)*",
            "((a|b)+)*",
            "((a*){3}b){1,}",
            "(a*){0,1000}",
        ] {
            regex(pattern);
        }
    });
}

#[test]
fn escape_errors() {
    expect_errors(&[
        (r"\q", "unknown escape", 0),
        (r"a\A", "unknown escape", 1),
        (r"\1", "unknown escape", 0),
        (r"(a)\1", "unknown escape", 3),
        (r"\x41", "unknown escape", 0),
        (r"\é", "unknown escape", 0),
        (r"\ ", "unknown escape", 0),
        (r"ab\z", "unknown escape", 2),
        (r"[\b]", "unknown escape", 1),
        (r"[a\B]", "unknown escape", 2),
        (r"[\q]", "unknown escape", 1),
        ("\\", "incomplete escape", 0),
        ("abc\\", "incomplete escape", 3),
        ("(a)\\", "incomplete escape", 3),
        (r"\u", "invalid Unicode escape", 0),
        (r"\u41", "invalid Unicode escape", 0),
        (r"\u{}", "invalid Unicode escape", 0),
        (r"\u{41", "invalid Unicode escape", 0),
        (r"\u{xyz}", "invalid Unicode escape", 0),
        (r"\u{1234567}", "invalid Unicode escape", 0),
        (r"\u{110000}", "invalid Unicode escape", 0),
        (r"\u{D800}", "invalid Unicode escape", 0),
        (r"\u{DFFF}", "invalid Unicode escape", 0),
        (r"ab\u{FFFFFF}", "invalid Unicode escape", 2),
        (r"[a\u{}]", "invalid Unicode escape", 2),
    ]);
}

#[test]
fn the_first_error_in_reading_order_is_reported() {
    // SPEC 5.1 gives these as examples of its rule.
    expect_errors(&[
        ("a)(?", "unmatched closing parenthesis", 1),
        ("(a[b", "missing closing bracket", 2),
        (r"*\q", "nothing to repeat", 0),
        (r"\q*(", "unknown escape", 0),
        ("[z-a](", "invalid class range", 1),
        // Several constructs are open at the end: the one opened last.
        ("((a", "missing closing parenthesis", 1),
        ("(a{2", "invalid repetition", 2),
        (r"[a\", "incomplete escape", 2),
        (r"(\u{41", "invalid Unicode escape", 1),
        ("([a", "missing closing bracket", 1),
    ]);
}

#[test]
fn repetition_counts_have_a_limit() {
    expect_errors(&[
        ("a{1001}", "repetition count too large", 1),
        ("a{1001,}", "repetition count too large", 1),
        ("a{0,1001}", "repetition count too large", 1),
        ("ab{5,99999}", "repetition count too large", 2),
        // More digits than any integer type holds.
        (
            "a{99999999999999999999999999999999999999999}",
            "repetition count too large",
            1,
        ),
        (
            "a{1,340282366920938463463374607431768211456}",
            "repetition count too large",
            1,
        ),
        // SPEC 5.2: the size of the counts is checked before their order...
        ("a{2000,1}", "repetition count too large", 1),
        // ...and the form of the whole `{...}` before the size.
        ("a{2000", "invalid repetition", 1),
        ("a{2000,x}", "invalid repetition", 1),
    ]);
    bounded(|| {
        regex("a{1000}");
        regex("a{1000,}");
        regex("a{999,1000}");
        regex("a{0001000}");
    });
}

#[test]
fn group_nesting_has_a_limit() {
    bounded(|| {
        let nested = |depth: usize| format!("{}a{}", "(".repeat(depth), ")".repeat(depth));
        regex(&nested(200));
        // The 201st `(` is at byte offset 200.
        assert_eq!(
            rejection(&nested(201)),
            ("nesting too deep".to_string(), 200),
            "201 nested groups"
        );
        // The limit counts groups that are open at the same time, not
        // groups in total.
        regex(&"(a)".repeat(1000));
        regex(&format!("{}{}", nested(200), nested(200)));
        let message = "é".to_string() + &nested(250);
        assert_eq!(
            rejection(&message),
            ("nesting too deep".to_string(), 202),
            "250 nested groups after `é`"
        );
        // SPEC 5.1: the 201st `(` is noticed before the `?` behind it. A
        // `(?` that still has room is reported for what it is.
        assert_eq!(
            rejection(&format!("{}(?:a)", "(".repeat(200))),
            ("nesting too deep".to_string(), 200),
            "`(?` as the 201st group"
        );
        assert_eq!(
            rejection(&format!("{}(?:a)", "(".repeat(199))),
            ("unsupported group syntax".to_string(), 199),
            "`(?` as the 200th group"
        );
    });
}

#[test]
fn deep_nesting_is_an_error_and_not_a_crash() {
    bounded(|| {
        // If this test ends the whole test program with a stack overflow,
        // your parser recursed once per `(`. The limit of SPEC 6 exists so
        // that it can stop long before that.
        let depth = if common::MIRI { 2_000 } else { 100_000 };
        let open_only = "(".repeat(depth);
        assert_eq!(rejection(&open_only), ("nesting too deep".to_string(), 200));
        let balanced = format!("{}a{}", "(".repeat(depth), ")".repeat(depth));
        assert_eq!(rejection(&balanced), ("nesting too deep".to_string(), 200));
    });
}

#[test]
fn the_error_type_is_a_good_citizen() {
    bounded(|| {
        let error = Regex::new("a(b").expect_err("`a(b` is invalid");
        assert_eq!(error.offset(), 1);
        assert_eq!(
            error.to_string(),
            "missing closing parenthesis",
            "Display is the message alone (SPEC 2.2)"
        );
        assert!(!format!("{error:?}").is_empty(), "Error implements Debug");

        // It can travel in a `Box<dyn Error + Send + Sync>` and with `?`.
        fn compile(
            pattern: &str,
        ) -> Result<Regex, Box<dyn std::error::Error + Send + Sync + 'static>> {
            Ok(Regex::new(pattern)?)
        }
        let boxed = compile("[z-a]").expect_err("`[z-a]` is invalid");
        assert_eq!(boxed.to_string(), "invalid class range");
        assert!(compile("[a-z]").is_ok());
    });
}

#[test]
fn offsets_are_bytes_on_character_boundaries() {
    bounded(|| {
        // Every pattern here is invalid; the offset must be usable to slice
        // the pattern, whatever comes before the error.
        for pattern in [
            "日本語(",
            "ééé[a",
            "😀😀*+",
            r"ωω\q",
            "ñ{2}{3}",
            r"🦀\u{}",
            "αβγ)",
            "ü[z-a]",
        ] {
            let (message, offset) = rejection(pattern);
            assert!(
                pattern.is_char_boundary(offset),
                "pattern {pattern:?}: offset {offset} of \"{message}\" is not on a character boundary"
            );
        }
        assert_eq!(
            rejection("日本語("),
            ("missing closing parenthesis".to_string(), 9)
        );
        assert_eq!(rejection("😀😀*+"), ("nothing to repeat".to_string(), 9));
        assert_eq!(
            rejection("αβγ)"),
            ("unmatched closing parenthesis".to_string(), 6)
        );
    });
}

#[test]
fn no_pattern_makes_regex_new_panic() {
    bounded(|| {
        // SPEC 2.1: `Regex::new` never panics, whatever the pattern is.
        // SPEC 2.2 and 5: an error has one of sixteen messages, and its
        // offset is the offset of a character of the pattern (or its end).
        const MESSAGES: [&str; 16] = [
            "missing closing parenthesis",
            "unmatched closing parenthesis",
            "nesting too deep",
            "unsupported group syntax",
            "missing closing bracket",
            "empty character class",
            "invalid class range",
            "unsupported class syntax",
            "nothing to repeat",
            "invalid repetition",
            "repetition count too large",
            "repeated expression can be empty",
            "unknown escape",
            "incomplete escape",
            "invalid Unicode escape",
            "pattern too large",
        ];
        // Mostly metacharacters, and a few starts of longer constructs.
        const PIECES: [&str; 36] = [
            "(", ")", "[", "]", "{", "}", "|", "*", "+", "?", "\\", "^", "$", ".", "-", ",", "a",
            "b", "0", "1", "9", "u", "d", "w", "é", "日", "😀", " ", "(?", "{2", "{1,", "\\u{",
            "[^", "1001", "(a", "a{1000}",
        ];
        let mut generator = Generator::new(0x5EED_0001);
        let rounds = if common::MIRI { 300 } else { 30_000 };
        for _ in 0..rounds {
            let length = generator.below(10);
            let pattern = generator.text(&PIECES, length);
            if let Err(error) = Regex::new(&pattern) {
                let message = error.to_string();
                assert!(
                    MESSAGES.contains(&message.as_str()),
                    "pattern {pattern:?}: {message:?} is not one of the messages of SPEC 5.2"
                );
                assert!(
                    pattern.is_char_boundary(error.offset()),
                    "pattern {pattern:?}: offset {} of \"{message}\" is not the offset of a \
                     character of the pattern",
                    error.offset()
                );
            }
        }
    });
}
