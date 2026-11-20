//! DRILL M7-08 · A check moved in front of the loop
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: panic
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which length does the sub-slice need, and what happens when it is not there?

// FIX: the sub-slice `&b[..n]` moved the bounds check in front of the loop.
// But with `n = a.len()` it demands that `b` is at least as long as `a`, and
// it panicked for the shorter `b`. `zip` stops at the end of the shorter slice
// and needs no index at all.
/// The dot product over the positions that both slices have.
fn dot(a: &[i32], b: &[i32]) -> i32 {
    // `zip` stops at the end of the shorter slice: no index, no check.
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn main() {
    assert_eq!(dot(&[1, 2, 3], &[4, 5, 6]), 32); // KEEP
    // The shorter side decides.
    assert_eq!(dot(&[1, 2, 3], &[4, 5]), 14); // KEEP
}
