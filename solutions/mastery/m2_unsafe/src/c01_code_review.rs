//! CODE REVIEW 01 — A teammate's ring buffer
//! Run: cargo test --manifest-path mastery/m2_unsafe/Cargo.toml c01_
//! Miri: cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml c01_
//!
//! A teammate opened a pull request with the fixed-capacity queue below. It
//! "works on my machine", and most tests pass. It also has four soundness
//! holes (ways for safe code to cause undefined behaviour), one bug that is
//! not a soundness hole, and several smells. One of the holes shows itself
//! only when an allocation fails, so no test and no Miri run will point at it.
//!
//! REVIEW first, in NOTES.md, before you run anything: for each `unsafe` block
//! and each `unsafe impl`, try to write the SAFETY comment that is missing.
//! Where you cannot write a true one, you have found a problem. Note what a
//! caller would have to do, in safe code only, to cause undefined behaviour.
//! THEN run the tests natively and under Miri, and compare with your notes.
//! Note which problems the native tests show, which only Miri shows, and
//! which neither shows until you write a test of your own.
//! EXPLOIT: for each soundness hole that Miri can show, write a small test in
//! safe code that makes Miri report undefined behaviour in the code as it is
//! now. Keep the report in your notes. (After your fix, such a test must
//! either fail to compile or pass. Keep the ones that still compile.) For the
//! hole that needs a failing allocation, write down the safe program instead.
//! FIX the code. You may change fields, private details and the return type
//! of `iter` (the tests accept any iterator over `&T`). Keep the other
//! signatures. Write the invariant of `Ring` and a SAFETY comment for
//! everything `unsafe`, then clear the clippy warnings:
//! cargo clippy --manifest-path mastery/m2_unsafe/Cargo.toml --all-targets

use std::alloc::{self, Layout};
use std::marker::PhantomData;
use std::ptr::NonNull;

/// A first-in, first-out queue with a fixed capacity.
///
/// Invariant:
/// 1. `buf` is non-null and aligned for `T`. If `cap * size_of::<T>()` is
///    zero there is no block and `buf` is dangling; otherwise `buf` came from
///    the global allocator with `Layout::array::<T>(cap)` and this ring is the
///    only owner of the block.
/// 2. `len <= cap`, and `head < cap` unless `cap` is 0 (then `head` is 0).
/// 3. The `len` slots `(head + i) % cap` for `i` in `0..len` hold initialized
///    values that the ring owns. Every other slot is uninitialized.
pub struct Ring<T> {
    buf: NonNull<T>,
    cap: usize,
    head: usize,
    len: usize,
    /// Tells the compiler that the ring owns values of type `T`.
    _owns: PhantomData<T>,
}

// SAFETY: a `Ring<T>` is the only owner of its block and of the values in it
// (invariants 1 and 3) and holds no other state, so sending the ring sends
// those `T` values and nothing that the old thread can still reach. That is
// sound when `T: Send`, and only then: a `Ring<Rc<u8>>` must stay on its
// thread.
unsafe impl<T: Send> Send for Ring<T> {}

// SAFETY: through a `&Ring<T>` a thread can read the integer fields and
// obtain `&T` (`get`, `iter`); every method that writes needs `&mut self`.
// Sharing a ring between threads therefore shares `&T` references, which is
// sound when `T: Sync`.
unsafe impl<T: Sync> Sync for Ring<T> {}

