//! Test only: SPEC section 4 written down as directly as possible, and a
//! comparison of the real matcher with it on many generated cases.
//!
//! The oracle is a backtracking matcher. It walks the syntax tree and tries
//! every way to match in exactly the order that the SPEC's lists `M(e, i)`
//! give, so its first success *is* the match the SPEC defines. It can take
//! exponential time, which is fine for haystacks of a few characters, and it
//! shares nothing with the automaton: neither the compiler nor the state
//! sets. Agreement between the two is the strongest evidence in this crate
//! that the compiler and the matcher implement the SPEC.

use crate::ast::Ast;
use crate::parser::parse;
use crate::{Regex, RegexBuilder};

struct Oracle<'a> {
    haystack: &'a str,
    case_insensitive: bool,
    /// Counts down; the search gives up at zero so that one unlucky case
    /// cannot stall the test.
    fuel: u64,
}

/// The oracle ran out of fuel.
struct TooSlow;

/// A callback for "the expression before me can end at this offset".
/// It answers whether the rest of the pattern matched from there.
type Then<'k> = &'k mut dyn FnMut(&mut Oracle<'_>, usize) -> Result<bool, TooSlow>;

impl Oracle<'_> {
    fn char_at(&self, i: usize) -> Option<char> {
        self.haystack[i..].chars().next()
    }

    fn char_before(&self, i: usize) -> Option<char> {
        self.haystack[..i].chars().next_back()
    }

    fn same(&self, a: char, b: char) -> bool {
        a == b || self.case_insensitive && a.eq_ignore_ascii_case(&b)
    }

    /// Calls `then` with every element of `M(ast, i)` in order, until one
    /// call answers `true`. Returns whether one did.
    fn ends(&mut self, ast: &Ast, i: usize, then: Then<'_>) -> Result<bool, TooSlow> {
        self.fuel = self.fuel.checked_sub(1).ok_or(TooSlow)?;
        let consume =
            |oracle: &mut Self, accept: &dyn Fn(&Self, char) -> bool, then: Then<'_>| match oracle
                .char_at(i)
            {
                Some(c) if accept(oracle, c) => then(oracle, i + c.len_utf8()),
                _ => Ok(false),
            };
        match ast {
            Ast::Empty => then(self, i),
            Ast::Literal(want) => consume(self, &|oracle, c| oracle.same(c, *want), then),
            Ast::Dot => consume(self, &|_, c| c != '\n', then),
            Ast::Class { set, negated } => consume(
                self,
                &|oracle, c| {
                    let other_case = if c.is_ascii_lowercase() {
                        c.to_ascii_uppercase()
                    } else {
                        c.to_ascii_lowercase()
                    };
                    let listed =
                        set.contains(c) || oracle.case_insensitive && set.contains(other_case);
                    listed != *negated
                },
                then,
            ),
            Ast::Look(look) => {
                if look.holds(self.char_before(i), self.char_at(i)) {
                    then(self, i)
                } else {
                    Ok(false)
                }
            }
            Ast::Concat(parts) => self.sequence(parts, i, then),
            Ast::Alternate(branches) => {
                for branch in branches {
                    if self.ends(branch, i, then)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Ast::Repeat { sub, min, max } => self.repeat(sub, *min, *max, i, then),
        }
    }

    fn sequence(&mut self, parts: &[Ast], i: usize, then: Then<'_>) -> Result<bool, TooSlow> {
        match parts.split_first() {
            None => then(self, i),
            Some((first, rest)) => {
                self.ends(first, i, &mut |oracle, j| oracle.sequence(rest, j, then))
            }
        }
    }

    fn repeat(
        &mut self,
        sub: &Ast,
        min: u32,
        max: Option<u32>,
        i: usize,
        then: Then<'_>,
    ) -> Result<bool, TooSlow> {
        match (min, max) {
            // `e{0,0}` is the empty string.
            (0, Some(0)) => then(self, i),
            // `e{0,m}` is `(e e{0,m-1})?`.
            (0, Some(max)) => Ok(self.ends(sub, i, &mut |oracle, j| {
                oracle.repeat(sub, 0, Some(max - 1), j, then)
            })? || then(self, i)?),
            // `e*` is `(e+)?`.
            (0, None) => Ok(self.plus(sub, i, then)? || then(self, i)?),
            (1, None) => self.plus(sub, i, then),
            // A required copy, then the rest.
            (min, max) => {
                let max = max.map(|max| max - 1);
                self.ends(sub, i, &mut |oracle, j| {
                    oracle.repeat(sub, min - 1, max, j, then)
                })
            }
        }
    }

    /// `e+`: after a round that ended at `j`, first try another round from
    /// there, then stop at `j`. The parser refuses a loop whose body can be
    /// empty, so `j > i` and the recursion ends.
    fn plus(&mut self, sub: &Ast, i: usize, then: Then<'_>) -> Result<bool, TooSlow> {
        self.ends(sub, i, &mut |oracle, j| {
            assert!(j > i, "a round of a loop consumed nothing");
            Ok(oracle.plus(sub, j, then)? || then(oracle, j)?)
        })
    }
}

/// The match of SPEC 4.3 among those that start at `from` or later.
fn oracle_find(
    ast: &Ast,
    case_insensitive: bool,
    haystack: &str,
    from: usize,
) -> Result<Option<(usize, usize)>, TooSlow> {
    let mut oracle = Oracle {
        haystack,
        case_insensitive,
        fuel: 3_000_000,
    };
    let boundaries = haystack
        .char_indices()
        .map(|(i, _)| i)
        .chain([haystack.len()]);
    for start in boundaries.filter(|&start| start >= from) {
        let mut end = None;
        let matched = oracle.ends(ast, start, &mut |_, e| {
            end = Some(e);
            Ok(true)
        })?;
        if matched {
            return Ok(Some((start, end.expect("set by the callback"))));
        }
    }
    Ok(None)
}

/// SPEC 4.4, on top of [`oracle_find`].
fn oracle_find_all(
    ast: &Ast,
    case_insensitive: bool,
    haystack: &str,
) -> Result<Vec<(usize, usize)>, TooSlow> {
    let mut all = Vec::new();
    let mut from = 0;
    let mut last_end = None;
    while let Some((start, end)) = oracle_find(ast, case_insensitive, haystack, from)? {
        if start < end || last_end != Some(end) {
            all.push((start, end));
            last_end = Some(end);
        }
        if start < end {
            from = end;
        } else {
            match haystack[end..].chars().next() {
                Some(c) => from = end + c.len_utf8(),
                None => break,
            }
        }
    }
    Ok(all)
}

/// A small deterministic random number generator (SplitMix64), so that the
/// generated cases are the same on every run and every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len() as u64) as usize]
    }
}

