//! LESSON 06 — The aliasing rules, as Miri enforces them
//! Run: cargo test --manifest-path mastery/m2_unsafe/Cargo.toml l06_
//! Miri: cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml l06_
//! Read first: "Lesson 06" in CONCEPTS.md (one directory up).
//!
//! In this lesson the native tests can only check results. Whether you kept
//! the aliasing rules is decided by Miri, so the Miri command above is part of
//! the lesson, not an extra.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then do task M: for each specimen in
//! `examples/aliasing.rs`, write down what you expect Miri to say under each of
//! its two aliasing models, and only then run it. Then consider:
//! 1. You hold `p: *mut T`, made from `r: &mut T`. Which later uses of `r`
//!    make `p` unusable?
//! 2. A list node is owned by a `Box` and also reachable through a raw
//!    pointer. Which operations on the `Box` end the raw pointer's permission?
//!
//! IMPLEMENT tasks A and B. Signatures are given; the fields of `Queue` are
//! yours to design. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: In your own words: when does creating a reference, or
//! using an old one, take away the permission of a raw pointer? Then describe
//! one specimen that the two models judge differently, and say what a careful
//! author does about such code.
//!
//! MIRI LAB: `examples/miri_l06_queue.rs` is a queue whose nodes are owned by
//! `Box` and whose tail is a raw pointer. Every native test of task B would
//! pass with it. Predict what Miri reports, run
//! `cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l06_queue`,
//! compare with CONCEPTS.md, then repair the example until Miri is silent.

use std::marker::PhantomData;
use std::ptr::NonNull;

/// A. Mutable access to two different elements of one slice at the same time.
/// Returns `None` if `i == j` or if an index is out of bounds.
///
/// The standard library has `get_disjoint_mut` for this, and `split_at_mut`
/// can do it without `unsafe`; here you write it by hand with raw pointers.
/// Technique: take one pointer to the whole slice and derive both element
/// pointers from it. Once you hold a raw pointer into the slice, do not call
/// anything that borrows the whole slice again (`as_mut_ptr`, `len`,
/// `get_mut`) before you have finished using that pointer.
pub fn pair_mut<T>(slice: &mut [T], i: usize, j: usize) -> Option<(&mut T, &mut T)> {
    todo!("Implement this task")
}

/// B. A first-in, first-out queue as a singly linked list: `push_back` at the
/// tail and `pop_front` at the head, each in constant time and with exactly
/// one allocation per element.
///
/// Invariant: TODO. Say who owns each node, what `head` and `tail` are for an
/// empty queue, and which kinds of pointer to a node may exist between calls.
pub struct Queue<T> {
    // Design the fields and a node type. This placeholder only lets the
    // starter compile.
    _todo: PhantomData<T>,
}

impl<T> Queue<T> {
    /// An empty queue. Does not allocate.
    pub const fn new() -> Queue<T> {
        todo!() // Implement this task.
    }

    pub fn len(&self) -> usize {
        todo!("Implement this task")
    }

    pub fn is_empty(&self) -> bool {
        todo!("Implement this task")
    }

    /// Adds a value at the back. Constant time: do not walk the list.
    pub fn push_back(&mut self, value: T) {
        todo!("Implement this task")
    }

    /// Removes and returns the value at the front.
    pub fn pop_front(&mut self) -> Option<T> {
        todo!("Implement this task")
    }

    /// The value at the front, without removing it.
    pub fn peek_front(&self) -> Option<&T> {
        todo!("Implement this task")
    }

    /// The value at the back, mutably, without removing it.
    pub fn peek_back_mut(&mut self) -> Option<&mut T> {
        todo!("Implement this task")
    }
}

impl<T> Default for Queue<T> {
    fn default() -> Queue<T> {
        Queue::new()
    }
}

