//! DRILL M1-16 · A foreign trait for a foreign type
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which crate defines `Display`, and which crate defines `Vec`?

use std::fmt;

struct Celsius(f64);

impl fmt::Display for Vec<Celsius> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.iter().map(|c| c.0.to_string()).collect();
        f.write_str(&parts.join(", "))
    }
}

fn main() {
    let week = vec![Celsius(20.5), Celsius(19.0)];
    assert_eq!(week.to_string(), "20.5, 19"); // KEEP
}
