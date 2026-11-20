//! DRILL 17-04 · String + &str consumes the left side
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0382
//!
//! Run `cargo drill ch17`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what is the signature of `Add for String`?

fn main() {
    let base = String::from("log");
    // FIX: `base + ".txt"` moves `base` and reuses its buffer. `format!` only
    // borrows `base`, so it can still be read on the last line.
    let file = format!("{base}.txt");
    assert_eq!(file, "log.txt"); // KEEP
    assert_eq!(base, "log"); // KEEP
}
