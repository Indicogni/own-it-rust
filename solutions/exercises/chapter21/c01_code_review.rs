//! CODE REVIEW 01 — A teammate's pull request
//! Run: cargo test --test chapter21 c01_
//! Then: cargo clippy --test chapter21
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and some behavior tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every lost or misdirected wakeup, every wake made while a
//!    lock is held, and every allocation made per poll.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. Keep the types and signatures; the tests depend on them.
//! 4. Run clippy and resolve its warning for this file. Read what it suggests,
//!    but check the suggestion against your review: the tidy loop that clippy
//!    proposes still makes the allocation that the tests reject.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

struct Shared {
    value: Option<String>,
    waiter: Option<Waker>,
}

/// A. Resolves to the published message, moving it out of the shared state.
struct Message(Arc<Mutex<Shared>>);

impl Future for Message {
    type Output = String;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<String> {
        let mut shared = self.0.lock().unwrap();
        // FIX: move the message out instead of cloning it.
        if let Some(value) = shared.value.take() {
            return Poll::Ready(value);
        }
        // FIX: store the waker of every poll. The executor may pass a new one,
        // and then the first one no longer reaches this task.
        shared.waiter = Some(cx.waker().clone());
        Poll::Pending
    }
}

/// B. Store the message and wake the waiting task, if any. Called once.
fn publish(shared: &Arc<Mutex<Shared>>, message: String) {
    // FIX: take the waker out under the lock and wake after the guard is
    // dropped, so that the woken task can lock the state at once.
    let waiter = {
        let mut guard = shared.lock().unwrap();
        guard.value = Some(message);
        guard.waiter.take()
    };
    if let Some(waker) = waiter {
        waker.wake();
    }
}

/// C. Sum `items`, adding at most `budget` (> 0) items per poll so that other
/// tasks get a turn. The total fits in u64.
struct ChunkedSum {
    items: Vec<u64>,
    next: usize,
    total: u64,
    budget: usize,
}

impl Future for ChunkedSum {
    type Output = u64;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u64> {
        // FIX: sum the chunk in place; no copy of the remaining items.
        // The chunk is summed into a local first because reading `self.items`
        // and writing `self.total` in one loop would borrow `self` twice
        // through the Pin.
        let end = self.next.saturating_add(self.budget).min(self.items.len());
        let chunk: u64 = self.items[self.next..end].iter().sum();
        self.total += chunk;
        self.next = end;
        if self.next == self.items.len() {
            Poll::Ready(self.total)
        } else {
            // FIX: Pending must arrange a wake. This future is ready to
            // continue at once, so it wakes itself.
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, within};
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        task::Wake,
    };

    struct Counter(AtomicUsize);
    impl Wake for Counter {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn counted() -> (Arc<Counter>, Waker) {
        let c = Arc::new(Counter(AtomicUsize::new(0)));
        let w = Waker::from(Arc::clone(&c));
        (c, w)
    }
    fn shared() -> Arc<Mutex<Shared>> {
        Arc::new(Mutex::new(Shared {
            value: None,
            waiter: None,
        }))
    }

    // The A and B tests run inside `within`: a fix that locks the shared state
    // a second time on the same thread never returns. `within` turns that into
    // a failure instead of a test run that hangs.
    const STUCK: &str = "a poll of Message or a call of publish";
    #[test]
    fn a_published_message_is_ready() {
        within(10, STUCK, || {
            let state = shared();
            publish(&state, "hello".into());
            let mut msg = Message(Arc::clone(&state));
            let out = Pin::new(&mut msg).poll(&mut Context::from_waker(Waker::noop()));
            assert_eq!(out, Poll::Ready("hello".to_string()));
        });
    }

