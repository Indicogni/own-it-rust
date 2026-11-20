//! DRILL 02-10 · A variable and its block
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch02`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: between which two braces does `label` exist?

fn main() {
    let guests = 12;
    if guests > 10 {
        let label = "many";
    }
    assert_eq!(label, "many"); // KEEP
}
