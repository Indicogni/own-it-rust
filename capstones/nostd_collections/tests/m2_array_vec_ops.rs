//! Milestone 2: the rest of `ArrayVec`: insertion and removal, `retain`,
//! `drain`, iteration by value, the standard traits, and panic safety
//! (SPEC sections 4.4 to 4.8).
//!
//! ```text
//! cargo test --test m2_array_vec_ops
//! cargo +nightly-2026-11-20 miri test --test m2_array_vec_ops
//! ```

// The placeholder types of the starter have no destructor yet. Until yours
// have one, Clippy would flag every `drop(collection)` in this file.
#![allow(clippy::drop_non_drop, clippy::forget_non_drop)]

mod common;

use common::{Ledger, Rng, Tracked, bounded, expect_panic, steps, tags};
use nostd_collections::ArrayVec;

fn vec_of<const N: usize>(values: &[u32]) -> ArrayVec<u32, N> {
    let mut v = ArrayVec::new();
    for &value in values {
        v.push(value);
    }
    v
}

fn tracked<'a, const N: usize>(ledger: &'a Ledger, tags: &[u32]) -> ArrayVec<Tracked<'a>, N> {
    let mut v = ArrayVec::new();
    for &tag in tags {
        v.push(ledger.item(tag));
    }
    v
}

// ---------------------------------------------------------------------------
// SPEC 4.4: insert, remove, swap_remove
// ---------------------------------------------------------------------------

#[test]
fn insert_moves_the_later_elements_up() {
    bounded(|| {
        let mut v: ArrayVec<u32, 6> = vec_of(&[1, 2, 3]);
        v.insert(1, 9);
        assert_eq!(*v, [1, 9, 2, 3], "insert in the middle");
        v.insert(0, 8);
        assert_eq!(*v, [8, 1, 9, 2, 3], "insert at the front");
        v.insert(5, 7);
        assert_eq!(*v, [8, 1, 9, 2, 3, 7], "insert at index == len appends");
        assert!(v.is_full());
    });
}

#[test]
fn try_insert_on_a_full_vector_hands_the_value_back() {
    bounded(|| {
        let mut v: ArrayVec<String, 2> = ArrayVec::new();
        assert!(v.try_insert(0, "b".to_string()).is_ok());
        assert!(v.try_insert(0, "a".to_string()).is_ok());
        let error = v
            .try_insert(1, "x".to_string())
            .expect_err("try_insert on a full vector must return Err");
        assert_eq!(
            error.into_inner(),
            "x",
            "the error carries the rejected value"
        );
        assert_eq!(*v, ["a", "b"], "a rejected insert changes nothing");
    });
}

#[test]
#[should_panic(expected = "capacity")]
fn insert_on_a_full_vector_panics() {
    bounded(|| {
        let mut v: ArrayVec<u32, 2> = vec_of(&[1, 2]);
        v.insert(1, 3);
    });
}

#[test]
#[should_panic(expected = "out of bounds")]
fn insert_behind_the_end_panics() {
    bounded(|| {
        let mut v: ArrayVec<u32, 8> = vec_of(&[1, 2]);
        v.insert(3, 9);
    });
}

#[test]
#[should_panic(expected = "out of bounds")]
fn try_insert_checks_the_index_before_the_capacity() {
    bounded(|| {
        let mut v: ArrayVec<u32, 2> = vec_of(&[1, 2]);
        // The vector is full *and* the index is wrong. SPEC 4.4: the index
        // is a bug of the caller and is reported first.
        let _ = v.try_insert(3, 9);
    });
}

#[test]
fn remove_moves_the_later_elements_down() {
    bounded(|| {
        let mut v: ArrayVec<u32, 5> = vec_of(&[1, 2, 3, 4, 5]);
        assert_eq!(v.remove(1), 2, "remove returns the element at the index");
        assert_eq!(*v, [1, 3, 4, 5], "the later elements keep their order");
        assert_eq!(v.remove(0), 1);
        assert_eq!(v.remove(2), 5, "removing the last element");
        assert_eq!(*v, [3, 4]);
        v.push(6);
        assert_eq!(*v, [3, 4, 6]);
    });
}

#[test]
#[should_panic(expected = "out of bounds")]
fn remove_at_len_panics() {
    bounded(|| {
        let mut v: ArrayVec<u32, 4> = vec_of(&[1, 2]);
        v.remove(2);
    });
}

