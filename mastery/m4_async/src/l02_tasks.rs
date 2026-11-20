//! LESSON 02 — Tasks, a run queue, spawn and JoinHandle
//! Read first: CONCEPTS.md, "Lesson 02". Start after: lesson 01.
//! Run: cargo test --manifest-path mastery/m4_async/Cargo.toml l02_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. A task is woken three times before the executor gets to it. How often
//!    should it be polled, and what in your design makes that so?
//! 2. `block_on` returns while five spawned tasks are still waiting. Who owns
//!    their futures now, and when do the values inside them get dropped?
//!
//! IMPLEMENT the tasks; keep the tests. The types and their fields are given.
//! Read the invariants in the comments before you write a body.
//! EXPLAIN: Draw who owns whom: executor, spawner, shared state, run queue,
//! task, waker, join handle. Mark each arrow strong or weak, and show why no
//! cycle of strong arrows can survive `Executor::drop`.

use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::task::{Context, Poll, Wake, Waker};

/// The future of a spawned task, with its type erased. The output is `()`:
/// the task stores its real result in the join state before it finishes.
type TaskFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// The error of a [`JoinHandle`] whose task ended without a result: the
/// executor was dropped first, the task was aborted, or the task panicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

/// Locks a mutex and ignores poisoning. Every critical section in this file
/// leaves the data consistent, so the data is usable after a panic elsewhere.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One entry of the run queue.
enum Runnable {
    /// The future that was passed to `block_on`. It lives on the stack of
    /// `block_on`, so the queue only holds this marker for it.
    Root,
    Task(Arc<Task>),
}

/// State shared by the executor, its spawners, and (weakly) its tasks.
struct Shared {
    state: Mutex<State>,
    /// Signalled when something is pushed to an empty queue.
    work: Condvar,
}

struct State {
    /// Entries that are ready to be polled, oldest first.
    queue: VecDeque<Runnable>,
    /// `true` while a `Runnable::Root` entry is in `queue`.
    root_queued: bool,
    /// Every task that has not finished, by id. This is the owner of the
    /// tasks: wakers and join handles may come and go.
    tasks: BTreeMap<u64, Arc<Task>>,
    next_id: u64,
    /// Set by `Executor::drop`. Nothing is queued or registered afterwards.
    closed: bool,
}

struct Task {
    id: u64,
    /// `Some` until the task finishes or is cancelled. Only the thread that
    /// is inside `block_on` takes the future out, polls it, and puts it back.
    future: Mutex<Option<TaskFuture>>,
    /// `true` from the moment the task is pushed to the queue until the
    /// executor takes it out. It keeps a task from being queued twice.
    queued: AtomicBool,
    /// Set by `JoinHandle::abort`.
    aborted: AtomicBool,
    /// Weak, because `Shared` owns the tasks.
    shared: Weak<Shared>,
}

impl Task {
    /// A. Put this task on the run queue unless it is there already, or the
    /// executor is gone or closed. Wake the executor thread *after* the lock
    /// is released.
    fn schedule(self: &Arc<Self>) {
        todo!("A: set `queued`; push only if it was not set before")
    }

    /// B. Run one step of this task on the executor thread.
    ///
    /// Clear `queued` first, so that a wake during the poll queues the task
    /// again. Take the future out of its slot. If the task was aborted, drop
    /// the future. Otherwise poll it with a waker made from this task; put it
    /// back if it is pending. A finished or dropped future means the task is
    /// over: remove it from `State::tasks`. Hold no lock while you poll or
    /// drop a future.
    fn run(self: Arc<Self>) {
        todo!("B: one poll, and the checks around it")
    }
}

/// A. A task is its own wake target: waking means scheduling.
impl Wake for Task {
    fn wake(self: Arc<Self>) {
        todo!("A: schedule this task")
    }

    fn wake_by_ref(self: &Arc<Self>) {
        todo!("A: schedule this task")
    }
}

/// The wake target of the future that `block_on` runs.
struct Root {
    shared: Weak<Shared>,
}

/// A. Waking the root pushes `Runnable::Root` unless one is queued already
/// (`State::root_queued`), then signals the executor thread.
impl Wake for Root {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        todo!("A: queue the root marker once")
    }
}

