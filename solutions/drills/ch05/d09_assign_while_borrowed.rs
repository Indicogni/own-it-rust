//! DRILL 05-09 · Assigning to a borrowed value
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0506
//!
//! Run `cargo drill ch05`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: while `r` is still used, may `limit` change?

fn main() {
    let mut limit = 10;
    let r = &limit;
    // FIX: read through `r` before the assignment. A shared reference promises
    // that the value it sees does not change while the reference is in use.
    assert_eq!(*r, 10); // KEEP
    limit = 20;
    assert_eq!(limit, 20); // KEEP
}
