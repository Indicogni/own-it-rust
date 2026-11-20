//! LESSON 03 — A timer heap and sleep
//! Read first: CONCEPTS.md, "Lesson 03". Start after: lesson 02.
//! Run: cargo test --manifest-path mastery/m4_async/Cargo.toml l03_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. `BinaryHeap` gives you the largest element first. A timer driver needs the
//!    earliest deadline first. What do you store to get that?
//! 2. A `Sleep` future is dropped long before its deadline. What happens to
//!    its entry in your data structure, and what would happen if nothing did?
//!
//! IMPLEMENT the tasks; keep the tests. From this lesson on you design the
//! fields yourself: the public names and signatures are fixed, everything
//! inside the types is yours. Replace each `_design: ()` placeholder.
//! EXPLAIN: State the lock rule of this lesson in one sentence, then give one
//! concrete program that deadlocks when the driver breaks it. Say why `Sleep`
//! reads the clock in `poll` although the driver already compared the times.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};

/// Locks a mutex and ignores poisoning (see lesson 02).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Names one timer in a [`TimerHeap`]. Ids are never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimerId(u64);

/// A. The data structure of a timer driver: a set of `(deadline, waker)`
/// entries that gives up its entries in deadline order. It is plain data:
/// it has no lock, no thread and no clock. Callers pass the time in.
///
/// Contract:
/// - A timer is *live* from `insert` until it is cancelled or taken as due.
/// - `len` counts live timers; `next_deadline` is the earliest live deadline.
/// - A cancelled timer never appears in `take_due`.
/// - Timers with equal deadlines come out in the order they were inserted.
pub struct TimerHeap {
    _design: (),
}

impl TimerHeap {
    /// An empty heap.
    pub fn new() -> TimerHeap {
        todo!("A: an empty heap")
    }

    /// Adds a timer and returns its id.
    pub fn insert(&mut self, deadline: Instant, waker: Waker) -> TimerId {
        todo!("A: store the entry")
    }

    /// The stored waker of a live timer, so that the caller can compare or
    /// replace it. `None` if the timer is not live.
    pub fn waker_mut(&mut self, id: TimerId) -> Option<&mut Waker> {
        todo!("A: find the waker")
    }

    /// Cancels a live timer and hands its waker back, so that the caller
    /// decides where the waker is dropped. `None` if the timer is not live.
    pub fn cancel(&mut self, id: TimerId) -> Option<Waker> {
        todo!("A: remove the entry, or make sure it is skipped later")
    }

    /// The earliest deadline of a live timer.
    pub fn next_deadline(&self) -> Option<Instant> {
        todo!("A: the earliest live deadline")
    }

    /// Removes every live timer whose deadline is `now` or earlier and
    /// returns their wakers: earliest deadline first, insertion order among
    /// equals. The caller wakes them.
    pub fn take_due(&mut self, now: Instant) -> Vec<Waker> {
        todo!("A: pop everything that is due")
    }

    /// The number of live timers.
    pub fn len(&self) -> usize {
        todo!("A: count live timers only")
    }

    /// `true` if there is no live timer.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for TimerHeap {
    fn default() -> Self {
        TimerHeap::new()
    }
}

/// B. Owns the timer thread. The thread sleeps until the earliest deadline
/// (or until a timer with an earlier deadline is added), takes what is due,
/// and wakes it.
///
/// The lock rule of this lesson: while the timer lock is held, call no code
/// that you did not write. That means no `wake` and no drop of a `Waker`.
/// Move wakers out of the critical section first.
///
/// Dropping the driver stops and joins the thread and releases every waker
/// that is still registered, without waking it. A `Sleep` that is still
/// waiting then stays pending for ever, so drop the tasks (the executor)
/// before the driver.
pub struct TimerDriver {
    _design: (),
}

impl TimerDriver {
    /// Starts the timer thread.
    pub fn start() -> TimerDriver {
        todo!("B: shared state and one thread")
    }

