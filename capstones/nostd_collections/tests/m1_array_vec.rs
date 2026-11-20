//! Milestone 1: `CapacityError` and the core of `ArrayVec`
//! (SPEC sections 3 and 4.1 to 4.3).
//!
//! ```text
//! cargo test --test m1_array_vec
//! cargo +nightly-2026-11-20 miri test --test m1_array_vec
//! ```

// The placeholder types of the starter have no destructor yet. Until yours
// have one, Clippy would flag every `drop(collection)` in this file.
#![allow(clippy::drop_non_drop, clippy::forget_non_drop)]

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};

use common::{Ledger, bounded, lies_inside, tags};
use nostd_collections::ArrayVec;

/// SPEC 4.1: `new` is a `const fn`. This function compiles only if it is.
const fn made_in_a_const_fn() -> ArrayVec<u32, 4> {
    ArrayVec::new()
}

// ---------------------------------------------------------------------------
// SPEC 4.1 and 4.2: construction, push, pop, the slice view
// ---------------------------------------------------------------------------

#[test]
fn a_new_vector_is_empty() {
    bounded(|| {
        let mut v = made_in_a_const_fn();
        assert_eq!(v.capacity(), 4, "capacity is the const parameter N");
        assert_eq!(v.len(), 0, "a new vector holds nothing");
        assert!(v.is_empty(), "a new vector is empty");
        assert!(!v.is_full(), "a new vector with N > 0 is not full");
        assert_eq!(v.pop(), None, "pop on an empty vector returns None");
        assert!(
            v.as_slice().is_empty(),
            "the slice of a new vector is empty"
        );

        let d: ArrayVec<u32, 4> = ArrayVec::default();
        assert!(d.is_empty(), "Default::default() is an empty vector");
    });
}

#[test]
fn pushed_values_appear_in_the_slice_in_order() {
    bounded(|| {
        let mut v: ArrayVec<u32, 4> = ArrayVec::new();
        v.push(10);
        v.push(20);
        v.push(30);
        assert_eq!(v.len(), 3, "three values were pushed");
        assert!(!v.is_empty());
        assert!(!v.is_full(), "3 of 4 places are taken");
        assert_eq!(
            v.as_slice(),
            &[10, 20, 30],
            "as_slice shows the values in push order"
        );
        // Everything below reaches slice methods through `Deref<Target = [T]>`.
        assert_eq!(v[1], 20, "indexing comes from Deref to a slice");
        assert_eq!(v.first(), Some(&10));
        assert_eq!(v.last(), Some(&30));
        assert_eq!(v.iter().sum::<u32>(), 60);
        assert!(v.contains(&20));
        v.push(40);
        assert!(v.is_full(), "4 of 4 places are taken");
        assert_eq!(*v, [10, 20, 30, 40]);
    });
}

#[test]
fn pop_returns_the_last_value_first() {
    bounded(|| {
        let mut v: ArrayVec<&str, 3> = ArrayVec::new();
        v.push("a");
        v.push("b");
        v.push("c");
        assert_eq!(v.pop(), Some("c"), "pop takes from the end");
        assert_eq!(v.len(), 2);
        assert_eq!(*v, ["a", "b"]);
        v.push("d");
        assert_eq!(v.pop(), Some("d"));
        assert_eq!(v.pop(), Some("b"));
        assert_eq!(v.pop(), Some("a"));
        assert_eq!(v.pop(), None, "the vector is empty again");
        assert_eq!(v.len(), 0);
    });
}

#[test]
fn try_push_on_a_full_vector_hands_the_value_back() {
    bounded(|| {
        let mut v: ArrayVec<String, 2> = ArrayVec::new();
        assert!(
            v.try_push("a".to_string()).is_ok(),
            "there is room for the first value"
        );
        assert!(
            v.try_push("b".to_string()).is_ok(),
            "there is room for the second value"
        );
        let error = v
            .try_push("c".to_string())
            .expect_err("try_push on a full vector must return Err");
        assert_eq!(
            error.into_inner(),
            "c",
            "the error must carry the rejected value, not drop it"
        );
        assert_eq!(v.len(), 2, "a rejected push changes nothing");
        assert_eq!(*v, ["a", "b"], "the stored values are untouched");
    });
}

