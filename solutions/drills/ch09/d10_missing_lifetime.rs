//! DRILL 09-10 · A struct that borrows
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0106
//!
//! Run `cargo drill ch09`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what must the struct say about how long the text lives?

// FIX: a struct that stores a reference names its lifetime, so the compiler
// can check that no Excerpt outlives the text it points into.
struct Excerpt<'a> {
    text: &'a str,
}

fn main() {
    let report = String::from("All sensors nominal. Next check at noon.");
    let first = Excerpt { text: &report[..20] };
    assert_eq!(first.text, "All sensors nominal."); // KEEP
}
