//! DRILL 16-05 · Two mutable indices
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch16`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how would the compiler know that index 0 and index 2 are different places?

fn main() {
    let mut slots = vec![1, 2, 3];
    // One call lends both elements and checks at run time that the indices
    // differ. For a plain swap, `slots.swap(0, 2)` is shorter still.
    let [a, b] = slots
        .get_disjoint_mut([0, 2])
        .expect("0 and 2 are different and in range");
    std::mem::swap(a, b);
    assert_eq!(slots, [3, 2, 1]); // KEEP
}
