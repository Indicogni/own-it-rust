//! LESSON 03 — A completion cell
//! Run: cargo test --test chapter21 l03_
//! Read first: "Lesson 03" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. Must poll replace an old waker when the executor supplies a new one?
//! 2. Should a completion function invoke wake while holding its state mutex?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Trace a completion-before-poll and poll-before-completion interleaving. Explain why one locked state prevents lost wakeups.
//! COMPILER LAB (optional; examples/ch21_scratch.rs): Replace a registered counted waker with another, complete, and inspect which counter changes. Discuss dropping a receiver before completion.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};
struct State<T> {
    value: Option<T>,
    waiter: Option<Waker>,
}
struct Receive<T>(Arc<Mutex<State<T>>>);
struct Complete<T>(Arc<Mutex<State<T>>>);
/// A. Construct a single-producer/single-consumer cell. Complete is consumed
/// when fulfilled. Dropping an unfulfilled producer leaves a pending receiver;
/// this minimal model deliberately has no disconnect result.
fn completion<T>() -> (Complete<T>, Receive<T>) {
    let state = Arc::new(Mutex::new(State {
        value: None,
        waiter: None,
    }));
    (Complete(Arc::clone(&state)), Receive(state))
}
impl<T> Future for Receive<T> {
    type Output = T;
    /// B. Under one lock: take a present value, otherwise register the current
    /// waker and return Pending. Never block waiting for a value.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        // One guard covers both the check and the registration, so `finish`
        // cannot store a value between the two.
        let mut state = self.0.lock().unwrap();
        if let Some(value) = state.value.take() {
            return Poll::Ready(value);
        }
        // Always store the waker of this poll: the executor may pass a
        // different one each time, and only the newest is valid.
        state.waiter = Some(cx.waker().clone());
        Poll::Pending
    }
}
impl<T> Complete<T> {
    /// C. Store the value under the lock, take the registered waker, unlock,
    /// then wake if present. One producer means no overwrite is possible.
    fn finish(self, value: T) {
        let waiter = {
            let mut state = self.0.lock().unwrap();
            state.value = Some(value);
            state.waiter.take()
        };
        // The guard is gone here. A task woken on another thread can lock
        // the state at once.
        if let Some(waker) = waiter {
            waker.wake();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Wake;
    struct Counter(AtomicUsize);
    impl std::task::Wake for Counter {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn counted() -> (Arc<Counter>, std::task::Waker) {
        let c = Arc::new(Counter(AtomicUsize::new(0)));
        let w = std::task::Waker::from(Arc::clone(&c));
        (c, w)
    }
    /// A waker that records whether `lock` was free at the moment it was woken.
    struct Probe<T> {
        lock: Arc<Mutex<T>>,
        seen: Mutex<Vec<bool>>,
    }
    impl<T: Send> Wake for Probe<T> {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            let free = self.lock.try_lock().is_ok();
            self.seen.lock().unwrap().push(free);
        }
    }
    // The tests run inside `within`: a poll or finish that locks the state a
    // second time on the same thread, or that waits for a value, never returns.
    // `within` turns that into a failure after a few seconds.
    const STUCK: &str = "a poll or finish on the completion cell";
    #[test]
    fn a_b_c_complete_before_poll_is_ready() {
        within(10, STUCK, || {
            let (tx, mut rx) = completion();
            tx.finish(String::from("ok"));
            let mut cx = Context::from_waker(Waker::noop());
            assert_eq!(
                Pin::new(&mut rx).poll(&mut cx),
                Poll::Ready(String::from("ok")),
                "a value stored before the first poll must be returned by that poll"
            );
        });
    }
    #[test]
    fn a_b_c_latest_registered_waker_is_used() {
        within(10, STUCK, || {
            let (tx, mut rx) = completion();
            let (a, wa) = counted();
            let (b, wb) = counted();
            assert!(
                Pin::new(&mut rx)
                    .poll(&mut Context::from_waker(&wa))
                    .is_pending()
            );
            assert!(
                Pin::new(&mut rx)
                    .poll(&mut Context::from_waker(&wb))
                    .is_pending()
            );
            tx.finish(17);
            assert_eq!(
                a.0.load(Ordering::Relaxed),
                0,
                "the first waker was replaced by a later poll, so it must not be woken"
            );
            assert_eq!(
                b.0.load(Ordering::Relaxed),
                1,
                "finish must wake the waker of the most recent poll, exactly once"
            );
            assert_eq!(
                Pin::new(&mut rx).poll(&mut Context::from_waker(&wb)),
                Poll::Ready(17)
            );
        });
    }
    #[test]
    fn a_b_dropped_producer_leaves_the_receiver_pending() {
        within(10, STUCK, || {
            let (tx, mut rx) = completion::<u8>();
            drop(tx);
            let (c, w) = counted();
            assert!(
                Pin::new(&mut rx)
                    .poll(&mut Context::from_waker(&w))
                    .is_pending()
            );
            assert_eq!(
                c.0.load(Ordering::Relaxed),
                0,
                "registering a waker must not wake it"
            );
        });
    }
    #[test]
    fn c_wake_happens_after_unlocking() {
        within(10, STUCK, || {
            let (tx, mut rx) = completion::<u32>();
            let probe = Arc::new(Probe {
                lock: Arc::clone(&rx.0),
                seen: Mutex::new(Vec::new()),
            });
            let waker = Waker::from(Arc::clone(&probe));
            assert!(
                Pin::new(&mut rx)
                    .poll(&mut Context::from_waker(&waker))
                    .is_pending()
            );
            tx.finish(5);
            assert_eq!(
                *probe.seen.lock().unwrap(),
                [true],
                "wake exactly once, after the state lock is released \
                 (each entry says whether the lock was free during a wake)"
            );
            let mut cx = Context::from_waker(Waker::noop());
            assert_eq!(Pin::new(&mut rx).poll(&mut cx), Poll::Ready(5));
        });
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. The waker checks whether `state` is free. It is woken once while the
        // lock is held, and once after the guard is dropped.
        let state = Arc::new(Mutex::new(0u8));
        let probe = Arc::new(Probe {
            lock: Arc::clone(&state),
            seen: Mutex::new(Vec::new()),
        });
        let waker = Waker::from(Arc::clone(&probe));
        {
            let _guard = state.lock().unwrap();
            waker.wake_by_ref();
        }
        waker.wake_by_ref();
        let p1: Vec<bool> = vec![false, true];
        assert_eq!(p1, *probe.seen.lock().unwrap(), "P1: waking under a lock");

        // P2. A slot holds a clone of waker `wa`. Strong count of `a` before and
        // after the slot's waker is replaced by a clone of `wb`.
        let (a, wa) = counted();
        let (_b, wb) = counted();
        let mut slot = Some(wa.clone());
        let before = Arc::strong_count(&a);
        slot.replace(wb.clone());
        let after = Arc::strong_count(&a);
        let p2: (usize, usize) = (3, 2);
        assert_eq!(
            p2,
            (before, after),
            "P2: replacing a stored waker drops the old one"
        );
        drop(slot);
    }
}
