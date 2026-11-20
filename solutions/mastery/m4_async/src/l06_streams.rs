//! LESSON 06 — A Stream trait, a channel, and adapters
//! Read first: CONCEPTS.md, "Lesson 06". Start after: lesson 05.
//! Run: cargo test --manifest-path mastery/m4_async/Cargo.toml l06_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. `poll_next` can return three different things. What does each one
//!    promise about the next call?
//! 2. A bounded channel is full. `try_send` could wait, drop the value, or
//!    hand it back. Which one does a function that is not `async` have to do?
//!
//! IMPLEMENT the tasks; keep the tests. From this lesson on you design the
//! types too. The contract fixes the public names and signatures. Every
//! struct, field and private helper is yours. You may change a return type
//! `impl Stream<..>` or `impl Future<..>` to a type that you name.
//! EXPLAIN: List every way in which the receiving task of your channel can be
//! woken, and for each one say why the wake cannot be lost. Then explain why
//! `merge` must remember that one side has ended.
//! MIRI LAB (optional): `cargo +nightly-2026-11-20 miri test --manifest-path
//! mastery/m4_async/Cargo.toml l06_` checks the projections in your adapters.

use std::collections::VecDeque;
use std::future::Future;
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker, ready};

/// A value that produces a sequence of items over time: the asynchronous
/// form of `Iterator`.
///
/// - `Ready(Some(item))`: the next item.
/// - `Ready(None)`: the stream has ended. The caller must not poll again.
/// - `Pending`: no item yet. The waker of `cx` will be woken when it is
///   worth polling again.
pub trait Stream {
    type Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
}

impl<S: ?Sized + Stream + Unpin> Stream for &mut S {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        Pin::new(&mut **self).poll_next(cx)
    }
}

/// A pinned pointer to a stream is a stream. With this impl you can write
/// `let mut s = pin!(stream);` and then call `s.next()` on any stream.
impl<P> Stream for Pin<P>
where
    P: DerefMut + Unpin,
    P::Target: Stream,
{
    type Item = <P::Target as Stream>::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().as_mut().poll_next(cx)
    }
}

/// Why [`Sender::try_send`] did not accept a value. The value comes back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrySendError<T> {
    /// The queue holds `capacity` values.
    Full(T),
    /// The receiver was dropped.
    Closed(T),
}

/// Locks a mutex and ignores poisoning (see lesson 02).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the senders and the receiver share.
struct Channel<T> {
    state: Mutex<ChannelState<T>>,
}

struct ChannelState<T> {
    queue: VecDeque<T>,
    capacity: usize,
    /// The number of `Sender` values that exist. Zero means that no value
    /// can arrive any more.
    senders: usize,
    receiver_alive: bool,
    /// The waker of the receiving task, while it waits on an empty queue.
    waiter: Option<Waker>,
}

/// A. A bounded queue with many senders and one receiver.
///
/// - The queue holds at most `capacity` values. `capacity` must be at
///   least 1; `channel` panics otherwise.
/// - Values arrive in the order in which they were accepted.
/// - The receiver is a [`Stream`]. It ends (`None`) when the queue is empty
///   and every sender has been dropped. Values that are still queued are
///   delivered first.
/// - When the receiver is dropped, the queued values are dropped with it,
///   and later sends fail with `Closed`.
/// - The receiver's waker is stored once and used once: the newest one
///   replaces an older one, and whoever wakes it takes it out of the
///   channel, so the channel never keeps a waker that it has already woken.
/// - The lock rule of lesson 03 applies: no wake and no drop of a waker or
///   of a `T` while the channel's lock is held.
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    assert!(capacity > 0, "a channel needs room for at least one value");
    let channel = Arc::new(Channel {
        state: Mutex::new(ChannelState {
            queue: VecDeque::with_capacity(capacity),
            capacity,
            senders: 1,
            receiver_alive: true,
            waiter: None,
        }),
    });
    (
        Sender {
            channel: Arc::clone(&channel),
        },
        Receiver { channel },
    )
}

/// The sending end of [`channel`]. Clones send into the same queue.
pub struct Sender<T> {
    channel: Arc<Channel<T>>,
}

