//! `rgx`: a regular-expression engine whose matching time is linear in the
//! length of the haystack, for every pattern.
//!
//! The pipeline has three stages, one module each:
//!
//! ```text
//! pattern text --parser--> syntax tree --compile--> automaton --vm--> matches
//!                 Ast                     Program
//! ```
//!
//! This file is the public face: [`Regex`] owns a compiled automaton and is
//! immutable and shareable between threads; a [`Matcher`] adds the scratch
//! memory that one thread needs to run it. The language, the meaning of a
//! match and the limits are in SPEC.md; DESIGN.md explains the choices.
//!
//! ```
//! use rgx::Regex;
//!
//! let re = Regex::new(r"[a-z]+\d*")?;
//! assert_eq!(re.find("-- ab12 cd"), Some(3..7));
//! assert_eq!(re.find_iter("-- ab12 cd").collect::<Vec<_>>(), [3..7, 8..10]);
//!
//! let error = Regex::new("a(b").unwrap_err();
//! assert_eq!((error.to_string().as_str(), error.offset()), ("missing closing parenthesis", 1));
//! # Ok::<(), rgx::Error>(())
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod ast;
mod class;
mod compile;
mod error;
#[cfg(test)]
mod oracle;
mod parser;
mod vm;

use std::borrow::BorrowMut;
use std::fmt;
use std::iter::{Enumerate, FusedIterator};
use std::ops::Range;
use std::str::{FromStr, SplitTerminator};

use compile::Program;
use vm::{Scratch, Want};

pub use error::{Error, ErrorKind};

/// The largest count in `{n}`, `{n,}` and `{n,m}` (SPEC 6).
pub const MAX_REPEAT: u32 = 1000;
/// The largest number of groups that may be open at one place (SPEC 6).
pub const MAX_NESTING: u32 = 200;
/// The largest cost of a pattern (SPEC 6.1).
pub const MAX_COST: u64 = 100_000;

/// A compiled pattern.
///
/// A `Regex` never changes after it is built, so `&Regex` can be handed to
/// any number of threads. The methods on `Regex` itself are conveniences:
/// each call sets up fresh scratch memory. Code that searches many
/// haystacks should make one [`Matcher`] and keep it.
pub struct Regex {
    pattern: Box<str>,
    program: Program,
}

/// Options for compiling a pattern.
///
/// ```
/// let re = rgx::Regex::builder().case_insensitive(true).build("[^a]b")?;
/// assert!(re.is_match("xB"));
/// assert!(!re.is_match("Ab"));
/// # Ok::<(), rgx::Error>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct RegexBuilder {
    case_insensitive: bool,
}

impl RegexBuilder {
    /// A builder with every option off.
    #[must_use]
    pub fn new() -> RegexBuilder {
        RegexBuilder::default()
    }

    /// Ignore the case of the ASCII letters `A` to `Z` (SPEC 3.9).
    pub fn case_insensitive(&mut self, yes: bool) -> &mut RegexBuilder {
        self.case_insensitive = yes;
        self
    }

    /// Compiles `pattern` with the options chosen so far.
    ///
    /// # Errors
    ///
    /// If `pattern` is not in the language of SPEC section 3 or breaks a
    /// limit of section 6. The [`Error`] has the message and the offset
    /// that SPEC section 5 gives for the pattern.
    pub fn build(&self, pattern: &str) -> Result<Regex, Error> {
        let ast = parser::parse(pattern)?;
        // The cost is computed on the tree. Nothing whose size depends on
        // the repetition counts exists before this check has passed.
        if ast.cost() > MAX_COST {
            return Err(Error::new(ErrorKind::PatternTooLarge, 0));
        }
        Ok(Regex {
            pattern: pattern.into(),
            program: compile::compile(&ast, self.case_insensitive),
        })
    }
}

impl Regex {
    /// Compiles `pattern`.
    ///
    /// # Errors
    ///
    /// As [`RegexBuilder::build`]: one error, with a message and an offset,
    /// for every pattern that SPEC sections 3 and 6 do not allow.
    pub fn new(pattern: &str) -> Result<Regex, Error> {
        RegexBuilder::new().build(pattern)
    }

    /// Starts a [`RegexBuilder`].
    #[must_use]
    pub fn builder() -> RegexBuilder {
        RegexBuilder::new()
    }

