//! DRILL 21-01 · Polling without a pin
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0599
//!
//! Run `cargo drill ch21`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what is the receiver type of `poll`?

use std::future::{Future, ready};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

// FIX: `poll` takes `self: Pin<&mut Self>`, so it cannot be called on the
// future itself. `Ready<i32>` is `Unpin`, so `Pin::new(&mut job)` is allowed.
fn main() {
    let mut cx = Context::from_waker(Waker::noop());
    let mut job = ready(5);
    let result = Pin::new(&mut job).poll(&mut cx);
    assert_eq!(result, Poll::Ready(5)); // KEEP
}
