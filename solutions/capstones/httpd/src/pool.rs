//! A bounded pool of worker threads (SPEC section 10).
//!
//! The pool is generic over the item it hands to workers; it knows nothing
//! about sockets. What it guarantees is a number: at most `workers + queue`
//! items are inside it at any moment, counted from a successful
//! [`Pool::try_submit`] until the handler for that item has returned. A
//! submission above that number is refused at once and the item is handed
//! back, so the caller decides what "full" means for it.

use std::collections::VecDeque;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

pub struct Pool<T> {
    shared: Arc<Shared<T>>,
    workers: Vec<JoinHandle<()>>,
}

struct Shared<T> {
    state: Mutex<State<T>>,
    /// Signalled when an item is queued and when the pool closes.
    wake: Condvar,
    /// `workers + queue`.
    capacity: usize,
}

struct State<T> {
    waiting: VecDeque<T>,
    /// Items waiting plus items a worker is busy with.
    admitted: usize,
    closed: bool,
}

impl<T> Shared<T> {
    /// The handler runs outside the lock, so a panic can only poison the
    /// mutex between two plain assignments to `State`. The data is
    /// consistent either way and the pool must keep working.
    fn lock(&self) -> MutexGuard<'_, State<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<T: Send + 'static> Pool<T> {
    /// Starts `workers` threads that each call `handler` with one item at a
    /// time. Up to `queue` more items may wait.
    pub fn new(
        workers: usize,
        queue: usize,
        handler: impl Fn(T) + Send + Sync + 'static,
    ) -> std::io::Result<Self> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                waiting: VecDeque::with_capacity(queue),
                admitted: 0,
                closed: false,
            }),
            wake: Condvar::new(),
            capacity: workers + queue,
        });
        let handler = Arc::new(handler);
        let workers = (0..workers)
            .map(|index| {
                let shared = Arc::clone(&shared);
                let handler = Arc::clone(&handler);
                thread::Builder::new()
                    .name(format!("worker-{index}"))
                    .spawn(move || work(&shared, &*handler))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { shared, workers })
    }

    /// Admits `item` if there is room and returns it otherwise. Never
    /// blocks longer than the short critical section.
    pub fn try_submit(&self, item: T) -> Result<(), T> {
        let mut state = self.shared.lock();
        if state.closed || state.admitted == self.shared.capacity {
            return Err(item);
        }
        state.admitted += 1;
        state.waiting.push_back(item);
        drop(state);
        self.shared.wake.notify_one();
        Ok(())
    }

    /// Stops admission and returns the items no worker has started on.
    /// Workers finish the item they have and then exit.
    pub fn close(&self) -> Vec<T> {
        let mut state = self.shared.lock();
        state.closed = true;
        let waiting: Vec<T> = state.waiting.drain(..).collect();
        state.admitted -= waiting.len();
        drop(state);
        self.shared.wake.notify_all();
        waiting
    }

    /// Waits until every worker has exited. Call [`Pool::close`] first.
    pub fn join(self) {
        for worker in self.workers {
            // A worker cannot panic: `work` catches the handler's panics.
            let _ = worker.join();
        }
    }
}

