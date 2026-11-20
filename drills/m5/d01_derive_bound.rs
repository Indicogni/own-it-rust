//! DRILL M5-01 · The bound a derive adds
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m5`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which bound does `#[derive(Clone)]` put on `T`, and does this struct need it?

use std::rc::Rc;

/// Not `Clone`, on purpose.
struct Session {
    user: &'static str,
}

#[derive(Clone)]
struct Shared<T>(Rc<T>);

fn main() {
    let first = Shared(Rc::new(Session { user: "ann" })); // KEEP
    let second = first.clone(); // KEEP
    assert!(Rc::ptr_eq(&first.0, &second.0)); // KEEP
    assert_eq!(second.0.user, "ann"); // KEEP
}
