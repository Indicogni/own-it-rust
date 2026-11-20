//! DRILL 14-02 · Reading a variable a closure is mutating
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0502
//!
//! Run `cargo drill ch14`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how long does the closure's borrow of `count` last?

fn main() {
    let mut count = 0;
    let mut bump = || count += 1;
    bump();
    bump();
    // FIX: read `count` after the closure's last call. The closure's mutable
    // borrow ends at its last use, so the read no longer overlaps it.
    println!("count so far: {count}");
    assert_eq!(count, 2); // KEEP
}
