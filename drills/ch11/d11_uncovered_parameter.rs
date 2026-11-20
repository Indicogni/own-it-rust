//! DRILL 11-11 · A foreign trait, a local type, and a type parameter
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch11`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: Add is foreign and Celsius is local. Which types could T stand for?

use std::ops::Add;

#[derive(Debug, PartialEq, Clone, Copy)]
struct Celsius(f64);

impl<T: Into<f64>> Add<Celsius> for T {
    type Output = Celsius;
    fn add(self, rhs: Celsius) -> Celsius {
        Celsius(self.into() + rhs.0)
    }
}

fn main() {
    assert_eq!(1.5 + Celsius(20.0), Celsius(21.5)); // KEEP
}
