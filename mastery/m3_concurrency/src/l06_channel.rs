//! LESSON 06 — A bounded channel with `Mutex` and `Condvar`
//! Run: cargo test l06_
//! Model: cargo model l06_
//! Read first: "Lesson 06" in CONCEPTS.md (same package).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A sender is woken because one slot became free. Why must it check the
//!    queue again before it pushes?
//! 2. The receiver is dropped while three senders wait for space. How many of
//!    them must be woken, and by which call?
//!
//! IMPLEMENT the contract below. You design the shared state. Use `Arc`,
//! `Mutex` and `Condvar` from `crate::sync`. This lesson needs no `unsafe`
//! and no atomics. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Write the predicate of each of your waits (the condition
//! under which the thread must keep sleeping). For each predicate, list every
//! place in your code that can make it false, and show that each such place
//! notifies the right condition variable.
//!
//! MODEL LAB (optional): Change the `while` (or `loop`) around the wait in
//! `send` into an `if` and run `cargo model l06_`. Loom does not produce
//! spurious wake-ups, and still finds a failure. Which thread "stole" the
//! free slot?

use crate::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::collections::VecDeque;
use std::marker::PhantomData;

// The imports above are the ones the reference solution uses. Remove what
// your design does not need.

// CONTRACT
//
// `channel(capacity)` returns the two ends of a queue that holds at most
// `capacity` messages. There can be many senders (`Sender` is `Clone`) and
// one receiver.
//
// 1. `channel(0)` panics with a message that contains "at least one message":
//    a channel without a buffer is out of scope here.
// 2. Messages arrive in the order in which the `send` calls completed.
// 3. `send` puts the message into the queue. While the queue is full it
//    **blocks** (on a `Condvar`, without spinning). If the receiver is gone,
//    it returns `Err(SendError(value))`, also when the receiver goes away
//    while the call is blocked. The value is always given back, never lost.
// 4. `try_send` never blocks: `Err(TrySendError::Full(value))` when the queue
//    is full, `Err(TrySendError::Closed(value))` when the receiver is gone.
// 5. `recv` takes the oldest message. While the queue is empty and at least
//    one sender exists, it blocks. When the queue is empty and every sender
//    is gone, it returns `Err(RecvError)`. Messages that were sent before the
//    last sender was dropped are still delivered first.
// 6. Dropping the receiver drops every message that is still in the queue,
//    even if senders still exist.
// 7. After the channel has been created, `send`, `recv` and `Sender::clone`
//    make no heap allocation: reserve the buffer once, in `channel`.
// 8. No lost wake-up and no deadlock, for every interleaving. The model tests
//    check this with one and with two senders.
// 9. Both ends can move to another thread when `T: Send`, and a `Sender` can
//    be shared between threads. The compiler derives this from `Arc`, `Mutex`
//    and `Condvar`; do not add a field that takes it away.

/// `send` returns this, with the message, when the receiver is gone.
#[derive(Debug, PartialEq, Eq)]
pub struct SendError<T>(pub T);

/// Why `try_send` did not queue the message. The message is handed back.
#[derive(Debug, PartialEq, Eq)]
pub enum TrySendError<T> {
    /// The queue holds `capacity` messages.
    Full(T),
    /// The receiver is gone.
    Closed(T),
}

/// `recv` returns this when the queue is empty and every sender is gone.
#[derive(Debug, PartialEq, Eq)]
pub struct RecvError;

/// The sending end. Clone it to send from several threads.
///
/// Replace the placeholder field with your own, and write the `Drop` impl.
pub struct Sender<T> {
    _replace_me: PhantomData<T>,
}

/// The receiving end.
///
/// Replace the placeholder field with your own, and write the `Drop` impl.
pub struct Receiver<T> {
    _replace_me: PhantomData<T>,
}

/// Creates a channel that holds at most `capacity` messages.
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    todo!("Implement this task")
}

impl<T> Sender<T> {
    /// Queues `value`, waiting for space if necessary.
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        todo!("Implement this task")
    }

    /// Queues `value` if there is space right now.
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        todo!("Implement this task")
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        todo!("Implement this task")
    }
}