#[test]
#[should_panic(expected = "capacity")]
fn push_on_a_full_vector_panics() {
    bounded(|| {
        let mut v: ArrayVec<u8, 2> = ArrayVec::new();
        v.push(1);
        v.push(2);
        v.push(3);
    });
}

#[test]
fn the_slice_can_be_changed_in_place() {
    bounded(|| {
        let mut v: ArrayVec<i32, 8> = ArrayVec::new();
        for n in [5, 3, 9, 1] {
            v.push(n);
        }
        v[0] = 7;
        assert_eq!(*v, [7, 3, 9, 1], "IndexMut comes from DerefMut");
        v.sort_unstable();
        assert_eq!(*v, [1, 3, 7, 9], "slice methods that need &mut work");
        for n in v.iter_mut() {
            *n *= 2;
        }
        assert_eq!(*v, [2, 6, 14, 18]);
        v.as_mut_slice().reverse();
        assert_eq!(v.as_slice(), &[18, 14, 6, 2]);
        v.swap(0, 3);
        assert_eq!(v.len(), 4, "changing elements never changes the length");
        assert_eq!(v.pop(), Some(18));
    });
}

#[test]
fn the_elements_are_stored_inside_the_vector() {
    bounded(|| {
        let mut v: ArrayVec<u64, 4> = ArrayVec::new();
        v.push(1);
        v.push(2);
        assert!(
            lies_inside(v.as_slice(), &v),
            "the slice must point into the ArrayVec value itself (inline storage), not to \
             memory somewhere else"
        );
        let first = std::ptr::from_ref(&v[0]);
        let second = std::ptr::from_ref(&v[1]);
        assert_eq!(
            second as usize - first as usize,
            size_of::<u64>(),
            "neighbouring elements must be exactly one element apart, as in an array"
        );
    });
}

// ---------------------------------------------------------------------------
// SPEC 4.3: ownership of the elements
// ---------------------------------------------------------------------------

#[test]
fn dropping_the_vector_drops_each_element_once_in_index_order() {
    bounded(|| {
        let ledger = Ledger::new();
        {
            let mut v: ArrayVec<_, 8> = ArrayVec::new();
            for tag in [1, 2, 3, 4, 5] {
                v.push(ledger.item(tag));
            }
            assert_eq!(ledger.live(), 5, "push must not drop anything");
        }
        ledger.assert_all_dropped_once("after the vector was dropped");
        assert_eq!(
            ledger.dropped(),
            [1, 2, 3, 4, 5],
            "the elements must be dropped in index order"
        );
    });
}

#[test]
fn only_the_initialised_part_is_dropped() {
    bounded(|| {
        let ledger = Ledger::new();
        {
            let mut v: ArrayVec<_, 8> = ArrayVec::new();
            v.push(ledger.item(1));
            v.push(ledger.item(2));
            v.push(ledger.item(3));
            let popped = v.pop().expect("three values were pushed");
            assert_eq!(popped.tag, 3);
            assert_eq!(
                ledger.live(),
                3,
                "pop moves the value out; it must not drop it"
            );
            drop(popped);
            assert_eq!(
                ledger.dropped(),
                [3],
                "the popped value is dropped by its new owner"
            );
        }
        ledger.assert_all_dropped_once(
            "after a vector that had a value popped was dropped (the popped slot must not be \
             dropped again)",
        );
        assert_eq!(ledger.dropped(), [3, 1, 2]);
    });
}

