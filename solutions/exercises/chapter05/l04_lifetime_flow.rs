//! LESSON 04 — Lifetimes through functions
//! Run: cargo test --test chapter05 l04_
//! Read first: CONCEPTS.md, section "Lesson 04 · Lifetimes through functions".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does a lifetime annotation keep an owner alive? Can a function return a reference to its local String?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Describe the input-to-output lifetime relationship and why it does not allocate or extend any lifetime.
//! COMPILER / TRANSFER LAB (optional; tests do not check it): Write a function returning a reference to a local integer; inspect the diagnostic. Repair by taking a borrowed input, then by returning an owned integer. Add an unnecessary static bound and observe which calls it forbids.
//! Use examples/ch05_scratch.rs; repair intentional failures afterward.

/// A. Split at the FIRST colon; return two borrowed halves without the colon.
/// Return None when absent; empty halves are valid.
fn split_record(line: &str) -> Option<(&str, &str)> {
    line.split_once(':')
}
/// B. Return a reference to the first greatest integer; None for empty input.
/// The signature writes out the lifetime that the compiler would otherwise fill
/// in: the result borrows from `values`, so it is valid as long as `values` is.
// Clippy would shorten this to `fn greatest(values: &[i32]) -> Option<&i32>`.
// The long form stays so that you can read the relationship.
#[allow(clippy::needless_lifetimes)]
fn greatest<'a>(values: &'a [i32]) -> Option<&'a i32> {
    // `Iterator::max` returns the LAST of several equal maxima, so scan by
    // hand and replace the candidate only for a strictly greater value.
    let mut best = values.first()?;
    for value in &values[1..] {
        if value > best {
            best = value;
        }
    }
    Some(best)
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_halves_borrow_the_original() {
        let s = String::from("港:ok:later");
        let (a, b) = split_record(&s).unwrap();
        assert_eq!((a, b), ("港", "ok:later"));
        assert_eq!(
            a.as_ptr(),
            s.as_ptr(),
            "the first half must point into the original line, not into a new String"
        );
        assert_eq!(split_record(":"), Some(("", "")));
        assert_eq!(split_record("none"), None);
    }
    #[test]
    fn b_result_borrows_first_maximum() {
        let a = [-2, 8, 8, 1];
        assert!(
            std::ptr::eq(greatest(&a).unwrap(), &a[1]),
            "with a tie, return a reference to the FIRST greatest element (index 1 here)"
        );
        assert_eq!(greatest(&[]), None);
        assert_eq!(greatest(&[-3]), Some(&-3));
        assert_eq!(greatest(&[-5, -2, -9]), Some(&-2));
    }

    #[test]
    fn a_b_results_borrow_without_allocating() {
        let line = String::from("key:value");
        let (halves, allocations) = count(|| split_record(&line));
        assert_eq!(halves, Some(("key", "value")));
        assert_eq!(allocations, 0, "split_record should return borrows");
        let values = [4, 7, 1];
        let (best, allocations) = count(|| greatest(&values));
        assert!(
            std::ptr::eq(best.unwrap(), &values[1]),
            "greatest must return a reference into the slice, not to a copy"
        );
        assert_eq!(allocations, 0, "greatest should scan the slice in place");
    }

    fn first_word(text: &str) -> &str {
        text.split(' ').next().unwrap()
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. rsplit_once searches from the END.
        let p1: Option<(&str, &str)> = Some(("a:b", "c"));
        assert_eq!(p1, "a:b:c".rsplit_once(':'), "P1: rsplit_once");

        // P2. Does the returned &str point into the argument? And what is
        // the first word of a line that starts with a space?
        let text = String::from("hi there");
        let p2: (bool, usize) = (true, 0);
        assert_eq!(
            p2,
            (
                first_word(&text).as_ptr() == text.as_ptr(),
                first_word(" lead").len()
            ),
            "P2: a borrowed result is a window into the input"
        );

        // P3. trim returns a subslice: its byte offset into `padded`, and its length.
        let padded = "  pad  ";
        let trimmed = padded.trim();
        let p3: (usize, usize) = (2, 3);
        assert_eq!(
            p3,
            (
                trimmed.as_ptr() as usize - padded.as_ptr() as usize,
                trimmed.len()
            ),
            "P3: trim borrows the middle"
        );
    }
}
