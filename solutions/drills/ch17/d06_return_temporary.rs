//! DRILL 17-06 · Returning a borrow of a temporary
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0515
//!
//! Run `cargo drill ch17`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who owns the lowercase String after the function returns?

// FIX: `to_lowercase` builds a new String that this function owns. A `&str`
// into it would point at freed memory after the return, so return the String
// itself. A String compares equal to a `&str` with the same text.
fn normalize(input: &str) -> String {
    input.trim().to_lowercase()
}

fn main() {
    assert_eq!(normalize("  Rust "), "rust"); // KEEP
}
