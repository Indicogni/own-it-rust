//! DRILL 17-09 · parse without a target type
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0284
//!
//! Run `cargo drill ch17`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how does `parse` know which FromStr impl to call?

fn main() {
    // FIX: `parse` can produce any type that implements `FromStr`, and `+ 2`
    // does not decide which one. Name the type with a turbofish.
    let total = "40".parse::<i32>().unwrap() + 2;
    assert_eq!(total, 42); // KEEP
}
