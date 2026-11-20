//! DRILL M6-06 · Giving a handle away
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what happens to `self` at the end of `into_raw`?

use std::sync::atomic::{AtomicUsize, Ordering};

/// How many handles have been released.
static RELEASED: AtomicUsize = AtomicUsize::new(0);

/// Owns a resource of a library, known by its id.
struct Handle {
    id: usize,
}

impl Drop for Handle {
    fn drop(&mut self) {
        RELEASED.fetch_add(1, Ordering::SeqCst);
    }
}

impl Handle {
    /// Gives up ownership: the caller will release the resource later.
    fn into_raw(self) -> usize {
        self.id
    }
}

fn main() {
    let handle = Handle { id: 41 };
    let raw = handle.into_raw();
    assert_eq!(raw, 41); // KEEP
    // into_raw must not release the resource: the caller still uses it.
    assert_eq!(RELEASED.load(Ordering::SeqCst), 0); // KEEP
}
