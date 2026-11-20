//! Milestone 6: what the types promise beyond single calls: sizes, thread
//! safety, variance, and working together (SPEC section 8).
//!
//! ```text
//! cargo test --test m6_contracts
//! cargo +nightly-2026-11-20 miri test --test m6_contracts
//! ```

mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::{Ledger, bounded};
use nostd_collections::{Arena, ArrayVec, CapacityError, RingBuffer};

/// `true` if the type implements the trait. Unlike a `where` clause, this
/// also works for the answer "no": the test then fails with a message, and
/// the file still compiles.
///
/// How it works: an inherent associated constant is preferred over one from
/// a trait, but the inherent one exists only if the bound holds.
macro_rules! implements {
    ($ty:ty: $($bound:tt)+) => {{
        #[allow(dead_code)]
        trait Fallback {
            const IMPLEMENTS: bool = false;
        }
        impl<T: ?Sized> Fallback for T {}
        struct Probe<T: ?Sized>(std::marker::PhantomData<T>);
        #[allow(dead_code)]
        impl<T: ?Sized + $($bound)+> Probe<T> {
            const IMPLEMENTS: bool = true;
        }
        <Probe<$ty>>::IMPLEMENTS
    }};
}

// ---------------------------------------------------------------------------
// SPEC 8.1: sizes
// ---------------------------------------------------------------------------

const WORD: usize = size_of::<usize>();

#[test]
fn the_probe_macro_works() {
    assert!(implements!(u8: Send));
    assert!(!implements!(Rc<u8>: Send));
    assert!(!implements!(Cell<u8>: Sync));
}

#[test]
fn array_vec_is_its_elements_plus_a_length() {
    macro_rules! check {
        ($t:ty, $n:expr) => {
            let size = size_of::<ArrayVec<$t, $n>>();
            let elements = size_of::<[$t; $n]>();
            assert!(
                size >= elements,
                "ArrayVec<{}, {}> is {size} bytes, less than its {elements} bytes of elements: \
                 the elements are not stored inline",
                stringify!($t),
                $n
            );
            assert!(
                size <= elements + 2 * WORD,
                "ArrayVec<{}, {}> is {size} bytes; SPEC 8.1 allows the {elements} bytes of the \
                 elements plus two machine words. A slot must cost exactly one element: no \
                 Option, no flag per slot",
                stringify!($t),
                $n
            );
        };
    }
    check!(u8, 32);
    check!(u16, 7);
    check!(u64, 8);
    check!(usize, 1);
    check!(String, 4);
    check!(Option<Box<u8>>, 3);
    check!(u8, 0);
    check!((), 1000);
}

#[test]
fn ring_buffer_is_its_elements_plus_two_indices() {
    macro_rules! check {
        ($t:ty, $n:expr) => {
            let size = size_of::<RingBuffer<$t, $n>>();
            let elements = size_of::<[$t; $n]>();
            assert!(
                size >= elements,
                "RingBuffer<{}, {}> is {size} bytes, less than its {elements} bytes of \
                 elements: the elements are not stored inline",
                stringify!($t),
                $n
            );
            assert!(
                size <= elements + 3 * WORD,
                "RingBuffer<{}, {}> is {size} bytes; SPEC 8.1 allows the {elements} bytes of \
                 the elements plus three machine words",
                stringify!($t),
                $n
            );
        };
    }
    check!(u8, 32);
    check!(u16, 7);
    check!(u64, 8);
    check!(String, 4);
    check!(u8, 0);
    check!((), 1000);
}

#[test]
fn arena_is_its_storage_plus_a_cursor() {
    macro_rules! check {
        ($n:expr) => {
            let storage: usize = $n;
            let size = size_of::<Arena<$n>>();
            let limit = storage.next_multiple_of(16) + 16;
            assert!(
                size >= storage,
                "Arena<{}> is {size} bytes: the storage is not inline",
                $n
            );
            assert!(
                size <= limit,
                "Arena<{}> is {size} bytes; SPEC 8.1 allows {limit}",
                $n
            );
        };
    }
    check!(0);
    check!(1);
    check!(16);
    check!(100);
    check!(4096);
}

// ---------------------------------------------------------------------------
// SPEC 8.2: Send and Sync
// ---------------------------------------------------------------------------

#[test]
fn collections_are_send_and_sync_exactly_when_their_elements_are() {
    assert!(
        implements!(ArrayVec<u32, 4>: Send),
        "ArrayVec<u32, N> must be Send"
    );
    assert!(
        implements!(ArrayVec<u32, 4>: Sync),
        "ArrayVec<u32, N> must be Sync"
    );
    assert!(
        implements!(RingBuffer<String, 4>: Send),
        "RingBuffer<String, N> must be Send"
    );
    assert!(
        implements!(RingBuffer<String, 4>: Sync),
        "RingBuffer<String, N> must be Sync"
    );

    assert!(
        !implements!(ArrayVec<Rc<u8>, 4>: Send),
        "ArrayVec<Rc<u8>, N> must not be Send: moving it to another thread moves the Rc"
    );
    assert!(!implements!(ArrayVec<Rc<u8>, 4>: Sync));
    assert!(
        !implements!(RingBuffer<Rc<u8>, 4>: Send),
        "RingBuffer<Rc<u8>, N> must not be Send"
    );
    assert!(!implements!(RingBuffer<Rc<u8>, 4>: Sync));

    assert!(
        implements!(ArrayVec<Cell<u8>, 4>: Send),
        "Cell<u8> is Send, so the vector is"
    );
    assert!(
        !implements!(ArrayVec<Cell<u8>, 4>: Sync),
        "ArrayVec<Cell<u8>, N> must not be Sync: &ArrayVec gives &Cell<u8>"
    );
    assert!(!implements!(RingBuffer<Cell<u8>, 4>: Sync));
}