/// Atoms that consume a character.
const ATOMS: &[&str] = &[
    "a", "a", "b", "b", "é", "A", ".", "[ab]", "[^a]", "[a-bé]", "[^é-ž]", r"\w", r"\W", r"\d",
    r"\s", "1", " ", r"\.", "[B-b]",
];
/// Atoms that can match the empty string.
const EMPTY_ATOMS: &[&str] = &["^", "$", r"\b", r"\B", "()"];
/// Quantifiers that make a loop, with whether the result can be empty.
const LOOPS: &[(&str, bool)] = &[("*", true), ("+", false), ("{1,}", false), ("{2,}", false)];
/// Quantifiers that write out copies: (text, can the result be empty when
/// the operand cannot?).
const COUNTS: &[(&str, bool)] = &[
    ("?", true),
    ("{0}", true),
    ("{2}", false),
    ("{0,2}", true),
    ("{1,3}", false),
];
const HAYSTACK_CHARS: &[&str] = &[
    "a", "a", "a", "b", "b", "é", "A", "B", " ", "1", "_", ".", "\n", "ž",
];

/// Appends a random valid pattern to `pattern` and returns whether it can
/// match the empty string (SPEC 3.7). `depth` limits how deep groups nest.
///
/// The generator needs that answer for itself: it must not put a loop
/// around an expression that can be empty, because the parser refuses that.
fn random_pattern(rng: &mut Rng, depth: u32, pattern: &mut String) -> bool {
    let branches = if rng.below(4) == 0 {
        2 + rng.below(2)
    } else {
        1
    };
    let mut any_branch_nullable = false;
    for branch in 0..branches {
        if branch > 0 {
            pattern.push('|');
        }
        // Now and then an empty branch.
        let parts = if branches > 1 && rng.below(6) == 0 {
            0
        } else {
            1 + rng.below(3)
        };
        let mut all_parts_nullable = true;
        for _ in 0..parts {
            let mut nullable = if depth > 0 && rng.below(3) == 0 {
                pattern.push('(');
                let inner = random_pattern(rng, depth - 1, pattern);
                pattern.push(')');
                inner
            } else if rng.below(5) == 0 {
                pattern.push_str(rng.pick(EMPTY_ATOMS));
                true
            } else {
                pattern.push_str(rng.pick(ATOMS));
                false
            };
            if rng.below(3) == 0 {
                let loops_allowed = !nullable;
                if loops_allowed && rng.below(2) == 0 {
                    let (text, empty) = LOOPS[rng.below(LOOPS.len() as u64) as usize];
                    pattern.push_str(text);
                    nullable = empty;
                } else {
                    let (text, empty) = COUNTS[rng.below(COUNTS.len() as u64) as usize];
                    pattern.push_str(text);
                    nullable |= empty;
                }
            }
            all_parts_nullable &= nullable;
        }
        any_branch_nullable |= all_parts_nullable;
    }
    any_branch_nullable
}

