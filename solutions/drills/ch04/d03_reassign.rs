//! DRILL 04-03 · Reassigning a moved binding
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ok
//!
//! Run `cargo drill ch04`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: is a moved-from variable dead forever?

fn consume(s: String) -> usize {
    s.len()
}

// Nothing to fix: this program compiles and runs. Moving `label` into
// `consume` leaves the variable without a value, and the assignment gives it
// a new one.
fn main() {
    let mut label = String::from("one");
    let n = consume(label);
    label = String::from("three");
    assert_eq!((n, label.as_str()), (3, "three")); // KEEP
}