#[test]
fn swap_remove_fills_the_hole_with_the_last_element() {
    bounded(|| {
        let mut v: ArrayVec<u32, 5> = vec_of(&[1, 2, 3, 4, 5]);
        assert_eq!(v.swap_remove(1), 2);
        assert_eq!(*v, [1, 5, 3, 4], "the last element takes the free place");
        assert_eq!(v.swap_remove(3), 4, "swap_remove of the last element");
        assert_eq!(*v, [1, 5, 3]);
        assert_eq!(v.swap_remove(0), 1);
        assert_eq!(*v, [3, 5]);
    });
}

#[test]
#[should_panic(expected = "out of bounds")]
fn swap_remove_on_an_empty_vector_panics() {
    bounded(|| {
        let mut v: ArrayVec<u32, 4> = ArrayVec::new();
        v.swap_remove(0);
    });
}

#[test]
fn insert_and_remove_move_values_without_dropping_them() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut v: ArrayVec<_, 6> = tracked(&ledger, &[1, 2, 3, 4]);
        v.insert(2, ledger.item(9));
        let removed = v.remove(0);
        let swapped = v.swap_remove(0);
        assert_eq!((removed.tag, swapped.tag), (1, 2));
        assert_eq!(tags(v.iter()), [4, 9, 3]);
        assert_eq!(
            ledger.live(),
            5,
            "insert, remove and swap_remove only move values; none may be dropped"
        );
        drop(removed);
        drop(swapped);
        drop(v);
        ledger.assert_all_dropped_once("after insert, remove, swap_remove and drop");
    });
}

// ---------------------------------------------------------------------------
// SPEC 4.5: retain
// ---------------------------------------------------------------------------

#[test]
fn retain_keeps_the_order_and_drops_the_rest_once() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut v: ArrayVec<_, 8> = tracked(&ledger, &[1, 2, 3, 4, 5, 6, 7]);
        let mut seen = Vec::new();
        let mut dropped_before = Vec::new();
        v.retain(|item| {
            seen.push(item.tag);
            dropped_before.push(ledger.dropped().len());
            item.tag % 3 != 0
        });
        assert_eq!(
            seen,
            [1, 2, 3, 4, 5, 6, 7],
            "retain visits every element once, in order"
        );
        assert_eq!(
            dropped_before,
            [0, 0, 0, 1, 1, 1, 2],
            "SPEC 4.5: a rejected element is dropped before the closure is called for the next \
             element. The numbers are how many elements were dropped at each call"
        );
        assert_eq!(
            tags(v.iter()),
            [1, 2, 4, 5, 7],
            "the kept elements keep their order"
        );
        assert_eq!(
            ledger.dropped(),
            [3, 6],
            "the rejected elements are dropped, each once"
        );
        v.push(ledger.item(8));
        assert_eq!(tags(v.iter()), [1, 2, 4, 5, 7, 8]);
        drop(v);
        ledger.assert_all_dropped_once("after retain and drop");
    });
}

#[test]
fn retain_with_all_or_nothing() {
    bounded(|| {
        let mut v: ArrayVec<u32, 4> = vec_of(&[1, 2, 3]);
        v.retain(|_| true);
        assert_eq!(*v, [1, 2, 3], "keeping everything changes nothing");
        v.retain(|_| false);
        assert!(v.is_empty(), "keeping nothing empties the vector");
        v.retain(|_| panic!("retain on an empty vector must not call the closure"));

        let mut units: ArrayVec<(), 6> = ArrayVec::new();
        for _ in 0..6 {
            units.push(());
        }
        let mut calls = 0;
        units.retain(|_| {
            calls += 1;
            calls % 2 == 0
        });
        assert_eq!(units.len(), 3, "retain works for zero-sized elements");
    });
}

#[test]
fn retain_survives_a_panicking_closure() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut v: ArrayVec<_, 8> = tracked(&ledger, &[1, 2, 3, 4, 5, 6]);
        expect_panic(
            "retain with a closure that panics at element 4",
            "boom at 4",
            || {
                v.retain(|item| {
                    if item.tag == 4 {
                        panic!("boom at 4");
                    }
                    item.tag != 2
                })
            },
        );
        ledger.assert_no_double_drop("after the closure of retain panicked");
        assert_eq!(
            ledger.dropped(),
            [2],
            "only the element that was rejected before the panic is gone"
        );
        assert_eq!(
            tags(v.iter()),
            [1, 3, 4, 5, 6],
            "SPEC 4.5: after a panic the vector holds the elements kept so far, then the \
             element the closure panicked on, then the elements not yet visited"
        );
        v.push(ledger.item(7));
        assert_eq!(v.len(), 6, "the vector is fully usable after the panic");
        drop(v);
        ledger.assert_all_dropped_once("after the vector was dropped");
    });
}

