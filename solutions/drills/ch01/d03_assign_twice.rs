//! DRILL 01-03 · Changing a variable
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch01`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what did the first line promise about `score`?

fn main() {
    let mut score = 10;
    score += 5;
    assert_eq!(score, 15); // KEEP
}
