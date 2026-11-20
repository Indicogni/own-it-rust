//! LESSON 08 — Capstone: a wait-free ring buffer for one producer and one consumer
//! Run: cargo test l08_
//! Model: cargo model l08_
//! Read first: "Lesson 08" in CONCEPTS.md (same package).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Two indices describe the queue. Which thread writes each one, and which
//!    thread only reads it?
//! 2. A slot that the consumer has just emptied will be written again by the
//!    producer. Which pair of atomic operations orders those two accesses?
//!
//! IMPLEMENT the contract below from a blank page: there are no fields, no
//! helper types and no hints in the code. Import from `crate::sync`, never
//! from `std`, and keep every slot in a `crate::sync::UnsafeCell`, so that
//! the model checker sees each slot access. Keep the supplied tests.
//! EXPLAIN afterward: Write your invariants: which slots are initialised,
//! expressed with the two indices, and who may touch which slot. Give the
//! ordering of each of the four index operations, and for each `Acquire`
//! name the `Release` it pairs with and the two accesses that this orders.
//!
//! VERIFY THREE WAYS (this is the mastery evidence for the module):
//! 1. `cargo test l08_` and `cargo test --release l08_` (real threads);
//! 2. `cargo model l08_` (every interleaving of a small scenario);
//! 3. `cargo +nightly-2026-11-20 miri test l08_` (undefined behaviour and leaks).
//!
//! Then write one more model test of your own in the `model` module, for a
//! scenario that the supplied models do not cover, and say in NOTES.md what
//! it would catch.

use crate::sync::atomic::{AtomicUsize, Ordering};
use crate::sync::{Arc, UnsafeCell};
use std::mem::MaybeUninit;

// CONTRACT
//
// `ring(capacity)` creates a first-in, first-out queue with a fixed number of
// slots and returns its two ends. Exactly one thread produces and exactly one
// thread consumes; the two may run at the same time.
//
// 1. `ring(0)` panics with a message that contains "at least one slot".
//    Any other capacity works, also one that is not a power of two.
// 2. `Producer::push(&mut self, value)` stores the value and returns `Ok(())`,
//    or returns `Err(value)` at once if `capacity` values are waiting.
//    `Consumer::pop(&mut self)` returns the oldest value, or `None` at once if
//    the queue is empty. Exactly `capacity` values fit.
// 3. Both operations are wait-free: no lock, no loop, no waiting for the other
//    thread. Each is a bounded number of steps.
// 4. After `ring` has returned, `push` and `pop` make no heap allocation.
//    `ring` itself makes at most two.
// 5. The two ends are separate values that can move to different threads
//    (`Send` when `T: Send`; `T: Sync` is not required). Neither end is
//    `Clone`: "one producer, one consumer" is enforced by ownership and by
//    `&mut self`.
// 6. `Producer::capacity` and `Consumer::capacity` return the capacity.
// 7. When both ends are gone, every value that is still in the queue is
//    dropped exactly once. No value is ever dropped twice or leaked.
// 8. Every atomic operation uses the weakest ordering that your argument
//    needs. The model tests reject orderings that are too weak.

/// The state shared by the two ends.
///
/// `slots` has one slot more than the capacity. `head` and `tail` are slot
/// numbers in `0..slots.len()`.
///
/// Invariants:
/// - The initialised slots are exactly `head, head + 1, ..., tail - 1`
///   (counting modulo `slots.len()`). So `head == tail` means empty, and
///   `tail + 1 == head` means full: one slot always stays unused, which is how
///   the two cases can be told apart.
/// - Only the producer stores to `tail`, and it only writes the slot at
///   `tail`. Only the consumer stores to `head`, and it only reads the slot
///   at `head`. A slot therefore changes hands only through an index store.
struct Ring<T> {
    slots: Box<[UnsafeCell<MaybeUninit<T>>]>,
    /// Next slot to pop. Written by the consumer.
    head: AtomicUsize,
    /// Next slot to push into. Written by the producer.
    tail: AtomicUsize,
}

