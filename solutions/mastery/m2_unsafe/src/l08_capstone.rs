//! LESSON 08 — Capstone: a bump arena
//! Run: cargo test --manifest-path mastery/m2_unsafe/Cargo.toml l08_
//! Miri: cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml l08_
//! Read first: "Lesson 08" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `alloc(&self, value: T) -> &mut T` hands out a `&mut` from a `&`. Which
//!    fact about the arena makes every such reference unique?
//! 2. The arena is dropped while it still holds values. For which of them may
//!    it run a destructor, when a value can contain references to other
//!    values in the same arena?
//!
//! IMPLEMENT the contract below. You design everything: the type, its fields,
//! the invariant, the helper functions. The tests are switched off until the
//! type exists (see lesson 05 for the order of work). Write the invariant
//! first; each SAFETY comment should point back to it. Add one edge-case test.
//! EXPLAIN afterward: Give the invariant of your arena. Then explain the two
//! design decisions in the contract: why `alloc` never runs destructors, and
//! why `alloc_with_drop` needs `T: 'static`. Show the safe program that would
//! read freed memory if that bound were missing.
//!
//! MIRI LAB: `examples/miri_l08_arena.rs` is a bump allocator with every check
//! except one. Predict what Miri reports, run
//! `cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l08_arena`,
//! compare with CONCEPTS.md, then repair the example until Miri is silent.
//!
//! CONTRACT
//!
//! One public type in this file: `Arena`, a bump allocator. It takes large
//! blocks ("chunks") from the global allocator and hands out pieces of them
//! one after the other. A single value is never freed. Everything is freed at
//! once, when the arena is dropped or reset.
//!
//! - `Arena::new() -> Arena` does not allocate.
//! - `Arena::with_capacity(bytes: usize) -> Arena` reserves one chunk with room
//!   for at least `bytes` bytes. That room starts at an address that is a
//!   multiple of 16, so that `with_capacity(800)` holds 100 `u64` values
//!   without a second chunk. Panics with `capacity overflow` if no such chunk
//!   can exist.
//! - `alloc<T>(&self, value: T) -> &mut T` moves `value` into the arena. The
//!   reference is valid for as long as the arena is borrowed. Its address is
//!   aligned for `T`, never changes, and its bytes belong to no other value.
//!   `T` may contain references to other values in the same arena.
//!   The arena **never runs the destructor** of a value stored with `alloc`.
//! - `alloc_with_drop<T: 'static>(&self, value: T) -> &mut T` is like `alloc`,
//!   but the arena runs the value's destructor exactly once, when the arena
//!   is dropped or reset. Destructors run in reverse order of allocation.
//! - `alloc_slice_copy<T: Copy>(&self, values: &[T]) -> &mut [T]` copies a
//!   slice into the arena.
//! - `alloc_str(&self, text: &str) -> &mut str` copies a string slice.
//! - `reset(&mut self)` runs the pending destructors and forgets every value,
//!   but keeps memory for reuse: when the same sequence of allocations is
//!   repeated after each reset, the third and later rounds must not call the
//!   global allocator.
//! - Dropping the arena runs the pending destructors and frees every chunk.
//!   If a destructor panics, the remaining destructors still run, and the
//!   chunks are still freed.
//! - Zero-sized values, empty slices and empty strings take no room and never
//!   cause a call to the global allocator. (One exception is allowed: a
//!   zero-sized value with a destructor that is stored with `alloc_with_drop`
//!   has to be remembered, and the record of it may take room.)
//! - Chunks grow geometrically: `n` small values need about `log2(n)` chunks.
//!   A value that is larger than the usual chunk still works.
//! - `Arena` is neither `Send` nor `Sync`.
//!
//! Clippy's `mut_from_ref` lint rejects the signature of `alloc`. It is right
//! to be suspicious of `&self -> &mut T`; an arena is the standard exception.
//! Allow the lint on those functions, with a comment that says why.
//!
//! COMPILER CHECKS
//!
//! Two parts of the contract are promises about programs that must not
//! compile: the `'static` bound of `alloc_with_drop`, and the `&mut self` of
//! `reset`. A test that runs cannot observe them, and Miri cannot either, so
//! the three examples below are built by
//! `cargo test --manifest-path mastery/m2_unsafe/Cargo.toml --doc l08_`.
//! The first one must compile and run. It fails until your `Arena` exists.
//! The other two must be rejected by the compiler. Until the first one passes
//! they are rejected only because there is no `Arena`, and that proves nothing.
//!
//! ```
//! use m2_unsafe::l08_capstone::Arena;
//!
//! let mut arena = Arena::new();
//! let owned: &mut String = arena.alloc_with_drop(String::from("owned"));
//! owned.push('!');
//! let fixed: &mut &'static str = arena.alloc_with_drop("lives forever");
//! assert_eq!((owned.as_str(), *fixed), ("owned!", "lives forever"));
//! arena.reset();
//! ```
//!
//! `alloc_with_drop` must refuse a value that borrows from something that
//! lives shorter than `'static`. Here `local` is dropped before `arena`:
//!
//! ```compile_fail
//! use m2_unsafe::l08_capstone::Arena;
//!
//! let arena = Arena::new();
//! let local = String::from("dropped before the arena");
//! arena.alloc_with_drop(&local);
//! ```
//!
//! `reset` must refuse to run while a reference into the arena is in use:
//!
//! ```compile_fail
//! use m2_unsafe::l08_capstone::Arena;
//!
//! let mut arena = Arena::new();
//! let number = arena.alloc(1u8);
//! arena.reset();
//! *number += 1;
//! ```

