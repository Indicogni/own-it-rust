//! DRILL 01-06 · One missing character
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch01`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: compare the two lines that print or check something. What does one
//! of them have after its name that the other does not?

fn main() {
    let total = 2 + 3;
    println("total = {}", total);
    assert_eq!(total, 5); // KEEP
}
