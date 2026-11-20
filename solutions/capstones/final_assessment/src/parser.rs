//! Pattern text to syntax tree: a recursive-descent parser.
//!
//! The grammar is SPEC section 3.1. There is one function per rule, and the
//! only recursion is `alternation -> ... -> atom -> group -> alternation`,
//! which the nesting limit bounds. A pattern can therefore never overflow
//! the stack, however long it is.

use crate::ast::{Ast, Look};
use crate::class::{self, RangeSet};
use crate::error::{Error, ErrorKind};
use crate::{MAX_NESTING, MAX_REPEAT};

/// Parses `pattern`. The result is not yet checked against the cost limit.
pub(crate) fn parse(pattern: &str) -> Result<Ast, Error> {
    let mut parser = Parser {
        pattern,
        pos: 0,
        open_groups: 0,
    };
    let whole = parser.alternation()?;
    // `alternation` stops at the end or at a `)` that no group is waiting for.
    match parser.peek() {
        None => Ok(whole.ast),
        Some(_) => Err(parser.error(ErrorKind::UnmatchedCloseParen, parser.pos)),
    }
}

/// A parsed expression, and whether it can match the empty string by the
/// rules of SPEC 3.7.
///
/// The flag is computed on the way up, together with the tree. Asking the
/// finished tree instead would walk a nested group once for every loop
/// around it.
struct Piece {
    ast: Ast,
    nullable: bool,
}

impl Piece {
    /// An atom that consumes one character.
    fn consuming(ast: Ast) -> Piece {
        Piece {
            ast,
            nullable: false,
        }
    }

    /// An assertion: it matches the empty string where it holds.
    fn look(look: Look) -> Piece {
        Piece {
            ast: Ast::Look(look),
            nullable: true,
        }
    }
}

struct Parser<'p> {
    pattern: &'p str,
    /// Byte offset of the next character to read.
    pos: usize,
    /// How many groups are open at `pos`.
    open_groups: u32,
}

/// One element of a class: a single character or a predefined set.
enum ClassItem {
    Char(char),
    Set {
        ranges: &'static [(char, char)],
        negated: bool,
    },
}

/// What an escape sequence means, before the place decides what is allowed.
enum Escape {
    Char(char),
    Set {
        ranges: &'static [(char, char)],
        negated: bool,
    },
    Look(Look),
}

