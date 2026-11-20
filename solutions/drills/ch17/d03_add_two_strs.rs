//! DRILL 17-03 · Adding two &str
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0369
//!
//! Run `cargo drill ch17`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which side of `+` must own a buffer to append into?

fn main() {
    let greeting = "hello, ";
    let name = "ferris";
    // FIX: `+` needs an owned String on the left, because it appends into that
    // String's buffer. `format!("{greeting}{name}")` works too.
    let message = greeting.to_owned() + name;
    assert_eq!(message, "hello, ferris"); // KEEP
}
