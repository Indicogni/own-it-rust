//! LESSON 02 — Layout, alloc, dealloc and NonNull: building Box
//! Run: cargo test --manifest-path mastery/m2_unsafe/Cargo.toml l02_
//! Miri: cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml l02_
//! Read first: "Lesson 02" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. The allocator's `dealloc` takes a `Layout`. Where does a box keep it?
//! 2. What must `MyBox::new(())` return, when asking the allocator for zero
//!    bytes is forbidden?
//!
//! IMPLEMENT the tasks below. The fields and signatures are given. Use
//! `std::alloc::{alloc, dealloc, handle_alloc_error}` directly: no `Box`, no
//! `Vec`. Write a `// SAFETY:` comment for every `unsafe` block, and complete
//! the `# Safety` section of `from_raw`. Keep the supplied tests and add one
//! edge case.
//! EXPLAIN afterward: List the three facts in the invariant of `MyBox` and say
//! which method establishes each one and which methods rely on it. How do
//! `into_inner`, `into_raw` and `leak` avoid running `Drop for MyBox`?
//!
//! MIRI LAB: `examples/miri_l02_box.rs` has an `into_inner` that looks right and
//! passes a quick native check. Predict what Miri reports, run
//! `cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l02_box`,
//! compare with CONCEPTS.md, then repair the example until Miri is silent.
//!
//! Lesson 07 comes back to this type and asks when it may cross threads.

use std::alloc::{self, Layout};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

/// One value on the heap, with one owner.
///
/// Invariant:
/// 1. `ptr` is non-null, aligned for `T`, and points to an initialized `T`.
/// 2. If `T` has a size above zero, `ptr` came from the global allocator with
///    `Layout::new::<T>()`, and this box is the only owner of that block.
/// 3. If `T` is zero-sized, there is no block: `ptr` is `NonNull::dangling()`.
pub struct MyBox<T> {
    ptr: NonNull<T>,
    /// Tells the compiler that a `MyBox<T>` owns a `T`, as a `T` field would.
    _owns: PhantomData<T>,
}

impl<T> MyBox<T> {
    /// A. Moves `value` to the heap. Never asks the allocator for zero bytes.
    /// If the allocator returns null, calls `handle_alloc_error`.
    pub fn new(value: T) -> MyBox<T> {
        todo!("Implement this task")
    }

    /// B. Moves the value out and frees the block. The value's destructor does
    /// not run here: the caller now owns the value.
    pub fn into_inner(self) -> T {
        todo!("Implement this task")
    }

    /// C. Gives up ownership and returns the pointer. Nothing is dropped or
    /// freed. The pointer stays valid until it is passed to `from_raw`.
    pub fn into_raw(self) -> *mut T {
        todo!("Implement this task")
    }

    /// C. Takes ownership back.
    ///
    /// # Safety
    ///
    /// TODO: state what the caller must guarantee about `ptr`.
    pub unsafe fn from_raw(ptr: *mut T) -> MyBox<T> {
        todo!("Implement this task")
    }

    /// C. Gives up ownership and returns a reference that lives as long as the
    /// caller likes. The value is never dropped unless the caller turns the
    /// reference back into a box with `from_raw`.
    pub fn leak<'a>(self) -> &'a mut T
    where
        T: 'a,
    {
        todo!("Implement this task")
    }
}

impl<T> Deref for MyBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        todo!("Implement this task")
    }
}

impl<T> DerefMut for MyBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        todo!("Implement this task")
    }
}

impl<T> Drop for MyBox<T> {
    /// A. Runs the value's destructor, then frees the block.
    fn drop(&mut self) {
        todo!("Implement this task")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::{audit, counting};
    use std::cell::Cell;

    /// Counts how often values of this type are dropped.
    struct Counted<'a>(&'a Cell<u32>);

    impl Drop for Counted<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[repr(align(64))]
    struct Wide(u8);

    /// A zero-sized type with an alignment and a destructor.
    #[repr(align(16))]
    struct Marker;

    thread_local! {
        static MARKERS_DROPPED: Cell<u32> = const { Cell::new(0) };
    }

