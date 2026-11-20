//! DRILL M1-04 · A function that accepts less than is asked
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0308
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `apply` will call `f` with a short-lived `&str`. Can `keep` take it?

// FIX: `apply` needs a function that works for EVERY lifetime
// (`for<'a> fn(&'a str) -> &'a str`). `keep` promised to work only for
// 'static text, which is less. Nothing in `keep` needs 'static, so let it
// accept any lifetime.
fn keep(text: &str) -> &str {
    text
}

fn apply(f: fn(&str) -> &str, text: &str) -> usize {
    f(text).len()
}

fn main() {
    let local = String::from("local");
    assert_eq!(apply(keep, &local), 5); // KEEP
}
