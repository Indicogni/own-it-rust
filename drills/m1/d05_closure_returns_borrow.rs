//! DRILL M1-05 · A closure that returns part of its argument
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which lifetime does the compiler give to the closure's result when nothing tells it?

fn main() {
    let first_word = |line: &str| line.split(' ').next().unwrap_or("");
    assert_eq!(first_word("hello world"), "hello"); // KEEP
}
