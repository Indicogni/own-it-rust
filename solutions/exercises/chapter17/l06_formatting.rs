//! LESSON 06 — The formatting language
//! Run: cargo test --test chapter17 l06_
//! Read first: "Lesson 06" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does field width truncate long text? What does precision mean
//! for a string versus a float? Is terminal column width equal to formatting width?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: How do named/indexed arguments and dynamic parameters make a
//! formatter reusable? Why should pointer addresses never be hard-coded in tests?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch17_scratch.rs):
//! Format integers in decimal/binary/octal/hex with #, signs, and zero padding; floats in
//! fixed/scientific notation; vectors with {:?} and {:#?}; references with {:p}. Use {1}
//! {0} and named arguments. Verify fmt width counts Unicode scalars rather than UTF-8 bytes
//! or terminal columns.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete when its tests, predictions, and explanation are done. The lab is optional.

/// A. Right-align label in width Unicode scalars, then ": ", then value with
/// precision digits after decimal. Inputs are finite nonnegative floats.
/// Rounding is whatever the standard formatter does; do not round by hand.
fn row(label: &str, value: f64, width: usize, precision: usize) -> String {
    format!("{label:>width$}: {value:.precision$}")
}
/// B. Format u16 as lowercase hex with 0x prefix and total minimum width 6,
/// zero-filled after prefix (e.g. 10 -> "0x000a").
fn hex_code(value: u16) -> String {
    // `#` adds "0x", and the width 6 includes those two characters.
    format!("{value:#06x}")
}
/// C. Exact one-line debug representation of a slice, not a custom serializer.
fn debug_values(values: &[i32]) -> String {
    format!("{values:?}")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_dynamic_parameters_control_alignment_and_rounding() {
        assert_eq!(
            row("air", 1.25, 5, 1),
            "  air: 1.2",
            "1.25 is stored exactly, so it is a true tie, and ties round to the even digit"
        );
        assert_eq!(row("é", 2.0, 3, 2), "  é: 2.00");
        assert_eq!(row("long", 0.0, 2, 0), "long: 0");
        assert_eq!(row("🦀", 9.999, 2, 2), " 🦀: 10.00");
        assert_eq!(row("", 3.0, 0, 3), ": 3.000");
    }
    #[test]
    fn b_numeric_flags_keep_prefix_before_padding() {
        assert_eq!(hex_code(10), "0x000a");
        assert_eq!(hex_code(u16::MAX), "0xffff");
        assert_eq!(hex_code(0), "0x0000");
    }
    #[test]
    fn c_debug_format_uses_standard_collection_syntax() {
        assert_eq!(debug_values(&[1, -2]), "[1, -2]");
        assert_eq!(debug_values(&[]), "[]");
        assert_eq!(debug_values(&[i32::MIN]), "[-2147483648]");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Does a width smaller than the text truncate it?
        let p1: String = "toolong".to_string();
        assert_eq!(
            p1,
            format!("{:>4}", "toolong"),
            "P1: width smaller than the text"
        );

        // P2. Precision on a string.
        let p2: String = "ab".to_string();
        assert_eq!(p2, format!("{:.2}", "abcdef"), "P2: precision on str");

        // P3. Sign with zero padding, and alternate binary.
        let p3: (String, String) = ("+0007".to_string(), "0b101".to_string());
        assert_eq!(
            p3,
            (format!("{:+05}", 7), format!("{:#b}", 5)),
            "P3: numeric flags"
        );

        // P4. Scientific notation.
        let p4: String = "1.2345e3".to_string();
        assert_eq!(p4, format!("{:e}", 1234.5), "P4: {{:e}}");

        // P5. Rounding with precision. 0.5, 1.5, 2.5 and 0.25 are stored exactly
        // in binary, so each one is a true tie between two neighbours.
        let p5: (String, String, String, String) = (
            "0".to_string(),
            "2".to_string(),
            "2".to_string(),
            "0.2".to_string(),
        );
        assert_eq!(
            p5,
            (
                format!("{:.0}", 0.5),
                format!("{:.0}", 1.5),
                format!("{:.0}", 2.5),
                format!("{:.1}", 0.25)
            ),
            "P5: rounding of exact ties"
        );
    }
}
