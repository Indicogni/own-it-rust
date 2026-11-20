//! LESSON 04 — IntoIter and Drain: panic safety and leak safety
//! Run: cargo test --manifest-path mastery/m2_unsafe/Cargo.toml l04_
//! Miri: cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml l04_
//! Read first: "Lesson 04" in CONCEPTS.md (one directory up).
//! Needs: your `MyVec` from lesson 03. Use only its public functions
//! (`into_raw_parts`, `from_raw_parts`, `as_mut_ptr`, `set_len`, `len`).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `mem::forget` is a safe function. What does that mean for a type whose
//!    destructor repairs a data structure?
//! 2. A destructor panics while a `Drain` is dropping the rest of its range.
//!    Who moves the tail of the vector back?
//!
//! IMPLEMENT the tasks below. The type names and trait impls are given; the
//! fields, the invariants and any helper types are yours to design. Write the
//! invariant of each type in its doc comment first. Keep the supplied tests
//! and add one edge case.
//! EXPLAIN afterward: For `Drain`, say what the vector looks like (a) while the
//! drain is alive, (b) after the drain was dropped, (c) after the drain was
//! leaked with `mem::forget`. Why is (c) safe, and what does it cost?
//!
//! MIRI LAB: `examples/miri_l04_drain.rs` is a drain that works in every normal
//! use. Predict what Miri reports for the one unusual use in `main`, run
//! `cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l04_drain`,
//! compare with CONCEPTS.md, then repair the example until Miri is silent.

use crate::l03_vec::MyVec;
use std::marker::PhantomData;
use std::ops::Range;
use std::ptr::{self, NonNull};

/// A. An iterator that owns the elements of a `MyVec<T>` and its block.
///
/// Invariant: TODO. Say who owns the block and which slots hold values.
pub struct IntoIter<T> {
    // Design the fields. This placeholder only lets the starter compile.
    _todo: PhantomData<T>,
}

impl<T> IntoIterator for MyVec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    /// A. Takes over the block. Does not allocate and does not move any element.
    fn into_iter(self) -> IntoIter<T> {
        todo!("Implement this task")
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        todo!("Implement this task")
    }

    /// Both numbers are the exact count of elements left.
    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!("Implement this task")
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        todo!("Implement this task")
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}

impl<T> Drop for IntoIter<T> {
    /// A. Drops the elements that were not taken, front to back, and frees the
    /// block. If an element's destructor panics, the elements after it are
    /// still dropped and the block is still freed.
    fn drop(&mut self) {
        todo!("Implement this task")
    }
}

/// B. An iterator that moves a range of elements out of a `MyVec<T>` and
/// leaves the vector with the elements before and after the range.
///
/// Invariant: TODO. Say what the vector's length is while a `Drain` is alive,
/// and who owns the elements in the range and after it.
pub struct Drain<'a, T> {
    // Design the fields. This placeholder only lets the starter compile.
    _todo: PhantomData<&'a mut MyVec<T>>,
}

impl<T> MyVec<T> {
    /// B. Starts draining the elements in `range`. Panics with a message that
    /// contains `drain range` if `range.start > range.end` or
    /// `range.end > len`. Does not allocate.
    ///
    /// Leak safety: if the returned `Drain` is leaked with `mem::forget`, the
    /// vector must still be valid. It then holds exactly the elements before
    /// the range (its length is `range.start`); the range and the tail are
    /// lost. Other leak-safe choices exist, such as an empty vector. The tests
    /// ask for this one, which is what the pinned standard library does.
    pub fn drain(&mut self, range: Range<usize>) -> Drain<'_, T> {
        todo!("Implement this task")
    }
}

impl<T> Iterator for Drain<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        todo!("Implement this task")
    }

    /// Both numbers are the exact count of elements left in the range.
    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!("Implement this task")
    }
}

impl<T> DoubleEndedIterator for Drain<'_, T> {
    fn next_back(&mut self) -> Option<T> {
        todo!("Implement this task")
    }
}