impl<T> Drop for Queue<T> {
    /// Drops the remaining values front to back and frees every node.
    fn drop(&mut self) {
        todo!("Implement this task")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::audit;
    use crate::support::{Verdict, verdict_matches};
    use std::cell::RefCell;

    /// Compiles only if `Queue<T>` is covariant in `T`.
    fn _covariant<'a>(long: Queue<&'static str>) -> Queue<&'a str> {
        long
    }

    #[test]
    fn a_pair_mut_gives_two_different_elements() {
        let mut words = ["a".to_string(), "b".to_string(), "c".to_string()];
        let (first, last) = pair_mut(&mut words, 0, 2).expect("0 and 2 are valid and different");
        std::mem::swap(first, last);
        first.push('!');
        last.push('?');
        assert_eq!(words, ["c!", "b", "a?"]);
        let (later, earlier) = pair_mut(&mut words, 2, 1).expect("the order of i and j is free");
        assert_eq!((later.as_str(), earlier.as_str()), ("a?", "b"));
    }

    #[test]
    fn a_pair_mut_refuses_overlap_and_bad_indices() {
        let mut numbers = [1, 2, 3];
        assert!(
            pair_mut(&mut numbers, 1, 1).is_none(),
            "the same index twice would give two &mut to one element"
        );
        assert!(pair_mut(&mut numbers, 0, 3).is_none());
        assert!(pair_mut(&mut numbers, 3, 0).is_none());
        assert!(pair_mut::<u8>(&mut [], 0, 1).is_none());
        let mut units = [(), ()];
        assert!(pair_mut(&mut units, 0, 1).is_some());
    }

    #[test]
    fn a_pair_mut_points_into_the_slice() {
        let mut numbers = [10u64, 20, 30];
        let base = numbers.as_ptr();
        let (a, b) = pair_mut(&mut numbers, 2, 0).unwrap();
        let (a, b): (*const u64, *const u64) = (a, b);
        assert!(
            std::ptr::eq(a, base.wrapping_add(2)) && std::ptr::eq(b, base),
            "the references must point at the elements themselves, not at copies"
        );
    }

    #[test]
    fn b_queue_is_first_in_first_out() {
        let mut queue = Queue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.pop_front(), None::<String>);
        for word in ["a", "b", "c"] {
            queue.push_back(word.to_string());
        }
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.peek_front().map(String::as_str), Some("a"));
        assert_eq!(queue.pop_front().as_deref(), Some("a"));
        queue.push_back("d".to_string());
        assert_eq!(queue.pop_front().as_deref(), Some("b"));
        assert_eq!(queue.pop_front().as_deref(), Some("c"));
        assert_eq!(queue.pop_front().as_deref(), Some("d"));
        assert_eq!(queue.pop_front(), None);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn b_queue_can_be_refilled_after_it_ran_empty() {
        let mut queue = Queue::new();
        for round in 0..3 {
            queue.push_back(round);
            queue.push_back(round + 10);
            assert_eq!(queue.pop_front(), Some(round));
            assert_eq!(queue.pop_front(), Some(round + 10));
            assert!(
                queue.is_empty() && queue.peek_front().is_none(),
                "an emptied queue must not keep a pointer to its old last node"
            );
        }
        queue.push_back(99);
        assert_eq!(queue.peek_front(), Some(&99));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn b_peeks_reach_the_values_in_place() {
        let mut queue = Queue::new();
        assert!(queue.peek_back_mut().is_none());
        queue.push_back(String::from("only"));
        queue.peek_back_mut().unwrap().push('!');
        assert_eq!(
            queue.peek_front().map(String::as_str),
            Some("only!"),
            "with one element, front and back are the same node"
        );
        queue.push_back(String::from("last"));
        *queue.peek_back_mut().unwrap() += "?";
        let front: *const String = queue.peek_front().unwrap();
        assert!(
            std::ptr::eq(front, queue.peek_front().unwrap()),
            "peek must point at the stored value, not at a copy"
        );
        assert_eq!(queue.pop_front().as_deref(), Some("only!"));
        assert_eq!(queue.pop_front().as_deref(), Some("last?"));
    }

    #[test]
    fn b_one_allocation_per_element_and_none_left_behind() {
        let ((), heap) = audit(|| {
            let mut queue = Queue::new();
            for n in 0..10u64 {
                queue.push_back(n);
            }
            for n in 0..4 {
                assert_eq!(queue.pop_front(), Some(n));
            }
            for n in 10..14 {
                queue.push_back(n);
            }
            assert_eq!(queue.len(), 10);
        });
        assert!(
            heap.allocations <= 14,
            "14 pushes made {} allocations: one node per element is enough",
            heap.allocations
        );
        assert_eq!(heap.live, 0, "dropping the queue must free every node");
        assert_eq!(heap.bad_frees, 0);
        let (queue, heap) = audit(Queue::<u8>::new);
        assert_eq!(heap.allocations, 0, "an empty queue needs no memory");
        drop(queue);
    }

    #[test]
    fn b_drop_drops_the_remaining_values_front_to_back() {
        struct Noisy<'a>(char, &'a RefCell<String>);
        impl Drop for Noisy<'_> {
            fn drop(&mut self) {
                self.1.borrow_mut().push(self.0);
            }
        }
        let log = RefCell::new(String::new());
        let mut queue = Queue::new();
        for name in "abcd".chars() {
            queue.push_back(Noisy(name, &log));
        }
        drop(queue.pop_front());
        assert_eq!(*log.borrow(), "a");
        drop(queue);
        assert_eq!(
            *log.borrow(),
            "abcd",
            "each remaining value must be dropped exactly once, front to back"
        );
    }

    /// M. For each specimen in `examples/aliasing.rs`, predict Miri's verdict
    /// under Stacked Borrows and under Tree Borrows, then run both commands
    /// from the top of that file and record what you saw: (stacked, tree).
    #[test]
    fn m_miri_verdicts() {
        use Verdict::{Accepted, Rejected};
        let a: (Verdict, Verdict) = guess();
        let b: (Verdict, Verdict) = guess();
        let c: (Verdict, Verdict) = guess();
        let d: (Verdict, Verdict) = guess();
        let e: (Verdict, Verdict) = guess();
        let f: (Verdict, Verdict) = guess();
        let g: (Verdict, Verdict) = guess();
        let h: (Verdict, Verdict) = guess();
        for (name, (stacked, tree)) in "abcdefgh".chars().zip([a, b, c, d, e, f, g, h]) {
            assert!(
                verdict_matches(name, stacked, tree),
                "specimen {name}: Miri does not say ({stacked:?}, {tree:?}). Run it again \
                 under both models and read the first `error:` line, if there is one."
            );
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Pointer arithmetic counts in elements, addresses count in bytes.
        let values = [1u16, 2, 3, 4];
        let first = values.as_ptr();
        let last = first.wrapping_add(3);
        // SAFETY: both pointers point into `values`, and the distance between
        // them is a multiple of the element size.
        let elements = unsafe { last.offset_from(first) };
        let p1: (isize, usize) = guess();
        assert_eq!(
            p1,
            (elements, last.addr() - first.addr()),
            "P1: (elements apart, bytes apart) (left: your prediction, right: actual)"
        );

        // P2. Two slices that start at the same element but differ in length.
        let (short, long) = (&values[1..2], &values[1..3]);
        let p2: (bool, bool) = guess();
        assert_eq!(
            p2,
            (std::ptr::eq(short, long), std::ptr::addr_eq(short, long)),
            "P2: (ptr::eq, ptr::addr_eq) (left: your prediction, right: actual)"
        );

        // P3. A reborrow does not move anything: are these the same address?
        // And is a pointer to the first field the address of the struct?
        #[repr(C)]
        struct Pair {
            left: u32,
            right: u32,
        }
        let mut pair = Pair { left: 1, right: 2 };
        let whole = &raw const pair;
        let exclusive = &mut pair;
        let reborrowed: *const Pair = &*exclusive;
        let left = &raw const exclusive.left;
        let right = &raw const exclusive.right;
        let p3: (bool, bool, usize) = guess();
        assert_eq!(
            p3,
            (
                std::ptr::eq(whole, reborrowed),
                whole.addr() == left.addr(),
                right.addr() - left.addr()
            ),
            "P3: (same struct?, first field at the struct's address?, bytes between \
             the fields) (left: your prediction, right: actual)"
        );
    }
}
