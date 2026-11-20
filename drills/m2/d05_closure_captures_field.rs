//! DRILL M2-05 · A Send wrapper around a raw pointer
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which value does the closure capture: the wrapper, or something smaller?

use std::thread;

struct SendPtr(*mut u32);

// SAFETY: `main` joins the thread before it uses the value again, so only one
// thread at a time touches the `u32`.
unsafe impl Send for SendPtr {}

fn main() {
    let mut total = 0u32;
    let ptr = SendPtr(&raw mut total);
    thread::spawn(move || {
        // SAFETY: `total` lives until the end of `main`, which joins this
        // thread first and does not use `total` before the join.
        unsafe { *ptr.0 += 5 };
    })
    .join()
    .unwrap();
    assert_eq!(total, 5); // KEEP
}
