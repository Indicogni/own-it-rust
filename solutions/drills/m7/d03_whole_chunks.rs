//! DRILL M7-03 · Whole chunks only
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: panic
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does `chunks_exact` do with elements that do not fill a chunk?

// FIX: `chunks_exact(2)` yields only whole chunks and leaves the fifth element
// out, so the second assertion panicked. Add the elements that follow the last
// whole chunk.
/// Adds up the values two at a time.
fn sum_pairs(values: &[u32]) -> u32 {
    let mut sum = 0;
    let pairs = values.chunks_exact(2); // KEEP
    for pair in pairs {
        sum += pair[0] + pair[1];
    }
    // `chunks_exact` leaves out the elements that do not fill a chunk. They
    // are the tail of the slice after the last whole chunk.
    let whole = values.len() - values.len() % 2;
    sum + values[whole..].iter().sum::<u32>()
}

fn main() {
    assert_eq!(sum_pairs(&[1, 2, 3, 4]), 10); // KEEP
    assert_eq!(sum_pairs(&[1, 2, 3, 4, 5]), 15); // KEEP
}
