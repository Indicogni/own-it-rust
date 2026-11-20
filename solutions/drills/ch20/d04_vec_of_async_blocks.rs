//! DRILL 20-04 · Two async blocks in one Vec
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0308
//!
//! Run `cargo drill ch20`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many future types are there here?

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// Polls a future once with a waker that does nothing. Enough for futures
/// that never return Pending.
fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("this tiny executor cannot wait"),
    }
}

// FIX: every async block has its own anonymous type, even when two blocks
// look the same, and a Vec holds one element type. One async fn called twice
// gives two futures of the same type, and needs no Box.
// (`Vec<Pin<Box<dyn Future<Output = i32>>>>` with `Box::pin(async { 1 })` is
// the fix when the futures really come from different code.)
async fn job(value: i32) -> i32 {
    value
}

fn main() {
    let jobs = vec![job(1), job(2)];
    let total: i32 = jobs.into_iter().map(run).sum();
    assert_eq!(total, 3); // KEEP
}