    #[test]
    fn a_latest_waker_is_woken() {
        within(10, STUCK, || {
            let state = shared();
            let mut msg = Message(Arc::clone(&state));
            let (first, w1) = counted();
            let (second, w2) = counted();
            assert!(
                Pin::new(&mut msg)
                    .poll(&mut Context::from_waker(&w1))
                    .is_pending()
            );
            // The executor moved the task and now polls it with a different waker.
            assert!(
                Pin::new(&mut msg)
                    .poll(&mut Context::from_waker(&w2))
                    .is_pending()
            );
            publish(&state, "news".into());
            assert_eq!(
                (
                    first.0.load(Ordering::Relaxed),
                    second.0.load(Ordering::Relaxed)
                ),
                (0, 1),
                "(wakes of the first waker, wakes of the second): \
                 the task now listens on the second waker"
            );
        });
    }

    #[test]
    fn a_message_is_moved_not_cloned() {
        within(10, STUCK, || {
            let state = shared();
            let message = String::from("a large payload");
            let p = message.as_ptr();
            publish(&state, message);
            let mut msg = Message(Arc::clone(&state));
            match Pin::new(&mut msg).poll(&mut Context::from_waker(Waker::noop())) {
                Poll::Ready(s) => assert_eq!(
                    s.as_ptr(),
                    p,
                    "the output must be the published String itself (same heap buffer): \
                     take it out of the shared state instead of cloning"
                ),
                Poll::Pending => panic!("message was published"),
            }
            assert!(
                state.lock().unwrap().value.is_none(),
                "after Ready the shared state must not keep a second copy of the message"
            );
        });
    }

    #[test]
    fn b_wake_happens_after_unlocking() {
        struct Probe(Arc<Mutex<Shared>>, Mutex<Vec<bool>>);
        impl Wake for Probe {
            fn wake(self: Arc<Self>) {
                self.wake_by_ref();
            }
            fn wake_by_ref(self: &Arc<Self>) {
                let free = self.0.try_lock().is_ok();
                self.1.lock().unwrap().push(free);
            }
        }
        within(10, STUCK, || {
            let state = shared();
            let probe = Arc::new(Probe(Arc::clone(&state), Mutex::new(Vec::new())));
            let waker = Waker::from(Arc::clone(&probe));
            let mut msg = Message(Arc::clone(&state));
            assert!(
                Pin::new(&mut msg)
                    .poll(&mut Context::from_waker(&waker))
                    .is_pending()
            );
            publish(&state, "x".into());
            assert_eq!(
                *probe.1.lock().unwrap(),
                [true],
                "each entry says whether the lock was free during a wake: \
                 a woken task on another thread would immediately block on this lock"
            );
        });
    }

    #[test]
    fn c_sum_is_correct() {
        let mut f = ChunkedSum {
            items: (1..=10).collect(),
            next: 0,
            total: 0,
            budget: 3,
        };
        let mut cx = Context::from_waker(Waker::noop());
        let total = loop {
            if let Poll::Ready(total) = Pin::new(&mut f).poll(&mut cx) {
                break total;
            }
        };
        assert_eq!(total, 55, "the sum of 1..=10");
    }

    #[test]
    fn c_pending_requests_a_repoll() {
        let mut f = ChunkedSum {
            items: vec![1, 2, 3],
            next: 0,
            total: 0,
            budget: 2,
        };
        let (c, w) = counted();
        assert!(
            Pin::new(&mut f)
                .poll(&mut Context::from_waker(&w))
                .is_pending()
        );
        assert_eq!(
            c.0.load(Ordering::Relaxed),
            1,
            "Pending with no wake: a real executor would never poll this task again"
        );
    }

    #[test]
    fn c_polls_do_not_allocate() {
        let mut f = ChunkedSum {
            items: (0..1000).collect(),
            next: 0,
            total: 0,
            budget: 10,
        };
        let mut cx = Context::from_waker(Waker::noop());
        let (_, allocations) = count(|| {
            for _ in 0..50 {
                let _ = Pin::new(&mut f).poll(&mut cx);
            }
        });
        assert_eq!(allocations, 0, "50 polls cost {allocations} allocations");
    }
}