// ---------------------------------------------------------------------------
// SPEC 4.6: drain
// ---------------------------------------------------------------------------

#[test]
fn drain_removes_a_range_and_yields_it_in_order() {
    bounded(|| {
        let mut v: ArrayVec<u32, 8> = vec_of(&[10, 11, 12, 13, 14, 15]);
        let middle: Vec<u32> = v.drain(1..4).collect();
        assert_eq!(middle, [11, 12, 13], "drain yields the range in order");
        assert_eq!(*v, [10, 14, 15], "the elements behind the range move down");

        let all: Vec<u32> = v.drain(..).collect();
        assert_eq!(all, [10, 14, 15]);
        assert!(v.is_empty(), "drain(..) empties the vector");
        v.push(1);
        assert_eq!(*v, [1], "the vector is usable after drain");
    });
}

#[test]
fn drain_accepts_every_kind_of_range() {
    bounded(|| {
        let full = || -> ArrayVec<u32, 6> { vec_of(&[0, 1, 2, 3, 4, 5]) };

        let mut v = full();
        assert_eq!(
            v.drain(4..).collect::<Vec<_>>(),
            [4, 5],
            "a range without an end"
        );
        assert_eq!(*v, [0, 1, 2, 3]);

        let mut v = full();
        assert_eq!(
            v.drain(..2).collect::<Vec<_>>(),
            [0, 1],
            "a range without a start"
        );
        assert_eq!(*v, [2, 3, 4, 5]);

        let mut v = full();
        assert_eq!(
            v.drain(1..=2).collect::<Vec<_>>(),
            [1, 2],
            "an inclusive range"
        );
        assert_eq!(*v, [0, 3, 4, 5]);

        let mut v = full();
        assert_eq!(v.drain(3..3).count(), 0, "an empty range yields nothing");
        assert_eq!(*v, [0, 1, 2, 3, 4, 5], "an empty range removes nothing");

        let mut v = full();
        assert_eq!(
            v.drain(6..).count(),
            0,
            "the empty range at the end is allowed"
        );
        assert_eq!(v.len(), 6);
    });
}

#[test]
fn drain_is_double_ended_and_knows_its_length() {
    bounded(|| {
        let mut v: ArrayVec<u32, 8> = vec_of(&[0, 1, 2, 3, 4, 5, 6]);
        {
            let mut drain = v.drain(1..6);
            assert_eq!(
                drain.len(),
                5,
                "ExactSizeIterator::len is the size of the range"
            );
            assert_eq!(drain.next(), Some(1));
            assert_eq!(drain.next_back(), Some(5));
            assert_eq!(drain.len(), 3);
            assert_eq!(drain.next_back(), Some(4));
            assert_eq!(drain.next(), Some(2));
            assert_eq!(drain.next(), Some(3));
            assert_eq!(drain.len(), 0);
            assert_eq!(drain.next(), None, "both ends have met");
            assert_eq!(drain.next_back(), None);
            assert_eq!(drain.next(), None, "the iterator stays finished");
        }
        assert_eq!(*v, [0, 6]);
    });
}

#[test]
fn a_dropped_drain_removes_the_whole_range() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut v: ArrayVec<_, 8> = tracked(&ledger, &[1, 2, 3, 4, 5, 6]);

        // Not iterated at all.
        drop(v.drain(4..));
        assert_eq!(tags(v.iter()), [1, 2, 3, 4]);
        assert_eq!(
            ledger.dropped(),
            [5, 6],
            "an unused drain still drops its range"
        );

        // Iterated half way.
        let mut drain = v.drain(..3);
        let first = drain.next().expect("the range has three elements");
        drop(drain);
        assert_eq!(first.tag, 1);
        assert_eq!(
            ledger.dropped(),
            [5, 6, 2, 3],
            "dropping the drain drops the elements it did not yield, in order"
        );
        assert_eq!(
            tags(v.iter()),
            [4],
            "the element behind the range moved to the front"
        );
        drop(first);
        drop(v);
        ledger.assert_all_dropped_once("after two drains and drop");
    });
}