/// A single-threaded executor: every task is polled on the thread that calls
/// [`Executor::block_on`]. Wakes may come from any thread.
pub struct Executor {
    shared: Arc<Shared>,
}

impl Executor {
    /// An executor with an empty queue.
    pub fn new() -> Executor {
        Executor {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    queue: VecDeque::new(),
                    root_queued: false,
                    tasks: BTreeMap::new(),
                    next_id: 0,
                    closed: false,
                }),
                work: Condvar::new(),
            }),
        }
    }

    /// A handle that spawns tasks onto this executor. It can be cloned and
    /// sent to other threads and into tasks.
    pub fn spawner(&self) -> Spawner {
        Spawner {
            shared: Arc::clone(&self.shared),
        }
    }

    /// B. Run `future` and all spawned tasks on this thread until `future`
    /// is ready, and return its output. Tasks that are still alive then stay
    /// with the executor: a later `block_on` continues them, and dropping
    /// the executor cancels them.
    ///
    /// Take queue entries in order, oldest first, one at a time. When the
    /// queue is empty, sleep on `Shared::work` until something is pushed; do
    /// not spin. The root future is queued once at the start. A panic in a
    /// task or in `future` passes through `block_on`.
    ///
    /// `&mut self` is a promise the compiler checks: only one thread at a
    /// time polls the tasks of this executor.
    pub fn block_on<F: Future>(&mut self, future: F) -> F::Output {
        todo!("B: the executor loop")
    }
}

impl Default for Executor {
    fn default() -> Self {
        Executor::new()
    }
}

// D. Implement `Drop for Executor` here. Dropping the executor cancels every
// task that has not finished: mark the state closed, empty the queue and the
// task table, and drop each future. Drop the futures after the state lock is
// released, because a destructor may wake a task or spawn.

/// Spawns tasks onto one executor.
#[derive(Clone)]
pub struct Spawner {
    shared: Arc<Shared>,
}

impl Spawner {
    /// C. Create a task for `future` and put it on the run queue. The future
    /// is not polled here; it first runs inside `block_on`.
    ///
    /// Wrap `future` so that its output goes to the join state: build a
    /// `Completer`, and box an async block that awaits `future` and then
    /// calls `Completer::complete`. If the wrapper is dropped early, the
    /// completer's `Drop` reports `Cancelled`.
    ///
    /// If the executor is closed, drop the future at once (outside the
    /// lock); the handle then reports `Cancelled`.
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        todo!("C: build the task, register it, queue it")
    }
}

/// What a join handle and its task share.
struct Join<T> {
    state: Mutex<JoinState<T>>,
}

struct JoinState<T> {
    outcome: Outcome<T>,
    /// The waker of whoever awaits the handle.
    waiter: Option<Waker>,
}

enum Outcome<T> {
    Running,
    Done(T),
    Cancelled,
    /// The handle has already returned the value.
    Taken,
}

/// The task's end of the join state. Exactly one of `complete` and `drop`
/// decides the outcome.
struct Completer<T>(Arc<Join<T>>);

impl<T> Completer<T> {
    /// C. The task finished with `output`.
    fn complete(self, output: T) {
        todo!("C: store Done(output) and wake the waiter")
    }

    /// C. If the outcome is still `Running`, replace it with `outcome` and
    /// wake the waiter, after the lock is released. Otherwise do nothing.
    fn finish(&self, outcome: Outcome<T>) {
        todo!("C: change Running to the given outcome, exactly once")
    }
}

// C. Implement `Drop for Completer<T>` here. A completer that is dropped
// without `complete` means that the task's future was dropped before it
// finished: report `Cancelled`.

/// Awaits the result of a spawned task. Dropping the handle does not stop
/// the task: it keeps running, and its result is thrown away.
pub struct JoinHandle<T> {
    join: Arc<Join<T>>,
    task: Weak<Task>,
}

impl<T> JoinHandle<T> {
    /// D. Ask the executor to drop the task's future instead of polling it
    /// again. Only set the flag and schedule the task; the executor thread
    /// does the dropping (see `Task::run`). A task that has already finished
    /// keeps its result.
    pub fn abort(&self) {
        todo!("D: flag the task and schedule it")
    }

