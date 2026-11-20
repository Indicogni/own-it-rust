//! DRILL 05-07 · Which input does the result borrow?
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0106
//!
//! Run `cargo drill ch05`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: from the signature alone, which argument must outlive the result?

// FIX: the result can be either input, so both inputs and the result share
// one lifetime: the result is valid only while BOTH arguments are.
fn longer<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

fn main() {
    assert_eq!(longer("north", "east"), "north"); // KEEP
}
