//! DRILL 01-02 · Text where a number is expected
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch01`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which type does the left side of the first `=` ask for, and which
//! type is the value on the right?

fn main() {
    let seats: u32 = 4;
    let rows: u32 = 3;
    assert_eq!(seats * rows, 12); // KEEP
}
