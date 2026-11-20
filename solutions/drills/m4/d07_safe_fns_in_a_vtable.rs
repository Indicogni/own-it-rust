//! DRILL M4-07 · Safe functions in a waker vtable
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ok
//!
//! Run `cargo drill m4`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `RawWakerVTable::new` takes `unsafe fn` pointers. May a safe `fn` stand in for one?

use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{RawWaker, RawWakerVTable, Waker};

static WAKES: AtomicUsize = AtomicUsize::new(0);
static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, drop_waker);

fn clone(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

fn wake(_data: *const ()) {
    WAKES.fetch_add(1, Ordering::SeqCst);
}

fn drop_waker(_data: *const ()) {}

fn main() {
    // SAFETY: the four functions never use the data pointer, own nothing,
    // and only touch a static atomic, so every part of the RawWaker contract
    // holds for any pointer and from any thread.
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let second = waker.clone();
    waker.wake_by_ref();
    second.wake();
    assert_eq!(WAKES.load(Ordering::SeqCst), 2); // KEEP
    assert!(waker.will_wake(&waker.clone())); // KEEP
}
