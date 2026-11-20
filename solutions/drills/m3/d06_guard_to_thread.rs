//! DRILL M3-06 · Handing a guard to a worker
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0277
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which thread must unlock a `std::sync::Mutex`?

use std::sync::Mutex;
use std::thread;

fn main() {
    let total = Mutex::new(0);
    thread::scope(|s| {
        // `MutexGuard` is not `Send`: on some systems a mutex must be
        // unlocked by the thread that locked it. Share the mutex (`&Mutex<T>`
        // is `Send` when `T: Send`) and lock it inside the worker.
        s.spawn(|| *total.lock().unwrap() += 5);
    });
    assert_eq!(*total.lock().unwrap(), 5); // KEEP
}