    /// A handle that creates `Sleep` futures. It can be cloned and sent to
    /// other threads and into tasks.
    pub fn handle(&self) -> Timers {
        todo!("B: a handle to the shared state")
    }
}

// B. Implement `Drop for TimerDriver` here (see the contract above).

/// Creates [`Sleep`] futures on one [`TimerDriver`].
#[derive(Clone)]
pub struct Timers {
    _design: (),
}

impl Timers {
    /// C. A future that completes `duration` from now.
    pub fn sleep(&self, duration: Duration) -> Sleep {
        self.sleep_until(Instant::now() + duration)
    }

    /// C. A future that completes at `deadline`. It registers nothing until
    /// it is polled.
    pub fn sleep_until(&self, deadline: Instant) -> Sleep {
        todo!("C: build the future; do not touch the heap yet")
    }

    /// B. The number of live timers: sleeps that were polled, are not ready
    /// yet, and were not dropped.
    pub fn pending(&self) -> usize {
        todo!("B: ask the heap")
    }
}

/// C. The future of [`Timers::sleep`]. It must be `Unpin` and `Send`.
///
/// - `poll` returns `Ready` if and only if the clock has reached the
///   deadline. It reads the clock itself, so it never completes early, also
///   not after a wake that came from somewhere else.
/// - Otherwise it makes sure that the driver holds the waker of *this* poll
///   for its timer, and returns `Pending`. It never wakes itself.
/// - Dropping the future cancels its timer.
/// - The lock rule applies: wakers are dropped after the lock is released.
pub struct Sleep {
    _design: (),
}

impl Sleep {
    /// The moment at which this future becomes ready.
    pub fn deadline(&self) -> Instant {
        todo!("C: the deadline")
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        todo!("C: ready, or registered with the newest waker")
    }
}

