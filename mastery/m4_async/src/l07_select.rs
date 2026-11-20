//! LESSON 07 — Select and cancellation safety
//! Read first: CONCEPTS.md, "Lesson 07". Start after: lesson 06 (this lesson
//! uses its `Stream` trait, which is given code).
//! Run: cargo test --manifest-path mastery/m4_async/Cargo.toml l07_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. `select(a, b)` returns when `a` finishes. What happens to `b`, and to
//!    every value that `b` owned at that moment?
//! 2. A loop calls `select(reader.next_line(), tick)` again and again. Where
//!    must the bytes of a half-read line live so that no tick can lose them?
//!
//! IMPLEMENT the tasks; keep the tests. You design the types. The contract
//! fixes the public names and signatures only.
//! EXPLAIN: Define "cancellation safe" for a method such as `next_line` in
//! two sentences. Then classify each of these and say why: your `next_line`,
//! `Receiver::next` from lesson 06, `AsyncStream::read` and
//! `AsyncStream::write_all` from lesson 04.

use crate::l06_streams::Stream;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker, ready};

/// The output of [`select`]: which future finished, and its output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

/// A. Run two futures until the first one finishes.
///
/// - Each poll polls `a` first. If `a` is ready, `b` is not polled in that
///   poll. So `a` wins when both are ready.
/// - Nothing is polled before the returned future is polled.
/// - When the returned future completes, the future that lost has been
///   dropped. That is its cancellation.
/// - The futures are stored inline: no `Box`, no `Unpin` bound.
///
/// To keep a future alive across several `select` calls, pin it outside
/// and pass `fut.as_mut()`: a `Pin<&mut F>` is a future too, and dropping
/// it does not drop `F`.
///
/// The starter is an `async fn`. You may keep that, or change the signature
/// to return a future type that you write yourself.
pub async fn select<A, B>(a: A, b: B) -> Either<A::Output, B::Output>
where
    A: Future,
    B: Future,
{
    todo!("A: poll both in order; the loser is dropped with your future")
}

/// B. A flag that many tasks can wait for. All clones share one flag.
///
/// - `cancel` sets the flag and wakes every waiting future. Calling it
///   again does nothing.
/// - `cancelled()` returns a future that completes once the flag is set, at
///   once if it is set already. The future owns its share of the token: it
///   is `Send + 'static` and does not borrow `self`.
/// - A wait future that is dropped removes its waker from the token. A loop
///   that creates and drops one wait future per round must not make the
///   token grow.
/// - The lock rule of lesson 03 applies: no wake and no drop of a waker
///   while the token's lock is held.
#[derive(Clone, Default)]
pub struct CancelToken {
    _design: (),
}

impl CancelToken {
    /// A token that is not cancelled.
    pub fn new() -> CancelToken {
        CancelToken::default()
    }

    /// Sets the flag and wakes all waiters.
    pub fn cancel(&self) {
        todo!("B: set the flag, then wake outside the lock")
    }

    /// `true` once `cancel` was called on any clone.
    pub fn is_cancelled(&self) -> bool {
        todo!("B: read the flag")
    }

    /// A future that completes when the token is cancelled. `use<>` says
    /// that the future captures nothing from `&self`.
    pub fn cancelled(&self) -> impl Future<Output = ()> + Send + use<> {
        todo!("B: design the wait future");
        std::future::pending()
    }
}

/// The error of [`LineReader::next_line`]: a line was longer than allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooLong;

/// C. Cuts a stream of byte chunks into lines.
///
/// `source` gives the bytes in chunks of any size; a chunk may hold a part
/// of a line, or several lines.
pub struct LineReader<S> {
    source: S,
    _design: (),
}

impl<S: Stream<Item = Vec<u8>> + Unpin> LineReader<S> {
    /// A reader that accepts lines of at most `max_len` bytes, not counting
    /// the final `\n`.
    pub fn new(source: S, max_len: usize) -> LineReader<S> {
        todo!("C: an empty reader")
    }

