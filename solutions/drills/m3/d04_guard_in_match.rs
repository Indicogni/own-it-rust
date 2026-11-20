//! DRILL M3-04 · A guard inside a `match` head
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how long does the temporary guard in the `match` expression live?

use std::sync::Mutex;

fn main() {
    let jobs = Mutex::new(vec![3, 4]);
    // A temporary in the head of a `match` lives until the end of the whole
    // `match`. The guard was therefore still alive inside the arm, and
    // `try_lock` returned `Err`. Take the job out in a statement of its own:
    // the guard is then dropped at that statement's semicolon.
    let job = jobs.lock().unwrap().pop();
    match job {
        Some(job) => jobs.try_lock().unwrap().push(job * 10), // KEEP
        None => println!("nothing to do"),
    }
    assert_eq!(*jobs.lock().unwrap(), [3, 40]); // KEEP
}