/// The loop of one worker thread.
fn work<T>(shared: &Shared<T>, handler: &(impl Fn(T) + ?Sized)) {
    loop {
        let item = {
            let mut state = shared.lock();
            loop {
                if let Some(item) = state.waiting.pop_front() {
                    break item;
                }
                if state.closed {
                    return;
                }
                state = shared
                    .wake
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        // A panic in the handler must cost one item, not one worker: a pool
        // that loses a thread per bug shrinks to nothing under bad input.
        // `AssertUnwindSafe` is honest here because the item is consumed by
        // the call and nothing else is shared with the handler by this
        // function. The default panic hook has already printed the message.
        let _ = panic::catch_unwind(AssertUnwindSafe(|| handler(item)));
        shared.lock().admitted -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    const LONG: Duration = Duration::from_secs(10);

    /// A handler that reports each item and then waits for permission to
    /// finish, so a test can hold workers busy for exactly as long as it
    /// wants.
    fn gated_pool(
        workers: usize,
        queue: usize,
    ) -> (Pool<u32>, mpsc::Receiver<u32>, mpsc::Sender<()>) {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let release_rx = Mutex::new(release_rx);
        let pool = Pool::new(workers, queue, move |item| {
            started_tx.send(item).expect("the test is listening");
            // Each `()` releases one handler.
            let _ = release_rx.lock().expect("no panics here").recv();
        })
        .expect("threads start");
        (pool, started_rx, release_tx)
    }

    #[test]
    fn admits_workers_plus_queue_and_refuses_the_next() {
        let (pool, started, release) = gated_pool(2, 1);
        assert_eq!(pool.try_submit(1), Ok(()));
        assert_eq!(pool.try_submit(2), Ok(()));
        // Both workers are busy before the queue is tested.
        let mut busy = [
            started.recv_timeout(LONG).unwrap(),
            started.recv_timeout(LONG).unwrap(),
        ];
        busy.sort();
        assert_eq!(busy, [1, 2]);
        assert_eq!(pool.try_submit(3), Ok(()), "one item may wait");
        assert_eq!(
            pool.try_submit(4),
            Err(4),
            "the pool is full; the item comes back"
        );

        // One handler finishes: the waiting item starts, and only then is
        // there room for another.
        release.send(()).unwrap();
        assert_eq!(started.recv_timeout(LONG).unwrap(), 3);
        assert_eq!(pool.try_submit(5), Ok(()));
        assert_eq!(pool.try_submit(6), Err(6));

        for _ in 0..4 {
            release.send(()).unwrap();
        }
        assert_eq!(started.recv_timeout(LONG).unwrap(), 5);
        assert!(pool.close().is_empty());
        pool.join();
    }

    #[test]
    fn a_queue_of_zero_still_admits_one_item_per_worker() {
        let (pool, started, release) = gated_pool(1, 0);
        assert_eq!(pool.try_submit(1), Ok(()));
        assert_eq!(started.recv_timeout(LONG).unwrap(), 1);
        assert_eq!(pool.try_submit(2), Err(2));
        release.send(()).unwrap();
        pool.close();
        pool.join();
    }

    #[test]
    fn close_returns_what_no_worker_started() {
        let (pool, started, release) = gated_pool(1, 3);
        for item in 1..=4 {
            assert_eq!(pool.try_submit(item), Ok(()));
        }
        assert_eq!(started.recv_timeout(LONG).unwrap(), 1);
        assert_eq!(pool.close(), [2, 3, 4]);
        assert_eq!(pool.try_submit(9), Err(9), "a closed pool admits nothing");
        release.send(()).unwrap();
        pool.join();
        assert!(
            started.try_recv().is_err(),
            "returned items were never handled"
        );
    }

    #[test]
    fn a_panicking_handler_costs_neither_a_worker_nor_a_slot() {
        let handled = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&handled);
        let (done_tx, done_rx) = mpsc::channel();
        let pool = Pool::new(1, 8, move |item: u32| {
            counter.fetch_add(1, Ordering::SeqCst);
            if item.is_multiple_of(2) {
                // Keep the expected panics out of the test output.
                panic::resume_unwind(Box::new("even items fail"));
            }
            done_tx.send(item).expect("the test is listening");
        })
        .expect("threads start");

        for round in 0..20 {
            for item in [2, 4, 1] {
                assert_eq!(
                    pool.try_submit(item),
                    Ok(()),
                    "round {round}: a slot leaked"
                );
            }
            assert_eq!(
                done_rx.recv_timeout(LONG),
                Ok(1),
                "round {round}: the worker died"
            );
        }
        pool.close();
        pool.join();
        assert_eq!(handled.load(Ordering::SeqCst), 60);
    }
}
