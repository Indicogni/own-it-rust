//! DRILL 19-02 · Arc does not make RefCell thread-safe
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0277
//!
//! Run `cargo drill ch19`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which trait lets two threads share `&T`?

use std::sync::{Arc, Mutex};
use std::thread;

// FIX: `Arc` shares a value between threads only if the value is `Sync`.
// `RefCell` counts its borrows in a plain integer, which two threads could
// change at the same time, so it is not `Sync`. `Mutex` is the thread-safe
// tool for the same job: `lock()` takes the place of `borrow_mut()`.
fn main() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let writer = Arc::clone(&log);
    thread::spawn(move || writer.lock().unwrap().push("worker"))
        .join()
        .unwrap();
    let entries = log.lock().unwrap().clone();
    assert_eq!(entries, ["worker"]); // KEEP
}