fn random_haystack(rng: &mut Rng) -> String {
    (0..rng.below(9))
        .map(|_| rng.pick(HAYSTACK_CHARS))
        .collect()
}

/// Compares everything the public API reports with the oracle.
/// Returns `false` if the oracle gave up on this case.
fn check(pattern: &str, case_insensitive: bool, haystack: &str) -> bool {
    let ast =
        parse(pattern).unwrap_or_else(|e| panic!("generated pattern {pattern:?} is invalid: {e}"));
    let regex: Regex = RegexBuilder::new()
        .case_insensitive(case_insensitive)
        .build(pattern)
        .expect("parsed above");
    let context = || {
        format!("pattern {pattern:?}, case_insensitive {case_insensitive}, haystack {haystack:?}")
    };

    let Ok(expected) = oracle_find(&ast, case_insensitive, haystack, 0) else {
        return false;
    };
    assert_eq!(
        regex.find(haystack).map(|m| (m.start, m.end)),
        expected,
        "find: {}",
        context()
    );
    assert_eq!(
        regex.is_match(haystack),
        expected.is_some(),
        "is_match: {}",
        context()
    );

    let Ok(expected_all) = oracle_find_all(&ast, case_insensitive, haystack) else {
        return false;
    };
    let all: Vec<_> = regex
        .find_iter(haystack)
        .map(|m| (m.start, m.end))
        .collect();
    assert_eq!(all, expected_all, "find_iter: {}", context());

    // One matcher for all the searches below: the scratch memory of one
    // search must not leak into the next.
    let mut matcher = regex.matcher();
    for from in haystack
        .char_indices()
        .map(|(i, _)| i)
        .chain([haystack.len()])
    {
        let Ok(expected) = oracle_find(&ast, case_insensitive, haystack, from) else {
            return false;
        };
        assert_eq!(
            matcher.find_at(haystack, from).map(|m| (m.start, m.end)),
            expected,
            "find_at {from}: {}",
            context()
        );
    }
    true
}

