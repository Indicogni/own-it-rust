//! LESSON 04 — An error type that users can act on, and that you can still change
//! Run: cargo test --manifest-path mastery/m8_api/Cargo.toml l04_
//! Read first: "Lesson 04" in CONCEPTS.md (same package).
//!
//! From this lesson on you design the types yourself. This file is the whole
//! source of a library, `quota`. It gives you no signatures: the contract below
//! is prose, and the tests are programs in `fixtures/l04/cases/` that are
//! compiled against this file as the crate `quota`. They name only the items
//! that the contract names. Everything else (fields, helper functions, private
//! types) is your decision.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider: a caller of `parse` gets an
//! error. List three different things callers may want to do with it. Which of
//! them need a type they can inspect, and which need only a text?
//!
//! IMPLEMENT the library from the contract. Keep the supplied tests, and add one
//! case of your own to `fixtures/l04/cases/` (copy `parses.rs` as a start).
//! EXPLAIN afterward: Why does this contract ask for a struct with a `kind()`
//! method and not for a public enum with one variant per failure? Name one
//! later change that the struct allows in a minor release and the enum does not.
//!
//! # The contract
//!
//! The library reads a text that gives limits by name, one per line:
//!
//! ```text
//! # limits for the free plan
//! uploads 20
//! downloads   500
//! ```
//!
//! **`parse(text: &str) -> Result<Vec<Quota>, ParseError>`** returns the quotas
//! in the order of the text.
//!
//! - White space around a line is ignored. Empty lines, and lines whose first
//!   character is `#`, are skipped.
//! - Every other line is a name, then white space, then the limit. The name is
//!   the text before the first white space. The limit is the rest of the line
//!   without the white space around it, and must parse as a `u32`.
//! - Lines are numbered from 1, and skipped lines count.
//! - `parse` stops at the first line that is wrong. On one line, the checks are
//!   made in the order of the table below.
//!
//! **`Quota`** has the methods `name(&self) -> &str` and `limit(&self) -> u32`,
//! and implements `Debug`, `Clone` and `PartialEq`.
//!
//! **`ParseError`** is a struct without public fields. It has two methods:
//! `kind(&self) -> ErrorKind` and `line(&self) -> usize`.
//!
//! **`ErrorKind`** is an enum with exactly these variants today. More will come,
//! so a user's `match` must be forced to have a wildcard arm. It implements
//! `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` and `Hash`.
//!
//! | The line is wrong because       | `kind()`       | The error is displayed as, for line 2 and the name `uploads` |
//! | ------------------------------- | -------------- | ------------------------------------------------------------- |
//! | there is nothing after the name | `MissingLimit` | ``line 2: `uploads` has no limit``                            |
//! | the limit is not a `u32`        | `InvalidLimit` | ``line 2: invalid limit for `uploads` ``                      |
//! | an earlier line has this name   | `Duplicate`    | ``line 2: `uploads` is defined twice``                        |
//!
//! `ParseError` behaves like a good error of the standard library:
//!
//! - It implements `std::error::Error`, `Debug` and `Display`, and it is `Send`,
//!   `Sync` and `'static`, so that it can cross threads and can be boxed.
//! - Its `Display` text is exactly the text of the table: lower case, no full
//!   stop, and no line break.
//! - For `InvalidLimit`, `source()` returns the `std::num::ParseIntError` that
//!   the standard library reported. The `Display` text does not repeat that
//!   error's message: a program that prints the whole chain would otherwise
//!   print it twice. For the other kinds, `source()` is `None`.

use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

/// One limit: a name and the largest allowed number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quota {
    name: String,
    limit: u32,
}

impl Quota {
    /// The name, as written in the text.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The largest allowed number.
    pub fn limit(&self) -> u32 {
        self.limit
    }
}

/// What is wrong with a line. More kinds may be added in a minor release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// There is nothing after the name.
    MissingLimit,
    /// The limit is not a `u32`.
    InvalidLimit,
    /// An earlier line has the same name.
    Duplicate,
}

/// The first wrong line of a text.
///
/// A struct with private fields, not an enum: callers branch on [`kind`], and
/// the library stays free to carry more data (a column, the file name) or to
/// replace the parser behind `source()` without a major release.
///
/// [`kind`]: ParseError::kind
#[derive(Debug)]
pub struct ParseError {
    kind: ErrorKind,
    line: usize,
    name: String,
    // Kept as the concrete type: it is private, so it is not a promise. Callers
    // see it only as `&dyn Error` through `source()`.
    source: Option<ParseIntError>,
}