use std::alloc::{self, Layout};
use std::cell::{Cell, RefCell};
use std::ptr::{self, NonNull};

/// The size of the first chunk, in bytes, unless a larger one is needed.
const FIRST_CHUNK: usize = 1024;
/// Every chunk is aligned at least this much, so that most values can start
/// at the first byte of a new chunk.
const CHUNK_ALIGN: usize = 16;

#[cold]
fn capacity_overflow() -> ! {
    panic!("capacity overflow")
}

/// One block from the global allocator. Freed when dropped.
struct Chunk {
    start: NonNull<u8>,
    layout: Layout,
}

impl Chunk {
    fn new(size: usize, align: usize) -> Chunk {
        let Ok(layout) = Layout::from_size_align(size.max(1), align.max(CHUNK_ALIGN)) else {
            capacity_overflow()
        };
        // SAFETY: the layout has a size of at least one byte.
        let raw = unsafe { alloc::alloc(layout) };
        let start = NonNull::new(raw).unwrap_or_else(|| alloc::handle_alloc_error(layout));
        Chunk { start, layout }
    }

    /// The first byte and the byte just past the last one.
    fn bounds(&self) -> (*mut u8, *mut u8) {
        let start = self.start.as_ptr();
        // SAFETY: the chunk is `layout.size()` bytes long, so the result is
        // the one-past-the-end pointer of the same block.
        (start, unsafe { start.add(self.layout.size()) })
    }
}

impl Drop for Chunk {
    fn drop(&mut self) {
        // SAFETY: `start` came from the global allocator with `layout` in
        // `Chunk::new`, and a chunk is dropped only once.
        unsafe { alloc::dealloc(self.start.as_ptr(), self.layout) };
    }
}

/// A value whose destructor the arena has promised to run.
struct Pending {
    value: *mut u8,
    /// Drops the value behind the pointer; the function knows the type.
    drop_value: unsafe fn(*mut u8),
}

/// A bump allocator.
///
/// Invariant:
/// 1. `chunks` owns every block that the arena took from the global allocator.
///    A block is freed only by `reset` or by dropping the arena. Both need
///    exclusive access to the arena, so no reference handed out by an `alloc`
///    function (they all borrow `&self`) is alive at that moment.
/// 2. `cursor <= end`, and both point into the last chunk or just past its
///    end. Without a chunk both are null. The bytes in `cursor..end` are free:
///    they are part of no value.
/// 3. Every value handed out lies inside one chunk, below that chunk's cursor
///    position at the time, and no two values share a byte.
/// 4. `pending` has exactly one entry for each value stored with
///    `alloc_with_drop` whose type needs dropping, in order of allocation.
///    Such a value stays initialized until its entry is removed, and its type
///    is `'static`.
pub struct Arena {
    cursor: Cell<*mut u8>,
    end: Cell<*mut u8>,
    chunks: RefCell<Vec<Chunk>>,
    pending: RefCell<Vec<Pending>>,
}