    /// C. `true` once the task has finished or was cancelled.
    pub fn is_finished(&self) -> bool {
        todo!("C: look at the outcome")
    }
}

/// C. `Ok(output)` when the task finished, `Err(Cancelled)` when it ended
/// without a result. While the task runs, store the newest waker and return
/// `Pending`. Polling again after `Ok` is a bug of the caller: panic.
impl<T> Future for JoinHandle<T> {
    type Output = Result<T, Cancelled>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        todo!("C: report the outcome or register the waker")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{DropLog, Gate, WakeCounter, guess, within, yield_now};
    use std::sync::atomic::AtomicUsize;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    fn log_to(log: &Arc<Mutex<Vec<String>>>, entry: impl Into<String>) {
        log.lock().unwrap().push(entry.into());
    }

    #[test]
    fn b_block_on_returns_the_output() {
        let output = within(10, "block_on(async { 6 * 7 })", || {
            Executor::new().block_on(async { 6 * 7 })
        });
        assert_eq!(output, 42, "block_on must return the future's output");
    }

    #[test]
    fn a_b_root_future_is_woken_from_another_thread() {
        let output = within(
            10,
            "block_on of a future that another thread completes",
            || {
                let gate = Gate::new();
                let opener = gate.clone();
                let mut executor = Executor::new();
                let helper = thread::spawn(move || opener.open());
                executor.block_on(gate.wait());
                helper.join().unwrap();
                gate.polls()
            },
        );
        assert!(output >= 1, "the root future must be polled at least once");
    }

    #[test]
    fn b_c_spawned_tasks_run_and_join() {
        let outputs = within(10, "joining three tasks", || {
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            executor.block_on(async move {
                let a = spawner.spawn(async { 1 });
                let b = spawner.spawn(async { String::from("two") });
                let c = spawner.spawn(async {
                    yield_now().await;
                    3.0
                });
                (a.await, b.await, c.await)
            })
        });
        assert_eq!(
            outputs,
            (Ok(1), Ok(String::from("two")), Ok(3.0)),
            "each JoinHandle must give the output of its own task"
        );
    }

    #[test]
    fn c_spawn_does_not_poll() {
        let executor = Executor::new();
        let spawner = executor.spawner();
        let (future, report) = crate::support::Anchored::new(0, ());
        let handle = spawner.spawn(future);
        assert_eq!(
            report.polls(),
            0,
            "spawn must only queue the task; it is first polled inside block_on"
        );
        assert!(
            !handle.is_finished(),
            "a task that never ran is not finished"
        );
    }

    #[test]
    fn b_tasks_take_turns_in_queue_order() {
        let log = within(10, "three tasks that yield", || {
            let log = Arc::new(Mutex::new(Vec::new()));
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            let handles: Vec<_> = ["a", "b", "c"]
                .into_iter()
                .map(|name| {
                    let log = Arc::clone(&log);
                    spawner.spawn(async move {
                        for step in 1..=2 {
                            log_to(&log, format!("{name}{step}"));
                            yield_now().await;
                        }
                    })
                })
                .collect();
            executor.block_on(async move {
                for handle in handles {
                    handle.await.unwrap();
                }
            });
            log.lock().unwrap().clone()
        });
        assert_eq!(
            log,
            ["a1", "b1", "c1", "a2", "b2", "c2"],
            "a task that yields goes to the back of the queue, so the tasks alternate"
        );
    }

