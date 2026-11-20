//! Milestone 3: the core of `RingBuffer`: both ends, wrap-around, the two
//! slices, and ownership of the elements (SPEC sections 5.1 to 5.4).
//!
//! ```text
//! cargo test --test m3_ring
//! cargo +nightly-2026-11-20 miri test --test m3_ring
//! ```

// The placeholder types of the starter have no destructor yet. Until yours
// have one, Clippy would flag every `drop(collection)` in this file.
#![allow(clippy::drop_non_drop, clippy::forget_non_drop)]

mod common;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::{Ledger, Rng, Tracked, bounded, expect_panic, lies_inside, steps, tags};
use nostd_collections::RingBuffer;

/// SPEC 5.1: `new` is a `const fn`. This function compiles only if it is.
const fn made_in_a_const_fn() -> RingBuffer<u32, 4> {
    RingBuffer::new()
}

/// The contents from front to back, read through `as_slices`.
fn contents<T: Copy, const N: usize>(ring: &RingBuffer<T, N>) -> Vec<T> {
    let (first, second) = ring.as_slices();
    [first, second].concat()
}

fn tags_of<const N: usize>(ring: &RingBuffer<Tracked<'_>, N>) -> Vec<u32> {
    let (first, second) = ring.as_slices();
    tags(first.iter().chain(second))
}

/// A buffer whose contents wrap around the end of the storage: it is filled,
/// two elements leave at the front and two arrive at the back.
/// The result holds `3, 4, 5, 6` in a buffer of capacity 4.
fn wrapped() -> RingBuffer<u32, 4> {
    let mut ring = RingBuffer::new();
    for value in [1, 2, 3, 4] {
        ring.push_back(value).unwrap();
    }
    assert_eq!(ring.pop_front(), Some(1));
    assert_eq!(ring.pop_front(), Some(2));
    ring.push_back(5).unwrap();
    ring.push_back(6).unwrap();
    ring
}

// ---------------------------------------------------------------------------
// SPEC 5.1 and 5.2: construction, the two ends
// ---------------------------------------------------------------------------

#[test]
fn a_new_buffer_is_empty() {
    bounded(|| {
        let mut ring = made_in_a_const_fn();
        assert_eq!(ring.capacity(), 4, "capacity is the const parameter N");
        assert_eq!(ring.len(), 0);
        assert!(ring.is_empty());
        assert!(!ring.is_full());
        assert_eq!(
            ring.pop_front(),
            None,
            "pop_front on an empty buffer returns None"
        );
        assert_eq!(
            ring.pop_back(),
            None,
            "pop_back on an empty buffer returns None"
        );
        assert_eq!(ring.front(), None);
        assert_eq!(ring.back(), None);
        assert_eq!(ring.get(0), None);
        let (first, second) = ring.as_slices();
        assert!(first.is_empty() && second.is_empty());

        let d: RingBuffer<u32, 4> = RingBuffer::default();
        assert!(d.is_empty(), "Default::default() is an empty buffer");
    });
}

#[test]
fn values_leave_in_the_order_they_arrived() {
    bounded(|| {
        let mut ring: RingBuffer<&str, 4> = RingBuffer::new();
        for word in ["first", "second", "third"] {
            assert!(
                ring.push_back(word).is_ok(),
                "push_back into a buffer with room succeeds"
            );
        }
        assert_eq!(ring.len(), 3);
        assert_eq!(ring.front(), Some(&"first"), "front is the oldest value");
        assert_eq!(ring.back(), Some(&"third"), "back is the newest value");
        assert_eq!(ring.pop_front(), Some("first"));
        assert_eq!(ring.pop_front(), Some("second"));
        assert_eq!(ring.pop_front(), Some("third"));
        assert_eq!(ring.pop_front(), None);
        assert!(ring.is_empty());
    });
}