impl Arena {
    pub fn new() -> Arena {
        Arena {
            cursor: Cell::new(ptr::null_mut()),
            end: Cell::new(ptr::null_mut()),
            chunks: RefCell::new(Vec::new()),
            pending: RefCell::new(Vec::new()),
        }
    }

    pub fn with_capacity(bytes: usize) -> Arena {
        let arena = Arena::new();
        if bytes > 0 {
            arena.add_chunk(bytes, CHUNK_ALIGN);
        }
        arena
    }

    /// Makes a new chunk of at least `size` bytes the current one. The free
    /// bytes of the old chunk are not used again until `reset`.
    #[cold]
    fn add_chunk(&self, size: usize, align: usize) {
        let mut chunks = self.chunks.borrow_mut();
        let doubled = chunks
            .last()
            .map_or(FIRST_CHUNK, |last| last.layout.size().saturating_mul(2));
        let chunk = Chunk::new(size.max(doubled), align);
        let (start, end) = chunk.bounds();
        chunks.push(chunk);
        self.cursor.set(start);
        self.end.set(end);
    }

    /// Takes `layout.size()` bytes at the next address that is aligned for
    /// `layout` out of the current chunk, if they fit.
    fn bump(&self, layout: Layout) -> Option<NonNull<u8>> {
        let cursor = self.cursor.get();
        let address = cursor.addr();
        let padding = address.checked_next_multiple_of(layout.align())? - address;
        let needed = padding.checked_add(layout.size())?;
        if needed > self.end.get().addr() - address {
            return None;
        }
        // SAFETY: by invariant 2, `cursor..end` lies inside the current chunk,
        // and `padding + layout.size() <= end - cursor`, so both results stay
        // inside that chunk or just past its end. (Without a chunk both
        // pointers are null and the check above already returned, because the
        // callers never ask for zero bytes.)
        let (start, next) = unsafe {
            let start = cursor.add(padding);
            (start, start.add(layout.size()))
        };
        // The bytes `start..next` are no longer free (invariants 2 and 3).
        self.cursor.set(next);
        NonNull::new(start)
    }

    /// Reserves room for `layout`, which must not be zero-sized. The result is
    /// aligned for `layout` and valid for reads and writes of `layout.size()`
    /// bytes that belong to no other value, until the arena is reset or dropped.
    fn reserve(&self, layout: Layout) -> NonNull<u8> {
        debug_assert!(layout.size() > 0);
        if let Some(room) = self.bump(layout) {
            return room;
        }
        // The new chunk is at least `layout.size()` bytes long and aligned at
        // least as much as `layout`, so the second attempt cannot fail.
        self.add_chunk(layout.size(), layout.align());
        self.bump(layout)
            .expect("a fresh chunk has room for the value it was made for")
    }

    /// Moves `value` into the arena and returns its address.
    fn place<T>(&self, value: T) -> NonNull<T> {
        let slot = if size_of::<T>() == 0 {
            NonNull::dangling()
        } else {
            self.reserve(Layout::new::<T>()).cast::<T>()
        };
        // SAFETY: `slot` is aligned for `T` and valid for a write of `T`:
        // either `reserve` set aside `size_of::<T>()` bytes for it, or `T` is
        // zero-sized and a dangling pointer is valid for a write of no bytes.
        // The room is uninitialized, so `write` (no drop of old contents) is
        // the right operation.
        unsafe { slot.write(value) };
        slot
    }

