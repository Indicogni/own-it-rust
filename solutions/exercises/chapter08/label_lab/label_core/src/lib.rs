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

/// The largest number of characters (not bytes) a label may have.
pub const MAX_LEN: usize = 24;

// Step 2: the two modules now live in src/validate.rs and src/model.rs.
// Neither is `pub`, so no other crate can write `label_core::validate` or
// `label_core::model`.
mod model;
mod validate;

// The facade: the only public path to the two types.
pub use model::{Label, LabelError};

/// 1D. Validate `raw` and return its tag: the short form of
/// `Label::new(raw)` followed by `Label::tag`.
///
/// Step 3: add an example to this comment. It must be a code block that
/// `cargo test --doc` runs, with at least one assertion.
///
/// ```
/// use label_core::{LabelError, tag};
///
/// assert_eq!(tag(" Bay 7 "), Ok("tag:Bay 7".to_string()));
/// assert_eq!(tag("   "), Err(LabelError::Empty));
/// ```
pub fn tag(raw: &str) -> Result<String, LabelError> {
    Ok(Label::new(raw)?.tag())
}

/// 6A. Validate `raw` and return it as a slug, for example `"north-dock"`
/// for `" North Dock "`. Use `label_slug::slugify` on the validated text.
///
/// This function exists only when the `slug` feature is enabled.
#[cfg(feature = "slug")]
pub fn slug(raw: &str) -> Result<String, LabelError> {
    let label = Label::new(raw)?;
    Ok(label_slug::slugify(label.as_str()))
}

/// 6B. Return the names of the optional features this build was compiled
/// with, in alphabetical order. Today there is only one feature, `slug`.
pub fn enabled_features() -> Vec<&'static str> {
    // `cfg!` is right here: both branches compile in every build, because
    // neither names an item that the feature adds.
    if cfg!(feature = "slug") {
        vec!["slug"]
    } else {
        Vec::new()
    }
}

/// Step 7. Return the optimisation level this crate was compiled with, as
/// the build script recorded it in the environment variable
/// `LABEL_CORE_OPT_LEVEL`. Read the variable at compile time.
pub fn opt_level() -> &'static str {
    env!("LABEL_CORE_OPT_LEVEL")
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