impl ParseError {
    /// What is wrong.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The number of the wrong line, counted from 1.
    pub fn line(&self) -> usize {
        self.line
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ParseError {
            kind, line, name, ..
        } = self;
        match kind {
            ErrorKind::MissingLimit => write!(f, "line {line}: `{name}` has no limit"),
            ErrorKind::InvalidLimit => write!(f, "line {line}: invalid limit for `{name}`"),
            ErrorKind::Duplicate => write!(f, "line {line}: `{name}` is defined twice"),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

/// Parses limits, one `name limit` pair per line.
///
/// # Errors
///
/// Returns the first wrong line as a [`ParseError`].
pub fn parse(text: &str) -> Result<Vec<Quota>, ParseError> {
    let mut quotas: Vec<Quota> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let error = |kind, name: &str, source| ParseError {
            kind,
            line: index + 1,
            name: name.to_string(),
            source,
        };
        let Some((name, limit)) = line.split_once(char::is_whitespace) else {
            return Err(error(ErrorKind::MissingLimit, line, None));
        };
        let limit = limit
            .trim()
            .parse::<u32>()
            .map_err(|source| error(ErrorKind::InvalidLimit, name, Some(source)))?;
        if quotas.iter().any(|quota| quota.name == name) {
            return Err(error(ErrorKind::Duplicate, name, None));
        }
        quotas.push(Quota {
            name: name.to_string(),
            limit,
        });
    }
    Ok(quotas)
}

#[cfg(test)]
mod tests {
    use crate::support;

    /// Compiles one program of `fixtures/l04/cases/` against this file, built
    /// as the library crate `quota`.
    #[track_caller]
    fn case(name: &str) {
        let quota = support::library_from_file("quota", "src/l04_errors.rs");
        support::case(&format!("fixtures/l04/cases/{name}"), &[("quota", &quota)]);
    }

    #[test]
    fn a_good_text_is_parsed() {
        case("parses.rs");
    }

    #[test]
    fn b_errors_name_the_kind_and_the_line() {
        case("kinds_and_lines.rs");
    }

    #[test]
    fn c_errors_are_displayed_as_the_contract_says() {
        case("display.rs");
    }

    #[test]
    fn d_the_error_is_a_standard_error() {
        case("error_trait.rs");
    }

    #[test]
    fn d_the_cause_is_in_the_chain_and_not_in_the_text() {
        case("source_chain.rs");
    }

    #[test]
    fn e_users_must_expect_more_kinds() {
        case("match_without_wildcard.rs");
    }

    #[test]
    fn e_users_cannot_build_a_parse_error() {
        case("no_public_constructor.rs");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        use std::error::Error;
        use std::fmt;
        use std::num::ParseIntError;

        /// An error with a cause.
        #[derive(Debug)]
        struct BadField(ParseIntError);

        impl fmt::Display for BadField {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bad field")
            }
        }

        impl Error for BadField {
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                Some(&self.0)
            }
        }

        /// The number of errors in a chain: the error, its source, and so on.
        fn links(error: &(dyn Error + 'static)) -> usize {
            let mut count = 1;
            let mut current = error;
            while let Some(source) = current.source() {
                count += 1;
                current = source;
            }
            count
        }

        let cause = || "x1".parse::<u8>().expect_err("x1 is not a number");

        // P1. How many errors are in the chain of a `BadField`?
        let p1: usize = 2;
        assert_eq!(
            p1,
            links(&BadField(cause())),
            "P1: the error and its source"
        );

        // P2. `?` and `.into()` can turn a `&str` into a boxed error. What does
        // that error display, and how long is its chain?
        let boxed: Box<dyn Error + Send + Sync> = "disk full".into();
        let p2_text: &str = "disk full";
        let p2_links: usize = 1;
        assert_eq!(p2_text, boxed.to_string(), "P2: the text is the message");
        assert_eq!(
            p2_links,
            links(&*boxed),
            "P2: an error made from text has no source"
        );

        // P3. A boxed error has forgotten its type at compile time. Which of
        // these two questions about the box does `downcast_ref` answer with
        // `Some`?
        let boxed: Box<dyn Error + Send + Sync> = Box::new(BadField(cause()));
        let p3_is_bad_field: bool = true;
        let p3_is_parse_int_error: bool = false;
        assert_eq!(
            p3_is_bad_field,
            boxed.downcast_ref::<BadField>().is_some(),
            "P3: the box holds a BadField"
        );
        assert_eq!(
            p3_is_parse_int_error,
            boxed.downcast_ref::<ParseIntError>().is_some(),
            "P3: downcasting looks at the error itself, not at its sources"
        );

        // P4. `std::io::Error::other` wraps any error. How long is the chain of
        // the wrapper? (Count: the io error, then whatever `source()` returns.)
        let wrapped = std::io::Error::other(BadField(cause()));
        let p4_text: &str = "bad field";
        let p4_links: usize = 2;
        assert_eq!(
            p4_text,
            wrapped.to_string(),
            "P4: an io error displays as the error inside it"
        );
        assert_eq!(
            p4_links,
            links(&wrapped),
            "P4: the io error stands in for the error inside it: its `source()` is the \
             source of the inner error, so the BadField is not counted twice"
        );
    }
}