#[test]
fn a_full_buffer_hands_the_value_back() {
    bounded(|| {
        let mut ring: RingBuffer<String, 2> = RingBuffer::new();
        ring.push_back("a".to_string()).unwrap();
        ring.push_back("b".to_string()).unwrap();
        assert!(ring.is_full());
        let error = ring
            .push_back("c".to_string())
            .expect_err("push_back on a full buffer must return Err");
        assert_eq!(error.to_string(), "insufficient capacity");
        assert_eq!(
            error.into_inner(),
            "c",
            "the error carries the rejected value"
        );
        let error = ring
            .push_front("d".to_string())
            .expect_err("push_front on a full buffer must return Err");
        assert_eq!(error.into_inner(), "d");
        assert_eq!(ring.len(), 2, "a rejected push changes nothing");
        assert_eq!(ring.pop_front().as_deref(), Some("a"));
        assert_eq!(ring.pop_front().as_deref(), Some("b"));
    });
}

#[test]
fn the_buffer_can_be_used_forever() {
    bounded(|| {
        // 100 values travel through 3 places: the contents wrap around the
        // end of the storage again and again.
        let mut ring: RingBuffer<u32, 3> = RingBuffer::new();
        ring.push_back(0).unwrap();
        ring.push_back(1).unwrap();
        for value in 2..100 {
            ring.push_back(value).unwrap();
            assert!(ring.is_full());
            assert_eq!(
                ring.pop_front(),
                Some(value - 2),
                "the oldest value leaves first"
            );
            assert_eq!(ring.len(), 2);
            assert_eq!(contents(&ring), [value - 1, value]);
        }
    });
}

#[test]
fn both_ends_work() {
    bounded(|| {
        let mut ring: RingBuffer<u32, 5> = RingBuffer::new();
        ring.push_back(3).unwrap();
        ring.push_front(2).unwrap();
        ring.push_back(4).unwrap();
        ring.push_front(1).unwrap();
        assert_eq!(
            contents(&ring),
            [1, 2, 3, 4],
            "push_front adds before the front"
        );
        assert_eq!(ring.front(), Some(&1));
        assert_eq!(ring.back(), Some(&4));
        assert_eq!(ring.pop_back(), Some(4), "pop_back takes the newest value");
        assert_eq!(
            ring.pop_front(),
            Some(1),
            "pop_front takes the oldest value"
        );
        ring.push_front(0).unwrap();
        ring.push_front(9).unwrap();
        ring.push_front(8).unwrap();
        assert!(ring.is_full());
        assert_eq!(contents(&ring), [8, 9, 0, 2, 3]);
        assert_eq!(ring.pop_back(), Some(3));
        assert_eq!(ring.pop_back(), Some(2));
        assert_eq!(ring.pop_back(), Some(0));
        assert_eq!(ring.pop_back(), Some(9));
        assert_eq!(ring.pop_back(), Some(8));
        assert_eq!(ring.pop_back(), None);
    });
}

#[test]
fn a_stack_at_the_front_wraps_backwards() {
    bounded(|| {
        let mut ring: RingBuffer<u32, 3> = RingBuffer::new();
        for round in 0..50 {
            ring.push_front(round).unwrap();
            ring.push_front(round + 100).unwrap();
            assert_eq!(ring.pop_back(), Some(round), "round {round}");
            assert_eq!(ring.pop_back(), Some(round + 100), "round {round}");
            assert!(ring.is_empty());
        }
    });
}

// ---------------------------------------------------------------------------
// SPEC 5.2: push_overwrite
// ---------------------------------------------------------------------------

#[test]
fn push_overwrite_replaces_the_oldest_value() {
    bounded(|| {
        let mut ring: RingBuffer<u32, 3> = RingBuffer::new();
        assert_eq!(
            ring.push_overwrite(1),
            None,
            "there is room: nothing is evicted"
        );
        assert_eq!(ring.push_overwrite(2), None);
        assert_eq!(ring.push_overwrite(3), None);
        assert_eq!(
            ring.push_overwrite(4),
            Some(1),
            "a full buffer evicts its front"
        );
        assert_eq!(ring.push_overwrite(5), Some(2));
        assert_eq!(
            contents(&ring),
            [3, 4, 5],
            "the newest three values remain, in order"
        );
        assert_eq!(ring.len(), 3);

        let mut one: RingBuffer<&str, 1> = RingBuffer::new();
        assert_eq!(one.push_overwrite("a"), None);
        assert_eq!(one.push_overwrite("b"), Some("a"));
        assert_eq!(one.front(), Some(&"b"));

        let mut none: RingBuffer<String, 0> = RingBuffer::new();
        assert_eq!(
            none.push_overwrite("x".to_string()).as_deref(),
            Some("x"),
            "N = 0: the buffer cannot keep the value, so the value itself comes back"
        );
        assert!(none.is_empty());
    });
}

