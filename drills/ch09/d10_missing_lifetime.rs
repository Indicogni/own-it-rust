//! DRILL 09-10 · A struct that borrows
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch09`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what must the struct say about how long the text lives?

struct Excerpt {
    text: &str,
}

fn main() {
    let report = String::from("All sensors nominal. Next check at noon.");
    let first = Excerpt { text: &report[..20] };
    assert_eq!(first.text, "All sensors nominal."); // KEEP
}
