//! DRILL 18-07 · Where are the bytes?
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch18`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: has BufWriter passed anything to the Vec yet?

use std::io::{BufWriter, Write};

fn main() {
    let mut out = BufWriter::new(Vec::new());
    out.write_all(b"saved").unwrap();
    // FIX: the five bytes were still in BufWriter's own buffer. Flush hands
    // them to the Vec.
    out.flush().unwrap();
    let written = out.get_ref().len();
    assert_eq!(written, 5); // KEEP
}