    /// The pattern this value was compiled from.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.pattern
    }

    /// A matcher with its own scratch memory. Make one per thread, or one
    /// per loop, and use it for every haystack.
    #[must_use]
    pub fn matcher(&self) -> Matcher<'_> {
        Matcher {
            program: &self.program,
            scratch: Scratch::new(&self.program),
        }
    }

    /// Is there a match anywhere in `haystack`?
    #[must_use]
    pub fn is_match(&self, haystack: &str) -> bool {
        self.matcher().is_match(haystack)
    }

    /// The byte range of the match (SPEC 4.3), if there is one.
    #[must_use]
    pub fn find(&self, haystack: &str) -> Option<Range<usize>> {
        self.matcher().find(haystack)
    }

    /// The successive matches in `haystack`, from left to right, none
    /// overlapping another (SPEC 4.4). Nothing is searched before the first
    /// call of `next`, and no call allocates.
    pub fn find_iter<'r, 'h>(&'r self, haystack: &'h str) -> Matches<'h, Matcher<'r>> {
        Matches::new(self.matcher(), haystack)
    }

    /// The lines of `text` that contain a match, each with its number
    /// (the first line is 1). The lines are slices of `text`.
    pub fn matching_lines<'r, 'h>(&'r self, text: &'h str) -> MatchingLines<'r, 'h> {
        MatchingLines {
            matcher: self.matcher(),
            lines: lines(text),
        }
    }
}

impl fmt::Debug for Regex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The automaton of a large pattern is thousands of lines; the
        // pattern says the same thing.
        f.debug_tuple("Regex").field(&self.as_str()).finish()
    }
}

impl fmt::Display for Regex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Regex {
    type Err = Error;

    fn from_str(pattern: &str) -> Result<Regex, Error> {
        Regex::new(pattern)
    }
}

/// A [`Regex`] together with the scratch memory for running it.
///
/// The methods take `&mut self` because a search writes to that memory.
/// After the matcher is made, no search allocates.
#[derive(Debug)]
pub struct Matcher<'r> {
    program: &'r Program,
    scratch: Scratch,
}

impl<'r> Matcher<'r> {
    /// Is there a match anywhere in `haystack`? Stops at the first position
    /// where the answer is certain.
    pub fn is_match(&mut self, haystack: &str) -> bool {
        vm::search(
            self.program,
            &mut self.scratch,
            haystack,
            0,
            Want::Existence,
        )
        .is_some()
    }

    /// The byte range of the match (SPEC 4.3), if there is one.
    pub fn find(&mut self, haystack: &str) -> Option<Range<usize>> {
        self.find_at(haystack, 0)
    }

    /// Like [`find`](Self::find), for a match that starts at byte offset
    /// `from` or later. `^`, `\b` and `\B` still see the text before `from`.
    ///
    /// # Panics
    ///
    /// If `from` is not on a character boundary of `haystack`.
    pub fn find_at(&mut self, haystack: &str, from: usize) -> Option<Range<usize>> {
        vm::search(self.program, &mut self.scratch, haystack, from, Want::Span)
            .map(|(start, end)| start..end)
    }

    /// Like [`Regex::find_iter`], on this matcher's scratch memory.
    pub fn find_iter<'m, 'h>(&'m mut self, haystack: &'h str) -> Matches<'h, &'m mut Matcher<'r>> {
        Matches::new(self, haystack)
    }
}

/// Iterator over the matches in one haystack. See [`Regex::find_iter`].
///
/// `M` is the matcher it runs on: a [`Matcher`] of its own, or a `&mut` to
/// one that the caller keeps.
#[derive(Debug)]
#[must_use = "iterators are lazy and search nothing until `next` is called"]
pub struct Matches<'h, M> {
    matcher: M,
    haystack: &'h str,
    /// Where the next search starts, or `None` when the iterator is done.
    from: Option<usize>,
    /// Where the previous reported match ended.
    last_end: Option<usize>,
}

impl<'h, M> Matches<'h, M> {
    fn new(matcher: M, haystack: &'h str) -> Matches<'h, M> {
        Matches {
            matcher,
            haystack,
            from: Some(0),
            last_end: None,
        }
    }
}

impl<'r, M: BorrowMut<Matcher<'r>>> Iterator for Matches<'_, M> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Range<usize>> {
        loop {
            let found = self.matcher.borrow_mut().find_at(self.haystack, self.from?);
            let Some(found) = found else {
                self.from = None;
                return None;
            };
            if !found.is_empty() {
                self.from = Some(found.end);
            } else {
                // Searching again from the same place would find the same
                // empty match for ever. Go on one character further; at the
                // end of the haystack there is nowhere to go.
                let next_char = self.haystack[found.end..].chars().next();
                self.from = next_char.map(|c| found.end + c.len_utf8());
                if self.last_end == Some(found.end) {
                    // An empty match that touches the previous match is not
                    // reported (SPEC 4.4).
                    continue;
                }
            }
            self.last_end = Some(found.end);
            return Some(found);
        }
    }
}

