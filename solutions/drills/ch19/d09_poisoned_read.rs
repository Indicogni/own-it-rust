//! DRILL 19-09 · Reading after a worker panicked
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch19`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: does a panic while locked destroy the data?

use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

// FIX: the worker panicked while it held the guard, so the mutex is poisoned
// and `lock()` returns `Err`. The data is still there, with the pushed 2. The
// error carries the guard; `into_inner` takes it out. That is a deliberate
// decision that the value is still usable, which is true for this Vec.
fn main() {
    let items = Arc::new(Mutex::new(vec![1]));
    let worker = Arc::clone(&items);
    let crashed = thread::spawn(move || {
        let mut guard = worker.lock().unwrap();
        guard.push(2);
        panic!("worker failed after pushing");
    })
    .join();
    assert!(crashed.is_err()); // KEEP
    let items = items.lock().unwrap_or_else(PoisonError::into_inner);
    assert_eq!(*items, [1, 2]); // KEEP
}
