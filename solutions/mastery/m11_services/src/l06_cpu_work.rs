//! LESSON 06 — CPU-bound work in a service: rayon, and the bridge to async
//! Read first: CONCEPTS.md, "Lesson 06". Start after: lesson 01 and ecosystem E06.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml l06_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: an async handler calls a function
//! that computes for 200 ms and contains no `.await`. The runtime has four worker
//! threads. What happens to the other requests when five such calls arrive at the
//! same time?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! Several tests check *how* you solved a task: they need two calls in flight at
//! the same moment, or look at the thread a job ran on, or let a job wait for a
//! message that only a free async runtime can send.
//! EXPLAIN afterward: Why does a blocked async worker thread hurt requests that
//! have nothing to do with the slow one? Why does `CpuLimiter` release its slot
//! when the job ends and not when the caller goes away? When is `spawn_blocking`
//! enough and when do you want a rayon pool?
//! JUDGMENT: rayon is the standard tool for data parallelism: it splits one big
//! computation over the cores. tokio's `spawn_blocking` pool is meant for blocking
//! calls; it may grow to hundreds of threads, so it does not limit CPU use by
//! itself. For CPU work in a service, give the work its own pool sized to the
//! cores you can spare, bound the queue in front of it, and reject when the queue
//! is full. If single jobs take seconds, consider a separate worker process and a
//! job queue instead, so that deploys and crashes of one do not affect the other.

use rayon::prelude::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use tokio::sync::{Semaphore, oneshot};

/// A. Compute `score(item)` for every item, in parallel, and return the scores in
/// the order of `items`. Use rayon's parallel iterators.
///
/// Keep the signature. `F` is `Sync` but not `Send`, and rayon wants a closure
/// it can send to its threads. You do not need more than `Sync` for that: ask
/// yourself what is true of `&F`.
pub fn score_all<T, F>(items: &[T], score: F) -> Vec<u64>
where
    T: Sync,
    F: Fn(&T) -> u64 + Sync,
{
    // `collect` on an indexed parallel iterator keeps the input order.
    // `&score` is `Send` because `F` is `Sync`; that is all rayon needs.
    items.par_iter().map(&score).collect()
}

/// A. The index and score of the best item, computed in parallel. `None` for an
/// empty slice. When several items share the best score, the answer is the one
/// with the **lowest index**, on every run: a parallel reduction combines partial
/// results in an order you do not control, so the tie rule must be part of what
/// you compare.
pub fn best<T, F>(items: &[T], score: F) -> Option<(usize, u64)>
where
    T: Sync,
    F: Fn(&T) -> u64 + Sync,
{
    items
        .par_iter()
        .enumerate()
        .map(|(index, item)| (score(item), std::cmp::Reverse(index)))
        // Tuples compare field by field: highest score first, then the highest
        // `Reverse(index)`, which is the lowest index.
        .max()
        .map(|(score, std::cmp::Reverse(index))| (index, score))
}

/// B. `pixels` holds rows of `width` bytes each (`width > 0`, and the length is a
/// multiple of `width`). Call `edit(row_index, row)` once for every row, with the
/// rows processed in parallel. Use `rayon::scope` and spawn one task per row: a
/// scope lets tasks borrow `pixels` and `edit` from this stack frame, because it
/// does not return before all of its tasks have finished.
pub fn edit_rows<F>(pixels: &mut [u8], width: usize, edit: F)
where
    F: Fn(usize, &mut [u8]) + Sync,
{
    let edit = &edit;
    rayon::scope(|scope| {
        // `chunks_mut` hands out non-overlapping `&mut` rows, so each task owns
        // its row and no lock is needed.
        for (index, row) in pixels.chunks_mut(width).enumerate() {
            scope.spawn(move |_| edit(index, row));
        }
    });
}