// SAFETY: the two ends use `&Ring<T>` from two threads. Each slot is accessed
// by one thread at a time (second invariant), and the index stores and loads
// order those accesses, as explained in `push` and `pop`. Values cross from
// the producer's thread to the consumer's thread by value, so `T: Send` is
// required; no `&T` is shared, so `T: Sync` is not.
unsafe impl<T: Send> Sync for Ring<T> {}

impl<T> Ring<T> {
    fn after(&self, index: usize) -> usize {
        if index + 1 == self.slots.len() {
            0
        } else {
            index + 1
        }
    }
}

/// The end that puts values in.
pub struct Producer<T> {
    ring: Arc<Ring<T>>,
}

/// The end that takes values out.
pub struct Consumer<T> {
    ring: Arc<Ring<T>>,
}

/// Creates a queue for `capacity` values and returns its two ends.
pub fn ring<T>(capacity: usize) -> (Producer<T>, Consumer<T>) {
    assert!(capacity > 0, "a ring buffer needs at least one slot");
    let slots = (0..=capacity)
        .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
        .collect();
    let ring = Arc::new(Ring {
        slots,
        head: AtomicUsize::new(0),
        tail: AtomicUsize::new(0),
    });
    let producer = Producer {
        ring: Arc::clone(&ring),
    };
    (producer, Consumer { ring })
}

impl<T> Producer<T> {
    /// Stores `value`, or hands it back if the queue is full.
    pub fn push(&mut self, value: T) -> Result<(), T> {
        let ring = &*self.ring;
        // Relaxed: only this thread stores to `tail`, so it reads its own
        // latest value.
        let tail = ring.tail.load(Ordering::Relaxed);
        let next = ring.after(tail);
        // Acquire: pairs with the consumer's Release store to `head`. Any
        // slot that this value of `head` shows as free was read by the
        // consumer before that store, so the consumer's read of a slot
        // happens before the write below reuses it.
        if next == ring.head.load(Ordering::Acquire) {
            return Err(value);
        }
        // SAFETY: `tail` is in bounds. The slot at `tail` is outside the
        // initialised range, so the consumer does not touch it, and
        // `&mut self` rules out a second producer. Its old content, if any,
        // was moved out by the consumer, so writing without dropping is right.
        ring.slots[tail].with_mut(|slot| unsafe { slot.cast::<T>().write(value) });
        // Release: the write above happens before any Acquire load of `tail`
        // that sees the slot as filled.
        ring.tail.store(next, Ordering::Release);
        Ok(())
    }

    /// How many values the queue can hold.
    pub fn capacity(&self) -> usize {
        self.ring.slots.len() - 1
    }
}

impl<T> Consumer<T> {
    /// Takes the oldest value, or returns `None` if the queue is empty.
    pub fn pop(&mut self) -> Option<T> {
        let ring = &*self.ring;
        // Relaxed: only this thread stores to `head`.
        let head = ring.head.load(Ordering::Relaxed);
        // Acquire: pairs with the producer's Release store to `tail`, so the
        // producer's write of the slot happens before the read below.
        if head == ring.tail.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: `head` is in bounds and differs from `tail`, so the slot is
        // initialised, and the producer does not touch it until `head` moves
        // past it. `&mut self` rules out a second consumer, and moving `head`
        // below makes sure that the value is not read a second time.
        let value = ring.slots[head].with(|slot| unsafe { slot.cast::<T>().read() });
        // Release: the read above happens before the producer's next write
        // to this slot, which comes after an Acquire load of `head`.
        ring.head.store(ring.after(head), Ordering::Release);
        Some(value)
    }

    /// How many values the queue can hold.
    pub fn capacity(&self) -> usize {
        self.ring.slots.len() - 1
    }
}

