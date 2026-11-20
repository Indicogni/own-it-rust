//! DRILL 05-04 · Two mutable borrows into one vector
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0499
//!
//! Run `cargo drill ch05`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: can the compiler tell that index 0 and index 1 differ?

fn main() {
    let mut pair = vec![String::from("left"), String::from("right")];
    // FIX: indexing borrows the whole vector, so two `&mut pair[i]` cannot be
    // alive together. `swap` exchanges two elements inside one borrow. When you
    // need both references at once, use
    // `let [a, b] = pair.get_disjoint_mut([0, 1]).unwrap();` or `split_at_mut(1)`.
    pair.swap(0, 1);
    assert_eq!(pair, ["right", "left"]); // KEEP
}
