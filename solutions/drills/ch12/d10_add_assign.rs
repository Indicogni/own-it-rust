//! DRILL 12-10 · Compound assignment is its own trait
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0368
//!
//! Run `cargo drill ch12`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: does implementing Add give you +=?

use std::ops::{Add, AddAssign};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Score(u32);

impl Add for Score {
    type Output = Score;
    fn add(self, rhs: Score) -> Score {
        Score(self.0 + rhs.0)
    }
}

// FIX: `+=` is its own trait. Implementing `Add` does not provide it.
impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Score) {
        self.0 += rhs.0;
    }
}

fn main() {
    let mut total = Score(0);
    for points in [Score(3), Score(4)] {
        total += points; // KEEP
    }
    assert_eq!(total, Score(7)); // KEEP
}
