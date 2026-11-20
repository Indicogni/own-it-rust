//! DRILL 18-08 · A file name is not a str
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0308
//!
//! Run `cargo drill ch18`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what type does `file_name` return, and why?

use std::path::Path;

fn main() {
    let path = Path::new("photos/cat.jpg");
    // FIX: a file name is an OsStr. `to_str` returns None when the name is not
    // Unicode, so the conversion is a visible decision.
    let name: &str = path.file_name().unwrap().to_str().unwrap();
    assert_eq!(name, "cat.jpg"); // KEEP
}
