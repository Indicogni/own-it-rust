//! DRILL M2-10 · A counter in a const
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: panic
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many `Cell` values does this program create?

use std::cell::Cell;

// FIX: a `const` is not a place in memory. Each use of `HITS` created a new
// `Cell` with the value 0, changed it, and threw it away, so the final `get`
// read a fresh 0. Shared state needs one place: a `static`. A `Cell` is not
// `Sync`, so it cannot be an ordinary `static`; `thread_local!` gives each
// thread its own.
thread_local! {
    static HITS: Cell<u32> = const { Cell::new(0) };
}

fn record() {
    HITS.set(HITS.get() + 1);
}

fn main() {
    record();
    record();
    assert_eq!(HITS.get(), 2); // KEEP
}
