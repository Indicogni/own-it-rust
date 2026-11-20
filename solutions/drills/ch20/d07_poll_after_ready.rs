//! DRILL 20-07 · Polling a finished future again
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch20`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does a completed future owe you?

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

fn main() {
    let mut cx = Context::from_waker(Waker::noop());
    let mut job = pin!(async { 21 * 2 });
    let first = job.as_mut().poll(&mut cx);
    assert_eq!(first, Poll::Ready(42)); // KEEP
    // FIX: `Ready` hands over the output once and ends the future. Polling a
    // finished async block panics ("`async fn` resumed after completion"),
    // so the second poll is removed: the value is already in `first`.
}
