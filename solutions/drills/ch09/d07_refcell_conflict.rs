//! DRILL 09-07 · RefCell borrows at run time
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch09`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how long does the first guard live?

use std::cell::RefCell;

fn main() {
    let log = RefCell::new(vec![String::from("start")]);
    // FIX: read the length through a temporary guard. It is released at the
    // end of this statement, before `borrow_mut` runs.
    let before = log.borrow().len();
    log.borrow_mut().push(String::from("next"));
    assert_eq!(before, 1); // KEEP
    assert_eq!(log.borrow().len(), 2); // KEEP
}
