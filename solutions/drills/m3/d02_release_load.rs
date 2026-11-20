//! DRILL M3-02 · A load with Release
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: error
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what could a load release, and to whom?

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread;

static DATA: AtomicU32 = AtomicU32::new(0);
static READY: AtomicBool = AtomicBool::new(false);

fn main() {
    let writer = thread::spawn(|| {
        DATA.store(7, Ordering::Relaxed);
        READY.store(true, Ordering::Release); // KEEP
    });
    writer.join().unwrap();
    // `Release` describes a store: "what I did so far is visible to whoever
    // acquires this value". A load has nothing to release, so the compiler
    // rejects it (the `invalid_atomic_ordering` lint is an error by default).
    // The load that pairs with a `Release` store is an `Acquire` load.
    if READY.load(Ordering::Acquire) {
        assert_eq!(DATA.load(Ordering::Relaxed), 7); // KEEP
    }
}
