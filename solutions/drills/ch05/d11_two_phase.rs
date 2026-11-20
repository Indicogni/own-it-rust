//! DRILL 05-11 · Reading a vector inside its own push
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ok
//!
//! Run `cargo drill ch05`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `push` needs `&mut steps`. Is the argument computed before or after
//! that exclusive borrow starts to count?

fn main() {
    let mut steps = vec![0, 1, 2];
    // Nothing to fix: this is a two-phase borrow. The `&mut steps` for `push` is
    // reserved first, the argument `steps.len()` is then read through a shared
    // borrow, and only after that does the exclusive borrow become active.
    steps.push(steps.len());
    steps.push(steps[0] + steps[3]);
    assert_eq!(steps, [0, 1, 2, 3, 3]); // KEEP
}