#[test]
fn a_rejected_value_is_not_dropped_by_the_vector() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut v: ArrayVec<_, 1> = ArrayVec::new();
        v.push(ledger.item(1));
        let error = v.try_push(ledger.item(2)).expect_err("the vector is full");
        assert_eq!(
            ledger.live(),
            2,
            "the rejected value must still be alive inside the error"
        );
        let back = error.into_inner();
        assert_eq!(back.tag, 2);
        drop(back);
        drop(v);
        ledger.assert_all_dropped_once("after the vector and the rejected value were dropped");
    });
}

#[test]
fn truncate_drops_the_tail_in_index_order() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut v: ArrayVec<_, 8> = ArrayVec::new();
        for tag in 1..=6 {
            v.push(ledger.item(tag));
        }
        v.truncate(10);
        assert_eq!(
            v.len(),
            6,
            "truncate to more than the length changes nothing"
        );
        v.truncate(6);
        assert_eq!(v.len(), 6, "truncate to the current length changes nothing");
        assert!(ledger.dropped().is_empty(), "nothing was removed yet");

        v.truncate(2);
        assert_eq!(v.len(), 2);
        assert_eq!(tags(v.iter()), [1, 2], "the first two elements stay");
        assert_eq!(
            ledger.dropped(),
            [3, 4, 5, 6],
            "the tail is dropped in index order"
        );

        v.push(ledger.item(7));
        assert_eq!(
            tags(v.iter()),
            [1, 2, 7],
            "the freed places can be used again"
        );
        drop(v);
        ledger.assert_all_dropped_once("after truncate, push and drop");
    });
}

#[test]
fn clear_drops_everything_and_the_vector_stays_usable() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut v: ArrayVec<_, 4> = ArrayVec::new();
        for tag in [1, 2, 3, 4] {
            v.push(ledger.item(tag));
        }
        v.clear();
        assert_eq!(v.len(), 0);
        assert!(v.is_empty());
        assert_eq!(ledger.dropped(), [1, 2, 3, 4], "clear drops in index order");
        v.clear();
        assert_eq!(
            ledger.dropped(),
            [1, 2, 3, 4],
            "clear on an empty vector does nothing"
        );
        for tag in [5, 6, 7, 8] {
            v.push(ledger.item(tag));
        }
        assert!(
            v.is_full(),
            "the whole capacity is available again after clear"
        );
        drop(v);
        ledger.assert_all_dropped_once("after clear, refill and drop");
    });
}

#[test]
fn heap_owning_elements_work() {
    // Under Miri this test reports a leak or a double free that the ledger
    // tests cannot see for types that own memory.
    bounded(|| {
        let mut v: ArrayVec<String, 4> = ArrayVec::new();
        v.push("one".to_string());
        v.push("two".to_string());
        v.push("three".to_string());
        v[0].push_str("!!");
        assert_eq!(v.pop().as_deref(), Some("three"));
        v.truncate(1);
        v.push(String::from("four"));
        assert_eq!(*v, ["one!!", "four"]);
        let mut boxes: ArrayVec<Box<[u8]>, 2> = ArrayVec::new();
        boxes.push(vec![1, 2, 3].into_boxed_slice());
        assert_eq!(&*boxes[0], &[1, 2, 3]);
    });
}

// ---------------------------------------------------------------------------
// SPEC 4.1: capacity 0 and zero-sized elements
// ---------------------------------------------------------------------------

