//! DRILL 17-07 · to_string without Display
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0599
//!
//! Run `cargo drill ch17`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: where does `to_string` come from?

struct Celsius(f64);

// FIX: `to_string` exists for every type that implements `Display`, so
// implementing `Display` is what adds it.
impl std::fmt::Display for Celsius {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}°C", self.0)
    }
}

fn main() {
    let t = Celsius(21.5);
    assert_eq!(t.to_string(), "21.5°C"); // KEEP
}
