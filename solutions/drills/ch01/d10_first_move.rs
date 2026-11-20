//! DRILL 01-10 · A first look at ownership
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch01`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: a String is not copied by `=`. Which of the two names holds the
//! text after the second line of `main`?

fn main() {
    let name = String::from("Ada");
    let badge = name.clone();
    assert_eq!(name, "Ada"); // KEEP
    assert_eq!(badge, "Ada"); // KEEP
}