#[test]
fn a_vector_with_capacity_zero_is_empty_and_full() {
    bounded(|| {
        let mut v: ArrayVec<String, 0> = ArrayVec::new();
        assert_eq!(v.capacity(), 0);
        assert!(v.is_empty(), "N = 0: always empty");
        assert!(v.is_full(), "N = 0: always full");
        let error = v
            .try_push("x".to_string())
            .expect_err("N = 0: no push can succeed");
        assert_eq!(error.into_inner(), "x");
        assert_eq!(v.pop(), None);
        assert!(v.as_slice().is_empty());
        v.clear();
        v.truncate(0);
        assert_eq!(v.len(), 0);
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
        let mut v: ArrayVec<Tick, 5> = ArrayVec::new();
        for _ in 0..5 {
            v.push(Tick);
        }
        assert!(v.is_full(), "five zero-sized values fill a capacity of 5");
        assert!(
            v.try_push(Tick).is_err(),
            "the capacity also limits zero-sized elements"
        );
        // The rejected Tick was dropped with the error: 1 so far.
        assert_eq!(TICKS_DROPPED.load(Ordering::Relaxed), 1);
        let popped = v.pop();
        assert!(popped.is_some());
        assert_eq!(v.len(), 4);
        drop(popped);
        assert_eq!(TICKS_DROPPED.load(Ordering::Relaxed), 2);
        assert_eq!(
            v.iter().count(),
            4,
            "the slice of zero-sized elements has the right length"
        );
        v.truncate(3);
        assert_eq!(TICKS_DROPPED.load(Ordering::Relaxed), 3);
        drop(v);
        assert_eq!(
            TICKS_DROPPED.load(Ordering::Relaxed),
            6,
            "every zero-sized element must be dropped exactly once: 6 were created"
        );
    });
}

#[test]
fn a_huge_capacity_costs_nothing_for_zero_sized_elements() {
    bounded(|| {
        let mut v: ArrayVec<(), { usize::MAX }> = ArrayVec::new();
        assert_eq!(v.capacity(), usize::MAX);
        v.push(());
        v.push(());
        v.push(());
        assert_eq!(v.len(), 3);
        assert!(!v.is_full());
        assert_eq!(v.pop(), Some(()));
        assert_eq!(v.as_slice().len(), 2);
    });
}

// ---------------------------------------------------------------------------
// SPEC 3: CapacityError
// ---------------------------------------------------------------------------

/// A type without `Debug`, `Display`, `Clone` or `PartialEq`.
struct Opaque(u8);

#[test]
fn capacity_error_displays_a_fixed_text() {
    bounded(|| {
        let mut v: ArrayVec<u8, 0> = ArrayVec::new();
        let error = v.try_push(7).expect_err("N = 0");
        assert_eq!(
            error.to_string(),
            "insufficient capacity",
            "the Display text is fixed by SPEC 3"
        );
        assert!(
            !format!("{error:?}").is_empty(),
            "Debug must print something"
        );
        assert_eq!(error.into_inner(), 7);
    });
}

#[test]
fn capacity_error_is_an_error_for_every_element_type() {
    bounded(|| {
        fn describe(error: &dyn std::error::Error) -> String {
            error.to_string()
        }
        let mut v: ArrayVec<Opaque, 1> = ArrayVec::new();
        // `unwrap` needs `CapacityError<Opaque>: Debug`, although `Opaque`
        // has no `Debug`. This line does not compile otherwise.
        v.try_push(Opaque(1)).unwrap();
        let error = v.try_push(Opaque(2)).expect_err("the vector is full");
        assert_eq!(
            describe(&error),
            "insufficient capacity",
            "CapacityError<T> must implement core::error::Error without bounds on T"
        );
        assert!(!format!("{error:?}").is_empty());
        assert_eq!(error.into_inner().0, 2);
        assert_eq!(v[0].0, 1);
    });
}

#[test]
fn capacity_error_works_with_the_question_mark() {
    bounded(|| {
        fn fill(
            v: &mut ArrayVec<String, 2>,
            words: &[&str],
        ) -> Result<(), Box<dyn std::error::Error>> {
            for word in words {
                v.try_push(word.to_string())?;
            }
            Ok(())
        }
        let mut v = ArrayVec::new();
        assert!(fill(&mut v, &["a", "b"]).is_ok());
        let error = fill(&mut v, &["c"]).expect_err("the third word does not fit");
        assert_eq!(error.to_string(), "insufficient capacity");
    });
}
