//! DRILL M4-01 · Polling a field through a pin
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m4`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what is the type of `self.inner` here, and what does `poll` need instead?

use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll, Waker};

/// A future that carries a name for log lines.
struct Named<F> {
    name: &'static str,
    inner: F,
}

impl<F: Future> Future for Named<F> {
    type Output = (&'static str, F::Output);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let name = self.name;
        self.inner.poll(cx).map(|output| (name, output))
    }
}

fn main() {
    let inner = async { 7 }; // KEEP
    let mut job = pin!(Named { name: "job", inner });
    let mut cx = Context::from_waker(Waker::noop());
    let output = job.as_mut().poll(&mut cx); // KEEP
    assert_eq!(output, Poll::Ready(("job", 7))); // KEEP
}
