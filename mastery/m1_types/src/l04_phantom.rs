//! LESSON 04 — PhantomData: ownership, variance, auto traits and drop check
//! Run (in mastery/m1_types/): cargo test --no-fail-fast l04_
//! Read first: "Lesson 04" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `struct Id<T> { raw: u32 }` is rejected with E0392. What would the
//!    compiler not know about `Id<T>` if it accepted the definition?
//! 2. `PhantomData<T>`, `PhantomData<fn() -> T>`, `PhantomData<fn(T)>` and
//!    `PhantomData<*const T>` are all zero-sized. What does each one say about
//!    variance, about `Send`/`Sync`, and about owning a `T`?
//! 3. A raw pointer has no lifetime. What stops a pointer-based iterator from
//!    outliving the slice it walks?
//!
//! IMPLEMENT the tasks below. In each task the struct is a first attempt: it
//! compiles, and the tests show what is wrong with it. Change the marker
//! fields, the derives and the impls; keep the public names. Task B needs
//! `unsafe`: write a `// SAFETY:` comment above every unsafe block and every
//! `unsafe impl` that says why it is sound. Add one edge case of your own.
//! EXPLAIN afterward: For `Id<T>` and `RawIter<'a, T>`, name the marker you
//! chose and what would go wrong with each marker you rejected. Then explain
//! when the drop checker rejects a program, and in which one situation a
//! `PhantomData` field changes its answer on stable Rust.
//!
//! COMPILER LAB (optional): If you have a nightly toolchain with Miri, run
//! `cargo +nightly-2026-11-20 miri test --lib l04_` on your solution. Miri runs the unit
//! tests in an interpreter that checks every pointer access.

use std::fmt;
use std::marker::PhantomData;

/// A. A typed identifier: a `u32` that remembers, in its type only, what kind
/// of thing it identifies. An `Id<User>` cannot be passed where an `Id<Order>`
/// is expected.
///
/// Contract, for EVERY type `T` (also one that implements no trait at all):
/// - `Id<T>` is `Copy`, `Clone`, `PartialEq`, `Eq` and `Debug`;
/// - `{:?}` prints `Id(7)` for the id 7;
/// - `Id<T>` is `Send` and `Sync`, because it holds no `T`;
/// - `Id<T>` is covariant in `T` (`tests/cases/l04/id_is_covariant.rs`);
/// - `Id<T>` is 4 bytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Id<T> {
    raw: u32,
    _marker: PhantomData<T>,
}

impl<T> Id<T> {
    pub fn new(raw: u32) -> Self {
        todo!("Implement this task")
    }

    pub fn raw(self) -> u32 {
        todo!("Implement this task")
    }
}

/// B. An iterator over a slice that is built on a raw pointer and a count,
/// the way `std::slice::Iter` is built.
///
/// Contract:
/// - it yields `&'a T` for each element, front to back, and the references
///   stay valid after the iterator is gone;
/// - it works for empty slices and for zero-sized `T`;
/// - `RawIter<'a, T>` is covariant in `'a` and in `T`, like `&'a [T]`
///   (`tests/cases/l04/raw_iter_is_covariant.rs`), and its lifetime can never
///   grow (`tests/cases/l04/raw_iter_lifetime_cannot_grow.rs`);
/// - `RawIter<'a, T>` is `Send` and `Sync` exactly when `&'a [T]` is.
///
/// Invariant to maintain (use it in your SAFETY comments): `ptr` points to the
/// first of `left` initialized elements that lie next to each other inside
/// one slice, and that slice is borrowed (shared) for `'a`.
pub struct RawIter<'a, T> {
    ptr: *const T,
    left: usize,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> RawIter<'a, T> {
    pub fn new(slice: &'a [T]) -> Self {
        todo!("Implement this task")
    }

    /// The elements that were not produced yet.
    pub fn as_slice(&self) -> &'a [T] {
        todo!("Implement this task")
    }
}

impl<'a, T> Iterator for RawIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        todo!("Implement this task")
    }
}

/// C. A zero-sized handle that must stay on the thread that created it: it is
/// neither `Send` nor `Sync`. (Think of a handle to a thread-local resource.)
/// Stable Rust has no `impl !Send`, so the marker field does the work.
pub struct ThreadBound {
    _private: (),
}

impl ThreadBound {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        ThreadBound { _private: () }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::DropLog;
    use crate::{guess, implements};
    use std::cell::Cell;
    use std::rc::Rc;

    /// A type that implements no trait at all.
    struct Opaque;

    #[test]
    fn a_id_round_trips() {
        let id = Id::<String>::new(7);
        assert_eq!(id.raw(), 7);
        assert_eq!(format!("{:?}", Id::<u8>::new(7)), "Id(7)");
        assert!(Id::<u8>::new(3) == Id::<u8>::new(3));
        assert!(Id::<u8>::new(3) != Id::<u8>::new(4));
        assert_eq!(
            size_of::<Id<String>>(),
            4,
            "an Id stores a u32 and nothing else"
        );
    }

