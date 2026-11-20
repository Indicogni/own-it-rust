//! DRILL M7-07 · Skipping the bounds check
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who promises that `i` is in range, and how does the compiler want that promise written?

/// Adds up the first `n` values.
fn sum_first(values: &[u64], n: usize) -> u64 {
    let mut sum = 0;
    for i in 0..n {
        sum += *values.get_unchecked(i);
    }
    sum
}

fn main() {
    assert_eq!(sum_first(&[1, 2, 3, 4], 3), 6); // KEEP
    assert_eq!(sum_first(&[1, 2], 2), 3); // KEEP
}
