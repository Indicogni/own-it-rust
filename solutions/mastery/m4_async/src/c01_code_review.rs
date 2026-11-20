//! CODE REVIEW 01 — A teammate's runtime helpers
//! Do this after lesson 07.
//! Run: cargo test --manifest-path mastery/m4_async/Cargo.toml c01_
//! Then: cargo clippy --manifest-path mastery/m4_async/Cargo.toml --all-targets
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and some behavior tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every reference that is leaked, every waker that is stale or kept
//!    too long, every wake made while a lock is held, every cleanup that a
//!    cancelled future skips, and every needless allocation. For each
//!    `unsafe` block, try to write the SAFETY comment that is missing; where
//!    you cannot, you have found a bug.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. Keep the public names and signatures; the tests depend on them.
//! 4. Run clippy and resolve its warnings for this file.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// Counts the wakes of [`counting_waker`].
#[derive(Debug, Default)]
pub struct Tally {
    wakes: AtomicUsize,
}

impl Tally {
    pub fn wakes(&self) -> usize {
        self.wakes.load(Ordering::SeqCst)
    }
}

static TALLY_VTABLE: RawWakerVTable =
    RawWakerVTable::new(tally_clone, tally_wake, tally_wake_by_ref, tally_drop);

/// A. A waker that adds 1 to `tally` on every wake, written without the
/// `Wake` trait so that it never allocates. The waker owns the reference
/// that is passed in.
pub fn counting_waker(tally: Arc<Tally>) -> Waker {
    let data = Arc::into_raw(tally).cast::<()>();
    // SAFETY: `data` comes from `Arc::<Tally>::into_raw` and carries one
    // reference, which the waker now owns. The functions in `TALLY_VTABLE`
    // keep the RawWaker contract: `clone` adds a reference, `wake` and
    // `drop` release one, `wake_by_ref` releases none. `Tally` is
    // `Send + Sync`, so the functions are safe to call from any thread.
    unsafe { Waker::from_raw(RawWaker::new(data, &TALLY_VTABLE)) }
}

/// # Safety
/// `data` must come from `Arc::<Tally>::into_raw`, and the calling waker's
/// reference must be alive.
unsafe fn tally_clone(data: *const ()) -> RawWaker {
    // SAFETY: `data` is a live `Arc<Tally>` pointer by this function's
    // contract; the new reference belongs to the returned RawWaker.
    unsafe { Arc::increment_strong_count(data.cast::<Tally>()) };
    RawWaker::new(data, &TALLY_VTABLE)
}

/// # Safety
/// As for `tally_clone`; the caller gives up its reference.
unsafe fn tally_wake(data: *const ()) {
    // SAFETY: the caller hands over its reference; the rebuilt `Arc`
    // releases exactly that reference at the end of this function.
    // (The reviewed code only counted here, so every `wake` leaked one.)
    let tally = unsafe { Arc::from_raw(data.cast::<Tally>()) };
    tally.wakes.fetch_add(1, Ordering::SeqCst);
}

/// # Safety
/// As for `tally_clone`.
unsafe fn tally_wake_by_ref(data: *const ()) {
    // SAFETY: the calling waker keeps its reference for the whole call, so
    // the `Tally` is alive; nothing is taken from the count.
    let tally = unsafe { &*data.cast::<Tally>() };
    tally.wakes.fetch_add(1, Ordering::SeqCst);
}

/// # Safety
/// As for `tally_wake`.
unsafe fn tally_drop(data: *const ()) {
    // SAFETY: the caller hands over its reference, which is released here.
    drop(unsafe { Arc::from_raw(data.cast::<Tally>()) });
}

#[derive(Default)]
struct LatchState {
    open: bool,
    waiter: Option<Waker>,
}

/// B. A one-way switch that one task waits for: `open` turns it on, and the
/// future of `wait` completes once it is on.
#[derive(Clone, Default)]
pub struct Latch {
    state: Arc<Mutex<LatchState>>,
}

impl Latch {
    pub fn new() -> Latch {
        Latch::default()
    }

    /// Turns the switch on and wakes the task that waits for it.
    pub fn open(&self) {
        let waiter = {
            let mut state = self.state.lock().unwrap();
            state.open = true;
            // Taken out: the waker is used once and then released, and it
            // is woken only after the lock is gone.
            state.waiter.take()
        };
        if let Some(waiter) = waiter {
            waiter.wake();
        }
    }

