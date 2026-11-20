//! REVIEW 01 — Spaced retrieval: Chapters 17, 21, and 22 inside Chapter 23
//! Run: cargo test --test chapter23 r01_
//!
//! Do this WITHOUT reopening Chapters 17, 21, or 22 or their hints. Struggling
//! to recall is what makes the knowledge stick; look things up only after a
//! real attempt, and note in NOTES.md anything you had to look up.
//!
//! Each task mixes an earlier idea (macro hygiene and evaluation, wakers,
//! UTF-8 boundaries) with this chapter's unsafe contracts. Write a SAFETY
//! comment for every unsafe block.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{RawWaker, RawWakerVTable, Waker};

/// A. (Ch22 macros + Ch23 unchecked access) Expand to an `Option<&T>`:
/// evaluate the slice expression once, then the index expression once, check
/// the bounds, and only then use `get_unchecked` inside an unsafe block.
macro_rules! checked_get {
    ($slice:expr, $index:expr) => {{
        // Bind each argument once, in order. The local names are hygienic:
        // they cannot clash with names in the caller's expressions.
        let slice = $slice;
        let index: usize = $index;
        if index < slice.len() {
            // SAFETY: `index < slice.len()` was checked on the line above.
            Some(unsafe { slice.get_unchecked(index) })
        } else {
            None
        }
    }};
}

/// B. (Ch21 wakers + Ch23 raw pointers) Build a Waker by hand, without the
/// `std::task::Wake` trait. Its data pointer comes from `Arc::into_raw`, and
/// each live Waker owns one strong count of that Arc. `wake` and `wake_by_ref`
/// add 1 to the counter; clone adds an owner; drop (supplied) releases one.
fn counting_waker(wakes: Arc<AtomicUsize>) -> Waker {
    let data = Arc::into_raw(wakes).cast::<()>();
    // SAFETY: `data` comes from `Arc::<AtomicUsize>::into_raw`, and the strong
    // count that `wakes` owned now belongs to this Waker. That is the contract
    // of every function in VTABLE. `AtomicUsize` is Send + Sync, so the Waker
    // may be used from any thread, as `Waker` requires.
    unsafe { Waker::from_raw(RawWaker::new(data, &VTABLE)) }
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_raw, wake_raw, wake_by_ref_raw, drop_raw);

// Contract for every vtable function: `data` came from
// `Arc::<AtomicUsize>::into_raw`, and the calling Waker owns one strong count.

unsafe fn clone_raw(data: *const ()) -> RawWaker {
    // SAFETY: `data` came from `Arc::<AtomicUsize>::into_raw`, and the Waker
    // being cloned keeps the Arc alive during this call.
    unsafe { Arc::increment_strong_count(data.cast::<AtomicUsize>()) };
    RawWaker::new(data, &VTABLE)
}
unsafe fn wake_raw(data: *const ()) {
    // SAFETY: `wake` consumes the Waker, so its strong count is ours to
    // release. Rebuilding the Arc takes that count over; it is released when
    // `wakes` goes out of scope at the end of this function.
    let wakes = unsafe { Arc::from_raw(data.cast::<AtomicUsize>()) };
    wakes.fetch_add(1, Ordering::SeqCst);
}
unsafe fn wake_by_ref_raw(data: *const ()) {
    // SAFETY: the Waker is only borrowed, so its strong count keeps the
    // AtomicUsize alive during this call. A shared reference is enough for
    // `fetch_add`, and no Arc is rebuilt, so no count changes.
    let wakes = unsafe { &*data.cast::<AtomicUsize>() };
    wakes.fetch_add(1, Ordering::SeqCst);
}
// Supplied so an unfinished task cannot panic again while a test unwinds.
unsafe fn drop_raw(data: *const ()) {
    // SAFETY: per the contract above, this Waker owns one strong count.
    drop(unsafe { Arc::from_raw(data.cast::<AtomicUsize>()) });
}

