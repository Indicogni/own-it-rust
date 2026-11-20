//! LESSON 04 — Text iteration, trimming, and casing
//! Run: cargo test --test chapter17 l04_
//! Read first: "Lesson 04" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: How do lines, split, and split_terminator differ at a final
//! delimiter? Does trim return an owned string?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why is lowercasing not a universal Unicode case-insensitive
//! comparison? Why do chars and bytes answer different questions?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch17_scratch.rs):
//! Compare lines(), split('\n'), split_terminator('\n'), split_whitespace(), and
//! split_ascii_whitespace() on "a\r\nb\n" and Unicode whitespace. Compare trim,
//! trim_start_matches, and trim_end_matches. Observe lowercase Greek sigma in whole strings
//! versus per-char conversion.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete when its tests, predictions, and explanation are done. The lab is optional.

/// A. Return trimmed nonblank borrowed lines. lines() handles LF/CRLF.
fn clean_lines(text: &str) -> Vec<&str> {
    // `lines` removes "\n" and "\r\n"; `trim` returns a slice of the same text.
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}
/// B. Uppercase whole text, including one-to-many Unicode mappings.
fn shout(text: &str) -> String {
    text.to_uppercase()
}
/// C. Trim only ASCII '-' and '_' from both ends, keeping inner occurrences.
fn unframe(text: &str) -> &str {
    // An array of chars is a pattern that matches any one of them.
    text.trim_matches(['-', '_'])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_lines_and_trim_borrow_valid_utf8() {
        assert_eq!(clean_lines(" a\r\n \r\n\u{2003}é \n"), ["a", "é"]);
        assert!(clean_lines("").is_empty());
        assert_eq!(clean_lines("x\n\n\ny"), ["x", "y"]);
        assert_eq!(clean_lines("  inner  space  "), ["inner  space"]);
    }
    #[test]
    fn b_whole_string_case_conversion_can_expand() {
        assert_eq!(shout("Straße é"), "STRASSE É");
        assert_eq!(shout(""), "");
        assert_eq!(
            shout("ﬁx 🦀"),
            "FIX 🦀",
            "the ﬁ ligature expands to two letters"
        );
    }
    #[test]
    fn c_pattern_trimming_preserves_interior_and_spaces() {
        assert_eq!(unframe("__a-b__"), "a-b");
        assert_eq!(unframe(" _a_ "), " _a_ ");
        assert_eq!(unframe("---"), "");
        assert_eq!(unframe("-_a_-"), "a", "both characters at both ends");
        assert_eq!(unframe("—a—"), "—a—", "an em dash is not ASCII '-'");
        assert_eq!(unframe("_é_"), "é");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Counting pieces of "a\nb\n": lines() versus split('\n').
        let p1: (usize, usize) = (2, 3);
        assert_eq!(
            p1,
            ("a\nb\n".lines().count(), "a\nb\n".split('\n').count()),
            "P1: a final delimiter"
        );

        // P2. split('\n') on CRLF text: what do the pieces look like?
        let p2: Vec<&str> = vec!["a\r", "b"];
        assert_eq!(
            p2,
            "a\r\nb".split('\n').collect::<Vec<_>>(),
            "P2: split on CRLF text"
        );

        // P3. Lowercasing a whole word versus one char at a time.
        let whole = "ΣΑΣ".to_lowercase();
        let per_char: String = "ΣΑΣ".chars().flat_map(char::to_lowercase).collect();
        let p3: (bool, usize) = (false, 6); // (equal?, byte length of `whole`)
        assert_eq!(
            p3,
            (whole == per_char, whole.len()),
            "P3: whole-string versus per-char lowercasing"
        );

        // P4. Byte length after trim_start on "  hi  ".
        let p4: usize = 4;
        assert_eq!(p4, "  hi  ".trim_start().len(), "P4: trim_start");
    }
}