    // An arena is the standard exception to this lint: every call returns a
    // different piece of memory, so the `&mut T` never aliases anything.
    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        let slot = self.place(value);
        // SAFETY: `slot` is aligned and was initialized by `place` just now.
        // By invariant 3 its bytes belong to no other value, and the arena
        // itself keeps no reference to them, so the `&mut T` is unique. By
        // invariant 1 the chunk outlives the borrow of `self` that the result
        // is tied to. The arena never reads or drops the value afterwards.
        unsafe { &mut *slot.as_ptr() }
    }

    // See `alloc` for why this lint is allowed.
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_with_drop<T: 'static>(&self, value: T) -> &mut T {
        /// Drops the `T` behind an untyped pointer.
        ///
        /// # Safety
        ///
        /// `value` must point to an initialized `T` that nobody uses afterwards.
        unsafe fn drop_value<T>(value: *mut u8) {
            // SAFETY: the caller passes a pointer to an initialized `T` that
            // is not used again.
            unsafe { value.cast::<T>().drop_in_place() };
        }

        let slot = self.place(value);
        if std::mem::needs_drop::<T>() {
            self.pending.borrow_mut().push(Pending {
                value: slot.as_ptr().cast::<u8>(),
                drop_value: drop_value::<T>,
            });
        }
        // SAFETY: as in `alloc`. The arena keeps the raw pointer in `pending`
        // but uses it only in `run_destructors`, which needs `&mut self` and
        // so runs after this reference has ended.
        unsafe { &mut *slot.as_ptr() }
    }

    // See `alloc` for why this lint is allowed.
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_slice_copy<T: Copy>(&self, values: &[T]) -> &mut [T] {
        let layout = Layout::for_value(values);
        let slot = if layout.size() == 0 {
            NonNull::dangling()
        } else {
            self.reserve(layout).cast::<T>()
        };
        // SAFETY: `values` is valid for reads of `values.len()` elements.
        // `slot` is aligned for `T` and valid for writes of as many elements:
        // `reserve` set aside `size_of_val(values)` bytes (or the size is zero
        // and a dangling pointer is enough). The destination was free until
        // now, so it cannot overlap `values`, even if `values` lives in this
        // arena. `T: Copy`, so a byte copy makes independent values. After the
        // copy the elements are initialized, and the new slice is unique and
        // lives as long as the borrow of `self`, as explained in `alloc`.
        unsafe {
            ptr::copy_nonoverlapping(values.as_ptr(), slot.as_ptr(), values.len());
            std::slice::from_raw_parts_mut(slot.as_ptr(), values.len())
        }
    }

    // See `alloc` for why this lint is allowed.
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_str(&self, text: &str) -> &mut str {
        let bytes = self.alloc_slice_copy(text.as_bytes());
        // SAFETY: `bytes` is a copy of the bytes of a `str`, so it is valid
        // UTF-8, and nobody else can change it.
        unsafe { std::str::from_utf8_unchecked_mut(bytes) }
    }

    /// Runs every pending destructor, last allocated first. If one of them
    /// panics, the others still run while the panic unwinds.
    fn run_destructors(pending: &mut Vec<Pending>) {
        /// Continues with the rest of the list when dropped during unwinding.
        struct Rest<'a>(&'a mut Vec<Pending>);

        impl Drop for Rest<'_> {
            fn drop(&mut self) {
                Arena::run_destructors(self.0);
            }
        }

        while let Some(entry) = pending.pop() {
            let rest = Rest(pending);
            // SAFETY: by invariant 4 the pointer leads to an initialized value
            // of the type that `drop_value` was made for. The entry has been
            // removed from the list, so this is the value's only drop. The
            // chunk is still allocated, because the callers free chunks only
            // after this function. No reference to the value is alive: the
            // callers have exclusive access to the arena. The type is
            // `'static`, so the destructor cannot reach other values of this
            // arena, some of which are already dropped.
            unsafe { (entry.drop_value)(entry.value) };
            // No panic: stay in this loop instead of recursing.
            std::mem::forget(rest);
        }
    }

    pub fn reset(&mut self) {
        Arena::run_destructors(self.pending.get_mut());
        // Keep the newest chunk, which is the largest one, and free the rest.
        let chunks = self.chunks.get_mut();
        let keep_from = chunks.len().saturating_sub(1);
        drop(chunks.drain(..keep_from));
        // `&mut self` proves that no reference into the arena is alive, so
        // every byte of the kept chunk is free again (invariants 2 and 3).
        let (start, end) = chunks
            .last()
            .map_or((ptr::null_mut(), ptr::null_mut()), Chunk::bounds);
        self.cursor.set(start);
        self.end.set(end);
    }
}

impl Default for Arena {
    fn default() -> Arena {
        Arena::new()
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        Arena::run_destructors(self.pending.get_mut());
        // The chunks are freed when the `chunks` field is dropped, which
        // happens after this function, also if a destructor above panicked.
    }
}

