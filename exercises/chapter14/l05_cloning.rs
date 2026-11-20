//! LESSON 05 — Copy and Clone for captures
//! Run: cargo test --test chapter14 l05_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: If a closure captures only an i32 by move,
//! can the closure be Copy? If it owns a Vec, can cloning create independent state?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: How do the capabilities of captured fields determine a closure’s
//! Clone and Copy implementations?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch14_scratch.rs):
//! Write helpers requiring F: Copy and F: Clone. Pass closures capturing an i32, String,
//! &String, and &mut String. Predict each result before checking. Explain why copying a
//! shared reference does not clone the value it points to.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete when its tests, predictions, and explanation are done. The lab is optional.

/// A1. Return a closure that adds `offset` to its argument. The return type promises
/// `Copy`, so the closure may capture only `Copy` values, by value.
/// Inputs are bounded so addition fits.
fn add_offset(offset: i32) -> impl Fn(i32) -> i32 + Copy {
    // Placeholder so the starter compiles: replace this closure with your own.
    |_input: i32| -> i32 { todo!("Task 1: implement the documented contract") }
}
/// A2. `step` is a stateful closure that is `Copy`. Save a copy of it in its starting
/// state, call the original twice, then call the saved copy once.
/// Return (the original's second result, the saved copy's result).
fn replay_from_start<F: FnMut() -> u32 + Copy>(mut step: F) -> (u32, u32) {
    todo!("Task 2: implement the documented contract")
}
/// B1. Return a closure that owns `seed`. Each call appends its argument to the owned
/// vector and returns a snapshot (a clone) of the whole vector. The return type
/// promises `Clone`, so a clone of the closure gets its own vector.
fn appender(mut seed: Vec<i32>) -> impl FnMut(i32) -> Vec<i32> + Clone {
    // Placeholder so the starter compiles: replace this closure with your own.
    |_value: i32| -> Vec<i32> { todo!("Task 3: implement the documented contract") }
}
/// B2. Clone `append` before calling anything. Call the original with 1 and the clone
/// with 2. Return (the original's result, the clone's result).
/// Neither output may contain the other closure's edit.
fn fork<F: FnMut(i32) -> Vec<i32> + Clone>(mut append: F) -> (Vec<i32>, Vec<i32>) {
    todo!("Task 4: implement the documented contract")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;
    use std::mem::size_of_val;
    #[test]
    fn a1_scalar_capture_makes_copy_possible() {
        let add = add_offset(7);
        let copy = add; // compiles only because the closure is Copy
        assert_eq!((add(3), copy(3)), (10, 10));
        assert_eq!(add_offset(-4)(0), -4);
        assert_eq!(add_offset(0)(-9), -9);
        assert_eq!(
            size_of_val(&add),
            size_of::<i32>(),
            "the closure's only state should be the copied i32"
        );
    }
    #[test]
    fn a2_a_copy_keeps_the_state_it_was_copied_with() {
        let mut n = 10;
        let counter = move || {
            n += 1;
            n
        };
        assert_eq!(
            replay_from_start(counter),
            (12, 11),
            "(original's second result, saved copy's result): the copy must be taken \
             before the first call, so it still starts from 10"
        );
        let mut power = 1;
        assert_eq!(
            replay_from_start(move || {
                power *= 2;
                power
            }),
            (4, 2),
            "(original's second result, saved copy's result)"
        );
    }
    #[test]
    fn b1_the_closure_owns_and_grows_its_vector() {
        let mut push = appender(vec![9]);
        assert_eq!(push(1), [9, 1]);
        assert_eq!(
            push(2),
            [9, 1, 2],
            "the closure keeps its state between calls"
        );
        let mut twin = push.clone();
        assert_eq!(twin(7), [9, 1, 2, 7]);
        assert_eq!(
            push(3),
            [9, 1, 2, 3],
            "a clone of the closure must own a separate vector"
        );
        assert_eq!(appender(vec![])(5), [5]);
    }
    #[test]
    fn b2_cloned_owned_captures_are_independent() {
        // A stateful closure that is not an appender: it owns a running total
        // and returns it in a one-element vector.
        let running = |mut total: i32| {
            move |value| {
                total += value;
                vec![total]
            }
        };
        assert_eq!(
            fork(running(9)),
            (vec![10], vec![11]),
            "(original's result, clone's result): clone before the first call, so \
             both closures start from 9"
        );
        assert_eq!(fork(running(0)), (vec![1], vec![2]));
        assert_eq!(
            fork(appender(vec![5, 6])),
            (vec![5, 6, 1], vec![5, 6, 2]),
            "fork must also work with your own appender"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. This FnMut closure owns a copied i32, so the closure itself is Copy.
        // (f's second result, g's first result, hits afterwards)?
        let mut hits = 0;
        let mut f = move || {
            hits += 1;
            hits
        };
        let mut g = f;
        f();
        let second = f();
        let p1: (i32, i32, i32) = guess();
        assert_eq!(
            p1,
            (second, g(), hits),
            "P1: (f's second result, g's first result, hits afterwards); copying a \
             closure copies its state"
        );

        // P2. Cloning a closure that owns a Vec: does the clone share the Vec's buffer?
        let v = Vec::from([1, 2, 3]);
        let owner = move || v.as_ptr();
        let twin = owner.clone();
        let p2: bool = guess(); // true if both closures see the same buffer
        assert_eq!(p2, owner() == twin(), "P2: cloning clones owned captures");

        // P3. A closure that borrows a String is Copy. Do the copies see one buffer?
        let s = String::from("shared");
        let reader = || s.as_ptr();
        let copy = reader;
        let p3: bool = guess(); // true if both closures see the same buffer
        assert_eq!(
            p3,
            reader() == copy(),
            "P3: copying a borrow copies the pointer"
        );
    }
}
