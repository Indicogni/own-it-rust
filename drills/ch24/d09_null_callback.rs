//! DRILL 24-09 · Null function pointers
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch24`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: can a Rust function pointer ever be null?

use std::sync::atomic::{AtomicU32, Ordering};

type Callback = extern "C" fn();

static TICKS: AtomicU32 = AtomicU32::new(0);

extern "C" fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

fn main() {
    // A C library would start with "no callback registered yet".
    let mut registered: Callback = std::ptr::null();
    registered = tick;
    registered();
    assert_eq!(TICKS.load(Ordering::Relaxed), 1); // KEEP
}
