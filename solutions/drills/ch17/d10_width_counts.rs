//! DRILL 17-10 · Width counts what?
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch17`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: does `{:<5}` pad to 5 bytes, 5 scalars, or 5 screen columns?

fn main() {
    let cell = format!("{:<5}|", "é"); // KEEP
    // FIX: `{:<5}` pads to 5 scalars, and `len()` counts bytes ('é' has two).
    // Count scalars to get the 5 of the cell plus the '|'.
    let width = cell.chars().count();
    assert_eq!(width, 6); // KEEP
}
