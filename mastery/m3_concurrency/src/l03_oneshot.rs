//! LESSON 03 — A one-shot channel
//! Run: cargo test l03_
//! Model: cargo model l03_
//! Read first: "Lesson 03" in CONCEPTS.md (same package).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. The sender stores the message, sets the state and unparks the receiver.
//!    What goes wrong if it unparks first and sets the state second?
//! 2. `park` may return although nobody called `unpark`. Which line of `recv`
//!    makes that harmless?
//!
//! IMPLEMENT the tasks below. Import from `crate::sync`, never from `std`.
//! The types and their fields are given; the orderings and the `unsafe`
//! blocks are yours. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: For each value of `state`, say whether `message` is
//! initialised and who may access it. Then explain why `recv` cannot miss a
//! wake-up, and why the receiver is not `Send`.
//!
//! MODEL LAB (optional): In `Sender::send`, swap the order of the store and
//! the `unpark` call and run `cargo model l03_`. Loom reports a deadlock and
//! lists the state of every thread. Find the interleaving that it found.

use crate::sync::atomic::{AtomicU8, Ordering};
use crate::sync::thread::{self, Thread};
use crate::sync::{Arc, UnsafeCell};
use std::marker::PhantomData;
use std::mem::MaybeUninit;

/// No message yet, and the sender still exists.
const EMPTY: u8 = 0;
/// The sender wrote `message`. Nobody has taken it out.
const READY: u8 = 1;
/// The receiver took the message out. `message` is uninitialised again.
const TAKEN: u8 = 2;
/// The sender was dropped without sending. `message` was never written.
const CLOSED: u8 = 3;

/// The state shared by the two halves.
///
/// Invariant: `message` holds an initialised `T` exactly when `state == READY`.
/// Only the sender writes it (once, before it stores `READY`), and only the
/// receiver or `Drop` reads it (after it has seen `READY`).
///
/// D. Also write `impl<T> Drop for Shared<T>`: a message that was sent but
/// never received must be dropped exactly once, when the last half goes away.
struct Shared<T> {
    state: AtomicU8,
    message: UnsafeCell<MaybeUninit<T>>,
}

// SAFETY: both halves hold `&Shared<T>` on different threads. What crosses
// between the threads is a `T` by value, so `T: Send` is required; no `&T` is
// ever shared, so `T: Sync` is not. The rest of the argument is yours: your
// code must make sure that only one half touches `message` at a time.
unsafe impl<T: Send> Sync for Shared<T> {}

/// The sending half. `send` consumes it, so at most one message is sent.
///
/// B. Also write `impl<T> Drop for Sender<T>`: if the sender is dropped
/// without sending, the channel becomes `CLOSED` and the receiver is woken.
/// Remember that `send` also drops the sender, after it has sent.
pub struct Sender<T> {
    shared: Arc<Shared<T>>,
    /// The thread that called `channel`. It is the only thread that can ever
    /// hold the `Receiver`, because the `Receiver` is not `Send`.
    receiver: Thread,
}

/// The receiving half. It must stay on the thread that created the channel.
pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
    /// A raw pointer is not `Send`, so this marker makes the receiver `!Send`.
    _stays_on_its_thread: PhantomData<*const ()>,
}

/// `recv` returns this when the sender was dropped without sending.
#[derive(Debug, PartialEq, Eq)]
pub struct Disconnected;

/// A. Creates a channel for exactly one message. The calling thread is the
/// one that will receive: remember it with `thread::current()`.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    todo!("Implement this task")
}

impl<T> Sender<T> {
    /// B. Stores the message, publishes it, and wakes the receiver.
    ///
    /// The message is never lost: if the receiver is gone, or never calls
    /// `recv`, the message is dropped together with the channel.
    pub fn send(self, value: T) {
        todo!("Implement this task")
    }
}

