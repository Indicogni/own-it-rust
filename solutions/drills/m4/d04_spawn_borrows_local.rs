//! DRILL M4-04 · A task that borrows from its creator
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0373
//!
//! Run `cargo drill m4`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how long may a spawned task live, compared with the function that spawned it?

use std::future::Future;
use std::task::{Context, Poll, Waker};
use std::thread;

/// Polls until ready. A busy loop is enough for a drill.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

fn spawn<F>(future: F) -> thread::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    thread::spawn(move || block_on(future))
}

fn main() {
    let greeting = String::from("hello");
    // FIX: `move` puts the string inside the future, so the task owns it.
    let job = async move { greeting.len() };
    assert_eq!(spawn(job).join().unwrap(), 5); // KEEP
}
