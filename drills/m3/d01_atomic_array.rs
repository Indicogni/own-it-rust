//! DRILL M3-01 · An array of counters
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does `[value; 4]` do with `value`, and can an atomic be copied?

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

fn main() {
    let hits = [AtomicUsize::new(0); 4];
    thread::scope(|s| {
        for worker in 0..8 {
            let hits = &hits;
            s.spawn(move || hits[worker % 4].fetch_add(1, Ordering::Relaxed)); // KEEP
        }
    });
    let totals = hits.map(AtomicUsize::into_inner);
    assert_eq!(totals, [2, 2, 2, 2]); // KEEP
}
