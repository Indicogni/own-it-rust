//! Milestone 4: `RingBuffer` iterators and standard traits
//! (SPEC sections 5.5 and 5.6).
//!
//! ```text
//! cargo test --test m4_ring_iter
//! cargo +nightly-2026-11-20 miri test --test m4_ring_iter
//! ```

// The placeholder types of the starter have no destructor yet. Until yours
// have one, Clippy would flag every `drop(collection)` in this file.
#![allow(clippy::drop_non_drop, clippy::forget_non_drop)]

mod common;

use common::{Ledger, Tracked, bounded, expect_panic, tags};
use nostd_collections::RingBuffer;

/// A buffer of capacity `N` that holds `values` and whose front is in slot
/// `rotation` of the storage, so that longer contents wrap.
fn rotated<const N: usize>(rotation: usize, values: &[u32]) -> RingBuffer<u32, N> {
    let mut ring = RingBuffer::new();
    for _ in 0..rotation {
        ring.push_back(0).unwrap();
        ring.pop_front();
    }
    for &value in values {
        ring.push_back(value).unwrap();
    }
    ring
}

fn rotated_tracked<'a, const N: usize>(
    ledger: &'a Ledger,
    rotation: usize,
    tags: &[u32],
) -> RingBuffer<Tracked<'a>, N> {
    let mut ring = RingBuffer::new();
    for _ in 0..rotation {
        ring.push_back(ledger.item(0)).unwrap();
        // Leaking the placeholder keeps the ledger simple: only the real
        // elements count.
        std::mem::forget(ring.pop_front());
    }
    for &tag in tags {
        ring.push_back(ledger.item(tag)).unwrap();
    }
    ring
}

/// Compiles only if the argument is a double-ended iterator that knows its
/// length.
fn exact_and_double_ended<I: DoubleEndedIterator + ExactSizeIterator>(iter: I) -> I {
    iter
}

// ---------------------------------------------------------------------------
// SPEC 5.5: iter, iter_mut, into_iter
// ---------------------------------------------------------------------------

#[test]
fn iter_goes_from_front_to_back_in_every_rotation() {
    bounded(|| {
        for rotation in 0..4 {
            let ring: RingBuffer<u32, 4> = rotated(rotation, &[1, 2, 3, 4]);
            let forward: Vec<u32> = ring.iter().copied().collect();
            assert_eq!(
                forward,
                [1, 2, 3, 4],
                "rotation {rotation}: iter yields front to back"
            );
            let backward: Vec<u32> = ring.iter().rev().copied().collect();
            assert_eq!(
                backward,
                [4, 3, 2, 1],
                "rotation {rotation}: rev yields back to front"
            );
            assert_eq!(ring.len(), 4, "iterating by reference removes nothing");
        }
        let empty: RingBuffer<u32, 4> = RingBuffer::new();
        assert_eq!(empty.iter().next(), None);
        assert_eq!(empty.iter().next_back(), None);
        let none: RingBuffer<u32, 0> = RingBuffer::new();
        assert_eq!(none.iter().count(), 0);
    });
}

#[test]
fn iter_knows_its_length_and_meets_in_the_middle() {
    bounded(|| {
        for rotation in 0..5 {
            let ring: RingBuffer<u32, 5> = rotated(rotation, &[1, 2, 3, 4, 5]);
            let mut iter = exact_and_double_ended(ring.iter());
            assert_eq!(iter.len(), 5, "rotation {rotation}: ExactSizeIterator::len");
            assert_eq!(
                iter.size_hint(),
                (5, Some(5)),
                "rotation {rotation}: size_hint is exact"
            );
            assert_eq!(iter.next(), Some(&1));
            assert_eq!(iter.next_back(), Some(&5));
            assert_eq!(iter.len(), 3);
            assert_eq!(iter.next_back(), Some(&4));
            assert_eq!(iter.next_back(), Some(&3));
            assert_eq!(iter.len(), 1);
            assert_eq!(
                iter.next(),
                Some(&2),
                "rotation {rotation}: both ends meet without overlap"
            );
            assert_eq!(iter.len(), 0);
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next_back(), None);
            assert_eq!(iter.next(), None, "the iterator stays finished");
        }
    });
}

