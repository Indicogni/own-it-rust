//! DRILL 02-02 · An if that produces a value
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch02`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which value would `label` get when there are only 3 guests?

fn main() {
    let guests = 12;
    let label = if guests > 10 {
        "many"
    };
    assert_eq!(label, "many"); // KEEP
}