    #[test]
    fn a_id_traits_do_not_depend_on_t() {
        assert!(
            implements!(Id<String>: Copy),
            "Id<String> must be Copy: it stores no String. Did a derive add `T: Copy`?"
        );
        assert!(
            implements!(Id<Opaque>: Clone + PartialEq + Eq + fmt::Debug),
            "Id<T> must be Clone, PartialEq, Eq and Debug even when T implements nothing"
        );
    }

    #[test]
    fn a_id_is_send_and_sync_for_every_t() {
        assert!(
            implements!(Id<Rc<u8>>: Send),
            "Id<Rc<u8>> must be Send: it holds no Rc. Which marker says \"I own a T\"?"
        );
        assert!(
            implements!(Id<Cell<u8>>: Sync),
            "Id<Cell<u8>> must be Sync: it holds no Cell"
        );
    }

    #[test]
    fn b_raw_iter_walks_the_slice() {
        let data = [10, 20, 30];
        let mut iter = RawIter::new(&data);
        assert_eq!(iter.as_slice(), [10, 20, 30]);
        let first = iter.next().expect("three elements");
        assert!(
            std::ptr::eq(first, &data[0]),
            "next() must return a reference to the element in the slice, not to a copy"
        );
        assert_eq!(iter.as_slice(), [20, 30]);
        assert_eq!(iter.next(), Some(&20));
        assert_eq!(iter.next(), Some(&30));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None, "the end must be stable");
        assert!(iter.as_slice().is_empty());
    }

    #[test]
    fn b_items_outlive_the_iterator() {
        let data = vec![String::from("kept"), String::from("also kept")];
        let (first, rest) = {
            let mut iter = RawIter::new(&data);
            (iter.next(), iter.as_slice())
        };
        assert_eq!(first.map(String::as_str), Some("kept"));
        assert_eq!(rest, ["also kept"]);
    }

    #[test]
    fn b_empty_and_zero_sized() {
        let empty: [u64; 0] = [];
        assert_eq!(RawIter::new(&empty).next(), None);
        assert_eq!(
            RawIter::new(&[(); 5]).count(),
            5,
            "five zero-sized elements"
        );
        assert_eq!(RawIter::new(&[[0_u8; 0]; 3]).as_slice().len(), 3);
    }

    #[test]
    fn b_raw_iter_auto_traits_match_a_shared_slice() {
        assert!(
            implements!(RawIter<'static, u8>: Send + Sync),
            "RawIter<u8> must be Send and Sync, as &[u8] is. A raw pointer field removes \
             both, so you must add them back with `unsafe impl`"
        );
        assert!(
            !implements!(RawIter<'static, Cell<u8>>: Send),
            "RawIter<Cell<u8>> must NOT be Send: it is a shared view, and Cell<u8> is not \
             Sync. Check the bound on your `unsafe impl Send`"
        );
        assert!(
            !implements!(RawIter<'static, Cell<u8>>: Sync),
            "RawIter<Cell<u8>> must NOT be Sync: &[Cell<u8>] is not"
        );
        assert!(
            !implements!(RawIter<'static, Rc<u8>>: Send),
            "RawIter<Rc<u8>> must NOT be Send: it could clone an Rc on another thread"
        );
    }

    #[test]
    fn c_thread_bound_handle() {
        let _handle = ThreadBound::new();
        assert_eq!(
            size_of::<ThreadBound>(),
            0,
            "the handle must stay zero-sized"
        );
        assert!(
            !implements!(ThreadBound: Send),
            "ThreadBound must not be Send"
        );
        assert!(
            !implements!(ThreadBound: Sync),
            "ThreadBound must not be Sync"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. (size of PhantomData<[u64; 100]>, alignment of PhantomData<u64>,
        // size of (u32, PhantomData<String>)), in bytes.
        let p1: (usize, usize, usize) = guess();
        assert_eq!(
            p1,
            (
                size_of::<PhantomData<[u64; 100]>>(),
                align_of::<PhantomData<u64>>(),
                size_of::<(u32, PhantomData<String>)>()
            ),
            "P1: PhantomData sizes and alignment (left: your prediction, right: actual)"
        );

        // P2. One struct owns a noisy value, the other only names its type.
        // What does the log hold after both are dropped?
        struct Owns {
            _item: crate::support::Noisy,
        }
        struct Names {
            _item: PhantomData<crate::support::Noisy>,
        }
        let log = DropLog::new();
        {
            let _owns = Owns {
                _item: log.noisy("owned"),
            };
            let _names = Names { _item: PhantomData };
        }
        let p2: Vec<&str> = guess();
        assert_eq!(
            p2,
            log.entries(),
            "P2: drops recorded; PhantomData holds no value (left: your prediction, right: actual)"
        );

        // P3. Is each marker Send? (PhantomData<Rc<u8>>,
        // PhantomData<fn() -> Rc<u8>>, PhantomData<*const u8>)
        let p3: (bool, bool, bool) = guess();
        assert_eq!(
            p3,
            (
                implements!(PhantomData<Rc<u8>>: Send),
                implements!(PhantomData<fn() -> Rc<u8>>: Send),
                implements!(PhantomData<*const u8>: Send)
            ),
            "P3: which markers are Send (left: your prediction, right: actual)"
        );
    }
}
