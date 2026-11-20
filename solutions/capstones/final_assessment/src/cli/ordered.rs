//! Work done by several threads, results delivered in the order of the
//! work items.
//!
//! This is the only place in the program where threads meet. It knows
//! nothing about files or patterns: it maps a slice of items to results and
//! hands the results to one consumer, in slice order, whichever thread
//! finished first.
//!
//! Three facts make the output deterministic and the memory bounded:
//!
//! - A result waits in a map until every result before it has been consumed.
//! - A worker may only take item `i` while `i < consumed + window`. A slow
//!   first item therefore holds back the workers after at most `window`
//!   items; finished results cannot pile up without limit.
//! - Only the calling thread runs `consume`, so the consumer needs no lock.

use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread;

/// How many items per thread may be taken ahead of the consumer.
const WINDOW_PER_THREAD: usize = 4;

/// Runs `work` on every item, on up to `threads` threads, and passes the
/// results to `consume` in the order of `items`.
///
/// Each thread calls `make_state` once and keeps the value for all its
/// items; this is where a worker gets its scratch memory. `consume` can end
/// the run early with [`ControlFlow::Break`]; items that no thread has
/// started are then never started.
///
/// If `work` panics, the panic is passed on to the caller after the other
/// threads have stopped.
pub fn for_each_ordered<T, S, R>(
    items: &[T],
    threads: usize,
    make_state: impl Fn() -> S + Sync,
    work: impl Fn(&mut S, &T) -> R + Sync,
    mut consume: impl FnMut(R) -> ControlFlow<()>,
) where
    T: Sync,
    R: Send,
{
    let threads = threads.min(items.len());
    if threads <= 1 {
        // Nothing to coordinate. This path is also the plain statement of
        // what the parallel path below must be equal to.
        let mut state = make_state();
        for item in items {
            if consume(work(&mut state, item)).is_break() {
                return;
            }
        }
        return;
    }

    let gate = Gate::new(items.len(), threads * WINDOW_PER_THREAD);
    let (sender, receiver) = mpsc::channel::<(usize, R)>();
    thread::scope(|scope| {
        for _ in 0..threads {
            let sender = sender.clone();
            let (gate, make_state, work) = (&gate, &make_state, &work);
            scope.spawn(move || {
                let _stop_others_if_this_panics = StopOnPanic(gate);
                let mut state = make_state();
                while let Some(index) = gate.take() {
                    let result = work(&mut state, &items[index]);
                    if sender.send((index, result)).is_err() {
                        break;
                    }
                }
            });
        }
        // The workers hold the only senders now: when they are all gone,
        // `recv` reports it instead of waiting for ever.
        drop(sender);

        let mut waiting = BTreeMap::new();
        let mut consumed = 0;
        'run: while consumed < items.len() {
            let Ok((index, result)) = receiver.recv() else {
                // Every worker has ended before the work was done: one of
                // them panicked. The scope passes the panic on.
                break;
            };
            waiting.insert(index, result);
            while let Some(result) = waiting.remove(&consumed) {
                if consume(result).is_break() {
                    break 'run;
                }
                consumed += 1;
                gate.set_consumed(consumed);
            }
        }
        // Normal end, early end or panic: no worker may wait at the gate.
        gate.stop();
    });
}

/// Decides which item a worker takes next, and whether it has to wait.
struct Gate {
    state: Mutex<GateState>,
    changed: Condvar,
    len: usize,
    window: usize,
}

struct GateState {
    /// The first item that no worker has taken.
    next: usize,
    /// How many results the consumer has finished with.
    consumed: usize,
    stopped: bool,
}

impl Gate {
    fn new(len: usize, window: usize) -> Gate {
        Gate {
            state: Mutex::new(GateState {
                next: 0,
                consumed: 0,
                stopped: false,
            }),
            changed: Condvar::new(),
            len,
            window,
        }
    }

