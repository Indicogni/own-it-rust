//! DRILL 24-10 · Returning a borrowed C string
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0515
//!
//! Run `cargo drill ch24`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who owns the bytes after make_name returns?

use std::ffi::CString;

// FIX: `owned` is dropped when make_name returns, so a &CStr into it would
// dangle. Return the owner. A CString derefs to CStr, so the caller's
// `.to_bytes()` still works.
fn make_name(label: &str) -> CString {
    CString::new(label).unwrap()
}

fn main() {
    assert_eq!(make_name("repo").to_bytes(), b"repo"); // KEEP
}
