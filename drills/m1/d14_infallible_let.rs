//! DRILL M1-14 · A pattern that leaves a variant out
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many values does the type `Infallible` have?

use std::convert::Infallible;

fn parse_name(text: &str) -> Result<String, Infallible> {
    Ok(text.trim().to_string())
}

fn main() {
    let Ok(name) = parse_name("  Ada "); // KEEP
    assert_eq!(name, "Ada"); // KEEP
}
