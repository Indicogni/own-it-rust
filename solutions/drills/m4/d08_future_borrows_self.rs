//! DRILL M4-08 · A future that outlives the handle it came from
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0505
//!
//! Run `cargo drill m4`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: in edition 2024, which lifetimes does a returned `impl Trait` capture unless you say otherwise?

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Waker};

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

struct Token {
    cancelled: Arc<AtomicBool>,
}

impl Token {
    /// A future that reports whether the token was cancelled.
    // FIX: `use<>` says that the returned type captures no lifetime. Without
    // it, edition 2024 assumes that the future may borrow from `&self`.
    fn check(&self) -> impl Future<Output = bool> + use<> {
        let cancelled = Arc::clone(&self.cancelled);
        async move { cancelled.load(Ordering::SeqCst) }
    }
}

fn main() {
    let cancelled = Arc::new(AtomicBool::new(true));
    let token = Token { cancelled };
    let check = token.check();
    drop(token); // KEEP
    assert!(block_on(check)); // KEEP
}