/// An element type that cannot be cloned.
#[derive(Debug, PartialEq)]
struct Unique(&'static str);

#[test]
fn iter_can_be_cloned() {
    bounded(|| {
        let ring: RingBuffer<Unique, 3> = {
            let mut ring = RingBuffer::new();
            ring.push_back(Unique("a")).unwrap();
            ring.push_back(Unique("b")).unwrap();
            ring.push_back(Unique("c")).unwrap();
            ring.pop_front();
            ring.push_back(Unique("d")).unwrap();
            ring
        };
        let mut iter = ring.iter();
        assert_eq!(iter.next(), Some(&Unique("b")));
        // SPEC 5.5: the iterator is `Clone` without a bound on `T`. The next
        // line compiles only if that is so: `Unique` is not `Clone`, and the
        // iterator holds references, which can always be copied. (A derived
        // `Clone` asks for `T: Clone`; write the implementation by hand.)
        let mut copy = iter.clone();
        // The clone continues from the same position, and the two are
        // independent from then on.
        assert_eq!(iter.next(), Some(&Unique("c")));
        assert_eq!(copy.next(), Some(&Unique("c")));
        assert_eq!(copy.next(), Some(&Unique("d")));
        assert_eq!(copy.next(), None);
        assert_eq!(iter.next(), Some(&Unique("d")));
    });
}

#[test]
fn iter_mut_changes_every_element() {
    bounded(|| {
        for rotation in 0..4 {
            let mut ring: RingBuffer<u32, 4> = rotated(rotation, &[1, 2, 3, 4]);
            {
                let iter = exact_and_double_ended(ring.iter_mut());
                assert_eq!(iter.len(), 4);
                for (index, value) in iter.enumerate() {
                    *value += 10 * index as u32;
                }
            }
            assert_eq!(
                ring.iter().copied().collect::<Vec<_>>(),
                [1, 12, 23, 34],
                "rotation {rotation}: iter_mut visits front to back"
            );
            if let Some(last) = ring.iter_mut().next_back() {
                *last = 0;
            }
            assert_eq!(ring.back(), Some(&0));
            // All references of one pass can be alive at the same time.
            let mut all: Vec<&mut u32> = ring.iter_mut().collect();
            all.reverse();
            for (index, value) in all.into_iter().enumerate() {
                *value = index as u32;
            }
            assert_eq!(ring.iter().copied().collect::<Vec<_>>(), [3, 2, 1, 0]);
        }
    });
}

#[test]
fn for_loops_over_references() {
    bounded(|| {
        let mut ring: RingBuffer<u32, 4> = rotated(3, &[1, 2, 3]);
        let mut seen = Vec::new();
        for value in &ring {
            seen.push(*value);
        }
        assert_eq!(
            seen,
            [1, 2, 3],
            "`for x in &ring` yields &T from front to back"
        );
        for value in &mut ring {
            *value *= 10;
        }
        assert_eq!(seen.len(), ring.len());
        let by_ref = exact_and_double_ended((&ring).into_iter());
        assert_eq!(by_ref.rev().copied().collect::<Vec<_>>(), [30, 20, 10]);
    });
}

#[test]
fn into_iter_yields_the_values_from_front_to_back() {
    bounded(|| {
        for rotation in 0..4 {
            let ring: RingBuffer<u32, 4> = rotated(rotation, &[1, 2, 3, 4]);
            let mut iter = exact_and_double_ended(ring.into_iter());
            assert_eq!(iter.len(), 4);
            assert_eq!(iter.next(), Some(1));
            assert_eq!(iter.next_back(), Some(4));
            assert_eq!(iter.len(), 2);
            assert_eq!(iter.next_back(), Some(3));
            assert_eq!(iter.next(), Some(2));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next_back(), None);

            let ring: RingBuffer<u32, 4> = rotated(rotation, &[1, 2, 3]);
            assert_eq!(ring.into_iter().rev().collect::<Vec<_>>(), [3, 2, 1]);
        }
    });
}

#[test]
fn into_iter_drops_what_was_not_taken() {
    bounded(|| {
        let ledger = Ledger::new();
        let ring: RingBuffer<_, 4> = rotated_tracked(&ledger, 3, &[1, 2, 3, 4]);
        let mut iter = ring.into_iter();
        let first = iter.next().expect("four elements");
        let last = iter.next_back().expect("three elements left");
        assert_eq!((first.tag, last.tag), (1, 4));
        drop(iter);
        assert_eq!(
            ledger.dropped(),
            [2, 3],
            "dropping the iterator drops the elements it still owns, front to back, and not \
             the ones it already gave away"
        );
        drop(first);
        drop(last);
        ledger.assert_no_double_drop("after into_iter was dropped half way");
        assert_eq!(
            ledger.live(),
            3,
            "only the three leaked placeholders of the helper remain"
        );
    });
}

// ---------------------------------------------------------------------------
// SPEC 5.6: standard traits
// ---------------------------------------------------------------------------

