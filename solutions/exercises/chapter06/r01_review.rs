//! REVIEW 01 — Spaced retrieval: Chapters 3–5 inside Chapter 6
//! Run: cargo test --test chapter06 r01_
//!
//! Do this WITHOUT reopening Chapters 3–5 or their hints. Struggling to recall
//! is what makes the knowledge stick; look things up only after a real
//! attempt, and note in NOTES.md anything you had to look up.
//!
//! Each task mixes an earlier idea (borrowed lifetimes, moving out of
//! collections, integer overflow) with this chapter's expressions.

/// A. (Ch5 lifetimes + Ch6 let else) Lines look like "key=value". Return the
/// trimmed value of the FIRST line whose trimmed key equals `key`, borrowed
/// from that line. Skip lines without '=' using let else and continue.
/// Allocate nothing.
fn lookup<'a>(lines: &[&'a str], key: &str) -> Option<&'a str> {
    for &line in lines {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() == key {
            return Some(value.trim());
        }
    }
    None
}

/// B. (Ch4 Option::take + Ch6 loops) Walk `slots` in order and take every
/// message out, leaving None behind, until the first empty slot: break there
/// and leave later slots untouched. Return the taken messages in order.
/// Do not clone any String.
fn drain_until_gap(slots: &mut [Option<String>]) -> Vec<String> {
    let mut taken = Vec::new();
    for slot in slots.iter_mut() {
        // take() moves the String out through &mut and leaves None behind.
        match slot.take() {
            Some(message) => taken.push(message),
            None => break,
        }
    }
    taken
}

/// C. (Ch3 overflow + Ch6 casts) Return the mean of `values`, rounded toward
/// zero, or None when empty. The sum must not overflow: widen with `as` (or
/// i64::from) before adding, then cast the mean back.
fn mean(values: &[i32]) -> Option<i32> {
    if values.is_empty() {
        return None;
    }
    let sum: i64 = values.iter().map(|&value| i64::from(value)).sum();
    // The mean of i32 values always fits in an i32, so this cast loses nothing.
    Some((sum / values.len() as i64) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source;
    use practice::{alloc::count, within};
    use std::rc::Rc;

    #[test]
    fn a_lookup_borrows_from_the_matching_line() {
        // `within` fails the test with a message if your loop never ends.
        // Allocations are counted per thread, so `count` runs inside it.
        within(10, "the loop in lookup", || {
            let owned = String::from(" port = 8080 ");
            let lines = ["junk", "host=example", owned.as_str(), "port=1"];
            let (value, allocations) = count(|| lookup(&lines, "port"));
            assert_eq!(value, Some("8080"));
            assert_eq!(
                value.unwrap().as_ptr() as usize,
                owned.as_ptr() as usize + 8,
                "the value must be a slice of the matching line (8 bytes in), not a copy"
            );
            assert_eq!(allocations, 0, "lookup should only borrow");
            assert_eq!(lookup(&lines, "missing"), None);
            assert_eq!(lookup(&["a=1=2"], "a"), Some("1=2"));
            assert_eq!(lookup(&["港=é"], "港"), Some("é"));
            assert_eq!(lookup(&[], "a"), None);
        });
    }

    #[test]
    fn a_result_outlives_the_key() {
        let lines = ["name=ok"];
        let value = {
            let key = String::from("name");
            lookup(&lines, &key)
        };
        assert_eq!(value, Some("ok"));
    }

    #[test]
    fn a_source_uses_let_else_and_continue() {
        let body = source::body(include_str!("r01_review.rs"), "lookup");
        assert!(
            source::has_let_else(&body) && source::has_word(&body, "continue"),
            "task A asks for `let ... else {{ continue }}` inside lookup to skip lines \
             without '=' (this test reads your source)"
        );
    }

    #[test]
    fn b_drain_stops_at_the_first_gap() {
        within(10, "the loop in drain_until_gap", || {
            let mut slots = vec![
                Some("a".to_string()),
                Some("b".into()),
                None,
                Some("c".into()),
            ];
            let p = slots[0].as_ref().unwrap().as_ptr();
            let (taken, allocations) = count(|| drain_until_gap(&mut slots));
            assert_eq!(taken, ["a", "b"]);
            assert_eq!(taken[0].as_ptr(), p, "move the String out; do not clone it");
            assert!(
                allocations <= 1,
                "only the result vector may allocate: move each String out of its slot \
                 instead of cloning it"
            );
            assert_eq!(
                slots,
                [None, None, None, Some("c".into())],
                "taken slots become None; slots after the first gap stay untouched"
            );
            assert!(drain_until_gap(&mut slots).is_empty());
            let mut full = vec![Some("x".to_string())];
            assert_eq!(drain_until_gap(&mut full), ["x"]);
            assert_eq!(full, [None]);
        });
    }

    #[test]
    fn c_mean_does_not_overflow() {
        assert_eq!(mean(&[1, 2, 3, 4]), Some(2));
        assert_eq!(mean(&[-3, -4]), Some(-3));
        assert_eq!(mean(&[i32::MAX, i32::MAX]), Some(i32::MAX));
        assert_eq!(mean(&[i32::MIN, i32::MIN, i32::MIN]), Some(i32::MIN));
        assert_eq!(mean(&[i32::MIN, i32::MAX]), Some(0));
        assert_eq!(mean(&[]), None);
    }

    /// P. Chapters 3–5 recall: predict each value BEFORE running.
    #[test]
    fn p_predictions() {
        let p1: (u8, Option<i8>, i32) = (144, None, 2);
        assert_eq!(
            p1,
            (
                200u8.wrapping_mul(2),
                i8::MIN.checked_abs(),
                (-7i32).rem_euclid(3)
            ),
            "P1: Ch3 integer methods"
        );

        let words = size_of::<usize>();
        let p2: (usize, usize, usize) = (2, 1, 1);
        assert_eq!(
            p2,
            (
                size_of::<&str>() / words,
                size_of::<&[u8; 4]>() / words,
                size_of::<Option<&u8>>() / words
            ),
            "P2: Ch5 reference sizes, in machine words"
        );

        let a = Rc::new(String::from("shared"));
        let b = Rc::clone(&a);
        {
            let c = Rc::clone(&b);
            assert_eq!(c.len(), 6);
        }
        let p3: usize = 2;
        assert_eq!(
            p3,
            Rc::strong_count(&a),
            "P3: Ch4 strong count after a scope"
        );
    }
}