impl Parser<'_> {
    fn error(&self, kind: ErrorKind, offset: usize) -> Error {
        Error::new(kind, offset)
    }

    fn rest(&self) -> &str {
        &self.pattern[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    /// The character after the next one.
    fn peek_second(&self) -> Option<char> {
        self.rest().chars().nth(1)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    /// Steps over one ASCII character that `peek` has shown, and hands
    /// `value` back.
    fn skip_then<T>(&mut self, value: T) -> T {
        self.pos += 1;
        value
    }

    fn at_quantifier(&self) -> bool {
        matches!(self.peek(), Some('*' | '+' | '?' | '{'))
    }

    /// `alternation = concat { "|" concat }`
    fn alternation(&mut self) -> Result<Piece, Error> {
        let first = self.concat()?;
        if self.peek() != Some('|') {
            return Ok(first);
        }
        // One branch that can be empty is enough.
        let mut nullable = first.nullable;
        let mut branches = vec![first.ast];
        while self.eat('|') {
            let branch = self.concat()?;
            nullable |= branch.nullable;
            branches.push(branch.ast);
        }
        Ok(Piece {
            ast: Ast::Alternate(branches),
            nullable,
        })
    }

    /// `concat = { repeat }`
    fn concat(&mut self) -> Result<Piece, Error> {
        // Every part must be able to be empty; no parts at all is empty.
        let mut nullable = true;
        let mut parts = Vec::new();
        while !matches!(self.peek(), None | Some('|' | ')')) {
            let part = self.repeat()?;
            nullable &= part.nullable;
            parts.push(part.ast);
        }
        Ok(Piece {
            ast: Ast::concat(parts),
            nullable,
        })
    }

    /// `repeat = atom [ quantifier ]`
    fn repeat(&mut self) -> Result<Piece, Error> {
        if self.at_quantifier() {
            return Err(self.error(ErrorKind::NothingToRepeat, self.pos));
        }
        let atom = self.atom()?;
        let quantifier = self.pos;
        let (min, max) = match self.peek() {
            Some('*') => self.skip_then((0, None)),
            Some('+') => self.skip_then((1, None)),
            Some('?') => self.skip_then((0, Some(1))),
            Some('{') => self.counts()?,
            _ => return Ok(atom),
        };
        // A loop goes round as long as it likes, so each round must consume
        // something: SPEC 3.7 refuses a loop around an expression that can
        // be empty. A count only writes out copies and is free of this.
        if max.is_none() && atom.nullable {
            return Err(self.error(ErrorKind::EmptyRepetition, quantifier));
        }
        // A second quantifier has nothing of its own to repeat. This also
        // rejects the lazy and possessive forms `*?` and `*+`.
        if self.at_quantifier() {
            return Err(self.error(ErrorKind::NothingToRepeat, self.pos));
        }
        Ok(Piece {
            ast: Ast::repeat(atom.ast, min, max),
            // Zero copies are allowed, or every copy can be empty.
            nullable: min == 0 || atom.nullable,
        })
    }

    /// Reads `{n}`, `{n,}` or `{n,m}` and returns the two bounds.
    fn counts(&mut self) -> Result<(u32, Option<u32>), Error> {
        let open = self.pos;
        let invalid = Error::new(ErrorKind::InvalidRepetition, open);
        self.pos += 1; // the `{`
        let min = self.number().ok_or_else(|| invalid.clone())?;
        let max = if self.eat(',') {
            if self.peek() == Some('}') {
                None
            } else {
                Some(self.number().ok_or_else(|| invalid.clone())?)
            }
        } else {
            Some(min)
        };
        if !self.eat('}') {
            return Err(invalid);
        }
        // The form is right. Now the values: first the size of each count,
        // then their order (SPEC 5.2).
        if min > MAX_REPEAT || max.is_some_and(|max| max > MAX_REPEAT) {
            return Err(self.error(ErrorKind::RepetitionTooLarge, open));
        }
        if max.is_some_and(|max| min > max) {
            return Err(invalid);
        }
        Ok((min, max))
    }

    /// Reads one or more decimal digits. A number too large for `u32`
    /// becomes `u32::MAX`, which is above every limit, so it is reported as
    /// "too large" and never wraps around.
    fn number(&mut self) -> Option<u32> {
        let mut value: Option<u32> = None;
        while let Some(digit) = self.peek().and_then(|c| c.to_digit(10)) {
            self.pos += 1;
            value = Some(value.unwrap_or(0).saturating_mul(10).saturating_add(digit));
        }
        value
    }

    /// `atom = group | class | "." | "^" | "$" | escape | literal`
    fn atom(&mut self) -> Result<Piece, Error> {
        let start = self.pos;
        let c = self
            .bump()
            .expect("`concat` only calls for an atom when a character is left");
        Ok(match c {
            '(' => return self.group(start),
            '[' => Piece::consuming(self.class(start)?),
            '.' => Piece::consuming(Ast::Dot),
            '^' => Piece::look(Look::Start),
            '$' => Piece::look(Look::End),
            '\\' => match self.escape(start)? {
                Escape::Char(c) => Piece::consuming(Ast::Literal(c)),
                Escape::Set { ranges, negated } => Piece::consuming(Ast::Class {
                    set: RangeSet::from_chars(ranges),
                    negated,
                }),
                Escape::Look(look) => Piece::look(look),
            },
            // `]` and `}` on their own are ordinary characters, like
            // everything else that is left.
            c => Piece::consuming(Ast::Literal(c)),
        })
    }

    /// The rest of a group whose `(` at `open` has been read.
    fn group(&mut self, open: usize) -> Result<Piece, Error> {
        // In reading order: the `(` itself is one too many before the `?`
        // after it is unsupported (SPEC 5.1).
        if self.open_groups == MAX_NESTING {
            return Err(self.error(ErrorKind::NestingTooDeep, open));
        }
        if self.peek() == Some('?') {
            return Err(self.error(ErrorKind::UnsupportedGroupSyntax, open));
        }
        self.open_groups += 1;
        let inner = self.alternation()?;
        if !self.eat(')') {
            return Err(self.error(ErrorKind::MissingCloseParen, open));
        }
        self.open_groups -= 1;
        Ok(inner)
    }

    /// The rest of an escape whose `\` at `start` has been read.
    fn escape(&mut self, start: usize) -> Result<Escape, Error> {
        let Some(c) = self.bump() else {
            return Err(self.error(ErrorKind::IncompleteEscape, start));
        };
        Ok(match c {
            'n' => Escape::Char('\n'),
            't' => Escape::Char('\t'),
            'r' => Escape::Char('\r'),
            'd' => Escape::Set {
                ranges: class::DIGIT,
                negated: false,
            },
            'D' => Escape::Set {
                ranges: class::DIGIT,
                negated: true,
            },
            'w' => Escape::Set {
                ranges: class::WORD,
                negated: false,
            },
            'W' => Escape::Set {
                ranges: class::WORD,
                negated: true,
            },
            's' => Escape::Set {
                ranges: class::SPACE,
                negated: false,
            },
            'S' => Escape::Set {
                ranges: class::SPACE,
                negated: true,
            },
            'b' => Escape::Look(Look::WordBoundary),
            'B' => Escape::Look(Look::NotWordBoundary),
            'u' => Escape::Char(
                self.unicode_escape()
                    .ok_or_else(|| self.error(ErrorKind::InvalidUnicodeEscape, start))?,
            ),
            c if c.is_ascii_punctuation() => Escape::Char(c),
            _ => return Err(self.error(ErrorKind::UnknownEscape, start)),
        })
    }

    /// Reads `{H...}` after `\u`: one to six hex digits that name a scalar
    /// value.
    fn unicode_escape(&mut self) -> Option<char> {
        if !self.eat('{') {
            return None;
        }
        let mut value = 0_u32;
        let mut digits = 0;
        while let Some(digit) = self.peek().and_then(|c| c.to_digit(16)) {
            self.pos += 1;
            digits += 1;
            if digits > 6 {
                return None;
            }
            value = value * 16 + digit;
        }
        if digits == 0 || !self.eat('}') {
            return None;
        }
        // Refuses surrogates and values above U+10FFFF.
        char::from_u32(value)
    }

    /// The rest of a class whose `[` at `open` has been read.
    fn class(&mut self, open: usize) -> Result<Ast, Error> {
        let negated = self.eat('^');
        // Collected as they come; `RangeSet::new` sorts and merges once.
        let mut ranges: Vec<(u32, u32)> = Vec::new();
        loop {
            let item_start = self.pos;
            match self.peek() {
                None => return Err(self.error(ErrorKind::MissingCloseBracket, open)),
                Some(']') => {
                    self.pos += 1;
                    break;
                }
                Some(_) => {}
            }
            let item = self.class_item(open)?;
            // A `-` makes a range only between two elements: when the class
            // goes on after it with something other than `]`.
            let is_range =
                self.peek() == Some('-') && !matches!(self.peek_second(), None | Some(']'));
            match item {
                ClassItem::Char(lo) if is_range => {
                    self.pos += 1; // the `-`
                    match self.class_item(open)? {
                        ClassItem::Char(hi) if lo <= hi => ranges.push((lo as u32, hi as u32)),
                        _ => return Err(self.error(ErrorKind::InvalidClassRange, item_start)),
                    }
                }
                ClassItem::Char(c) => ranges.push((c as u32, c as u32)),
                ClassItem::Set {
                    ranges: set,
                    negated: false,
                } => {
                    ranges.extend(set.iter().map(|&(lo, hi)| (lo as u32, hi as u32)));
                }
                ClassItem::Set {
                    ranges: set,
                    negated: true,
                } => {
                    ranges.extend_from_slice(RangeSet::from_chars(set).complement().ranges());
                }
            }
        }
        if ranges.is_empty() {
            return Err(self.error(ErrorKind::EmptyClass, open));
        }
        Ok(Ast::Class {
            set: RangeSet::new(ranges),
            negated,
        })
    }

    /// One element of the class that began at `open`. The caller has checked
    /// that the next character is not the closing `]`.
    fn class_item(&mut self, open: usize) -> Result<ClassItem, Error> {
        let start = self.pos;
        let Some(c) = self.bump() else {
            return Err(self.error(ErrorKind::MissingCloseBracket, open));
        };
        Ok(match c {
            '[' => return Err(self.error(ErrorKind::UnsupportedClassSyntax, start)),
            '\\' => match self.escape(start)? {
                Escape::Char(c) => ClassItem::Char(c),
                Escape::Set { ranges, negated } => ClassItem::Set { ranges, negated },
                // `\b` and `\B` test a position; a class holds characters.
                Escape::Look(_) => return Err(self.error(ErrorKind::UnknownEscape, start)),
            },
            c => ClassItem::Char(c),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(c: char) -> Ast {
        Ast::Literal(c)
    }

    fn rep(sub: Ast, min: u32, max: Option<u32>) -> Ast {
        Ast::Repeat {
            sub: Box::new(sub),
            min,
            max,
        }
    }

    #[test]
    fn precedence_is_alternation_then_concatenation_then_repetition() {
        assert_eq!(
            parse("ab*|c").unwrap(),
            Ast::Alternate(vec![
                Ast::Concat(vec![lit('a'), rep(lit('b'), 0, None)]),
                lit('c')
            ])
        );
        assert_eq!(
            parse("(ab)+").unwrap(),
            rep(Ast::Concat(vec![lit('a'), lit('b')]), 1, None)
        );
    }

    #[test]
    fn empty_pieces_are_allowed() {
        assert_eq!(parse("").unwrap(), Ast::Empty);
        assert_eq!(parse("()").unwrap(), Ast::Empty);
        assert_eq!(
            parse("a|").unwrap(),
            Ast::Alternate(vec![lit('a'), Ast::Empty])
        );
        assert_eq!(
            parse("(|)").unwrap(),
            Ast::Alternate(vec![Ast::Empty, Ast::Empty])
        );
    }

    #[test]
    fn counted_forms() {
        assert_eq!(parse("a{3}").unwrap(), rep(lit('a'), 3, Some(3)));
        assert_eq!(parse("a{3,}").unwrap(), rep(lit('a'), 3, None));
        assert_eq!(parse("a{3,5}").unwrap(), rep(lit('a'), 3, Some(5)));
        assert_eq!(parse("a{0,}").unwrap(), rep(lit('a'), 0, None));
    }

    #[test]
    fn what_costs_nothing_is_empty() {
        for pattern in [
            "a{0}",
            "a{0,0}",
            "(){5}",
            "()()",
            "((){1000}){1000}",
            "(a{3}){0}b{0}",
            "(()()){7}",
        ] {
            assert_eq!(parse(pattern).unwrap(), Ast::Empty, "{pattern}");
        }
        // This one costs 1 and stays.
        assert_eq!(parse("(){2,3}").unwrap(), rep(Ast::Empty, 2, Some(3)));
        assert_eq!(
            parse("a()b").unwrap(),
            Ast::Concat(vec![lit('a'), lit('b')])
        );
    }

    #[test]
    fn a_loop_needs_something_to_consume() {
        // (pattern, offset of the quantifier that makes the loop)
        let refused: &[(&str, usize)] = &[
            ("()*", 2),
            ("()+", 2),
            ("(a*)*", 4),
            ("(a?)+", 4),
            ("(a|)*", 4),
            ("(a*b?)+", 6),
            ("(a{0})*", 6),
            ("(a{0,5}){1,}", 8),
            ("^*", 1),
            (r"\b+", 2),
            ("(^|a)+", 5),
            ("((a|b)*)*", 8),
            ("x(y|(z?))+", 9),
            ("()**", 2),
        ];
        for &(pattern, offset) in refused {
            let error = parse(pattern).expect_err(pattern);
            assert_eq!(
                (error.kind(), error.offset()),
                (ErrorKind::EmptyRepetition, offset),
                "pattern {pattern:?}"
            );
        }
        // A count writes out copies, and `?` is one optional copy: neither
        // is a loop. A loop around something that must consume is fine.
        for pattern in [
            "(a*){3}",
            "(a*){0,5}",
            "(a*)?",
            "()?",
            "(){2}",
            "^?",
            r"\b{2}",
            "(a|b)*",
            "(a+)+",
            "(a*b)+",
            "(^a)*",
            "(a{1,5}){2,}",
            "((a|b)+)*",
        ] {
            assert!(parse(pattern).is_ok(), "{pattern:?} is valid");
        }
    }

    #[test]
    fn closing_brackets_and_braces_are_literals_outside_their_construct() {
        assert_eq!(parse("]}").unwrap(), Ast::Concat(vec![lit(']'), lit('}')]));
    }

    #[test]
    fn dashes_in_classes() {
        let class = |text: &str| match parse(text).unwrap() {
            Ast::Class {
                set,
                negated: false,
            } => set,
            other => panic!("{text} parsed as {other:?}"),
        };
        assert_eq!(class("[a-c]"), RangeSet::from_chars(&[('a', 'c')]));
        assert_eq!(
            class("[-a]"),
            RangeSet::from_chars(&[('-', '-'), ('a', 'a')])
        );
        assert_eq!(
            class("[a-]"),
            RangeSet::from_chars(&[('-', '-'), ('a', 'a')])
        );
        assert_eq!(
            class("[a-c-e]"),
            RangeSet::from_chars(&[('a', 'c'), ('-', '-'), ('e', 'e')])
        );
        assert_eq!(
            class(r"[\d-z]"),
            RangeSet::from_chars(&[('0', '9'), ('-', '-'), ('z', 'z')])
        );
        assert_eq!(class("[+--]"), RangeSet::from_chars(&[('+', '-')]));
        assert_eq!(
            class(r"[\]a^]"),
            RangeSet::from_chars(&[(']', '^'), ('a', 'a')])
        );
    }

    #[test]
    fn errors_carry_the_documented_offset() {
        let cases: &[(&str, ErrorKind, usize)] = &[
            ("a(b", ErrorKind::MissingCloseParen, 1),
            ("ab)", ErrorKind::UnmatchedCloseParen, 2),
            ("a(?:b)", ErrorKind::UnsupportedGroupSyntax, 1),
            ("x[ab", ErrorKind::MissingCloseBracket, 1),
            ("[]", ErrorKind::EmptyClass, 0),
            ("[^]", ErrorKind::EmptyClass, 0),
            ("[az-a]", ErrorKind::InvalidClassRange, 2),
            (r"[a-\d]", ErrorKind::InvalidClassRange, 1),
            ("[a[:alpha:]]", ErrorKind::UnsupportedClassSyntax, 2),
            ("*a", ErrorKind::NothingToRepeat, 0),
            ("a|+", ErrorKind::NothingToRepeat, 2),
            ("a**", ErrorKind::NothingToRepeat, 2),
            ("a{2}{3}", ErrorKind::NothingToRepeat, 4),
            ("a{", ErrorKind::InvalidRepetition, 1),
            ("a{,3}", ErrorKind::InvalidRepetition, 1),
            ("a{3,2}", ErrorKind::InvalidRepetition, 1),
            ("a{1001}", ErrorKind::RepetitionTooLarge, 1),
            ("a{99999999999999999999}", ErrorKind::RepetitionTooLarge, 1),
            ("a{1001,2}", ErrorKind::RepetitionTooLarge, 1),
            (r"a\q", ErrorKind::UnknownEscape, 1),
            (r"[\b]", ErrorKind::UnknownEscape, 1),
            ("ab\\", ErrorKind::IncompleteEscape, 2),
            (r"\u{110000}", ErrorKind::InvalidUnicodeEscape, 0),
            (r"\u{D800}", ErrorKind::InvalidUnicodeEscape, 0),
            (r"é\u{}", ErrorKind::InvalidUnicodeEscape, 2),
        ];
        for &(pattern, kind, offset) in cases {
            let error = parse(pattern).expect_err(pattern);
            assert_eq!(
                (error.kind(), error.offset()),
                (kind, offset),
                "pattern {pattern:?}"
            );
        }
    }

    #[test]
    fn the_nesting_limit_is_exact() {
        let nested = |depth: usize| format!("{}a{}", "(".repeat(depth), ")".repeat(depth));
        assert!(parse(&nested(MAX_NESTING as usize)).is_ok());
        let error = parse(&nested(MAX_NESTING as usize + 1)).unwrap_err();
        assert_eq!(
            (error.kind(), error.offset()),
            (ErrorKind::NestingTooDeep, MAX_NESTING as usize)
        );
    }
}
