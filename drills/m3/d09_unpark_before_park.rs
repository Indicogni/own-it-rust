//! DRILL M3-09 · Unpark first, park second
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does `unpark` leave behind when the thread is not parked?

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

fn main() {
    // The wake-up comes before the sleep.
    thread::current().unpark(); // KEEP
    thread::park(); // KEEP

    let ready = Arc::new(AtomicBool::new(false));
    let worker = {
        let ready = Arc::clone(&ready);
        thread::spawn(move || {
            while !ready.load(Ordering::Acquire) {
                thread::park();
            }
            "woke up"
        })
    };
    ready.store(true, Ordering::Release); // KEEP
    worker.thread().unpark(); // KEEP
    assert_eq!(worker.join().unwrap(), "woke up"); // KEEP
}
