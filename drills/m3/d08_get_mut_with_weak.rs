//! DRILL M3-08 · `Arc::get_mut` with an observer
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what could the observer do while you hold `&mut` to the value?

use std::sync::Arc;

fn main() {
    let mut config = Arc::new(vec![1, 2, 3]);
    let observer = Arc::downgrade(&config); // KEEP
    Arc::get_mut(&mut config).unwrap().push(4); // KEEP
    assert_eq!(observer.upgrade().unwrap().len(), 4); // KEEP
}
