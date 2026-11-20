//! DRILL M5-06 · A derive on a function
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m5`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which kinds of item may carry `#[derive(...)]`, and which kind of macro may stand on a function?

#[derive(Debug)]
fn version() -> u32 {
    3 // KEEP
}

fn main() {
    assert_eq!(version(), 3); // KEEP
}