    #[test]
    fn b_a_busy_root_does_not_starve_the_tasks() {
        let turns = within(10, "a root future that yields in a loop", || {
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            let done = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&done);
            let handle = spawner.spawn(async move { flag.store(true, Ordering::SeqCst) });
            executor.block_on(async move {
                let mut turns = 0;
                while !done.load(Ordering::SeqCst) {
                    turns += 1;
                    assert!(
                        turns < 1000,
                        "the root future has yielded 1000 times and the spawned task \
                         still has not run: the root must wait in the same queue as the tasks"
                    );
                    yield_now().await;
                }
                handle.await.unwrap();
                turns
            })
        });
        assert!(
            turns <= 2,
            "the task was queued before the root's second turn"
        );
    }

    #[test]
    fn a_many_wakes_give_one_poll() {
        let polls = within(10, "a task that is woken three times in a row", || {
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            let polls = Arc::new(AtomicUsize::new(0));
            let release = Arc::new(AtomicBool::new(false));
            let stash: Arc<Mutex<Option<Waker>>> = Arc::default();
            let (seen, go, keep) = (Arc::clone(&polls), Arc::clone(&release), Arc::clone(&stash));
            let handle = spawner.spawn(std::future::poll_fn(move |cx| {
                let poll_number = seen.fetch_add(1, Ordering::SeqCst) + 1;
                if poll_number == 1 {
                    cx.waker().wake_by_ref();
                    cx.waker().wake_by_ref();
                    let clone = cx.waker().clone();
                    clone.wake();
                    return Poll::Pending;
                }
                if !go.load(Ordering::SeqCst) {
                    *keep.lock().unwrap() = Some(cx.waker().clone());
                    return Poll::Pending;
                }
                Poll::Ready(())
            }));
            executor.block_on(async move {
                // Give a task that was queued too often the chance to show it.
                for _ in 0..5 {
                    yield_now().await;
                }
                let polls_after_three_wakes = polls.load(Ordering::SeqCst);
                release.store(true, Ordering::SeqCst);
                let waker = stash
                    .lock()
                    .unwrap()
                    .take()
                    .expect("the task was polled twice");
                waker.wake();
                handle.await.unwrap();
                polls_after_three_wakes
            })
        });
        assert_eq!(
            polls, 2,
            "three wakes before the task's next turn must queue it once: \
             one first poll, and one poll for all three wakes"
        );
    }

    #[test]
    fn a_b_a_wake_from_another_thread_gets_a_sleeping_executor_going() {
        let output = within(10, "a task that is completed by a helper thread", || {
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            let gate = Gate::new();
            let wait = gate.wait();
            let handle = spawner.spawn(async move {
                wait.await;
                "woken"
            });
            let (polled_tx, polled_rx) = mpsc::channel();
            let opener = gate.clone();
            let helper = thread::spawn(move || {
                // Open the gate only after the task has gone to sleep on it.
                polled_rx.recv().unwrap();
                opener.open();
            });
            let watched = gate.clone();
            let output = executor.block_on(async move {
                while watched.polls() == 0 {
                    yield_now().await;
                }
                polled_tx.send(()).unwrap();
                handle.await
            });
            helper.join().unwrap();
            output
        });
        assert_eq!(output, Ok("woken"));
    }

    #[test]
    fn b_an_idle_executor_sleeps() {
        const QUIET: Duration = Duration::from_millis(200);
        let (waker_tx, waker_rx) = mpsc::channel::<Waker>();
        let gate = Gate::new();
        let opener = gate.clone();
        let (done_tx, done_rx) = mpsc::channel();
        thread::spawn(move || {
            let mut executor = Executor::new();
            let wait = gate.wait();
            let mut wait = pin!(wait);
            executor.block_on(std::future::poll_fn(move |cx| {
                let _ = waker_tx.send(cx.waker().clone());
                wait.as_mut().poll(cx)
            }));
            let _ = done_tx.send(());
        });
        let _first_waker = waker_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("block_on did not poll the root future within 10 s");
        // Nobody wakes the root for a while. An executor that sleeps does not
        // poll; one that spins or polls on a timer does.
        thread::sleep(QUIET);
        let polls_while_quiet = opener.polls();
        opener.open();
        done_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the gate was opened, which wakes the root, but block_on did not return");
        assert!(
            polls_while_quiet <= 2,
            "the root future was polled {polls_while_quiet} times in {QUIET:?} although nobody \
             woke it: when the queue is empty, block_on must wait on the condition variable"
        );
    }

    #[test]
    fn b_everything_is_polled_on_the_block_on_thread() {
        let (expected, seen) = within(10, "tasks that record their thread", || {
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    spawner.spawn(async {
                        yield_now().await;
                        thread::current().id()
                    })
                })
                .collect();
            let seen = executor.block_on(async move {
                let mut seen = Vec::new();
                for handle in handles {
                    seen.push(handle.await.unwrap());
                }
                seen
            });
            (thread::current().id(), seen)
        });
        assert!(
            seen.iter().all(|id| *id == expected),
            "every task must be polled on the thread that called block_on"
        );
    }

    #[test]
    fn b_c_a_finished_task_is_dropped_at_once_and_late_wakes_are_harmless() {
        within(
            10,
            "a task that finishes while a waker to it is kept",
            || {
                let mut executor = Executor::new();
                let spawner = executor.spawner();
                let drops = DropLog::new();
                let kept: Arc<Mutex<Option<Waker>>> = Arc::default();
                let keep = Arc::clone(&kept);
                let guard = drops.guard("task state");
                let handle = spawner.spawn(async move {
                    let _guard = guard;
                    std::future::poll_fn(|cx| {
                        *keep.lock().unwrap() = Some(cx.waker().clone());
                        Poll::Ready(())
                    })
                    .await;
                });
                let seen = drops.clone();
                executor.block_on(async move {
                    handle.await.unwrap();
                    assert_eq!(
                        seen.events(),
                        ["task state"],
                        "everything that a task owns must be dropped when the task finishes, \
                     not when the executor is dropped"
                    );
                });
                let late = kept
                    .lock()
                    .unwrap()
                    .take()
                    .expect("the task stored its waker");
                late.wake_by_ref();
                late.wake();
                // A stale entry in the queue must not be polled or panic.
                executor.block_on(async {
                    yield_now().await;
                });
            },
        );
    }

    #[test]
    fn c_dropping_a_handle_detaches_the_task() {
        let ran = within(10, "a detached task", || {
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            let ran = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&ran);
            drop(spawner.spawn(async move {
                yield_now().await;
                flag.store(true, Ordering::SeqCst);
            }));
            executor.block_on(async {
                for _ in 0..3 {
                    yield_now().await;
                }
            });
            ran.load(Ordering::SeqCst)
        });
        assert!(ran, "dropping the JoinHandle must not stop the task");
    }

    #[test]
    fn c_join_handle_stores_the_newest_waker() {
        within(10, "polling a JoinHandle by hand", || {
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            let gate = Gate::new();
            let wait = gate.wait();
            let mut handle = spawner.spawn(async move {
                wait.await;
                7
            });
            let (old, old_waker) = WakeCounter::new();
            let (new, new_waker) = WakeCounter::new();
            assert!(
                Pin::new(&mut handle)
                    .poll(&mut Context::from_waker(&old_waker))
                    .is_pending(),
                "the task has not run yet"
            );
            assert!(
                Pin::new(&mut handle)
                    .poll(&mut Context::from_waker(&new_waker))
                    .is_pending()
            );
            assert_eq!(
                Arc::strong_count(&old),
                2,
                "the waker of the first poll must be dropped when a newer one replaces it"
            );
            gate.open();
            executor.block_on(async {
                yield_now().await;
                yield_now().await;
            });
            assert_eq!(
                (old.count(), new.count()),
                (0, 1),
                "(wakes of the old waker, wakes of the new one) when the task finishes"
            );
            assert!(handle.is_finished());
            assert_eq!(
                Pin::new(&mut handle).poll(&mut Context::from_waker(&new_waker)),
                Poll::Ready(Ok(7))
            );
        });
    }

    #[test]
    fn d_dropping_the_executor_cancels_what_is_left() {
        within(10, "dropping an executor with unfinished tasks", || {
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            let drops = DropLog::new();
            let gate = Gate::new();

            let (wait, guard) = (gate.wait(), drops.guard("waiting"));
            let mut waiting = spawner.spawn(async move {
                let _guard = guard;
                wait.await;
            });
            // Run the task up to its await.
            let watched = gate.clone();
            executor.block_on(async move {
                while watched.polls() == 0 {
                    yield_now().await;
                }
            });
            // This task is queued but never gets a turn.
            let guard = drops.guard("never polled");
            let mut never_polled = spawner.spawn(async move {
                let _guard = guard;
            });
            let (counter, waker) = WakeCounter::new();
            let mut cx = Context::from_waker(&waker);
            assert!(Pin::new(&mut waiting).poll(&mut cx).is_pending());
            assert!(
                drops.events().is_empty(),
                "nothing may be dropped while the executor is alive and the tasks are unfinished"
            );

            drop(executor);

            let mut events = drops.events();
            events.sort_unstable();
            assert_eq!(
                events,
                ["never polled", "waiting"],
                "dropping the executor must drop the future of every unfinished task"
            );
            assert_eq!(
                counter.count(),
                1,
                "a handle that waits for a cancelled task must be woken"
            );
            assert_eq!(
                Pin::new(&mut waiting).poll(&mut cx),
                Poll::Ready(Err(Cancelled))
            );
            assert_eq!(
                Pin::new(&mut never_polled).poll(&mut cx),
                Poll::Ready(Err(Cancelled))
            );

            // The gate still holds a waker of the cancelled task. Using it
            // must be harmless.
            gate.open();

            let guard = drops.guard("too late");
            let mut too_late = spawner.spawn(async move {
                let _guard = guard;
            });
            assert!(
                drops.events().contains(&"too late"),
                "a future that is spawned after the executor is gone must be dropped at once"
            );
            assert_eq!(
                Pin::new(&mut too_late).poll(&mut cx),
                Poll::Ready(Err(Cancelled))
            );
        });
    }

    #[test]
    fn d_a_destructor_may_use_the_executor_while_it_is_dropped() {
        /// Wakes another task and spawns a new one when it is dropped, as
        /// a connection that says goodbye in its destructor does.
        struct LastWords {
            spawner: Spawner,
            other_task: Arc<Mutex<Option<Waker>>>,
            spawned: mpsc::Sender<JoinHandle<()>>,
        }
        impl Drop for LastWords {
            fn drop(&mut self) {
                if let Some(waker) = self.other_task.lock().unwrap().take() {
                    waker.wake();
                }
                let _ = self.spawned.send(self.spawner.spawn(async {}));
            }
        }

        within(
            10,
            "dropping an executor that owns a task whose destructor wakes and spawns \
             (drop the futures after the state lock is released)",
            || {
                let mut executor = Executor::new();
                let spawner = executor.spawner();
                // A parked task. Its waker is what the destructor wakes.
                let parked: Arc<Mutex<Option<Waker>>> = Arc::default();
                let keep = Arc::clone(&parked);
                drop(spawner.spawn(std::future::poll_fn(move |cx| {
                    *keep.lock().unwrap() = Some(cx.waker().clone());
                    Poll::<()>::Pending
                })));
                let (spawned_tx, spawned_rx) = mpsc::channel();
                let last_words = LastWords {
                    spawner: spawner.clone(),
                    other_task: Arc::clone(&parked),
                    spawned: spawned_tx,
                };
                let gate = Gate::new();
                let wait = gate.wait();
                drop(spawner.spawn(async move {
                    let _last_words = last_words;
                    wait.await;
                }));
                // Run both tasks up to the point where they wait.
                let watched = gate.clone();
                executor.block_on(async move {
                    while watched.polls() == 0 {
                        yield_now().await;
                    }
                });
                assert!(
                    parked.lock().unwrap().is_some(),
                    "the parked task was polled and has stored its waker"
                );

                drop(executor);

                let mut late = spawned_rx
                    .try_recv()
                    .expect("dropping the executor must drop the future that owns `LastWords`");
                assert_eq!(
                    Pin::new(&mut late).poll(&mut Context::from_waker(Waker::noop())),
                    Poll::Ready(Err(Cancelled)),
                    "a task that is spawned while the executor is being dropped is cancelled at once"
                );
            },
        );
    }

    #[test]
    fn b_d_a_task_that_is_over_leaves_the_task_table() {
        within(10, "running three tasks and aborting a fourth", || {
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            let gate = Gate::new();
            let finishing: Vec<_> = (0..3)
                .map(|n| {
                    spawner.spawn(async move {
                        yield_now().await;
                        n
                    })
                })
                .collect();
            // This one waits for a gate that nobody opens.
            let aborted = spawner.spawn(gate.wait());
            let registered = |executor: &Executor| lock(&executor.shared.state).tasks.len();
            assert_eq!(
                registered(&executor),
                4,
                "tasks in `State::tasks` after four calls of `spawn`"
            );
            executor.block_on(async move {
                for handle in finishing {
                    handle.await.unwrap();
                }
                aborted.abort();
                assert_eq!(aborted.await, Err(Cancelled));
            });
            assert_eq!(
                registered(&executor),
                0,
                "tasks left in `State::tasks` although every task has finished or was aborted: \
                 `Task::run` must remove a task that is over, or a server that runs for weeks \
                 keeps an entry for every task it ever had"
            );
        });
    }

    #[test]
    fn d_abort_drops_the_future_on_the_executor_thread() {
        within(10, "aborting a task", || {
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            let drops = DropLog::new();
            let gate = Gate::new();
            let (wait, guard) = (gate.wait(), drops.guard("aborted task"));
            let handle = spawner.spawn(async move {
                let _guard = guard;
                wait.await;
                "finished"
            });
            let finished = spawner.spawn(async { "kept" });
            let seen = drops.clone();
            let outcomes = executor.block_on(async move {
                yield_now().await;
                handle.abort();
                assert!(
                    seen.events().is_empty(),
                    "abort must only flag and schedule the task; \
                     the executor drops the future on the task's next turn"
                );
                let aborted = handle.await;
                assert_eq!(seen.events(), ["aborted task"]);
                // Aborting a finished task changes nothing.
                yield_now().await;
                finished.abort();
                (aborted, finished.await)
            });
            assert_eq!(
                outcomes,
                (Err(Cancelled), Ok("kept")),
                "(aborted task, task that had finished before abort)"
            );
        });
    }

    #[test]
    fn b_d_a_panicking_task_does_not_break_the_executor() {
        within(10, "a task that panics", || {
            let mut executor = Executor::new();
            let spawner = executor.spawner();
            let drops = DropLog::new();
            let guard = drops.guard("bystander");
            let gate = Gate::new();
            let wait = gate.wait();
            let mut bystander = spawner.spawn(async move {
                let _guard = guard;
                wait.await;
            });
            let mut doomed = spawner.spawn(async {
                yield_now().await;
                panic!("this panic is part of the test");
            });
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                executor.block_on(std::future::pending::<()>())
            }));
            assert!(
                result.is_err(),
                "the panic of a task must pass through block_on"
            );
            let mut cx = Context::from_waker(Waker::noop());
            assert_eq!(
                Pin::new(&mut doomed).poll(&mut cx),
                Poll::Ready(Err(Cancelled)),
                "the handle of a task that panicked reports Cancelled"
            );
            // The executor is still usable and can be dropped normally.
            executor.block_on(async {
                yield_now().await;
            });
            drop(executor);
            assert_eq!(drops.events(), ["bystander"]);
            assert_eq!(
                Pin::new(&mut bystander).poll(&mut cx),
                Poll::Ready(Err(Cancelled))
            );
        });
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // The predictions use only the standard library: a queue of boxed
        // futures that is polled round-robin with a waker that does nothing.
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut queue: VecDeque<Pin<Box<dyn Future<Output = ()>>>> = VecDeque::new();
        for (name, steps) in [("x", 2), ("y", 1), ("z", 3)] {
            let log = Arc::clone(&log);
            queue.push_back(Box::pin(async move {
                for step in 1..=steps {
                    log_to(&log, format!("{name}{step}"));
                    yield_now().await;
                }
            }));
        }
        let mut cx = Context::from_waker(Waker::noop());
        let mut polls = 0;
        while let Some(mut future) = queue.pop_front() {
            polls += 1;
            if future.as_mut().poll(&mut cx).is_pending() {
                queue.push_back(future);
            }
        }

        // P1. In which order were the steps logged? Write them as one string
        // with single spaces, for example "x1 x2 y1 z1 z2 z3".
        let p1: &str = guess();
        assert_eq!(p1, log.lock().unwrap().join(" "), "P1: round-robin order");

        // P2. How many polls did the loop make in total? (A task with n steps
        // yields n times.)
        let p2: usize = guess();
        assert_eq!(p2, polls, "P2: polls until all three tasks finished");

        // P3. An `Arc` is shared by a queue entry and by two wakers that were
        // cloned from one another, and by nothing else. How many strong
        // references does it have?
        let (counter, waker) = WakeCounter::new();
        let queue_entry = Arc::clone(&counter);
        let second = waker.clone();
        drop(counter);
        let p3: usize = guess();
        assert_eq!(
            p3,
            Arc::strong_count(&queue_entry),
            "P3: one reference per owner"
        );
        drop((waker, second));
    }
}
