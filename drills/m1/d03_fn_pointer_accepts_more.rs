//! DRILL M1-03 · A function that accepts more than is asked
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `count` accepts every `&str`. Is that enough for a caller that passes only `&'static str`?

fn count(text: &str) -> usize {
    text.len()
}

fn main() {
    // A callback that will only ever be given string literals.
    let on_literal: fn(&'static str) -> usize = count; // KEEP
    assert_eq!(on_literal("abc"), 3); // KEEP
}
