//! DRILL M2-01 · Reading a Cell that holds a String
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what would `get` have to do with a value that cannot be copied bit by bit?

use std::cell::Cell;

fn main() {
    let name = Cell::new(String::from("ada"));
    let seen = name.get();
    name.set(String::from("grace"));
    assert_eq!(seen, "ada"); // KEEP
    assert_eq!(name.into_inner(), "grace"); // KEEP
}
