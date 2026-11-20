//! Host acceptance tests for `kcore::RingBuffer` (SPEC 5.1).
//!
//! ```text
//! cargo test --test host_ring
//! cargo +nightly-2026-11-20 miri test --test host_ring
//! ```

use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;

use kcore::RingBuffer;

/// SPEC 5.1: `new` is a `const fn`, so a queue can initialise a `static`
/// without lazy initialisation. The kernel needs that before it has a heap.
/// This function compiles only if `new` is a `const fn`.
const fn made_in_a_const_fn() -> RingBuffer<u32, 4> {
    RingBuffer::new()
}

#[test]
fn new_queue_is_empty() {
    let mut queue: RingBuffer<u8, 3> = RingBuffer::new();
    assert_eq!(queue.capacity(), 3, "capacity is the const parameter N");
    assert_eq!(queue.len(), 0, "a new queue holds nothing");
    assert!(queue.is_empty(), "a new queue is empty");
    assert!(!queue.is_full(), "a new queue with N > 0 is not full");
    assert_eq!(queue.pop(), None, "pop on an empty queue returns None");
}

#[test]
fn values_come_out_in_the_order_they_went_in() {
    let mut queue: RingBuffer<&str, 4> = RingBuffer::new();
    for word in ["first", "second", "third"] {
        assert_eq!(
            queue.push(word),
            Ok(()),
            "push into a queue with room succeeds"
        );
    }
    assert_eq!(queue.len(), 3, "three values were pushed");
    assert_eq!(queue.pop(), Some("first"), "the oldest value leaves first");
    assert_eq!(queue.pop(), Some("second"));
    assert_eq!(queue.pop(), Some("third"));
    assert_eq!(queue.pop(), None, "the queue is empty again");
}

#[test]
fn a_full_queue_hands_the_value_back() {
    let mut queue: RingBuffer<String, 2> = RingBuffer::new();
    assert!(queue.push("a".to_string()).is_ok());
    assert!(queue.push("b".to_string()).is_ok());
    assert!(queue.is_full(), "two values fill a queue of capacity 2");
    assert_eq!(
        queue.push("c".to_string()),
        Err("c".to_string()),
        "push on a full queue must return the rejected value, not drop it"
    );
    assert_eq!(queue.len(), 2, "a rejected push changes nothing");
    assert_eq!(
        queue.pop().as_deref(),
        Some("a"),
        "the queued values are untouched"
    );
    assert_eq!(queue.pop().as_deref(), Some("b"));
}

#[test]
fn zero_capacity_queue_is_always_full_and_empty() {
    let mut queue: RingBuffer<u8, 0> = RingBuffer::new();
    assert!(
        queue.is_empty() && queue.is_full(),
        "N = 0: empty and full at once"
    );
    assert_eq!(
        queue.push(7),
        Err(7),
        "N = 0: every push is rejected (and must not divide by zero)"
    );
    assert_eq!(queue.pop(), None, "N = 0: nothing to pop");
}

#[test]
fn indices_wrap_around_many_times() {
    // A model with the same contract; the two must never disagree.
    let mut queue: RingBuffer<u32, 5> = RingBuffer::new();
    let mut model: VecDeque<u32> = VecDeque::new();
    // A small linear congruential generator: deterministic "random" choices.
    let mut state: u32 = 0x2545_f491;
    for step in 0..2_000_u32 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        if !(state >> 16).is_multiple_of(3) {
            let expected = if model.len() < 5 {
                model.push_back(step);
                Ok(())
            } else {
                Err(step)
            };
            assert_eq!(
                queue.push(step),
                expected,
                "step {step}: push disagrees with the model"
            );
        } else {
            assert_eq!(
                queue.pop(),
                model.pop_front(),
                "step {step}: pop disagrees with the model"
            );
        }
        assert_eq!(
            queue.len(),
            model.len(),
            "step {step}: len disagrees with the model"
        );
        assert_eq!(
            queue.is_empty(),
            model.is_empty(),
            "step {step}: is_empty is wrong"
        );
        assert_eq!(
            queue.is_full(),
            model.len() == 5,
            "step {step}: is_full is wrong"
        );
    }
}

#[test]
fn new_is_a_const_fn() {
    let mut queue = made_in_a_const_fn();
    assert_eq!(queue.push(11), Ok(()));
    assert_eq!(
        queue.pop(),
        Some(11),
        "a queue made in a const fn works like any other"
    );
}

/// Counts how many times values of this type were dropped.
struct CountsDrops(Rc<Cell<usize>>);

impl Drop for CountsDrops {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn dropping_the_queue_drops_what_is_still_queued_exactly_once() {
    let drops = Rc::new(Cell::new(0));
    {
        let mut queue: RingBuffer<CountsDrops, 4> = RingBuffer::new();
        // Move the head away from slot 0 so that the live values wrap.
        for _ in 0..3 {
            assert!(queue.push(CountsDrops(Rc::clone(&drops))).is_ok());
        }
        drop(queue.pop());
        drop(queue.pop());
        assert_eq!(
            drops.get(),
            2,
            "a popped value is dropped by its new owner, once"
        );
        for _ in 0..3 {
            assert!(queue.push(CountsDrops(Rc::clone(&drops))).is_ok());
        }
        assert_eq!(drops.get(), 2, "pushing must not drop anything");
        assert_eq!(queue.len(), 4);
    }
    assert_eq!(
        drops.get(),
        6,
        "the queue owned 4 values when it was dropped: each must be dropped exactly once \
         (2 earlier drops + 4)"
    );
}

#[test]
fn an_empty_queue_drops_nothing() {
    let drops = Rc::new(Cell::new(0));
    {
        let mut queue: RingBuffer<CountsDrops, 2> = RingBuffer::new();
        assert!(queue.push(CountsDrops(Rc::clone(&drops))).is_ok());
        drop(queue.pop());
    }
    assert_eq!(
        drops.get(),
        1,
        "a value that was popped must not be dropped again by the queue"
    );
}
