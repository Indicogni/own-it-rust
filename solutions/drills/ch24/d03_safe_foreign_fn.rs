//! DRILL 24-03 · Safe items in an unsafe extern block
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ok
//!
//! Run `cargo drill ch24`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does `safe fn` inside the block promise, and who promises it?

use std::ffi::c_int;

// FIX: the program compiled and ran, but it was not sound. `safe fn` promised
// that every argument is fine, and C's abs(INT_MIN) is undefined behavior.
// The foreign function is unsafe again, under a different Rust name, and a
// safe Rust function with the old name checks the one bad input.
unsafe extern "C" {
    #[link_name = "abs"]
    fn c_abs(x: c_int) -> c_int;
}

fn abs(x: c_int) -> c_int {
    assert!(x != c_int::MIN, "abs(c_int::MIN) does not fit in a c_int");
    // SAFETY: C's abs is defined for every int except INT_MIN, checked above.
    unsafe { c_abs(x) }
}

fn main() {
    assert_eq!(abs(-5), 5); // KEEP
}
