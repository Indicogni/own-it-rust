//! LESSON 09 — Interior mutability and layout attributes
//! Run: cargo test --test chapter23 l09_
//! Read first: "Lesson 09" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `&T` promises that the value behind it does not change. Which one std
//!    type removes that promise for its contents?
//! 2. A `u32` field sits at byte offset 1 of a packed struct. What is wrong
//!    with `&record.count`, even if nobody reads through that reference?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain why `MyCell<T>` may change its contents through
//! `&self`, why it must not be `Sync`, and why it never gives out a reference
//! to its contents. Then say what `repr(transparent)` guarantees for the cast
//! in task B, and what `repr(packed)` takes away in task C.
//!
//! COMPILER / TRANSFER LAB (optional): Try two programs, one at a time. First,
//! use a `&std::cell::Cell<u32>` inside `std::thread::scope(|s| s.spawn(..))`
//! and read the E0277 error. Second, write `&wire.count` for a packed struct
//! and read the E0793 error; repair it once with a copy and once with
//! `&raw const` and `read_unaligned`.
//! Use examples/ch23_scratch.rs for this, and leave it compiling afterward.

use std::cell::UnsafeCell;

/// A value that can be replaced through a shared reference, like
/// `std::cell::Cell`.
///
/// Invariant: no reference to the contents ever leaves this type. Every
/// method moves or copies a whole `T` in or out, and keeps no pointer.
///
/// The `UnsafeCell` field makes this type `!Sync` automatically. Do not add
/// `unsafe impl Sync`: in NOTES.md, describe what two threads could do with
/// one `&MyCell<u64>` if it were `Sync`.
struct MyCell<T> {
    value: UnsafeCell<T>,
}
impl<T> MyCell<T> {
    fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }
    /// A. Store `value` and return the old contents. Nothing is cloned and
    /// nothing is dropped. Write the SAFETY comment: why is this write
    /// through `&self` the only access to the contents at this moment?
    fn replace(&self, value: T) -> T {
        todo!("A: implement the contract")
    }
    /// A. Store `value`. The old contents are dropped exactly once, and only
    /// after the new value is in place.
    fn set(&self, value: T) {
        todo!("A: implement the contract")
    }
}
impl<T: Copy> MyCell<T> {
    /// A. Return a copy of the contents.
    fn get(&self) -> T {
        todo!("A: implement the contract")
    }
}

/// A user id.
///
/// B (first part). The cast in `as_ids` is sound only if `UserId` is
/// guaranteed to have the layout of its single field. The default layout
/// gives no such guarantee. Add the one attribute that does.
#[derive(Debug, PartialEq, Clone, Copy)]
struct UserId(u32);

/// B. View a slice of raw numbers as a slice of ids: the same memory, the
/// same length, no allocation. Build the result with a raw pointer cast and
/// `slice::from_raw_parts` (not `transmute`), and write the SAFETY comment.
fn as_ids(raw: &[u32]) -> &[UserId] {
    todo!("B: implement the contract")
}

/// One record of a wire format: exactly 7 bytes. `packed` removes the padding,
/// so `count` and `port` are usually NOT aligned. Copying a field out
/// (`let n = wire.count;`) is always fine. A reference to such a field must
/// never exist: `&wire.count` does not compile.
#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(C, packed)]
struct Wire {
    kind: u8,
    count: u32,
    port: u16,
}

/// # Safety
/// Implementors must be `Copy`, have no padding, and every bit pattern of
/// their size must be a valid value. (Lesson 05 defines the same trait; it is
/// repeated here so that the lessons stay independent.)
unsafe trait Plain: Copy {}
// SAFETY: integers have no padding, and every bit pattern is a valid integer.
unsafe impl Plain for u16 {}
// SAFETY: as above.
unsafe impl Plain for u32 {}
// SAFETY: as above.
unsafe impl Plain for u64 {}
// SAFETY: `Wire` is `repr(C, packed)`, so it has no padding, and its three
// integer fields accept every bit pattern.
unsafe impl Plain for Wire {}

