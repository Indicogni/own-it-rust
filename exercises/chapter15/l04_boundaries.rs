//! LESSON 04 — Stopping, peeking, and resuming
//! Run: cargo test --test chapter15 l04_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does take_while preserve the first rejected
//! item? Does Peekable::peek consume it? Is every iterator permanently exhausted after
//! None?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why does by_ref permit continued use of the original iterator? What
//! guarantee does fuse add?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch15_scratch.rs):
//! Use take_while on [1,2,0,3] by_ref and inspect the remaining items. Write an iterator
//! yielding Some(1), None, Some(2), then compare it with fuse().
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, and explanation are done.

use std::iter::Peekable;

/// A. Skip leading zeros, then keep consecutive nonzero values until the next zero.
fn active_region(values: &[i32]) -> Vec<i32> {
    todo!("Task 1: implement the documented contract")
}
/// B. Remove the leading ASCII digits from `chars` and return them as a String.
/// The first character that is not an ASCII digit must stay in `chars`, so the
/// caller can still read it. An empty result is valid. Use `peek` or `next_if`.
fn take_digits<I: Iterator<Item = char>>(chars: &mut Peekable<I>) -> String {
    todo!("Task 2: implement the documented contract")
}
/// C. Consume the first two items with by_ref().take(2), then collect the remainder.
fn split_two<I: Iterator<Item = i32>>(mut input: I) -> (Vec<i32>, Vec<i32>) {
    todo!("Task 3: implement the documented contract")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{guess, within};

    /// Names the task in the message that `within` prints after a timeout.
    const DIGITS: &str = "take_digits, which must advance the iterator in every loop turn,";

    /// Runs `take_digits` on the characters of `text` and returns the digits
    /// together with everything that is still in the iterator afterwards.
    /// `within` turns a loop that peeks but never advances into a test failure.
    fn split_digits(text: &'static str) -> (String, String) {
        within(2, DIGITS, move || {
            let mut chars = text.chars().peekable();
            let digits = take_digits(&mut chars);
            (digits, chars.collect())
        })
    }
    #[test]
    fn a_skip_and_take_while_have_distinct_boundaries() {
        assert_eq!(active_region(&[0, 0, 3, 4, 0, 5]), [3, 4]);
        assert!(active_region(&[0, 0]).is_empty());
        assert_eq!(active_region(&[2, 3]), [2, 3]);
        assert_eq!(active_region(&[0, 1, 0, 0, 2]), [1]);
        assert!(active_region(&[]).is_empty());
    }
    #[test]
    fn b_peek_keeps_the_delimiter() {
        assert_eq!(
            split_digits("12🦀3"),
            ("12".into(), "🦀3".into()),
            "the first non-digit must still be in the iterator after take_digits returns"
        );
        assert_eq!(split_digits("x9"), ("".into(), "x9".into()));
        assert_eq!(split_digits(""), ("".into(), "".into()));
        assert_eq!(
            split_digits("4٣"),
            ("4".into(), "٣".into()),
            "only ASCII digits count; ٣ is a digit in another script"
        );
        assert_eq!(split_digits("007"), ("007".into(), "".into()));
    }
    #[test]
    fn b_take_digits_can_be_called_again_on_the_same_iterator() {
        let (first, sign, second, rest) = within(2, DIGITS, || {
            let mut chars = "12+345=".chars().peekable();
            let first = take_digits(&mut chars);
            let sign = chars.next();
            let second = take_digits(&mut chars);
            (first, sign, second, chars.collect::<String>())
        });
        assert_eq!(
            (first.as_str(), sign, second.as_str(), rest.as_str()),
            ("12", Some('+'), "345", "="),
            "each call must stop in front of the delimiter, without swallowing it"
        );
    }
    #[test]
    fn c_by_ref_borrows_instead_of_consuming_the_iterator() {
        assert_eq!(
            split_two(1..6),
            (vec![1, 2], vec![3, 4, 5]),
            "the remainder starts right after the two items that were taken"
        );
        assert_eq!(split_two(std::iter::once(9)), (vec![9], vec![]));
    }
    #[test]
    fn c_fuse_makes_a_resuming_source_stay_exhausted() {
        let mut n = 0;
        let source = std::iter::from_fn(move || {
            n += 1;
            match n {
                1 => Some(1),
                2 => None,
                3 => Some(2),
                _ => None,
            }
        });
        let mut fused = source.fuse();
        assert_eq!(
            split_two(fused.by_ref()),
            (vec![1], vec![]),
            "a fused source that returned None once has nothing more to give"
        );
        assert_eq!(fused.next(), None);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. take_while through by_ref: (what it collects, what next() returns after).
        let mut it = [1, 2, 0, 3].into_iter();
        let head: Vec<i32> = it.by_ref().take_while(|&n| n != 0).collect();
        let p1: (Vec<i32>, Option<i32>) = guess();
        assert_eq!(
            p1,
            (head, it.next()),
            "P1: (items collected by take_while, the next item of the original iterator)"
        );

        // P2. How many items survive skip_while(is zero)?
        let survivors = [0, 1, 0, 2].iter().skip_while(|&&n| n == 0).count();
        let p2: usize = guess();
        assert_eq!(p2, survivors, "P2: items left after skip_while");

        // P3. A source that is not fused: three calls to next().
        let mut n = 0;
        let mut flaky = std::iter::from_fn(|| {
            n += 1;
            (n != 2).then_some(n)
        });
        let calls = (flaky.next(), flaky.next(), flaky.next());
        let p3: (Option<i32>, Option<i32>, Option<i32>) = guess();
        assert_eq!(
            p3, calls,
            "P3: three calls to next() on a source that is not fused"
        );
    }
}
