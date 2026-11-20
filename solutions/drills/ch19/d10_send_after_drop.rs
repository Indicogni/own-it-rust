//! DRILL 19-10 · Sending after the receiver is gone
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch19`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who is left to receive the second message?

use std::sync::mpsc;
use std::thread;

// FIX: the worker receives one value and returns, which drops the receiver.
// The second `send` then fails and hands the value back in its error. A
// failed send is how a producer learns that the consumer has stopped, so it
// is handled here instead of unwrapped.
fn main() {
    let (tx, rx) = mpsc::channel::<u32>();
    let worker = thread::spawn(move || rx.recv().unwrap() * 2);
    tx.send(1).unwrap();
    let doubled = worker.join().unwrap(); // KEEP
    if let Err(mpsc::SendError(unsent)) = tx.send(2) {
        assert_eq!(unsent, 2);
    }
    assert_eq!(doubled, 2); // KEEP
}
