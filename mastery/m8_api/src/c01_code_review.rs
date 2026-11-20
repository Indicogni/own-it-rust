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

/// A set of tags: short lower-case words. The tags are kept sorted and
/// without duplicates.
pub struct TagSet {
    pub tags: Vec<String>,
}

impl TagSet {
    /// Makes an empty set.
    pub fn create() -> TagSet {
        TagSet { tags: Vec::new() }
    }

    /// The number of tags.
    pub fn get_len(&self) -> usize {
        self.tags.len()
    }

    /// Adds a tag, in lower case. With `strict`, a tag that contains a space is
    /// refused. Returns false if the tag was refused or was already there.
    pub fn add_tag(&mut self, tag: &String, strict: bool) -> bool {
        if strict && tag.contains(' ') {
            return false;
        }
        if self.tags.contains(tag) {
            return false;
        }
        self.tags.push(tag.to_lowercase());
        self.tags.sort();
        true
    }

    /// Whether the set has this tag. Upper and lower case are the same tag.
    pub fn has_tag(&self, tag: &String) -> bool {
        self.tags.binary_search(&tag.to_lowercase()).is_ok()
    }

    /// The tags as one text, separated by ", ".
    pub fn as_string(&self) -> String {
        let mut text = String::new();
        for tag in &self.tags {
            text = text + tag + ", ";
        }
        text
    }

    /// The tags as a vector.
    pub fn to_vec(self) -> Vec<String> {
        self.tags
    }

    /// The tags, one after the other.
    pub fn tags_iter(&self) -> std::slice::Iter<'_, String> {
        self.tags.iter()
    }

    /// Reads tags that are separated by commas and adds them. A message for
    /// every tag that was refused is pushed to `errors`.
    pub fn load(&mut self, text: &str, errors: &mut Vec<String>) {
        for part in text.split(',') {
            let tag = part.trim().to_string();
            if tag.is_empty() {
                continue;
            }
            if !self.add_tag(&tag, true) {
                errors.push(format!("bad tag: {tag}"));
            }
        }
    }
}

/// A set with the tags of both sets.
pub fn merge(a: &TagSet, b: &TagSet) -> TagSet {
    let mut merged = TagSet::create();
    for tag in a.tags.iter().chain(b.tags.iter()) {
        merged.add_tag(tag, false);
    }
    merged
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
