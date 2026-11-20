//! DRILL 18-04 · lines() consumes the reader
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0382
//!
//! Run `cargo drill ch18`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does `lines` take: `&self`, `&mut self`, or `self`?

use std::io::{BufRead, BufReader};

fn main() {
    let mut reader = BufReader::new(&b"a\nb\n"[..]);
    // FIX: `lines` takes `self`. `&mut BufReader` is a BufRead too, so lend the
    // reader instead of giving it away.
    let count = (&mut reader).lines().count();
    let rest = reader.into_inner();
    assert_eq!(count, 2); // KEEP
    assert!(rest.is_empty()); // KEEP
}