#[test]
fn the_automaton_agrees_with_the_definition_on_generated_cases() {
    // Debug builds and Miri do less; the cases are a prefix of the same list.
    // The release number is large on purpose. An earlier design of this
    // engine agreed with its definition on the first 24 000 cases and
    // differed on a later one (DESIGN.md, section 2.3): a test like this one
    // is worth as much as the number of cases it runs.
    let cases = if cfg!(miri) {
        60
    } else if cfg!(debug_assertions) {
        10_000
    } else {
        100_000
    };
    let mut rng = Rng(2024);
    let mut checked = 0;
    for case in 0..cases {
        let mut pattern = String::new();
        random_pattern(&mut rng, 3, &mut pattern);
        let case_insensitive = case % 5 == 0;
        for _ in 0..4 {
            let haystack = random_haystack(&mut rng);
            checked += usize::from(check(&pattern, case_insensitive, &haystack));
        }
    }
    // The fuel limit may skip a few cases, never most of them.
    assert!(
        checked >= cases * 4 * 9 / 10,
        "the oracle gave up on too many cases: {checked} checked"
    );
}

#[test]
fn the_spec_examples_hold_for_the_oracle_and_the_automaton() {
    // (pattern, haystack, match): the table of SPEC 4.3.
    type Example = (&'static str, &'static str, Option<(usize, usize)>);
    let examples: &[Example] = &[
        ("a|ab", "ab", Some((0, 1))),
        ("ab|a", "ab", Some((0, 2))),
        ("b|ab", "ab", Some((0, 2))),
        ("b|^a", "ab", Some((0, 1))),
        ("a*", "baaa", Some((0, 0))),
        ("a+", "baaa", Some((1, 4))),
        ("a{2,3}", "aaaa", Some((0, 3))),
        ("(a|ab)(c|bcd)", "abcd", Some((0, 4))),
        ("(a|b)*b", "abab", Some((0, 4))),
        ("(a|ab)*", "abab", Some((0, 1))),
        ("(a|ab)*c", "ababc", Some((0, 5))),
        ("(|a){2}", "aa", Some((0, 0))),
        ("(|a){2}b", "ab", Some((0, 2))),
        ("(a?){2}a", "aa", Some((0, 2))),
        ("x*", "", Some((0, 0))),
        ("$", "ab", Some((2, 2))),
        (r"\b", "  ab", Some((2, 2))),
        ("^$", "a", None),
    ];
    for &(pattern, haystack, expected) in examples {
        let ast = parse(pattern).unwrap();
        let by_definition = oracle_find(&ast, false, haystack, 0)
            .ok()
            .expect("tiny cases never run out of fuel");
        assert_eq!(
            by_definition, expected,
            "the oracle on {pattern:?} and {haystack:?}"
        );
        assert!(
            check(pattern, false, haystack),
            "{pattern:?} on {haystack:?}"
        );
    }
}

/// `M(pattern, i)` of SPEC 4.2, written out as a list.
fn ends_list(pattern: &str, haystack: &str, i: usize) -> Vec<usize> {
    let ast = parse(pattern).unwrap();
    let mut oracle = Oracle {
        haystack,
        case_insensitive: false,
        fuel: 100_000,
    };
    let mut list = Vec::new();
    // Answering "no" to every end makes the oracle go through all of them.
    let exhausted = oracle.ends(&ast, i, &mut |_, end| {
        list.push(end);
        Ok(false)
    });
    assert!(
        matches!(exhausted, Ok(false)),
        "{pattern:?} ran out of fuel"
    );
    list
}

