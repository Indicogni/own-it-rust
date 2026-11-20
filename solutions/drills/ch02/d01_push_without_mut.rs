//! DRILL 02-01 · Growing a vector
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch02`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `push` changes the vector. What did `let numbers` promise?

fn main() {
    let mut numbers = Vec::new();
    for n in 1..=2 {
        numbers.push(n);
    }
    assert_eq!(numbers, [1, 2]); // KEEP
}
