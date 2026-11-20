//! LESSON 05 — Pin projection by hand
//! Read first: CONCEPTS.md, "Lesson 05". Start after: lesson 02 (lessons 03 and
//! 04 are not needed here).
//! Run: cargo test --manifest-path mastery/m4_async/Cargo.toml l05_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. `poll` receives `Pin<&mut Self>`. Why can you not write
//!    `self.future.poll(cx)` when `future` is a field of type `F`?
//! 2. A finished inner future should be dropped early. Why is
//!    `self.inner.take()` wrong for a pinned `Option<F>`, and what is right?
//!
//! IMPLEMENT the tasks; keep the tests. The fields are given, because the
//! decision "pinned or not" is made per field and is the topic of the lesson.
//! Every `unsafe` block you write needs a `// SAFETY:` comment that argues
//! from the four duties in CONCEPTS.md; `cargo clippy` warns if one is missing.
//! EXPLAIN: For `Fuse<F>`, write the complete safety argument: which fields
//! are structurally pinned, and why each of the four duties holds. Then name
//! one line that would break each duty.
//! MIRI LAB (optional; needs a nightly toolchain with the `miri` component):
//! `cargo +nightly-2026-11-20 miri test --manifest-path mastery/m4_async/Cargo.toml l05_`

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

/// The error of a [`Timeout`] whose deadline came first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elapsed;

/// A. Runs `future` until `deadline` completes.
///
/// Both futures are stored inline: no `Box`, and no `Unpin` bound.
/// - `future` and `deadline` are *structurally pinned*: when the `Timeout`
///   is pinned, so are they.
/// - `polls` is not pinned: it is plain data that is read and written
///   through `&mut`.
pub struct Timeout<F, D> {
    future: F,
    deadline: D,
    polls: u32,
}

impl<F, D> Timeout<F, D> {
    /// Wraps `future`. `deadline` is any future with the output `()`, for
    /// example a `Sleep` from lesson 03.
    pub fn new(future: F, deadline: D) -> Timeout<F, D> {
        Timeout {
            future,
            deadline,
            polls: 0,
        }
    }

    /// How many times this `Timeout` has been polled.
    pub fn polls(&self) -> u32 {
        self.polls
    }

    /// A. The projection: from a pinned `Timeout` to its parts, each with
    /// the access that its field allows.
    fn project(self: Pin<&mut Self>) -> (Pin<&mut F>, Pin<&mut D>, &mut u32) {
        // SAFETY: `future` and `deadline` are structurally pinned, `polls`
        // is not. The four duties hold:
        // 1. Unpin: there is no manual `Unpin` impl, so `Timeout<F, D>` is
        //    `Unpin` only when `F` and `D` are.
        // 2. Drop: `Timeout` has no `Drop` impl, so no destructor of ours
        //    can move a pinned field. The fields are dropped in place.
        // 3. No moves: this module never moves `future` or `deadline` out
        //    of a pinned `Timeout` and never hands out `&mut F` or `&mut D`;
        //    the only access is through the `Pin<&mut _>` returned here.
        //    They are never overwritten or forgotten, so their destructors
        //    run before their memory is reused.
        // 4. Layout: the struct is not `repr(packed)`, so every field is
        //    aligned and is dropped where it is.
        // `get_unchecked_mut` is used only to reach the fields; the
        // `&mut Self` does not escape and nothing is moved through it.
        unsafe {
            let this = self.get_unchecked_mut();
            (
                Pin::new_unchecked(&mut this.future),
                Pin::new_unchecked(&mut this.deadline),
                &mut this.polls,
            )
        }
    }
}

/// A. Count the poll. Poll `future` first: if it is ready, return
/// `Ok(output)` and do not poll `deadline`. Otherwise poll `deadline`: if it
/// is ready, return `Err(Elapsed)`.
impl<F: Future, D: Future<Output = ()>> Future for Timeout<F, D> {
    type Output = Result<F::Output, Elapsed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (future, deadline, polls) = self.project();
        *polls += 1;
        if let Poll::Ready(output) = future.poll(cx) {
            return Poll::Ready(Ok(output));
        }
        if deadline.poll(cx).is_ready() {
            return Poll::Ready(Err(Elapsed));
        }
        Poll::Pending
    }
}

