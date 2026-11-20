//! DRILL 11-05 · impl Trait is one concrete type
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch11`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many types can hide behind one impl Iterator?

// The two branches build different iterator types, so the function returns a
// trait object: one allocation, and each `next` call goes through the vtable.
fn numbers(descending: bool) -> Box<dyn Iterator<Item = u32>> {
    if descending {
        Box::new((1..=3).rev())
    } else {
        Box::new(1..=3)
    }
}

fn main() {
    assert_eq!(numbers(true).collect::<Vec<_>>(), [3, 2, 1]); // KEEP
    assert_eq!(numbers(false).collect::<Vec<_>>(), [1, 2, 3]); // KEEP
}
