//! DRILL M1-14 · A pattern that leaves a variant out
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ok
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many values does the type `Infallible` have?

use std::convert::Infallible;

fn parse_name(text: &str) -> Result<String, Infallible> {
    Ok(text.trim().to_string())
}

// NOTHING TO FIX: this compiles and runs. `Infallible` is an enum without
// variants, so no value of it exists and `Err(_)` can never be built. The
// pattern `Ok(name)` therefore covers every possible value, and `let` accepts
// it without `else`. With `Result<String, String>` the same line is E0005.
fn main() {
    let Ok(name) = parse_name("  Ada "); // KEEP
    assert_eq!(name, "Ada"); // KEEP
}