#[test]
fn a_leaked_drain_leaves_a_valid_vector() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut v: ArrayVec<_, 8> = tracked(&ledger, &[1, 2, 3, 4, 5, 6]);
        let mut drain = v.drain(2..4);
        let taken = drain.next().expect("the range has two elements");
        assert_eq!(taken.tag, 3);
        // Leaking is safe Rust. The destructor of the drain never runs.
        std::mem::forget(drain);

        let left = tags(v.iter());
        assert_eq!(
            &left[..2],
            [1, 2],
            "the elements before the range must survive a leaked drain"
        );
        assert!(
            !left.contains(&3),
            "element 3 was moved out through the drain, but the vector still shows it: the \
             same value now has two owners. SPEC 4.6 (leak safety): {left:?}"
        );
        let mut distinct = left.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            left.len(),
            "no element may appear twice: {left:?}"
        );
        assert!(
            left.len() <= 5,
            "at most the five elements that were not taken can remain"
        );

        v.push(ledger.item(7));
        assert_eq!(
            v.last().map(|item| item.tag),
            Some(7),
            "the vector is still usable"
        );
        drop(taken);
        drop(v);
        // Leaked elements are never dropped; that is allowed. A double drop
        // is not.
        ledger.assert_no_double_drop("after a drain was leaked and the vector was dropped");
    });
}

#[test]
fn drain_with_a_range_behind_the_end_panics() {
    bounded(|| {
        let mut v: ArrayVec<u32, 8> = vec_of(&[1, 2, 3]);
        expect_panic(
            "drain(1..5) on a vector of length 3",
            "out of bounds",
            || v.drain(1..5).count(),
        );
        expect_panic(
            "drain(..=3) on a vector of length 3",
            "out of bounds",
            || v.drain(..=3).count(),
        );
        let (start, end) = (2, 1);
        expect_panic("drain(2..1)", "out of bounds", || {
            v.drain(start..end).count()
        });
        // "The index behind the last" of this range does not fit into usize.
        // A wrapping `end + 1` would turn it into the empty range 0..0.
        expect_panic("drain(..=usize::MAX)", "out of bounds", || {
            v.drain(..=usize::MAX).count()
        });
        assert_eq!(*v, [1, 2, 3], "a rejected range changes nothing");
    });
}

// ---------------------------------------------------------------------------
// SPEC 4.7: iteration
// ---------------------------------------------------------------------------

#[test]
fn for_loops_over_references() {
    bounded(|| {
        let mut v: ArrayVec<u32, 4> = vec_of(&[1, 2, 3]);
        let mut sum = 0;
        for value in &v {
            sum += *value;
        }
        assert_eq!(sum, 6, "`for x in &v` yields &T");
        for value in &mut v {
            *value *= 10;
        }
        assert_eq!(*v, [10, 20, 30], "`for x in &mut v` yields &mut T");
    });
}

#[test]
fn into_iter_yields_the_values_in_order() {
    bounded(|| {
        let v: ArrayVec<String, 4> = ["a", "b", "c"].into_iter().map(String::from).collect();
        let mut iter = v.into_iter();
        assert_eq!(iter.len(), 3, "ExactSizeIterator::len");
        assert_eq!(iter.next().as_deref(), Some("a"));
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next_back().as_deref(), Some("c"));
        assert_eq!(iter.next().as_deref(), Some("b"));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        assert_eq!(iter.len(), 0);

        let v: ArrayVec<u32, 5> = vec_of(&[1, 2, 3, 4, 5]);
        assert_eq!(v.into_iter().rev().collect::<Vec<_>>(), [5, 4, 3, 2, 1]);

        let empty: ArrayVec<u32, 0> = ArrayVec::new();
        assert_eq!(empty.into_iter().next(), None);
    });
}

