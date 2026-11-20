//! DRILL 13-13 · Borrowing from a temporary
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch13`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who owns the String that `trim` borrows from, and when is it dropped?

fn read_line() -> String {
    String::from("  42  ")
}

fn main() {
    let text = read_line().trim();
    assert_eq!(text, "42"); // KEEP
}
