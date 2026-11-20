//! DRILL 22-01 · Define before use
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: error
//!
//! Run `cargo drill ch22`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: where does a macro_rules! name start being visible?

// FIX: a macro_rules! name is visible only in the text AFTER its definition
// (textual scope), unlike a function. Move the definition above `main`.
macro_rules! twice {
    ($x:expr) => {
        $x * 2
    };
}

fn main() {
    let total = twice!(21);
    assert_eq!(total, 42); // KEEP
}
