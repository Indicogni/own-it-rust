//! DRILL 21-02 · Pin::new on an async block
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0277
//!
//! Run `cargo drill ch21`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: may this future move after it starts?

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

// FIX: no async block is `Unpin`, so `Pin::new` rejects it. `pin!` pins the
// future to this stack frame, where it stays until `main` returns, and gives
// a `Pin<&mut _>`. `Box::pin(async { .. })` works as well, at the cost of one
// allocation.
fn main() {
    let mut cx = Context::from_waker(Waker::noop());
    let mut job = pin!(async {
        let text = String::from("borrowed across an await");
        let view = &text;
        std::future::ready(()).await;
        view.len()
    });
    let result = job.as_mut().poll(&mut cx);
    assert_eq!(result, Poll::Ready(24)); // KEEP
}
