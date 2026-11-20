//! DRILL 19-07 · A Cell in a static
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0277
//!
//! Run `cargo drill ch19`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many threads can see a static at once?

use std::sync::atomic::{AtomicU32, Ordering};

// FIX: every thread can reach a `static`, so its type must be `Sync`, and
// `Cell` is not. An atomic integer is the `Sync` counterpart. `fetch_add`
// returns the old value, so the new total is one more.
static REQUESTS: AtomicU32 = AtomicU32::new(0);

/// Counts one request and returns the new total.
fn record() -> u32 {
    REQUESTS.fetch_add(1, Ordering::Relaxed) + 1
}

fn main() {
    assert_eq!(record(), 1); // KEEP
    assert_eq!(record(), 2); // KEEP
}