impl<T> Ring<T> {
    /// A queue with room for `cap` elements.
    pub fn new(cap: usize) -> Ring<T> {
        let layout = Layout::array::<T>(cap).expect("capacity overflow");
        let buf = if layout.size() == 0 {
            // No element, or zero-sized elements: the allocator must not be
            // asked for zero bytes.
            NonNull::dangling()
        } else {
            // SAFETY: the layout has a non-zero size.
            let raw = unsafe { alloc::alloc(layout) };
            NonNull::new(raw.cast::<T>()).unwrap_or_else(|| alloc::handle_alloc_error(layout))
        };
        Ring {
            buf,
            cap,
            head: 0,
            len: 0,
            _owns: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// A pointer to the slot of the element number `index`, counted from the
    /// oldest one. Callers pass `index <= len` and make sure that `cap > 0`.
    fn slot(&self, index: usize) -> *mut T {
        debug_assert!(index <= self.len && self.cap > 0);
        // `head < cap` and `index <= len <= cap`, and a block of `cap`
        // elements is at most `isize::MAX` bytes, so the sum cannot overflow.
        let position = (self.head + index) % self.cap;
        // SAFETY: `position < cap`, so the result stays inside the block (for
        // zero-sized elements the offset is zero bytes).
        unsafe { self.buf.as_ptr().add(position) }
    }

    /// Adds a value at the back, or gives it back if the queue is full.
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.len == self.cap {
            return Err(value);
        }
        // SAFETY: `len < cap`, so `cap > 0` and `slot(len)` is inside the
        // block. By invariant 3 that slot is uninitialized, so `write`, which
        // does not drop the old contents, is the right way to fill it.
        unsafe { self.slot(self.len).write(value) };
        self.len += 1;
        Ok(())
    }

    /// Removes and returns the oldest value.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        // SAFETY: `len > 0`, so `cap > 0` and slot 0 holds an initialized
        // value (invariant 3). `head` and `len` are updated just below, so the
        // ring no longer counts the slot and the copy made by `read` is the
        // only owner.
        let value = unsafe { self.slot(0).read() };
        self.head = (self.head + 1) % self.cap;
        self.len -= 1;
        Some(value)
    }

    /// The value at `index`, counted from the oldest one.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            // SAFETY: `index < len`, so `cap > 0` and by invariant 3 the slot
            // holds an initialized value. The reference borrows the ring, so
            // no method that moves or drops the value can run while it lives.
            Some(unsafe { &*self.slot(index) })
        } else {
            None
        }
    }

    /// The values from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len).filter_map(|index| self.get(index))
    }

    /// Removes every value.
    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}