impl<T> Sender<T> {
    /// A. Queue `value` if there is room, and wake the receiver if it waits.
    /// This function never waits: a full queue gives `Full(value)`, a
    /// dropped receiver gives `Closed(value)`.
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        let waiter = {
            let mut state = lock(&self.channel.state);
            if !state.receiver_alive {
                return Err(TrySendError::Closed(value));
            }
            if state.queue.len() >= state.capacity {
                return Err(TrySendError::Full(value));
            }
            state.queue.push_back(value);
            state.waiter.take()
        };
        if let Some(waiter) = waiter {
            waiter.wake();
        }
        Ok(())
    }
}

/// A. A clone is one more sender.
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        lock(&self.channel.state).senders += 1;
        Sender {
            channel: Arc::clone(&self.channel),
        }
    }
}

// A. Implement `Drop for Sender<T>` here: when the last sender goes away, a
// waiting receiver must be woken, because its stream has just ended.
impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let waiter = {
            let mut state = lock(&self.channel.state);
            state.senders -= 1;
            if state.senders == 0 {
                state.waiter.take()
            } else {
                None
            }
        };
        if let Some(waiter) = waiter {
            waiter.wake();
        }
    }
}

/// The receiving end of [`channel`]. It must be `Unpin`.
pub struct Receiver<T> {
    channel: Arc<Channel<T>>,
}

/// A. The next value; `None` once the queue is empty and no sender is left;
/// otherwise store the newest waker and return `Pending`.
impl<T> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let mut state = lock(&self.channel.state);
        if let Some(value) = state.queue.pop_front() {
            return Poll::Ready(Some(value));
        }
        if state.senders == 0 {
            return Poll::Ready(None);
        }
        // The check and the registration happen under one lock, and every
        // send and every sender drop takes the same lock, so no value and
        // no end of stream can slip in between.
        let replaced = match &state.waiter {
            Some(waiter) if waiter.will_wake(cx.waker()) => None,
            _ => state.waiter.replace(cx.waker().clone()),
        };
        drop(state);
        drop(replaced);
        Poll::Pending
    }
}

// A. Implement `Drop for Receiver<T>` here (see the contract of `channel`).
impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let (queue, waiter) = {
            let mut state = lock(&self.channel.state);
            state.receiver_alive = false;
            (std::mem::take(&mut state.queue), state.waiter.take())
        };
        // Values and the waker are dropped outside the lock: a destructor
        // of `T` may use a sender of this channel.
        drop(queue);
        drop(waiter);
    }
}

/// B. A stream that gives the items of an iterator, one per poll, and then
/// ends. It is never pending. After the end it keeps returning `None`.
///
/// The stream is `Unpin` for every iterator, also for one that is not
/// `Unpin` itself. Your stream type never pins the iterator, so it may say
/// so with an `Unpin` impl of its own (see CONCEPTS).
pub fn iter<I: IntoIterator>(items: I) -> impl Stream<Item = I::Item> + Unpin {
    Iter {
        items: items.into_iter().fuse(),
    }
}

/// The stream of [`iter`]. `Fuse` makes `None` permanent.
struct Iter<I> {
    items: std::iter::Fuse<I>,
}

// An iterator is plain data that is only used through `&mut`. Nothing in
// `Iter` relies on a stable address, so it can be `Unpin` for every `I`.
impl<I> Unpin for Iter<I> {}

impl<I: Iterator> Stream for Iter<I> {
    type Item = I::Item;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<I::Item>> {
        Poll::Ready(self.items.next())
    }
}

/// A stream stored inline next to plain state. All adapters with one inner
/// stream are built from it, so the projection is written, and argued, once.
struct With<S, T> {
    /// Structurally pinned.
    stream: S,
    /// Not pinned.
    state: T,
}

impl<S, T> With<S, T> {
    fn project(self: Pin<&mut Self>) -> (Pin<&mut S>, &mut T) {
        // SAFETY: `stream` is structurally pinned, `state` is not.
        // 1. Unpin: `With` has no manual `Unpin` impl, so it is `Unpin` only
        //    when `S` (and `T`) are.
        // 2. Drop: `With` has no `Drop` impl.
        // 3. No moves: `stream` is reached only through the `Pin<&mut S>`
        //    returned here; no code in this module moves it out of a pinned
        //    `With`, overwrites it, or forgets it.
        // 4. Layout: not `repr(packed)`.
        // `state` is never pinned, so `&mut T` is fine: moving a `T` breaks
        // no promise.
        unsafe {
            let this = self.get_unchecked_mut();
            (Pin::new_unchecked(&mut this.stream), &mut this.state)
        }
    }
}

/// State of `map`: the function.
struct MapFn<F>(F);

