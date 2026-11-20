//! DRILL M2-05 · A Send wrapper around a raw pointer
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0277
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which value does the closure capture: the wrapper, or something smaller?

use std::thread;

struct SendPtr(*mut u32);

// SAFETY: `main` joins the thread before it uses the value again, so only one
// thread at a time touches the `u32`.
unsafe impl Send for SendPtr {}

// FIX: since edition 2021 a closure captures only the places it uses. The
// body used `ptr.0`, so the closure captured the field, a bare `*mut u32`,
// and the `Send` impl of the wrapper never came into play. `let ptr = ptr;`
// uses the whole wrapper, so the whole wrapper is captured. (For this program
// `thread::scope` with a plain `&mut total` needs no `unsafe` at all, and
// would be the better design.)
fn main() {
    let mut total = 0u32;
    let ptr = SendPtr(&raw mut total);
    thread::spawn(move || {
        let ptr = ptr;
        // SAFETY: `total` lives until the end of `main`, which joins this
        // thread first and does not use `total` before the join.
        unsafe { *ptr.0 += 5 };
    })
    .join()
    .unwrap();
    assert_eq!(total, 5); // KEEP
}
