//! LESSON 02 — Generating finite sequences
//! Run: cargo test --test chapter15 l02_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: When does a from_fn closure execute? Does
//! successors yield its first value even if the step returns None for it?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: How do you guarantee termination and avoid overflow at the final step?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch15_scratch.rs):
//! Generate 5 values with std::iter::repeat_n and once().chain(...). Create an
//! intentionally infinite repeat/cycle and demonstrate that take bounds the work; never
//! collect an unbounded iterator.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, and explanation are done.

/// A. Lazily yield the powers of two that are <= limit, starting at 1; limit = 0
/// yields nothing. The sequence must end by itself for every limit, including
/// u32::MAX, where one more doubling would overflow. Do not build a Vec.
/// Suggested tools: `successors` with `checked_mul`, then `take_while`.
#[allow(unreachable_code)]
fn powers(limit: u32) -> impl Iterator<Item = u32> {
    // Replace both lines. The second one only gives the unfinished function an
    // iterator type, so that the file compiles.
    todo!("Task 1: implement the documented contract");
    std::iter::empty()
}
/// B. Use from_fn to yield start,start-1,...,0 exactly once each.
fn countdown(start: u8) -> impl Iterator<Item = u8> {
    let mut current = Some(start);
    std::iter::from_fn(move || -> Option<u8> { todo!("Task 2: implement the documented contract") })
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, guess};
    use std::cell::Cell;
    #[test]
    fn a_successors_stops_at_limit_or_overflow() {
        // `take(40)` bounds the test: a sequence that never ends fails an
        // assertion below instead of filling memory.
        let first = |limit: u32| powers(limit).take(40).collect::<Vec<u32>>();
        assert!(first(0).is_empty(), "no power of two is <= 0");
        assert_eq!(first(10), [1, 2, 4, 8]);
        assert_eq!(first(1), [1]);
        assert_eq!(first(3), [1, 2]);
        let all = first(u32::MAX);
        assert_eq!(
            all.len(),
            32,
            "u32 holds 32 powers of two; the sequence must end when doubling would \
             overflow, not wrap around to 0 and continue"
        );
        assert_eq!(all.last(), Some(&(1 << 31)));
    }
    #[test]
    fn a_powers_are_generated_on_demand() {
        let (n, allocations) = count(|| powers(u32::MAX).take(40).count());
        assert_eq!(n, 32);
        assert_eq!(
            allocations, 0,
            "powers made {allocations} heap allocation(s); return the lazy iterator \
             itself, not a Vec turned back into an iterator"
        );
    }
    #[test]
    fn b_from_fn_includes_zero_and_stays_exhausted() {
        // `take` bounds each check, so a countdown that never ends cannot hang.
        assert_eq!(
            countdown(3).take(50).collect::<Vec<_>>(),
            [3, 2, 1, 0],
            "countdown(3) yields 3, 2, 1, 0 and then ends"
        );
        let mut i = countdown(0);
        assert_eq!(i.next(), Some(0), "zero itself is yielded once");
        assert_eq!(i.next(), None);
        assert_eq!(i.next(), None, "after None the countdown stays finished");
        assert_eq!(
            countdown(u8::MAX).take(1000).count(),
            256,
            "255 down to 0 is 256 values"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Calls to the from_fn closure: (before anything consumes, after take(3)).
        let calls = Cell::new(0);
        let mut naturals = std::iter::from_fn(|| {
            calls.set(calls.get() + 1);
            Some(calls.get())
        });
        let before = calls.get();
        let first: Vec<u32> = naturals.by_ref().take(3).collect();
        let p1: (u32, u32) = guess();
        assert_eq!(
            p1,
            (before, calls.get()),
            "P1: closure calls (before consuming, after take(3))"
        );
        assert_eq!(first, [1, 2, 3]);

        // P2. Powers of 3 in u8, stopping at overflow: (how many, last value).
        let powers: Vec<u8> = std::iter::successors(Some(1u8), |&x| x.checked_mul(3)).collect();
        let p2: (usize, u8) = guess();
        assert_eq!(
            p2,
            (powers.len(), powers[powers.len() - 1]),
            "P2: (how many powers of 3 fit in u8, the last one)"
        );

        // P3. Calls to the successors step: (right after creation, after the whole
        // sequence has been collected). When exactly the step runs between those
        // two moments is a detail of the standard library, so it is not asked.
        let steps = Cell::new(0);
        let falling = std::iter::successors(Some(10u8), |&x| {
            steps.set(steps.get() + 1);
            x.checked_sub(4)
        });
        let created = steps.get();
        let values: Vec<u8> = falling.collect();
        let p3: (u32, u32, usize) = guess();
        assert_eq!(
            p3,
            (created, steps.get(), values.len()),
            "P3: (step calls after creation, step calls after collecting, items collected)"
        );
    }
}
