//! DRILL M4-05 · A heap of deadlines and wakers
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0277
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
    // FIX: a `Waker` cannot be compared, so it cannot be part of a heap
    // element. The heap orders (deadline, id); the wakers live in a map.
    let mut wakers = HashMap::new();
    let mut timers = Vec::new();
    for (id, deadline) in deadlines.into_iter().enumerate() {
        wakers.insert(id, Waker::noop().clone());
        timers.push((deadline, id));
    }
    let (first, id) = earliest(timers).unwrap();
    let waker = wakers.remove(&id).unwrap();
    waker.wake(); // KEEP
    assert_eq!(first, deadlines[1]); // KEEP
}
