//! DRILL 22-03 · Evaluating an argument twice
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch22`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many times does `$x` appear in the expansion?

// FIX: `$x * $x` pasted the call twice (1 * 2, two calls). Bind the argument
// once, then use the local twice.
macro_rules! square {
    ($x:expr) => {{
        let value = $x;
        value * value
    }};
}

fn main() {
    let mut calls = 0;
    let mut next = || {
        calls += 1;
        calls
    };
    let one = square!(next());
    assert_eq!(one, 1); // KEEP
    assert_eq!(calls, 1); // KEEP
}
