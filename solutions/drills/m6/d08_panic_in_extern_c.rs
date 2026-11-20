//! DRILL M6-08 · Catching a panic from a C-ABI function
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: panic
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which frames may a panic travel through?

use std::ffi::c_int;
use std::panic::catch_unwind;

/// A callback with the C ABI, as a C library would call it.
// FIX: a function with the ABI "C" promises its caller that it never unwinds.
// When a panic tries to leave it, Rust keeps the promise by aborting the
// process ("panic in a function that cannot unwind"), and `catch_unwind` in
// the caller never sees anything. The ABI "C-unwind" allows unwinding, which
// is right here because the caller is Rust.
//
// When the caller really is C, do not use "C-unwind" unless the C code was
// built to be unwound through. Catch the panic inside the callback instead and
// return an error value; lesson 05 shows how.
extern "C-unwind" fn checked_half(n: c_int) -> c_int {
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
