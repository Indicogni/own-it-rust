//! CODE REVIEW 01 — A teammate's pull request
//! Run: cargo test --test chapter17 c01_
//! Then: cargo clippy --test chapter17
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and some behavior tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every byte-versus-scalar mistake, and every allocation
//!    that borrowing or a better std API would avoid.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. You MAY change parameter and return types where the doc comment
//!    says so; the tests are written to accept the better signature.
//! 4. Run clippy and resolve its warnings for this file.

use std::borrow::Cow;

/// Turn a title into a URL slug. First lowercase the whole title with
/// `str::to_lowercase` (not one char at a time: the two differ for a Greek
/// capital sigma at the end of a word). Then keep every scalar of the lowercase
/// text for which `char::is_alphanumeric` is true, replace every run of other
/// scalars with one '-', and trim '-' from both ends. A scalar that lowercasing
/// produces and that is not alphanumeric (such as a combining mark) counts as
/// "other". No Unicode normalization is done. You may change the parameter type.
// REVIEW: `&str` accepts a `&String` and also literals and slices. The test
// was `is_ascii_alphanumeric`, which deleted every non-ASCII letter. Each kept
// char was turned into a temporary String (`c.to_string()`); `push` writes it
// into the one output buffer. A dash is written only when another kept char
// follows, so there is nothing to trim at the end and no second String.
fn slugify(title: &str) -> String {
    let lower = title.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut pending_dash = false;
    for c in lower.chars() {
        if c.is_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c);
        } else {
            pending_dash = true;
        }
    }
    out
}

/// Shorten text for display. If it has more than `max` Unicode scalars, keep
/// the first `max - 1` and append '…', so the result has exactly `max`
/// scalars. Text that fits comes back unchanged. `max` is at least 1.
/// You may change the return type.
// REVIEW: the old code compared and sliced by bytes. It cut non-ASCII text too
// early and panicked when the cut fell inside a scalar. `Cow` returns text that
// fits as a borrow, so the common case allocates nothing.
fn truncate_display(text: &str, max: usize) -> Cow<'_, str> {
    // Byte offset at which scalar number `max - 1` starts: the end of the part
    // to keep, if the text turns out to be too long.
    let mut cut = 0;
    for (position, (offset, _)) in text.char_indices().enumerate() {
        if position == max - 1 {
            cut = offset;
        }
        if position == max {
            // A scalar number `max` exists, so the text has more than `max`.
            return Cow::Owned(format!("{}…", &text[..cut]));
        }
    }
    Cow::Borrowed(text)
}

/// Parse "k=v;k=v" settings. Segments are separated by ';'. Skip empty
/// segments and segments without '='; split each pair on its first '='.
/// You may change the return type to borrow from `text`.
// REVIEW: `split_once` says "first '='" directly and returns None for a segment
// without one (which covers the empty segment). The keys and values are slices
// of `text`, so the only allocation is the Vec.
fn parse_pairs(text: &str) -> Vec<(&str, &str)> {
    text.split(';')
        .filter_map(|segment| segment.split_once('='))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;

    /// Calls `slugify` with a `&String`, which also coerces to `&str`.
    fn slug(title: &str) -> String {
        let owned = String::from(title);
        slugify(&owned)
    }

    fn pairs<A: AsRef<str>, B: AsRef<str>>(v: &[(A, B)]) -> Vec<(&str, &str)> {
        v.iter().map(|(a, b)| (a.as_ref(), b.as_ref())).collect()
    }

    #[test]
    fn a_slugify_collapses_separators() {
        assert_eq!(slug("  Hello, World!  "), "hello-world");
        assert_eq!(slug("Rust 2024"), "rust-2024");
        assert_eq!(slug("--"), "");
    }

    #[test]
    fn a_slugify_keeps_unicode_letters() {
        assert_eq!(slug("Café Olé"), "café-olé");
        assert_eq!(slug("ΣΟΦΙΑ & Ünïcode"), "σοφια-ünïcode");
        assert_eq!(
            slug("ΟΔΟΣ"),
            "οδος",
            "lowercase the whole title: a capital sigma at the end of a word becomes 'ς'"
        );
    }

    #[test]
    fn a_slugify_does_not_allocate_per_char() {
        let title = "word ".repeat(40);
        let (out, allocations) = count(|| slugify(&title));
        assert!(out.starts_with("word-word"));
        assert!(
            allocations <= 10,
            "a 200-byte title cost {allocations} allocations; push each kept char \
             into one String instead of building a String per char"
        );
    }

    #[test]
    fn b_truncate_counts_scalars() {
        assert_eq!(truncate_display("hello", 5), "hello");
        assert_eq!(truncate_display("hello world", 6), "hello…");
        assert_eq!(truncate_display("héllo wörld", 5), "héll…");
        assert_eq!(truncate_display("é", 1), "é");
        assert_eq!(truncate_display("🦀🦀🦀", 2), "🦀…");
        assert_eq!(truncate_display("ab", 1), "…");
    }

    #[test]
    fn b_truncate_borrows_text_that_fits() {
        let text = "short enough";
        let (len, allocations) = count(|| truncate_display(text, 40).len());
        assert_eq!(len, text.len());
        assert_eq!(
            allocations, 0,
            "text that fits cost {allocations} allocation(s); it can be returned borrowed"
        );
    }

    #[test]
    fn c_parse_pairs_splits_on_the_first_equals() {
        let out = parse_pairs("a=1;;b=x=y;junk;c=");
        assert_eq!(pairs(&out), [("a", "1"), ("b", "x=y"), ("c", "")]);
        assert!(parse_pairs("").is_empty());
    }

    #[test]
    fn c_parse_pairs_borrows_from_the_input() {
        let text: String = (0..50).map(|i| format!("key{i}=value{i};")).collect();
        let (n, allocations) = count(|| parse_pairs(&text).len());
        assert_eq!(n, 50);
        assert!(
            allocations <= 8,
            "50 pairs cost {allocations} allocations; borrow each key and value from `text`"
        );
    }
}
