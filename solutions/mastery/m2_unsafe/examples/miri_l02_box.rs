//! MIRI LAB 02 — Moving a value out of a box (lesson 02)
//!
//! Run it under Miri, from the repository root:
//!
//!   cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l02_box
//!
//! 1. PREDICT. Read the program. Every `unsafe` block has a SAFETY comment.
//!    Decide which comment is false or leaves something out, and what Miri
//!    will report, before you run anything.
//! 2. RUN the command above. Read the first `error:` line and the source line
//!    that it points to.
//! 3. COMPARE with "What Miri says about the wrong version" in CONCEPTS.md,
//!    lesson 02.
//! 4. REPAIR the program until Miri finishes without a report. Do not change
//!    `main`. Correct the SAFETY comments too.
//!
//! A native `cargo run --example miri_l02_box` may print the expected result, print
//! something else, or crash. That is what undefined behaviour means: only the
//! Miri run tells you whether the program is right.

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::ptr::NonNull;

/// One value on the heap. (No zero-sized types, to keep the example short.)
struct MiniBox<T> {
    ptr: NonNull<T>,
}

impl<T> MiniBox<T> {
    fn new(value: T) -> MiniBox<T> {
        let layout = Layout::new::<T>();
        assert!(
            layout.size() > 0,
            "MiniBox does not support zero-sized types"
        );
        // SAFETY: the layout has a non-zero size.
        let raw = unsafe { alloc(layout) }.cast::<T>();
        let ptr = NonNull::new(raw).unwrap_or_else(|| handle_alloc_error(layout));
        // SAFETY: the block is new, large enough and aligned for a `T`.
        unsafe { ptr.write(value) };
        MiniBox { ptr }
    }

    /// Moves the value out of the box.
    fn into_inner(self) -> T {
        // FIX: `self` was dropped at the end of this function, so
        // `Drop for MiniBox` dropped the value a second time and the caller
        // was left with a `T` whose resources were already gone. Now the
        // destructor is switched off and only the block is freed.
        let this = std::mem::ManuallyDrop::new(self);
        // SAFETY: `ptr` is aligned and points to an initialized `T`. The copy
        // made by `read` becomes the only owner: `Drop for MiniBox` will not
        // run, because the box is wrapped in `ManuallyDrop`.
        let value = unsafe { this.ptr.read() };
        // SAFETY: the block came from `alloc` with this layout in `new`, and
        // nothing uses it after this line.
        unsafe { dealloc(this.ptr.as_ptr().cast::<u8>(), Layout::new::<T>()) };
        value
    }
}

impl<T> Drop for MiniBox<T> {
    fn drop(&mut self) {
        // SAFETY: `ptr` points to an initialized `T` that this box owns, and
        // the block came from `alloc` with this layout in `new`.
        unsafe {
            self.ptr.drop_in_place();
            dealloc(self.ptr.as_ptr().cast::<u8>(), Layout::new::<T>());
        }
    }
}

fn main() {
    let boxed = MiniBox::new(String::from("moved out of the box"));
    let text = boxed.into_inner();
    let first = text.as_bytes()[0];
    println!(
        "{} bytes, the first one is {:?}",
        text.len(),
        char::from(first)
    );
    assert_eq!(first, b'm');
}