#[test]
fn push_overwrite_gives_the_evicted_value_to_the_caller() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut ring: RingBuffer<_, 2> = RingBuffer::new();
        assert!(ring.push_overwrite(ledger.item(1)).is_none());
        assert!(ring.push_overwrite(ledger.item(2)).is_none());
        let evicted = ring
            .push_overwrite(ledger.item(3))
            .expect("the buffer was full");
        assert_eq!(evicted.tag, 1);
        assert_eq!(
            ledger.live(),
            3,
            "the evicted value is returned, not dropped"
        );
        drop(evicted);
        drop(ring);
        ledger.assert_all_dropped_once("after push_overwrite and drop");
    });
}

// ---------------------------------------------------------------------------
// SPEC 5.3: get, front, back, the two slices
// ---------------------------------------------------------------------------

#[test]
fn get_counts_from_the_front() {
    bounded(|| {
        let mut ring = wrapped();
        assert_eq!(
            ring.get(0),
            Some(&3),
            "index 0 is the front, wherever it is stored"
        );
        assert_eq!(ring.get(1), Some(&4));
        assert_eq!(ring.get(2), Some(&5));
        assert_eq!(ring.get(3), Some(&6));
        assert_eq!(
            ring.get(4),
            None,
            "an index at or behind the length gives None"
        );
        assert_eq!(ring.get(usize::MAX), None);
        assert_eq!(ring.front(), Some(&3));
        assert_eq!(ring.back(), Some(&6));

        *ring.get_mut(2).expect("index 2 exists") = 50;
        assert_eq!(ring.get_mut(4), None);
        assert_eq!(
            contents(&ring),
            [3, 4, 50, 6],
            "get_mut changes the element in place"
        );
    });
}

#[test]
fn the_two_slices_show_the_contents_in_order() {
    bounded(|| {
        // Every rotation of the contents inside a storage of 5 places.
        for rotation in 0..5u32 {
            let mut ring: RingBuffer<u32, 5> = RingBuffer::new();
            for _ in 0..rotation {
                ring.push_back(0).unwrap();
            }
            for _ in 0..rotation {
                ring.pop_front();
            }
            for len in 0..=5u32 {
                let (first, second) = ring.as_slices();
                let want: Vec<u32> = (10..10 + len).collect();
                assert_eq!(
                    [first, second].concat(),
                    want,
                    "rotation {rotation}, length {len}: the first slice followed by the second \
                     is the contents from front to back"
                );
                assert!(
                    second.is_empty() || !first.is_empty(),
                    "rotation {rotation}, length {len}: the contents start in the first slice"
                );
                if len > 0 {
                    assert!(
                        lies_inside(first, &ring),
                        "the slices must point into the RingBuffer value itself (inline storage)"
                    );
                }
                if !second.is_empty() {
                    assert!(lies_inside(second, &ring));
                }
                if len < 5 {
                    ring.push_back(10 + len).unwrap();
                }
            }
        }
    });
}

#[test]
fn a_buffer_that_never_wrapped_has_an_empty_second_slice() {
    bounded(|| {
        let mut ring: RingBuffer<u32, 4> = RingBuffer::new();
        for value in [1, 2, 3, 4] {
            ring.push_back(value).unwrap();
            let (first, second) = ring.as_slices();
            assert_eq!(first.len(), value as usize);
            assert!(
                second.is_empty(),
                "SPEC 5.3: a buffer that was only pushed at the back, {value} time(s), is one \
                 run of elements"
            );
        }
    });
}

