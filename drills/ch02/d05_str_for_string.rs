//! DRILL 02-05 · A literal for a String parameter
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch02`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what is the type of the literal "hello", and what does `shout` ask
//! for?

fn shout(text: String) -> String {
    text.to_uppercase()
}

fn main() {
    let loud = shout("hello");
    assert_eq!(loud, "HELLO"); // KEEP
}
