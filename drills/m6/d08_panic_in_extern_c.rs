//! DRILL M6-08 · Catching a panic from a C-ABI function
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which frames may a panic travel through?

use std::ffi::c_int;
use std::panic::catch_unwind;

/// A callback with the C ABI, as a C library would call it.
extern "C" fn checked_half(n: c_int) -> c_int {
    if n % 2 != 0 {
        panic!("{n} is odd");
    }
    n / 2
}

fn main() {
    let result = catch_unwind(|| checked_half(7)); // KEEP
    assert!(result.is_err(), "the panic should be caught here"); // KEEP
    assert_eq!(checked_half(8), 4); // KEEP
}