/// C. Return a raw pointer to the `count` field of `wire` without creating a
/// reference to that field, not even for a moment.
fn count_field(wire: &Wire) -> *const u32 {
    todo!("C: implement the contract")
}

/// C. Read the `T` whose first byte is `bytes[offset]`. That address can
/// have any alignment, so use `read_unaligned`. Return None unless all
/// `size_of::<T>()` bytes lie inside `bytes`; the range check itself must not
/// overflow.
fn read_at<T: Plain>(bytes: &[u8], offset: usize) -> Option<T> {
    todo!("C: implement the contract")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, guess};
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::atomic::AtomicU32;

    /// Supplied. `implements!(Type: Trait)` is true when the type implements
    /// the trait. It works because an inherent constant, which exists only
    /// when the bound holds, is preferred over a trait constant.
    macro_rules! implements {
        ($ty:ty : $($bound:tt)+) => {{
            struct Probe<T: ?Sized>(std::marker::PhantomData<T>);
            #[allow(dead_code)]
            trait Fallback {
                const YES: bool = false;
            }
            impl<T: ?Sized> Fallback for Probe<T> {}
            #[allow(dead_code)]
            impl<T: ?Sized + $($bound)+> Probe<T> {
                const YES: bool = true;
            }
            <Probe<$ty>>::YES
        }};
    }

    /// Adds 1 to a shared counter when it is dropped.
    struct Spy(Rc<Cell<usize>>);
    impl Drop for Spy {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn a_cell_changes_through_shared_references() {
        let cell = MyCell::new(5_u32);
        let (first, second) = (&cell, &cell);
        first.set(6);
        assert_eq!(
            second.get(),
            6,
            "both shared references must see the same contents"
        );
        assert_eq!(second.replace(9), 6, "replace returns the old contents");
        assert_eq!(first.get(), 9);

        // A closure that captures only `&calls` can still count its calls.
        fn run_twice(f: impl Fn()) {
            f();
            f();
        }
        let calls = MyCell::new(0_u32);
        run_twice(|| calls.set(calls.get() + 1));
        assert_eq!(calls.get(), 2);
    }
    #[test]
    fn a_replace_moves_values_in_and_out() {
        let old = String::from("old");
        let new = String::from("new");
        let (p_old, p_new) = (old.as_ptr(), new.as_ptr());
        let cell = MyCell::new(old);
        let (out, allocations) = count(|| cell.replace(new));
        assert_eq!(
            (out.as_str(), allocations),
            ("old", 0),
            "(old contents, allocations): replace moves two Strings; it never clones"
        );
        assert_eq!(
            out.as_ptr(),
            p_old,
            "replace must return the original String, with its original buffer"
        );
        let now = cell.replace(String::new());
        assert_eq!(
            now.as_ptr(),
            p_new,
            "the cell must hold the String that was moved in, not a copy"
        );
    }
    #[test]
    fn a_every_value_is_dropped_exactly_once() {
        let drops = Rc::new(Cell::new(0));
        let cell = MyCell::new(Spy(Rc::clone(&drops)));
        cell.set(Spy(Rc::clone(&drops)));
        assert_eq!(
            drops.get(),
            1,
            "drops after set: the old contents must be dropped, exactly once \
             (a plain `write` over them would leak the old value)"
        );
        let out = cell.replace(Spy(Rc::clone(&drops)));
        assert_eq!(
            drops.get(),
            1,
            "drops after replace: the old contents moved to the caller; nothing is dropped"
        );
        drop(out);
        assert_eq!(drops.get(), 2, "drops after dropping the returned value");
        drop(cell);
        assert_eq!(
            drops.get(),
            3,
            "drops in total: each of the three values exactly once"
        );
    }
    #[test]
    fn a_cell_is_send_but_not_sync() {
        assert!(
            implements!(MyCell<u32>: Send),
            "moving the whole cell to another thread is fine"
        );
        assert!(
            !implements!(MyCell<u32>: Sync),
            "MyCell must stay !Sync: with `unsafe impl Sync`, two threads could call `set` \
             on one cell at the same time, which is a data race"
        );
        assert_eq!(
            size_of::<MyCell<u32>>(),
            size_of::<u32>(),
            "interior mutability needs no extra space"
        );
    }

    #[test]
    fn b_ids_are_a_view_of_the_same_memory() {
        let raw = vec![7_u32, 8, 9];
        let (ids, allocations) = count(|| as_ids(&raw));
        assert_eq!(ids, [UserId(7), UserId(8), UserId(9)]);
        assert_eq!(
            allocations, 0,
            "as_ids makes a view; it must not allocate a new buffer"
        );
        assert_eq!(
            ids.as_ptr().cast::<u32>(),
            raw.as_ptr(),
            "the ids must be the same memory as the numbers, not a copy"
        );
        assert!(as_ids(&[]).is_empty(), "an empty slice gives an empty view");
        assert_eq!(as_ids(&raw[1..]), [UserId(8), UserId(9)]);
    }
    #[test]
    fn b_technique_layout_is_guaranteed_not_assumed() {
        let file = include_str!("l09_cells_layout.rs");
        assert!(
            crate::source::code_of(file).contains("repr(transparent)"),
            "task B, first part: without `#[repr(transparent)]` on `UserId`, nothing \
             guarantees that `UserId` and `u32` have the same layout. The cast works today \
             and Miri accepts it, but your SAFETY argument has nothing to stand on"
        );
        assert!(
            !crate::source::body_of(file, "fn as_ids").contains("transmute"),
            "do not `transmute` a slice reference: the layout of `&[T]` itself is not \
             specified. Cast the element pointer and rebuild the slice with `from_raw_parts`"
        );
    }

    /// Eight-aligned storage, so that the alignment of each offset is known.
    #[repr(C, align(8))]
    struct Buffer([u8; 16]);

    #[test]
    fn c_field_pointer_is_made_without_a_reference() {
        #[repr(C, align(4))]
        struct Framed(Wire);
        let framed = Framed(Wire {
            kind: 1,
            count: 0xA1B2_C3D4,
            port: 9,
        });
        let field = count_field(&framed.0);
        assert_eq!(
            field.addr(),
            (&raw const framed).addr() + 1,
            "`count` starts one byte after the start of the record"
        );
        assert!(
            !field.is_aligned(),
            "an address of the form 4k + 1 is not aligned for u32"
        );
        // SAFETY: `field` points to the 4 initialized bytes of `framed.0.count`,
        // which is alive; `read_unaligned` has no alignment requirement.
        assert_eq!(unsafe { field.read_unaligned() }, 0xA1B2_C3D4);
    }
    #[test]
    fn c_reads_at_any_alignment() {
        let mut buffer = Buffer([0; 16]);
        buffer.0[1..5].copy_from_slice(&0xDEAD_BEEF_u32.to_ne_bytes());
        buffer.0[7..15].copy_from_slice(&u64::MAX.to_ne_bytes());
        assert_eq!(
            read_at::<u32>(&buffer.0, 1),
            Some(0xDEAD_BEEF),
            "offset 1 of an 8-aligned buffer is not aligned for u32: use read_unaligned"
        );
        assert_eq!(read_at::<u64>(&buffer.0, 7), Some(u64::MAX));
        assert_eq!(
            read_at::<u16>(&buffer.0, 14),
            Some(u16::from_ne_bytes([0xFF, 0])),
            "the last two bytes of the buffer are readable"
        );
        assert_eq!(
            read_at::<u32>(&buffer.0, 12),
            Some(u32::from_ne_bytes([0xFF, 0xFF, 0xFF, 0])),
            "the last four bytes of the buffer are readable"
        );
    }
    #[test]
    fn c_rejects_ranges_outside_the_buffer() {
        let buffer = Buffer([0xAA; 16]);
        assert_eq!(
            read_at::<u32>(&buffer.0, 13),
            None,
            "bytes 13..17 end outside"
        );
        assert_eq!(read_at::<u64>(&buffer.0, 9), None);
        assert_eq!(read_at::<u16>(&buffer.0, 16), None, "offset == len");
        assert_eq!(
            read_at::<u16>(&buffer.0, usize::MAX),
            None,
            "offset + size must not wrap around: use checked_add"
        );
        assert_eq!(read_at::<u32>(&[], 0), None);
        assert_eq!(read_at::<u64>(&buffer.0, 8), Some(0xAAAA_AAAA_AAAA_AAAA));
    }
    #[test]
    fn c_decodes_a_packed_record_from_bytes() {
        let mut buffer = Buffer([0; 16]);
        buffer.0[3] = 2;
        buffer.0[4..8].copy_from_slice(&70_000_u32.to_ne_bytes());
        buffer.0[8..10].copy_from_slice(&8080_u16.to_ne_bytes());
        let wire: Wire = read_at(&buffer.0, 3).unwrap();
        // `assert_eq!(wire.count, ..)` would not compile: the macro takes a
        // reference to its arguments. `{ wire.count }` copies the field first.
        assert_eq!({ wire.kind }, 2);
        assert_eq!({ wire.count }, 70_000);
        assert_eq!({ wire.port }, 8080);
        assert_eq!(
            read_at::<Wire>(&buffer.0, 10),
            None,
            "7 bytes do not fit at 10"
        );
    }
    #[test]
    fn c_technique_reads_are_unaligned() {
        let body = crate::source::body_of(include_str!("l09_cells_layout.rs"), "fn read_at");
        assert!(
            body.contains("read_unaligned"),
            "task C asks for `read_unaligned`. A plain `read` or `*p` at a misaligned \
             address is undefined behavior; a release build does not detect it"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// `left` in a failure message is your prediction; `right` is the real value.
    #[test]
    fn p_predictions() {
        // P1. Sizes in bytes of MyCell<u32>, std's Cell<u64>, and
        // UnsafeCell<u8>: what does interior mutability cost in space?
        let p1: (usize, usize, usize) = guess();
        assert_eq!(
            p1,
            (
                size_of::<MyCell<u32>>(),
                size_of::<Cell<u64>>(),
                size_of::<UnsafeCell<u8>>()
            ),
            "P1: the size of a cell"
        );

        // P2. Which of these hold? Cell<u32>: Send, Cell<u32>: Sync,
        // &Cell<u32>: Send, AtomicU32: Sync.
        let p2: (bool, bool, bool, bool) = guess();
        assert_eq!(
            p2,
            (
                implements!(Cell<u32>: Send),
                implements!(Cell<u32>: Sync),
                implements!(&Cell<u32>: Send),
                implements!(AtomicU32: Sync)
            ),
            "P2: Send and Sync for cells"
        );

        // P3. The same three fields with and without `packed`:
        // size of Padded, size of Wire, alignment of Wire, offset of Wire.count.
        #[repr(C)]
        struct Padded {
            kind: u8,
            count: u32,
            port: u16,
        }
        let p3: (usize, usize, usize, usize) = guess();
        assert_eq!(
            p3,
            (
                size_of::<Padded>(),
                size_of::<Wire>(),
                align_of::<Wire>(),
                std::mem::offset_of!(Wire, count)
            ),
            "P3: repr(C) with and without packed"
        );

        // P4. A Wire at the start of 4-aligned storage. Is a pointer to its
        // `count` field aligned for u32? And how large is the wrapper?
        #[repr(C, align(4))]
        struct Framed(Wire);
        let framed = Framed(Wire {
            kind: 1,
            count: 2,
            port: 3,
        });
        let field = &raw const framed.0.count;
        let p4: (bool, usize) = guess();
        assert_eq!(
            p4,
            (field.is_aligned(), size_of::<Framed>()),
            "P4: a field pointer inside a packed struct"
        );
    }
}