#[test]
fn into_iter_drops_what_was_not_taken() {
    bounded(|| {
        let ledger = Ledger::new();
        let v: ArrayVec<_, 8> = tracked(&ledger, &[1, 2, 3, 4, 5]);
        let mut iter = v.into_iter();
        let first = iter.next().expect("five elements");
        let last = iter.next_back().expect("four elements left");
        assert_eq!(
            ledger.live(),
            5,
            "into_iter and next move values; nothing is dropped"
        );
        drop(iter);
        assert_eq!(
            ledger.dropped(),
            [2, 3, 4],
            "dropping the iterator drops the elements it still owns, in order, and not the \
             ones it already gave away"
        );
        drop(first);
        drop(last);
        ledger.assert_all_dropped_once("after the iterator and the two taken values were dropped");

        let unused = Ledger::new();
        let v: ArrayVec<_, 4> = tracked(&unused, &[1, 2]);
        drop(v.into_iter());
        unused.assert_all_dropped_once("after an unused into_iter was dropped");
    });
}

// ---------------------------------------------------------------------------
// SPEC 4.8: standard traits
// ---------------------------------------------------------------------------

#[test]
fn extend_appends_what_fits() {
    bounded(|| {
        let mut v: ArrayVec<u32, 5> = vec_of(&[1]);
        v.extend([2, 3]);
        v.extend(std::iter::empty());
        assert_eq!(*v, [1, 2, 3]);
        v.extend(vec![4, 5]);
        assert!(v.is_full(), "extend up to exactly the capacity is fine");

        let mut v: ArrayVec<u32, 3> = vec_of(&[1]);
        expect_panic("extend with more items than fit", "capacity", || {
            v.extend(2..10)
        });
        assert_eq!(*v, [1, 2, 3], "the items that fit stay in the vector");
    });
}

#[test]
fn extend_survives_a_panicking_iterator() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut v: ArrayVec<_, 8> = tracked(&ledger, &[1]);
        expect_panic(
            "extend with an iterator that panics",
            "iterator gave up",
            || {
                v.extend((2..10).map(|tag| {
                    if tag == 4 {
                        panic!("iterator gave up");
                    }
                    ledger.item(tag)
                }))
            },
        );
        assert_eq!(
            tags(v.iter()),
            [1, 2, 3],
            "the items pushed before the panic stay"
        );
        drop(v);
        ledger.assert_all_dropped_once("after extend panicked and the vector was dropped");
    });
}

#[test]
fn collect_builds_a_vector() {
    bounded(|| {
        let v: ArrayVec<u32, 4> = (1..=3).collect();
        assert_eq!(*v, [1, 2, 3]);
        let full: ArrayVec<char, 3> = "abc".chars().collect();
        assert!(full.is_full());
        let none: ArrayVec<u8, 0> = std::iter::empty().collect();
        assert!(none.is_empty());

        let ledger = Ledger::new();
        expect_panic(
            "collect of 5 items into a capacity of 3",
            "capacity",
            || {
                (1..=5)
                    .map(|tag| ledger.item(tag))
                    .collect::<ArrayVec<_, 3>>()
            },
        );
        assert_eq!(
            ledger.created(),
            4,
            "the iterator is not asked again after the panic"
        );
        ledger.assert_all_dropped_once(
            "after collect panicked (the partly built vector and the item that did not fit must \
             be dropped)",
        );
    });
}

#[test]
fn clone_makes_an_independent_copy() {
    bounded(|| {
        let mut original: ArrayVec<String, 4> = ArrayVec::new();
        original.push("a".to_string());
        original.push("b".to_string());
        let mut copy = original.clone();
        assert_eq!(copy, original, "a clone is equal to its original");
        copy[0].push('!');
        copy.push("c".to_string());
        assert_eq!(
            *original,
            ["a", "b"],
            "changing the clone does not change the original"
        );
        assert_eq!(*copy, ["a!", "b", "c"]);

        let empty: ArrayVec<String, 0> = ArrayVec::new();
        assert!(empty.clone().is_empty());
    });
}

#[test]
fn clone_survives_a_panicking_element_clone() {
    bounded(|| {
        let ledger = Ledger::new();
        let original: ArrayVec<_, 8> = tracked(&ledger, &[1, 2, 3, 4, 5]);
        ledger.panic_on_clone(3);
        expect_panic(
            "clone of a vector whose third element panics in clone",
            "clone bomb",
            || original.clone(),
        );
        assert_eq!(
            ledger.created(),
            7,
            "two clones were made before the third one panicked"
        );
        assert_eq!(
            ledger.live(),
            5,
            "the two finished clones must be dropped when clone panics, and the original must \
             be untouched"
        );
        ledger.assert_no_double_drop("after clone panicked");
        assert_eq!(tags(original.iter()), [1, 2, 3, 4, 5]);
        drop(original);
        ledger.assert_all_dropped_once("after the original was dropped");
    });
}

