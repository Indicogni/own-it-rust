//! DRILL M3-08 · `Arc::get_mut` with an observer
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what could the observer do while you hold `&mut` to the value?

use std::sync::Arc;

fn main() {
    let mut config = Arc::new(vec![1, 2, 3]);
    // `get_mut` returns `Some` only for the one and only pointer to the
    // block, counting `Weak`s too: a `Weak` could be upgraded on another
    // thread at any moment, and the new `Arc` would read the value while it
    // is being changed. So change the value first and hand out the observer
    // afterwards. (`Arc::make_mut` is not the fix: with a `Weak` around it
    // moves the value to a new block, and the observer is left with nothing.)
    Arc::get_mut(&mut config).unwrap().push(4); // KEEP
    let observer = Arc::downgrade(&config); // KEEP
    assert_eq!(observer.upgrade().unwrap().len(), 4); // KEEP
}
