//! DRILL 24-02 · Calling a foreign function
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0133
//!
//! Run `cargo drill ch24`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: can the compiler check what abs does with its argument?

use std::ffi::c_int;

unsafe extern "C" {
    fn abs(x: c_int) -> c_int;
}

fn main() {
    // FIX: a foreign function is unsafe to call unless it is declared `safe`.
    // SAFETY: abs is defined for every c_int except c_int::MIN, and -5 is not
    // c_int::MIN.
    let v = unsafe { abs(-5) };
    assert_eq!(v, 5); // KEEP
}
