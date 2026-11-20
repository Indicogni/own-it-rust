//! DRILL M2-02 · An UnsafeCell in a static
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who is allowed to read a `static`, and what must its type promise for that?

use std::cell::UnsafeCell;

struct Slot(UnsafeCell<u32>);

static NEXT_ID: Slot = Slot(UnsafeCell::new(1));

fn next_id() -> u32 {
    // SAFETY: only `main` calls this function.
    unsafe {
        let id = *NEXT_ID.0.get();
        *NEXT_ID.0.get() = id + 1;
        id
    }
}

fn main() {
    assert_eq!(next_id(), 1); // KEEP
    assert_eq!(next_id(), 2); // KEEP
}
