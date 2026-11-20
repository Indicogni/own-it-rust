//! DRILL 20-11 · drop() before an await
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: error
//!
//! Run `cargo drill ch20`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: where does the compiler think the life of `guard` ends?

use std::future::{Future, ready};
use std::pin::pin;
use std::sync::Mutex;
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

/// Stands in for a runtime's `spawn`, which may move the future to another thread.
fn spawn_check<F: Future + Send>(future: F) -> F {
    future
}

// FIX: at run time `drop(guard)` did release the lock before the await. But
// the compiler decides what a future stores from scopes, not from calls to
// `drop`: `guard` was still in scope at the await, so the future's type
// contained a `MutexGuard`, which is not `Send`. A block ends the scope.
async fn bump(counter: &Mutex<u32>) -> u32 {
    let seen = {
        let mut guard = counter.lock().unwrap();
        *guard += 1;
        *guard
    };
    ready(()).await;
    seen
}

fn main() {
    let counter = Mutex::new(41);
    let task = spawn_check(bump(&counter)); // KEEP
    assert_eq!(run(task), 42); // KEEP
}
