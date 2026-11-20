//! DRILL M1-06 · A callback bound to one named lifetime
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0597
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who chooses 'a: the caller of `widest`, or `widest` itself?

// FIX: 'a is a parameter of `widest`, so the caller chooses it, and it is
// longer than the whole body of `widest`. `clean` was bound to that single
// lifetime, so it could not take a reference to the local `upper`. The bound
// `Fn(&str) -> &str` means `for<'x> Fn(&'x str) -> &'x str`: the callback
// works for every lifetime, also one that is shorter than the function body.
fn widest<F>(text: &str, clean: F) -> usize
where
    F: Fn(&str) -> &str,
{
    let upper = text.to_uppercase();
    clean(&upper).len().max(clean(text).len())
}

fn main() {
    assert_eq!(widest("  padded  ", |line| line.trim()), 6); // KEEP
}
