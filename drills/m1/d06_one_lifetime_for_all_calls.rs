//! DRILL M1-06 · A callback bound to one named lifetime
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who chooses 'a: the caller of `widest`, or `widest` itself?

fn widest<'a, F>(text: &'a str, clean: F) -> usize
where
    F: Fn(&'a str) -> &'a str,
{
    let upper = text.to_uppercase();
    clean(&upper).len().max(clean(text).len())
}

fn main() {
    assert_eq!(widest("  padded  ", |line| line.trim()), 6); // KEEP
}
