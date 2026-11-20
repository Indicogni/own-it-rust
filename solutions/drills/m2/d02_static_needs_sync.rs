//! DRILL M2-02 · An UnsafeCell in a static
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0277
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who is allowed to read a `static`, and what must its type promise for that?

use std::sync::atomic::{AtomicU32, Ordering};

// FIX: every thread can reach a `static`, so its type must be `Sync`.
// `UnsafeCell` never is. Writing `unsafe impl Sync for Slot` would compile,
// but then `next_id` would be a data race as soon as two threads call it: the
// SAFETY comment ("only `main` calls this") is a hope, not something the type
// enforces. An atomic integer is `Sync` and needs no `unsafe` at all.
static NEXT_ID: AtomicU32 = AtomicU32::new(1);

fn next_id() -> u32 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

fn main() {
    assert_eq!(next_id(), 1); // KEEP
    assert_eq!(next_id(), 2); // KEEP
}