/// C. Build a dedicated pool for CPU work with exactly `threads` threads, named
/// `cpu-0`, `cpu-1`, ... A named thread shows up in debuggers, profilers and
/// panic messages.
pub fn cpu_pool(threads: usize) -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .thread_name(|index| format!("cpu-{index}"))
        .build()
        .expect("building a rayon pool")
}

/// The job did not produce a result: it panicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobLost;

/// D. Run `job` on `pool` and wait for its result **without blocking the async
/// thread**: send the job to the pool with `ThreadPool::spawn`, and carry the
/// result back through a `tokio::sync::oneshot` channel that you `.await`.
///
/// A job that panics gives `Err(JobLost)`. Catch the panic inside the job
/// (`std::panic::catch_unwind`); a panic that escapes a spawned rayon job goes to
/// the pool's panic handler, and without a handler rayon aborts the process.
pub async fn offload<R, F>(pool: &rayon::ThreadPool, job: F) -> Result<R, JobLost>
where
    R: Send + 'static,
    F: FnOnce() -> R + Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    pool.spawn(move || {
        // The closure is consumed here and nothing observes its captures after a
        // panic, so asserting unwind safety is sound.
        if let Ok(value) = catch_unwind(AssertUnwindSafe(job)) {
            // The receiver is gone if the caller was cancelled; that is fine.
            let _ = tx.send(value);
        }
    });
    // A dropped sender means the job panicked.
    rx.await.map_err(|_| JobLost)
}

/// Why [`CpuLimiter::run`] returned no result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuError {
    /// Too many jobs are queued or running. The job was not started.
    Overloaded,
    /// The job panicked.
    JobLost,
}

/// E. A bounded entrance to a CPU pool. At most `max_pending` jobs may be queued
/// or running at once; one more is refused at once with `Overloaded`, so the
/// queue in front of the pool cannot grow without limit.
///
/// A job counts as pending from the moment `run` accepts it until the job itself
/// has finished. If the caller stops waiting (its future is dropped), the job
/// still runs and still uses a CPU, so its slot stays taken until it ends.
///
/// You may change the private fields.
#[derive(Debug, Clone)]
pub struct CpuLimiter {
    pool: Arc<rayon::ThreadPool>,
    slots: Arc<Semaphore>,
    max_pending: usize,
}

impl CpuLimiter {
    pub fn new(pool: Arc<rayon::ThreadPool>, max_pending: usize) -> CpuLimiter {
        CpuLimiter {
            pool,
            slots: Arc::new(Semaphore::new(max_pending)),
            max_pending,
        }
    }

    /// Jobs that are queued or running right now.
    pub fn pending(&self) -> usize {
        self.max_pending - self.slots.available_permits()
    }

