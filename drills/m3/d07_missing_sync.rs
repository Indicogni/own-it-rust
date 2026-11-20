//! DRILL M3-07 · A home-made lock shared by two threads
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill m3`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which trait lets two threads hold `&Lock<u64>`, and who can promise it here?

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

struct Lock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

impl<T> Lock<T> {
    fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self.locked.swap(true, Ordering::Acquire) {
            std::hint::spin_loop();
        }
        // SAFETY: `locked` is true and this call set it, so no other call is
        // between its `swap` and its `store` below.
        let result = f(unsafe { &mut *self.value.get() });
        self.locked.store(false, Ordering::Release);
        result
    }
}

fn main() {
    let total = Lock {
        locked: AtomicBool::new(false),
        value: UnsafeCell::new(0_u64),
    };
    thread::scope(|s| {
        s.spawn(|| total.with(|n| *n += 1)); // KEEP
        s.spawn(|| total.with(|n| *n += 1)); // KEEP
    });
    assert_eq!(total.with(|n| *n), 2); // KEEP
}
