//! DRILL M2-01 · Reading a Cell that holds a String
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0599
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what would `get` have to do with a value that cannot be copied bit by bit?

use std::cell::Cell;

// FIX: `Cell::get` exists only for `T: Copy`. Handing out a copy of a `String`
// would need `clone`, and a `&String` into the cell must never exist. A value
// that is not `Copy` leaves a cell by being swapped out: `replace` stores the
// new name and returns the old one in a single step.
fn main() {
    let name = Cell::new(String::from("ada"));
    let seen = name.replace(String::from("grace"));
    assert_eq!(seen, "ada"); // KEEP
    assert_eq!(name.into_inner(), "grace"); // KEEP
}
