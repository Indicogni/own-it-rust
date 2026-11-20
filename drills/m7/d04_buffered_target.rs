//! DRILL M7-04 · Looking at the target of a buffered writer
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: where are the five bytes after `write_all`, and how long does `writer` borrow `out`?

use std::io::{BufWriter, Write};

fn main() {
    let mut out: Vec<u8> = Vec::new();
    let mut writer = BufWriter::new(&mut out); // KEEP
    writer.write_all(b"hello").unwrap(); // KEEP
    assert_eq!(out.len(), 5); // KEEP
    writer.flush().unwrap(); // KEEP
}
