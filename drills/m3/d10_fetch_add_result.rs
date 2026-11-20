//! DRILL M3-10 · The value that `fetch_add` returns
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: does `fetch_add` return the value before or after the addition?

use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

/// Hands out 1, 2, 3, ... Each caller gets a different number.
fn next_id() -> u32 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed) + 1
}

fn main() {
    assert_eq!([next_id(), next_id(), next_id()], [1, 2, 3]); // KEEP
}