impl<T> Receiver<T> {
    /// C. Waits for the message. Returns `Err(Disconnected)` if the sender
    /// was dropped without sending. Block with `thread::park()`; do not spin.
    pub fn recv(self) -> Result<T, Disconnected> {
        todo!("Implement this task")
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use crate::implements;
    use crate::support::{DropCount, alloc::count, guess, within};
    use std::rc::Rc;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn b_send_then_recv_on_one_thread() {
        let got = within(10, "recv after send on the same thread", || {
            let (tx, rx) = channel();
            tx.send(String::from("hello"));
            rx.recv()
        });
        assert_eq!(got.as_deref(), Ok("hello"));
    }

    #[test]
    fn c_recv_waits_for_a_sender_on_another_thread() {
        let got = within(10, "recv while another thread sends", || {
            let (tx, rx) = channel();
            let sender = std::thread::spawn(move || tx.send(42_u64));
            let got = rx.recv();
            sender.join().unwrap();
            got
        });
        assert_eq!(got, Ok(42));
    }

    #[test]
    fn c_recv_parks_instead_of_spinning() {
        within(
            10,
            "waiting for `recv` to park (it must call `thread::park` from `crate::sync`, \
             and must not spin) and then to be woken by `send`",
            || {
                let parks = std::sync::Arc::new(AtomicUsize::new(0));
                let (tx_out, tx_in) = std::sync::mpsc::channel();
                let receiver = {
                    let parks = std::sync::Arc::clone(&parks);
                    std::thread::spawn(move || {
                        crate::sync::thread::count_parks_in(&parks);
                        let (tx, rx) = channel::<u8>();
                        tx_out.send(tx).unwrap();
                        rx.recv()
                    })
                };
                let tx = tx_in.recv().unwrap();
                // Nothing is sent until the receiver has parked at least once. A
                // receiver that spins never parks, and `within` then reports it.
                while parks.load(Ordering::SeqCst) == 0 {
                    std::thread::yield_now();
                }
                tx.send(7);
                assert_eq!(
                    receiver.join().unwrap(),
                    Ok(7),
                    "the parked receiver must be woken by `send`"
                );
            },
        );
    }

    #[test]
    fn c_recv_parks_again_after_an_early_wake_up() {
        within(
            10,
            "waiting for `recv` to park a second time after its first `park` returned \
             early (it must call `thread::park` from `crate::sync` in a loop that checks \
             the state again, and must not spin)",
            || {
                let parks = std::sync::Arc::new(AtomicUsize::new(0));
                let (tx_out, tx_in) = std::sync::mpsc::channel();
                let receiver = {
                    let parks = std::sync::Arc::clone(&parks);
                    std::thread::spawn(move || {
                        crate::sync::thread::count_parks_in(&parks);
                        // Leave a wake-up token behind, as earlier code on
                        // this thread might. The next `park` then returns at
                        // once, although nothing was sent.
                        std::thread::current().unpark();
                        let (tx, rx) = channel::<u8>();
                        tx_out.send(tx).unwrap();
                        rx.recv()
                    })
                };
                let tx = tx_in.recv().unwrap();
                // A correct `recv` wakes up, sees that nothing has arrived, and
                // parks again. One that parks only once returns too early.
                while parks.load(Ordering::SeqCst) < 2 && !receiver.is_finished() {
                    std::thread::yield_now();
                }
                assert!(
                    !receiver.is_finished(),
                    "`recv` returned before anything was sent. `park` may return although \
                     nobody called `unpark` for this wait, so `recv` must check the state \
                     again after every `park`"
                );
                tx.send(7);
                assert_eq!(
                    receiver.join().unwrap(),
                    Ok(7),
                    "after the early wake-up, `recv` must still deliver the message"
                );
            },
        );
    }

    #[test]
    fn b_dropping_the_sender_disconnects() {
        let got = within(10, "recv after the sender was dropped", || {
            let (tx, rx) = channel::<u8>();
            drop(tx);
            rx.recv()
        });
        assert_eq!(got, Err(Disconnected));

        let got = within(10, "recv while another thread drops the sender", || {
            let (tx, rx) = channel::<u8>();
            let dropper = std::thread::spawn(move || drop(tx));
            let got = rx.recv();
            dropper.join().unwrap();
            got
        });
        assert_eq!(
            got,
            Err(Disconnected),
            "a receiver that is already waiting must be woken when the sender is dropped"
        );
    }

    #[test]
    fn d_every_message_is_dropped_exactly_once() {
        within(10, "the drop accounting test", || {
            // Sent and received: the caller of `recv` owns the message.
            let drops = DropCount::new();
            let (tx, rx) = channel();
            tx.send(drops.value(1));
            let message = rx.recv().unwrap();
            assert_eq!(drops.get(), 0, "a received message must still be alive");
            drop(message);
            assert_eq!(drops.get(), 1);

            // Sent, never received: the channel must drop it.
            let drops = DropCount::new();
            let (tx, rx) = channel();
            tx.send(drops.value(2));
            assert_eq!(
                drops.get(),
                0,
                "an unreceived message stays alive in the channel"
            );
            drop(rx);
            assert_eq!(
                drops.get(),
                1,
                "a message that was sent but never received must be dropped with the channel"
            );

            // Never sent: there is nothing to drop.
            let drops = DropCount::new();
            let (tx, rx) = channel::<crate::support::Counted>();
            drop(tx);
            assert_eq!(rx.recv().err(), Some(Disconnected));
            assert_eq!(drops.get(), 0);
        });
    }

    #[test]
    fn b_the_message_is_moved_not_copied() {
        within(10, "the allocation test", || {
            let text = String::from("a message that lives on the heap");
            let buffer = text.as_ptr();
            let ((tx, rx), allocations) = count(channel::<String>);
            assert!(
                allocations <= 1,
                "`channel` made {allocations} heap allocations; one `Arc` holds everything"
            );
            let (got, allocations) = count(move || {
                tx.send(text);
                rx.recv().unwrap()
            });
            assert_eq!(
                got.as_ptr(),
                buffer,
                "the receiver must get the same String (same heap buffer), not a copy"
            );
            assert_eq!(
                allocations, 0,
                "`send` and `recv` made {allocations} heap allocation(s); moving the \
                 message through the shared cell needs none"
            );
        });
    }

    #[test]
    fn a_sender_moves_between_threads_receiver_does_not() {
        let _ = channel::<u8>();
        assert!(
            implements!(Sender<String>: Send),
            "the sender must be able to move to another thread"
        );
        assert!(
            !implements!(Sender<Rc<u8>>: Send),
            "a sender for a type that is not `Send` must not be `Send` either"
        );
        assert!(
            !implements!(Receiver<String>: Send),
            "the sender unparks the thread that created the channel, so the \
             receiver must stay on that thread"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        use std::mem::{MaybeUninit, size_of};
        use std::sync::mpsc;

        // P1. A `MaybeUninit` goes out of scope with a value inside. Then a
        // second one is told to drop its value. How many drops after each?
        let drops = DropCount::new();
        {
            let _slot = MaybeUninit::new(drops.value(1));
        }
        let after_scope = drops.get();
        let mut slot = MaybeUninit::new(drops.value(2));
        // SAFETY: the slot was initialised on the line above.
        unsafe { slot.assume_init_drop() };
        let after_explicit_drop = drops.get();
        let p1: (usize, usize) = guess();
        assert_eq!(
            p1,
            (after_scope, after_explicit_drop),
            "P1: drops counted (after the scope ends, after `assume_init_drop`)"
        );

        // P2. Why the channel keeps its state in a separate byte. (Answer
        // for a 64-bit target, where a `u64` is aligned to 8 bytes.)
        let p2: (usize, usize) = guess();
        assert_eq!(
            p2,
            (size_of::<MaybeUninit<u64>>(), size_of::<Option<u64>>()),
            "P2: (size of MaybeUninit<u64>, size of Option<u64>) in bytes"
        );

        // P3. The standard library's channel: one message, then the sender is dropped.
        let (tx, rx) = mpsc::channel();
        tx.send(5_u8).unwrap();
        drop(tx);
        let first = rx.recv().is_ok();
        let second = rx.recv().is_ok();
        let p3: (bool, bool) = guess();
        assert_eq!(
            p3,
            (first, second),
            "P3: is each of two `recv` calls `Ok` after one send and a dropped sender?"
        );
    }
}

#[cfg(all(test, loom))]
mod model {
    use super::*;
    use crate::support::{DropCount, Tracked, model};
    use crate::sync::thread::JoinHandle;

    /// Starts the receiving thread. It creates the channel, hands the sender
    /// to the caller of this function, and then runs `receive`.
    ///
    /// The receiver gets a thread of its own for a reason that is special to
    /// loom 0.7.3: its `unpark` wakes a thread out of *any* blocking call, for
    /// example out of `join`, which real threads never do. In these models
    /// the receiving thread therefore blocks in `park` and nowhere else.
    fn receiver_thread<T: Send + 'static, R: Send + 'static>(
        receive: impl FnOnce(Receiver<T>) -> R + Send + 'static,
    ) -> (Sender<T>, JoinHandle<R>) {
        let (hand_over, take) = loom::sync::mpsc::channel();
        let handle = thread::spawn(move || {
            let (tx, rx) = channel();
            hand_over.send(tx).unwrap();
            receive(rx)
        });
        (take.recv().unwrap(), handle)
    }

    #[test]
    fn c_recv_gets_the_message() {
        model(|| {
            let (tx, receiver) = receiver_thread(|rx: Receiver<Tracked<u32>>| {
                // Reading the message must be ordered after the sender's
                // write. A deadlock report here means that `recv` missed its
                // wake-up.
                rx.recv().map(|message| message.get())
            });
            tx.send(Tracked::new(7_u32));
            assert_eq!(receiver.join().unwrap(), Ok(7));
        });
    }

    #[test]
    fn b_dropped_sender_wakes_the_receiver() {
        model(|| {
            let (tx, receiver) =
                receiver_thread(|rx: Receiver<Tracked<u32>>| rx.recv().map(|m| m.get()));
            drop(tx);
            assert_eq!(
                receiver.join().unwrap(),
                Err(Disconnected),
                "nothing was sent, so `recv` must report the disconnection"
            );
        });
    }

    #[test]
    fn d_unreceived_message_is_dropped_once() {
        model(|| {
            let drops = DropCount::new();
            // The receiver goes away without receiving, possibly while the
            // sender is still writing.
            let (tx, receiver) = receiver_thread(|rx: Receiver<Tracked<u32>>| drop(rx));
            tx.send(Tracked::counted(1_u32, &drops));
            receiver.join().unwrap();
            assert_eq!(
                drops.get(),
                1,
                "the message must be dropped exactly once, by whichever half goes last"
            );
        });
    }

    #[test]
    fn d_received_message_is_dropped_once() {
        model(|| {
            let drops = DropCount::new();
            let (tx, receiver) = receiver_thread(|rx: Receiver<Tracked<u32>>| rx.recv());
            tx.send(Tracked::counted(1_u32, &drops));
            let message = receiver.join().unwrap().expect("the sender sent a message");
            assert_eq!(drops.get(), 0, "the received message is still alive");
            drop(message);
            assert_eq!(
                drops.get(),
                1,
                "after `recv` the channel must not drop the message a second time"
            );
        });
    }
}
