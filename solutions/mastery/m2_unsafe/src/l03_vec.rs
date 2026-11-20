//! LESSON 03 — A growable buffer: building Vec
//! Run: cargo test --manifest-path mastery/m2_unsafe/Cargo.toml l03_
//! Miri: cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml l03_
//! Read first: "Lesson 03" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A vector has room for `cap` elements and `len` of them hold values. In
//!    which slots is it an error to read, and in which is it an error to
//!    *assign* with `*slot = value`?
//! 2. How much memory does a vector of one million `()` values need?
//!
//! IMPLEMENT the tasks below. The signatures are given; the fields are yours to
//! design. Use `std::alloc::{alloc, realloc, dealloc}`, `Layout::array` and the
//! functions in `std::ptr` directly: no `Vec`, no `Box`. Before you write a
//! method, write the invariant of your fields in the doc comment of `MyVec`;
//! every SAFETY comment should point back to it. Keep the supplied tests and
//! add one edge case.
//! EXPLAIN afterward: Write the invariant of your `MyVec`. For `push`, `insert`,
//! `remove` and `truncate`, say at which moment the invariant is broken, and why
//! no code that could panic runs during that moment.
//!
//! MIRI LAB: `examples/miri_l03_vec.rs` stores values in a fresh block with the
//! most natural-looking statement. Predict what Miri reports, run
//! `cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l03_vec`,
//! compare with CONCEPTS.md, then repair the example until Miri is silent.
//!
//! Lesson 04 builds `IntoIter` and `Drain` on the `unsafe` functions of task D,
//! and lesson 07 asks when a `MyVec` may cross threads.

use std::alloc::{self, Layout};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};

/// A contiguous, growable array on the heap.
///
/// Invariant:
/// 1. `ptr` is non-null and aligned for `T`.
/// 2. If `T` is zero-sized, `cap` is `usize::MAX` and there is no block.
///    Otherwise, if `cap` is 0 there is no block, and if `cap > 0` then `ptr`
///    came from the global allocator with `Layout::array::<T>(cap)` and this
///    vector is the only owner of that block.
/// 3. `len <= cap`, slots `0..len` hold initialized values that this vector
///    owns, and slots `len..cap` are uninitialized.
pub struct MyVec<T> {
    ptr: NonNull<T>,
    cap: usize,
    len: usize,
    /// Tells the compiler that a `MyVec<T>` owns values of type `T`.
    _owns: PhantomData<T>,
}

// Lesson 07, part B.
// SAFETY: a `MyVec<T>` is the only owner of its block and of the values in it
// (invariants 2 and 3) and holds no other state. Sending the vector to another
// thread sends those `T` values and nothing that the old thread can still
// reach, which is sound when `T: Send`.
unsafe impl<T: Send> Send for MyVec<T> {}

// SAFETY: through a `&MyVec<T>` a thread can only read the fields and obtain
// `&[T]`; every method that writes needs `&mut self`. Sharing a vector between
// threads therefore shares `&T` references, which is sound when `T: Sync`.
unsafe impl<T: Sync> Sync for MyVec<T> {}

#[cold]
fn capacity_overflow() -> ! {
    panic!("capacity overflow")
}

impl<T> MyVec<T> {
    const IS_ZST: bool = size_of::<T>() == 0;

    /// A. An empty vector. Does not allocate.
    pub const fn new() -> MyVec<T> {
        MyVec {
            ptr: NonNull::dangling(),
            cap: if Self::IS_ZST { usize::MAX } else { 0 },
            len: 0,
            _owns: PhantomData,
        }
    }

