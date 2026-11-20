//! DRILL 12-07 · &str + &str
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0369
//!
//! Run `cargo drill ch12`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who would own the result?

fn main() {
    let first = "air";
    let second = "port";
    // FIX: `+` needs an owned String on the left, because the result has to
    // live in a buffer that somebody owns.
    let word = first.to_owned() + second;
    assert_eq!(word, "airport"); // KEEP
}
