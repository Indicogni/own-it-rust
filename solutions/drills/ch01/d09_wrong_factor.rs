//! DRILL 01-09 · It compiles. Is it right?
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch01`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: work out `minutes(2)` by hand before you answer.

fn minutes(hours: u32) -> u32 {
    hours * 60
}

fn main() {
    assert_eq!(minutes(2), 120); // KEEP
    assert_eq!(minutes(0), 0); // KEEP
}
