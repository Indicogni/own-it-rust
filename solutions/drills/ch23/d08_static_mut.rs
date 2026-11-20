//! DRILL 23-08 · A mutable static
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0133
//!
//! Run `cargo drill ch23`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what could another thread be doing to HITS at the same moment?

use std::sync::atomic::{AtomicU32, Ordering};

// FIX: every use of a `static mut` is unsafe, because two threads could use
// it at the same moment. `unsafe { HITS += 1 }` would compile, but an atomic
// is the better repair: it is safe to share, and no `unsafe` remains.
static HITS: AtomicU32 = AtomicU32::new(0);

fn hit() {
    HITS.fetch_add(1, Ordering::Relaxed);
}

fn hits() -> u32 {
    HITS.load(Ordering::Relaxed)
}

fn main() {
    hit(); // KEEP
    hit(); // KEEP
    assert_eq!(hits(), 2); // KEEP
}
