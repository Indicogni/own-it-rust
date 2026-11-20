//! DRILL M5-03 · A helper attribute without its derive
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m5`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who makes `#[default]` a known attribute inside this enum?

// `#[default]` is a helper attribute of the derive `Default`. It is known only
// inside an item that has this derive.
#[derive(Debug, Default, PartialEq)]
enum Level {
    Low,
    #[default] // KEEP
    Middle,
    High,
}

fn main() {
    let level = Level::Middle; // KEEP
    assert_eq!(level, Level::Middle); // KEEP
    assert_ne!(Level::Low, Level::High); // KEEP
}
