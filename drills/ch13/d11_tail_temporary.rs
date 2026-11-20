//! DRILL 13-11 · A guard in the last expression of a block
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch13`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: the guard from borrow() is a temporary. In edition 2024, is it dropped before or after `words`?

use std::cell::RefCell;

fn count_words() -> usize {
    let words = RefCell::new(vec!["drop", "scope"]);
    words.borrow().len() // KEEP
}

fn main() {
    assert_eq!(count_words(), 2); // KEEP
}