    /// C. The next line.
    ///
    /// - `Ok(Some(line))`: the bytes up to the next `\n`, without the `\n`
    ///   and without a `\r` directly before it. Bytes that are not valid
    ///   UTF-8 are replaced as `String::from_utf8_lossy` does it.
    /// - `Ok(None)`: the source has ended and every byte was returned.
    ///   Bytes after the last `\n` count as a final line. After its end the
    ///   source is not polled again.
    /// - `Err(TooLong)`: the next line has more than `max_len` bytes before
    ///   its `\n` (a `\r` there counts as a byte of the line), or more than
    ///   `max_len` bytes have arrived for it and no `\n` is among them yet.
    ///   The reader does not go on reading; every later call gives the same
    ///   error.
    ///
    /// Cancellation safe: if the returned future is dropped before it
    /// completes, no byte is lost and no byte is delivered twice. The next
    /// call continues where this one stopped.
    pub async fn next_line(&mut self) -> Result<Option<String>, TooLong> {
        todo!("C: keep every received byte in `self`, not in this future")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::alloc::count;
    use crate::support::{
        Anchored, DropLog, Gate, WakeCounter, block_on, guess, poll_to_end, within,
    };
    use std::collections::VecDeque;
    use std::future::{pending, ready as ready_now};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{self, Sender};
    use std::task::Wake;
    use std::thread;

    fn noop_cx() -> Context<'static> {
        Context::from_waker(Waker::noop())
    }

    #[test]
    fn a_the_future_that_finishes_first_wins() {
        let (slow, _) = Anchored::new(5, "slow");
        let (fast, _) = Anchored::new(1, "fast");
        assert_eq!(block_on(select(slow, fast)), Either::Right("fast"));
        let (slow, _) = Anchored::new(5, "slow");
        let (fast, _) = Anchored::new(1, "fast");
        assert_eq!(block_on(select(fast, slow)), Either::Left("fast"));
    }

    #[test]
    fn a_the_first_argument_wins_a_tie_and_the_second_is_not_polled() {
        let (left, _) = Anchored::new(1, 'l');
        let (right, right_report) = Anchored::new(1, 'r');
        assert_eq!(
            block_on(select(left, right)),
            Either::Left('l'),
            "both are ready in the second poll; the first argument is polled first"
        );
        assert_eq!(
            right_report.polls(),
            1,
            "in the poll in which the first future is ready, the second must not be polled"
        );
    }

    #[test]
    fn a_select_is_lazy_and_drops_the_loser_when_it_completes() {
        let log = DropLog::new();
        let guard = log.guard("loser dropped");
        let (winner, winner_report) = Anchored::new(1, ());
        let loser = async move {
            let _guard = guard;
            pending::<()>().await;
        };
        let race = select(winner, loser);
        assert_eq!(
            winner_report.polls(),
            0,
            "select must not poll anything before it is polled itself"
        );
        let mut race = pin!(race);
        let mut cx = noop_cx();
        assert!(race.as_mut().poll(&mut cx).is_pending());
        assert!(log.events().is_empty(), "nobody has lost yet");
        assert!(race.as_mut().poll(&mut cx).is_ready());
        log.push("select completed");
        assert_eq!(
            log.events(),
            ["loser dropped", "select completed"],
            "when select completes, the future that lost must already be dropped"
        );
    }

    #[test]
    fn a_select_keeps_its_futures_in_place_and_does_not_allocate() {
        let (left, left_report) = Anchored::new(4, 1u8);
        let (right, right_report) = Anchored::new(6, 2u8);
        let mut cx = noop_cx();
        let (output, allocations) = count(|| {
            let mut race = pin!(select(left, right));
            poll_to_end(race.as_mut(), &mut cx, 100).0
        });
        assert_eq!(
            (output, allocations),
            (Either::Left(1), 0),
            "(output, allocations): select must hold both futures inline"
        );
        assert!(
            !left_report.moved() && !right_report.moved(),
            "select moved a future after its first poll"
        );
        assert!(
            left_report.dropped() && right_report.dropped(),
            "both futures are gone once the select future is gone"
        );
    }