#[test]
fn a_wrapped_buffer_has_two_non_empty_slices() {
    bounded(|| {
        let ring = wrapped();
        let (first, second) = ring.as_slices();
        assert_eq!(
            (first, second),
            (&[3, 4][..], &[5, 6][..]),
            "SPEC 5.3: four pushes, two pops and two pushes in a capacity of 4 leave two \
             elements at the end of the storage and two at its start"
        );
    });
}

#[test]
fn an_empty_buffer_remembers_its_front_slot() {
    bounded(|| {
        // One element travels through slot 0. The buffer is empty again,
        // and its front slot is now slot 1.
        let mut ring: RingBuffer<u32, 4> = RingBuffer::new();
        ring.push_back(0).unwrap();
        assert_eq!(ring.pop_front(), Some(0));
        assert!(ring.is_empty());
        for value in [1, 2, 3, 4] {
            ring.push_back(value).unwrap();
        }
        assert_eq!(
            ring.as_slices(),
            (&[1, 2, 3][..], &[4][..]),
            "SPEC 5.1: pop_front moves the front slot on by one, also when it takes the last \
             element. The front slot is not set back to slot 0, so the next four elements go \
             into slots 1, 2, 3 and 0"
        );

        // The same when the last element leaves at the back: slot 0 is freed
        // and stays the front slot.
        let mut ring: RingBuffer<u32, 4> = RingBuffer::new();
        ring.push_back(0).unwrap();
        assert_eq!(ring.pop_back(), Some(0));
        for value in [1, 2, 3, 4] {
            ring.push_back(value).unwrap();
        }
        assert_eq!(
            ring.as_slices(),
            (&[1, 2, 3, 4][..], &[][..]),
            "SPEC 5.1: pop_back leaves the front slot where it is"
        );
    });
}

#[test]
fn push_front_into_a_new_buffer_uses_the_last_slot() {
    bounded(|| {
        let mut ring: RingBuffer<u32, 4> = RingBuffer::new();
        ring.push_front(1).unwrap();
        let (first, second) = ring.as_slices();
        assert_eq!((first, second), (&[1][..], &[][..]));
        ring.push_back(2).unwrap();
        ring.push_back(3).unwrap();
        assert_eq!(
            ring.as_slices(),
            (&[1][..], &[2, 3][..]),
            "SPEC 5.1: the front slot of a new buffer is slot 0, and push_front writes to the \
             slot before it, which is the last slot. What is pushed at the back afterwards \
             starts at slot 0 again"
        );
        ring.push_front(0).unwrap();
        assert_eq!(
            ring.as_slices(),
            (&[0, 1][..], &[2, 3][..]),
            "the second push_front goes into the slot before the last one"
        );
    });
}

#[test]
fn the_mutable_slices_change_the_elements() {
    bounded(|| {
        let mut ring = wrapped();
        let (first, second) = ring.as_mut_slices();
        assert_eq!(first.len() + second.len(), 4);
        for value in first.iter_mut().chain(second.iter_mut()) {
            *value *= 10;
        }
        first.reverse();
        assert_eq!(contents(&ring), [40, 30, 50, 60]);
        assert_eq!(ring.pop_front(), Some(40));
    });
}

// ---------------------------------------------------------------------------
// SPEC 5.4: ownership of the elements
// ---------------------------------------------------------------------------

/// A full buffer of capacity 5 whose contents `0, 1, 2, 3, 4` wrap: three
/// elements sit at the end of the storage and two at its start. On the way,
/// two more elements with tag 0 are pushed, popped and dropped.
fn wrapped_tracked(ledger: &Ledger) -> RingBuffer<Tracked<'_>, 5> {
    let mut ring = RingBuffer::new();
    for _ in 0..3 {
        ring.push_back(ledger.item(0)).unwrap();
    }
    for _ in 0..2 {
        drop(ring.pop_front());
    }
    for tag in [1, 2, 3, 4] {
        ring.push_back(ledger.item(tag)).unwrap();
    }
    let (first, second) = ring.as_slices();
    assert_eq!(
        (tags(first), tags(second)),
        (vec![0, 1, 2], vec![3, 4]),
        "SPEC 5.3: three pushes, two pops and four pushes in a capacity of 5 leave three \
         elements at the end of the storage and two at its start"
    );
    ring
}

