//! DRILL 02-06 · Walking a vector by index
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch02`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which values does `i` take, one by one? Which is the last index of
//! a vector with three elements?

fn main() {
    let mut scores = vec![70, 85];
    scores.push(90);
    let mut total = 0;
    for i in 0..=scores.len() {
        total += scores[i];
    }
    assert_eq!(total, 245); // KEEP
}