#[test]
fn clone_copies_the_contents_in_order() {
    bounded(|| {
        for rotation in 0..3 {
            let mut original: RingBuffer<String, 3> = RingBuffer::new();
            for _ in 0..rotation {
                original.push_back(String::new()).unwrap();
                original.pop_front();
            }
            original.push_back("a".to_string()).unwrap();
            original.push_back("b".to_string()).unwrap();
            original.push_back("c".to_string()).unwrap();

            let mut copy = original.clone();
            assert_eq!(
                copy, original,
                "rotation {rotation}: a clone is equal to its original"
            );
            copy.get_mut(0).expect("three elements").push('!');
            assert_eq!(copy.pop_back().as_deref(), Some("c"));
            copy.push_front("z".to_string()).unwrap();
            assert_eq!(
                original.iter().map(String::as_str).collect::<Vec<_>>(),
                ["a", "b", "c"],
                "rotation {rotation}: changing the clone does not change the original"
            );
            assert_eq!(
                copy.iter().map(String::as_str).collect::<Vec<_>>(),
                ["z", "a!", "b"]
            );
        }
        let none: RingBuffer<String, 0> = RingBuffer::new();
        assert!(none.clone().is_empty());
    });
}

#[test]
fn clone_survives_a_panicking_element_clone() {
    bounded(|| {
        let ledger = Ledger::new();
        let original: RingBuffer<_, 5> = rotated_tracked(&ledger, 0, &[1, 2, 3, 4, 5]);
        ledger.panic_on_clone(4);
        expect_panic(
            "clone of a buffer whose fourth element panics in clone",
            "clone bomb",
            || original.clone(),
        );
        assert_eq!(
            ledger.created(),
            8,
            "three clones were made before the fourth one panicked"
        );
        assert_eq!(
            ledger.live(),
            5,
            "the three finished clones must be dropped when clone panics, and the original \
             must be untouched"
        );
        ledger.assert_no_double_drop("after clone panicked");
        assert_eq!(tags(&original), [1, 2, 3, 4, 5]);
        drop(original);
        ledger.assert_all_dropped_once("after the original was dropped");
    });
}

#[test]
fn equality_ignores_where_the_elements_are_stored() {
    bounded(|| {
        let straight: RingBuffer<u32, 4> = rotated(0, &[1, 2, 3]);
        for rotation in 1..4 {
            let turned: RingBuffer<u32, 4> = rotated(rotation, &[1, 2, 3]);
            assert_eq!(
                straight, turned,
                "rotation {rotation}: equal contents in the same order are equal, wherever the \
                 front is in the storage"
            );
        }
        let other_order: RingBuffer<u32, 4> = rotated(0, &[1, 3, 2]);
        assert_ne!(straight, other_order, "the order matters");
        let shorter: RingBuffer<u32, 4> = rotated(0, &[1, 2]);
        assert_ne!(straight, shorter, "a prefix is not equal");
        let empty_a: RingBuffer<u32, 4> = rotated(2, &[]);
        let empty_b: RingBuffer<u32, 4> = RingBuffer::new();
        assert_eq!(empty_a, empty_b, "two empty buffers are equal");
        fn needs_eq<T: Eq>(_: &T) {}
        needs_eq(&straight);
    });
}

#[test]
fn debug_prints_the_contents_from_front_to_back() {
    bounded(|| {
        let ring: RingBuffer<u32, 4> = rotated(3, &[1, 2, 3]);
        assert_eq!(
            format!("{ring:?}"),
            "[1, 2, 3]",
            "like a slice of the contents"
        );
        assert_eq!(format!("{ring:#?}"), format!("{:#?}", [1, 2, 3]));
        let empty: RingBuffer<u32, 4> = RingBuffer::new();
        assert_eq!(format!("{empty:?}"), "[]");
        let mut words: RingBuffer<&str, 2> = RingBuffer::new();
        words.push_front("a").unwrap();
        assert_eq!(format!("{words:?}"), r#"["a"]"#);
    });
}

#[test]
fn zero_sized_elements_can_be_iterated() {
    bounded(|| {
        let mut ring: RingBuffer<(), 4> = RingBuffer::new();
        ring.push_back(()).unwrap();
        ring.push_front(()).unwrap();
        ring.push_back(()).unwrap();
        assert_eq!(ring.iter().len(), 3);
        assert_eq!(ring.iter().count(), 3);
        assert_eq!(ring.iter_mut().rev().count(), 3);
        assert_eq!(ring.clone(), ring);
        assert_eq!(format!("{ring:?}"), "[(), (), ()]");
        assert_eq!(ring.into_iter().count(), 3);
    });
}
