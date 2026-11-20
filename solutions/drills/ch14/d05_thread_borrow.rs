//! DRILL 14-05 · A thread borrowing a local
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0373
//!
//! Run `cargo drill ch14`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: could the thread outlive `main`'s locals?

use std::thread;

fn main() {
    let words = vec!["a", "bb", "ccc"];
    // FIX: `move` gives the vector to the thread, so the closure borrows nothing
    // from `main` and satisfies the `'static` bound of `thread::spawn`.
    let handle = thread::spawn(move || words.iter().map(|w| w.len()).sum::<usize>());
    assert_eq!(handle.join().unwrap(), 6); // KEEP
}