impl<S, U, F> Stream for With<S, MapFn<F>>
where
    S: Stream,
    F: FnMut(S::Item) -> U,
{
    type Item = U;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<U>> {
        let (stream, MapFn(f)) = self.project();
        stream.poll_next(cx).map(|item| item.map(f))
    }
}

/// State of `filter`: the test.
struct FilterFn<P>(P);

impl<S, P> Stream for With<S, FilterFn<P>>
where
    S: Stream,
    P: FnMut(&S::Item) -> bool,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        let (mut stream, FilterFn(keep)) = self.project();
        loop {
            // `ready!` passes `Pending` on. The inner stream has registered
            // the waker in that case, so nothing more is needed here.
            match ready!(stream.as_mut().poll_next(cx)) {
                Some(item) if keep(&item) => return Poll::Ready(Some(item)),
                Some(_) => {}
                None => return Poll::Ready(None),
            }
        }
    }
}

/// `collect` is a future: the state is the vector built so far.
impl<S: Stream> Future for With<S, Vec<S::Item>> {
    type Output = Vec<S::Item>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Vec<S::Item>> {
        let (mut stream, items) = self.project();
        while let Some(item) = ready!(stream.as_mut().poll_next(cx)) {
            items.push(item);
        }
        Poll::Ready(std::mem::take(items))
    }
}

/// The future of `next`.
struct Next<'a, S> {
    stream: &'a mut S,
}

impl<S: Stream + Unpin> Future for Next<'_, S> {
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        Pin::new(&mut *self.stream).poll_next(cx)
    }
}

/// B. Methods for every stream. The adapters are lazy: they do nothing until
/// they are polled. They store the inner stream inline (no `Box`) and work
/// for streams that are not `Unpin`, so they need a pin projection.
pub trait StreamExt: Stream + Sized {
    /// A future for the next item. It borrows the stream, so the stream can
    /// be used again afterwards. Dropping the future loses no item.
    fn next(&mut self) -> impl Future<Output = Option<Self::Item>>
    where
        Self: Unpin,
    {
        Next { stream: self }
    }

    /// A stream of `f(item)` for every item.
    fn map<U, F>(self, f: F) -> impl Stream<Item = U>
    where
        F: FnMut(Self::Item) -> U,
    {
        With {
            stream: self,
            state: MapFn(f),
        }
    }

    /// A stream of the items for which `keep` returns `true`. When the inner
    /// stream is pending, so is this one.
    fn filter<P>(self, keep: P) -> impl Stream<Item = Self::Item>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        With {
            stream: self,
            state: FilterFn(keep),
        }
    }

    /// A future that gathers every item until the stream ends.
    fn collect(self) -> impl Future<Output = Vec<Self::Item>> {
        With {
            stream: self,
            state: Vec::new(),
        }
    }
}

impl<S: Stream> StreamExt for S {}

/// C. One stream from two: items of `a` and `b` as they become ready.
///
/// - Each call of `poll_next` polls one side first and, if that side has no
///   item, the other side. The first call prefers `a`; the preference
///   changes sides on every call. Two busy streams therefore take turns.
/// - A side that has ended is never polled again.
/// - The merged stream ends when both sides have ended. It is pending when
///   no side has an item and at least one side is pending.
pub fn merge<A, B>(a: A, b: B) -> impl Stream<Item = A::Item>
where
    A: Stream,
    B: Stream<Item = A::Item>,
{
    Merge {
        a,
        b,
        state: MergeState {
            a_done: false,
            b_done: false,
            prefer_b: false,
        },
    }
}

/// The stream of [`merge`].
struct Merge<A, B> {
    /// Structurally pinned.
    a: A,
    /// Structurally pinned.
    b: B,
    /// Not pinned.
    state: MergeState,
}

struct MergeState {
    a_done: bool,
    b_done: bool,
    /// Which side the next call polls first.
    prefer_b: bool,
}

impl<A, B> Merge<A, B> {
    fn project(self: Pin<&mut Self>) -> (Pin<&mut A>, Pin<&mut B>, &mut MergeState) {
        // SAFETY: `a` and `b` are structurally pinned, `state` is not.
        // 1. Unpin: no manual impl; `Merge` is `Unpin` only when `A` and `B`
        //    are.
        // 2. Drop: `Merge` has no `Drop` impl.
        // 3. No moves: `a` and `b` are reached only through the pins
        //    returned here and are never moved, overwritten or forgotten.
        // 4. Layout: not `repr(packed)`.
        unsafe {
            let this = self.get_unchecked_mut();
            (
                Pin::new_unchecked(&mut this.a),
                Pin::new_unchecked(&mut this.b),
                &mut this.state,
            )
        }
    }
}