    #[test]
    fn a_a_pinned_loser_can_be_resumed() {
        let (slow, report) = Anchored::new(3, "slow result");
        let mut slow = pin!(slow);
        let first_round = block_on(select(slow.as_mut(), ready_now("tick")));
        assert_eq!(first_round, Either::Right("tick"));
        assert!(
            !report.dropped(),
            "select was given `Pin<&mut F>`: dropping that must not drop the future itself"
        );
        assert_eq!(block_on(slow), "slow result");
        assert_eq!(
            report.polls(),
            4,
            "one poll inside select and three more afterwards: the work of the first poll was kept"
        );
        assert!(!report.moved());
    }

    #[test]
    fn a_a_pending_select_is_woken_by_either_side() {
        let (first, second) = (Gate::new(), Gate::new());
        let mut race = pin!(select(first.wait(), second.wait()));
        let (counter, waker) = WakeCounter::new();
        let mut cx = Context::from_waker(&waker);
        assert!(race.as_mut().poll(&mut cx).is_pending());
        assert_eq!(
            (first.polls(), second.polls()),
            (1, 1),
            "(polls of the first, polls of the second): a pending poll must poll both, \
             so that both have the waker"
        );
        second.open();
        assert_eq!(
            counter.count(),
            1,
            "the second future must be able to wake the select"
        );
        assert_eq!(race.as_mut().poll(&mut cx), Poll::Ready(Either::Right(())));
    }

    #[test]
    fn b_cancel_wakes_every_waiter() {
        let token = CancelToken::new();
        let clone = token.clone();
        let mut first = pin!(token.cancelled());
        let mut second = pin!(clone.cancelled());
        let (first_counter, first_waker) = WakeCounter::new();
        let (second_counter, second_waker) = WakeCounter::new();
        assert!(
            first
                .as_mut()
                .poll(&mut Context::from_waker(&first_waker))
                .is_pending()
        );
        assert!(
            second
                .as_mut()
                .poll(&mut Context::from_waker(&second_waker))
                .is_pending()
        );
        assert!(!token.is_cancelled());
        clone.cancel();
        assert!(token.is_cancelled(), "all clones share one flag");
        assert_eq!(
            (first_counter.count(), second_counter.count()),
            (1, 1),
            "cancel must wake every waiting future once"
        );
        token.cancel();
        assert_eq!(
            (first_counter.count(), second_counter.count()),
            (1, 1),
            "a second cancel has nobody left to wake"
        );
        assert!(first.as_mut().poll(&mut noop_cx()).is_ready());
        assert!(second.as_mut().poll(&mut noop_cx()).is_ready());
    }

    #[test]
    fn b_a_cancelled_token_is_ready_at_once() {
        let token = CancelToken::new();
        token.cancel();
        let (counter, waker) = WakeCounter::new();
        let mut wait = pin!(token.cancelled());
        assert!(
            wait.as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_ready(),
            "a wait that starts after cancel must not be pending"
        );
        assert_eq!(
            Arc::strong_count(&counter),
            2,
            "a ready wait must not keep a waker"
        );
    }

    #[test]
    fn b_a_dropped_wait_removes_its_waker() {
        let token = CancelToken::new();
        let (counter, waker) = WakeCounter::new();
        let mut cx = Context::from_waker(&waker);
        // The shape of a select loop: one new wait future per round.
        for _ in 0..1000 {
            let mut wait = pin!(token.cancelled());
            assert!(wait.as_mut().poll(&mut cx).is_pending());
            assert!(wait.as_mut().poll(&mut cx).is_pending());
        }
        assert_eq!(
            Arc::strong_count(&counter),
            2,
            "every dropped wait future must take its waker out of the token; \
             otherwise a select loop makes the token grow without limit"
        );
        token.cancel();
        assert_eq!(
            counter.count(),
            0,
            "no waiter is left, so cancel wakes nobody"
        );
    }

    #[test]
    fn b_a_wait_keeps_only_the_newest_waker() {
        let token = CancelToken::new();
        let mut wait = pin!(token.cancelled());
        let (old, old_waker) = WakeCounter::new();
        let (new, new_waker) = WakeCounter::new();
        assert!(
            wait.as_mut()
                .poll(&mut Context::from_waker(&old_waker))
                .is_pending()
        );
        assert!(
            wait.as_mut()
                .poll(&mut Context::from_waker(&new_waker))
                .is_pending()
        );
        assert_eq!(
            Arc::strong_count(&old),
            2,
            "the waker of the first poll must be released when a newer one replaces it"
        );
        token.cancel();
        assert_eq!(
            (old.count(), new.count()),
            (0, 1),
            "(wakes of the old waker, wakes of the new one)"
        );
    }

