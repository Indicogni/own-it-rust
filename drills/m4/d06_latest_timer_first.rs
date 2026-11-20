//! DRILL M4-06 · Which timer fires first
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m4`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which element does `BinaryHeap::pop` return?

use std::collections::BinaryHeap;

fn main() {
    let mut deadlines = BinaryHeap::new();
    for millis in [30u64, 10, 20] {
        deadlines.push(millis);
    }
    let mut fired = Vec::new();
    while let Some(millis) = deadlines.pop() {
        fired.push(millis);
    }
    assert_eq!(fired, [10, 20, 30]); // KEEP
}
