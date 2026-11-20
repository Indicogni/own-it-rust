//! DRILL 04-07 · Shared ownership is shared, not mutable
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0596
//!
//! Run `cargo drill ch04`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does Rc promise every other owner?

use std::rc::Rc;

// FIX: an `Rc` gives shared, read-only access. `Rc::make_mut` first gives
// `config` a private copy of the text (because `other` also owns the old
// one) and then lets `config` change that copy. `other` keeps the old text.
fn main() {
    let mut config = Rc::new(String::from("zone=a"));
    let other = Rc::clone(&config);
    Rc::make_mut(&mut config).push_str(";debug");
    assert_eq!(*other, "zone=a"); // KEEP
}