#[test]
fn dropping_a_wrapped_buffer_drops_each_element_once_from_front_to_back() {
    bounded(|| {
        let ledger = Ledger::new();
        let ring = wrapped_tracked(&ledger);
        assert_eq!(
            ledger.dropped(),
            [0, 0],
            "the two popped values were dropped by the helper"
        );
        drop(ring);
        ledger.assert_all_dropped_once("after a wrapped buffer was dropped");
        assert_eq!(
            ledger.dropped(),
            [0, 0, 0, 1, 2, 3, 4],
            "the elements are dropped from front to back, not in storage order"
        );
    });
}

#[test]
fn popped_values_belong_to_the_caller() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut ring = wrapped_tracked(&ledger);
        let front = ring.pop_front().expect("four elements");
        let back = ring.pop_back().expect("three elements");
        assert_eq!((front.tag, back.tag), (0, 4));
        assert_eq!(
            ledger.live(),
            5,
            "pop moves values out; it must not drop them"
        );
        drop(ring);
        assert_eq!(
            ledger.dropped(),
            [0, 0, 1, 2, 3],
            "the buffer drops only what it still holds"
        );
        drop(front);
        drop(back);
        ledger.assert_all_dropped_once("after the popped values were dropped");
    });
}

#[test]
fn clear_drops_everything_and_the_buffer_stays_usable() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut ring = wrapped_tracked(&ledger);
        ring.clear();
        assert!(ring.is_empty());
        assert_eq!(ring.len(), 0);
        assert_eq!(
            ledger.dropped(),
            [0, 0, 0, 1, 2, 3, 4],
            "clear drops from front to back"
        );
        assert!(ring.pop_front().is_none());
        ring.clear();
        for tag in 5..10 {
            ring.push_back(ledger.item(tag)).unwrap();
        }
        assert!(
            ring.is_full(),
            "the whole capacity is available again after clear"
        );
        assert_eq!(tags_of(&ring), [5, 6, 7, 8, 9]);
        drop(ring);
        ledger.assert_all_dropped_once("after clear, refill and drop");
    });
}

#[test]
fn clear_survives_a_panicking_destructor() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut ring = wrapped_tracked(&ledger);
        // Element 1 is in the first of the two slices.
        ledger.panic_on_drop_of(1);
        expect_panic(
            "clear with an element whose destructor panics",
            "drop bomb",
            || ring.clear(),
        );
        ledger.assert_no_double_drop("right after the destructor panicked");
        // Using the buffer must be fine, whatever is left in it.
        let left = ring.len();
        assert!(
            left <= 4,
            "element 1 was dropped; at most four can be left, found {left}"
        );
        let _ = ring.push_back(ledger.item(9));
        drop(ring);
        ledger.assert_all_dropped_once(
            "after the buffer was dropped (the elements behind the panicking one, also those in \
             the second slice, must be dropped exactly once)",
        );
    });
}

#[test]
fn dropping_the_buffer_survives_a_panicking_destructor() {
    bounded(|| {
        let ledger = Ledger::new();
        let ring = wrapped_tracked(&ledger);
        // Element 1 is in the first of the two slices.
        ledger.panic_on_drop_of(1);
        expect_panic(
            "drop of a buffer with an element whose destructor panics",
            "drop bomb",
            || drop(ring),
        );
        ledger.assert_all_dropped_once(
            "after the drop panicked (SPEC 5.4: the elements behind the panicking one, also \
             those in the second slice, must still be dropped, and none of them twice)",
        );
        assert_eq!(
            ledger.dropped(),
            [0, 0, 0, 1, 2, 3, 4],
            "the elements are dropped from front to back, also when a destructor panics"
        );
    });
}

