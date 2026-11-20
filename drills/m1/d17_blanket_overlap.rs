//! DRILL M1-17 · A blanket impl and one more
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: does `bool` implement `Display`?

use std::fmt::Display;

trait Describe {
    fn describe(&self) -> String;
}

impl<T: Display> Describe for T {
    fn describe(&self) -> String {
        format!("<{self}>")
    }
}

impl Describe for bool {
    fn describe(&self) -> String {
        if *self { "yes" } else { "no" }.to_string()
    }
}

fn main() {
    assert_eq!(7.describe(), "<7>"); // KEEP
    assert_eq!(true.describe(), "yes"); // KEEP
}