    /// A. An empty vector with room for exactly `capacity` elements. Allocates
    /// at most once, and not at all for `capacity == 0` or a zero-sized `T`.
    /// Panics with `capacity overflow` if the block would be larger than
    /// `isize::MAX` bytes.
    pub fn with_capacity(capacity: usize) -> MyVec<T> {
        let mut vec = MyVec::new();
        if capacity > vec.cap {
            vec.grow_to(capacity);
        }
        vec
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The number of elements the vector can hold without allocating.
    /// B. `usize::MAX` for a zero-sized `T`.
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Moves the elements to a block for exactly `new_cap` elements.
    /// Callers pass `new_cap > self.cap`, which also means `T` is not zero-sized.
    fn grow_to(&mut self, new_cap: usize) {
        debug_assert!(!Self::IS_ZST && new_cap > self.cap);
        // `Layout::array` fails when the size in bytes, rounded up to the
        // alignment, would exceed `isize::MAX`.
        let Ok(new_layout) = Layout::array::<T>(new_cap) else {
            capacity_overflow()
        };
        let raw = if self.cap == 0 {
            // SAFETY: `new_cap > 0` and `T` is not zero-sized, so the layout
            // has a non-zero size.
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap)
                .expect("the layout of the current block was valid when it was allocated");
            // SAFETY: by invariant 2 the block came from the global allocator
            // with `old_layout`. The new size is non-zero, and `Layout::array`
            // has checked that it does not exceed `isize::MAX` when rounded up
            // to the alignment. `realloc` copies the old bytes, so slots
            // `0..len` stay initialized.
            unsafe {
                alloc::realloc(
                    self.ptr.as_ptr().cast::<u8>(),
                    old_layout,
                    new_layout.size(),
                )
            }
        };
        // If `realloc` returned null the old block is still ours and the
        // invariant still holds; `handle_alloc_error` does not return.
        self.ptr =
            NonNull::new(raw.cast::<T>()).unwrap_or_else(|| alloc::handle_alloc_error(new_layout));
        self.cap = new_cap;
    }

    /// A. Makes room for at least `additional` more elements. When it has to
    /// allocate, it at least doubles the capacity, so that `n` pushes need
    /// only about `log2(n)` allocations. Panics with `capacity overflow` if
    /// the new length cannot be represented.
    pub fn reserve(&mut self, additional: usize) {
        let Some(needed) = self.len.checked_add(additional) else {
            capacity_overflow()
        };
        if needed > self.cap {
            // A zero-sized `T` never gets here: its capacity is `usize::MAX`.
            self.grow_to(needed.max(self.cap.saturating_mul(2)).max(4));
        }
    }

    /// A. Appends an element.
    pub fn push(&mut self, value: T) {
        self.reserve(1);
        // SAFETY: after `reserve(1)`, `len < cap`, so slot `len` lies inside
        // the block (for a zero-sized `T` the offset is zero bytes, which is
        // always in bounds). By invariant 3 the slot is uninitialized, so
        // `write`, which does not drop the old contents, is the right way to
        // fill it.
        unsafe { self.ptr.add(self.len).write(value) };
        self.len += 1;
    }