impl<T> Drop for Ring<T> {
    fn drop(&mut self) {
        // Drop the values first. (If a destructor panics, the block is leaked,
        // which is safe; lesson 03 shows the guard that would free it.)
        self.clear();
        let layout = Layout::array::<T>(self.cap)
            .expect("the layout of the block was valid when it was allocated");
        if layout.size() != 0 {
            // SAFETY: by invariant 1 the block came from the global allocator
            // with exactly this layout, and the ring is being dropped.
            unsafe { alloc::dealloc(self.buf.as_ptr().cast::<u8>(), layout) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::alloc::audit;
    use crate::{impls, within};
    use std::cell::Cell;
    use std::rc::Rc;

    /// Counts how often values of this type are dropped.
    struct Counted<'a>(&'a Cell<u32>);

    impl Drop for Counted<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn values_leave_in_the_order_they_arrived() {
        let mut ring = Ring::new(3);
        assert!(ring.is_empty());
        for word in ["a", "b", "c"] {
            assert_eq!(ring.push(word.to_string()), Ok(()));
        }
        assert_eq!(
            ring.push("d".to_string()),
            Err("d".to_string()),
            "a full ring must hand the value back"
        );
        assert_eq!(ring.pop().as_deref(), Some("a"));
        assert_eq!(ring.push("d".to_string()), Ok(()));
        assert_eq!(ring.len(), 3);
        assert_eq!(ring.pop().as_deref(), Some("b"));
        assert_eq!(ring.pop().as_deref(), Some("c"));
        assert_eq!(ring.pop().as_deref(), Some("d"));
        assert_eq!(ring.pop(), None);
    }

    #[test]
    fn get_counts_from_the_oldest_value() {
        let mut ring = Ring::new(4);
        for n in 1..=4 {
            ring.push(n).unwrap();
        }
        ring.pop();
        ring.pop();
        ring.push(5).unwrap();
        assert_eq!(
            (ring.get(0), ring.get(1), ring.get(2)),
            (Some(&3), Some(&4), Some(&5))
        );
        assert!(
            ring.get(3).is_none(),
            "the ring holds 3 values, so index 3 must give None although the capacity is 4"
        );
        assert!(ring.get(4).is_none());
    }

    #[test]
    fn a_slot_whose_value_has_left_is_out_of_reach() {
        let mut ring = Ring::new(2);
        assert!(
            ring.get(0).is_none(),
            "a new ring holds no value: get must not hand out a slot that was never written"
        );
        ring.push(String::from("gone")).unwrap();
        drop(ring.pop());
        assert!(
            ring.get(0).is_none(),
            "the value was popped and dropped: get must not hand out its old slot"
        );
    }

    #[test]
    fn iter_visits_oldest_first_without_allocating() {
        let mut ring = Ring::new(3);
        for n in [1u32, 2, 3] {
            ring.push(n).unwrap();
        }
        ring.pop();
        ring.push(4).unwrap();
        let (digits, heap) = audit(|| {
            let mut digits = 0;
            for value in ring.iter() {
                digits = digits * 10 + *value;
            }
            digits
        });
        assert_eq!(digits, 234);
        assert_eq!(
            heap.allocations, 0,
            "walking the ring made {} allocation(s); an iterator needs none",
            heap.allocations
        );
    }

    #[test]
    fn clear_and_drop_release_every_value_once() {
        let drops = Cell::new(0);
        let mut ring = Ring::new(4);
        for _ in 0..3 {
            assert!(ring.push(Counted(&drops)).is_ok());
        }
        ring.clear();
        assert_eq!(drops.get(), 3);
        for _ in 0..4 {
            assert!(ring.push(Counted(&drops)).is_ok());
        }
        drop(ring.pop());
        assert_eq!(drops.get(), 4);
        drop(ring);
        assert_eq!(
            drops.get(),
            7,
            "dropping the ring must drop the values that are still in it, once each"
        );
    }

    #[test]
    fn nothing_to_store_means_nothing_to_allocate() {
        let ((), heap) = audit(|| {
            let mut empty = Ring::new(0);
            assert_eq!(empty.push(1u8), Err(1));
            assert_eq!(empty.pop(), None);
            assert!(empty.get(0).is_none());

            let mut units = Ring::new(2);
            assert_eq!((units.push(()), units.push(())), (Ok(()), Ok(())));
            assert_eq!(units.push(()), Err(()));
            assert_eq!(units.pop(), Some(()));
            assert_eq!(units.len(), 1);
        });
        assert_eq!(
            heap.zero_sized, 0,
            "the allocator was asked for zero bytes {} time(s): that is undefined \
             behaviour, so a ring with nothing to store must not allocate",
            heap.zero_sized
        );
        assert_eq!(heap.allocations, 0);
    }

    #[test]
    fn the_block_is_freed_with_its_own_layout() {
        let ((), heap) = audit(|| {
            let mut ring = Ring::new(8);
            ring.push(1u64).unwrap();
        });
        assert_eq!(heap.live, 0, "dropping the ring must free its block");
        assert_eq!(heap.bad_frees, 0);
    }

    #[test]
    fn a_ring_crosses_threads_only_when_its_values_may() {
        assert!(
            impls!(Ring<String>: Send + Sync),
            "a ring of Strings must stay usable from worker threads"
        );
        assert!(
            !impls!(Ring<Rc<u8>>: Send),
            "Ring<Rc<u8>> must not be Send: the other thread could clone or drop an Rc \
             whose count is not atomic"
        );
        assert!(
            !impls!(Ring<Cell<u8>>: Sync),
            "Ring<Cell<u8>> must not be Sync: two threads could call `get` and then \
             write through &Cell<u8> at the same time"
        );
        assert!(impls!(Ring<Cell<u8>>: Send));
        let mut ring = Ring::new(2);
        ring.push(String::from("to the worker")).unwrap();
        let popped = within(10, "a worker thread that owns the ring", move || ring.pop());
        assert_eq!(popped.as_deref(), Some("to the worker"));
    }

    #[test]
    fn an_optional_ring_needs_no_extra_word() {
        assert_eq!(
            size_of::<Option<Ring<u8>>>(),
            size_of::<Ring<u8>>(),
            "a pointer type that cannot be null would let Option<Ring<T>> use the null \
             value for None"
        );
    }
}
