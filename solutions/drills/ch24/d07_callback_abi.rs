//! DRILL 24-07 · Callback ABIs
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0308
//!
//! Run `cargo drill ch24`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which calling convention does `double` use?

use std::ffi::c_int;

type Callback = extern "C" fn(c_int) -> c_int;

fn apply(cb: Callback, x: c_int) -> c_int {
    cb(x)
}

// FIX: the calling convention is part of a function pointer's type. `apply`
// wants an `extern "C"` function, so `double` must be declared as one.
extern "C" fn double(x: c_int) -> c_int {
    x * 2
}

fn main() {
    assert_eq!(apply(double, 4), 8); // KEEP
}
