//! DRILL 01-07 · Calling a function
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch01`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many values does `area` ask for, and how many does the call
//! give it?

fn area(width: u32, height: u32) -> u32 {
    width * height
}

fn main() {
    let room = area(4);
    assert_eq!(room, 20); // KEEP
}
