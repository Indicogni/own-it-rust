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
            weak: Cell::new(0),
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
        // SAFETY: a `Strong` exists, so the allocation is alive.
        let shared = unsafe { self.0.as_ref() };
        shared.strong.set(shared.strong.get() - 1);
        if shared.strong.get() == 0 {
            // SAFETY: this was the last owner, so nobody needs the value any
            // more. The allocation came from `Box::new` in `new`.
            drop(unsafe { Box::from_raw(self.0.as_ptr()) });
        }
    }
}

impl<T> Weak<T> {
    /// True while the value is still alive.
    fn is_alive(&self) -> bool {
        // SAFETY: the pointer came from a `Strong`, and the counts are plain
        // integers, so reading them cannot go wrong.
        unsafe { self.0.as_ref() }.strong.get() > 0
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        // SAFETY: as in `is_alive`.
        let shared = unsafe { self.0.as_ref() };
        shared.weak.set(shared.weak.get() - 1);
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
