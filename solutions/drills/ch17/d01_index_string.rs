//! DRILL 17-01 · Indexing a String
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0277
//!
//! Run `cargo drill ch17`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what would `s[0]` return if a character can take up to four bytes?

fn main() {
    let word = String::from("héllo");
    // FIX: a String cannot be indexed with a number, because a byte offset is
    // not a character position. `chars()` decodes the scalars one by one.
    let first = word.chars().next().unwrap();
    assert_eq!(first, 'h'); // KEEP
}
