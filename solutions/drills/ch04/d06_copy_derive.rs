//! DRILL 04-06 · Copy is opt-in and all-or-nothing
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0204
//!
//! Run `cargo drill ch04`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: can every field of this struct be copied bit for bit?

#[derive(Clone, Debug, PartialEq)]
struct Reading {
    value: u16,
    unit: String,
}

// FIX: a type can be `Copy` only if every field is `Copy`, and `String` is
// not. The KEEP lines only need `Clone`, so drop `Copy` from the derive.
fn main() {
    let a = Reading { value: 7, unit: "mV".into() };
    let b = a.clone(); // KEEP
    assert_eq!(a, b); // KEEP
}
