//! DRILL 05-02 · Returning a reference to a local
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0515
//!
//! Run `cargo drill ch05`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who owns `upper` after the function returns?

// FIX: the uppercase text is new data that only this function owns, so hand
// the owner to the caller instead of a reference to a local.
fn shout(text: &str) -> String {
    text.to_uppercase()
}

fn main() {
    assert_eq!(shout("hi"), "HI"); // KEEP
}
