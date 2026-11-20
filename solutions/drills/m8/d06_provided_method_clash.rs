//! DRILL M8-06 · A provided method meets a user's method
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m8`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: two traits in scope offer `describe` for `Square`. Which one does `square.describe()` mean?

/// Stands for version 1.2.0 of a library crate.
mod shapes {
    pub trait Shape {
        fn area(&self) -> f64; // KEEP

        /// New in 1.2.0. It has a body, so no implementation had to change.
        fn describe(&self) -> String { // KEEP
            format!("a shape of area {}", self.area()) // KEEP
        } // KEEP
    }

    pub struct Square(pub f64);

    impl Shape for Square {
        fn area(&self) -> f64 {
            self.0 * self.0
        }
    }
}

use shapes::{Shape, Square};

/// A user's trait, written for 1.1.0.
trait Describe { // KEEP
    fn describe(&self) -> String; // KEEP
} // KEEP

impl Describe for Square { // KEEP
    fn describe(&self) -> String { // KEEP
        format!("square with side {}", self.0) // KEEP
    } // KEEP
} // KEEP

fn main() {
    let square = Square(2.0); // KEEP
    assert_eq!(square.area(), 4.0); // KEEP
    // Name the trait whose method is meant.
    assert_eq!(Describe::describe(&square), "square with side 2");
}
