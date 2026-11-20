//! DRILL 24-01 · Declaring foreign functions
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: error
//!
//! Run `cargo drill ch24`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who vouches that these declarations match the real C library?

use std::ffi::c_int;

// FIX: in edition 2024 an extern block must be written `unsafe extern`. The
// word `unsafe` is your signature under a promise: these declarations match
// the real C library. The compiler cannot check that for you.
unsafe extern "C" {
    fn abs(x: c_int) -> c_int;
}

fn main() {
    // SAFETY: abs is defined for every c_int except c_int::MIN.
    let v = unsafe { abs(-5) };
    assert_eq!(v, 5); // KEEP
}
