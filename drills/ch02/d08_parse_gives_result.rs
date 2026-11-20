//! DRILL 02-08 · Text to number
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch02`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: can every text be turned into a number? What must `parse` therefore
//! return?

fn main() {
    let age: u32 = "42".parse();
    assert_eq!(age + 1, 43); // KEEP
}
