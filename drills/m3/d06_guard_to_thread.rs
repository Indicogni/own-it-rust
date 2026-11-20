//! DRILL M3-06 · Handing a guard to a worker
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which thread must unlock a `std::sync::Mutex`?

use std::sync::Mutex;
use std::thread;

fn main() {
    let total = Mutex::new(0);
    thread::scope(|s| {
        let mut guard = total.lock().unwrap();
        s.spawn(move || *guard += 5);
    });
    assert_eq!(*total.lock().unwrap(), 5); // KEEP
}