#[test]
fn heap_owning_elements_work() {
    // Under Miri this test reports a leak or a double free that the ledger
    // tests cannot see for types that own memory.
    bounded(|| {
        let mut ring: RingBuffer<String, 3> = RingBuffer::new();
        for round in 0..7 {
            if let Some(old) = ring.push_overwrite(format!("line {round}")) {
                assert_eq!(old, format!("line {}", round - 3));
            }
        }
        ring.get_mut(1).expect("three lines").push('!');
        assert_eq!(ring.pop_back().as_deref(), Some("line 6"));
        assert_eq!(ring.pop_front().as_deref(), Some("line 4"));
        ring.push_front("first".to_string()).unwrap();
        let (first, second) = ring.as_slices();
        assert_eq!([first, second].concat(), ["first", "line 5!"]);
    });
}

// ---------------------------------------------------------------------------
// SPEC 5.1: capacity 0 and zero-sized elements
// ---------------------------------------------------------------------------

#[test]
fn a_buffer_with_capacity_zero_is_empty_and_full() {
    bounded(|| {
        let mut ring: RingBuffer<u8, 0> = RingBuffer::new();
        assert_eq!(ring.capacity(), 0);
        assert!(ring.is_empty(), "N = 0: always empty");
        assert!(ring.is_full(), "N = 0: always full");
        assert_eq!(ring.push_back(1).map_err(|e| e.into_inner()), Err(1));
        assert_eq!(ring.push_front(2).map_err(|e| e.into_inner()), Err(2));
        assert_eq!(ring.pop_front(), None);
        assert_eq!(ring.pop_back(), None);
        assert_eq!(ring.front(), None);
        assert_eq!(ring.back(), None);
        assert_eq!(ring.get(0), None);
        assert_eq!(ring.get_mut(0), None);
        let (first, second) = ring.as_slices();
        assert!(first.is_empty() && second.is_empty());
        let (first, second) = ring.as_mut_slices();
        assert!(first.is_empty() && second.is_empty());
        ring.clear();
        assert_eq!(ring.len(), 0);
    });
}

/// A zero-sized element with a destructor. Only the test below uses the
/// counter, so tests that run at the same time do not disturb it.
struct Tick;

static TICKS_DROPPED: AtomicUsize = AtomicUsize::new(0);

