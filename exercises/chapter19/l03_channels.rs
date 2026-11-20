//! LESSON 03 — Ownership and channel closure
//! Run: cargo test --test chapter19 l03_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. When does recv report disconnection?
//! 2. Does send guarantee the receiver processed the message?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Explain FIFO per sender, nondeterministic ordering across senders, and bounded-channel backpressure.
//! COMPILER LAB (examples/ch19_scratch.rs): Clone a sender and drop the original. Use try_recv to distinguish Empty from Disconnected, then drop the clone.

use std::{sync::mpsc, thread};
/// A. Spawn one worker. Send the Strings to it through an unbounded channel;
/// the worker calls `work` on each String it receives and collects the
/// results until the channel is closed. Close the channel (drop the sender)
/// before you join the worker. Return the results in input order.
fn channel_worker(
    items: Vec<String>,
    work: impl Fn(String) -> String + Send + 'static,
) -> Vec<String> {
    todo!("A: implement the contract")
}
/// B. Drain currently available messages without blocking. Return messages and
/// whether the channel is disconnected. Empty != disconnected.
fn drain_now<T>(input: &mpsc::Receiver<T>) -> (Vec<T>, bool) {
    todo!("B: implement the contract")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{guess, within};
    use std::{
        sync::{
            Arc, Mutex,
            mpsc::{TryRecvError, TrySendError},
        },
        thread::ThreadId,
    };
    /// Runs task A with a time limit, so a worker that never sees the channel
    /// close fails the test instead of blocking it forever.
    fn uppercased(items: Vec<String>) -> Vec<String> {
        within(
            10,
            "channel_worker (the worker's receive loop ends only when the sender is \
             dropped, so drop it before you join)",
            move || channel_worker(items, |s| s.to_uppercase()),
        )
    }
    #[test]
    fn a_ordered_worker_terminates_on_input_close() {
        assert_eq!(uppercased(vec!["one".into(), "ß".into()]), ["ONE", "SS"]);
        assert!(uppercased(vec![]).is_empty());
        let many: Vec<String> = (0..200).map(|i| format!("item{i}")).collect();
        let expected: Vec<String> = (0..200).map(|i| format!("ITEM{i}")).collect();
        assert_eq!(uppercased(many), expected);
    }
    #[test]
    fn a_all_work_happens_on_one_worker_thread() {
        let seen: Arc<Mutex<Vec<ThreadId>>> = Arc::default();
        let log = Arc::clone(&seen);
        let (caller, out) = within(
            10,
            "channel_worker (drop the sender before you join the worker)",
            move || {
                let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
                let out = channel_worker(items, move |s| {
                    log.lock().unwrap().push(thread::current().id());
                    s + "!"
                });
                (thread::current().id(), out)
            },
        );
        assert_eq!(out, ["a!", "b!", "c!"]);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 3, "call `work` exactly once per item");
        assert!(
            !seen.contains(&caller),
            "`work` ran on the calling thread: spawn a worker and send it the items"
        );
        assert!(
            seen.iter().all(|thread| *thread == seen[0]),
            "the items were processed on several threads: this task has exactly one worker"
        );
    }
    #[test]
    fn b_empty_and_disconnected_are_different() {
        // The sender stays alive during the first two calls. A blocking `recv`,
        // or a `for` loop over the receiver, would wait for it forever.
        let (first, second, third) = within(
            10,
            "drain_now (use try_recv; recv and the receiver's iterator block while \
             a sender is alive)",
            || {
                let (tx, rx) = mpsc::channel();
                tx.send(1).unwrap();
                tx.send(2).unwrap();
                let first = drain_now(&rx);
                let second = drain_now(&rx);
                drop(tx);
                (first, second, drain_now(&rx))
            },
        );
        assert_eq!(first, (vec![1, 2], false));
        assert_eq!(second, (vec![], false), "empty, but a sender is alive");
        assert_eq!(third, (vec![], true), "every sender is gone");
    }
    #[test]
    fn b_buffered_messages_arrive_before_disconnection() {
        let (tx, rx) = mpsc::channel();
        tx.send("last words").unwrap();
        drop(tx);
        assert_eq!(drain_now(&rx), (vec!["last words"], true));
        assert_eq!(drain_now(&rx), (vec![], true));
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. The only sender sends one value and is dropped. Two recv calls follow.
        let (tx, rx) = mpsc::channel();
        tx.send(5).unwrap();
        drop(tx);
        let first = rx.recv().ok();
        let second_failed = rx.recv().is_err();
        let p1: (Option<i32>, bool) = guess();
        assert_eq!(
            p1,
            (first, second_failed),
            "P1: buffered values outlive senders"
        );

        // P2. A sync_channel of capacity 1 with nobody receiving. Does the first
        // try_send succeed? Is the second one refused as Full?
        let (tx, rx) = mpsc::sync_channel(1);
        let first_ok = tx.try_send('a').is_ok();
        let second_full = matches!(tx.try_send('b'), Err(TrySendError::Full('b')));
        let p2: (bool, bool) = guess();
        assert_eq!(p2, (first_ok, second_full), "P2: bounded capacity");

        // P3. The receiver is gone. What does send hand back?
        let (tx, rx) = mpsc::channel::<String>();
        drop(rx);
        let returned = tx.send("lost?".to_string()).err().map(|e| e.0);
        let p3: Option<String> = guess();
        assert_eq!(p3, returned, "P3: a failed send returns the value");

        // P4. A clone of the sender is still alive after the original is dropped.
        let (tx, rx) = mpsc::channel::<u8>();
        let tx2 = tx.clone();
        drop(tx);
        let p4: bool = guess();
        assert_eq!(
            p4,
            rx.try_recv() == Err(TryRecvError::Disconnected),
            "P4: is the channel disconnected?"
        );
        drop(tx2);
    }
}
