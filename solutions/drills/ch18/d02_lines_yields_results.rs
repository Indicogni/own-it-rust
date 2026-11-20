//! DRILL 18-02 · What lines() yields
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0308
//!
//! Run `cargo drill ch18`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what can go wrong while reading one line, and how is that reported?

use std::io::{BufRead, Cursor};

fn main() {
    let input = Cursor::new("first\nsecond\n");
    // FIX: each item is an io::Result<String>, because reading or decoding a
    // line can fail. The first unwrap handles "no line", the second "error".
    let first: String = input.lines().next().unwrap().unwrap();
    assert_eq!(first, "first"); // KEEP
}
