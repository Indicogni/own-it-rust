//! REVIEW 01 — Spaced retrieval: Chapter 3 inside Chapter 4
//! Run: cargo test --test chapter04 r01_
//!
//! Do this WITHOUT reopening Chapter 3 or its hints. Struggling to recall is
//! what makes the knowledge stick; look things up only after a real attempt,
//! and note in NOTES.md anything you had to look up.
//!
//! Each task mixes an earlier idea (integer overflow, UTF-8, slices) with this
//! chapter's ownership rules.

/// A. (Ch3 overflow + Ch4 moves) Consume the readings and return them with
/// their total. The total must not wrap: return None if it does not fit in u16.
/// Do not clone or copy the vector's buffer.
fn with_total(readings: Vec<u8>) -> (Vec<u8>, Option<u16>) {
    todo!("A: borrow to sum, then move the vector back out")
}

/// B. (Ch3 UTF-8 + Ch4 ownership) Return the first `n` Unicode scalar values
/// (`char`s) of `text` as a new String (all of it if shorter). Never split one.
fn prefix_scalars(text: &str, n: usize) -> String {
    todo!("B: walk chars, not bytes")
}

/// C. (Ch3 slices + Ch4 indexed moves) Move the last `count` labels out of
/// `labels` into a new vector, preserving order. If fewer exist, move them all.
/// Do not clone any String.
fn split_off_tail(labels: &mut Vec<String>, count: usize) -> Vec<String> {
    todo!("C: compute the split point with saturating arithmetic")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, guess};

    #[test]
    fn a_total_is_checked_and_vector_is_not_copied() {
        let v = vec![200, 100, 7];
        let p = v.as_ptr();
        let (back, total) = with_total(v);
        assert_eq!(total, Some(307));
        assert_eq!(
            back.as_ptr(),
            p,
            "with_total must return the vector it was given (same heap buffer), not a copy"
        );
        assert_eq!(with_total(vec![255; 257]).1, Some(65_535));
        assert_eq!(with_total(vec![255; 258]).1, None);
        assert_eq!(with_total(vec![]), (vec![], Some(0)));
    }

    #[test]
    fn b_prefix_respects_scalar_boundaries() {
        assert_eq!(prefix_scalars("ñ🦀rust", 2), "ñ🦀");
        assert_eq!(prefix_scalars("abc", 10), "abc");
        assert_eq!(prefix_scalars("abc", 0), "");
        assert_eq!(prefix_scalars("e\u{301}x", 1), "e");
    }

    #[test]
    fn c_tail_moves_without_cloning() {
        let mut labels: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        let p = labels[1].as_ptr();
        let (tail, allocations) = count(|| split_off_tail(&mut labels, 2));
        assert_eq!(
            (labels.as_slice(), tail.as_slice()),
            (
                &["a".to_string()][..],
                &["b".to_string(), "c".to_string()][..]
            )
        );
        assert_eq!(
            tail[0].as_ptr(),
            p,
            "the tail must hold the original \"b\" String (same heap buffer), not a copy"
        );
        assert!(
            allocations <= 1,
            "split_off_tail made {allocations} heap allocations; only the new vector may allocate, \
             so more than 1 means Strings were cloned"
        );
        assert_eq!(split_off_tail(&mut labels, 9), ["a"]);
        assert!(labels.is_empty());
        assert!(split_off_tail(&mut labels, 1).is_empty());
    }

    /// P. Chapter 3 recall: predict each value BEFORE running.
    #[test]
    fn p_predictions() {
        // P1. 250u8 plus 10 with wrapping_add, checked_add and saturating_add.
        let p1: (u8, Option<u8>, u8) = guess();
        assert_eq!(
            p1,
            (
                250u8.wrapping_add(10),
                250u8.checked_add(10),
                250u8.saturating_add(10)
            ),
            "P1: overflow policies (left: your prediction, right: actual)"
        );
        // P2. The values of `-7 / 2` and `-7 % 2` for i32.
        let p2: (i32, i32) = guess();
        assert_eq!(
            p2,
            (-7 / 2, -7 % 2),
            "P2: signed division and remainder (left: your prediction, right: actual)"
        );
        // P3. `"née".len()` and `"née".chars().count()`.
        let p3: (usize, usize) = guess();
        assert_eq!(
            p3,
            ("née".len(), "née".chars().count()),
            "P3: len and chars().count() (left: your prediction, right: actual)"
        );
        // P4. Is `0.1 + 0.2 == 0.3` true for f64?
        let p4: bool = guess();
        assert_eq!(
            p4,
            0.1 + 0.2 == 0.3,
            "P4: float equality (left: your prediction, right: actual)"
        );
    }
}