    /// Run `job` on the pool like [`offload`], within the limit.
    pub async fn run<R, F>(&self, job: F) -> Result<R, CpuError>
    where
        R: Send + 'static,
        F: FnOnce() -> R + Send + 'static,
    {
        let slot = Arc::clone(&self.slots)
            .try_acquire_owned()
            .map_err(|_| CpuError::Overloaded)?;
        let (tx, rx) = oneshot::channel();
        self.pool.spawn(move || {
            let outcome = catch_unwind(AssertUnwindSafe(job));
            // The slot belongs to the job, not to the caller: free it when the
            // CPU is free, and before the caller can observe the result.
            drop(slot);
            if let Ok(value) = outcome {
                let _ = tx.send(value);
            }
        });
        rx.await.map_err(|_| CpuError::JobLost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::deadline;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Condvar, Mutex, mpsc};
    use std::time::Duration;

    /// A meeting point that proves calls overlap in time: `attend` returns `true`
    /// once `quorum` callers have been inside at the same moment. A caller that
    /// waits 5 s alone gives up, and then everyone gets `false`.
    struct Meeting {
        quorum: usize,
        state: Mutex<(usize, Option<bool>)>,
        changed: Condvar,
    }

    impl Meeting {
        fn new(quorum: usize) -> Meeting {
            Meeting {
                quorum,
                state: Mutex::new((0, None)),
                changed: Condvar::new(),
            }
        }

        fn attend(&self) -> bool {
            let mut state = self.state.lock().unwrap();
            state.0 += 1;
            if state.0 >= self.quorum && state.1.is_none() {
                state.1 = Some(true);
                self.changed.notify_all();
            }
            let (mut state, _) = self
                .changed
                .wait_timeout_while(state, Duration::from_secs(5), |s| s.1.is_none())
                .unwrap();
            if state.1.is_none() {
                state.1 = Some(false);
                self.changed.notify_all();
            }
            state.1 == Some(true)
        }

        fn met(&self) -> bool {
            self.state.lock().unwrap().1 == Some(true)
        }
    }

    /// A pool of four threads, so the tests do not depend on the machine's cores.
    fn four_threads() -> rayon::ThreadPool {
        rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap()
    }

    /// A pool whose panic handler records that it was called.
    fn pool_with_alarm(threads: usize) -> (Arc<rayon::ThreadPool>, Arc<AtomicBool>) {
        let alarm = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&alarm);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|index| format!("cpu-{index}"))
            .panic_handler(move |_| flag.store(true, Ordering::SeqCst))
            .build()
            .unwrap();
        (Arc::new(pool), alarm)
    }

    /// Polls `condition` until it holds, for at most five seconds.
    async fn wait_until(what: &str, condition: impl Fn() -> bool) {
        let poll = async {
            while !condition() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        };
        deadline(5, what, poll).await;
    }

    #[test]
    fn a_scores_keep_the_input_order() {
        crate::within(60, "the test `a_scores_keep_the_input_order`", move || {
            let words = ["pear", "fig", "banana", "kiwi", "apple"];
            let lengths = score_all(&words, |word| word.len() as u64);
            assert_eq!(lengths, [4, 3, 6, 4, 5]);
            assert!(score_all(&[] as &[u8], |_| 1).is_empty());
        })
    }

    #[test]
    fn a_scores_are_computed_in_parallel() {
        crate::within(
            60,
            "the test `a_scores_are_computed_in_parallel`",
            move || {
                let meeting = Meeting::new(3);
                let items: Vec<u64> = (0..16).collect();
                let scores = four_threads().install(|| {
                    score_all(&items, |item| {
                        meeting.attend();
                        item * 2
                    })
                });
                assert_eq!(scores, (0..16).map(|i| i * 2).collect::<Vec<u64>>());
                assert!(
                    meeting.met(),
                    "three calls of `score` never overlapped on a four-thread pool: \
             the work ran one item at a time"
                );
            },
        )
    }

    #[test]
    fn a_best_breaks_ties_by_lowest_index() {
        crate::within(
            60,
            "the test `a_best_breaks_ties_by_lowest_index`",
            move || {
                assert_eq!(best(&[] as &[u64], |v| *v), None);
                assert_eq!(best(&[3u64, 9, 4], |v| *v), Some((1, 9)));
                // Many equal maxima spread over the whole slice: every split of the work
                // must still give index 1.
                let mut values = vec![5u64; 10_000];
                values[0] = 1;
                let pool = four_threads();
                for _ in 0..20 {
                    assert_eq!(
                        pool.install(|| best(&values, |v| *v)),
                        Some((1, 5)),
                        "ties must go to the lowest index on every run"
                    );
                }
            },
        )
    }

    #[test]
    fn a_best_runs_in_parallel() {
        crate::within(60, "the test `a_best_runs_in_parallel`", move || {
            let meeting = Meeting::new(3);
            let items: Vec<u64> = (0..16).collect();
            let found = four_threads().install(|| {
                best(&items, |item| {
                    meeting.attend();
                    *item % 7
                })
            });
            assert_eq!(found, Some((6, 6)));
            assert!(meeting.met(), "`best` scored the items one at a time");
        })
    }

    #[test]
    fn b_every_row_is_edited_once() {
        crate::within(60, "the test `b_every_row_is_edited_once`", move || {
            let mut pixels: Vec<u8> = vec![0; 12];
            edit_rows(&mut pixels, 4, |index, row| {
                assert_eq!(row.len(), 4, "a row has `width` bytes");
                for pixel in row.iter_mut() {
                    *pixel += 10 * (index as u8 + 1);
                }
            });
            assert_eq!(pixels, [10, 10, 10, 10, 20, 20, 20, 20, 30, 30, 30, 30]);
            let mut empty: Vec<u8> = Vec::new();
            edit_rows(&mut empty, 3, |_, _| panic!("no rows, no calls"));
        })
    }

    #[test]
    fn b_rows_are_edited_in_parallel_and_borrow_local_data() {
        crate::within(
            60,
            "the test `b_rows_are_edited_in_parallel_and_borrow_local_data`",
            move || {
                let meeting = Meeting::new(3);
                // Local data that the tasks only borrow: no Arc, no clone, no 'static.
                let palette = vec![7u8, 8, 9];
                let calls = AtomicUsize::new(0);
                let mut pixels = vec![0u8; 8 * 3];
                four_threads().install(|| {
                    edit_rows(&mut pixels, 3, |_, row| {
                        meeting.attend();
                        calls.fetch_add(1, Ordering::SeqCst);
                        row.copy_from_slice(&palette);
                    });
                });
                assert_eq!(calls.load(Ordering::SeqCst), 8);
                assert!(pixels.chunks(3).all(|row| row == palette));
                assert!(
                    meeting.met(),
                    "three rows were never in work at the same time: spawn one task per row"
                );
            },
        )
    }

    #[test]
    fn c_pool_has_the_requested_named_threads() {
        crate::within(
            60,
            "the test `c_pool_has_the_requested_named_threads`",
            move || {
                let pool = cpu_pool(3);
                assert_eq!(pool.current_num_threads(), 3);
                let mut names: Vec<String> = pool.broadcast(|_| {
                    std::thread::current()
                        .name()
                        .unwrap_or("unnamed")
                        .to_owned()
                });
                names.sort();
                assert_eq!(names, ["cpu-0", "cpu-1", "cpu-2"]);

                // Parallel iterators started inside `install` use this pool, not the global one.
                let items: Vec<u32> = (0..64).collect();
                let outsiders = AtomicUsize::new(0);
                pool.install(|| {
                    score_all(&items, |_| {
                        let on_pool = std::thread::current()
                            .name()
                            .is_some_and(|name| name.starts_with("cpu-"));
                        if !on_pool {
                            outsiders.fetch_add(1, Ordering::SeqCst);
                        }
                        0
                    })
                });
                assert_eq!(outsiders.load(Ordering::SeqCst), 0);
            },
        )
    }

    #[tokio::test]
    async fn d_offload_runs_on_the_pool_and_returns_the_value() {
        crate::support::deadline(
            120,
            "the test `d_offload_runs_on_the_pool_and_returns_the_value`",
            async {
                let (pool, _alarm) = pool_with_alarm(2);
                let job = || {
                    let name = std::thread::current()
                        .name()
                        .unwrap_or("unnamed")
                        .to_owned();
                    (name, 6 * 7)
                };
                let (thread, value) = deadline(10, "offload", offload(&pool, job)).await.unwrap();
                assert_eq!(value, 42);
                assert!(
                    thread.starts_with("cpu-"),
                    "the job ran on thread {thread:?}, not on the given pool"
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn d_offload_leaves_the_async_thread_free() {
        crate::support::deadline(
            120,
            "the test `d_offload_leaves_the_async_thread_free`",
            async {
                // This test runs on a single-threaded runtime. The job reports that it has
                // started and then waits for an answer that only another async task on
                // the same runtime can send. If the job runs on the async thread, that
                // task never gets a turn.
                let (pool, _alarm) = pool_with_alarm(2);
                let (started_tx, mut started) = tokio::sync::mpsc::unbounded_channel::<()>();
                let (answer_tx, answer) = mpsc::channel::<&'static str>();
                let helper = tokio::spawn(async move {
                    if started.recv().await.is_some() {
                        let _ = answer_tx.send("the runtime is alive");
                    }
                });
                let job = move || {
                    let _ = started_tx.send(());
                    answer.recv_timeout(Duration::from_secs(5)).ok()
                };
                let heard = deadline(30, "offload", offload(&pool, job)).await;
                assert_eq!(
                    heard,
                    Ok(Some("the runtime is alive")),
                    "the async thread was blocked while the job ran: the job must run on \
             the pool and the handler must only `.await` its result"
                );
                helper.await.unwrap();
            },
        )
        .await
    }

    #[tokio::test]
    async fn d_a_panicking_job_is_reported_and_contained() {
        crate::support::deadline(
            120,
            "the test `d_a_panicking_job_is_reported_and_contained`",
            async {
                let (pool, alarm) = pool_with_alarm(1);
                let lost: Result<u8, JobLost> = deadline(
                    10,
                    "offload of a panicking job",
                    offload(&pool, || {
                        panic!("this job is corrupt (expected in this test)")
                    }),
                )
                .await;
                assert_eq!(lost, Err(JobLost));
                assert!(
                    !alarm.load(Ordering::SeqCst),
                    "the panic reached the pool's panic handler. Without a handler rayon \
             aborts the whole process: catch the panic inside the job"
                );
                let after = deadline(10, "the next job", offload(&pool, || "still working")).await;
                assert_eq!(
                    after,
                    Ok("still working"),
                    "the pool survives a panicking job"
                );
            },
        )
        .await
    }

    /// A job that reports when it starts and then waits until the test releases it.
    struct Held {
        started: tokio::sync::mpsc::UnboundedReceiver<()>,
        started_tx: tokio::sync::mpsc::UnboundedSender<()>,
        release: mpsc::Sender<()>,
        release_rx: Arc<Mutex<mpsc::Receiver<()>>>,
    }

    impl Held {
        fn new() -> Held {
            let (started_tx, started) = tokio::sync::mpsc::unbounded_channel();
            let (release, release_rx) = mpsc::channel();
            Held {
                started,
                started_tx,
                release,
                release_rx: Arc::new(Mutex::new(release_rx)),
            }
        }

        fn job(&self) -> impl FnOnce() -> bool + Send + 'static {
            let (started, release) = (self.started_tx.clone(), Arc::clone(&self.release_rx));
            move || {
                let _ = started.send(());
                let released = release
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10));
                released.is_ok()
            }
        }
    }

    #[tokio::test]
    async fn e_jobs_beyond_the_limit_are_refused_at_once() {
        crate::support::deadline(
            120,
            "the test `e_jobs_beyond_the_limit_are_refused_at_once`",
            async {
                let (pool, _alarm) = pool_with_alarm(1);
                let limiter = CpuLimiter::new(pool, 2);
                let mut held = Held::new();
                assert_eq!(limiter.pending(), 0);

                let running = tokio::spawn({
                    let (limiter, job) = (limiter.clone(), held.job());
                    async move { limiter.run(job).await }
                });
                deadline(10, "the first job starting", held.started.recv()).await;
                let queued = tokio::spawn({
                    let (limiter, job) = (limiter.clone(), held.job());
                    async move { limiter.run(job).await }
                });
                wait_until("two pending jobs", || limiter.pending() == 2).await;

                let refused = deadline(5, "the third job", limiter.run(|| "never runs")).await;
                assert_eq!(
                    refused,
                    Err(CpuError::Overloaded),
                    "one running and one queued is the limit; the third must be refused, not queued"
                );
                assert_eq!(limiter.pending(), 2, "a refused job takes no slot");

                held.release.send(()).unwrap();
                held.release.send(()).unwrap();
                assert_eq!(deadline(15, "job one", running).await.unwrap(), Ok(true));
                assert_eq!(deadline(15, "job two", queued).await.unwrap(), Ok(true));
                wait_until("all slots free again", || limiter.pending() == 0).await;
                assert_eq!(limiter.run(|| 7).await, Ok(7), "capacity is back");
            },
        )
        .await
    }

    #[tokio::test]
    async fn e_a_cancelled_caller_does_not_free_the_cpu() {
        crate::support::deadline(
            120,
            "the test `e_a_cancelled_caller_does_not_free_the_cpu`",
            async {
                let (pool, _alarm) = pool_with_alarm(1);
                let limiter = CpuLimiter::new(pool, 1);
                let mut held = Held::new();

                let caller = tokio::spawn({
                    let (limiter, job) = (limiter.clone(), held.job());
                    async move { limiter.run(job).await }
                });
                deadline(10, "the job starting", held.started.recv()).await;

                // The client hangs up. The job is still running on the pool.
                caller.abort();
                assert!(caller.await.unwrap_err().is_cancelled());
                assert_eq!(
                    limiter.run(|| "too early").await,
                    Err(CpuError::Overloaded),
                    "the caller is gone but its job still occupies the only CPU thread; \
             the slot must stay taken until the job itself ends"
                );

                held.release.send(()).unwrap();
                wait_until("the abandoned job finishing", || limiter.pending() == 0).await;
                assert_eq!(limiter.run(|| "now").await, Ok("now"));
            },
        )
        .await
    }

    #[tokio::test]
    async fn e_a_panicking_job_frees_its_slot() {
        crate::support::deadline(120, "the test `e_a_panicking_job_frees_its_slot`", async {
            let (pool, alarm) = pool_with_alarm(1);
            let limiter = CpuLimiter::new(pool, 1);
            let lost: Result<(), CpuError> = deadline(
                10,
                "a panicking job",
                limiter.run(|| panic!("this job is corrupt (expected in this test)")),
            )
            .await;
            assert_eq!(lost, Err(CpuError::JobLost));
            assert!(
                !alarm.load(Ordering::SeqCst),
                "catch the panic inside the job"
            );
            wait_until("the slot of the panicked job", || limiter.pending() == 0).await;
            assert_eq!(limiter.run(|| 1).await, Ok(1));
        })
        .await
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[tokio::test]
    async fn p_predictions() {
        crate::support::deadline(120, "the test `p_predictions`", async {
            // P1. `rayon::current_num_threads()` asks "how many threads has the pool I
            // am running in?". What does it return inside `install` on this pool?
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(3)
                .build()
                .unwrap();
            let p1: usize = 3;
            assert_eq!(p1, pool.install(rayon::current_num_threads), "P1: install");

            // P2. The items are processed on several threads in an unknown order.
            // What does `collect` produce?
            let p2: Vec<u32> = vec![30, 10, 20];
            let collected: Vec<u32> =
                pool.install(|| [3u32, 1, 2].par_iter().map(|n| n * 10).collect());
            assert_eq!(p2, collected, "P2: collect on an indexed parallel iterator");

            // P3. Is the thread that runs this test a worker of some rayon pool?
            let p3: bool = false;
            assert_eq!(
                p3,
                rayon::current_thread_index().is_some(),
                "P3: the test thread and rayon"
            );

            // P4. This test runs on a single-threaded tokio runtime. A task is spawned;
            // then this function computes for a while without `.await`. Has the task
            // run (a) before the first `.await` below, (b) after it?
            let ran = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&ran);
            let task = tokio::spawn(async move { flag.store(true, Ordering::SeqCst) });
            let mut busy = 0u64;
            for step in 0..100_000u64 {
                busy = std::hint::black_box(busy.wrapping_add(step));
            }
            let before_await = ran.load(Ordering::SeqCst);
            task.await.unwrap();
            let after_await = ran.load(Ordering::SeqCst);
            let p4: (bool, bool) = (false, true);
            assert_eq!(
                p4,
                (before_await, after_await),
                "P4: other tasks run only when this one yields at an await"
            );
        })
        .await
    }
}
