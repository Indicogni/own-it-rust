//! DRILL M6-02 · Matching on a status code
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0004
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which values can a C `int` have?

use std::ffi::c_int;

const OK: c_int = 0;
const NOT_FOUND: c_int = 2;
const FULL: c_int = 3;

// FIX: a status code is an integer, and the library may return any integer:
// today's three, or a new one next year. The compiler insists on an arm for
// the rest, and that arm is exactly what makes the binding survive a newer
// library. (A Rust `enum` for the status would move the problem to the place
// where the integer is converted, where an unknown value must also go
// somewhere.)
fn describe(status: c_int) -> &'static str {
    match status {
        OK => "ok",
        NOT_FOUND => "not found",
        FULL => "full",
        _ => "unknown",
    }
}

fn main() {
    assert_eq!(describe(3), "full"); // KEEP
    assert_eq!(describe(42), "unknown"); // KEEP
}
