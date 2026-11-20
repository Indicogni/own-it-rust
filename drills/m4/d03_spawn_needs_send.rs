//! DRILL M4-03 · A task that holds an Rc across an await
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m4`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which values are stored inside the future while it waits at the `.await`?

use std::future::Future;
use std::rc::Rc;
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

/// Runs the future on another thread, as a multi-threaded runtime may do.
fn spawn<F>(future: F) -> thread::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    thread::spawn(move || block_on(future))
}

async fn checkpoint() {}

fn main() {
    let job = async {
        let name = Rc::new(String::from("task"));
        checkpoint().await; // KEEP
        name.len()
    };
    assert_eq!(spawn(job).join().unwrap(), 4); // KEEP
}