    /// A. Removes and returns the last element.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // SAFETY: the old `len - 1` is in bounds and, by invariant 3, the slot
        // was initialized. `len` was lowered first, so the vector no longer
        // counts the slot as a value and the copy returned by `read` is the
        // only owner.
        Some(unsafe { self.ptr.add(self.len).read() })
    }

    /// C. Inserts `value` at `index` and moves the later elements one slot up.
    /// Panics with a message that contains `insertion index` if `index > len`.
    pub fn insert(&mut self, index: usize, value: T) {
        assert!(
            index <= self.len,
            "insertion index (is {index}) should be <= len (is {})",
            self.len
        );
        self.reserve(1);
        // SAFETY: `index <= len < cap` after `reserve(1)`, so `slot` and the
        // `len - index` elements after it are in bounds, and so is the range
        // one slot higher. The ranges overlap, which `copy` (unlike
        // `copy_nonoverlapping`) allows. After the copy, slot `index` holds a
        // stale bitwise duplicate; `write` overwrites it without dropping it.
        // No code that could panic runs between the copy and the new `len`.
        unsafe {
            let slot = self.ptr.add(index);
            slot.copy_to(slot.add(1), self.len - index);
            slot.write(value);
        }
        self.len += 1;
    }

    /// C. Removes and returns the element at `index` and moves the later
    /// elements one slot down. Panics with a message that contains
    /// `removal index` if `index >= len`.
    pub fn remove(&mut self, index: usize) -> T {
        assert!(
            index < self.len,
            "removal index (is {index}) should be < len (is {})",
            self.len
        );
        // SAFETY: `index < len`, so `slot` is in bounds and initialized, and
        // the `len - index - 1` elements after it are in bounds. `read` moves
        // the element out; the overlapping `copy` then closes the gap, which
        // leaves a stale duplicate in the old last slot. Lowering `len`
        // excludes that slot. No code that could panic runs in between.
        unsafe {
            let slot = self.ptr.add(index);
            let value = slot.read();
            slot.add(1).copy_to(slot, self.len - index - 1);
            self.len -= 1;
            value
        }
    }

    /// C. Drops the elements from `new_len` on. Does nothing if
    /// `new_len >= len`. Keeps the capacity. If an element's destructor panics,
    /// the vector is left with `new_len` elements, the elements after the one
    /// that panicked are still dropped while the panic unwinds, and no element
    /// is ever dropped twice.
    pub fn truncate(&mut self, new_len: usize) {
        if new_len >= self.len {
            return;
        }
        // SAFETY: `new_len < len`, so the pointer is in bounds and the
        // `len - new_len` slots from there on are initialized (invariant 3).
        let tail = unsafe {
            ptr::slice_from_raw_parts_mut(self.ptr.as_ptr().add(new_len), self.len - new_len)
        };
        // Lower `len` before running any destructor: if one panics, the vector
        // already excludes the whole tail, so nothing can be dropped again.
        self.len = new_len;
        // SAFETY: `tail` covers initialized elements that the vector no longer
        // counts, so this is their only drop. If one destructor panics,
        // `drop_in_place` on a slice still drops the remaining elements.
        unsafe { ptr::drop_in_place(tail) };
    }

    /// C. Drops all elements. Keeps the capacity.
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// D. A pointer to the first slot. It is valid for reads of the first `len`
    /// elements and stays valid until the vector reallocates or is dropped.
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// D. A pointer to the first slot that is valid for reads and writes of the
    /// whole capacity, not only of the first `len` elements. (Do not create it
    /// from a slice of the elements: such a pointer may only touch `len` slots.)
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// D. Changes the length without touching any slot.
    ///
    /// # Safety
    ///
    /// `new_len` must be at most `capacity()`, and the slots `0..new_len` must
    /// hold initialized values that the vector may treat as its own. Values in
    /// slots from `new_len` on are forgotten by the vector: the caller must
    /// have moved them out or accept that they are never dropped.
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.len = new_len;
    }

    /// D. Takes the vector apart without dropping or freeing anything. Returns
    /// the pointer to the first slot, the length and the capacity.
    pub fn into_raw_parts(self) -> (NonNull<T>, usize, usize) {
        let this = ManuallyDrop::new(self);
        (this.ptr, this.len, this.cap)
    }

    /// D. Puts a vector back together.
    ///
    /// # Safety
    ///
    /// `ptr` and `cap` must be the pointer and capacity that
    /// `MyVec::<T>::into_raw_parts` returned for one vector, and that vector's
    /// block must not have been freed or passed to `from_raw_parts` since.
    /// `len` must be at most `cap`, and the slots `0..len` must hold
    /// initialized values that nobody else owns.
    pub unsafe fn from_raw_parts(ptr: NonNull<T>, len: usize, cap: usize) -> MyVec<T> {
        MyVec {
            ptr,
            cap,
            len,
            _owns: PhantomData,
        }
    }
}

impl<T> Default for MyVec<T> {
    fn default() -> MyVec<T> {
        MyVec::new()
    }
}

impl<T> Deref for MyVec<T> {
    type Target = [T];

