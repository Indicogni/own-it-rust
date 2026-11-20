//! DRILL 19-01 · Spawning a borrowing closure
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0373
//!
//! Run `cargo drill ch19`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: could the worker outlive `main`'s locals?

use std::thread;

// FIX: `thread::spawn` needs a closure that borrows nothing from this stack
// frame, because the worker may still run after `main` has returned. `move`
// gives the vector to the worker. (`thread::scope` is the other repair: it
// lets the worker borrow `words`, because the scope joins it before returning.)
fn main() {
    let words = vec!["fork", "join"];
    let worker = thread::spawn(move || words.len());
    assert_eq!(worker.join().unwrap(), 2); // KEEP
}