impl<T> Receiver<T> {
    /// Takes the oldest message, waiting for one if necessary.
    pub fn recv(&self) -> Result<T, RecvError> {
        todo!("Implement this task")
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use crate::implements;
    use crate::support::{DropCount, alloc::count, guess, within};
    use std::rc::Rc;

    #[test]
    fn a_messages_arrive_in_order() {
        within(10, "sending and receiving on one thread", || {
            let (tx, rx) = channel(3);
            for n in 1..=3 {
                tx.send(n).unwrap();
            }
            assert_eq!(rx.recv(), Ok(1));
            tx.send(4).unwrap();
            assert_eq!(
                [rx.recv(), rx.recv(), rx.recv()],
                [Ok(2), Ok(3), Ok(4)],
                "messages must come out in the order in which they went in"
            );
        });
    }

    #[test]
    #[should_panic(expected = "at least one message")]
    fn a_zero_capacity_is_refused() {
        let _ = channel::<u8>(0);
    }

    #[test]
    fn b_try_send_reports_full_and_closed() {
        within(10, "the try_send test", || {
            let (tx, rx) = channel(2);
            assert_eq!(tx.try_send('a'), Ok(()));
            assert_eq!(tx.try_send('b'), Ok(()));
            assert_eq!(
                tx.try_send('c'),
                Err(TrySendError::Full('c')),
                "the queue holds 2 messages, so the third must come back as `Full`"
            );
            assert_eq!(rx.recv(), Ok('a'));
            assert_eq!(tx.try_send('c'), Ok(()), "`recv` freed one slot");
            drop(rx);
            assert_eq!(tx.try_send('d'), Err(TrySendError::Closed('d')));
            assert_eq!(
                tx.send('e'),
                Err(SendError('e')),
                "without a receiver, `send` must return its message"
            );
        });
    }

    #[test]
    fn c_recv_ends_after_the_last_sender() {
        within(10, "receiving after the senders are gone", || {
            let (tx, rx) = channel(4);
            let second = tx.clone();
            tx.send(1).unwrap();
            second.send(2).unwrap();
            drop(tx);
            assert_eq!(rx.recv(), Ok(1), "queued messages outlive their sender");
            assert_eq!(rx.recv(), Ok(2));
            // One sender still exists and the queue is empty: a `recv` here
            // would block. Drop it on another thread while we wait.
            let dropper = std::thread::spawn(move || drop(second));
            assert_eq!(
                rx.recv(),
                Err(RecvError),
                "an empty queue without senders must end with RecvError"
            );
            dropper.join().unwrap();
            assert_eq!(rx.recv(), Err(RecvError), "and it must stay that way");
        });
    }

    #[test]
    fn c_a_full_queue_makes_the_sender_wait() {
        let received = within(20, "a sender that needs the receiver to make room", || {
            let (tx, rx) = channel(1);
            // How many `send` calls have returned so far.
            let sent = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let producer = {
                let sent = std::sync::Arc::clone(&sent);
                std::thread::spawn(move || {
                    for n in 0..50_u32 {
                        tx.send(n).unwrap();
                        sent.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
            };
            let mut received = Vec::new();
            loop {
                // This holds at every moment, whatever the threads' speed:
                // with room for one message, at most one message more was
                // sent than was received.
                let sent_so_far = sent.load(std::sync::atomic::Ordering::SeqCst);
                assert!(
                    sent_so_far <= received.len() + 1,
                    "{sent_so_far} `send` calls returned, but only {} messages were \
                     received and the queue has room for 1: `send` must wait while \
                     the queue is full",
                    received.len()
                );
                match rx.recv() {
                    Ok(n) => received.push(n),
                    Err(RecvError) => break,
                }
            }
            producer.join().unwrap();
            received
        });
        assert_eq!(received, (0..50).collect::<Vec<_>>());
    }

    #[test]
    fn c_dropping_the_receiver_releases_blocked_senders() {
        within(
            20,
            "senders blocked on a full queue when the receiver goes away",
            || {
                let (tx, rx) = channel(1);
                tx.send(0_u32).unwrap();
                let senders: Vec<_> = (1..=3)
                    .map(|n| {
                        let tx = tx.clone();
                        std::thread::spawn(move || tx.send(n))
                    })
                    .collect();
                drop(rx);
                for (n, sender) in (1..=3).zip(senders) {
                    assert_eq!(
                        sender.join().unwrap(),
                        Err(SendError(n)),
                        "each blocked sender must wake up and get its own message back"
                    );
                }
            },
        );
    }

    #[test]
    fn d_many_senders_one_receiver() {
        let per_sender: u32 = if cfg!(miri) { 20 } else { 2_000 };
        let received = within(60, "the channel stress test", move || {
            let (tx, rx) = channel(4);
            let senders: Vec<_> = (0..4_u32)
                .map(|id| {
                    let tx = tx.clone();
                    std::thread::spawn(move || {
                        for n in 0..per_sender {
                            tx.send((id, n)).unwrap();
                        }
                    })
                })
                .collect();
            drop(tx);
            let mut received = Vec::new();
            while let Ok(message) = rx.recv() {
                received.push(message);
            }
            for sender in senders {
                sender.join().unwrap();
            }
            received
        });
        assert_eq!(
            received.len() as u32,
            4 * per_sender,
            "no message may be lost or repeated"
        );
        for id in 0..4 {
            let from_one: Vec<u32> = received
                .iter()
                .filter(|(sender, _)| *sender == id)
                .map(|(_, n)| *n)
                .collect();
            assert_eq!(
                from_one,
                (0..per_sender).collect::<Vec<_>>(),
                "the messages of sender {id} must arrive in the order it sent them"
            );
        }
    }

    #[test]
    fn e_every_message_is_dropped_exactly_once() {
        within(10, "the drop accounting test", || {
            let drops = DropCount::new();
            let (tx, rx) = channel(4);
            for id in 0..3 {
                tx.send(drops.value(id)).unwrap();
            }
            let first = rx.recv().unwrap();
            assert_eq!(first.id, 0);
            assert_eq!(drops.get(), 0, "queued and received messages are all alive");
            drop(rx);
            assert_eq!(
                drops.get(),
                2,
                "dropping the receiver must drop the two messages still in the queue, \
                 although a sender still exists"
            );
            let rejected = tx.send(drops.value(9)).unwrap_err();
            assert_eq!(
                rejected.0.id, 9,
                "the rejected message must come back unchanged"
            );
            drop(rejected);
            drop(first);
            drop(tx);
            assert_eq!(drops.get(), 4, "every message must be dropped exactly once");
        });
    }

    #[test]
    fn f_no_allocation_after_creation() {
        within(10, "the allocation test", || {
            let (tx, rx) = channel(64);
            // Warm-up, not counted: on some platforms a lock or a condition
            // variable allocates the first time it is used.
            tx.send(0_u64).unwrap();
            assert_eq!(rx.recv(), Ok(0));
            // One message fewer than the capacity: enough to make a queue
            // grow that was not reserved, without reaching the "full" state,
            // in which a channel might use a condition variable for the
            // first time.
            let (clone, allocations) = count(|| {
                for n in 0..63 {
                    tx.send(n).unwrap();
                }
                for _ in 0..63 {
                    rx.recv().unwrap();
                }
                tx.clone()
            });
            assert_eq!(
                allocations, 0,
                "63 sends, 63 receives and a clone made {allocations} heap allocation(s); \
                 reserve the whole buffer in `channel` and share one `Arc`"
            );
            drop(clone);
        });
    }

    #[test]
    fn f_ends_move_between_threads_for_send_messages() {
        let _ = channel::<u8>(1);
        assert!(implements!(Sender<String>: Send + Sync));
        assert!(implements!(Receiver<String>: Send));
        assert!(
            !implements!(Sender<Rc<u8>>: Send),
            "a channel moves messages to another thread, so they must be `Send`"
        );
        assert!(!implements!(Receiver<Rc<u8>>: Send));
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// These predictions use the standard library's bounded channel,
    /// `std::sync::mpsc::sync_channel`.
    #[test]
    fn p_predictions() {
        use std::sync::mpsc::sync_channel;

        // P1. Three `try_send` calls on a channel with room for two.
        let (tx, rx) = sync_channel(2);
        let attempts = [1, 2, 3].map(|n| tx.try_send(n).is_ok());
        let p1: [bool; 3] = guess();
        assert_eq!(
            p1, attempts,
            "P1: which of three try_send calls succeed (capacity 2)"
        );

        // P2. The sender is dropped while two messages are queued.
        drop(tx);
        let results = [rx.recv().ok(), rx.recv().ok(), rx.recv().ok()];
        let p2: [Option<i32>; 3] = guess();
        assert_eq!(
            p2, results,
            "P2: three recv calls after the sender was dropped"
        );

        // P3. The receiver is dropped; then the sender sends 9.
        let (tx, rx) = sync_channel(2);
        drop(rx);
        let returned = tx.send(9).map_err(|error| error.0);
        let p3: Result<(), i32> = guess();
        assert_eq!(
            p3, returned,
            "P3: the result of `send(9)` without a receiver"
        );
    }
}

#[cfg(all(test, loom))]
mod model {
    use super::*;
    use crate::support::{DropCount, Tracked, model, model_bounded};
    use crate::sync::atomic::{AtomicUsize, Ordering};
    use crate::sync::thread;

    #[test]
    fn c_one_sender_one_receiver() {
        model(|| {
            let (tx, rx) = channel(1);
            let sender = thread::spawn(move || {
                for n in 0..2_u32 {
                    tx.send(Tracked::new(n)).unwrap();
                }
            });
            // A deadlock report means that a blocked `send` or `recv` was
            // never notified.
            assert_eq!(rx.recv().map(|m| m.get()), Ok(0));
            assert_eq!(rx.recv().map(|m| m.get()), Ok(1));
            assert!(
                rx.recv().is_err(),
                "the sender is gone and the queue is empty, so `recv` must end"
            );
            sender.join().unwrap();
        });
    }

    #[test]
    fn c_a_woken_sender_checks_again() {
        model_bounded(2, || {
            let (tx, rx) = channel(1);
            tx.send(0_u32).unwrap();
            // The queue is full. Two more senders want to send.
            let finished = Arc::new(AtomicUsize::new(0));
            let senders: Vec<_> = (1..=2_u32)
                .map(|n| {
                    let (tx, finished) = (tx.clone(), Arc::clone(&finished));
                    thread::spawn(move || {
                        tx.send(n).unwrap();
                        finished.fetch_add(1, Ordering::SeqCst);
                    })
                })
                .collect();
            drop(tx);
            // One message leaves the queue, so exactly one slot is free.
            assert_eq!(rx.recv(), Ok(0));
            assert!(
                finished.load(Ordering::SeqCst) <= 1,
                "one slot was free, but both senders finished `send`: a sender that \
                 was woken pushed without checking that the slot was still free"
            );
            let mut rest = vec![rx.recv().unwrap(), rx.recv().unwrap()];
            rest.sort_unstable();
            assert_eq!(rest, [1, 2]);
            assert!(rx.recv().is_err());
            for sender in senders {
                sender.join().unwrap();
            }
        });
    }

    #[test]
    fn c_last_sender_wakes_the_receiver() {
        model(|| {
            let (tx, rx) = channel::<u32>(1);
            let second = tx.clone();
            let first_gone = thread::spawn(move || drop(tx));
            let second_gone = thread::spawn(move || drop(second));
            assert!(
                rx.recv().is_err(),
                "`recv` must return once both senders are gone"
            );
            first_gone.join().unwrap();
            second_gone.join().unwrap();
        });
    }

    #[test]
    fn c_dropped_receiver_wakes_every_sender() {
        model_bounded(2, || {
            let drops = DropCount::new();
            let (tx, rx) = channel(1);
            tx.send(Tracked::counted(0_u32, &drops)).unwrap();
            let senders: Vec<_> = (1..=2_u32)
                .map(|n| {
                    let (tx, drops) = (tx.clone(), drops);
                    thread::spawn(move || tx.send(Tracked::counted(n, &drops)).is_err())
                })
                .collect();
            drop(tx);
            drop(rx);
            for sender in senders {
                assert!(
                    sender.join().unwrap(),
                    "the queue was full and the receiver is gone, so `send` must fail"
                );
            }
            assert_eq!(drops.get(), 3, "every message must be dropped exactly once");
        });
    }
}