/// B. Remembers that its future has finished. The first `Ready` is passed
/// on; every later poll returns `Pending` without touching the future.
/// (Such a future is called *fused*. A select loop needs it: a branch that
/// has finished must not be polled again.)
///
/// `inner` is structurally pinned as a whole: a pinned `Fuse<F>` gives
/// `Pin<&mut Option<F>>`. When the future finishes it is dropped at once and
/// in place, and `inner` becomes `None`.
pub struct Fuse<F> {
    inner: Option<F>,
}

impl<F> Fuse<F> {
    /// Wraps `future`.
    pub fn new(future: F) -> Fuse<F> {
        Fuse {
            inner: Some(future),
        }
    }

    /// `true` once the inner future has finished.
    pub fn is_terminated(&self) -> bool {
        self.inner.is_none()
    }

    /// B. The projection to the only field.
    fn inner(self: Pin<&mut Self>) -> Pin<&mut Option<F>> {
        // SAFETY: `inner` is structurally pinned. The four duties hold:
        // 1. Unpin: no manual impl; `Fuse<F>: Unpin` only when
        //    `Option<F>: Unpin`, that is, when `F: Unpin`.
        // 2. Drop: `Fuse` has no `Drop` impl.
        // 3. No moves: the option is reached only through this
        //    `Pin<&mut Option<F>>`. `poll` empties it with `Pin::set`, which
        //    drops the old future in place; it never calls `Option::take`,
        //    which would move the future out.
        // 4. Layout: not `repr(packed)`.
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
    }
}

/// B. From `Pin<&mut Option<F>>` you need no more `unsafe`:
/// `Option::as_pin_mut` gives `Option<Pin<&mut F>>`, and `Pin::set` replaces
/// the pinned value after dropping the old one where it stands.
impl<F: Future> Future for Fuse<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let mut inner = self.inner();
        let Some(future) = inner.as_mut().as_pin_mut() else {
            return Poll::Pending;
        };
        let output = ready!(future.poll(cx));
        inner.set(None);
        Poll::Ready(output)
    }
}

/// C. The state machine that the compiler writes for
/// `async { let x = first.await; next(x).await }`, written by hand.
///
/// - `First`: `future` is structurally pinned; `next` is not pinned, it is
///   taken out by value when `future` finishes.
/// - `Second`: `future` is structurally pinned.
/// - `Done`: the output was returned.
///
/// A state change replaces the whole pinned enum with `Pin::set`, so the old
/// state, with its future, is dropped in place.
pub enum Chain<A, B, F> {
    First { future: A, next: Option<F> },
    Second { future: B },
    Done,
}

/// C. `first`, then the future that `next` builds from its output.
pub fn and_then<A, B, F>(first: A, next: F) -> Chain<A, B, F>
where
    A: Future,
    B: Future,
    F: FnOnce(A::Output) -> B,
{
    Chain::First {
        future: first,
        next: Some(next),
    }
}

