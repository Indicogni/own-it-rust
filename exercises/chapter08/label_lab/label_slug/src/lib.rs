//! label_slug: turn text into a slug that is safe in a file name or a URL.
//!
//! This crate is complete. In the lab it plays the part of an optional
//! dependency: `label_core` uses it only when its `slug` feature is enabled.
//!
//! ```
//! assert_eq!(label_slug::slugify("North Dock #7"), "north-dock-7");
//! ```

/// Returns `text` as a slug.
///
/// ASCII letters become lower case and ASCII digits are kept. Every run of
/// other characters becomes one `-`. The result never starts or ends with
/// `-`, so text without any ASCII letter or digit gives an empty string.
///
/// ```
/// use label_slug::slugify;
///
/// assert_eq!(slugify("  Cold   Storage  "), "cold-storage");
/// assert_eq!(slugify("a_b/c"), "a-b-c");
/// assert_eq!(slugify("港"), "");
/// ```
pub fn slugify(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn separators_collapse_and_are_trimmed() {
        assert_eq!(slugify("--A--B--"), "a-b");
        assert_eq!(slugify("Bay 12 (east)"), "bay-12-east");
        assert_eq!(slugify(""), "");
        assert_eq!(slugify("***"), "");
    }

    #[test]
    fn non_ascii_text_is_a_separator() {
        assert_eq!(slugify("caf\u{e9} au lait"), "caf-au-lait");
    }
}
