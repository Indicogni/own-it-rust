//! DRILL 19-08 · Condvar::wait takes the guard
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0382
//!
//! Run `cargo drill ch19`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does `wait` do with the guard you pass it?

use std::sync::{Condvar, Mutex};

// FIX: `wait` takes the guard by value, because it must unlock the mutex
// while the thread sleeps. It returns a new guard when the thread wakes up.
// Store that guard back in the variable, which therefore must be `mut`.
fn main() {
    let ready = Mutex::new(true);
    let changed = Condvar::new();
    let mut guard = ready.lock().unwrap();
    while !*guard {
        guard = changed.wait(guard).unwrap();
    }
    assert!(*guard); // KEEP
}
