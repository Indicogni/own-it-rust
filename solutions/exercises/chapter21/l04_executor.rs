//! LESSON 04 — A parker block_on
//! Run: cargo test --test chapter21 l04_
//! Read first: "Lesson 04" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. Can unpark happen before park without losing the signal?
//! 2. Why does using Waker::noop in a blocking executor fail for a pending future?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Explain the wake token, spurious park returns, and the runtime features this tiny executor lacks.
//! COMPILER LAB (optional; examples/ch21_scratch.rs): Try block_on on an immediately ready async block and on a self-waking one-yield future. Do not run an eternally Pending future.

use std::{
    future::Future,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
};
struct WakeThread(thread::Thread);
impl Wake for WakeThread {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}
/// A. Pin future, make a waker for the current thread, then poll in a loop.
/// Ready returns output; Pending parks until a signal (and then rechecks).
/// Contract: the supplied future eventually becomes ready and wakes correctly.
/// This executor has no timers, task spawning, I/O reactor, or cancellation.
fn block_on<F: Future>(future: F) -> F::Output {
    // The future stays in this stack frame until the function returns, so
    // pinning it here needs no allocation.
    let mut future = std::pin::pin!(future);
    let waker = Waker::from(Arc::new(WakeThread(thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return output,
            // If the wake already happened, `unpark` left a token and `park`
            // returns at once. `park` may also return for no reason. Both
            // cases are handled by polling again.
            Poll::Pending => thread::park(),
        }
    }
}
/// B. Return a future that yields Pending exactly once and wakes its caller,
/// then returns value. Use poll_fn with state captured by the closure.
fn yield_once<T>(value: T) -> impl Future<Output = T> {
    let mut yielded = false;
    let mut value = Some(value);
    std::future::poll_fn(move |cx| {
        if yielded {
            Poll::Ready(value.take().expect("polled after completion"))
        } else {
            yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;
    use std::{
        sync::{
            Condvar, Mutex,
            atomic::{AtomicUsize, Ordering},
            mpsc,
        },
        time::{Duration, Instant},
    };
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
    // `within` runs block_on on a helper thread. A block_on that parks and is
    // never unparked then fails after a few seconds instead of hanging the run.
    #[test]
    fn a_ready_future_returns_without_parking() {
        let out = within(5, "block_on(async { 3 })", || block_on(async { 3 }));
        assert_eq!(out, 3, "block_on must return the future's output");
    }
    #[test]
    fn a_b_wake_before_park_is_not_lost() {
        let out = within(
            5,
            "block_on(yield_once(..)) (yield_once must wake before it returns Pending, \
             and the waker that block_on passes must unpark its thread)",
            || block_on(yield_once(String::from("resumed"))),
        );
        assert_eq!(out, "resumed");
    }

    /// What the test thread can see of the future that `block_on` is running.
    #[derive(Default)]
    struct Seen {
        polls: usize,
        waker: Option<Waker>,
        release: bool,
        executor_gone: bool,
    }
    /// Marks the executor thread as finished, also when `block_on` panics.
    struct Gone(Arc<(Mutex<Seen>, Condvar)>);
    impl Drop for Gone {
        fn drop(&mut self) {
            let (lock, changed) = &*self.0;
            lock.lock().unwrap().executor_gone = true;
            changed.notify_all();
        }
    }
    #[test]
    fn a_block_on_sleeps_until_woken_and_polls_after_each_wake() {
        const QUIET: Duration = Duration::from_millis(300);
        const ROUNDS: usize = 20;
        const ROUNDS_LIMIT: Duration = Duration::from_secs(2);

        // The future counts its polls and publishes the waker of each poll.
        // It stays Pending until the test sets `release`.
        let seen = Arc::new((Mutex::new(Seen::default()), Condvar::new()));
        let future = {
            let seen = Arc::clone(&seen);
            std::future::poll_fn(move |cx| {
                let (lock, changed) = &*seen;
                let mut s = lock.lock().unwrap();
                s.polls += 1;
                s.waker = Some(cx.waker().clone());
                changed.notify_all();
                if s.release {
                    Poll::Ready(s.polls)
                } else {
                    Poll::Pending
                }
            })
        };
        // block_on gets its own thread, so the test can watch it from outside
        // and cannot hang if block_on never returns.
        let (out_tx, out_rx) = mpsc::channel();
        let gone = Gone(Arc::clone(&seen));
        thread::spawn(move || {
            let _gone = gone;
            let _ = out_tx.send(block_on(future));
        });

        let (lock, changed) = &*seen;
        // Waits until `target` polls have happened, the executor thread has
        // ended, or `deadline` has passed. Returns the number of polls so far.
        let polls_by = |target: usize, deadline: Instant| -> usize {
            let left = deadline.saturating_duration_since(Instant::now());
            let (s, _) = changed
                .wait_timeout_while(lock.lock().unwrap(), left, |s| {
                    s.polls < target && !s.executor_gone
                })
                .unwrap();
            s.polls
        };
        // Calls the newest waker the future was given, after releasing the lock.
        let wake = || {
            let waker = lock.lock().unwrap().waker.clone();
            if let Some(waker) = waker {
                waker.wake();
            }
        };

        let first = polls_by(1, Instant::now() + Duration::from_secs(5));
        assert!(
            first >= 1,
            "block_on did not poll the future: it panicked (see the message above), \
             returned early, or did nothing for 5 s"
        );

        // 1. Nobody wakes the task for a while. A parked executor does not poll.
        thread::sleep(QUIET);
        let quiet_polls = lock.lock().unwrap().polls;

        // 2. Each wake must be answered by a new poll, and quickly.
        let deadline = Instant::now() + ROUNDS_LIMIT;
        let mut polls = quiet_polls;
        let mut answered = 0;
        while answered < ROUNDS {
            wake();
            let now = polls_by(polls + 1, deadline);
            if now == polls {
                break;
            }
            polls = now;
            answered += 1;
        }

        // 3. Let the future finish, so that a polling executor stops running.
        lock.lock().unwrap().release = true;
        wake();

        assert!(
            quiet_polls <= 2,
            "the future was polled {quiet_polls} times in {QUIET:?} although nobody woke it: \
             after Pending, block_on must park until its waker is called, \
             not spin, yield, or sleep and try again"
        );
        assert!(
            answered == ROUNDS,
            "only {answered} of {ROUNDS} wakes were followed by a new poll within {ROUNDS_LIMIT:?}. \
             Each wake must lead to a new poll at once. Either the waker in the Context that \
             block_on passes to poll does not unpark the thread that runs block_on \
             (Waker::noop() cannot do that), or block_on sleeps for a fixed time instead of parking"
        );
        let output = out_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the future was released and woken, but block_on did not return within 5 s");
        assert_eq!(
            output,
            lock.lock().unwrap().polls,
            "block_on must return the output of the poll that was Ready"
        );
    }
    #[test]
    fn b_yielding_future_does_bounded_work() {
        let mut f = std::pin::pin!(yield_once(9));
        let (c, w) = counted();
        let mut cx = Context::from_waker(&w);
        assert!(
            f.as_mut().poll(&mut cx).is_pending(),
            "the first poll of yield_once must be Pending"
        );
        assert_eq!(
            c.0.load(Ordering::Relaxed),
            1,
            "Pending must arrange a wake"
        );
        assert_eq!(f.as_mut().poll(&mut cx), Poll::Ready(9));
        assert_eq!(c.0.load(Ordering::Relaxed), 1, "Ready needs no wake");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. This thread is unparked twice and then parks twice. Each park gives
        // up after 400 ms if nothing lets it through earlier. Does the first park
        // return at once? Does the second?
        let me = thread::current();
        me.unpark();
        me.unpark();
        let limit = Duration::from_millis(400);
        let mut at_once = [false; 2];
        for returned_at_once in &mut at_once {
            let start = Instant::now();
            thread::park_timeout(limit);
            *returned_at_once = start.elapsed() < limit / 2;
        }
        let p1: (bool, bool) = (true, false);
        assert_eq!(
            p1,
            (at_once[0], at_once[1]),
            "P1: what unpark leaves behind for a later park"
        );

        // P2. A future that returns Pending twice (waking itself each time), driven
        // by a simple loop. How many polls until Ready?
        let mut left = 2;
        let mut f = std::pin::pin!(std::future::poll_fn(|cx| {
            if left == 0 {
                return Poll::Ready(());
            }
            left -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }));
        let mut cx = Context::from_waker(Waker::noop());
        let mut polls = 1;
        while f.as_mut().poll(&mut cx).is_pending() {
            polls += 1;
        }
        let p2: usize = 3;
        assert_eq!(p2, polls, "P2: one poll per wake, plus the first");
    }
}