#[test]
fn debug_prints_like_a_slice() {
    bounded(|| {
        let v: ArrayVec<u32, 8> = vec_of(&[1, 2, 3]);
        assert_eq!(
            format!("{v:?}"),
            "[1, 2, 3]",
            "only the elements, without the free places"
        );
        assert_eq!(format!("{v:#?}"), format!("{:#?}", [1, 2, 3]));
        let empty: ArrayVec<u32, 8> = ArrayVec::new();
        assert_eq!(format!("{empty:?}"), "[]");
        let words: ArrayVec<&str, 2> = ["a"].into_iter().collect();
        assert_eq!(format!("{words:?}"), r#"["a"]"#);
    });
}

#[test]
fn equality_compares_the_elements() {
    bounded(|| {
        let a: ArrayVec<u32, 8> = vec_of(&[1, 2, 3]);
        let mut b: ArrayVec<u32, 8> = vec_of(&[1, 2, 3, 4]);
        assert_ne!(a, b, "different lengths are not equal");
        // The popped 4 still sits in the storage of `b`. It must not count.
        b.pop();
        assert_eq!(
            a, b,
            "equal elements are equal, whatever the free places hold"
        );
        b[2] = 9;
        assert_ne!(a, b);
        fn needs_eq<T: Eq>(_: &T) {}
        needs_eq(&a);
    });
}

#[test]
fn from_array_and_into_inner() {
    bounded(|| {
        let v = ArrayVec::from([1u32, 2, 3]);
        assert!(
            v.is_full(),
            "From<[T; N]> gives a full vector of capacity N"
        );
        assert_eq!(v.capacity(), 3);
        assert_eq!(*v, [1, 2, 3]);
        assert_eq!(
            v.into_inner().ok(),
            Some([1, 2, 3]),
            "a full vector turns into its array"
        );

        let mut partial: ArrayVec<u32, 3> = vec_of(&[1, 2]);
        partial = partial
            .into_inner()
            .expect_err("a vector that is not full cannot become an array");
        assert_eq!(*partial, [1, 2], "the vector comes back unchanged");
        partial.push(3);
        assert_eq!(partial.into_inner().ok(), Some([1, 2, 3]));

        let empty: ArrayVec<u32, 0> = ArrayVec::new();
        assert_eq!(empty.into_inner().ok(), Some([]), "N = 0: always full");
    });
}

#[test]
fn from_array_and_into_inner_move_without_dropping() {
    bounded(|| {
        let ledger = Ledger::new();
        let v = ArrayVec::from([ledger.item(1), ledger.item(2), ledger.item(3)]);
        let array = v.into_inner().expect("the vector is full");
        assert_eq!(
            ledger.live(),
            3,
            "From<[T; N]> and into_inner must not drop or copy elements"
        );
        assert_eq!(tags(&array), [1, 2, 3]);
        drop(array);
        ledger.assert_all_dropped_once("after the array was dropped");

        let ledger = Ledger::new();
        let partial: ArrayVec<_, 4> = tracked(&ledger, &[1, 2]);
        let back = partial.into_inner().expect_err("the vector is not full");
        assert_eq!(
            ledger.live(),
            2,
            "a refused into_inner must not drop anything"
        );
        drop(back);
        ledger.assert_all_dropped_once("after the returned vector was dropped");
    });
}

// ---------------------------------------------------------------------------
// SPEC 4.3: destructors that panic
// ---------------------------------------------------------------------------

#[test]
fn clear_survives_a_panicking_destructor() {
    bounded(|| {
        let ledger = Ledger::new();
        let mut v: ArrayVec<_, 8> = tracked(&ledger, &[1, 2, 3, 4]);
        ledger.panic_on_drop_of(2);
        expect_panic(
            "clear with an element whose destructor panics",
            "drop bomb",
            || v.clear(),
        );
        ledger.assert_no_double_drop("right after the destructor panicked");
        // Using the vector must be fine, whatever is left in it.
        let left = v.len();
        assert!(
            left <= 3,
            "element 2 was dropped; at most three can be left, found {left}"
        );
        v.push(ledger.item(9));
        drop(v);
        ledger.assert_all_dropped_once(
            "after the vector was dropped (the panicking destructor ran once, and every other \
             element must be dropped exactly once as well)",
        );
    });
}

#[test]
fn dropping_the_vector_survives_a_panicking_destructor() {
    bounded(|| {
        let ledger = Ledger::new();
        let v: ArrayVec<_, 8> = tracked(&ledger, &[1, 2, 3, 4]);
        ledger.panic_on_drop_of(1);
        expect_panic(
            "drop of a vector whose first element panics in drop",
            "drop bomb",
            || drop(v),
        );
        ledger.assert_all_dropped_once(
            "after the drop panicked (the elements behind the panicking one must still be \
             dropped)",
        );
    });
}

// ---------------------------------------------------------------------------
// Everything together, against std::vec::Vec as the model
// ---------------------------------------------------------------------------

#[test]
fn random_operations_match_a_vec() {
    bounded(|| {
        const N: usize = 6;
        let ledger = Ledger::new();
        let mut rng = Rng::new(0x5EED_0001);
        let mut ours: ArrayVec<Tracked<'_>, N> = ArrayVec::new();
        let mut model: Vec<u32> = Vec::new();
        let mut next_tag = 0;
        let mut fresh = || {
            next_tag += 1;
            next_tag
        };

        for step in 0..steps(2000, 250) {
            let op = rng.below(11);
            match op {
                0..=2 => {
                    let tag = fresh();
                    let pushed = ours.try_push(ledger.item(tag)).is_ok();
                    assert_eq!(
                        pushed,
                        model.len() < N,
                        "step {step}: try_push succeeds iff there is room"
                    );
                    if pushed {
                        model.push(tag);
                    }
                }
                3 => assert_eq!(
                    ours.pop().map(|item| item.tag),
                    model.pop(),
                    "step {step}: pop"
                ),
                4 => {
                    let index = rng.below(model.len() + 1);
                    let tag = fresh();
                    let inserted = ours.try_insert(index, ledger.item(tag)).is_ok();
                    assert_eq!(inserted, model.len() < N, "step {step}: try_insert");
                    if inserted {
                        model.insert(index, tag);
                    }
                }
                5 if !model.is_empty() => {
                    let index = rng.below(model.len());
                    assert_eq!(
                        ours.remove(index).tag,
                        model.remove(index),
                        "step {step}: remove({index})"
                    );
                }
                6 if !model.is_empty() => {
                    let index = rng.below(model.len());
                    assert_eq!(
                        ours.swap_remove(index).tag,
                        model.swap_remove(index),
                        "step {step}: swap_remove({index})"
                    );
                }
                7 => {
                    let len = rng.below(N + 2);
                    ours.truncate(len);
                    model.truncate(len);
                }
                8 => {
                    let modulus = 2 + rng.below(3) as u32;
                    ours.retain(|item| item.tag % modulus != 0);
                    model.retain(|tag| tag % modulus != 0);
                }
                9 => {
                    let start = rng.below(model.len() + 1);
                    let end = start + rng.below(model.len() - start + 1);
                    let take = rng.below(end - start + 1);
                    let got: Vec<u32> = ours
                        .drain(start..end)
                        .take(take)
                        .map(|item| item.tag)
                        .collect();
                    let want: Vec<u32> = model.drain(start..end).take(take).collect();
                    assert_eq!(
                        got, want,
                        "step {step}: drain({start}..{end}), first {take} items"
                    );
                }
                10 => {
                    let copy = ours.clone();
                    assert_eq!(copy, ours, "step {step}: a clone equals its original");
                    let by_value: Vec<u32> = copy.into_iter().rev().map(|item| item.tag).collect();
                    let want: Vec<u32> = model.iter().rev().copied().collect();
                    assert_eq!(by_value, want, "step {step}: clone().into_iter().rev()");
                }
                _ => {}
            }
            assert_eq!(
                tags(ours.iter()),
                model,
                "step {step}: contents after operation {op}"
            );
            assert_eq!(
                ours.len(),
                model.len(),
                "step {step}: len after operation {op}"
            );
            assert_eq!(
                ledger.live(),
                model.len(),
                "step {step}: after operation {op}, the values that are alive must be exactly \
                 the elements of the vector"
            );
            ledger.assert_no_double_drop("during the model test");
        }
        drop(ours);
        ledger.assert_all_dropped_once("at the end of the model test");
    });
}
