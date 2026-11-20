//! DRILL 04-04 · Moves inside a loop
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0382
//!
//! Run `cargo drill ch04`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many times can one String be given away?

// FIX: the first iteration moved `greeting` into the vector, so the second
// iteration had nothing to move. Three elements need three Strings.
fn main() {
    let greeting = String::from("hi");
    let mut out = Vec::new();
    for _ in 0..3 {
        out.push(greeting.clone());
    }
    assert_eq!(out, ["hi", "hi", "hi"]); // KEEP
}
