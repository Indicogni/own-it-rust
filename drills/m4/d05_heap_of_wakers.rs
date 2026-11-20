//! DRILL M4-05 · A heap of deadlines and wakers
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m4`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what must a `BinaryHeap` be able to do with any two of its elements?

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::task::Waker;
use std::time::{Duration, Instant};

/// The smallest item, found with a heap.
fn earliest<T: Ord>(items: Vec<T>) -> Option<T> {
    let mut heap: BinaryHeap<Reverse<T>> = items.into_iter().map(Reverse).collect();
    heap.pop().map(|Reverse(item)| item)
}

fn main() {
    let start = Instant::now();
    let deadlines = [30u64, 10, 20].map(|ms| start + Duration::from_millis(ms)); // KEEP
    let mut timers = Vec::new();
    for deadline in deadlines {
        timers.push((deadline, Waker::noop().clone()));
    }
    let (first, waker) = earliest(timers).unwrap();
    waker.wake(); // KEEP
    assert_eq!(first, deadlines[1]); // KEEP
}
