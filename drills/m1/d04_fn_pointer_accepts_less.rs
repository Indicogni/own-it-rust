//! DRILL M1-04 · A function that accepts less than is asked
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `apply` will call `f` with a short-lived `&str`. Can `keep` take it?

fn keep(text: &'static str) -> &'static str {
    text
}

fn apply(f: fn(&str) -> &str, text: &str) -> usize {
    f(text).len()
}

fn main() {
    let local = String::from("local");
    assert_eq!(apply(keep, &local), 5); // KEEP
}
