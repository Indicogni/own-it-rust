//! DRILL 20-02 · Calling an async fn without awaiting
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0369
//!
//! Run `cargo drill ch20`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does calling an async fn give you?

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

async fn base_price() -> u32 {
    40
}

// FIX: `base_price()` is a future, not a number. `.await` runs it and gives
// the u32 that `+` needs.
async fn total() -> u32 {
    base_price().await + 2
}

fn main() {
    assert_eq!(run(total()), 42); // KEEP
}