/// C. Drive the states in order within one call of `poll`: when the first
/// future finishes, build the second one and poll it in the same call.
/// Polling a `Chain` that is `Done` is a bug of the caller: panic with a
/// message that contains the words `polled after`. (A test looks for these
/// words, so that it can tell your panic from any other.)
impl<A, B, F> Future for Chain<A, B, F>
where
    A: Future,
    B: Future,
    F: FnOnce(A::Output) -> B,
{
    type Output = B::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<B::Output> {
        loop {
            // SAFETY: the `&mut Chain` is used only to find the current
            // variant and to reach its fields. Nothing is moved out of a
            // pinned field through it: the `future` fields are only re-pinned
            // below, and `next` is not structurally pinned.
            match unsafe { self.as_mut().get_unchecked_mut() } {
                Chain::First { future, next } => {
                    // SAFETY: `future` is structurally pinned. The duties:
                    // 1. Unpin: no manual impl, so `Chain` is `Unpin` only
                    //    when `A`, `B` and `F` are.
                    // 2. Drop: `Chain` has no `Drop` impl.
                    // 3. No moves: the futures are never moved out of a
                    //    pinned `Chain`. A state is left only through
                    //    `Pin::set` below, which drops the old variant,
                    //    and its future, in place.
                    // 4. Layout: not `repr(packed)`.
                    let output = ready!(unsafe { Pin::new_unchecked(future) }.poll(cx));
                    let next = next
                        .take()
                        .expect("`next` is present until the first future finishes");
                    let second = next(output);
                    self.set(Chain::Second { future: second });
                }
                Chain::Second { future } => {
                    // SAFETY: as above; `future` is structurally pinned.
                    let output = ready!(unsafe { Pin::new_unchecked(future) }.poll(cx));
                    self.set(Chain::Done);
                    return Poll::Ready(output);
                }
                Chain::Done => panic!("a Chain was polled after it returned its output"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::implements;
    use crate::support::alloc::count;
    use crate::support::{Anchored, DropLog, block_on, guess, poll_to_end};
    use std::future::{Ready, pending, ready as ready_now};
    use std::pin::pin;
    use std::task::Waker;

    fn noop_cx() -> Context<'static> {
        Context::from_waker(Waker::noop())
    }

    const MOVED: &str = "a pinned inner future was moved after its first poll: \
                         reach it through a projection, never by value";

    #[test]
    fn a_the_future_wins_when_it_finishes_first() {
        let (future, report) = Anchored::new(2, "done");
        let mut timeout = pin!(Timeout::new(future, pending::<()>()));
        let output = block_on(timeout.as_mut());
        assert_eq!(
            output,
            Ok("done"),
            "the future finished, the deadline never does"
        );
        assert_eq!(
            (timeout.polls(), report.polls()),
            (3, 3),
            "(polls of the Timeout, polls of the inner future): two Pending and one Ready"
        );
        assert!(!report.moved(), "{MOVED}");
    }

    #[test]
    fn a_the_deadline_wins_when_it_finishes_first() {
        let (future, slow) = Anchored::new(10, "too slow");
        let (deadline, clock) = Anchored::new(2, ());
        let mut timeout = pin!(Timeout::new(future, deadline));
        let output = block_on(timeout.as_mut());
        assert_eq!(output, Err(Elapsed), "the deadline finished first");
        assert_eq!(
            (timeout.polls(), slow.polls(), clock.polls()),
            (3, 3, 3),
            "(Timeout, future, deadline): every poll of the Timeout polls both parts"
        );
        assert!(!slow.moved() && !clock.moved(), "{MOVED}");
    }

    #[test]
    fn a_the_future_is_polled_first_and_wins_a_tie() {
        let (future, _) = Anchored::new(1, 7);
        let (deadline, clock) = Anchored::new(1, ());
        let output = block_on(Timeout::new(future, deadline));
        assert_eq!(
            output,
            Ok(7),
            "when both are ready in the same poll, the future's output counts"
        );
        assert_eq!(
            clock.polls(),
            1,
            "the deadline must not be polled in the poll in which the future is ready"
        );
    }

    #[test]
    fn a_timeout_is_inline_and_does_not_allocate() {
        let (future, _) = Anchored::new(3, [0u64; 8]);
        let (deadline, _) = Anchored::new(9, ());
        let parts = size_of_val(&future) + size_of_val(&deadline);
        let timeout = Timeout::new(future, deadline);
        assert!(
            size_of_val(&timeout) <= parts + 2 * size_of::<usize>(),
            "a Timeout must hold both futures inline, next to its counter"
        );
        let (output, allocations) = count(|| block_on(timeout));
        // block_on itself allocates its waker once.
        assert!(
            allocations <= 1,
            "polling a Timeout made {allocations} allocations: do not box the inner futures"
        );
        assert_eq!(output, Ok([0u64; 8]));
    }

    #[test]
    fn a_b_c_unpin_follows_the_pinned_fields() {
        assert!(
            implements!(Timeout<Ready<u8>, Ready<()>>: Unpin),
            "with Unpin parts the wrapper is Unpin: do not add PhantomPinned"
        );
        assert!(
            !implements!(Timeout<Anchored<u8>, Ready<()>>: Unpin),
            "a Timeout around a !Unpin future must not be Unpin: remove the manual Unpin impl"
        );
        assert!(
            !implements!(Timeout<Ready<u8>, Anchored<()>>: Unpin),
            "a Timeout around a !Unpin deadline must not be Unpin"
        );
        assert!(implements!(Fuse<Ready<u8>>: Unpin));
        assert!(
            !implements!(Fuse<Anchored<u8>>: Unpin),
            "a Fuse around a !Unpin future must not be Unpin"
        );
        assert!(
            !implements!(Chain<Anchored<u8>, Ready<u8>, fn(u8) -> Ready<u8>>: Unpin),
            "a Chain with a !Unpin first future must not be Unpin"
        );
        assert!(
            !implements!(Chain<Ready<u8>, Anchored<u8>, fn(u8) -> Anchored<u8>>: Unpin),
            "a Chain with a !Unpin second future must not be Unpin"
        );
    }

    #[test]
    fn b_fuse_passes_the_output_on_once() {
        let (future, report) = Anchored::new(1, 9);
        let mut fuse = pin!(Fuse::new(future));
        let mut cx = noop_cx();
        assert!(!fuse.is_terminated());
        assert_eq!(fuse.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(fuse.as_mut().poll(&mut cx), Poll::Ready(9));
        assert!(fuse.is_terminated(), "after Ready the Fuse is terminated");
        for _ in 0..3 {
            assert_eq!(
                fuse.as_mut().poll(&mut cx),
                Poll::Pending,
                "a terminated Fuse is pending for ever"
            );
        }
        assert_eq!(
            report.polls(),
            2,
            "the inner future must not be polled again after it finished"
        );
    }

    #[test]
    fn b_the_finished_future_is_dropped_at_once_and_in_place() {
        let (future, report) = Anchored::new(1, ());
        let mut fuse = pin!(Fuse::new(future));
        let mut cx = noop_cx();
        assert!(fuse.as_mut().poll(&mut cx).is_pending());
        assert!(!report.dropped());
        assert!(fuse.as_mut().poll(&mut cx).is_ready());
        assert!(
            report.dropped(),
            "the inner future must be dropped when it finishes, not when the Fuse is dropped"
        );
        assert!(
            !report.moved(),
            "the inner future was dropped at another address than the one it was polled at: \
             `Option::take` moves a pinned value; use `Pin::set`"
        );
    }

    #[test]
    fn b_fuse_is_inline_and_does_not_allocate() {
        let (future, _) = Anchored::new(2, 5u8);
        assert!(
            size_of::<Fuse<Anchored<u8>>>() <= size_of::<Option<Anchored<u8>>>(),
            "a Fuse is as large as its optional future"
        );
        let mut fuse = pin!(Fuse::new(future));
        let mut cx = noop_cx();
        let ((_, polls), allocations) = count(|| poll_to_end(fuse.as_mut(), &mut cx, 100));
        assert_eq!(
            (polls, allocations),
            (3, 0),
            "(polls until Ready, allocations)"
        );
    }

    #[test]
    fn c_chain_runs_the_second_future_with_the_first_output() {
        let (first, first_report) = Anchored::new(2, 20);
        let (second_report_tx, second_report_rx) = std::sync::mpsc::channel();
        let chain = and_then(first, move |n: i32| {
            let (second, report) = Anchored::new(1, n + 1);
            second_report_tx.send(report).unwrap();
            second
        });
        let output = block_on(chain);
        let second_report = second_report_rx.recv().unwrap();
        assert_eq!(
            output, 21,
            "the closure gets 20 and the second future gives 21"
        );
        assert_eq!(
            (first_report.polls(), second_report.polls()),
            (3, 2),
            "(polls of the first future, polls of the second)"
        );
        assert!(!first_report.moved() && !second_report.moved(), "{MOVED}");
        assert!(first_report.dropped() && second_report.dropped());
    }

    #[test]
    fn c_the_second_future_starts_in_the_poll_that_finishes_the_first() {
        let (first, _) = Anchored::new(0, ());
        let (second_report_tx, second_report_rx) = std::sync::mpsc::channel();
        let mut chain = pin!(and_then(first, move |()| {
            let (second, report) = Anchored::new(1, "end");
            second_report_tx.send(report).unwrap();
            second
        }));
        let mut cx = noop_cx();
        assert_eq!(chain.as_mut().poll(&mut cx), Poll::Pending);
        let second_report = second_report_rx.recv().unwrap();
        assert_eq!(
            second_report.polls(),
            1,
            "when the first future is ready, the same call of poll must build and poll \
             the second one; returning Pending without that loses the wakeup"
        );
        assert_eq!(chain.as_mut().poll(&mut cx), Poll::Ready("end"));
        assert!(
            matches!(*chain, Chain::Done),
            "after the output was returned the state is Done"
        );
    }

    #[test]
    fn c_each_state_is_dropped_in_place_when_it_is_left() {
        let log = DropLog::new();
        let (first, first_report) = Anchored::new(1, ());
        let guard = log.guard("closure state");
        let watcher = first_report.clone();
        let seen = log.clone();
        let mut chain = pin!(and_then(first, move |()| {
            drop(guard);
            let (second, _) = Anchored::new(1, ());
            let seen = seen.clone();
            async move {
                // The first poll of the second future: the first state must
                // be gone by now.
                if watcher.dropped() {
                    seen.push("first future dropped before the second ran");
                }
                second.await;
            }
        }));
        let mut cx = noop_cx();
        assert!(chain.as_mut().poll(&mut cx).is_pending());
        assert!(chain.as_mut().poll(&mut cx).is_pending());
        assert_eq!(
            log.events(),
            [
                "closure state",
                "first future dropped before the second ran"
            ],
            "leaving the first state must drop its future before the second future is polled"
        );
        assert!(!first_report.moved(), "{MOVED}");
        assert!(chain.as_mut().poll(&mut cx).is_ready());
    }

    #[test]
    fn c_chain_is_inline_and_does_not_allocate() {
        let (first, _) = Anchored::new(1, 1u8);
        let mut chain = pin!(and_then(first, |n: u8| ready_now(n + 1)));
        let mut cx = noop_cx();
        let ((output, _), allocations) = count(|| poll_to_end(chain.as_mut(), &mut cx, 100));
        assert_eq!(
            (output, allocations),
            (2, 0),
            "(output, allocations): a Chain holds its futures inline"
        );
    }

    #[test]
    #[should_panic(expected = "polled after")]
    fn c_polling_a_finished_chain_panics() {
        let mut chain = pin!(and_then(ready_now(1), |n: i32| ready_now(n)));
        let mut cx = noop_cx();
        assert_eq!(chain.as_mut().poll(&mut cx), Poll::Ready(1));
        let _ = chain.as_mut().poll(&mut cx);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. A vector has room for two elements of 32 bytes each and holds
        // one. A second element is inserted in front of it. By how many
        // bytes did the first element move?
        let mut inline: Vec<[u64; 4]> = Vec::with_capacity(2);
        inline.push([1; 4]);
        let before = std::ptr::from_ref(&inline[0]) as usize;
        inline.insert(0, [2; 4]);
        let after = std::ptr::from_ref(&inline[1]) as usize;
        let p1: usize = 32;
        assert_eq!(p1, after - before, "P1: bytes that an inline element moved");

        // P2. The same with boxed elements. By how many bytes did the
        // 32-byte value inside the first box move?
        let mut boxed: Vec<Box<[u64; 4]>> = Vec::with_capacity(2);
        boxed.push(Box::new([1; 4]));
        let before = std::ptr::from_ref(&*boxed[0]) as usize;
        boxed.insert(0, Box::new([2; 4]));
        let after = std::ptr::from_ref(&*boxed[1]) as usize;
        let p2: usize = 0;
        assert_eq!(p2, after - before, "P2: bytes that a boxed value moved");

        // P3. `slot` is a pinned Option that holds a value which logs its
        // name when it is dropped. What does the log hold at the end, in
        // order?
        let log = DropLog::new();
        {
            let mut slot = pin!(Some(log.guard("old value")));
            slot.set(None);
            log.push("after set");
            slot.set(Some(log.guard("new value")));
            log.push("end of scope");
        }
        let p3: [&str; 4] = ["old value", "after set", "end of scope", "new value"];
        assert_eq!(p3.as_slice(), log.events(), "P3: when Pin::set drops");
    }
}
