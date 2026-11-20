//! DRILL M7-07 · Skipping the bounds check
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0133
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who promises that `i` is in range, and how does the compiler want that promise written?

// FIX: `get_unchecked` is an unsafe method: the caller promises that the index
// is in range, and the call must stand in an `unsafe` block (E0133). No such
// promise is needed here. Take the sub-slice once, which checks the whole
// range, and let an iterator walk it.
/// Adds up the first `n` values.
fn sum_first(values: &[u64], n: usize) -> u64 {
    // One check for the whole range, then an iterator that needs none. No
    // `unsafe` is needed, and a wrong `n` still panics.
    values[..n].iter().sum()
}

fn main() {
    assert_eq!(sum_first(&[1, 2, 3, 4], 3), 6); // KEEP
    assert_eq!(sum_first(&[1, 2], 2), 3); // KEEP
}
