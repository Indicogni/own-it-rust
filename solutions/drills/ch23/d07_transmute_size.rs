//! DRILL 23-07 · transmute between sizes
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0512
//!
//! Run `cargo drill ch23`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many bytes does each side have?

// FIX: `transmute` needs two types of the same size; f32 has 4 bytes and u64
// has 8. The safe `to_bits` gives the 32 bits, and `u64::from` widens them.
fn main() {
    let bits: u64 = u64::from(1.5_f32.to_bits());
    assert_eq!(bits, 1.5_f32.to_bits() as u64); // KEEP
}
