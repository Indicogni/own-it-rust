//! LESSON 02 — Catching Result errors and printing context
//! Run: cargo test --test chapter07 l02_
//!
//! Read first: CONCEPTS.md, section "Lesson 02".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Does matching an Err require unwinding?
//! 2. What is lost by replacing every error with zero?
//!
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain recovery policy and how a user-facing message
//! differs from the underlying parse error.
//!
//! OPTIONAL LAB: LABS.md, lab 3 (Display and Debug of an error, and stderr).

/// A. Parse a trimmed u16; return the fallback on any parse error. Write it
/// with an explicit `match` on the Result: that is what this lesson
/// practises. (The tests cannot see which form you used; `unwrap_or` passes
/// too, and lesson 04 is where you practise the shorter methods.)
fn parse_or(text: &str, fallback: u16) -> u16 {
    // Written as a match on purpose; `unwrap_or(fallback)` is the short form.
    match text.trim().parse::<u16>() {
        Ok(value) => value,
        Err(_) => fallback,
    }
}
/// B. Render either "NAME=VALUE" or "NAME: invalid count". This message is
/// deliberately stable: it must not contain ParseIntError's Display text,
/// because the standard library does not promise to keep that wording.
fn describe(name: &str, result: Result<u16, std::num::ParseIntError>) -> String {
    match result {
        Ok(value) => format!("{name}={value}"),
        // The parse error is dropped on purpose: this message is the stable
        // text that users see.
        Err(_) => format!("{name}: invalid count"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::num::IntErrorKind;
    #[test]
    fn a_fallback_is_an_explicit_policy() {
        assert_eq!(parse_or(" 12 ", 7), 12);
        assert_eq!(parse_or("65536", 7), 7);
        assert_eq!(parse_or("", 8), 8);
        assert_eq!(parse_or("-1", 9), 9);
        assert_eq!(parse_or("65535", 0), 65535);
        assert_eq!(parse_or("\t0\n", 5), 0, "zero is a value, not a failure");
    }
    #[test]
    fn b_context_survives_error_rendering() {
        assert_eq!(describe("jobs", "4".parse()), "jobs=4");
        assert_eq!(describe("jobs", "no".parse()), "jobs: invalid count");
        assert_eq!(describe("港", "70000".parse()), "港: invalid count");
        assert_eq!(describe("empty", "".parse()), "empty: invalid count");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Which of these does `parse::<u8>()` accept? (No trimming happens.)
        let p1: (bool, bool, bool) = (false, true, false);
        assert_eq!(
            p1,
            (
                " 42".parse::<u8>().is_ok(),
                "+42".parse::<u8>().is_ok(),
                "-0".parse::<u8>().is_ok()
            ),
            "P1: what parse accepts"
        );

        // P2. The error kinds for "256" and "" as u8.
        let p2: (IntErrorKind, IntErrorKind) = (IntErrorKind::PosOverflow, IntErrorKind::Empty);
        assert_eq!(
            p2,
            (
                *"256".parse::<u8>().unwrap_err().kind(),
                *"".parse::<u8>().unwrap_err().kind()
            ),
            "P2: ParseIntError carries a kind"
        );

        // P3. Float and integer parsing of "1e3" and "3.0".
        let p3: (Option<f64>, Option<i32>) = (Some(1000.0), None);
        assert_eq!(
            p3,
            ("1e3".parse::<f64>().ok(), "3.0".parse::<i32>().ok()),
            "P3: each type has its own grammar"
        );
    }
}
