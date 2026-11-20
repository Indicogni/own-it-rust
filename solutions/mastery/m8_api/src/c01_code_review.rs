//! CODE REVIEW 01 — A library's interface, reviewed against the Rust API Guidelines
//! Run: cargo test --manifest-path mastery/m8_api/Cargo.toml c01_
//! Then: cargo clippy --manifest-path mastery/m8_api/Cargo.toml
//! Read first: "Code review 01" in CONCEPTS.md (same package).
//!
//! A teammate wants to publish this file as version 1.0.0 of a small library,
//! `tagset`. It compiles, and it mostly works. After 1.0.0, every name and every
//! type in it is a promise. This is the last cheap moment to change them.
//!
//! Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md.
//!    Use the checklist of the API Guidelines as your rubric
//!    (<https://rust-lang.github.io/api-guidelines/checklist.html>) and name the
//!    guideline for each finding, for example "C-GETTER". There are two real
//!    bugs as well; find them by reading.
//! 2. Run the tests. They compile this file as the library `tagset` and build
//!    the programs in `fixtures/c01/cases/` against it. Those programs are
//!    written for the reviewed interface, so at first none of them compiles.
//!    Compare what they expect with your review.
//! 3. Rewrite the library. Almost every signature changes; that is the point of
//!    a review before 1.0.0. The cases are the precise contract.
//! 4. Run clippy and resolve its warnings for this file.

use std::fmt;
use std::str::FromStr;

/// A set of tags: short lower-case words without white space.
///
/// The tags are kept sorted and without duplicates. Both rules are invariants
/// of the private field, so no code outside this file can break them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagSet {
    tags: Vec<String>,
}

/// Why a text cannot be a tag.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TagError {
    /// The text is empty.
    Empty,
    /// The text contains white space.
    Whitespace,
}

impl fmt::Display for TagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TagError::Empty => f.write_str("a tag cannot be empty"),
            TagError::Whitespace => f.write_str("a tag cannot contain white space"),
        }
    }
}

impl std::error::Error for TagError {}

impl TagSet {
    /// An empty set.
    pub fn new() -> TagSet {
        TagSet::default()
    }

    /// The number of tags.
    pub fn len(&self) -> usize {
        self.tags.len()
    }

    /// Whether the set has no tags.
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    /// Adds `tag`, in lower case. Returns `Ok(true)` if the tag is new and
    /// `Ok(false)` if the set already had it.
    ///
    /// # Errors
    ///
    /// Returns a [`TagError`] if `tag` is empty or contains white space. The
    /// set is unchanged then.
    pub fn insert(&mut self, tag: &str) -> Result<bool, TagError> {
        if tag.is_empty() {
            return Err(TagError::Empty);
        }
        if tag.contains(char::is_whitespace) {
            return Err(TagError::Whitespace);
        }
        // Compare what will be stored, not what was passed in.
        let tag = tag.to_lowercase();
        match self.tags.binary_search(&tag) {
            Ok(_) => Ok(false),
            Err(position) => {
                // Inserting at the search position keeps the vector sorted
                // without sorting it again.
                self.tags.insert(position, tag);
                Ok(true)
            }
        }
    }

    /// Whether the set has `tag`. Upper and lower case are the same tag.
    pub fn contains(&self, tag: &str) -> bool {
        self.tags.binary_search(&tag.to_lowercase()).is_ok()
    }

    /// The tags in alphabetical order.
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.tags.iter(),
        }
    }

    /// The tags in alphabetical order, as an owned vector.
    pub fn into_vec(self) -> Vec<String> {
        self.tags
    }

    /// A set with the tags of `self` and of `other`.
    pub fn union(&self, other: &TagSet) -> TagSet {
        let mut tags = self.tags.clone();
        tags.extend(other.tags.iter().cloned());
        tags.sort();
        tags.dedup();
        TagSet { tags }
    }
}

/// The iterator of [`TagSet::iter`]. It yields `&str`.
#[derive(Debug, Clone)]
pub struct Iter<'a> {
    inner: std::slice::Iter<'a, String>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        self.inner.next().map(String::as_str)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> IntoIterator for &'a TagSet {
    type Item = &'a str;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

/// The tags, separated by `", "`.
impl fmt::Display for TagSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, tag) in self.tags.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            f.write_str(tag)?;
        }
        Ok(())
    }
}

/// Reads tags that are separated by commas. White space around a tag is
/// ignored, and so are empty parts, as in `"a, , b,"`.
impl FromStr for TagSet {
    type Err = TagError;

    fn from_str(text: &str) -> Result<TagSet, TagError> {
        let mut set = TagSet::new();
        for part in text
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            set.insert(part)?;
        }
        Ok(set)
    }
}

#[cfg(test)]
mod tests {
    use crate::support;

    /// Compiles one program of `fixtures/c01/cases/` against this file, built
    /// as the library crate `tagset`.
    #[track_caller]
    fn case(name: &str) {
        let tagset = support::library_from_file("tagset", "src/c01_code_review.rs");
        support::case(
            &format!("fixtures/c01/cases/{name}"),
            &[("tagset", &tagset)],
        );
    }

    #[test]
    fn a_a_set_is_an_ordinary_value() {
        case("construct.rs");
    }

    #[test]
    fn b_inserting_reports_what_happened() {
        case("insert.rs");
    }

    #[test]
    fn b_one_tag_in_two_spellings_is_one_tag() {
        case("bug_case.rs");
    }

    #[test]
    fn c_reading_uses_the_standard_names_and_traits() {
        case("read.rs");
    }

    #[test]
    fn c_the_text_form_has_no_trailing_separator() {
        case("bug_separator.rs");
    }

    #[test]
    fn d_parsing_returns_a_result() {
        case("parse.rs");
    }

    #[test]
    fn d_union_is_a_method() {
        case("union.rs");
    }

    #[test]
    fn e_the_invariant_cannot_be_broken_from_outside() {
        case("private_field.rs");
        case("struct_expression.rs");
    }
}