    impl Drop for Marker {
        fn drop(&mut self) {
            MARKERS_DROPPED.with(|count| count.set(count.get() + 1));
        }
    }

    /// Compiles only if `MyBox<T>` is covariant in `T`, as `Box<T>` is: a box of
    /// a long-lived reference can be used as a box of a shorter-lived one.
    fn _covariant<'a>(long: MyBox<&'static str>) -> MyBox<&'a str> {
        long
    }

    #[test]
    fn a_box_reads_and_writes_its_value() {
        let mut boxed = MyBox::new(String::from("heap"));
        assert_eq!(*boxed, "heap");
        assert_eq!(boxed.len(), 4, "method calls must reach the String");
        boxed.push_str("ed");
        *boxed += "!";
        assert_eq!(*boxed, "heaped!");
    }

    #[test]
    fn a_drop_runs_the_destructor_once_and_frees_the_block() {
        let drops = Cell::new(0);
        let ((), heap) = audit(|| {
            let boxed = MyBox::new((Counted(&drops), 7u64));
            assert_eq!(boxed.1, 7);
            assert_eq!(drops.get(), 0, "the value must stay alive while the box is");
        });
        assert_eq!(
            drops.get(),
            1,
            "dropping the box must run the value's destructor exactly once"
        );
        assert_eq!(
            heap.live, 0,
            "dropping the box must free its block ({} block(s) are still allocated)",
            heap.live
        );
        assert_eq!(
            heap.bad_frees, 0,
            "the block was freed with a layout other than Layout::new::<T>()"
        );
    }

    #[test]
    fn a_block_has_the_size_and_alignment_of_the_type() {
        let (boxed, heap) = audit(|| MyBox::new(Wide(3)));
        assert_eq!(
            std::ptr::from_ref::<Wide>(&boxed).addr() % 64,
            0,
            "the value of a type with align(64) must sit at a multiple of 64"
        );
        assert_eq!(boxed.0, 3);
        if counting() {
            assert!(
                heap.requested(Layout::new::<Wide>()),
                "MyBox::new must ask the allocator for exactly Layout::new::<T>()"
            );
        }
        let ((), heap) = audit(|| drop(boxed));
        if counting() {
            assert_eq!(heap.live, -1, "dropping the box must free one block");
        }
        // The block was allocated before this audit began, so check the pair of
        // calls in one audit as well.
        let ((), heap) = audit(|| drop(MyBox::new(Wide(4))));
        assert_eq!(
            heap.bad_frees, 0,
            "dealloc must get the same layout as alloc: Layout::new::<T>()"
        );
    }

    #[test]
    fn a_zero_sized_value_needs_no_block() {
        let (boxed, heap) = audit(|| MyBox::new(Marker));
        assert_eq!(
            heap.allocations, 0,
            "a zero-sized value must not reach the allocator at all"
        );
        assert_eq!(
            std::ptr::from_ref::<Marker>(&boxed).addr() % align_of::<Marker>(),
            0,
            "even a pointer to a zero-sized value must be aligned"
        );
        let before = MARKERS_DROPPED.with(Cell::get);
        drop(boxed);
        assert_eq!(
            MARKERS_DROPPED.with(Cell::get) - before,
            1,
            "a zero-sized value still has a destructor, and the box must run it once"
        );
        let unit = MyBox::new(());
        let () = *unit;
        assert!(MyBox::new([0u64; 0]).into_inner().is_empty());
    }

    #[test]
    fn a_box_is_one_pointer_and_never_null() {
        assert_eq!(*MyBox::new(1u64), 1);
        assert_eq!(size_of::<MyBox<[u64; 8]>>(), size_of::<usize>());
        assert_eq!(
            size_of::<Option<MyBox<u64>>>(),
            size_of::<usize>(),
            "NonNull lets Option use the null pointer for None"
        );
    }

    #[test]
    fn a_moving_the_box_does_not_move_the_value() {
        let boxed = MyBox::new([1u8, 2, 3]);
        let before = std::ptr::from_ref::<[u8; 3]>(&boxed);
        let shelf = [boxed];
        assert!(
            std::ptr::eq(&*shelf[0], before),
            "moving a box copies its pointer; the value must stay where it is"
        );
    }

    #[test]
    fn b_into_inner_moves_the_value_out_and_frees_the_block() {
        let drops = Cell::new(0);
        let boxed = MyBox::new((Counted(&drops), String::from("payload")));
        let text = boxed.1.as_ptr();
        let (value, heap) = audit(|| boxed.into_inner());
        assert_eq!(
            drops.get(),
            0,
            "into_inner must not drop the value: the caller owns it now"
        );
        assert!(
            std::ptr::eq(value.1.as_ptr(), text),
            "into_inner must move the String (same text buffer), not clone it"
        );
        assert_eq!(heap.allocations, 0, "into_inner must not allocate");
        if counting() {
            assert_eq!(
                heap.live, -1,
                "into_inner must free exactly one block: the box's own"
            );
        }
        drop(value);
        assert_eq!(
            drops.get(),
            1,
            "the value is dropped once, by its new owner"
        );
    }

    #[test]
    fn c_raw_round_trip_keeps_the_block() {
        let drops = Cell::new(0);
        let boxed = MyBox::new((Counted(&drops), 10u32));
        let address = std::ptr::from_ref(&*boxed);
        let (back, heap) = audit(|| {
            let raw = boxed.into_raw();
            assert!(
                std::ptr::eq(raw, address),
                "into_raw must return the address of the value"
            );
            // SAFETY: `raw` is the pointer of a live box value; nothing else
            // uses it during this write.
            unsafe { (*raw).1 += 1 };
            // SAFETY: `raw` came from `into_raw` just above and is passed back once.
            unsafe { MyBox::from_raw(raw) }
        });
        assert_eq!(
            drops.get(),
            0,
            "into_raw and from_raw must not drop the value"
        );
        assert_eq!(
            heap.allocations, 0,
            "into_raw and from_raw must not allocate"
        );
        assert_eq!(
            heap.live, 0,
            "into_raw and from_raw must not free the block"
        );
        assert_eq!(back.1, 11);
        drop(back);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn c_leak_hands_out_the_value_for_any_lifetime() {
        let drops = Cell::new(0);
        let leaked: &mut (Counted<'_>, u8) = MyBox::new((Counted(&drops), 1)).leak();
        leaked.1 += 1;
        assert_eq!(leaked.1, 2);
        assert_eq!(drops.get(), 0, "a leaked value is never dropped");
        // Take it back so that the test itself leaks nothing.
        // SAFETY: the reference came from `leak` and is not used again.
        drop(unsafe { MyBox::from_raw(leaked) });
        assert_eq!(drops.get(), 1);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Size and alignment that the allocator is asked for.
        let pair = Layout::new::<(u8, u32)>();
        let p1: (usize, usize) = guess();
        assert_eq!(
            p1,
            (pair.size(), pair.align()),
            "P1: layout of (u8, u32) (left: your prediction, right: actual)"
        );

        // P2. Sizes in bytes on a 64-bit target:
        // (Option<NonNull<u8>>, Option<*mut u8>, Option<Box<u8>>).
        let p2: (usize, usize, usize) = guess();
        assert_eq!(
            p2,
            (
                size_of::<Option<NonNull<u8>>>(),
                size_of::<Option<*mut u8>>(),
                size_of::<Option<Box<u8>>>()
            ),
            "P2: which pointer types have a niche (left: your prediction, right: actual)"
        );

        // P3. Size of each layout: (three u16 in an array, the unit type, [u32; 0]),
        // and the alignment of the last one.
        let p3: (usize, usize, usize, usize) = guess();
        assert_eq!(
            p3,
            (
                Layout::array::<u16>(3).unwrap().size(),
                Layout::new::<()>().size(),
                Layout::new::<[u32; 0]>().size(),
                Layout::new::<[u32; 0]>().align()
            ),
            "P3: array and zero-sized layouts (left: your prediction, right: actual)"
        );
    }
}