    /// A future that completes when the switch is on.
    pub fn wait(&self) -> LatchWait {
        LatchWait {
            latch: self.clone(),
        }
    }
}

/// The future of [`Latch::wait`].
pub struct LatchWait {
    latch: Latch,
}

impl Future for LatchWait {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.latch.state.lock().unwrap();
        if state.open {
            return Poll::Ready(());
        }
        // Always the waker of the newest poll: the task may have moved to
        // another executor, or sit inside a select with its own waker.
        let replaced = match &state.waiter {
            Some(waiter) if waiter.will_wake(cx.waker()) => None,
            _ => state.waiter.replace(cx.waker().clone()),
        };
        drop(state);
        drop(replaced);
        Poll::Pending
    }
}

/// Takes 1 from the gauge when it is dropped: at the normal end, when the
/// future is cancelled, and when it panics.
struct InFlight<'a>(&'a AtomicUsize);

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// C. Runs `future` and keeps `gauge` equal to the number of futures that
/// are running inside `tracked` at this moment (a metric for a dashboard).
pub async fn tracked<F: Future>(gauge: &AtomicUsize, future: F) -> F::Output {
    gauge.fetch_add(1, Ordering::SeqCst);
    let _in_flight = InFlight(gauge);
    // `future` is a local of this async fn, so awaiting it in place needs no
    // box: the async fn's own future is its pinned storage.
    future.await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::alloc::count;
    use crate::support::{Anchored, WakeCounter, block_on, poll_to_end, within};
    use std::pin::pin;
    use std::sync::Weak;
    use std::sync::mpsc::{self, Sender};
    use std::task::Wake;
    use std::thread;

    fn noop_cx() -> Context<'static> {
        Context::from_waker(Waker::noop())
    }

    #[test]
    fn a_counting_waker_counts() {
        let tally = Arc::new(Tally::default());
        let waker = counting_waker(Arc::clone(&tally));
        waker.wake_by_ref();
        let second = waker.clone();
        second.wake();
        assert_eq!(tally.wakes(), 2, "one wake_by_ref and one wake");
    }

    #[test]
    fn a_counting_waker_does_not_allocate() {
        let tally = Arc::new(Tally::default());
        let ((), allocations) = count(|| {
            let waker = counting_waker(Arc::clone(&tally));
            let second = waker.clone();
            second.wake_by_ref();
            drop(second);
            drop(waker);
        });
        assert_eq!(allocations, 0, "the waker must reuse the Arc's allocation");
    }

    #[test]
    fn a_every_wake_releases_its_reference() {
        let tally = Arc::new(Tally::default());
        let weak: Weak<Tally> = Arc::downgrade(&tally);
        let waker = counting_waker(Arc::clone(&tally));
        for _ in 0..3 {
            let clone = waker.clone();
            clone.wake();
        }
        assert_eq!(
            Arc::strong_count(&tally),
            2,
            "three clones were woken by value, so their three references must be gone: \
             `wake` consumes the waker and must release what the waker owned"
        );
        drop(waker);
        drop(tally);
        assert!(
            weak.upgrade().is_none(),
            "the tally is still alive although nothing can reach it any more: a leak"
        );
    }

    #[test]
    fn b_a_latch_that_is_open_is_ready() {
        let latch = Latch::new();
        let mut wait = pin!(latch.wait());
        assert!(wait.as_mut().poll(&mut noop_cx()).is_pending());
        latch.open();
        assert!(wait.as_mut().poll(&mut noop_cx()).is_ready());
        assert!(
            pin!(latch.wait()).poll(&mut noop_cx()).is_ready(),
            "a wait that starts after `open` is ready at once"
        );
    }

    #[test]
    fn b_open_wakes_a_task_on_another_thread() {
        within(
            10,
            "block_on(latch.wait()) with a helper that opens",
            || {
                let latch = Latch::new();
                let opener = latch.clone();
                let helper = thread::spawn(move || opener.open());
                block_on(latch.wait());
                helper.join().unwrap();
            },
        );
    }

    #[test]
    fn b_open_wakes_the_newest_waker() {
        let latch = Latch::new();
        let mut wait = pin!(latch.wait());
        let (old, old_waker) = WakeCounter::new();
        let (new, new_waker) = WakeCounter::new();
        assert!(
            wait.as_mut()
                .poll(&mut Context::from_waker(&old_waker))
                .is_pending()
        );
        assert!(
            wait.as_mut()
                .poll(&mut Context::from_waker(&new_waker))
                .is_pending()
        );
        latch.open();
        assert_eq!(
            (old.count(), new.count()),
            (0, 1),
            "(wakes of the first waker, wakes of the second): a future must be woken \
             through the waker of its most recent poll"
        );
    }

    #[test]
    fn b_the_waker_is_released_when_it_has_been_used() {
        let latch = Latch::new();
        let mut wait = pin!(latch.wait());
        let (counter, waker) = WakeCounter::new();
        assert!(
            wait.as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_pending()
        );
        latch.open();
        latch.open();
        assert_eq!(counter.count(), 1, "the second `open` has nobody to wake");
        assert_eq!(
            Arc::strong_count(&counter),
            2,
            "after the wake the latch must not hold the waker any longer"
        );
    }

    /// A waker that looks at the latch when it is woken. That needs the lock.
    struct Probe(Latch, Mutex<Sender<bool>>);
    impl Wake for Probe {
        fn wake(self: Arc<Self>) {
            let open = pin!(self.0.wait()).poll(&mut noop_cx()).is_ready();
            let _ = self.1.lock().unwrap().send(open);
        }
    }

    #[test]
    fn b_open_wakes_after_releasing_the_lock() {
        within(
            10,
            "`open` with a waker that polls the latch at once (wake only after \
             the lock is released)",
            || {
                let latch = Latch::new();
                let (tx, rx) = mpsc::channel();
                let waker = Waker::from(Arc::new(Probe(latch.clone(), Mutex::new(tx))));
                let mut wait = pin!(latch.wait());
                assert!(
                    wait.as_mut()
                        .poll(&mut Context::from_waker(&waker))
                        .is_pending()
                );
                latch.open();
                assert_eq!(
                    rx.try_recv(),
                    Ok(true),
                    "the woken code must see the latch open"
                );
            },
        );
    }

    #[test]
    fn c_the_gauge_counts_running_futures() {
        let gauge = AtomicUsize::new(0);
        let (inner, _) = Anchored::new(2, "done");
        let mut outer = pin!(tracked(&gauge, async {
            let seen_inside = gauge.load(Ordering::SeqCst);
            (inner.await, seen_inside)
        }));
        assert_eq!(
            gauge.load(Ordering::SeqCst),
            0,
            "nothing runs before the first poll"
        );
        let output = block_on(outer.as_mut());
        assert_eq!(output, ("done", 1), "(output, gauge while the future ran)");
        assert_eq!(gauge.load(Ordering::SeqCst), 0, "the future has finished");
    }

    #[test]
    fn c_a_cancelled_future_leaves_the_gauge_clean() {
        let gauge = AtomicUsize::new(0);
        {
            let mut running = pin!(tracked(&gauge, std::future::pending::<()>()));
            assert!(running.as_mut().poll(&mut noop_cx()).is_pending());
            assert_eq!(gauge.load(Ordering::SeqCst), 1);
            // The future is dropped here, as a select or a timeout would do.
        }
        assert_eq!(
            gauge.load(Ordering::SeqCst),
            0,
            "a future that is dropped while it waits never reaches the code after its \
             `.await`: cleanup that must always happen belongs in a `Drop` guard"
        );
    }

    #[test]
    fn c_a_panicking_future_leaves_the_gauge_clean() {
        let gauge = AtomicUsize::new(0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            block_on(tracked(&gauge, async {
                panic!("this panic is part of the test");
            }))
        }));
        assert!(result.is_err());
        assert_eq!(
            gauge.load(Ordering::SeqCst),
            0,
            "a panic also skips the code after the `.await`"
        );
    }

    #[test]
    fn c_tracking_does_not_allocate() {
        let gauge = AtomicUsize::new(0);
        let (inner, report) = Anchored::new(2, 7u8);
        let mut cx = noop_cx();
        let (output, allocations) = count(|| {
            let mut running = pin!(tracked(&gauge, inner));
            poll_to_end(running.as_mut(), &mut cx, 100).0
        });
        assert_eq!(
            (output, allocations),
            (7, 0),
            "(output, allocations): the future can be awaited where it is"
        );
        assert!(!report.moved());
    }
}
