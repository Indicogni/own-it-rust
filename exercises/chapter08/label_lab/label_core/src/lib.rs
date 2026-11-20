//! label_core: validated labels for shelves, bins and parcels.
//!
//! A [`Label`] can only be made from text that passed validation, so every
//! function that receives one can rely on it.
//!
//! ```
//! use label_core::{Label, LabelError};
//!
//! let label = Label::new("  North Dock ")?;
//! assert_eq!(label.as_str(), "North Dock");
//! assert_eq!(label.tag(), "tag:North Dock");
//! assert_eq!(Label::new("   "), Err(LabelError::Empty));
//! # Ok::<(), LabelError>(())
//! ```
//!
//! The next three examples must NOT compile. Each one is a promise about the
//! public surface of this crate, and `cargo test --doc` checks all three.
//!
//! The validation helper is an implementation detail:
//!
//! ```compile_fail
//! let cleaned = label_core::validate::clean("  x ");
//! ```
//!
//! The types are reachable only through the crate root:
//!
//! ```compile_fail
//! let label = label_core::model::Label::new("x");
//! ```
//!
//! A `Label` cannot be built around its constructor:
//!
//! ```compile_fail
//! let label = label_core::Label { text: String::from("   ") };
//! ```

// The unfinished functions below leave their parameters and some imports
// unused. This line silences those warnings; delete it after step 1.
#![allow(unused_variables, unused_imports)]

/// The largest number of characters (not bytes) a label may have.
pub const MAX_LEN: usize = 24;

// Step 2: this module moves to src/validate.rs and stops being public.
pub mod validate {
    use crate::{LabelError, MAX_LEN};

    /// 1A. Trim whitespace from both ends of `raw` and return the rest as a
    /// borrow of `raw`. Return `Err(LabelError::Empty)` when nothing is left
    /// and `Err(LabelError::TooLong { len })` when the trimmed text has more
    /// than `MAX_LEN` characters; `len` is that number of characters.
    pub fn clean(raw: &str) -> Result<&str, LabelError> {
        todo!("1A: trim, then check the two rules")
    }
}

// Step 2: this module moves to src/model.rs and stops being public. The two
// types stay reachable as `label_core::Label` and `label_core::LabelError`.
pub mod model {
    use crate::validate;
    use std::fmt;

    /// Why some text cannot be a label.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum LabelError {
        /// Nothing is left after trimming.
        Empty,
        /// The trimmed text has `len` characters, more than
        /// [`MAX_LEN`](crate::MAX_LEN).
        TooLong {
            /// Number of characters after trimming.
            len: usize,
        },
    }

    impl fmt::Display for LabelError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                LabelError::Empty => write!(f, "label is empty"),
                LabelError::TooLong { len } => write!(
                    f,
                    "label has {len} characters; the limit is {}",
                    crate::MAX_LEN
                ),
            }
        }
    }

    impl std::error::Error for LabelError {}

    /// Label text that has passed validation.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Label {
        // Step 2: a public field lets any caller skip `Label::new`.
        pub text: String,
    }

    impl Label {
        /// 1B. Validate `raw` with `validate::clean` and own the cleaned text.
        pub fn new(raw: &str) -> Result<Label, LabelError> {
            todo!("1B: clean, then build the Label")
        }

        /// The validated text.
        pub fn as_str(&self) -> &str {
            &self.text
        }

        /// 1C. Return `"tag:"` followed by the text.
        pub fn tag(&self) -> String {
            todo!("1C: format the tag")
        }
    }
}

pub use model::{Label, LabelError};

/// 1D. Validate `raw` and return its tag: the short form of
/// `Label::new(raw)` followed by `Label::tag`.
///
/// Step 3: add an example to this comment. It must be a code block that
/// `cargo test --doc` runs, with at least one assertion.
pub fn tag(raw: &str) -> Result<String, LabelError> {
    todo!("1D: build a Label, then tag it")
}

/// 6A. Validate `raw` and return it as a slug, for example `"north-dock"`
/// for `" North Dock "`. Use `label_slug::slugify` on the validated text.
///
/// This function exists only when the `slug` feature is enabled.
#[cfg(feature = "slug")]
pub fn slug(raw: &str) -> Result<String, LabelError> {
    todo!("6A: validate, then call the optional dependency")
}

/// 6B. Return the names of the optional features this build was compiled
/// with, in alphabetical order. Today there is only one feature, `slug`.
pub fn enabled_features() -> Vec<&'static str> {
    todo!("6B: ask cfg! about the feature")
}

/// Step 7. Return the optimisation level this crate was compiled with, as
/// the build script recorded it in the environment variable
/// `LABEL_CORE_OPT_LEVEL`. Read the variable at compile time.
pub fn opt_level() -> &'static str {
    todo!("7: read the variable that build.rs passes on")
}

#[cfg(test)]
mod tests {
    // Unit tests are part of this crate, so they can call `validate::clean`
    // even after step 2 has hidden it from every other crate.
    use super::validate::clean;
    use super::{LabelError, MAX_LEN};

    #[test]
    fn step1_clean_trims_and_borrows() {
        let raw = "\t North Dock \n";
        let cleaned = clean(raw).expect("the text is valid");
        assert_eq!(cleaned, "North Dock");
        let start = raw.as_ptr() as usize;
        let at = cleaned.as_ptr() as usize;
        assert!(
            at >= start && at + cleaned.len() <= start + raw.len(),
            "clean must return a slice of its argument, not other text"
        );
    }

    #[test]
    fn step1_clean_rejects_empty_text() {
        assert_eq!(clean(""), Err(LabelError::Empty));
        assert_eq!(clean(" \u{3000}\t"), Err(LabelError::Empty));
    }

    #[test]
    fn step1_clean_counts_characters_not_bytes() {
        let exactly = "é".repeat(MAX_LEN);
        assert_eq!(
            clean(&exactly),
            Ok(exactly.as_str()),
            "{MAX_LEN} characters fit, even when they need more bytes"
        );
        let padded = format!("  {}x  ", "é".repeat(MAX_LEN));
        assert_eq!(
            clean(&padded),
            Err(LabelError::TooLong { len: MAX_LEN + 1 }),
            "measure after trimming, and report characters"
        );
    }
}
