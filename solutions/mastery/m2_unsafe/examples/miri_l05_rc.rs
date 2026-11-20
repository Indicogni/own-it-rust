//! MIRI LAB 05 — A weak pointer that outlives its memory (lesson 05)
//!
//! Run it under Miri, from the repository root:
//!
//!   cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l05_rc
//!
//! 1. PREDICT. Read the program. Every `unsafe` block has a SAFETY comment.
//!    Decide which comment is false or leaves something out, and what Miri
//!    will report, before you run anything.
//! 2. RUN the command above. Read the first `error:` line and the source line
//!    that it points to.
//! 3. COMPARE with "What Miri says about the wrong version" in CONCEPTS.md,
//!    lesson 05.
//! 4. REPAIR the program until Miri finishes without a report. Do not change
//!    `main`. Correct the SAFETY comments too.
//!
//! A native `cargo run --example miri_l05_rc` may print the expected result, print
//! something else, or crash. That is what undefined behaviour means: only the
//! Miri run tells you whether the program is right.

use std::cell::Cell;
use std::ptr::NonNull;

struct Shared<T> {
    strong: Cell<usize>,
    /// The number of `Weak` pointers, plus one while any `Strong` exists.
    weak: Cell<usize>,
    value: Option<T>,
}

/// An owner of the shared value.
struct Strong<T>(NonNull<Shared<T>>);

/// A pointer that does not keep the value alive.
struct Weak<T>(NonNull<Shared<T>>);

impl<T> Strong<T> {
    fn new(value: T) -> Strong<T> {
        let shared = Box::new(Shared {
            strong: Cell::new(1),
            // FIX, part 1: the owners together hold one weak reference.
            weak: Cell::new(1),
            value: Some(value),
        });
        Strong(NonNull::from(Box::leak(shared)))
    }

    fn get(&self) -> &T {
        // SAFETY: a `Strong` exists, so the allocation is alive and the value
        // has not been dropped. Nothing changes the value while it is shared.
        let shared = unsafe { self.0.as_ref() };
        shared
            .value
            .as_ref()
            .expect("the value lives as long as an owner")
    }

    fn downgrade(&self) -> Weak<T> {
        // SAFETY: a `Strong` exists, so the allocation is alive.
        let shared = unsafe { self.0.as_ref() };
        shared.weak.set(shared.weak.get() + 1);
        Weak(self.0)
    }
}

impl<T> Drop for Strong<T> {
    fn drop(&mut self) {
        // SAFETY: a `Strong` exists, so the allocation is alive. The shared
        // reference ends before the block is touched in any other way.
        let last = unsafe {
            let shared = self.0.as_ref();
            shared.strong.set(shared.strong.get() - 1);
            shared.strong.get() == 0
        };
        if last {
            // FIX, part 2: the last owner drops the *value*, but the
            // allocation must stay while `Weak` pointers can still read the
            // counts. Giving up the owners' weak reference frees it if no
            // `Weak` is left. (This lab keeps things simple with an `Option`;
            // lesson 05 drops the value in place instead.)
            // SAFETY: no other `Strong` exists, and a `Weak` only reads the
            // counts, so nothing else is using `value`.
            unsafe { (*self.0.as_ptr()).value = None };
            drop(Weak(self.0));
        }
    }
}

impl<T> Weak<T> {
    /// True while the value is still alive.
    fn is_alive(&self) -> bool {
        // SAFETY: a `Weak` exists, so `weak > 0` and the allocation is alive.
        // Only the `strong` field is borrowed, not `value`.
        unsafe { &(*self.0.as_ptr()).strong }.get() > 0
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        // SAFETY: a `Weak` exists, so the allocation is alive. Only the
        // `weak` field is borrowed.
        let weak = unsafe { &(*self.0.as_ptr()).weak };
        weak.set(weak.get() - 1);
        if weak.get() == 0 {
            // SAFETY: no `Weak` is left, and the owners' shared weak
            // reference is gone too, so no pointer to the allocation remains.
            // It came from `Box::new` in `Strong::new`. The value was already
            // set to `None`.
            drop(unsafe { Box::from_raw(self.0.as_ptr()) });
        }
    }
}

fn main() {
    let strong = Strong::new(String::from("shared"));
    let weak = strong.downgrade();
    println!(
        "{}: alive while an owner exists: {}",
        strong.get(),
        weak.is_alive()
    );
    drop(strong);
    println!("alive after the last owner is gone: {}", weak.is_alive());
    assert!(!weak.is_alive());
}
