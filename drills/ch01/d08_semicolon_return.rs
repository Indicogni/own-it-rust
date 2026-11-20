//! DRILL 01-08 · A function that returns nothing
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch01`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which value does a function body give back when its last line ends
//! with a semicolon?

fn double(n: i32) -> i32 {
    n * 2;
}

fn main() {
    assert_eq!(double(21), 42); // KEEP
}
