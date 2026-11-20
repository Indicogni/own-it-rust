//! LESSON 08 — Capstone: bounded parallel pipeline
//! Run: cargo test --test chapter19 l08_
//!
//! PREDICT before running:
//! 1. Can joining a producer before draining its full bounded channel deadlock?
//! 2. Why is output order from multiple workers unspecified?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Trace ownership and closure of every sender. Explain when sorted output is a contract versus hiding a bug.
//! COMPILER LAB (examples/ch19_scratch.rs): Extend to two stages and inject a worker Result error. Use completion channels, bounded recv_timeout, and joined handles; never use sleep as coordination.

use std::{sync::mpsc, thread};
/// A. Split `input` into chunks of `chunk_size` and spawn one producer per
/// chunk. A producer calls `work` on each element of its chunk and sends
/// (original_index, result) through a sync_channel with capacity 2. Drain the
/// channel on the calling thread while the producers run, join them all, then
/// sort by index and return the results in input order. Empty input is valid.
/// Use thread::scope to borrow `input` and `work`. chunk_size must be nonzero;
/// like `slice::chunks`, panic if it is 0.
fn bounded_pipeline(
    input: &[i32],
    chunk_size: usize,
    work: impl Fn(i32) -> i64 + Sync,
) -> Vec<i64> {
    todo!("A: implement the contract")
}
/// B. Return received items until channel closure, propagating the first
/// application error as soon as it arrives (do not wait for closure after an error). Dropping the receiver is the caller's cancellation signal;
/// producers must handle send errors instead of unwrap when cancellation is possible.
fn collect_results<T, E>(input: mpsc::Receiver<Result<T, E>>) -> Result<Vec<T>, E> {
    todo!("B: implement the contract")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;
    use std::{
        collections::HashSet,
        panic::{AssertUnwindSafe, catch_unwind},
        sync::{Barrier, Mutex},
        thread::ThreadId,
        time::Duration,
    };
    fn square(value: i32) -> i64 {
        i64::from(value) * i64::from(value)
    }
    /// Runs the pipeline with a time limit, so a deadlock fails the test instead of hanging it.
    fn squares_within(input: Vec<i32>, chunk_size: usize, secs: u64) -> Vec<i64> {
        within(
            secs,
            "bounded_pipeline (drain the capacity-2 channel while the producers \
             run, drop the original sender, then join)",
            move || bounded_pipeline(&input, chunk_size, square),
        )
    }
    #[test]
    fn a_bounded_pipeline_retains_input_order() {
        assert_eq!(
            squares_within(vec![3, -2, 0, i32::MIN, 7], 2, 10),
            [9, 4, 0, 4611686018427387904, 49]
        );
        assert_eq!(squares_within(vec![], 3, 10), []);
        assert_eq!(squares_within(vec![8], 9, 10), [64]);
    }
    #[test]
    fn a_many_producers_drain_without_deadlock() {
        let input: Vec<i32> = (-1500..1500).collect();
        let expected: Vec<i64> = input.iter().map(|&x| square(x)).collect();
        assert_eq!(squares_within(input, 7, 20), expected);
    }
    #[test]
    fn a_one_producer_thread_per_chunk() {
        // Chunks of 4 over 0..10 are [0, 1, 2, 3], [4, 5, 6, 7] and [8, 9].
        // The first element of each chunk waits until three producers have
        // arrived, so the producers must be running at the same time.
        let (caller, results, seen) = within(
            10,
            "bounded_pipeline (three producers wait for each other inside `work`, \
             so spawn one producer per chunk before you join any of them)",
            || {
                let gate = Barrier::new(3);
                let seen: Mutex<Vec<(i32, ThreadId)>> = Mutex::new(Vec::new());
                let input: Vec<i32> = (0..10).collect();
                let results = bounded_pipeline(&input, 4, |value| {
                    if value % 4 == 0 {
                        gate.wait();
                    }
                    seen.lock().unwrap().push((value, thread::current().id()));
                    i64::from(value) + 100
                });
                (thread::current().id(), results, seen.into_inner().unwrap())
            },
        );
        assert_eq!(results, (100..110).collect::<Vec<i64>>());
        assert_eq!(seen.len(), 10, "call `work` exactly once per element");
        assert!(
            seen.iter().all(|(_, thread)| *thread != caller),
            "`work` ran on the calling thread: producers do the work, the caller only collects"
        );
        let thread_of = |value: i32| seen.iter().find(|(v, _)| *v == value).unwrap().1;
        let producers: HashSet<ThreadId> = [0, 4, 8].map(thread_of).into_iter().collect();
        assert_eq!(
            producers.len(),
            3,
            "three chunks must be handled by three different producer threads"
        );
        for (value, thread) in &seen {
            assert_eq!(
                *thread,
                thread_of(value - value % 4),
                "element {value} was not handled by the producer that owns its chunk"
            );
        }
    }
    #[test]
    fn a_zero_chunk_size_panics() {
        let outcome = within(
            10,
            "bounded_pipeline with chunk_size 0 (it must panic, not loop)",
            || {
                catch_unwind(AssertUnwindSafe(|| bounded_pipeline(&[1, 2], 0, square))).map_err(
                    // A panic message is a `&str` or a `String`, depending on
                    // how the panic was raised.
                    |payload| match payload.downcast_ref::<&str>() {
                        Some(message) => message.to_string(),
                        None => payload
                            .downcast_ref::<String>()
                            .cloned()
                            .unwrap_or_default(),
                    },
                )
            },
        );
        let message = outcome
            .expect_err("chunk_size 0 must panic, as `slice::chunks` does; it returned a value");
        assert!(
            !message.contains("not yet implemented"),
            "task A is not implemented yet"
        );
    }
    #[test]
    fn b_application_failure_stops_collection() {
        let (tx, rx) = mpsc::channel();
        tx.send(Ok::<_, &str>(1)).unwrap();
        tx.send(Err("bad row")).unwrap();
        drop(tx);
        assert_eq!(collect_results(rx), Err("bad row"));
        let (tx, rx) = mpsc::channel();
        tx.send(Ok::<_, &str>(2)).unwrap();
        drop(tx);
        assert_eq!(collect_results(rx), Ok(vec![2]));
        let (tx, rx) = mpsc::channel::<Result<u8, &str>>();
        drop(tx);
        assert_eq!(collect_results(rx), Ok(vec![]));
    }
    #[test]
    fn b_first_error_returns_without_waiting_for_closure() {
        let (tx, rx) = mpsc::channel();
        tx.send(Ok::<u8, &str>(1)).unwrap();
        tx.send(Err("first")).unwrap();
        tx.send(Err("second")).unwrap();
        tx.send(Ok(3)).unwrap();
        // `tx` stays alive, like a producer that is still running.
        let (done_tx, done_rx) = mpsc::channel();
        let collector = thread::spawn(move || done_tx.send(collect_results(rx)).unwrap());
        let got = done_rx.recv_timeout(Duration::from_secs(5));
        drop(tx);
        collector.join().unwrap();
        assert_eq!(
            got,
            Ok(Err("first")),
            "return the first error immediately; dropping the receiver is the cancellation"
        );
    }
}
