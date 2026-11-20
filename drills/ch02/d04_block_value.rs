//! DRILL 02-04 · A block with its own x
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill ch02`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which `x` does the last line of the block read, and which `x` does
//! the assertion read?

fn main() {
    let x = 5;
    let y = {
        let x = x * 2;
        x + 1
    };
    assert_eq!((x, y), (5, 11)); // KEEP
}