#[test]
fn arena_is_send_but_not_sync() {
    assert!(
        implements!(Arena<64>: Send),
        "an arena that nobody borrows can move to another thread"
    );
    assert!(
        !implements!(Arena<64>: Sync),
        "SPEC 8.2: Arena must not be Sync. `alloc` takes &self and changes the arena without \
         synchronisation; two threads would hand out the same bytes"
    );
}

#[test]
fn capacity_error_passes_the_auto_traits_of_its_value_on() {
    assert!(implements!(CapacityError<u8>: Send + Sync + Unpin));
    assert!(!implements!(CapacityError<Rc<u8>>: Send));
    assert!(
        implements!(CapacityError<String>: std::error::Error + Send + Sync + 'static),
        "CapacityError<String> must convert into Box<dyn Error + Send + Sync>"
    );
}

#[test]
fn collections_move_to_another_thread() {
    bounded(|| {
        let mut v: ArrayVec<String, 4> = ArrayVec::new();
        v.push("made on the first thread".to_string());
        let mut ring: RingBuffer<String, 2> = RingBuffer::new();
        ring.push_back("queued".to_string()).unwrap();
        let mut arena: Arena<64> = Arena::new();
        arena.alloc(1u64).unwrap();
        arena.reset();

        let handle = std::thread::spawn(move || {
            v.push("changed on the second".to_string());
            let total = *arena.alloc(20u64).unwrap() + *arena.alloc(22u64).unwrap();
            (v, ring.pop_front(), total)
        });
        let (v, word, total) = handle.join().expect("the second thread must not panic");
        assert_eq!(*v, ["made on the first thread", "changed on the second"]);
        assert_eq!(word.as_deref(), Some("queued"));
        assert_eq!(total, 42);
    });
}

// ---------------------------------------------------------------------------
// SPEC 8.3: variance
// ---------------------------------------------------------------------------

/// These two functions compile only if the collections are covariant in
/// `T`: a collection of long-lived references can be used where a
/// collection of shorter-lived references is expected, as with `Vec<T>`.
fn shorten_vec<'a>(v: ArrayVec<&'static str, 3>) -> ArrayVec<&'a str, 3> {
    v
}

fn shorten_ring<'a>(ring: RingBuffer<&'static str, 3>) -> RingBuffer<&'a str, 3> {
    ring
}

#[test]
fn collections_are_covariant_in_their_element_type() {
    bounded(|| {
        let mut statics: ArrayVec<&'static str, 3> = ArrayVec::new();
        statics.push("static");
        let local = String::from("local");
        let mut mixed = shorten_vec(statics);
        mixed.push(&local);
        assert_eq!(*mixed, ["static", "local"]);

        let mut statics: RingBuffer<&'static str, 3> = RingBuffer::new();
        statics.push_back("static").unwrap();
        let mut mixed = shorten_ring(statics);
        mixed.push_front(&local).unwrap();
        assert_eq!(mixed.pop_back(), Some("static"));
        assert_eq!(mixed.pop_back(), Some(local.as_str()));
    });
}

// ---------------------------------------------------------------------------
// SPEC 8.4: the types together
// ---------------------------------------------------------------------------

#[test]
fn collections_nest() {
    bounded(|| {
        let ledger = Ledger::new();
        {
            let mut rows: ArrayVec<ArrayVec<_, 3>, 2> = ArrayVec::new();
            for row in 0..2 {
                let mut cells = ArrayVec::new();
                for column in 0..3 {
                    cells.push(ledger.item(10 * row + column));
                }
                rows.push(cells);
            }
            let mut history: RingBuffer<ArrayVec<_, 3>, 2> = RingBuffer::new();
            for round in 0..5 {
                let mut snapshot = ArrayVec::new();
                snapshot.push(ledger.item(100 + round));
                // Evicted snapshots are dropped here, with their elements.
                history.push_overwrite(snapshot);
            }
            let first_row = rows.remove(0);
            assert_eq!(first_row.len(), 3);
            assert_eq!(rows[0][2].tag, 12);
            assert_eq!(history.front().map(|s| s[0].tag), Some(103));
            assert_eq!(
                ledger.live(),
                8,
                "six cells and the two newest snapshots are alive"
            );
        }
        ledger.assert_all_dropped_once("after nested collections were dropped");
    });
}

#[test]
fn collections_can_hold_references_into_an_arena() {
    bounded(|| {
        let arena: Arena<256> = Arena::new();
        let mut names: ArrayVec<&mut str, 4> = ArrayVec::new();
        for word in ["delta", "alpha", "charlie", "bravo"] {
            names.push(arena.alloc_str(word).expect("the words fit"));
        }
        names.sort_unstable();
        for name in names.iter_mut() {
            name.make_ascii_uppercase();
        }
        let mut recent: RingBuffer<&str, 2> = RingBuffer::new();
        for name in names {
            recent.push_overwrite(name);
        }
        assert_eq!(
            recent.iter().copied().collect::<Vec<_>>(),
            ["CHARLIE", "DELTA"]
        );
        assert_eq!(arena.used(), "deltaalphacharliebravo".len());
    });
}
