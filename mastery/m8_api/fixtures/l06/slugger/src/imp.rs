//! The implementation. You do not need to edit this file.
//!
//! Every `cfg!(mutant_...)` below switches on one plausible bug. The names are
//! never set in a normal build. Lesson 06 sets them one at a time and runs your
//! documentation examples against the wrong library: an example that still
//! passes does not describe that part of the behaviour.

use crate::TagError;

/// The largest number of characters in a tag.
pub(crate) const MAX_TAG: usize = 20;

pub(crate) fn slug(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    // A separator was seen since the last kept character.
    let mut pending = false;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            if pending && !out.is_empty() {
                out.push('-');
            }
            pending = false;
            out.push(c.to_ascii_lowercase());
        } else {
            if cfg!(mutant_slug_keeps_runs) && pending && !out.is_empty() {
                out.push('-');
            }
            pending = true;
        }
    }
    if cfg!(mutant_slug_trailing_dash) && pending && !out.is_empty() {
        out.push('-');
    }
    out
}

pub(crate) fn truncate(slug: &str, max: usize) -> &str {
    if !cfg!(mutant_truncate_accepts_zero) {
        assert!(max > 0, "`max` must be at least 1");
    }
    let mut end = max.min(slug.len());
    while !slug.is_char_boundary(end) {
        end -= 1;
    }
    let cut = &slug[..end];
    if cfg!(mutant_truncate_keeps_dash) {
        cut
    } else {
        cut.trim_end_matches('-')
    }
}

pub(crate) fn check_tag(text: &str) -> Result<(), TagError> {
    if text.is_empty() {
        return Err(TagError::Empty);
    }
    if text.chars().count() > MAX_TAG {
        return Err(TagError::TooLong { max: MAX_TAG });
    }
    for c in text.chars() {
        let allowed = c.is_ascii_lowercase()
            || c.is_ascii_digit()
            || c == '-'
            || (cfg!(mutant_tag_allows_upper) && c.is_ascii_uppercase());
        if !allowed {
            return Err(TagError::InvalidChar(c));
        }
    }
    Ok(())
}