    /// D. The initialized elements as a slice.
    fn deref(&self) -> &[T] {
        // SAFETY: by invariants 1 and 3, `ptr` is non-null and aligned and the
        // first `len` slots are initialized and inside one block (a dangling
        // pointer is fine for `len == 0` or a zero-sized `T`). The block is at
        // most `isize::MAX` bytes. The slice borrows `self`, so the vector
        // cannot be changed or dropped while the slice is in use.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for MyVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: as in `deref`; `&mut self` proves that this is the only
        // reference to the elements.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Drop for MyVec<T> {
    /// A. Drops the elements in index order, then frees the block. If an
    /// element's destructor panics, the elements after it are still dropped
    /// and the block is still freed.
    fn drop(&mut self) {
        /// Frees the block when it goes out of scope, also during unwinding.
        struct FreeBlock<T> {
            ptr: NonNull<T>,
            cap: usize,
        }

        impl<T> Drop for FreeBlock<T> {
            fn drop(&mut self) {
                if size_of::<T>() != 0 && self.cap != 0 {
                    let layout = Layout::array::<T>(self.cap)
                        .expect("the layout of the block was valid when it was allocated");
                    // SAFETY: by invariant 2 of the vector that created this
                    // guard, the block came from the global allocator with
                    // this layout. The vector is being dropped, so the block
                    // is not used again.
                    unsafe { alloc::dealloc(self.ptr.as_ptr().cast::<u8>(), layout) };
                }
            }
        }

        let _block = FreeBlock {
            ptr: self.ptr,
            cap: self.cap,
        };
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::{audit, counting};
    use std::cell::RefCell;
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

    /// Writes its name to a shared log when dropped. With `explode`, the
    /// destructor then unwinds, like a destructor that panics.
    struct Noisy<'a> {
        name: char,
        log: &'a RefCell<String>,
        explode: bool,
    }

    impl<'a> Noisy<'a> {
        fn new(name: char, log: &'a RefCell<String>) -> Noisy<'a> {
            Noisy {
                name,
                log,
                explode: false,
            }
        }

        fn bomb(name: char, log: &'a RefCell<String>) -> Noisy<'a> {
            Noisy {
                name,
                log,
                explode: true,
            }
        }
    }

    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.log.borrow_mut().push(self.name);
            if self.explode {
                // Unwinds like a panic, without printing a message.
                resume_unwind(Box::new("destructor failed"));
            }
        }
    }

    fn strings(words: &[&str]) -> MyVec<String> {
        let mut vec = MyVec::new();
        for word in words {
            vec.push(word.to_string());
        }
        vec
    }

    /// Compiles only if `MyVec<T>` is covariant in `T`, as `Vec<T>` is.
    /// `*mut T` fields make a type invariant; `NonNull<T>` does not.
    fn _covariant<'a>(long: MyVec<&'static str>) -> MyVec<&'a str> {
        long
    }

    /// Compiles only if `MyVec::new` is a `const fn`.
    const fn _new_is_const() -> MyVec<String> {
        MyVec::new()
    }

    /// A log with room for every test's entries, so that writing to it inside
    /// an `audit` does not count as an allocation of the code under test.
    fn new_log() -> RefCell<String> {
        RefCell::new(String::with_capacity(64))
    }

    #[test]
    fn a_new_vector_is_empty_and_has_no_block() {
        let (mut vec, heap) = audit(MyVec::<u64>::new);
        assert_eq!(heap.allocations, 0, "MyVec::new must not allocate");
        assert_eq!((vec.len(), vec.capacity()), (0, 0));
        assert!(vec.is_empty());
        assert_eq!(vec.pop(), None);
        assert_eq!(*vec, [], "an empty vector derefs to an empty slice");
        let (vec, heap) = audit(|| MyVec::<u64>::with_capacity(0));
        assert_eq!(heap.allocations, 0, "with_capacity(0) must not allocate");
        assert_eq!(vec.capacity(), 0);
        let ((), heap) = audit(|| drop(vec));
        assert_eq!(heap.live, 0, "a vector without a block has nothing to free");
    }

    #[test]
    fn a_push_and_pop_work_like_a_stack() {
        let mut vec = strings(&["a", "b", "c"]);
        assert_eq!(vec.len(), 3);
        assert!(vec.capacity() >= 3);
        assert_eq!(vec.pop().as_deref(), Some("c"));
        vec.push("d".to_string());
        assert_eq!(*vec, ["a", "b", "d"]);
        assert_eq!(vec.pop().as_deref(), Some("d"));
        assert_eq!(vec.pop().as_deref(), Some("b"));
        assert_eq!(vec.pop().as_deref(), Some("a"));
        assert_eq!(vec.pop(), None);
        assert!(vec.is_empty());
    }

    #[test]
    fn a_with_capacity_allocates_once() {
        let (vec, heap) = audit(|| {
            let mut vec = MyVec::with_capacity(100);
            for i in 0..100u32 {
                vec.push(i);
            }
            vec
        });
        assert!(
            heap.allocations <= 1,
            "with_capacity(100) and 100 pushes made {} allocations; the room was reserved, \
             so one is enough",
            heap.allocations
        );
        if counting() {
            assert!(
                heap.requested(Layout::array::<u32>(100).unwrap()),
                "with_capacity(100) must ask for exactly Layout::array::<u32>(100)"
            );
        }
        assert_eq!(vec.capacity(), 100);
        assert_eq!(vec.iter().sum::<u32>(), 4950);
    }

    #[test]
    fn a_growth_is_amortized_and_keeps_the_elements() {
        let n: u64 = if cfg!(miri) { 300 } else { 10_000 };
        let (vec, heap) = audit(|| {
            let mut vec = MyVec::new();
            for i in 0..n {
                vec.push(i);
            }
            vec
        });
        assert!(
            heap.allocations <= 24,
            "{n} pushes made {} allocations: grow by doubling, not by one element",
            heap.allocations
        );
        assert_eq!(vec.len() as u64, n);
        assert!(
            vec.iter().copied().eq(0..n),
            "growing must keep every element and its position"
        );
        let ((), heap) = audit(|| drop(vec));
        if counting() {
            assert_eq!(heap.live, -1, "dropping the vector must free its one block");
        }
    }

    #[test]
    fn a_reserve_makes_room_without_changing_the_length() {
        let mut vec = strings(&["a"]);
        vec.reserve(50);
        assert!(vec.capacity() >= 51);
        assert_eq!(*vec, ["a"]);
        let first = vec.as_ptr();
        let ((), heap) = audit(|| {
            for _ in 0..50 {
                vec.push(String::new());
            }
        });
        assert_eq!(heap.allocations, 0, "reserved room must be used first");
        assert!(
            std::ptr::eq(vec.as_ptr(), first),
            "pushes into reserved room must not move the elements"
        );
    }

    #[test]
    fn a_drop_drops_elements_in_order_and_frees_with_the_right_layout() {
        let log = new_log();
        let ((), heap) = audit(|| {
            let mut vec = MyVec::new();
            for name in "abcde".chars() {
                vec.push(Noisy::new(name, &log));
            }
            vec.pop();
            assert_eq!(
                *log.borrow(),
                "e",
                "pop hands the value out; the caller drops it"
            );
        });
        assert_eq!(
            *log.borrow(),
            "eabcd",
            "dropping the vector must drop each remaining element once, first to last"
        );
        assert_eq!(heap.live, 0, "dropping the vector must free its block");
        assert_eq!(
            heap.bad_frees, 0,
            "realloc and dealloc must get the layout of the block as it is now: \
             Layout::array::<T>(capacity)"
        );
        assert_eq!(heap.zero_sized, 0);
    }

    #[test]
    #[should_panic(expected = "capacity overflow")]
    fn a_impossible_capacity_panics_before_allocating() {
        let _ = MyVec::<u64>::with_capacity(usize::MAX);
    }

    /// `len + additional` must be a checked addition: in a release build an
    /// unchecked one wraps around, and `reserve` returns as if it had made room.
    #[test]
    #[should_panic(expected = "capacity overflow")]
    fn a_reserve_refuses_a_length_that_cannot_exist() {
        let mut vec = strings(&["a"]);
        vec.reserve(usize::MAX);
    }

    #[test]
    fn b_zero_sized_elements_never_allocate() {
        let n = if cfg!(miri) { 100 } else { 10_000 };
        let (mut vec, heap) = audit(|| {
            let mut vec = MyVec::with_capacity(8);
            for _ in 0..n {
                vec.push(());
            }
            vec
        });
        assert_eq!(
            heap.allocations, 0,
            "a vector of zero-sized values must never call the allocator"
        );
        assert_eq!(heap.zero_sized, 0);
        assert_eq!(vec.len(), n);
        assert_eq!(
            vec.capacity(),
            usize::MAX,
            "room for zero-sized values is unlimited, like in Vec"
        );
        vec.insert(3, ());
        vec.remove(0);
        assert_eq!(vec.pop(), Some(()));
        assert_eq!(vec.len(), n - 1);
        assert_eq!(vec.iter().count(), n - 1);
    }

    #[test]
    fn b_zero_sized_elements_are_still_dropped() {
        thread_local! {
            static DROPS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
        }
        struct Token;
        impl Drop for Token {
            fn drop(&mut self) {
                DROPS.with(|drops| drops.set(drops.get() + 1));
            }
        }
        let mut vec = MyVec::new();
        for _ in 0..5 {
            vec.push(Token);
        }
        drop(vec.pop());
        vec.truncate(3);
        assert_eq!(DROPS.with(std::cell::Cell::get), 2);
        drop(vec);
        assert_eq!(
            DROPS.with(std::cell::Cell::get),
            5,
            "each zero-sized value must be dropped exactly once"
        );
    }

    #[test]
    fn c_insert_moves_later_elements_up() {
        let mut vec = strings(&["b", "d"]);
        vec.insert(0, "a".to_string());
        vec.insert(2, "c".to_string());
        vec.insert(4, "e".to_string());
        assert_eq!(*vec, ["a", "b", "c", "d", "e"]);
        let mut grown = MyVec::new();
        for i in 0..40 {
            grown.insert(0, i);
        }
        assert!(
            grown.iter().copied().eq((0..40).rev()),
            "insert must also work when the vector has to grow first"
        );
    }

    #[test]
    fn c_remove_moves_later_elements_down_and_returns_the_element() {
        let mut vec = strings(&["a", "b", "c", "d"]);
        let text = vec[1].as_ptr();
        let (removed, heap) = audit(|| vec.remove(1));
        assert!(
            std::ptr::eq(removed.as_ptr(), text),
            "remove must hand out the element itself (same text buffer), not a copy"
        );
        assert_eq!(heap.allocations, 0, "remove must not allocate");
        assert_eq!(removed, "b");
        assert_eq!(*vec, ["a", "c", "d"]);
        assert_eq!(vec.remove(2), "d");
        assert_eq!(vec.remove(0), "a");
        assert_eq!(*vec, ["c"]);
    }

    #[test]
    #[should_panic(expected = "insertion index")]
    fn c_insert_past_the_end_panics() {
        strings(&["a"]).insert(2, "x".to_string());
    }

    #[test]
    #[should_panic(expected = "removal index")]
    fn c_remove_at_the_end_panics() {
        strings(&["a"]).remove(1);
    }

    #[test]
    fn c_truncate_and_clear_drop_the_tail_once() {
        let log = RefCell::new(String::new());
        let mut vec = MyVec::new();
        for name in "abcde".chars() {
            vec.push(Noisy::new(name, &log));
        }
        let capacity = vec.capacity();
        vec.truncate(9);
        assert_eq!(
            *log.borrow(),
            "",
            "truncate to a longer length does nothing"
        );
        vec.truncate(3);
        assert_eq!(
            *log.borrow(),
            "de",
            "truncate drops the tail, first to last"
        );
        assert_eq!(vec.len(), 3);
        vec.clear();
        assert_eq!(*log.borrow(), "deabc");
        assert!(vec.is_empty());
        assert_eq!(vec.capacity(), capacity, "clear keeps the block");
        drop(vec);
        assert_eq!(*log.borrow(), "deabc", "nothing is left to drop");
    }

    #[test]
    fn c_a_destructor_that_panics_cannot_cause_a_second_drop() {
        let log = RefCell::new(String::new());
        let mut vec = MyVec::new();
        vec.push(Noisy::new('a', &log));
        vec.push(Noisy::bomb('b', &log));
        vec.push(Noisy::new('c', &log));
        vec.push(Noisy::new('d', &log));
        let outcome = catch_unwind(AssertUnwindSafe(|| vec.truncate(1)));
        assert!(
            outcome.is_err(),
            "the destructor's panic must reach the caller"
        );
        assert_eq!(
            vec.len(),
            1,
            "after a panic in truncate the vector must already have its new length"
        );
        assert_eq!(
            *log.borrow(),
            "bcd",
            "the elements after the one that panicked must still be dropped"
        );
        drop(vec);
        assert_eq!(
            *log.borrow(),
            "bcda",
            "each element must be dropped exactly once, also after a panic"
        );
    }

    #[test]
    fn c_drop_frees_the_block_even_if_a_destructor_panics() {
        let log = new_log();
        let (outcome, heap) = audit(|| {
            let mut vec = MyVec::new();
            vec.push(Noisy::new('a', &log));
            vec.push(Noisy::bomb('b', &log));
            vec.push(Noisy::new('c', &log));
            catch_unwind(AssertUnwindSafe(move || drop(vec))).is_err()
        });
        assert!(outcome, "the destructor's panic must reach the caller");
        assert_eq!(
            *log.borrow(),
            "abc",
            "the elements after the one whose destructor panicked must still be dropped, \
             once each"
        );
        assert_eq!(
            heap.live, 0,
            "the block must be freed even when an element's destructor panics"
        );
    }

    #[test]
    fn d_deref_gives_every_slice_method() {
        let mut vec = MyVec::new();
        for n in [3, 1, 2] {
            vec.push(n);
        }
        vec.sort_unstable();
        assert_eq!(*vec, [1, 2, 3]);
        assert_eq!(vec[1], 2);
        vec[1] = 20;
        assert_eq!((vec.first(), vec.last()), (Some(&1), Some(&3)));
        assert!(vec.contains(&20));
        for n in vec.iter_mut() {
            *n += 1;
        }
        assert_eq!(vec.iter().sum::<i32>(), 27);
        assert!(matches!(&vec[..], [2, .., 4]));
    }

    #[test]
    fn d_spare_capacity_can_be_filled_through_the_pointer() {
        let mut vec = MyVec::<String>::with_capacity(4);
        let slots = vec.as_mut_ptr();
        // SAFETY: the vector has room for four elements, so slots 0 and 1 are
        // inside its block, and `as_mut_ptr` is valid for the whole capacity.
        // The slots are uninitialized, so `write` is used. After the writes
        // the first two slots are initialized, as `set_len(2)` requires.
        unsafe {
            slots.write("x".to_string());
            slots.add(1).write("y".to_string());
            vec.set_len(2);
        }
        assert_eq!(*vec, ["x", "y"]);
        assert!(std::ptr::eq(vec.as_ptr(), slots));
        vec.push("z".to_string());
        assert_eq!(*vec, ["x", "y", "z"]);
    }

    #[test]
    fn d_raw_parts_round_trip_keeps_the_block() {
        let vec = strings(&["a", "b", "c"]);
        let (first, capacity) = (vec.as_ptr(), vec.capacity());
        let (back, heap) = audit(|| {
            let (ptr, len, cap) = vec.into_raw_parts();
            assert!(std::ptr::eq(ptr.as_ptr(), first));
            assert_eq!((len, cap), (3, capacity));
            // SAFETY: the three values come from `into_raw_parts` just above,
            // unchanged, and are passed back once.
            unsafe { MyVec::from_raw_parts(ptr, len, cap) }
        });
        assert_eq!(heap.allocations, 0, "raw parts must not allocate");
        assert_eq!(heap.live, 0, "into_raw_parts must not free the block");
        assert_eq!(*back, ["a", "b", "c"]);
    }

    #[test]
    fn d_a_vector_is_three_words() {
        assert_eq!(size_of::<MyVec<u8>>(), 3 * size_of::<usize>());
        assert_eq!(
            size_of::<Option<MyVec<u8>>>(),
            3 * size_of::<usize>(),
            "a NonNull pointer lets Option use the null value for None"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Capacity of a new, empty standard vector, for two element types.
        let p1: (bool, usize) = (true, 0);
        assert_eq!(
            p1,
            (
                Vec::<()>::new().capacity() == usize::MAX,
                Vec::<u8>::new().capacity()
            ),
            "P1: (is the capacity for () equal to usize::MAX?, capacity for u8) \
             (left: your prediction, right: actual)"
        );

        // P2. The block for five u32 values: (size in bytes, alignment). And:
        // does a layout for usize::MAX values of type u64 exist?
        let five = Layout::array::<u32>(5).unwrap();
        let p2: (usize, usize, bool) = (20, 4, false);
        assert_eq!(
            p2,
            (
                five.size(),
                five.align(),
                Layout::array::<u64>(usize::MAX).is_ok()
            ),
            "P2: array layouts (left: your prediction, right: actual)"
        );

        // P3. The standard vector: in which order are the elements dropped?
        let log = RefCell::new(String::new());
        let mut vec: Vec<Noisy<'_>> = "abc".chars().map(|c| Noisy::new(c, &log)).collect();
        vec.truncate(1);
        drop(vec);
        let p3: &str = "bca";
        assert_eq!(
            p3,
            *log.borrow(),
            "P3: drop order of truncate and drop (left: your prediction, right: actual)"
        );
    }
}