impl<T> ExactSizeIterator for Drain<'_, T> {}

impl<T> Drop for Drain<'_, T> {
    /// B. Drops the elements of the range that were not taken, then moves the
    /// tail down so that the vector is contiguous again, and restores the
    /// length. If an element's destructor panics, the rest of the range is
    /// still dropped and the tail is still moved back.
    fn drop(&mut self) {
        todo!("Implement this task")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::audit;
    use std::cell::RefCell;
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

    /// Writes its name to a shared log when dropped. With `explode`, the
    /// destructor then unwinds, like a destructor that panics.
    struct Noisy<'a> {
        name: char,
        log: &'a RefCell<String>,
        explode: bool,
    }

    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.log.borrow_mut().push(self.name);
            if self.explode {
                // Unwinds like a panic, without printing a message.
                resume_unwind(Box::new("destructor failed"));
            }
        }
    }

    /// A log with room for every test's entries, so that writing to it inside
    /// an `audit` does not count as an allocation of the code under test.
    fn new_log() -> RefCell<String> {
        RefCell::new(String::with_capacity(64))
    }

    /// One element per letter. An upper-case letter makes an element whose
    /// destructor panics; its name in the log is the letter as written.
    fn noisy<'a>(names: &str, log: &'a RefCell<String>) -> MyVec<Noisy<'a>> {
        let mut vec = MyVec::new();
        for name in names.chars() {
            vec.push(Noisy {
                name,
                log,
                explode: name.is_ascii_uppercase(),
            });
        }
        vec
    }

    fn names(vec: &MyVec<Noisy<'_>>) -> String {
        vec.iter().map(|noisy| noisy.name).collect()
    }

    fn numbers(range: Range<u32>) -> MyVec<u32> {
        let mut vec = MyVec::new();
        for n in range {
            vec.push(n);
        }
        vec
    }

    #[test]
    fn a_into_iter_moves_each_element_out_in_order() {
        let mut vec = MyVec::new();
        for word in ["a", "b", "c"] {
            vec.push(word.to_string());
        }
        let text = vec[0].as_ptr();
        let ((iter, first), heap) = audit(|| {
            let mut iter = vec.into_iter();
            let first = iter.next();
            (iter, first)
        });
        assert_eq!(
            heap.allocations, 0,
            "into_iter must take over the block, not copy the elements"
        );
        let first = first.expect("three elements were pushed");
        assert!(
            std::ptr::eq(first.as_ptr(), text),
            "next must hand out the element itself (same text buffer), not a clone"
        );
        let rest: Vec<String> = iter.collect();
        assert_eq!(
            (first.as_str(), rest),
            ("a", vec!["b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn a_into_iter_works_from_both_ends_and_knows_its_length() {
        let mut iter = numbers(0..5).into_iter();
        assert_eq!(iter.len(), 5);
        assert_eq!(iter.size_hint(), (5, Some(5)));
        assert_eq!((iter.next(), iter.next_back()), (Some(0), Some(4)));
        assert_eq!(iter.len(), 3);
        assert_eq!((iter.next_back(), iter.next()), (Some(3), Some(1)));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.len(), 0);
        assert_eq!(
            (iter.next(), iter.next_back(), iter.next()),
            (None, None, None),
            "an empty iterator stays empty from both ends"
        );
        let mut total = 0;
        for n in numbers(1..5) {
            total += n;
        }
        assert_eq!(total, 10);
        assert_eq!(MyVec::<String>::new().into_iter().next(), None);
        assert!(numbers(0..4).into_iter().rev().eq([3, 2, 1, 0]));
    }

    #[test]
    fn a_dropping_the_iterator_drops_the_rest_once_and_frees_the_block() {
        let log = new_log();
        let ((), heap) = audit(|| {
            let mut iter = noisy("abcde", &log).into_iter();
            let (first, last) = (iter.next(), iter.next_back());
            drop(iter);
            assert_eq!(
                *log.borrow(),
                "bcd",
                "dropping the iterator drops the elements it still holds, front to back"
            );
            drop(last);
            drop(first);
        });
        assert_eq!(*log.borrow(), "bcdea");
        assert_eq!(heap.live, 0, "dropping the iterator must free the block");
        assert_eq!(
            heap.bad_frees, 0,
            "the block must be freed with the layout of the vector it came from"
        );
    }

    #[test]
    fn a_into_iter_frees_the_block_even_if_a_destructor_panics() {
        let log = new_log();
        let (panicked, heap) = audit(|| {
            let iter = noisy("aBc", &log).into_iter();
            catch_unwind(AssertUnwindSafe(move || drop(iter))).is_err()
        });
        assert!(panicked, "the destructor's panic must reach the caller");
        assert_eq!(
            *log.borrow(),
            "aBc",
            "the elements after the one that panicked must still be dropped, once"
        );
        assert_eq!(
            heap.live, 0,
            "the block must be freed even when an element's destructor panics"
        );
    }

    #[test]
    fn a_into_iter_handles_zero_sized_elements() {
        let mut vec = MyVec::new();
        for _ in 0..6 {
            vec.push(());
        }
        let ((front, back, rest), heap) = audit(|| {
            let mut iter = vec.into_iter();
            (iter.next(), iter.next_back(), iter.count())
        });
        assert_eq!((front, back, rest), (Some(()), Some(()), 4));
        assert_eq!(heap.allocations, 0);
        assert_eq!(
            heap.live, 0,
            "there is no block to free for zero-sized elements"
        );
    }

    #[test]
    fn b_drain_removes_the_range_and_closes_the_gap() {
        let mut vec = numbers(0..8);
        let capacity = vec.capacity();
        let first = vec.as_ptr();
        let (drained, heap) = audit(|| {
            let mut drain = vec.drain(2..5);
            assert_eq!(drain.len(), 3);
            [drain.next(), drain.next(), drain.next(), drain.next()]
        });
        assert_eq!(drained, [Some(2), Some(3), Some(4), None]);
        assert_eq!(heap.allocations, 0, "drain must work inside the block");
        assert_eq!(
            *vec,
            [0, 1, 5, 6, 7],
            "the tail must move down to close the gap"
        );
        assert_eq!(vec.capacity(), capacity);
        assert!(std::ptr::eq(vec.as_ptr(), first), "drain keeps the block");
        vec.push(8);
        assert_eq!(*vec, [0, 1, 5, 6, 7, 8]);
    }

    #[test]
    fn b_drain_handles_the_edges_of_the_vector() {
        let mut vec = numbers(0..5);
        assert_eq!(vec.drain(2..2).next(), None);
        assert_eq!(*vec, [0, 1, 2, 3, 4], "an empty range removes nothing");
        assert!(vec.drain(0..2).eq([0, 1]));
        assert_eq!(*vec, [2, 3, 4]);
        assert!(vec.drain(1..3).eq([3, 4]));
        assert_eq!(*vec, [2]);
        assert!(vec.drain(0..1).eq([2]));
        assert!(vec.is_empty());
        assert_eq!(vec.drain(0..0).len(), 0);
    }

    #[test]
    fn b_drain_works_from_both_ends() {
        let mut vec = numbers(0..6);
        let mut drain = vec.drain(1..5);
        assert_eq!((drain.next_back(), drain.next()), (Some(4), Some(1)));
        assert_eq!(drain.size_hint(), (2, Some(2)));
        assert_eq!((drain.next_back(), drain.next()), (Some(3), Some(2)));
        assert_eq!((drain.next_back(), drain.next()), (None, None));
        drop(drain);
        assert_eq!(*vec, [0, 5]);
    }

    #[test]
    fn b_dropping_a_drain_early_drops_the_rest_of_the_range() {
        let log = new_log();
        let mut vec = noisy("abcdef", &log);
        let mut drain = vec.drain(1..5);
        let taken = drain.next();
        drop(drain);
        assert_eq!(
            *log.borrow(),
            "cde",
            "dropping the drain drops the elements of the range that were not taken"
        );
        assert_eq!(names(&vec), "af");
        drop(taken);
        drop(vec);
        assert_eq!(
            *log.borrow(),
            "cdebaf",
            "each element is dropped exactly once"
        );
    }

    #[test]
    fn b_a_leaked_drain_leaves_a_valid_vector() {
        let log = new_log();
        let mut vec = noisy("abcde", &log);
        let mut drain = vec.drain(1..3);
        let taken = drain.next();
        std::mem::forget(drain);
        assert_eq!(
            names(&vec),
            "a",
            "after a leaked drain the vector must end where the range began: it may \
             lose elements, but it must never reach a slot whose value was moved out"
        );
        vec.push(Noisy {
            name: 'z',
            log: &log,
            explode: false,
        });
        assert_eq!(names(&vec), "az", "the vector must stay usable");
        drop(taken);
        drop(vec);
        assert_eq!(
            *log.borrow(),
            "baz",
            "the leaked elements are never dropped, and no element is dropped twice"
        );
    }

    #[test]
    fn b_drain_restores_the_tail_even_if_a_destructor_panics() {
        let log = new_log();
        let mut vec = noisy("aBcde", &log);
        let outcome = catch_unwind(AssertUnwindSafe(|| drop(vec.drain(1..3))));
        assert!(
            outcome.is_err(),
            "the destructor's panic must reach the caller"
        );
        assert_eq!(*log.borrow(), "Bc");
        assert_eq!(
            names(&vec),
            "ade",
            "the tail must be moved back even when a destructor panics"
        );
        drop(vec);
        assert_eq!(
            *log.borrow(),
            "Bcade",
            "each element is dropped exactly once"
        );
    }

    #[test]
    fn b_drain_handles_zero_sized_elements() {
        let mut vec = MyVec::new();
        for _ in 0..10 {
            vec.push(());
        }
        let (count, heap) = audit(|| vec.drain(2..6).count());
        assert_eq!(count, 4);
        assert_eq!(heap.allocations, 0);
        assert_eq!(vec.len(), 6);
    }

    #[test]
    #[should_panic(expected = "drain range")]
    fn b_drain_rejects_a_reversed_range() {
        #[allow(clippy::reversed_empty_ranges)]
        let _ = numbers(0..4).drain(3..1);
    }

    #[test]
    #[should_panic(expected = "drain range")]
    fn b_drain_rejects_a_range_past_the_end() {
        let _ = numbers(0..4).drain(2..5);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. The standard vector after a drain that is dropped at once.
        let mut vec = vec![1, 2, 3, 4, 5];
        drop(vec.drain(1..3));
        let p1: Vec<i32> = guess();
        assert_eq!(
            p1, vec,
            "P1: an unused drain still removes its range (left: your prediction, right: actual)"
        );

        // P2. The length of the standard vector after a drain that is leaked.
        // (The documentation only promises that the vector stays valid and
        // "may have lost and leaked elements arbitrarily". Predict what the
        // pinned standard library does.)
        let mut vec = vec![1, 2, 3, 4, 5];
        std::mem::forget(vec.drain(1..3));
        let p2: usize = guess();
        assert_eq!(
            p2,
            vec.len(),
            "P2: length after a leaked drain (left: your prediction, right: actual)"
        );

        // P3. The standard `vec::IntoIter`: the log after these four drops.
        let log = RefCell::new(String::new());
        let std_vec: Vec<Noisy<'_>> = "abcd"
            .chars()
            .map(|name| Noisy {
                name,
                log: &log,
                explode: false,
            })
            .collect();
        let mut iter = std_vec.into_iter();
        let (first, last) = (iter.next(), iter.next_back());
        drop(iter);
        drop(last);
        drop(first);
        let p3: &str = guess();
        assert_eq!(
            p3,
            *log.borrow(),
            "P3: who drops which element (left: your prediction, right: actual)"
        );
    }
}
