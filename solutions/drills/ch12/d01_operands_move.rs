//! DRILL 12-01 · Operators take their operands by value
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0382
//!
//! Run `cargo drill ch12`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does Add::add(self, rhs) do to a non-Copy left operand?

use std::ops::Add;

#[derive(Debug, Clone, PartialEq)]
struct Money(Vec<u32>);

impl Add for Money {
    type Output = Money;
    fn add(mut self, rhs: Money) -> Money {
        self.0.extend(rhs.0);
        self
    }
}

fn main() {
    let wallet = Money(vec![5]);
    // FIX: `Add::add` takes `self` by value, so `wallet + ...` moved the wallet.
    // Add a clone and keep the original. (To avoid the copy, you could also
    // implement `Add<Money> for &Money` and write `&wallet + ...`.)
    let total = wallet.clone() + Money(vec![10]);
    assert_eq!(total, Money(vec![5, 10])); // KEEP
    assert_eq!(wallet.0.len(), 1); // KEEP
}
