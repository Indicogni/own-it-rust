//! DRILL 04-08 · Removing past the end
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: panic
//!
//! Run `cargo drill ch04`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: is this a compile-time question or a run-time one?

// FIX: the vector has one element, so its only valid index is 0. `remove(1)`
// compiled, because an index is checked when the program runs, and panicked.
fn main() {
    let mut queue = vec![String::from("job")];
    let taken = queue.remove(0);
    assert_eq!(taken, "job"); // KEEP
    assert!(queue.is_empty()); // KEEP
}
