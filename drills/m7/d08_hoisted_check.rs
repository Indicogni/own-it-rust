//! DRILL M7-08 · A check moved in front of the loop
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which length does the sub-slice need, and what happens when it is not there?

/// The dot product over the positions that both slices have.
fn dot(a: &[i32], b: &[i32]) -> i32 {
    let n = a.len();
    let b = &b[..n]; // one check here, none in the loop
    let mut sum = 0;
    for i in 0..n {
        sum += a[i] * b[i];
    }
    sum
}

fn main() {
    assert_eq!(dot(&[1, 2, 3], &[4, 5, 6]), 32); // KEEP
    // The shorter side decides.
    assert_eq!(dot(&[1, 2, 3], &[4, 5]), 14); // KEEP
}