crate::gate! { on, "08" =>
#[cfg(test)]
mod tests {
    use super::*;
    use crate::impls;
    use crate::support::alloc::{audit, counting};
    use std::cell::{Cell, RefCell};
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
    use std::rc::Rc;

    #[repr(align(64))]
    #[derive(Debug, PartialEq)]
    struct Wide(u8);

    /// Writes its name to a shared log when dropped. An upper-case name makes
    /// the destructor unwind afterwards, like a destructor that panics.
    struct Noisy(char, Rc<RefCell<String>>);

    impl Drop for Noisy {
        fn drop(&mut self) {
            self.1.borrow_mut().push(self.0);
            if self.0.is_ascii_uppercase() {
                // Unwinds like a panic, without printing a message.
                resume_unwind(Box::new("destructor failed"));
            }
        }
    }

    /// A log with room for every test's entries, so that writing to it inside
    /// an `audit` does not count as an allocation of the code under test.
    fn new_log() -> Rc<RefCell<String>> {
        Rc::new(RefCell::new(String::with_capacity(64)))
    }

    fn small_count() -> usize {
        if cfg!(miri) { 300 } else { 20_000 }
    }

    #[test]
    fn a_values_keep_their_address_while_the_arena_grows() {
        let arena = Arena::new();
        let first = arena.alloc(1u64);
        let second = arena.alloc(2u64);
        *first += 10;
        let mut later = Vec::new();
        for n in 0..small_count() {
            later.push(arena.alloc(n));
        }
        *second += 20;
        assert_eq!(
            (*first, *second),
            (11, 22),
            "values allocated early must stay where they are when new chunks are added"
        );
        for (n, value) in later.iter().enumerate() {
            assert_eq!(**value, n, "value {n} was overwritten or moved");
        }
    }

    #[test]
    fn a_every_value_is_aligned_for_its_type() {
        let arena = Arena::new();
        for round in 0..20u8 {
            let byte = arena.alloc(round);
            let wide = arena.alloc(Wide(round));
            let word = arena.alloc(u64::from(round));
            let half = arena.alloc(u16::from(round));
            assert_eq!(
                std::ptr::from_ref(wide).addr() % 64,
                0,
                "a value of a type with align(64) must sit at a multiple of 64"
            );
            assert_eq!(std::ptr::from_ref(word).addr() % align_of::<u64>(), 0);
            assert_eq!(std::ptr::from_ref(half).addr() % align_of::<u16>(), 0);
            assert_eq!(
                (*byte, &*wide, *word, *half),
                (round, &Wide(round), u64::from(round), u16::from(round))
            );
        }
    }

    #[test]
    fn a_values_do_not_share_bytes() {
        let arena = Arena::new();
        let mut spans: Vec<(usize, usize)> = Vec::new();
        for n in 0..200u32 {
            let small = arena.alloc([n as u8; 3]);
            spans.push((std::ptr::from_ref(small).addr(), 3));
            let word = arena.alloc(n);
            spans.push((std::ptr::from_ref(word).addr(), 4));
            let text = arena.alloc_str("seven b");
            spans.push((text.as_ptr().addr(), 7));
        }
        spans.sort_unstable();
        for pair in spans.windows(2) {
            let ((start, len), (next, _)) = (pair[0], pair[1]);
            assert!(
                start + len <= next,
                "two values overlap: one covers {start:#x}..{:#x}, the next starts at {next:#x}",
                start + len
            );
        }
    }

    #[test]
    fn a_zero_sized_values_take_no_room() {
        let ((), heap) = audit(|| {
            let arena = Arena::new();
            let unit: &mut () = arena.alloc(());
            let empty_array = arena.alloc([0u64; 0]);
            assert_eq!(
                std::ptr::from_ref(empty_array).addr() % align_of::<u64>(),
                0,
                "even a reference to a zero-sized value must be aligned"
            );
            assert!(arena.alloc_slice_copy::<u32>(&[]).is_empty());
            assert_eq!(arena.alloc_str(""), "");
            let () = *unit;
        });
        assert_eq!(
            heap.allocations, 0,
            "a new arena and zero-sized values must not reach the global allocator"
        );
        assert_eq!(heap.zero_sized, 0);
    }

    #[test]
    fn b_many_small_values_need_few_chunks() {
        let count = small_count();
        let ((), heap) = audit(|| {
            let arena = Arena::new();
            let mut sum = 0;
            for n in 0..count {
                sum += *arena.alloc(n);
            }
            assert_eq!(sum, count * (count - 1) / 2);
        });
        assert!(
            heap.allocations <= 40,
            "{count} small values made {} calls to the global allocator: take memory in \
             chunks that double in size",
            heap.allocations
        );
        assert_eq!(heap.live, 0, "dropping the arena must free every chunk");
        assert_eq!(
            heap.bad_frees, 0,
            "each chunk must be freed with the layout it was allocated with"
        );
        assert_eq!(heap.zero_sized, 0);
    }

    #[test]
    fn b_with_capacity_reserves_the_room_up_front() {
        let (arena, heap) = audit(|| Arena::with_capacity(100 * size_of::<u64>()));
        if counting() {
            assert!(
                heap.allocations >= 1,
                "with_capacity must reserve its chunk at once"
            );
        }
        let ((), heap) = audit(|| {
            for n in 0..100u64 {
                arena.alloc(n);
            }
        });
        assert_eq!(
            heap.allocations, 0,
            "100 u64 values must fit into the chunk that with_capacity reserved"
        );
    }

    #[test]
    #[should_panic(expected = "capacity overflow")]
    fn b_an_impossible_capacity_panics() {
        let _ = Arena::with_capacity(usize::MAX);
    }

    #[test]
    fn b_a_value_larger_than_a_chunk_still_fits() {
        let arena = Arena::new();
        let before = arena.alloc(1u8);
        let big = arena.alloc([7u8; 100_000]);
        let after = arena.alloc(2u8);
        big[99_999] = 9;
        assert_eq!((*before, big[0], big[99_999], *after), (1, 7, 9, 2));
    }

    #[test]
    fn c_slices_and_strings_are_copied_into_the_arena() {
        let arena = Arena::new();
        let source = [1u32, 2, 3];
        let copy = arena.alloc_slice_copy(&source);
        copy[0] = 10;
        assert_eq!((source, &*copy), ([1, 2, 3], &[10, 2, 3][..]));
        assert_eq!(std::ptr::from_ref(&copy[0]).addr() % align_of::<u32>(), 0);

        let text = arena.alloc_str("héllo");
        text.make_ascii_uppercase();
        assert_eq!(text, "HéLLO");
        let again = arena.alloc_str(text);
        assert!(
            !std::ptr::eq(again.as_ptr(), text.as_ptr()),
            "a string copied from the arena into the arena gets its own bytes"
        );
        assert_eq!(again, text);
    }

    /// A list node that points to another node in the same arena.
    struct Node<'a> {
        name: &'a str,
        next: Cell<Option<&'a Node<'a>>>,
    }

    #[test]
    fn d_values_may_point_to_each_other() {
        let arena = Arena::new();
        let first: &Node<'_> = arena.alloc(Node {
            name: arena.alloc_str("first"),
            next: Cell::new(None),
        });
        let second: &Node<'_> = arena.alloc(Node {
            name: arena.alloc_str("second"),
            next: Cell::new(Some(first)),
        });
        first.next.set(Some(second));
        let mut names = Vec::new();
        let mut node = first;
        for _ in 0..3 {
            names.push(node.name);
            node = node.next.get().expect("the two nodes form a cycle");
        }
        assert_eq!(names, ["first", "second", "first"]);
    }

    #[test]
    fn e_alloc_never_runs_a_destructor() {
        struct Flag<'a>(&'a Cell<bool>);
        impl Drop for Flag<'_> {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let dropped = Cell::new(false);
        {
            let arena = Arena::new();
            arena.alloc(Flag(&dropped));
        }
        assert!(
            !dropped.get(),
            "the arena must never run the destructor of a value stored with alloc"
        );
    }

    #[test]
    fn e_alloc_with_drop_runs_each_destructor_once_in_reverse_order() {
        let log = new_log();
        let ((), heap) = audit(|| {
            let arena = Arena::new();
            arena.alloc_with_drop(Noisy('a', Rc::clone(&log)));
            arena.alloc(5u64);
            let b = arena.alloc_with_drop(Noisy('b', Rc::clone(&log)));
            b.0 = 'x';
            arena.alloc_with_drop(String::from("owns heap memory"));
            arena.alloc_with_drop(Noisy('c', Rc::clone(&log)));
            assert_eq!(
                *log.borrow(),
                "",
                "nothing is dropped while the arena is alive"
            );
        });
        assert_eq!(
            *log.borrow(),
            "cxa",
            "the arena must drop the values of alloc_with_drop once, last allocated first"
        );
        assert_eq!(Rc::strong_count(&log), 1);
        assert_eq!(
            heap.live, 0,
            "the String's buffer and all chunks must be freed with the arena"
        );
    }

    #[test]
    fn e_reset_drops_the_values_and_reuses_the_memory() {
        let log = new_log();
        let count = small_count();
        let mut arena = Arena::new();
        let mut calls = Vec::new();
        for round in ['1', '2', '3', '4'] {
            let ((), heap) = audit(|| {
                arena.alloc_with_drop(Noisy(round, Rc::clone(&log)));
                for n in 0..count {
                    assert_eq!(*arena.alloc(n), n);
                }
            });
            calls.push(heap.allocations);
            arena.reset();
        }
        assert_eq!(
            *log.borrow(),
            "1234",
            "reset must run the pending destructors"
        );
        assert_eq!(
            calls[2..],
            [0, 0],
            "after two resets, a repeated workload must fit into memory the arena \
             already has (calls to the global allocator per round: {calls:?})"
        );
        arena.reset();
        assert_eq!(*log.borrow(), "1234", "a destructor must never run twice");
    }

    #[test]
    fn e_a_panicking_destructor_does_not_stop_the_others() {
        let log = new_log();
        let (panicked, heap) = audit(|| {
            let arena = Arena::new();
            arena.alloc_with_drop(Noisy('a', Rc::clone(&log)));
            arena.alloc_with_drop(Noisy('B', Rc::clone(&log)));
            arena.alloc_with_drop(Noisy('c', Rc::clone(&log)));
            catch_unwind(AssertUnwindSafe(move || drop(arena))).is_err()
        });
        assert!(panicked, "the destructor's panic must reach the caller");
        assert_eq!(
            *log.borrow(),
            "cBa",
            "the destructors after the one that panicked must still run, once each"
        );
        assert_eq!(
            heap.live, 0,
            "the chunks must be freed even when a destructor panics"
        );
    }

    #[test]
    fn f_the_arena_stays_on_its_thread() {
        assert!(
            !impls!(Arena: Sync),
            "Arena must not be Sync: two threads would move the same cursor"
        );
        assert!(
            !impls!(Arena: Send),
            "Arena must not be Send: alloc_with_drop accepts values that are not Send, \
             and the arena drops them wherever it is dropped"
        );
    }
}
}

