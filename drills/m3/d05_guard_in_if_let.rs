//! DRILL M3-05 · A guard inside an `if let` head
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: in edition 2024, is the temporary guard still alive in the `else` block?

use std::sync::Mutex;

fn main() {
    let jobs: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    if let Some(job) = jobs.lock().unwrap().pop() {
        println!("running job {job}");
    } else {
        jobs.try_lock().unwrap().push(1); // KEEP
    }
    assert_eq!(*jobs.lock().unwrap(), [1]); // KEEP
}
