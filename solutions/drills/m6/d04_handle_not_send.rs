//! DRILL M6-04 · A handle on another thread
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0277
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: the compiler sees a raw pointer. What does it not know about the thing behind it?

use std::thread;

/// An owned handle to a value that a library allocated.
struct Handle {
    raw: *mut u32,
}

// FIX: a raw pointer says nothing about ownership or threads, so the compiler
// assumes the worst and makes `Handle` neither `Send` nor `Sync`. The author
// of the wrapper knows more and must say so, with a reason.
// SAFETY: a `Handle` is the only owner of its allocation (like a `Box<u32>`),
// and the allocation is not tied to the thread that made it. Moving the
// handle moves the only access path.
unsafe impl Send for Handle {}

impl Handle {
    fn new(value: u32) -> Handle {
        Handle {
            raw: Box::into_raw(Box::new(value)),
        }
    }

    fn get(&self) -> u32 {
        // SAFETY: `raw` came from Box::into_raw and is freed only in Drop.
        unsafe { *self.raw }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: `raw` came from Box::into_raw, and Drop runs once.
        unsafe { drop(Box::from_raw(self.raw)) }
    }
}

fn main() {
    let handle = Handle::new(7);
    let seen = thread::spawn(move || handle.get()).join().unwrap(); // KEEP
    assert_eq!(seen, 7); // KEEP
}