#[cfg(test)]
mod predictions {
    use crate::guess;
    use std::alloc::Layout;
    use std::mem::needs_drop;

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Rounding an address up to an alignment: three ways to write it.
        let align = 8usize;
        let p1: (usize, usize, usize) = (16, 16, 24);
        assert_eq!(
            p1,
            (
                (13 + align - 1) & !(align - 1),
                16usize.next_multiple_of(align),
                17usize.div_ceil(align) * align
            ),
            "P1: 13, 16 and 17 rounded up to a multiple of 8 \
             (left: your prediction, right: actual)"
        );

        // P2. Which of these types have a destructor that must run?
        let p2: (bool, bool, bool, bool) = (false, true, false, true);
        assert_eq!(
            p2,
            (
                needs_drop::<&String>(),
                needs_drop::<String>(),
                needs_drop::<[u8; 4]>(),
                needs_drop::<Option<Box<u8>>>()
            ),
            "P2: needs_drop for &String, String, [u8; 4], Option<Box<u8>> \
             (left: your prediction, right: actual)"
        );

        // P3. A u64 followed by a u8, laid out by hand: at which offset does
        // the u8 start, how many bytes are used, and how large is the whole
        // thing once it is padded to its alignment?
        let (both, offset) = Layout::new::<u64>().extend(Layout::new::<u8>()).unwrap();
        let p3: (usize, usize, usize) = (8, 9, 16);
        assert_eq!(
            p3,
            (offset, both.size(), both.pad_to_align().size()),
            "P3: (offset of the u8, size before padding, size after padding) \
             (left: your prediction, right: actual)"
        );
    }
}
