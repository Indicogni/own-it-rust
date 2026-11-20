//! DRILL M7-03 · Whole chunks only
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does `chunks_exact` do with elements that do not fill a chunk?

/// Adds up the values two at a time.
fn sum_pairs(values: &[u32]) -> u32 {
    let mut sum = 0;
    let pairs = values.chunks_exact(2); // KEEP
    for pair in pairs {
        sum += pair[0] + pair[1];
    }
    sum
}

fn main() {
    assert_eq!(sum_pairs(&[1, 2, 3, 4]), 10); // KEEP
    assert_eq!(sum_pairs(&[1, 2, 3, 4, 5]), 15); // KEEP
}