    /// The critical sections below cannot panic, so a poisoned lock still
    /// guards consistent data. Going on is right, and it lets the other
    /// threads see `stopped` after a worker's panic.
    fn lock(&self) -> MutexGuard<'_, GateState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The index of the next item for the calling worker, or `None` when
    /// there is nothing left to do. Blocks while the consumer is more than
    /// a window behind.
    fn take(&self) -> Option<usize> {
        let mut state = self.lock();
        loop {
            if state.stopped || state.next >= self.len {
                return None;
            }
            if state.next < state.consumed + self.window {
                state.next += 1;
                return Some(state.next - 1);
            }
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    fn set_consumed(&self, consumed: usize) {
        self.lock().consumed = consumed;
        self.changed.notify_all();
    }

    fn stop(&self) {
        self.lock().stopped = true;
        self.changed.notify_all();
    }
}

/// Stops the run when the thread that owns it unwinds, so that the other
/// workers do not wait at the gate for a consumer that will never advance.
struct StopOnPanic<'a>(&'a Gate);

impl Drop for StopOnPanic<'_> {
    fn drop(&mut self) {
        if thread::panicking() {
            self.0.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn squares(len: usize, threads: usize) -> Vec<usize> {
        let items: Vec<usize> = (0..len).collect();
        let mut results = Vec::new();
        for_each_ordered(
            &items,
            threads,
            || (),
            |(), &n| {
                // The first items take the longest: they finish last.
                thread::sleep(Duration::from_micros(300_u64.saturating_sub(n as u64 * 10)));
                n * n
            },
            |square| {
                results.push(square);
                ControlFlow::Continue(())
            },
        );
        results
    }

    #[test]
    fn results_arrive_in_item_order() {
        let expected: Vec<usize> = (0..200).map(|n| n * n).collect();
        for threads in [1, 2, 3, 8, 64] {
            assert_eq!(squares(200, threads), expected, "with {threads} threads");
        }
        assert_eq!(squares(0, 4), []);
        assert_eq!(squares(1, 4), [0]);
    }

    #[test]
    fn workers_never_run_more_than_a_window_ahead() {
        let threads = 4;
        let items: Vec<usize> = (0..400).collect();
        let started = AtomicUsize::new(0);
        let mut consumed = 0;
        for_each_ordered(
            &items,
            threads,
            || (),
            |(), _| {
                started.fetch_add(1, Ordering::SeqCst);
            },
            |()| {
                // Give the workers time to run ahead if anything let them.
                thread::sleep(Duration::from_micros(200));
                consumed += 1;
                let ahead = started.load(Ordering::SeqCst) - consumed;
                assert!(
                    ahead <= threads * WINDOW_PER_THREAD,
                    "{ahead} items were started ahead of the consumer"
                );
                ControlFlow::Continue(())
            },
        );
        assert_eq!(consumed, 400);
    }

    #[test]
    fn break_stops_the_run() {
        let items: Vec<usize> = (0..10_000).collect();
        let started = AtomicUsize::new(0);
        let mut seen = Vec::new();
        for_each_ordered(
            &items,
            4,
            || (),
            |(), &n| {
                started.fetch_add(1, Ordering::SeqCst);
                n
            },
            |n| {
                seen.push(n);
                if n == 5 {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            },
        );
        assert_eq!(seen, [0, 1, 2, 3, 4, 5]);
        assert!(
            started.load(Ordering::SeqCst) < 100,
            "the workers went on long after the break"
        );
    }

    #[test]
    fn each_thread_gets_its_own_state() {
        let items = vec![(); 300];
        let states = AtomicUsize::new(0);
        for_each_ordered(
            &items,
            3,
            || states.fetch_add(1, Ordering::SeqCst),
            |_, ()| thread::sleep(Duration::from_micros(50)),
            |()| ControlFlow::Continue(()),
        );
        assert_eq!(states.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn a_panic_in_a_worker_reaches_the_caller() {
        let items: Vec<usize> = (0..500).collect();
        let caught = std::panic::catch_unwind(|| {
            for_each_ordered(
                &items,
                4,
                || (),
                |(), &n| assert!(n != 37, "item 37 is broken"),
                |()| ControlFlow::Continue(()),
            );
        });
        assert!(caught.is_err(), "the panic of a worker was swallowed");
    }
}
