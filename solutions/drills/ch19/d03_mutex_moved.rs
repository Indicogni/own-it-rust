//! DRILL 19-03 · A Mutex moved into the first worker
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0382
//!
//! Run `cargo drill ch19`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many owners can a plain Mutex have?

use std::sync::{Arc, Mutex};
use std::thread;

// FIX: a Mutex has one owner like any other value, so the first `move`
// closure took it and nothing was left for the second. `Arc` gives the Mutex
// several owners: each worker moves its own clone of the Arc.
fn main() {
    let total = Arc::new(Mutex::new(0));
    let for_a = Arc::clone(&total);
    let a = thread::spawn(move || *for_a.lock().unwrap() += 1);
    let b = thread::spawn(move || *total.lock().unwrap() += 1);
    a.join().unwrap(); // KEEP
    b.join().unwrap(); // KEEP
}
