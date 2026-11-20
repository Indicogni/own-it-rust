//! DRILL 13-12 · A guard in a while-let condition
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch13`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how long does the temporary from borrow_mut() in the `while let` line live?

use std::cell::RefCell;

// FIX: the guard is now a temporary inside this function and is dropped when it
// returns. Written directly in the `while let` condition, the guard lives until
// the end of each loop turn, so the `push` in the body found the queue borrowed.
fn next_job(queue: &RefCell<Vec<i32>>) -> Option<i32> {
    queue.borrow_mut().pop()
}

fn main() {
    let queue = RefCell::new(vec![1, 2]);
    let mut seen = Vec::new();
    while let Some(job) = next_job(&queue) {
        if job == 2 {
            queue.borrow_mut().push(10); // KEEP
        }
        seen.push(job); // KEEP
    }
    assert_eq!(seen, [2, 10, 1]); // KEEP
}
