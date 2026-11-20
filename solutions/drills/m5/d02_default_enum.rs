//! DRILL M5-02 · Default for an enum
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m5`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: a struct's default is the default of each field. What would an enum's be?

#[derive(Debug, Default, PartialEq)]
enum Mode {
    Fast,
    // The derive cannot guess which variant is the default: mark one.
    #[default]
    Safe,
}

fn main() {
    let mode = Mode::default(); // KEEP
    assert_eq!(mode, Mode::Safe); // KEEP
    assert_ne!(mode, Mode::Fast); // KEEP
}
