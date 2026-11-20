//! MIRI LAB 08 — A bump allocator with one check missing (lesson 08)
//!
//! Run it under Miri, from the repository root:
//!
//!   cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l08_arena
//!
//! 1. PREDICT. Read the program. Every `unsafe` block has a SAFETY comment.
//!    Decide which comment is false or leaves something out, and what Miri
//!    will report, before you run anything.
//! 2. RUN the command above. Read the first `error:` line and the source line
//!    that it points to.
//! 3. COMPARE with "What Miri says about the wrong version" in CONCEPTS.md,
//!    lesson 08.
//! 4. REPAIR the program until Miri finishes without a report. Do not change
//!    `main`. Correct the SAFETY comments too.
//!
//! A native `cargo run --example miri_l08_arena` may print the expected result, print
//! something else, or crash. That is what undefined behaviour means: only the
//! Miri run tells you whether the program is right.

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::cell::Cell;
use std::ptr::NonNull;

const SIZE: usize = 64;

/// Hands out pieces of one 64-byte block, one after the other.
struct Bump {
    block: NonNull<u8>,
    used: Cell<usize>,
}

impl Bump {
    fn layout() -> Layout {
        Layout::from_size_align(SIZE, 8).expect("64 and 8 form a valid layout")
    }

    fn new() -> Bump {
        // SAFETY: the layout has a non-zero size.
        let raw = unsafe { alloc(Bump::layout()) };
        let block = NonNull::new(raw).unwrap_or_else(|| handle_alloc_error(Bump::layout()));
        Bump {
            block,
            used: Cell::new(0),
        }
    }

    /// Moves a value without a destructor into the block.
    // An arena is the standard exception to this lint: every call returns a
    // different piece of memory.
    #[allow(clippy::mut_from_ref)]
    fn alloc<T: Copy>(&self, value: T) -> &mut T {
        // FIX: a value must start at a multiple of its alignment. The block
        // itself starts at a multiple of 8, so rounding the *offset* up is
        // enough for every type with an alignment of at most 8.
        assert!(align_of::<T>() <= 8, "the block is only aligned to 8");
        let start = self.used.get().next_multiple_of(align_of::<T>());
        let end = start + size_of::<T>();
        assert!(end <= SIZE, "the block is full");
        self.used.set(end);
        // SAFETY: the bytes `start..end` are inside the block and were not
        // handed out before, and `start` is a multiple of the alignment of
        // `T` in a block that is aligned to 8, so the slot is aligned and
        // valid for a write of `T`, and the new reference is unique.
        unsafe {
            let slot = self.block.as_ptr().add(start).cast::<T>();
            slot.write(value);
            &mut *slot
        }
    }
}

impl Drop for Bump {
    fn drop(&mut self) {
        // SAFETY: the block came from `alloc` with this layout in `new`.
        unsafe { dealloc(self.block.as_ptr(), Bump::layout()) };
    }
}

fn main() {
    let bump = Bump::new();
    let flag = bump.alloc(1u8);
    let number = bump.alloc(7u64);
    *number += u64::from(*flag);
    println!("flag {flag}, number {number}");
    assert_eq!(*number, 8);
}
