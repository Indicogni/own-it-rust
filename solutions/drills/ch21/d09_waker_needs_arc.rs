//! DRILL 21-09 · A Waker from a plain struct
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0277
//!
//! Run `cargo drill ch21`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how does a Waker keep its task alive?

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Wake, Waker};

struct Flag(AtomicBool);

impl Wake for Flag {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::Relaxed);
    }
}

fn main() {
    // FIX: a `Waker` is built from an `Arc<W>`, because wakers are cloned and
    // sent to other threads and all the clones share one task. Keeping a
    // second `Arc` here lets `main` read the flag afterwards.
    let flag = Arc::new(Flag(AtomicBool::new(false)));
    let waker = Waker::from(Arc::clone(&flag));
    waker.wake_by_ref(); // KEEP
    assert!(flag.0.load(Ordering::Relaxed)); // KEEP
}
