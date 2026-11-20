//! DRILL 19-04 · Two scoped workers, one mutable slice
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0499
//!
//! Run `cargo drill ch19`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: can two threads hold `&mut` to the same slice?

use std::thread;

// FIX: each closure captured the whole array mutably; the compiler does not
// look at the index ranges. `split_at_mut` hands out two `&mut` slices that
// cannot overlap, and each worker takes one of them.
fn main() {
    let mut pixels = [0u8; 6];
    let (left, right) = pixels.split_at_mut(3);
    thread::scope(|s| {
        s.spawn(|| left.fill(1));
        s.spawn(|| right.fill(2));
    });
    assert_eq!(pixels, [1, 1, 1, 2, 2, 2]); // KEEP
}
