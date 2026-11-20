//! DRILL M3-04 · A guard inside a `match` head
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how long does the temporary guard in the `match` expression live?

use std::sync::Mutex;

fn main() {
    let jobs = Mutex::new(vec![3, 4]);
    match jobs.lock().unwrap().pop() {
        Some(job) => jobs.try_lock().unwrap().push(job * 10), // KEEP
        None => println!("nothing to do"),
    }
    assert_eq!(*jobs.lock().unwrap(), [3, 40]); // KEEP
}