/// Polls one side unless it has ended. Returns an item if there is one.
fn poll_side<S: Stream>(
    stream: Pin<&mut S>,
    done: &mut bool,
    cx: &mut Context<'_>,
) -> Option<S::Item> {
    if *done {
        return None;
    }
    match stream.poll_next(cx) {
        Poll::Ready(Some(item)) => Some(item),
        Poll::Ready(None) => {
            *done = true;
            None
        }
        Poll::Pending => None,
    }
}

impl<A, B> Stream for Merge<A, B>
where
    A: Stream,
    B: Stream<Item = A::Item>,
{
    type Item = A::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<A::Item>> {
        let (a, b, state) = self.project();
        let b_first = state.prefer_b;
        state.prefer_b = !b_first;
        let item = if b_first {
            poll_side(b, &mut state.b_done, cx).or_else(|| poll_side(a, &mut state.a_done, cx))
        } else {
            poll_side(a, &mut state.a_done, cx).or_else(|| poll_side(b, &mut state.b_done, cx))
        };
        match item {
            Some(item) => Poll::Ready(Some(item)),
            None if state.a_done && state.b_done => Poll::Ready(None),
            // At least one side returned Pending in this call and has
            // registered the waker.
            None => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::alloc::count;
    use crate::support::{DropLog, WakeCounter, block_on, guess, within};
    use std::cell::Cell;
    use std::marker::PhantomPinned;
    use std::pin::pin;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::task::Wake;
    use std::thread;

    fn noop_cx() -> Context<'static> {
        Context::from_waker(Waker::noop())
    }

    /// Polls a stream once with a waker that does nothing.
    fn poll_once<S: Stream>(stream: Pin<&mut S>) -> Poll<Option<S::Item>> {
        stream.poll_next(&mut noop_cx())
    }

    /// A `!Unpin` stream that gives `items`, is pending (after waking
    /// itself) before each one, and records whether it was moved after its
    /// first poll or polled after its end.
    struct Rooted<T> {
        items: Cell<VecDeque<T>>,
        ready: Cell<bool>,
        home: Cell<Option<usize>>,
        ended: Cell<bool>,
        report: RootedReport,
        _pinned: PhantomPinned,
    }

    #[derive(Clone, Default)]
    struct RootedReport {
        moved: Arc<AtomicBool>,
        polled_after_end: Arc<AtomicBool>,
        polls: Arc<AtomicUsize>,
    }

    impl RootedReport {
        fn check(&self) {
            assert!(
                !self.moved.load(Ordering::SeqCst),
                "an adapter moved its inner stream after the first poll: \
                 store the stream inline and reach it through a pin projection"
            );
            assert!(
                !self.polled_after_end.load(Ordering::SeqCst),
                "an inner stream was polled again after it returned None"
            );
        }
        fn polls(&self) -> usize {
            self.polls.load(Ordering::SeqCst)
        }
    }

    fn rooted<T>(items: impl IntoIterator<Item = T>) -> (Rooted<T>, RootedReport) {
        let report = RootedReport::default();
        let stream = Rooted {
            items: Cell::new(items.into_iter().collect()),
            ready: Cell::new(false),
            home: Cell::new(None),
            ended: Cell::new(false),
            report: report.clone(),
            _pinned: PhantomPinned,
        };
        (stream, report)
    }

    impl<T> Stream for Rooted<T> {
        type Item = T;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
            let this: &Self = &self;
            let here = std::ptr::from_ref(this) as usize;
            match this.home.get() {
                None => this.home.set(Some(here)),
                Some(home) if home != here => this.report.moved.store(true, Ordering::SeqCst),
                Some(_) => {}
            }
            this.report.polls.fetch_add(1, Ordering::SeqCst);
            if this.ended.get() {
                this.report.polled_after_end.store(true, Ordering::SeqCst);
                return Poll::Ready(None);
            }
            if !this.ready.replace(true) {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            this.ready.set(false);
            let mut items = this.items.take();
            let item = items.pop_front();
            this.items.set(items);
            if item.is_none() {
                this.ended.set(true);
            }
            Poll::Ready(item)
        }
    }

    #[test]
    fn a_values_arrive_in_order() {
        let (tx, rx) = channel(4);
        let mut rx = pin!(rx);
        for n in 1..=3 {
            assert_eq!(tx.try_send(n), Ok(()));
        }
        for n in 1..=3 {
            assert_eq!(poll_once(rx.as_mut()), Poll::Ready(Some(n)));
        }
        assert_eq!(
            poll_once(rx.as_mut()),
            Poll::Pending,
            "an empty queue with a live sender is pending, not ended"
        );
    }

    #[test]
    fn a_a_full_queue_hands_the_value_back() {
        let (tx, rx) = channel(2);
        let mut rx = pin!(rx);
        assert_eq!(tx.try_send("a"), Ok(()));
        assert_eq!(tx.try_send("b"), Ok(()));
        assert_eq!(
            tx.try_send("c"),
            Err(TrySendError::Full("c")),
            "the third value does not fit into a queue of two"
        );
        assert_eq!(poll_once(rx.as_mut()), Poll::Ready(Some("a")));
        assert_eq!(
            tx.try_send("c"),
            Ok(()),
            "one value was received, so there is room again"
        );
        assert_eq!(poll_once(rx.as_mut()), Poll::Ready(Some("b")));
        assert_eq!(poll_once(rx.as_mut()), Poll::Ready(Some("c")));
    }

    #[test]
    fn a_zero_capacity_is_refused() {
        let (tx, _rx) = channel(1);
        assert_eq!(tx.try_send(1u8), Ok(()), "a capacity of one is allowed");
        assert!(
            std::panic::catch_unwind(|| channel::<u8>(0)).is_err(),
            "a channel that can never hold a value is a bug of the caller: `channel(0)` must panic"
        );
    }

    #[test]
    fn a_the_stream_ends_when_the_last_sender_is_dropped() {
        let (tx, rx) = channel(4);
        let mut rx = pin!(rx);
        let second = tx.clone();
        tx.try_send(1).unwrap();
        drop(tx);
        second.try_send(2).unwrap();
        let (counter, waker) = WakeCounter::new();
        let mut cx = Context::from_waker(&waker);
        assert_eq!(rx.as_mut().poll_next(&mut cx), Poll::Ready(Some(1)));
        assert_eq!(rx.as_mut().poll_next(&mut cx), Poll::Ready(Some(2)));
        assert_eq!(
            rx.as_mut().poll_next(&mut cx),
            Poll::Pending,
            "one sender is still alive"
        );
        assert_eq!(counter.count(), 0);
        second.clone().try_send(3).unwrap();
        assert_eq!(
            counter.count(),
            1,
            "a send into an empty queue must wake the waiting receiver"
        );
        drop(second);
        assert_eq!(
            counter.count(),
            1,
            "the receiver was woken and has not polled since, so nothing is registered"
        );
        assert_eq!(
            rx.as_mut().poll_next(&mut cx),
            Poll::Ready(Some(3)),
            "values that were queued before the last sender left must still arrive"
        );
        assert_eq!(
            rx.as_mut().poll_next(&mut cx),
            Poll::Ready(None),
            "empty queue and no sender: the stream has ended"
        );
    }

    #[test]
    fn a_dropping_the_last_sender_wakes_a_waiting_receiver() {
        let (tx, rx) = channel::<u8>(1);
        let mut rx = pin!(rx);
        let second = tx.clone();
        let (counter, waker) = WakeCounter::new();
        let mut cx = Context::from_waker(&waker);
        assert_eq!(rx.as_mut().poll_next(&mut cx), Poll::Pending);
        drop(tx);
        assert_eq!(
            counter.count(),
            0,
            "one sender is left; nothing has changed"
        );
        drop(second);
        assert_eq!(
            counter.count(),
            1,
            "when the last sender is dropped, the receiver must be woken to see the end"
        );
        assert_eq!(rx.as_mut().poll_next(&mut cx), Poll::Ready(None));
    }

    #[test]
    fn a_the_receiver_keeps_only_the_newest_waker() {
        let (tx, rx) = channel(1);
        let mut rx = pin!(rx);
        let (old, old_waker) = WakeCounter::new();
        let (new, new_waker) = WakeCounter::new();
        assert!(
            rx.as_mut()
                .poll_next(&mut Context::from_waker(&old_waker))
                .is_pending()
        );
        assert!(
            rx.as_mut()
                .poll_next(&mut Context::from_waker(&new_waker))
                .is_pending()
        );
        assert_eq!(
            Arc::strong_count(&old),
            2,
            "the waker of the first poll must be released when a newer one replaces it"
        );
        tx.try_send(5).unwrap();
        assert_eq!(
            (old.count(), new.count()),
            (0, 1),
            "(wakes of the old waker, wakes of the new one)"
        );
        assert_eq!(
            Arc::strong_count(&new),
            2,
            "a waker that was woken must not stay stored in the channel"
        );
    }

    #[test]
    fn a_a_dropped_receiver_closes_the_channel_and_drops_the_queue() {
        let log = DropLog::new();
        let (tx, rx) = channel(4);
        tx.try_send(log.guard("queued")).unwrap();
        drop(rx);
        assert_eq!(
            log.events(),
            ["queued"],
            "values that nobody can receive any more must be dropped with the receiver"
        );
        match tx.try_send(log.guard("late")) {
            Err(TrySendError::Closed(value)) => assert_eq!(
                value.name(),
                "late",
                "a closed channel hands the value back"
            ),
            _ => panic!("after the receiver is dropped, try_send must return Closed(value)"),
        }
    }

    /// A value that owns a sender of the channel it travels through, as a
    /// request that carries its own reply address does.
    struct Carrier(Sender<Carrier>);

    #[test]
    fn a_queued_values_are_dropped_after_the_lock_is_released() {
        within(
            10,
            "dropping a receiver whose queued value owns a sender of the same channel \
             (move the queue out of the critical section, then drop it)",
            || {
                let (tx, rx) = channel(2);
                assert!(tx.try_send(Carrier(tx.clone())).is_ok());
                // The receiver takes the queued value with it. Dropping that
                // value drops a sender, and that needs the channel's lock.
                drop(rx);
                assert!(
                    matches!(
                        tx.try_send(Carrier(tx.clone())),
                        Err(TrySendError::Closed(_))
                    ),
                    "after the receiver is dropped the channel is closed"
                );
            },
        );
    }

    /// A waker that sends into the channel when it is woken. That needs the
    /// channel's lock.
    struct SendOnWake(Sender<&'static str>);
    impl Wake for SendOnWake {
        fn wake(self: Arc<Self>) {
            let _ = self.0.try_send("sent by the waker");
        }
    }

    #[test]
    fn a_the_receiver_is_woken_after_the_lock_is_released() {
        within(
            10,
            "a send that wakes a waker which sends again (wake the receiver \
             after releasing the channel's lock)",
            || {
                let (tx, rx) = channel(4);
                let mut rx = pin!(rx);
                let waker = Waker::from(Arc::new(SendOnWake(tx.clone())));
                let mut cx = Context::from_waker(&waker);
                assert!(rx.as_mut().poll_next(&mut cx).is_pending());
                tx.try_send("first").unwrap();
                assert_eq!(rx.as_mut().poll_next(&mut cx), Poll::Ready(Some("first")));
                assert_eq!(
                    rx.as_mut().poll_next(&mut cx),
                    Poll::Ready(Some("sent by the waker"))
                );
            },
        );
    }

    #[test]
    fn a_values_cross_threads_and_wake_a_blocked_receiver() {
        let received = within(20, "receiving 500 values from another thread", || {
            let (tx, rx) = channel(8);
            let producer = thread::spawn(move || {
                for n in 0..500u32 {
                    let mut value = n;
                    // Wait for room by trying again; a real program would
                    // drop the value or apply backpressure instead.
                    while let Err(TrySendError::Full(back)) = tx.try_send(value) {
                        value = back;
                        thread::yield_now();
                    }
                }
            });
            let received = block_on(rx.collect());
            producer.join().unwrap();
            received
        });
        assert_eq!(
            received,
            (0..500).collect::<Vec<u32>>(),
            "every value must arrive once and in order"
        );
    }

    #[test]
    fn b_iter_gives_each_item_then_none_for_ever() {
        let mut stream = pin!(iter(["a", "b"]));
        assert_eq!(poll_once(stream.as_mut()), Poll::Ready(Some("a")));
        assert_eq!(poll_once(stream.as_mut()), Poll::Ready(Some("b")));
        assert_eq!(poll_once(stream.as_mut()), Poll::Ready(None));
        assert_eq!(
            poll_once(stream.as_mut()),
            Poll::Ready(None),
            "after its end `iter` keeps returning None"
        );
    }

    #[test]
    fn b_next_borrows_the_stream() {
        let (tx, rx) = channel(2);
        let mut rx = pin!(rx);
        tx.try_send(1).unwrap();
        assert_eq!(block_on(rx.next()), Some(1));
        // A `next` future that is dropped while pending takes nothing.
        {
            let mut pending_next = pin!(rx.next());
            assert!(pending_next.as_mut().poll(&mut noop_cx()).is_pending());
        }
        tx.try_send(2).unwrap();
        drop(tx);
        assert_eq!(
            block_on(rx.next()),
            Some(2),
            "dropping a pending `next` future must not lose an item"
        );
        assert_eq!(block_on(rx.next()), None);
    }

    #[test]
    fn b_map_filter_and_collect_work_together() {
        let (inner, report) = rooted(1..=10);
        let output = block_on(inner.filter(|n| n % 2 == 0).map(|n| n * 10).collect());
        assert_eq!(output, [20, 40, 60, 80, 100]);
        report.check();
    }

    #[test]
    fn b_adapters_are_lazy() {
        let calls = Cell::new(0);
        let (inner, report) = rooted([1, 2, 3]);
        let stream = inner.map(|n| {
            calls.set(calls.get() + 1);
            n
        });
        assert_eq!(
            (calls.get(), report.polls()),
            (0, 0),
            "(calls of the closure, polls of the inner stream) before the adapter is polled"
        );
        let mut stream = pin!(stream);
        assert_eq!(
            poll_once(stream.as_mut()),
            Poll::Pending,
            "the inner stream is pending first, so the adapter is pending"
        );
        assert_eq!(poll_once(stream.as_mut()), Poll::Ready(Some(1)));
        assert_eq!(calls.get(), 1, "one item, one call");
    }

    #[test]
    fn b_filter_passes_pending_on_and_skips_within_one_poll() {
        // `iter` is always ready, so one poll of `filter` must skip 1, 2, 3.
        let mut ready_inner = pin!(iter([1, 2, 3, 40, 5]).filter(|n| *n > 10));
        assert_eq!(
            poll_once(ready_inner.as_mut()),
            Poll::Ready(Some(40)),
            "filter must poll the inner stream again when it rejects an item; \
             returning Pending there loses the wakeup"
        );
        assert_eq!(poll_once(ready_inner.as_mut()), Poll::Ready(None));

        let (inner, report) = rooted([1, 20]);
        let mut stream = pin!(inner.filter(|n| *n > 10));
        let (counter, waker) = WakeCounter::new();
        let mut cx = Context::from_waker(&waker);
        assert_eq!(stream.as_mut().poll_next(&mut cx), Poll::Pending);
        assert_eq!(
            (report.polls(), counter.count()),
            (1, 1),
            "(polls of the inner stream, wakes): filter must return the inner stream's \
             Pending and must not wake by itself"
        );
        assert_eq!(
            stream.as_mut().poll_next(&mut cx),
            Poll::Pending,
            "1 is rejected, then the inner stream is pending again"
        );
        assert_eq!(stream.as_mut().poll_next(&mut cx), Poll::Ready(Some(20)));
        report.check();
    }

    #[test]
    fn b_adapters_store_the_stream_inline_and_do_not_allocate() {
        let (inner, report) = rooted([3u8, 4, 5]);
        let mut stream = pin!(inner.map(|n| n + 1).filter(|n| n % 2 == 0));
        let mut cx = noop_cx();
        let (items, allocations) = count(|| {
            let mut items = [0u8; 2];
            let mut found = 0;
            for _ in 0..100 {
                match stream.as_mut().poll_next(&mut cx) {
                    Poll::Ready(Some(item)) => {
                        assert!(
                            found < items.len(),
                            "3, 4, 5 plus one, then only the even numbers, gives 4 and 6; \
                             a third item arrived: {item}"
                        );
                        items[found] = item;
                        found += 1;
                    }
                    Poll::Ready(None) => return items,
                    Poll::Pending => {}
                }
            }
            panic!("the adapted stream of three items did not end within 100 polls")
        });
        assert_eq!(
            (items, allocations),
            ([4, 6], 0),
            "(items, allocations): map and filter must not box the inner stream"
        );
        report.check();
    }

    #[test]
    fn c_merge_takes_turns() {
        let merged = block_on(merge(iter([1, 2, 3]), iter([10, 20, 30])).collect());
        assert_eq!(
            merged,
            [1, 10, 2, 20, 3, 30],
            "two streams that are always ready must alternate, starting with the first"
        );
    }

    #[test]
    fn c_merge_continues_with_the_side_that_is_left() {
        let (short, short_report) = rooted([1]);
        let (long, long_report) = rooted([10, 20, 30]);
        let merged = block_on(merge(short, long).collect());
        let mut sorted = merged.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, [1, 10, 20, 30], "no item may be lost or repeated");
        assert_eq!(
            merged
                .iter()
                .filter(|n| **n >= 10)
                .copied()
                .collect::<Vec<_>>(),
            [10, 20, 30],
            "the order within one side must be kept"
        );
        short_report.check();
        long_report.check();
    }

    #[test]
    fn c_merge_waits_on_both_sides() {
        let (left_tx, left) = channel(2);
        let (right_tx, right) = channel(2);
        let mut merged = pin!(merge(left, right));
        let (counter, waker) = WakeCounter::new();
        let mut cx = Context::from_waker(&waker);
        assert_eq!(merged.as_mut().poll_next(&mut cx), Poll::Pending);
        right_tx.try_send("right").unwrap();
        assert_eq!(
            counter.count(),
            1,
            "a pending merge must have registered the waker with both sides"
        );
        assert_eq!(
            merged.as_mut().poll_next(&mut cx),
            Poll::Ready(Some("right"))
        );
        assert_eq!(merged.as_mut().poll_next(&mut cx), Poll::Pending);
        left_tx.try_send("left").unwrap();
        assert_eq!(counter.count(), 2);
        assert_eq!(
            merged.as_mut().poll_next(&mut cx),
            Poll::Ready(Some("left"))
        );
        drop(left_tx);
        assert_eq!(
            merged.as_mut().poll_next(&mut cx),
            Poll::Pending,
            "one side has ended, the other is still open"
        );
        drop(right_tx);
        assert_eq!(
            merged.as_mut().poll_next(&mut cx),
            Poll::Ready(None),
            "merge ends when both sides have ended"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        /// A stream that counts down: n, n - 1, ..., 1, then ends.
        struct Countdown(u32);
        impl Stream for Countdown {
            type Item = u32;
            fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<u32>> {
                if self.0 == 0 {
                    return Poll::Ready(None);
                }
                self.0 -= 1;
                Poll::Ready(Some(self.0 + 1))
            }
        }

        // P1. This loop drains a stream of three items. How many times does
        // it call `poll_next`?
        let mut stream = Countdown(3);
        let mut calls = 0;
        let mut sum = 0;
        loop {
            calls += 1;
            match Pin::new(&mut stream).poll_next(&mut noop_cx()) {
                Poll::Ready(Some(n)) => sum += n,
                Poll::Ready(None) => break,
                Poll::Pending => unreachable!("Countdown is never pending"),
            }
        }
        let p1: (u32, u32) = (4, 6);
        assert_eq!(
            p1,
            (calls, sum),
            "P1: (calls of poll_next, sum of the items)"
        );

        // P2. The rule of `merge`, played with two queues: each turn prefers
        // one side and takes from the other side if the preferred one is
        // empty; the preference changes every turn, starting with `a`.
        // Write the resulting order.
        let mut a = VecDeque::from([1, 2]);
        let mut b = VecDeque::from([10, 20, 30, 40]);
        let mut order = Vec::new();
        let mut prefer_b = false;
        while !a.is_empty() || !b.is_empty() {
            let item = if prefer_b {
                b.pop_front().or_else(|| a.pop_front())
            } else {
                a.pop_front().or_else(|| b.pop_front())
            };
            prefer_b = !prefer_b;
            order.extend(item);
        }
        let p2: [i32; 6] = [1, 10, 2, 20, 30, 40];
        assert_eq!(
            p2.as_slice(),
            order,
            "P2: the order in which merge gives the items"
        );

        // P3. A bounded queue of capacity 2 is offered five values while
        // nobody takes one out. How many are accepted?
        let capacity = 2;
        let mut queue = VecDeque::new();
        let mut accepted = 0;
        for value in 0..5 {
            if queue.len() < capacity {
                queue.push_back(value);
                accepted += 1;
            }
        }
        let p3: usize = 2;
        assert_eq!(
            p3, accepted,
            "P3: values that a full queue turns away are not stored"
        );
    }
}
