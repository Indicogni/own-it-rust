//! DRILL 01-01 · A name the compiler does not know
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch01`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: is every name on the line with `total` declared somewhere above it,
//! letter for letter?

fn main() {
    let apples = 3;
    let pears = 4;
    let total = apples + pears;
    assert_eq!(total, 7); // KEEP
}
