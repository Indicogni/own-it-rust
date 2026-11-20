//! DRILL M1-16 · A foreign trait for a foreign type
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0117
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which crate defines `Display`, and which crate defines `Vec`?

use std::fmt;

struct Celsius(f64);

// FIX: `Display` and `Vec` both belong to the standard library. `Celsius` is
// local, but it sits inside `Vec<..>`, and only the outermost type counts for
// the orphan rule. (`Box<Celsius>` and `&Celsius` would count: `Box` and `&`
// are "fundamental".) A local newtype around the vector is a local type, so
// the impl is allowed.
struct Week(Vec<Celsius>);

impl fmt::Display for Week {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.0.iter().map(|c| c.0.to_string()).collect();
        f.write_str(&parts.join(", "))
    }
}

fn main() {
    let week = Week(vec![Celsius(20.5), Celsius(19.0)]);
    assert_eq!(week.to_string(), "20.5, 19"); // KEEP
}
