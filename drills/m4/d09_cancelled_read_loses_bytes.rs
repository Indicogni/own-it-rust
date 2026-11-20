//! DRILL M4-09 · A read that is cancelled halfway
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m4`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: where do the bytes of the first chunk live when the first call is dropped?

use std::collections::VecDeque;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// A source of byte chunks. `None` in the script means "nothing yet".
struct Source {
    script: VecDeque<Option<&'static [u8]>>,
}

impl Source {
    /// Reads up to the next newline.
    async fn read_line(&mut self) -> String {
        let mut line = Vec::new();
        loop {
            let chunk = std::future::poll_fn(|_| match self.script.pop_front() {
                Some(Some(chunk)) => Poll::Ready(chunk),
                Some(None) | None => Poll::Pending,
            })
            .await;
            line.extend_from_slice(chunk);
            if line.ends_with(b"\n") {
                line.pop();
                return String::from_utf8(line).unwrap();
            }
        }
    }
}

fn main() {
    let script = VecDeque::from([Some(&b"he"[..]), None, Some(&b"llo\n"[..])]);
    let mut source = Source { script };
    let mut cx = Context::from_waker(Waker::noop());
    {
        // A timeout fires: the first call is polled once and then dropped.
        let mut first_call = pin!(source.read_line()); // KEEP
        assert!(first_call.as_mut().poll(&mut cx).is_pending()); // KEEP
    }
    let mut second_call = pin!(source.read_line()); // KEEP
    let line = second_call.as_mut().poll(&mut cx); // KEEP
    assert_eq!(line, Poll::Ready(String::from("hello"))); // KEEP
}
