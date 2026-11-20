//! DRILL M6-01 · A closure where C wants a function pointer
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0308
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

// FIX: a closure that captures `step` is code plus data, and a C function
// pointer has room for the code address only. A named `extern "C"` function
// that needs no captured data fits. (A closure that captures nothing does not
// fit either: it can become a Rust `fn` pointer, but not an `extern "C"` one.)
// When the callback really needs data, the C API must offer a `void *userdata`
// parameter; lesson 05 is about that.
extern "C" fn add_three(n: c_int) -> c_int {
    n + 3
}

fn main() {
    let result = apply_twice(add_three, 1);
    assert_eq!(result, 7); // KEEP
}