/// C. (Ch17 UTF-8 + Ch23 unchecked slicing) Split `text` at the FIRST `sep`
/// byte, which is removed. Return None if `sep` is not ASCII or does not
/// occur. Build both halves with `str::get_unchecked`, justified by the fact
/// that an ASCII byte is always a char boundary. Borrow; do not allocate.
fn split_once_ascii(text: &str, sep: u8) -> Option<(&str, &str)> {
    if !sep.is_ascii() {
        return None;
    }
    let at = text.bytes().position(|byte| byte == sep)?;
    // SAFETY: `at < text.len()`, so both ranges are in bounds. `text[at]` is
    // an ASCII byte. In UTF-8 a byte below 0x80 is always a complete character
    // and never part of a longer one, so `at` and `at + 1` are both char
    // boundaries, and both halves are valid UTF-8.
    unsafe { Some((text.get_unchecked(..at), text.get_unchecked(at + 1..))) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::cell::RefCell;
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll};

    #[test]
    fn a_checked_get_bounds_and_identity() {
        let values = [10, 20, 30];
        let middle: Option<&i32> = checked_get!(&values, 1);
        assert!(
            std::ptr::eq(middle.unwrap(), &values[1]),
            "the macro must borrow the element in place, not copy it"
        );
        let past: Option<&i32> = checked_get!(&values, 3);
        assert_eq!(past, None);
        let huge: Option<&i32> = checked_get!(&values, usize::MAX);
        assert_eq!(huge, None);
        let empty: Option<&u8> = checked_get!(&[][..], 0);
        assert_eq!(empty, None);
    }

    #[test]
    fn a_checked_get_evaluates_each_argument_once_in_order() {
        let log = RefCell::new(Vec::<&str>::new());
        let values = [String::from("x"), String::from("y")];
        let got: Option<&String> = checked_get!(
            {
                log.borrow_mut().push("slice");
                &values[..]
            },
            {
                log.borrow_mut().push("index");
                1
            }
        );
        assert_eq!(got.map(String::as_str), Some("y"));
        assert_eq!(
            *log.borrow(),
            ["slice", "index"],
            "each argument must be evaluated exactly once, the slice first"
        );
    }

    #[test]
    fn a_c_technique_unchecked_access_after_a_check() {
        let file = include_str!("r01_review.rs");
        for item in ["macro_rules! checked_get", "fn split_once_ascii"] {
            let body = crate::source::body_of(file, item);
            assert!(
                body.contains("get_unchecked"),
                "`{item}` should do its own check and then use `get_unchecked` in an unsafe \
                 block with a SAFETY comment. The checked std methods give the same result \
                 but do not practise writing the proof"
            );
        }
    }

    #[test]
    fn b_wakes_are_counted_and_owners_balanced() {
        let wakes = Arc::new(AtomicUsize::new(0));
        let waker = counting_waker(Arc::clone(&wakes));
        assert_eq!(Arc::strong_count(&wakes), 2, "the Waker owns one count");
        waker.wake_by_ref();
        waker.wake_by_ref();
        assert_eq!(
            wakes.load(Ordering::SeqCst),
            2,
            "wakes counted after two wake_by_ref calls"
        );
        let copy = waker.clone();
        assert_eq!(Arc::strong_count(&wakes), 3, "clone adds an owner");
        drop(copy);
        assert_eq!(Arc::strong_count(&wakes), 2, "drop releases one");
        waker.wake();
        assert_eq!(
            wakes.load(Ordering::SeqCst),
            3,
            "wake() must count one more wake"
        );
        assert_eq!(Arc::strong_count(&wakes), 1, "wake consumes the Waker");
    }

    #[test]
    fn b_a_future_can_wake_through_it() {
        /// Ready on the second poll; wakes itself after the first.
        struct YieldOnce(bool);
        impl Future for YieldOnce {
            type Output = ();
            fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                if self.0 {
                    return Poll::Ready(());
                }
                self.0 = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
        let wakes = Arc::new(AtomicUsize::new(0));
        let waker = counting_waker(Arc::clone(&wakes));
        let mut cx = Context::from_waker(&waker);
        let mut fut = pin!(YieldOnce(false));
        assert!(fut.as_mut().poll(&mut cx).is_pending());
        assert_eq!(
            wakes.load(Ordering::SeqCst),
            1,
            "the future woke itself once through your waker"
        );
        assert!(fut.as_mut().poll(&mut cx).is_ready());
        drop(waker);
        assert_eq!(
            Arc::strong_count(&wakes),
            1,
            "after the Waker is dropped, only the test's own Arc remains"
        );
    }

    #[test]
    fn c_split_respects_char_boundaries() {
        assert_eq!(split_once_ascii("naïve=été", b'='), Some(("naïve", "été")));
        assert_eq!(split_once_ascii("a=b=c", b'='), Some(("a", "b=c")));
        assert_eq!(split_once_ascii("=x", b'='), Some(("", "x")));
        assert_eq!(split_once_ascii("x=", b'='), Some(("x", "")));
        assert_eq!(split_once_ascii("no separator", b'='), None);
        assert_eq!(split_once_ascii("", b'='), None);
    }

    #[test]
    fn c_non_ascii_separators_are_rejected() {
        // 0xA9 is the second byte of "é": splitting there would break UTF-8.
        assert!(split_once_ascii("é", 0xA9).is_none());
        assert!(split_once_ascii("aé b", 0xC3).is_none());
    }

    #[test]
    fn c_split_borrows_without_allocating() {
        let text = String::from("key=value");
        let (parts, allocations) = count(|| split_once_ascii(&text, b'='));
        let (key, value) = parts.unwrap();
        assert_eq!(
            allocations, 0,
            "both halves are borrowed; nothing is copied"
        );
        assert_eq!(
            key.as_ptr(),
            text.as_ptr(),
            "the first half must point at the start of the input"
        );
        assert_eq!(
            value.as_ptr(),
            text[4..].as_ptr(),
            "the second half must point just after the separator"
        );
    }

    /// P. Chapters 17–22 recall: predict each value BEFORE running.
    /// `left` in a failure message is your prediction; `right` is the real value.
    #[test]
    fn p_predictions() {
        // P1. Expression fragments keep their grouping.
        macro_rules! neg {
            ($x:expr) => {
                -$x
            };
        }
        let p1: i32 = 3;
        assert_eq!(p1, neg!(2 - 5), "P1: macro substitution");

        // P2. Polls needed until this async block is ready, and its output.
        struct YieldOnce(bool);
        impl Future for YieldOnce {
            type Output = ();
            fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                if self.0 {
                    return Poll::Ready(());
                }
                self.0 = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
        let mut cx = Context::from_waker(Waker::noop());
        let mut fut = pin!(async {
            YieldOnce(false).await;
            7
        });
        let mut polls = 0;
        let output = loop {
            polls += 1;
            if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                break v;
            }
        };
        let p2: (i32, i32) = (2, 7); // (polls, output)
        assert_eq!(p2, (polls, output), "P2: polling to completion");

        // P3. Byte index of 'v' in "naïve".
        let p3: Option<usize> = Some(4);
        assert_eq!(p3, "naïve".find('v'), "P3: byte offsets");
    }
}
