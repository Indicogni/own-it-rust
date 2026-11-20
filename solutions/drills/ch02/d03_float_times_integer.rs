//! DRILL 02-03 · Two kinds of number
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch02`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what are the types of `price` and of `count`? Does Rust convert
//! between them for you?

fn main() {
    let price = 2.5;
    let count = 4.0;
    let total = price * count;
    assert_eq!(total, 10.0); // KEEP
}
