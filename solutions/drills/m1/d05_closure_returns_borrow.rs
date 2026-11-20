//! DRILL M1-05 · A closure that returns part of its argument
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: error
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which lifetime does the compiler give to the closure's result when nothing tells it?

// FIX: a closure bound with `let` gets no signature from outside. The
// compiler then gives the argument and the result two unrelated lifetimes,
// and the body cannot return one as the other ("lifetime may not live long
// enough"; this error has no code). A function item follows the elision rules
// of `fn`, which tie the result to the argument. (Another repair is to pass
// the closure through a function whose parameter is
// `F: for<'a> Fn(&'a str) -> &'a str`.)
fn main() {
    fn first_word(line: &str) -> &str {
        line.split(' ').next().unwrap_or("")
    }
    assert_eq!(first_word("hello world"), "hello"); // KEEP
}
