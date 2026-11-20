//! MIRI LAB 03 — Filling fresh memory (lesson 03)
//!
//! Run it under Miri, from the repository root:
//!
//!   cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l03_vec
//!
//! 1. PREDICT. Read the program. Every `unsafe` block has a SAFETY comment.
//!    Decide which comment is false or leaves something out, and what Miri
//!    will report, before you run anything.
//! 2. RUN the command above. Read the first `error:` line and the source line
//!    that it points to.
//! 3. COMPARE with "What Miri says about the wrong version" in CONCEPTS.md,
//!    lesson 03.
//! 4. REPAIR the program until Miri finishes without a report. Do not change
//!    `main`. Correct the SAFETY comments too.
//!
//! A native `cargo run --example miri_l03_vec` may print the expected result, print
//! something else, or crash. That is what undefined behaviour means: only the
//! Miri run tells you whether the program is right.

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::ptr::NonNull;

/// A stack with room for a fixed number of values.
struct FixedStack<T> {
    slots: NonNull<T>,
    cap: usize,
    len: usize,
}

impl<T> FixedStack<T> {
    fn new(cap: usize) -> FixedStack<T> {
        let layout = Layout::array::<T>(cap).expect("capacity overflow");
        assert!(
            layout.size() > 0,
            "FixedStack needs room for at least one byte"
        );
        // SAFETY: the layout has a non-zero size.
        let raw = unsafe { alloc(layout) }.cast::<T>();
        let slots = NonNull::new(raw).unwrap_or_else(|| handle_alloc_error(layout));
        FixedStack { slots, cap, len: 0 }
    }

    fn push(&mut self, value: T) {
        assert!(self.len < self.cap, "the stack is full");
        // FIX: an assignment `*slot = value` first drops the old value of the
        // slot. A slot above `len` has no old value: its bytes are whatever
        // the allocator left there. `write` stores the new value without
        // looking at the old bytes.
        // SAFETY: `len < cap`, so the slot is inside the block, and the block
        // is aligned for `T` and owned by this stack. The slot is
        // uninitialized, and `write` does not read or drop it.
        unsafe { self.slots.add(self.len).write(value) };
        self.len += 1;
    }

    fn as_slice(&self) -> &[T] {
        // SAFETY: the first `len` slots are initialized and inside the block.
        unsafe { std::slice::from_raw_parts(self.slots.as_ptr(), self.len) }
    }
}

impl<T> Drop for FixedStack<T> {
    fn drop(&mut self) {
        // SAFETY: the first `len` slots hold values that this stack owns, and
        // the block came from `alloc` with this layout in `new`.
        unsafe {
            std::ptr::slice_from_raw_parts_mut(self.slots.as_ptr(), self.len).drop_in_place();
            let layout = Layout::array::<T>(self.cap).expect("checked in new");
            dealloc(self.slots.as_ptr().cast::<u8>(), layout);
        }
    }
}

fn main() {
    let mut stack = FixedStack::new(2);
    stack.push(String::from("first"));
    stack.push(String::from("second"));
    println!("{:?}", stack.as_slice());
    assert_eq!(stack.as_slice(), ["first", "second"]);
}
