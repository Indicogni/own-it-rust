//! LESSON 07 — ToOwned and clone-on-write
//! Run: cargo test --test chapter13 l07_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: What owned type corresponds to
//! str::to_owned()? Does Cow::Borrowed allocate when read? When is it promoted to Owned?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Explain why ToOwned generalizes Clone for unsized borrowed data.
//! Distinguish Cow::into_owned from Cow::to_mut.
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch13_scratch.rs):
//! Create Cow<[u8]> from a slice; call to_mut twice, edit once, and confirm the original
//! slice stays unchanged. Do the same with Cow<str> and explain the owned type.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete when its tests, predictions, and explanation are done. The lab is optional.

use std::borrow::Cow;
/// A. Remove every '\r'. Borrow unchanged input, own changed output.
fn unix_text(text: &str) -> Cow<'_, str> {
    todo!("Task 1: implement the documented contract")
}
/// B. Use ToOwned to turn arbitrary borrowed text into an independent String.
fn own_text(text: &str) -> String {
    todo!("Task 2: implement the documented contract")
}
/// C. Append a suffix through Cow::to_mut, returning the resulting Cow.
/// An empty suffix must preserve a borrowed representation.
fn append<'a>(mut text: Cow<'a, str>, suffix: &str) -> Cow<'a, str> {
    todo!("Task 3: implement the documented contract")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, guess};
    use std::hint::black_box;
    #[test]
    fn a_allocation_is_conditional() {
        assert!(
            matches!(unix_text("a\nb"), Cow::Borrowed(_)),
            "text without '\\r' must come back borrowed"
        );
        assert_eq!(unix_text("a\r\nb\r"), "a\nb");
        assert!(
            matches!(unix_text("\r"), Cow::Owned(_)),
            "changed text cannot borrow the input; it must be owned"
        );
    }
    #[test]
    fn b_to_owned_copies_unsized_text() {
        let source = String::from("β");
        let mut owned = own_text(&source);
        owned.push('!');
        assert_eq!(source, "β");
        assert_eq!(owned, "β!");
    }
    #[test]
    fn c_clone_on_write_preserves_source_and_skips_empty_edits() {
        let s = String::from("a");
        let out = append(Cow::Borrowed(&s), "🦀");
        assert!(
            matches!(out, Cow::Owned(_)),
            "appending text to a borrowed Cow must make it owned"
        );
        assert_eq!(out, "a🦀");
        assert_eq!(s, "a", "the borrowed source must stay untouched");
        assert!(
            matches!(append(Cow::Borrowed("a"), ""), Cow::Borrowed(_)),
            "an empty suffix changes nothing, so the Cow must stay borrowed"
        );
        assert_eq!(append(Cow::Owned("x".into()), "!"), "x!");
    }

    #[test]
    fn a_unchanged_text_is_never_copied() {
        let text = "line one\nline two\n".repeat(20);
        let (out, allocations) = count(|| unix_text(black_box(&text)));
        assert!(
            matches!(out, Cow::Borrowed(_)),
            "unchanged text must come back borrowed"
        );
        assert_eq!(
            allocations, 0,
            "check for '\\r' before you allocate: unchanged text needs no new String"
        );
        assert!(
            matches!(unix_text(""), Cow::Borrowed("")),
            "empty text is unchanged text"
        );
    }

    #[test]
    fn c_owned_input_is_edited_in_place() {
        let mut s = String::with_capacity(16);
        s.push('x');
        let ptr = s.as_ptr();
        let out = append(Cow::Owned(s), "yz");
        assert_eq!(out, "xyz");
        assert_eq!(
            out.as_ptr(),
            ptr,
            "to_mut on an owned Cow must edit the same buffer, not copy it"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Heap allocations made by into_owned on each variant.
        let (_, borrowed) = count(|| black_box(Cow::Borrowed(black_box("text")).into_owned()));
        let owned_input: Cow<str> = Cow::Owned(String::from("text"));
        let (_, owned) = count(|| black_box(owned_input.into_owned()));
        let p1: (usize, usize) = guess();
        assert_eq!(
            p1,
            (borrowed, owned),
            "P1: into_owned (left: your prediction, right: the real allocation counts)"
        );

        // P2. After calling to_mut WITHOUT editing, which variant is it? And does ==
        // care about the variant?
        let mut c = Cow::Borrowed("x");
        let _ = c.to_mut();
        let p2: (bool, bool) = guess();
        assert_eq!(
            p2,
            (
                matches!(c, Cow::Owned(_)),
                Cow::Borrowed("x") == Cow::<str>::Owned("x".into())
            ),
            "P2: to_mut and == (left: your prediction, right: the real answers)"
        );
    }
}
