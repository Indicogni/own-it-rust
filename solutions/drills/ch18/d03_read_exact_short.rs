//! DRILL 18-03 · read_exact on a short input
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch18`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many bytes are available, and how many does read_exact insist on?

use std::io::Read;

fn main() {
    let mut input: &[u8] = b"abc";
    let mut header = [0u8; 4];
    // FIX: read_exact must fill its whole buffer, and only three bytes exist.
    // Ask for exactly the three bytes that are there.
    input.read_exact(&mut header[..3]).unwrap();
    assert_eq!(&header[..3], b"abc"); // KEEP
}
