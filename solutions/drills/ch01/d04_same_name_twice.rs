//! DRILL 01-04 · Three lets, one name
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch01`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: does a second `let` change the old variable, or make a new one?

fn main() {
    let price = 20;
    let price = price + 5;
    let price = price * 2;
    assert_eq!(price, 50); // KEEP
}
