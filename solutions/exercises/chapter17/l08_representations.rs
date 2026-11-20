//! LESSON 08 — UTF-8, borrowed text views, and delayed allocation
//! Run: cargo test --test chapter17 l08_
//! Read first: "Lesson 08" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does into_bytes copy a String allocation? What information
//! does FromUtf8Error retain? Which Cow variant does from_utf8_lossy return for valid
//! input?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why should OS strings and paths not be forced through
//! to_str().unwrap()? Which conversions guarantee UTF-8 and which preserve arbitrary bytes?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch17_scratch.rs):
//! Borrow String as str, [u8], OsStr, and Path. Build text with collect<char>(), extend,
//! and join, then split ownership with into_bytes. Try from_utf8 and from_utf8_lossy on
//! [0x66,0x80,0x6f]; record exact errors/replacement scalars. Paths are covered further in
//! chapter 18.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete when its tests, predictions, and explanation are done. The lab is optional.

use std::borrow::Cow;
/// A. Strictly convert owned bytes to String; on invalid UTF-8 return original bytes.
fn decode(bytes: Vec<u8>) -> Result<String, Vec<u8>> {
    // The error owns the rejected Vec and gives it back without a copy.
    String::from_utf8(bytes).map_err(|err| err.into_bytes())
}
/// B. Repair invalid UTF-8 with U+FFFD. Borrow bytes as text when valid.
fn display_bytes(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}
/// C. Trim words and join nonempty words with one ASCII space.
/// Treat strings as a collection of borrowed slices, allocate no intermediate owned Strings.
fn joined(words: &[&str]) -> String {
    // One output String; every word stays a borrowed slice until it is copied
    // into that String. (Collecting the slices into a `Vec<&str>` and calling
    // `join(" ")` is an equally good answer.)
    let mut out = String::new();
    for word in words.iter().map(|word| word.trim()) {
        if word.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}
/// D. Return the same allocation as owned bytes; do not clone.
fn into_packet(text: String) -> Vec<u8> {
    text.into_bytes()
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_strict_conversion_preserves_invalid_input() {
        let bytes = "é".as_bytes().to_vec();
        let ptr = bytes.as_ptr();
        let text = decode(bytes).unwrap();
        assert_eq!(text, "é");
        assert_eq!(
            text.as_ptr(),
            ptr,
            "decode must reuse the Vec's buffer for the String, not copy the bytes"
        );
        assert_eq!(decode(vec![b'a', 0xff]), Err(vec![b'a', 0xff]));
        let bad = vec![b'x', 0xe9, b'y'];
        let ptr = bad.as_ptr();
        let back = decode(bad).unwrap_err();
        assert_eq!(
            back.as_ptr(),
            ptr,
            "on invalid UTF-8 the caller must get the original buffer back, not a copy"
        );
        assert_eq!(decode(vec![]), Ok(String::new()));
    }
    #[test]
    fn b_lossy_conversion_allocates_only_when_repairing() {
        assert!(
            matches!(display_bytes(b"ok"), Cow::Borrowed(_)),
            "valid UTF-8 needs no repair, so the result must borrow the input bytes"
        );
        assert_eq!(display_bytes(&[b'a', 0xff, b'b']), "a\u{FFFD}b");
        assert!(
            matches!(display_bytes(&[0xff]), Cow::Owned(_)),
            "a repaired text is new data, so the result must be Cow::Owned"
        );
    }
    #[test]
    fn c_join_omits_blanks_without_modifying_words() {
        assert_eq!(joined(&[" a ", "", " \t", "é"]), "a é");
        assert_eq!(joined(&[]), "");
        assert_eq!(joined(&["\u{2003}x\u{2003}", "y z"]), "x y z");
    }
    #[test]
    fn c_join_allocates_no_string_per_word() {
        let words: Vec<String> = (0..100).map(|i| format!(" w{i} ")).collect();
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        let (out, allocations) = count(|| joined(&words));
        assert!(out.starts_with("w0 w1 w2"));
        assert!(
            allocations <= 16,
            "joining 100 words cost {allocations} allocations; build one output String \
             from borrowed slices instead of one String per word"
        );
    }
    #[test]
    fn d_owned_utf8_can_become_bytes_without_copy() {
        let s = String::from("🦀");
        let ptr = s.as_ptr();
        let bytes = into_packet(s);
        assert_eq!(bytes, "🦀".as_bytes());
        assert_eq!(
            bytes.as_ptr(),
            ptr,
            "into_packet must hand over the String's own buffer, not a copy"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        let bytes = vec![0x66, 0x80, 0x6f];
        // P1. How many bytes are valid before the error?
        let p1: usize = 1;
        assert_eq!(
            p1,
            String::from_utf8(bytes.clone())
                .unwrap_err()
                .utf8_error()
                .valid_up_to(),
            "P1: valid_up_to"
        );

        // P2. After lossy repair: (scalar count, byte length).
        let repaired = String::from_utf8_lossy(&bytes);
        let p2: (usize, usize) = (3, 5);
        assert_eq!(
            p2,
            (repaired.chars().count(), repaired.len()),
            "P2: lossy repair (scalars, bytes)"
        );

        // P3. Allocations made by String::into_bytes.
        let s = String::from("payload");
        let (raw, allocations) = count(|| s.into_bytes());
        let p3: usize = 0;
        assert_eq!(p3, allocations, "P3: into_bytes");
        assert_eq!(raw.len(), 7);

        // P4. A borrowed Cow after to_mut(): which variant is it now?
        let mut text: Cow<'_, str> = Cow::Borrowed("draft");
        text.to_mut().push('!');
        let p4: bool = true; // true if Owned
        assert_eq!(p4, matches!(text, Cow::Owned(_)), "P4: clone on write");
    }
}