#[test]
fn the_lists_of_spec_4_2() {
    let examples: &[(&str, &str, &[usize])] = &[
        ("a|ab", "ab", &[1, 2]),
        ("ab|a", "ab", &[2, 1]),
        ("a*", "aaa", &[3, 2, 1, 0]),
        ("a+", "aaa", &[3, 2, 1]),
        ("a?", "b", &[0]),
        ("a{2,3}", "aaaa", &[3, 2]),
        ("(a|ab)(c|bcd)", "abcd", &[4, 3]),
        ("(a|b)*b", "abab", &[4, 2]),
        ("(a|ab)*", "abab", &[1, 3, 4, 2, 0]),
        ("(|a){2}", "aa", &[0, 1, 1, 2]),
        ("(a|){2}", "aa", &[2, 1, 1, 0]),
        ("a$", "ab", &[]),
    ];
    for &(pattern, haystack, expected) in examples {
        assert_eq!(
            ends_list(pattern, haystack, 0),
            expected,
            "M({pattern:?}, 0) in {haystack:?}"
        );
    }
}

#[test]
fn the_find_iter_table_of_spec_4_4() {
    type Example = (&'static str, &'static str, &'static [(usize, usize)]);
    let examples: &[Example] = &[
        ("a", "banana", &[(1, 2), (3, 4), (5, 6)]),
        ("aa", "aaaaa", &[(0, 2), (2, 4)]),
        ("a|ab", "abab", &[(0, 1), (2, 3)]),
        ("", "ab", &[(0, 0), (1, 1), (2, 2)]),
        ("", "é", &[(0, 0), (2, 2)]),
        ("x*", "ab", &[(0, 0), (1, 1), (2, 2)]),
        ("a*", "baaa", &[(0, 0), (1, 4)]),
        ("a*", "aab", &[(0, 2), (3, 3)]),
        ("a|", "ba", &[(0, 0), (1, 2)]),
        (r"\b", "ab cd", &[(0, 0), (2, 2), (3, 3), (5, 5)]),
        ("$", "ab", &[(2, 2)]),
        ("^a", "aaa", &[(0, 1)]),
        (r"\Ba", "aaa", &[(1, 2), (2, 3)]),
    ];
    for &(pattern, haystack, expected) in examples {
        let ast = parse(pattern).unwrap();
        let by_definition = oracle_find_all(&ast, false, haystack)
            .ok()
            .expect("tiny cases never run out of fuel");
        assert_eq!(
            by_definition, expected,
            "the oracle on {pattern:?} and {haystack:?}"
        );
        assert!(
            check(pattern, false, haystack),
            "{pattern:?} on {haystack:?}"
        );
    }
}

/// Pieces that make a random string likely to be an interesting pattern:
/// every metacharacter, what counts and escapes are made of, and
/// characters of one, two and four bytes.
const PATTERN_PIECES: &[&str] = &[
    "a", "b", "é", "😀", "(", ")", "(", ")", "[", "]", "{", "}", "|", "*", "+", "?", "\\", "^",
    "$", ".", "-", ",", "0", "2", "9", "u", "d", "W", "b", "n", "q", " ",
];

#[test]
fn random_strings_never_panic() {
    let cases = if cfg!(miri) { 400 } else { 100_000 };
    let mut rng = Rng(7);
    let mut compiled = 0;
    for _ in 0..cases {
        let pattern: String = (0..rng.below(12))
            .map(|_| rng.pick(PATTERN_PIECES))
            .collect();
        match Regex::new(&pattern) {
            Err(error) => {
                let offset = error.offset();
                assert!(
                    offset < pattern.len() && pattern.is_char_boundary(offset),
                    "pattern {pattern:?}: offset {offset} of {error:?} is not the offset of a character"
                );
            }
            Ok(regex) => {
                compiled += 1;
                for haystack in ["", "ab", "aé😀b\n2-, q"] {
                    assert_eq!(regex.is_match(haystack), regex.find(haystack).is_some());
                    for range in regex.find_iter(haystack) {
                        assert!(
                            haystack.get(range.clone()).is_some(),
                            "pattern {pattern:?}, haystack {haystack:?}: {range:?} is not a slice"
                        );
                    }
                }
            }
        }
    }
    // Both outcomes must be well represented, or the test tests nothing.
    assert!(
        compiled > cases / 20 && compiled < cases * 19 / 20,
        "{compiled} of {cases} random strings compiled"
    );
}
