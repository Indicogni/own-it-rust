//! DRILL 01-05 · Where does the statement end?
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch01`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: where does the compiler notice the problem, and where is the
//! mistake really?

fn main() {
    let width = 6;
    let height = 7;
    assert_eq!(width * height, 42); // KEEP
}