impl Drop for Tick {
    fn drop(&mut self) {
        TICKS_DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn zero_sized_elements_are_counted_and_dropped() {
    bounded(|| {
        assert_eq!(size_of::<Tick>(), 0, "the test needs a zero-sized type");
        let mut ring: RingBuffer<Tick, 3> = RingBuffer::new();
        for _ in 0..10 {
            // 3 fit; from then on each push evicts one, which is dropped here.
            drop(ring.push_overwrite(Tick));
        }
        assert_eq!(ring.len(), 3);
        assert_eq!(
            TICKS_DROPPED.load(Ordering::Relaxed),
            7,
            "seven of ten were evicted"
        );
        assert!(
            ring.push_back(Tick).is_err(),
            "the capacity also limits zero-sized elements"
        );
        assert_eq!(
            TICKS_DROPPED.load(Ordering::Relaxed),
            8,
            "the rejected value was dropped by the test"
        );
        drop(ring.pop_front());
        let (first, second) = ring.as_slices();
        assert_eq!(first.len() + second.len(), 2);
        drop(ring);
        assert_eq!(
            TICKS_DROPPED.load(Ordering::Relaxed),
            11,
            "every zero-sized element must be dropped exactly once: 11 were created"
        );
    });
}

#[test]
fn a_huge_capacity_does_not_overflow_the_index_arithmetic() {
    bounded(|| {
        // Only possible for zero-sized elements. After push_front the front
        // is in the last slot, usize::MAX - 1; "front + offset" overflows
        // unless the wrap-around is computed with care (SPEC 5.1).
        let mut ring: RingBuffer<(), { usize::MAX }> = RingBuffer::new();
        assert_eq!(ring.capacity(), usize::MAX);
        ring.push_front(()).unwrap();
        ring.push_back(()).unwrap();
        ring.push_back(()).unwrap();
        ring.push_back(()).unwrap();
        assert_eq!(ring.len(), 4);
        assert_eq!(ring.get(3), Some(&()));
        assert_eq!(ring.get(4), None);
        assert_eq!(ring.back(), Some(&()));
        let (first, second) = ring.as_slices();
        assert_eq!(first.len() + second.len(), 4);
        assert_eq!(ring.pop_back(), Some(()));
        assert_eq!(ring.pop_front(), Some(()));
        assert_eq!(ring.pop_front(), Some(()));
        assert_eq!(ring.len(), 1);
        assert!(!ring.is_full());
    });
}

// ---------------------------------------------------------------------------
// Everything together, against std::collections::VecDeque as the model
// ---------------------------------------------------------------------------

#[test]
fn random_operations_match_a_vecdeque() {
    bounded(|| {
        const N: usize = 5;
        let ledger = Ledger::new();
        let mut rng = Rng::new(0x5EED_0003);
        let mut ours: RingBuffer<Tracked<'_>, N> = RingBuffer::new();
        let mut model: VecDeque<u32> = VecDeque::new();
        let mut next_tag = 0;
        let mut fresh = || {
            next_tag += 1;
            next_tag
        };

        for step in 0..steps(2000, 250) {
            let op = rng.below(12);
            match op {
                0..=2 => {
                    let tag = fresh();
                    let pushed = ours.push_back(ledger.item(tag)).is_ok();
                    assert_eq!(
                        pushed,
                        model.len() < N,
                        "step {step}: push_back succeeds iff there is room"
                    );
                    if pushed {
                        model.push_back(tag);
                    }
                }
                3 | 4 => {
                    let tag = fresh();
                    let pushed = ours.push_front(ledger.item(tag)).is_ok();
                    assert_eq!(
                        pushed,
                        model.len() < N,
                        "step {step}: push_front succeeds iff there is room"
                    );
                    if pushed {
                        model.push_front(tag);
                    }
                }
                5 | 6 => assert_eq!(
                    ours.pop_front().map(|item| item.tag),
                    model.pop_front(),
                    "step {step}: pop_front"
                ),
                7 => assert_eq!(
                    ours.pop_back().map(|item| item.tag),
                    model.pop_back(),
                    "step {step}: pop_back"
                ),
                8 | 9 => {
                    let tag = fresh();
                    let want = if model.len() == N {
                        model.pop_front()
                    } else {
                        None
                    };
                    model.push_back(tag);
                    let evicted = ours.push_overwrite(ledger.item(tag));
                    assert_eq!(
                        evicted.map(|item| item.tag),
                        want,
                        "step {step}: push_overwrite"
                    );
                }
                10 => {
                    let index = rng.below(N + 1);
                    assert_eq!(
                        ours.get(index).map(|item| item.tag),
                        model.get(index).copied(),
                        "step {step}: get({index})"
                    );
                    if let (Some(item), Some(value)) = (ours.get_mut(index), model.get_mut(index)) {
                        item.tag += 1_000_000;
                        *value += 1_000_000;
                    }
                }
                11 if rng.below(8) == 0 => {
                    ours.clear();
                    model.clear();
                }
                _ => {}
            }
            let want: Vec<u32> = model.iter().copied().collect();
            assert_eq!(
                tags_of(&ours),
                want,
                "step {step}: contents after operation {op}"
            );
            assert_eq!(
                ours.len(),
                model.len(),
                "step {step}: len after operation {op}"
            );
            assert_eq!(ours.is_empty(), model.is_empty(), "step {step}: is_empty");
            assert_eq!(ours.is_full(), model.len() == N, "step {step}: is_full");
            assert_eq!(
                ours.front().map(|item| item.tag),
                model.front().copied(),
                "step {step}: front"
            );
            assert_eq!(
                ours.back().map(|item| item.tag),
                model.back().copied(),
                "step {step}: back"
            );
            assert_eq!(
                ledger.live(),
                model.len(),
                "step {step}: after operation {op}, the values that are alive must be exactly \
                 the elements of the buffer"
            );
            ledger.assert_no_double_drop("during the model test");
        }
        drop(ours);
        ledger.assert_all_dropped_once("at the end of the model test");
    });
}
