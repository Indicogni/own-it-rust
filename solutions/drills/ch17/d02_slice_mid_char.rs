//! DRILL 17-02 · Slicing inside a character
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch17`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: is byte 2 the start of a character in "héllo"?

fn main() {
    let word = "héllo";
    // FIX: 'é' occupies bytes 1 and 2, so byte 2 is inside it. The first two
    // characters end at byte 3.
    let head = &word[..3];
    assert_eq!(head, "hé"); // KEEP
}