impl<'r, M: BorrowMut<Matcher<'r>>> FusedIterator for Matches<'_, M> {}

/// The lines of `text` with their numbers, starting at 1 (SPEC 7.2).
///
/// A line ends at a line feed, which is not part of it. Text after the last
/// line feed is a line too; an empty text has no lines. A carriage return is
/// an ordinary character.
pub fn lines(text: &str) -> Lines<'_> {
    Lines {
        inner: text.split_terminator('\n').enumerate(),
    }
}

/// Iterator over numbered lines. See [`lines`].
#[derive(Clone, Debug)]
#[must_use = "iterators are lazy and do nothing until `next` is called"]
pub struct Lines<'h> {
    inner: Enumerate<SplitTerminator<'h, char>>,
}

impl<'h> Iterator for Lines<'h> {
    type Item = (usize, &'h str);

    fn next(&mut self) -> Option<(usize, &'h str)> {
        self.inner.next().map(|(index, line)| (index + 1, line))
    }
}

impl FusedIterator for Lines<'_> {}

/// Iterator over the lines that contain a match. See
/// [`Regex::matching_lines`].
///
/// The items borrow from the text (`'h`) and not from the regex (`'r`), so
/// they can be kept after the regex is gone.
#[derive(Debug)]
#[must_use = "iterators are lazy and search nothing until `next` is called"]
pub struct MatchingLines<'r, 'h> {
    matcher: Matcher<'r>,
    lines: Lines<'h>,
}

impl<'h> Iterator for MatchingLines<'_, 'h> {
    type Item = (usize, &'h str);

    fn next(&mut self) -> Option<(usize, &'h str)> {
        let matcher = &mut self.matcher;
        self.lines.find(|(_, line)| matcher.is_match(line))
    }
}

impl FusedIterator for MatchingLines<'_, '_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_is_send_and_sync_and_its_error_is_a_std_error() {
        fn shareable<T: Send + Sync>() {}
        fn error<T: std::error::Error + Send + Sync + 'static>() {}
        shareable::<Regex>();
        error::<Error>();
    }

    #[test]
    fn the_cost_limit_is_exact() {
        // cost(a{1000}) = 1000, so 100 of them cost exactly MAX_COST.
        let at_limit = "a{1000}".repeat(100);
        assert!(Regex::new(&at_limit).is_ok());
        let error = Regex::new(&format!("{at_limit}a")).unwrap_err();
        assert_eq!(
            (error.kind(), error.offset()),
            (ErrorKind::PatternTooLarge, 0)
        );
    }

    #[test]
    fn a_cost_that_overflows_u64_is_still_too_large() {
        let mut pattern = String::from("a");
        for _ in 0..8 {
            pattern = format!("({pattern}){{1000}}");
        }
        assert_eq!(
            Regex::new(&pattern).unwrap_err().kind(),
            ErrorKind::PatternTooLarge
        );
    }

    #[test]
    fn empty_matches_in_find_iter() {
        let all = |pattern: &str, haystack: &str| {
            Regex::new(pattern)
                .unwrap()
                .find_iter(haystack)
                .collect::<Vec<_>>()
        };
        assert_eq!(all("a*", "baaa"), [0..0, 1..4]);
        assert_eq!(all("", "aé"), [0..0, 1..1, 3..3]);
        assert_eq!(all("a|", "ba"), [0..0, 1..2]);
        assert_eq!(all(r"\b", "ab cd"), [0..0, 2..2, 3..3, 5..5]);
        assert_eq!(all("^a|a$", "aaa"), [0..1, 2..3]);
    }

    #[test]
    fn a_borrowed_matcher_gives_the_same_matches() {
        let re = Regex::new("[0-9]+").unwrap();
        let mut matcher = re.matcher();
        for haystack in ["a1b22", "", "333"] {
            let borrowed: Vec<_> = matcher.find_iter(haystack).collect();
            assert_eq!(borrowed, re.find_iter(haystack).collect::<Vec<_>>());
        }
    }

    #[test]
    fn lines_follow_the_spec() {
        let numbered = |text| lines(text).collect::<Vec<_>>();
        assert_eq!(numbered(""), []);
        assert_eq!(numbered("\n"), [(1, "")]);
        assert_eq!(numbered("a\nb"), [(1, "a"), (2, "b")]);
        assert_eq!(numbered("a\n\nb\r\n"), [(1, "a"), (2, ""), (3, "b\r")]);
    }
}
