//! DRILL M4-10 · Pending, and nobody to call back
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: panic
//!
//! Run `cargo drill m4`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: after `Pending`, who knows that this task wants to be polled again?

use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

#[derive(Default)]
struct Door {
    open: bool,
    waiter: Option<Waker>,
}

struct Enter(Arc<Mutex<Door>>);

impl Future for Enter {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut door = self.0.lock().unwrap();
        if door.open {
            return Poll::Ready(());
        }
        // FIX: `Pending` is a promise that somebody will wake this task.
        // Leave the waker where `open` can find it.
        door.waiter = Some(cx.waker().clone());
        Poll::Pending
    }
}

fn open(door: &Mutex<Door>) {
    let waiter = {
        let mut door = door.lock().unwrap();
        door.open = true;
        door.waiter.take()
    };
    if let Some(waiter) = waiter {
        waiter.wake();
    }
}

struct CountWakes(AtomicUsize);

impl Wake for CountWakes {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn main() {
    let door = Arc::new(Mutex::new(Door::default()));
    let wakes = Arc::new(CountWakes(AtomicUsize::new(0)));
    let waker = Waker::from(Arc::clone(&wakes));
    let mut cx = Context::from_waker(&waker);
    let mut enter = pin!(Enter(Arc::clone(&door)));
    assert!(enter.as_mut().poll(&mut cx).is_pending()); // KEEP
    open(&door); // KEEP
    // An executor polls a task again only after a wake.
    assert_eq!(wakes.0.load(Ordering::SeqCst), 1, "never woken"); // KEEP
    assert!(enter.as_mut().poll(&mut cx).is_ready()); // KEEP
}
