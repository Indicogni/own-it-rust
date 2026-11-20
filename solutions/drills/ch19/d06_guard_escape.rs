//! DRILL 19-06 · Returning a reference out of a guard
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0515
//!
//! Run `cargo drill ch19`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what happens to the lock when the guard is dropped?

use std::sync::Mutex;

// FIX: the reference pointed into data that is reachable only through the
// guard, and the guard is dropped (and the lock released) when the function
// returns. An owned copy can leave the function; the borrowed element cannot.
fn first(queue: &Mutex<Vec<String>>) -> String {
    queue.lock().unwrap()[0].clone()
}

fn main() {
    let queue = Mutex::new(vec![String::from("job-1")]);
    assert_eq!(first(&queue), "job-1"); // KEEP
}