impl<T> Drop for Ring<T> {
    fn drop(&mut self) {
        // `&mut self`: both ends are gone, and `Arc` ordered everything they
        // did before this point, so Relaxed loads see the final indices.
        let mut head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        while head != tail {
            // SAFETY: slots from `head` up to `tail` are initialised (first
            // invariant) and nothing else can reach them any more. Each is
            // visited once.
            self.slots[head].with_mut(|slot| unsafe { slot.cast::<T>().drop_in_place() });
            head = self.after(head);
        }
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use crate::implements;
    use crate::support::{DropCount, alloc::count, guess, within};
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn a_first_in_first_out() {
        let (mut tx, mut rx) = ring(3);
        assert_eq!(rx.pop(), None::<u8>, "a new queue is empty");
        assert_eq!(tx.push(1), Ok(()));
        assert_eq!(tx.push(2), Ok(()));
        assert_eq!(rx.pop(), Some(1));
        assert_eq!(tx.push(3), Ok(()));
        assert_eq!(
            [rx.pop(), rx.pop(), rx.pop()],
            [Some(2), Some(3), None],
            "values must come out in the order in which they went in"
        );
    }

    #[test]
    #[should_panic(expected = "at least one slot")]
    fn a_zero_capacity_is_refused() {
        let _ = ring::<u8>(0);
    }

    #[test]
    fn b_exactly_capacity_values_fit() {
        for capacity in [1, 2, 3, 4, 5, 7, 8] {
            let (mut tx, mut rx) = ring(capacity);
            assert_eq!(tx.capacity(), capacity);
            assert_eq!(rx.capacity(), capacity);
            for n in 0..capacity {
                assert_eq!(
                    tx.push(n),
                    Ok(()),
                    "capacity {capacity}: value number {} must fit",
                    n + 1
                );
            }
            assert_eq!(
                tx.push(99),
                Err(99),
                "capacity {capacity}: one value too many must come back as `Err`"
            );
            assert_eq!(rx.pop(), Some(0));
            assert_eq!(tx.push(capacity), Ok(()), "`pop` freed one slot");
            assert_eq!(tx.push(99), Err(99));
        }
    }

    #[test]
    fn b_indices_wrap_around_many_times() {
        for capacity in [1, 3, 4] {
            let (mut tx, mut rx) = ring(capacity);
            let mut next_in = 0_u32;
            let mut next_out = 0_u32;
            for round in 0..200 {
                // Push a varying number of values, then pop a varying number.
                for _ in 0..=(round % (capacity + 1)) {
                    if tx.push(next_in).is_ok() {
                        next_in += 1;
                    }
                }
                for _ in 0..=(round % 2) {
                    if let Some(value) = rx.pop() {
                        assert_eq!(
                            value, next_out,
                            "capacity {capacity}: values must stay in order across wrap-around"
                        );
                        next_out += 1;
                    }
                }
            }
            while let Some(value) = rx.pop() {
                assert_eq!(value, next_out);
                next_out += 1;
            }
            assert_eq!(next_out, next_in, "every pushed value must come out");
            assert!(next_in > 100, "the test must really wrap around");
        }
    }

    #[test]
    fn c_values_are_dropped_exactly_once() {
        let drops = DropCount::new();
        let (mut tx, mut rx) = ring(4);
        // Go around the buffer first, so that the queue does not start at slot 0.
        for id in 0..6 {
            assert!(tx.push(drops.value(id)).is_ok());
            let value = rx.pop().expect("one value was pushed");
            assert_eq!(value.id, id);
        }
        assert_eq!(
            drops.get(),
            6,
            "each popped value was dropped once, by the test"
        );

        for id in 6..10 {
            assert!(tx.push(drops.value(id)).is_ok());
        }
        let rejected = tx.push(drops.value(1_000)).unwrap_err();
        assert_eq!(
            rejected.id, 1_000,
            "a rejected value must come back unchanged"
        );
        drop(rejected);
        assert_eq!(drops.get(), 7, "the four queued values must still be alive");
        drop(rx.pop());
        assert_eq!(drops.get(), 8);
        drop(tx);
        assert_eq!(
            drops.get(),
            8,
            "the consumer still exists, so the queued values must stay alive"
        );
        drop(rx);
        assert_eq!(
            drops.get(),
            11,
            "when both ends are gone, the three values left in the queue must be \
             dropped, each exactly once"
        );
    }

    #[test]
    fn d_no_allocation_after_creation() {
        let ((mut tx, mut rx), allocations) = count(|| ring::<u64>(8));
        assert!(
            allocations <= 2,
            "`ring` made {allocations} heap allocations; one shared block and one \
             block of slots are enough"
        );
        let (sum, allocations) = count(|| {
            let mut sum = 0;
            for n in 0..100 {
                tx.push(n).unwrap();
                sum += rx.pop().unwrap();
            }
            sum
        });
        assert_eq!(sum, (0..100).sum::<u64>());
        assert_eq!(
            allocations, 0,
            "100 pushes and pops made {allocations} heap allocation(s); the slots \
             are reserved once, in `ring`"
        );
    }

    #[test]
    fn d_the_value_is_moved_not_copied() {
        let (mut tx, mut rx) = ring(2);
        let text = String::from("a value that lives on the heap");
        let buffer = text.as_ptr();
        tx.push(text).unwrap();
        let popped = rx.pop().unwrap();
        assert_eq!(
            popped.as_ptr(),
            buffer,
            "`pop` must return the String that was pushed (same heap buffer), not a copy"
        );
    }

    #[test]
    fn e_each_end_is_send_and_neither_is_clone() {
        let _ = ring::<u8>(1);
        assert!(implements!(Producer<String>: Send));
        assert!(implements!(Consumer<String>: Send));
        assert!(
            implements!(Producer<Cell<u32>>: Send),
            "values are moved through the queue and never shared, so `T: Send` is enough"
        );
        assert!(implements!(Consumer<Cell<u32>>: Send));
        assert!(
            !implements!(Producer<Rc<u32>>: Send),
            "a value pushed on one thread is popped on another, so `T` must be `Send`"
        );
        assert!(!implements!(Consumer<Rc<u32>>: Send));
        assert!(
            !implements!(Producer<u32>: Clone),
            "a second producer would break the single-producer rule"
        );
        assert!(!implements!(Consumer<u32>: Clone));
    }

    #[test]
    fn f_two_threads_pass_every_value_in_order() {
        let total: u64 = if cfg!(miri) { 300 } else { 200_000 };
        let (count, sum) = within(60, "the ring buffer stress test", move || {
            let (mut tx, mut rx) = ring(7);
            let producer = std::thread::spawn(move || {
                for n in 0..total {
                    let mut value = Box::new(n);
                    // The queue never blocks. A full queue hands the value
                    // back, and the producer tries again.
                    while let Err(back) = tx.push(value) {
                        value = back;
                        std::thread::yield_now();
                    }
                }
            });
            let mut expected = 0_u64;
            let mut sum = 0_u64;
            while expected < total {
                match rx.pop() {
                    Some(value) => {
                        assert_eq!(*value, expected, "values must arrive in order, none lost");
                        sum += *value;
                        expected += 1;
                    }
                    None => std::thread::yield_now(),
                }
            }
            producer.join().unwrap();
            assert!(rx.pop().is_none());
            (expected, sum)
        });
        assert_eq!(count, total);
        assert_eq!(sum, total * (total - 1) / 2);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// Index arithmetic for ring buffers. No threads are involved.
    #[test]
    fn p_predictions() {
        // P1. A buffer with 4 slots where `head == tail` means "empty" and one
        // slot always stays unused. How many values fit?
        let slots = 4;
        let (head, mut tail, mut stored) = (0, 0, 0);
        while (tail + 1) % slots != head {
            tail = (tail + 1) % slots;
            stored += 1;
        }
        let p1: usize = 3;
        assert_eq!(
            p1, stored,
            "P1: values that fit in 4 slots when one stays unused"
        );

        // P2. Another design: the indices only ever count up, and the slot is
        // `index % slots`. The number of stored values is `tail - head`, also
        // after `tail` has wrapped past the largest `usize`.
        let head = usize::MAX - 1;
        let tail = head.wrapping_add(3);
        let p2: (usize, usize) = (1, 3);
        assert_eq!(
            p2,
            (tail, tail.wrapping_sub(head)),
            "P2: (the value of `tail` after wrapping, the number of stored values)"
        );

        // P3. The slot numbers that such a counter visits around the wrap,
        // for 4 slots and for 3 slots. (`usize::MAX` is one less than a power
        // of two, and it is divisible by 3.)
        let around_the_wrap = [usize::MAX - 1, usize::MAX, 0, 1];
        let four_slots = around_the_wrap.map(|index| index % 4);
        let three_slots = around_the_wrap.map(|index| index % 3);
        let p3: ([usize; 4], [usize; 4]) = ([2, 3, 0, 1], [2, 0, 0, 1]);
        assert_eq!(
            p3,
            (four_slots, three_slots),
            "P3: slot numbers around the wrap (with 4 slots, with 3 slots). \
             Which of the two visits a slot twice in a row?"
        );
    }
}

#[cfg(all(test, loom))]
mod model {
    use super::*;
    use crate::support::{DropCount, Tracked, model};
    use crate::sync::thread;

    /// Pushes `value`, trying again (and letting the other thread run) while
    /// the queue is full.
    fn push_when_possible<T>(tx: &mut Producer<T>, mut value: T) {
        while let Err(back) = tx.push(value) {
            value = back;
            thread::yield_now();
        }
    }

    /// Pops, trying again (and letting the other thread run) while the queue
    /// is empty.
    fn pop_when_possible<T>(rx: &mut Consumer<T>) -> T {
        loop {
            match rx.pop() {
                Some(value) => return value,
                None => thread::yield_now(),
            }
        }
    }

    #[test]
    fn a_values_cross_in_order() {
        model(|| {
            let (mut tx, mut rx) = ring(2);
            let producer = thread::spawn(move || {
                for n in 0..2_u32 {
                    push_when_possible(&mut tx, Tracked::new(n));
                }
            });
            // Reading each value must be ordered after the producer's write.
            for n in 0..2_u32 {
                assert_eq!(pop_when_possible(&mut rx).get(), n);
            }
            producer.join().unwrap();
            assert!(rx.pop().is_none());
        });
    }

    #[test]
    fn b_a_slot_is_reused_safely() {
        model(|| {
            // One usable slot and three values: every push after the first
            // reuses a slot that the consumer has just emptied. The producer's
            // write of that slot must be ordered after the consumer's read.
            let (mut tx, mut rx) = ring(1);
            let producer = thread::spawn(move || {
                for n in 0..3_u32 {
                    push_when_possible(&mut tx, Tracked::new(n));
                }
            });
            for n in 0..3_u32 {
                assert_eq!(pop_when_possible(&mut rx).get(), n);
            }
            producer.join().unwrap();
        });
    }

    #[test]
    fn c_leftover_values_are_dropped_once() {
        model(|| {
            let drops = DropCount::new();
            let (mut tx, mut rx) = ring(2);
            let producer = thread::spawn(move || {
                let mut pushed = 0;
                for n in 0..2_u32 {
                    if tx.push(Tracked::counted(n, &drops)).is_ok() {
                        pushed += 1;
                    }
                }
                pushed
                // The producer end is dropped here, possibly first.
            });
            // At most one value is taken out; the rest stays in the queue.
            let taken = rx.pop();
            drop(rx);
            let pushed: usize = producer.join().unwrap();
            assert_eq!(pushed, 2, "two values fit into two slots");
            drop(taken);
            assert_eq!(
                drops.get(),
                2,
                "the popped value and the value left in the queue must each be dropped once"
            );
        });
    }
}
