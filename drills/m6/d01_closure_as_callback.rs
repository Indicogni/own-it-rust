//! DRILL M6-01 · A closure where C wants a function pointer
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: a C function pointer is one address. Where would it keep `step`?

use std::ffi::c_int;

/// The callback type of a C library: `int (*)(int)`.
type Callback = extern "C" fn(c_int) -> c_int;

/// Stands in for a C function that calls the callback twice.
fn apply_twice(callback: Callback, value: c_int) -> c_int {
    callback(callback(value))
}

fn main() {
    let step = 3;
    let result = apply_twice(|n| n + step, 1);
    assert_eq!(result, 7); // KEEP
}
