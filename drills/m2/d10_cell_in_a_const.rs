//! DRILL M2-10 · A counter in a const
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many `Cell` values does this program create?

use std::cell::Cell;

const HITS: Cell<u32> = Cell::new(0);

fn record() {
    HITS.set(HITS.get() + 1);
}

fn main() {
    record();
    record();
    assert_eq!(HITS.get(), 2); // KEEP
}