    #[test]
    fn b_the_wait_future_owns_its_share_of_the_token() {
        fn spawnable<F: Future<Output = ()> + Send + 'static>(future: F) -> F {
            future
        }
        let token = CancelToken::new();
        let wait = spawnable(token.cancelled());
        let canceller = token.clone();
        drop(token);
        within(
            10,
            "a wait that outlives the token it came from",
            move || {
                let helper = thread::spawn(move || canceller.cancel());
                block_on(wait);
                helper.join().unwrap();
            },
        );
    }

    /// A waker that asks the token a question when it is woken and when it
    /// is dropped. Both need the token's lock.
    struct Probe(CancelToken, Mutex<Sender<&'static str>>);
    impl Wake for Probe {
        fn wake(self: Arc<Self>) {
            self.0.is_cancelled();
            let _ = self.1.lock().unwrap().send("woken");
        }
    }
    impl Drop for Probe {
        fn drop(&mut self) {
            self.0.is_cancelled();
            let _ = self.1.lock().unwrap().send("dropped");
        }
    }

    #[test]
    fn b_wakers_are_woken_and_dropped_after_the_lock_is_released() {
        within(
            10,
            "a waker that uses the token when it is woken or dropped (move wakers \
             out of the critical section before you wake or drop them)",
            || {
                let token = CancelToken::new();
                let (tx, events) = mpsc::channel();
                let probe = || Waker::from(Arc::new(Probe(token.clone(), Mutex::new(tx.clone()))));

                // Dropped by cancel's wake.
                let mut wait = pin!(token.cancelled());
                assert!(
                    wait.as_mut()
                        .poll(&mut Context::from_waker(&probe()))
                        .is_pending()
                );
                // Dropped when the wait future is dropped.
                {
                    let mut dropped_wait = pin!(token.cancelled());
                    assert!(
                        dropped_wait
                            .as_mut()
                            .poll(&mut Context::from_waker(&probe()))
                            .is_pending()
                    );
                }
                assert_eq!(events.try_iter().collect::<Vec<_>>(), ["dropped"]);
                // Dropped when a newer waker replaces it.
                let mut repolled = pin!(token.cancelled());
                assert!(
                    repolled
                        .as_mut()
                        .poll(&mut Context::from_waker(&probe()))
                        .is_pending()
                );
                assert!(repolled.as_mut().poll(&mut noop_cx()).is_pending());
                assert_eq!(events.try_iter().collect::<Vec<_>>(), ["dropped"]);
                token.cancel();
                assert_eq!(events.try_iter().collect::<Vec<_>>(), ["woken", "dropped"]);
            },
        );
    }

    /// One step of a scripted byte source.
    enum Step {
        Chunk(&'static [u8]),
        /// Return `Pending` once (after waking the task).
        Wait,
    }
    use Step::{Chunk, Wait};

    /// A byte source that plays a script and then ends. It counts its polls
    /// and panics if it is polled after it returned `None`.
    struct Script {
        steps: VecDeque<Step>,
        polls: Arc<AtomicUsize>,
        ended: bool,
    }

    fn script(steps: impl IntoIterator<Item = Step>) -> (Script, Arc<AtomicUsize>) {
        let polls = Arc::new(AtomicUsize::new(0));
        let source = Script {
            steps: steps.into_iter().collect(),
            polls: Arc::clone(&polls),
            ended: false,
        };
        (source, polls)
    }

    impl Stream for Script {
        type Item = Vec<u8>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Vec<u8>>> {
            assert!(
                !self.ended,
                "the source was polled again after it returned None"
            );
            self.polls.fetch_add(1, Ordering::SeqCst);
            match self.steps.pop_front() {
                Some(Chunk(bytes)) => Poll::Ready(Some(bytes.to_vec())),
                Some(Wait) => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                None => {
                    self.ended = true;
                    Poll::Ready(None)
                }
            }
        }
    }

    /// Reads every line until the end or the first error.
    fn all_lines(mut reader: LineReader<Script>) -> (Vec<String>, Result<(), TooLong>) {
        block_on(async move {
            let mut lines = Vec::new();
            loop {
                match reader.next_line().await {
                    Ok(Some(line)) => lines.push(line),
                    Ok(None) => return (lines, Ok(())),
                    Err(error) => return (lines, Err(error)),
                }
            }
        })
    }

    #[test]
    fn c_lines_are_cut_at_newlines_whatever_the_chunks_are() {
        let (source, _) = script([
            Chunk(b"he"),
            Chunk(b"llo\nwo"),
            Wait,
            Chunk(b"rld\n"),
            Chunk(b"a\nb\n\nc\r\n"),
            Chunk(b"no newline at the end"),
        ]);
        let (lines, end) = all_lines(LineReader::new(source, 64));
        assert_eq!(
            lines,
            ["hello", "world", "a", "b", "", "c", "no newline at the end"],
            "lines may span chunks, a chunk may hold several lines, an empty line is a line, \
             `\\r\\n` ends a line like `\\n`, and the bytes after the last newline are a line"
        );
        assert_eq!(end, Ok(()));
    }

    #[test]
    fn c_buffered_lines_do_not_touch_the_source() {
        let (source, polls) = script([Chunk(b"one\ntwo\nthree\n")]);
        let mut reader = LineReader::new(source, 64);
        let lines = block_on(async {
            let mut lines = Vec::new();
            for _ in 0..3 {
                lines.push(reader.next_line().await.unwrap().unwrap());
            }
            lines
        });
        assert_eq!(lines, ["one", "two", "three"]);
        assert_eq!(
            polls.load(Ordering::SeqCst),
            1,
            "three lines arrived in one chunk, so the source is polled once"
        );
    }

    #[test]
    fn c_the_end_is_reported_once_the_bytes_are_used_up() {
        let (source, polls) = script([Chunk(b"last\n")]);
        let mut reader = LineReader::new(source, 64);
        let results = block_on(async {
            [
                reader.next_line().await,
                reader.next_line().await,
                reader.next_line().await,
            ]
        });
        assert_eq!(
            results,
            [Ok(Some(String::from("last"))), Ok(None), Ok(None)],
            "after the end, next_line keeps returning Ok(None)"
        );
        assert_eq!(
            polls.load(Ordering::SeqCst),
            2,
            "one poll for the chunk and one for the end; a source that has ended \
             must not be polled again"
        );
    }

    #[test]
    fn c_invalid_utf8_is_replaced() {
        let (source, _) = script([Chunk(b"caf\xff\n")]);
        let (lines, _) = all_lines(LineReader::new(source, 64));
        assert_eq!(lines, ["caf\u{fffd}"]);
    }

    #[test]
    fn c_a_line_may_be_exactly_as_long_as_the_limit() {
        let (source, _) = script([Chunk(b"abcd"), Wait, Chunk(b"\nab\n")]);
        let (lines, end) = all_lines(LineReader::new(source, 4));
        assert_eq!(lines, ["abcd", "ab"], "four bytes fit into a limit of four");
        assert_eq!(end, Ok(()));
    }

    #[test]
    fn c_a_line_over_the_limit_is_an_error_that_stays() {
        let (source, _) = script([Chunk(b"ok\nabcde\nnext\n")]);
        let mut reader = LineReader::new(source, 4);
        let results = block_on(async {
            [
                reader.next_line().await,
                reader.next_line().await,
                reader.next_line().await,
            ]
        });
        assert_eq!(
            results,
            [Ok(Some(String::from("ok"))), Err(TooLong), Err(TooLong)],
            "five bytes before the newline are too long for a limit of four, \
             and the reader does not recover"
        );
    }

    #[test]
    fn c_a_flood_without_newline_is_stopped_early() {
        let flood = std::iter::repeat_n(b"xxxxxxxx".as_slice(), 1000).map(Chunk);
        let (source, polls) = script(flood);
        let (lines, end) = all_lines(LineReader::new(source, 20));
        assert!(lines.is_empty());
        assert_eq!(end, Err(TooLong));
        assert!(
            polls.load(Ordering::SeqCst) <= 3,
            "the reader took {} chunks of 8 bytes although the limit is 20 bytes: \
             check the limit before you ask for more",
            polls.load(Ordering::SeqCst)
        );
    }

    #[test]
    fn c_a_cancelled_call_loses_no_bytes() {
        let (source, _) = script([Chunk(b"he"), Wait, Chunk(b"llo\nwor"), Wait, Chunk(b"ld\n")]);
        let mut reader = LineReader::new(source, 64);
        let mut cx = noop_cx();
        let mut lines = Vec::new();
        let mut cancelled = 0;
        loop {
            // One poll, then the future is dropped: that is what a select
            // does to the branch that loses.
            let mut call = pin!(reader.next_line());
            match call.as_mut().poll(&mut cx) {
                Poll::Ready(Ok(Some(line))) => lines.push(line),
                Poll::Ready(Ok(None)) => break,
                Poll::Ready(Err(TooLong)) => panic!("no line is too long here"),
                Poll::Pending => cancelled += 1,
            }
            assert!(
                cancelled <= 100,
                "100 calls were cancelled and no line has arrived: next_line returns Pending \
                 without asking the source for more bytes"
            );
        }
        assert_eq!(cancelled, 2, "the script makes the reader wait twice");
        assert_eq!(
            lines,
            ["hello", "world"],
            "bytes that were received before a call was cancelled must still be there \
             for the next call: keep the buffer in the LineReader, not in a local \
             variable of next_line"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // A line reader that is NOT cancellation safe: the bytes of a
        // half-read line live in a local variable of the future.
        async fn fragile_line(chunks: &mut VecDeque<Step>) -> String {
            let mut line = Vec::new();
            loop {
                let chunk = std::future::poll_fn(|cx| match chunks.pop_front() {
                    Some(Chunk(bytes)) => Poll::Ready(bytes),
                    Some(Wait) | None => {
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                })
                .await;
                line.extend_from_slice(chunk);
                if line.ends_with(b"\n") {
                    line.pop();
                    return String::from_utf8(line).unwrap();
                }
            }
        }

        // P1. The first call is polled once and then dropped. What does the
        // second call return?
        let mut chunks = VecDeque::from([Chunk(b"he"), Wait, Chunk(b"llo\n")]);
        {
            let mut first_call = pin!(fragile_line(&mut chunks));
            assert!(first_call.as_mut().poll(&mut noop_cx()).is_pending());
        }
        let second_call = block_on(fragile_line(&mut chunks));
        let p1: &str = guess();
        assert_eq!(p1, second_call, "P1: what a cancelled call takes with it");

        // P2. A future holds a guard across an await and is dropped while
        // it is pending. A second future is never polled at all. Which
        // names are in the log afterwards, in order?
        let log = DropLog::new();
        let (held, never_started) = (log.guard("held"), log.guard("never started"));
        let waiting = async move {
            let _held = held;
            pending::<()>().await;
        };
        let unpolled = async move {
            let _guard = never_started;
        };
        {
            let mut waiting = pin!(waiting);
            assert!(waiting.as_mut().poll(&mut noop_cx()).is_pending());
        }
        log.push("after the first drop");
        drop(unpolled);
        let p2: [&str; 3] = guess();
        assert_eq!(
            p2.as_slice(),
            log.events(),
            "P2: cancellation runs destructors"
        );

        // P3. `slow` needs three pending polls before it is ready. It is
        // polled once through a `Pin<&mut _>` that is then dropped, and
        // afterwards it is polled until it is ready. How many polls did it
        // see in total?
        let (slow, report) = Anchored::new(3, ());
        let mut slow = pin!(slow);
        {
            let mut borrowed = slow.as_mut();
            assert!(borrowed.as_mut().poll(&mut noop_cx()).is_pending());
        }
        block_on(slow);
        let p3: usize = guess();
        assert_eq!(p3, report.polls(), "P3: polls of a future that was resumed");
    }
}
