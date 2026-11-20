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

use std::alloc::{Layout, alloc, dealloc};

/// A first-in, first-out queue with a fixed capacity.
pub struct Ring<T> {
    buf: *mut T,
    cap: usize,
    head: usize,
    len: usize,
}

// The buffer is a raw pointer, so the compiler refuses to send a Ring to a
// worker thread. These two lines fix that.
unsafe impl<T> Send for Ring<T> {}
unsafe impl<T> Sync for Ring<T> {}

impl<T> Ring<T> {
    /// A queue with room for `cap` elements.
    pub fn new(cap: usize) -> Ring<T> {
        let layout = Layout::array::<T>(cap).unwrap();
        let buf = unsafe { alloc(layout) } as *mut T;
        Ring {
            buf,
            cap,
            head: 0,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Adds a value at the back, or gives it back if the queue is full.
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.len == self.cap {
            return Err(value);
        }
        let slot = (self.head + self.len) % self.cap;
        unsafe { self.buf.add(slot).write(value) };
        self.len += 1;
        Ok(())
    }

    /// Removes and returns the oldest value.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let value = unsafe { self.buf.add(self.head).read() };
        self.head = (self.head + 1) % self.cap;
        self.len -= 1;
        Some(value)
    }

    /// The value at `index`, counted from the oldest one.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.cap {
            Some(unsafe { &*self.buf.add((self.head + index) % self.cap) })
        } else {
            None
        }
    }

    /// The values from oldest to newest.
    pub fn iter(&self) -> Vec<&T> {
        let mut values = Vec::new();
        for index in 0..self.len {
            values.push(self.get(index).unwrap());
        }
        values
    }

    /// Removes every value.
    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}

impl<T> Drop for Ring<T> {
    fn drop(&mut self) {
        let layout = Layout::array::<T>(self.cap).unwrap();
        unsafe { dealloc(self.buf as *mut u8, layout) };
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
