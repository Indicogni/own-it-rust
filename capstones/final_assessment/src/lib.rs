//! The `rgx` library.
//!
//! This is a blank-file project. SPEC.md says what the engine must do; which
//! modules, types, traits and functions do it is your design.
//!
//! The acceptance tests call only the items in this file, which are the
//! facade of SPEC section 2. Keep their names and what they accept and
//! return; everything else about them is yours. You may give the two
//! structs any fields (or turn `Error` into an enum), return named iterator
//! types instead of `impl Iterator`, add methods, add public items, and
//! split the crate into as many modules as you like.

use std::fmt;
use std::ops::Range;

/// A compiled pattern (SPEC 2.1).
#[derive(Debug)]
pub struct Regex {
    _your_design: (),
}

/// Why a pattern was rejected (SPEC 2.2 and section 5).
#[derive(Debug)]
pub struct Error {
    _your_design: (),
}

impl Regex {
    /// Compiles `pattern`, or says why it cannot be compiled.
    pub fn new(pattern: &str) -> Result<Regex, Error> {
        todo!("SPEC sections 3, 5 and 6: compile {pattern:?}")
    }

    /// Is there a match anywhere in `haystack`?
    pub fn is_match(&self, haystack: &str) -> bool {
        todo!("SPEC section 4: search {haystack:?}")
    }

    /// The byte range of the match in `haystack`, if there is one.
    pub fn find(&self, haystack: &str) -> Option<Range<usize>> {
        todo!("SPEC 4.3: search {haystack:?}")
    }

    /// The successive matches in `haystack`.
    pub fn find_iter(&self, haystack: &str) -> impl Iterator<Item = Range<usize>> {
        std::iter::from_fn(move || todo!("SPEC 4.4: the next match of {self:?} in {haystack:?}"))
    }

    /// The lines of `text` that contain a match, with their line numbers.
    pub fn matching_lines<'h>(&self, text: &'h str) -> impl Iterator<Item = (usize, &'h str)> {
        std::iter::from_fn(move || {
            todo!("SPEC 2.1: the next matching line of {self:?} in {text:?}")
        })
    }
}

impl Error {
    /// The byte offset in the pattern that the error points at.
    pub fn offset(&self) -> usize {
        todo!("SPEC section 5")
    }
}

impl fmt::Display for Error {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("SPEC section 5: the message, and nothing else")
    }
}

impl std::error::Error for Error {}
