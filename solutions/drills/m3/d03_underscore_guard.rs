//! DRILL M3-03 · Holding a lock with `let _`
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: error
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: when is a value dropped that is bound to `_`?

use std::sync::Mutex;

fn main() {
    let balance = Mutex::new(100);
    // `let _ = ...` binds nothing, so the guard is dropped at the end of that
    // statement and the lock is free again on the next line. The compiler
    // rejects it for lock guards (`let_underscore_lock`, an error by
    // default). A name that starts with an underscore keeps the guard alive
    // until the end of the block and still silences the unused warning.
    let _guard = balance.lock().unwrap();
    assert!(balance.try_lock().is_err(), "the lock must be held here"); // KEEP
}
