//! DRILL M7-04 · Looking at the target of a buffered writer
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0502
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: where are the five bytes after `write_all`, and how long does `writer` borrow `out`?

// FIX: two problems in one line. The five bytes stay in the buffer of the
// `BufWriter` until `flush`. And `BufWriter` has a destructor, so it borrows
// `out` until it is dropped; looking at `out` before that is a second borrow
// (E0502). Flush, drop the writer, then look.
use std::io::{BufWriter, Write};

fn main() {
    let mut out: Vec<u8> = Vec::new();
    let mut writer = BufWriter::new(&mut out); // KEEP
    writer.write_all(b"hello").unwrap(); // KEEP
    writer.flush().unwrap(); // KEEP
    // `BufWriter` has a destructor, so it borrows `out` until it is dropped.
    drop(writer);
    assert_eq!(out.len(), 5); // KEEP
}