// C. Implement `Drop for Sleep` here (see the contract above).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{WakeCounter, WakeLog, block_on, guess, within};
    use std::sync::mpsc::{self, Sender};
    use std::task::Wake;

    const LONG: Duration = Duration::from_secs(3600);
    const LIMIT: Duration = Duration::from_secs(10);

    /// A waker that sends its name when it is woken.
    struct Named(&'static str, Mutex<Sender<&'static str>>);
    impl Wake for Named {
        fn wake(self: Arc<Self>) {
            let _ = self.1.lock().unwrap().send(self.0);
        }
    }
    fn named(name: &'static str, tx: &Sender<&'static str>) -> Waker {
        Waker::from(Arc::new(Named(name, Mutex::new(tx.clone()))))
    }

    /// Wakes every waker and returns the names in wake order.
    fn names(due: Vec<Waker>, rx: &mpsc::Receiver<&'static str>) -> Vec<&'static str> {
        for waker in due {
            waker.wake();
        }
        rx.try_iter().collect()
    }

    #[test]
    fn a_take_due_returns_timers_in_deadline_order() {
        let (tx, rx) = mpsc::channel();
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        let mut heap = TimerHeap::new();
        assert_eq!(heap.next_deadline(), None, "an empty heap has no deadline");
        heap.insert(at(30), named("thirty", &tx));
        heap.insert(at(10), named("ten", &tx));
        heap.insert(at(20), named("twenty", &tx));
        heap.insert(at(40), named("forty", &tx));
        assert_eq!(heap.len(), 4);
        assert_eq!(
            heap.next_deadline(),
            Some(at(10)),
            "next_deadline is the earliest deadline, not the first one inserted"
        );
        assert!(
            heap.take_due(at(9)).is_empty(),
            "nothing is due before the earliest deadline"
        );
        assert_eq!(
            names(heap.take_due(at(20)), &rx),
            ["ten", "twenty"],
            "a deadline that equals `now` is due, and earlier deadlines come first"
        );
        assert_eq!((heap.len(), heap.next_deadline()), (2, Some(at(30))));
        assert_eq!(names(heap.take_due(at(1000)), &rx), ["thirty", "forty"]);
        assert!(heap.is_empty());
        assert_eq!(heap.next_deadline(), None);
    }

    #[test]
    fn a_equal_deadlines_keep_insertion_order() {
        let (tx, rx) = mpsc::channel();
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut heap = TimerHeap::new();
        for name in ["first", "second", "third", "fourth", "fifth"] {
            heap.insert(deadline, named(name, &tx));
        }
        assert_eq!(
            names(heap.take_due(deadline), &rx),
            ["first", "second", "third", "fourth", "fifth"],
            "timers with the same deadline must come out in insertion order: \
             a waker cannot be compared, so order the entries by (deadline, id)"
        );
    }

    #[test]
    fn a_cancelled_timers_never_fire() {
        let (tx, rx) = mpsc::channel();
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        let mut heap = TimerHeap::new();
        let early = heap.insert(at(10), named("early", &tx));
        let middle = heap.insert(at(20), named("middle", &tx));
        let _late = heap.insert(at(30), named("late", &tx));

        let (counter, waker) = WakeCounter::new();
        let counted = heap.insert(at(15), waker);
        assert_eq!(Arc::strong_count(&counter), 2);
        let returned = heap.cancel(counted);
        assert!(
            returned.is_some(),
            "cancel must hand the waker of a live timer back to the caller"
        );
        assert_eq!(
            Arc::strong_count(&counter),
            2,
            "the heap must not keep a second copy of a cancelled timer's waker"
        );
        drop(returned);
        assert_eq!(
            heap.len(),
            3,
            "len counts live timers only: a cancelled timer is not live, \
             even if its entry is still stored somewhere"
        );

        assert!(heap.cancel(early).is_some());
        assert_eq!(
            (heap.len(), heap.next_deadline()),
            (2, Some(at(20))),
            "after the earliest timer is cancelled, the next live one is the earliest"
        );
        assert!(
            heap.cancel(early).is_none(),
            "a timer can be cancelled only once"
        );
        assert!(
            heap.waker_mut(early).is_none(),
            "a cancelled timer is not live"
        );
        assert!(heap.cancel(middle).is_some());
        assert_eq!(names(heap.take_due(at(100)), &rx), ["late"]);
        assert_eq!(counter.count(), 0, "a cancelled timer must never be woken");
        assert!(
            heap.cancel(TimerId(u64::MAX)).is_none(),
            "an unknown id is not live"
        );
    }

    #[test]
    fn a_waker_mut_replaces_the_stored_waker() {
        let (tx, rx) = mpsc::channel();
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut heap = TimerHeap::new();
        let id = heap.insert(deadline, named("old", &tx));
        let slot = heap.waker_mut(id).expect("the timer is live");
        *slot = named("new", &tx);
        assert_eq!(heap.len(), 1, "replacing a waker does not add a timer");
        assert_eq!(names(heap.take_due(deadline), &rx), ["new"]);
        assert!(
            heap.waker_mut(id).is_none(),
            "a timer that was taken is not live"
        );
    }

    #[test]
    fn a_ids_are_not_reused() {
        let mut heap = TimerHeap::new();
        let deadline = Instant::now();
        let first = heap.insert(deadline, Waker::noop().clone());
        heap.take_due(deadline);
        let second = heap.insert(deadline, Waker::noop().clone());
        assert_ne!(first, second, "an old id must never name a newer timer");
        assert!(heap.cancel(first).is_none());
        assert_eq!(heap.len(), 1);
    }

    #[test]
    fn b_c_sleep_waits_at_least_the_duration() {
        let waited = within(20, "block_on(timers.sleep(30 ms))", || {
            let driver = TimerDriver::start();
            let timers = driver.handle();
            let start = Instant::now();
            block_on(timers.sleep(Duration::from_millis(30)));
            (start.elapsed(), timers.pending())
        });
        assert!(
            waited.0 >= Duration::from_millis(30),
            "sleep returned after {:?}: it must never complete before its deadline",
            waited.0
        );
        assert_eq!(waited.1, 0, "a timer that fired is not live any more");
    }

    #[test]
    fn c_a_deadline_in_the_past_is_ready_at_once() {
        within(20, "polling a sleep whose deadline has passed", || {
            let driver = TimerDriver::start();
            let timers = driver.handle();
            let mut sleep = timers.sleep_until(Instant::now());
            assert_eq!(
                Pin::new(&mut sleep).poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(()),
                "a sleep whose deadline has passed is ready on its first poll"
            );
            assert_eq!(timers.pending(), 0, "a ready sleep registers no timer");
        });
    }

    #[test]
    fn b_c_the_driver_wakes_sleepers_in_deadline_order() {
        let order = within(20, "three sleeps with different deadlines", || {
            let driver = TimerDriver::start();
            let timers = driver.handle();
            let (tx, rx) = mpsc::channel();
            let start = Instant::now();
            let mut sleeps: Vec<Sleep> = [("c", 600), ("a", 250), ("b", 400)]
                .into_iter()
                .map(|(name, ms)| {
                    let mut sleep = timers.sleep_until(start + Duration::from_millis(ms));
                    let waker = named(name, &tx);
                    assert!(
                        Pin::new(&mut sleep)
                            .poll(&mut Context::from_waker(&waker))
                            .is_pending(),
                        "the deadline is in the future"
                    );
                    sleep
                })
                .collect();
            assert_eq!(timers.pending(), 3, "each pending sleep has one live timer");
            let order: Vec<_> = (0..3)
                .map(|_| {
                    rx.recv_timeout(LIMIT)
                        .expect("the driver did not wake a sleeper within 10 s")
                })
                .collect();
            assert!(
                start.elapsed() >= Duration::from_millis(600),
                "the last wake came before the last deadline"
            );
            for sleep in &mut sleeps {
                assert!(
                    Pin::new(sleep)
                        .poll(&mut Context::from_waker(Waker::noop()))
                        .is_ready(),
                    "after its wake a sleep must be ready"
                );
            }
            order
        });
        assert_eq!(
            order,
            ["a", "b", "c"],
            "the driver must wake sleepers in deadline order, \
             also when an earlier deadline is registered after a later one"
        );
    }

    #[test]
    fn c_sleep_does_not_wake_itself_and_cancels_on_drop() {
        within(20, "a sleep that is dropped early", || {
            let driver = TimerDriver::start();
            let timers = driver.handle();
            let (counter, waker) = WakeCounter::new();
            let mut sleep = timers.sleep(LONG);
            assert_eq!(timers.pending(), 0, "an unpolled sleep registers nothing");
            assert!(
                Pin::new(&mut sleep)
                    .poll(&mut Context::from_waker(&waker))
                    .is_pending()
            );
            assert!(
                Pin::new(&mut sleep)
                    .poll(&mut Context::from_waker(&waker))
                    .is_pending()
            );
            assert_eq!(
                timers.pending(),
                1,
                "polling one sleep twice must not register two timers"
            );
            assert_eq!(
                counter.count(),
                0,
                "a pending sleep must not wake itself; the driver wakes it at the deadline"
            );
            drop(sleep);
            assert_eq!(
                timers.pending(),
                0,
                "dropping a sleep must cancel its timer"
            );
            assert_eq!(
                Arc::strong_count(&counter),
                2,
                "a cancelled timer must release its waker"
            );
        });
    }

    #[test]
    fn c_sleep_keeps_only_the_newest_waker() {
        let woken = within(20, "a sleep that is polled with two wakers", || {
            let driver = TimerDriver::start();
            let timers = driver.handle();
            let (old, old_waker) = WakeCounter::new();
            let (new_waker, woken) = WakeLog::pair();
            let mut sleep = timers.sleep(Duration::from_millis(250));
            assert!(
                Pin::new(&mut sleep)
                    .poll(&mut Context::from_waker(&old_waker))
                    .is_pending()
            );
            assert!(
                Pin::new(&mut sleep)
                    .poll(&mut Context::from_waker(&new_waker))
                    .is_pending()
            );
            assert_eq!(
                Arc::strong_count(&old),
                2,
                "the waker of the first poll must be released when a newer one replaces it"
            );
            let waking_thread = woken
                .recv_timeout(LIMIT)
                .expect("the newest waker was not woken within 10 s");
            (old.count(), waking_thread != thread::current().id())
        });
        assert_eq!(
            woken,
            (0, true),
            "(wakes of the replaced waker, the wake came from the driver thread)"
        );
    }

    #[test]
    fn b_an_earlier_timer_interrupts_a_long_wait() {
        within(
            60,
            "short sleeps that start while the driver waits for a long one (when a \
             timer becomes the earliest one, the driver thread must be told)",
            || {
                let driver = TimerDriver::start();
                let timers = driver.handle();
                let mut long = timers.sleep(LONG);
                assert!(
                    Pin::new(&mut long)
                        .poll(&mut Context::from_waker(Waker::noop()))
                        .is_pending()
                );
                // After each round the driver thread goes back to waiting for
                // the long timer. Several rounds, because in a single round a
                // driver that is slow to start could see both timers at once.
                for _ in 0..5 {
                    block_on(timers.sleep(Duration::from_millis(20)));
                }
                assert_eq!(timers.pending(), 1, "only the long sleep is left");
            },
        );
    }

    /// A waker that calls into the timers when it is woken and when it is
    /// dropped. Both calls need the timer lock.
    struct Probe {
        timers: Timers,
        events: Mutex<Sender<&'static str>>,
    }
    impl Wake for Probe {
        fn wake(self: Arc<Self>) {
            self.timers.pending();
            let _ = self.events.lock().unwrap().send("woken");
        }
    }
    impl Drop for Probe {
        fn drop(&mut self) {
            self.timers.pending();
            let _ = self.events.lock().unwrap().send("dropped");
        }
    }
    fn probe(timers: &Timers) -> (Waker, mpsc::Receiver<&'static str>) {
        let (tx, rx) = mpsc::channel();
        let waker = Waker::from(Arc::new(Probe {
            timers: timers.clone(),
            events: Mutex::new(tx),
        }));
        (waker, rx)
    }

    #[test]
    fn b_the_driver_wakes_after_releasing_its_lock() {
        within(
            20,
            "a timer whose waker uses the timers again (the driver must release \
             its lock before it calls wake)",
            || {
                let driver = TimerDriver::start();
                let timers = driver.handle();
                let (waker, events) = probe(&timers);
                let mut sleep = timers.sleep(Duration::from_millis(250));
                assert!(
                    Pin::new(&mut sleep)
                        .poll(&mut Context::from_waker(&waker))
                        .is_pending()
                );
                drop(waker);
                assert_eq!(
                    events.recv_timeout(LIMIT),
                    Ok("woken"),
                    "the wake did not finish: the driver called wake while it held the timer lock"
                );
                assert_eq!(
                    events.recv_timeout(LIMIT),
                    Ok("dropped"),
                    "the fired timer held the last clone of the waker, so the wake drops it"
                );
                drop(sleep);
            },
        );
    }

    #[test]
    fn c_a_cancelled_waker_is_dropped_after_the_lock_is_released() {
        within(
            20,
            "dropping a Sleep whose timer holds the last clone of a waker (move the \
             waker out of the critical section, then drop it)",
            || {
                let driver = TimerDriver::start();
                let timers = driver.handle();
                let (waker, events) = probe(&timers);
                let mut sleep = timers.sleep(LONG);
                assert!(
                    Pin::new(&mut sleep)
                        .poll(&mut Context::from_waker(&waker))
                        .is_pending()
                );
                drop(waker);
                drop(sleep);
                assert_eq!(events.try_recv(), Ok("dropped"));
            },
        );
    }

    #[test]
    fn c_a_replaced_waker_is_dropped_after_the_lock_is_released() {
        within(
            20,
            "polling a Sleep with a new waker while its timer holds the last clone \
             of the old one (move the old waker out of the critical section, then drop it)",
            || {
                let driver = TimerDriver::start();
                let timers = driver.handle();
                let (waker, events) = probe(&timers);
                let mut sleep = timers.sleep(LONG);
                assert!(
                    Pin::new(&mut sleep)
                        .poll(&mut Context::from_waker(&waker))
                        .is_pending()
                );
                drop(waker);
                assert!(
                    Pin::new(&mut sleep)
                        .poll(&mut Context::from_waker(Waker::noop()))
                        .is_pending()
                );
                assert_eq!(events.try_recv(), Ok("dropped"));
            },
        );
    }

    #[test]
    fn b_dropping_the_driver_stops_the_thread_and_releases_wakers() {
        within(
            20,
            "dropping a TimerDriver that has pending timers (stop and join the \
             thread; drop the wakers outside the lock)",
            || {
                let driver = TimerDriver::start();
                let timers = driver.handle();
                let (counter, waker) = WakeCounter::new();
                let (probe_waker, events) = probe(&timers);
                let mut first = timers.sleep(LONG);
                let mut second = timers.sleep(LONG);
                assert!(
                    Pin::new(&mut first)
                        .poll(&mut Context::from_waker(&waker))
                        .is_pending()
                );
                assert!(
                    Pin::new(&mut second)
                        .poll(&mut Context::from_waker(&probe_waker))
                        .is_pending()
                );
                drop(probe_waker);
                drop(driver);
                assert_eq!(
                    Arc::strong_count(&counter),
                    2,
                    "dropping the driver must release every registered waker; \
                     if the thread is still running, it still owns them"
                );
                assert_eq!(
                    counter.count(),
                    0,
                    "shutdown releases wakers without waking"
                );
                assert_eq!(events.try_recv(), Ok("dropped"));
                // Late calls on a handle must not panic or block.
                drop(first);
                drop(second);
                assert_eq!(timers.pending(), 0);
            },
        );
    }

    #[test]
    fn c_sleep_is_unpin_and_send() {
        fn check<T: Unpin + Send>() {}
        check::<Sleep>();
        check::<Timers>();
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Three numbers go into a BinaryHeap. Which one does `pop` give
        // first?
        let mut plain = BinaryHeap::from([20, 5, 30]);
        let p1: i32 = guess();
        assert_eq!(Some(p1), plain.pop(), "P1: BinaryHeap is a max-heap");

        // P2. The same numbers, each wrapped in `Reverse`. Which number does
        // `pop` give first now?
        let mut reversed = BinaryHeap::from([Reverse(20), Reverse(5), Reverse(30)]);
        let p2: i32 = guess();
        assert_eq!(
            Some(p2),
            reversed.pop().map(|Reverse(n)| n),
            "P2: Reverse flips the order"
        );

        // P3. Entries are (deadline in ms, id). Two have the same deadline.
        // In which order do the ids come out?
        let mut entries = BinaryHeap::from([
            Reverse((50, 2)),
            Reverse((10, 3)),
            Reverse((50, 1)),
            Reverse((10, 4)),
        ]);
        let mut ids = Vec::new();
        while let Some(Reverse((_, id))) = entries.pop() {
            ids.push(id);
        }
        let p3: [i32; 4] = guess();
        assert_eq!(p3.as_slice(), ids, "P3: tuples compare field by field");

        // P4. `t0` is taken first and `t1` later. What does each expression
        // give? (`duration_since` and `saturating_duration_since` return the
        // same value for an earlier argument.)
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(5);
        let p4: (u128, u128) = guess();
        assert_eq!(
            p4,
            (
                t1.saturating_duration_since(t0).as_millis(),
                t0.saturating_duration_since(t1).as_millis(),
            ),
            "P4: (t1 since t0, t0 since t1) in milliseconds"
        );
    }
}
