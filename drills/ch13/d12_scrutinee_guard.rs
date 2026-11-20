//! DRILL 13-12 · A guard in a while-let condition
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch13`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how long does the temporary from borrow_mut() in the `while let` line live?

use std::cell::RefCell;

fn main() {
    let queue = RefCell::new(vec![1, 2]);
    let mut seen = Vec::new();
    while let Some(job) = queue.borrow_mut().pop() {
        if job == 2 {
            queue.borrow_mut().push(10); // KEEP
        }
        seen.push(job); // KEEP
    }
    assert_eq!(seen, [2, 10, 1]); // KEEP
}
