//! Integration tests for label_core.
//!
//! Cargo compiles every file in `tests/` as a separate crate that depends on
//! the library. This file can therefore use only what a real caller can use:
//! the public API. Tests named `stepN_...` belong to step N of the lab.

use label_core::{Label, LabelError, MAX_LEN, tag};

#[test]
fn step1_labels_are_trimmed_and_tagged() {
    let label = Label::new("  North Dock\n").expect("the text is valid");
    assert_eq!(label.as_str(), "North Dock");
    assert_eq!(label.tag(), "tag:North Dock");
    let inner = Label::new("a  b").expect("inner spaces are allowed");
    assert_eq!(inner.as_str(), "a  b", "only the ends are trimmed");
    assert_eq!(
        Label::new("港").map(|label| label.tag()),
        Ok("tag:港".into())
    );
}

#[test]
fn step1_invalid_text_is_rejected() {
    assert_eq!(Label::new(""), Err(LabelError::Empty));
    assert_eq!(Label::new(" \t "), Err(LabelError::Empty));
    let long = "x".repeat(MAX_LEN + 3);
    assert_eq!(
        Label::new(&long),
        Err(LabelError::TooLong { len: MAX_LEN + 3 })
    );
    assert!(
        Label::new(&"x".repeat(MAX_LEN)).is_ok(),
        "the limit itself fits"
    );
}

#[test]
fn step1_errors_explain_themselves() {
    assert_eq!(LabelError::Empty.to_string(), "label is empty");
    assert_eq!(
        LabelError::TooLong { len: 30 }.to_string(),
        "label has 30 characters; the limit is 24"
    );
    // LabelError works wherever a standard error is expected.
    let boxed: Box<dyn std::error::Error> = Box::new(LabelError::Empty);
    assert_eq!(boxed.to_string(), "label is empty");
}

#[test]
fn step1_facade_function_matches_the_type() {
    assert_eq!(tag(" Bay 7 "), Ok("tag:Bay 7".to_string()));
    assert_eq!(tag("a:b"), Ok("tag:a:b".to_string()));
    assert_eq!(tag("  "), Err(LabelError::Empty));
    for raw in ["x", " padded ", "", &"y".repeat(40)] {
        assert_eq!(
            tag(raw),
            Label::new(raw).map(|label| label.tag()),
            "tag({raw:?}) must agree with Label::new followed by Label::tag"
        );
    }
}

/// Runs only in a build WITHOUT the `slug` feature (the default).
#[cfg(not(feature = "slug"))]
#[test]
fn step6_default_build_has_no_optional_parts() {
    assert_eq!(
        label_core::enabled_features(),
        Vec::<&str>::new(),
        "this build was compiled without --features slug"
    );
}

/// Runs only in a build WITH the `slug` feature:
/// `cargo test -p label_core --features slug`.
#[cfg(feature = "slug")]
mod with_slug {
    use label_core::{LabelError, MAX_LEN, enabled_features, slug};

    #[test]
    fn step6_feature_is_reported() {
        assert_eq!(enabled_features(), ["slug"]);
    }

    #[test]
    fn step6_slug_validates_first() {
        assert_eq!(slug(" North Dock #7 "), Ok("north-dock-7".to_string()));
        assert_eq!(slug("a_b/c"), Ok("a-b-c".to_string()));
        assert_eq!(slug("   "), Err(LabelError::Empty));
        assert_eq!(
            slug(&"x".repeat(MAX_LEN + 1)),
            Err(LabelError::TooLong { len: MAX_LEN + 1 })
        );
        assert_eq!(
            slug("港"),
            Ok(String::new()),
            "a valid label may still have an empty slug; that is label_slug's rule"
        );
    }

    /// `slug = ["dep:label_slug"]` keeps the dependency's name out of the
    /// feature list. Without the `dep:` prefix Cargo would also create a
    /// feature called `label_slug`, and callers could start to rely on it.
    #[test]
    #[allow(unexpected_cfgs)]
    fn step6_dependency_name_is_not_a_feature() {
        let dependency_is_a_feature = cfg!(feature = "label_slug");
        assert!(
            !dependency_is_a_feature,
            "write the feature as slug = [\"dep:label_slug\"], with the dep: prefix"
        );
    }
}

#[test]
fn step7_build_script_passes_the_opt_level() {
    // The dev and test profiles use opt-level 0; the release profile uses 3.
    let expected = if cfg!(debug_assertions) { "0" } else { "3" };
    assert_eq!(
        label_core::opt_level(),
        expected,
        "opt_level() must report what Cargo told the build script"
    );
}
