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

// Write the library here.

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
        let p1: usize = crate::guess();
        assert_eq!(
            p1,
            links(&BadField(cause())),
            "P1: the error and its source"
        );

        // P2. `?` and `.into()` can turn a `&str` into a boxed error. What does
        // that error display, and how long is its chain?
        let boxed: Box<dyn Error + Send + Sync> = "disk full".into();
        let p2_text: &str = crate::guess();
        let p2_links: usize = crate::guess();
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
        let p3_is_bad_field: bool = crate::guess();
        let p3_is_parse_int_error: bool = crate::guess();
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
        let p4_text: &str = crate::guess();
        let p4_links: usize = crate::guess();
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
