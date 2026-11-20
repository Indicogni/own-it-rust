//! LESSON 01 — RawWaker and its vtable, by hand
//! Read first: CONCEPTS.md, "Lesson 01". Start after: Chapter 21 (all lessons).
//! Run: cargo test --manifest-path mastery/m4_async/Cargo.toml l01_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. A `Waker` is two words. Which two, and which of them says how to clone it?
//! 2. `wake(self)` consumes the waker and `wake_by_ref(&self)` does not. What must
//!    each one do to the reference count of the task behind the waker?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Write the contract of the four vtable functions as a table: who owns
//! the reference before the call, and who owns it after. Then say why the vtable
//! in this lesson is a `static` and not a `const`.
//! MIRI LAB (optional; needs a nightly toolchain with the `miri` component): run
//! `cargo +nightly-2026-11-20 miri test --manifest-path mastery/m4_async/Cargo.toml l01_`.
//! Miri runs the tests in an interpreter that reports a use after free, a
//! double drop, or a leak. The ordinary tests can miss all three.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{RawWaker, RawWakerVTable, Wake, Waker};

/// The thing that gets woken in this lesson: it counts its wakes.
#[derive(Debug, Default)]
pub struct Signal {
    wakes: AtomicUsize,
}

impl Signal {
    /// A new signal that has not been woken yet.
    pub fn new() -> Arc<Signal> {
        Arc::new(Signal::default())
    }

    /// How many times a waker of this signal was woken.
    pub fn wakes(&self) -> usize {
        self.wakes.load(Ordering::SeqCst)
    }

    fn notify(&self) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
    }
}

/// A. The safe way. Implement `Wake` so that each wake calls `notify` once.
/// Implement `wake_by_ref` too: the default version clones the `Arc` only to
/// drop it again.
impl Wake for Signal {
    fn wake(self: Arc<Self>) {
        todo!("A: count one wake")
    }

    fn wake_by_ref(self: &Arc<Self>) {
        todo!("A: count one wake without touching the reference count")
    }
}

/// A. Build a waker from the `Wake` implementation above.
pub fn waker_via_trait(signal: Arc<Signal>) -> Waker {
    todo!("A: one conversion")
}

/// B. The table of the four functions for a waker whose data pointer came
/// from `Arc::<Signal>::into_raw`.
///
/// It is a `static`, so there is exactly one table at one address, and
/// `Waker::will_wake` can compare two wakers by comparing pointers.
static SIGNAL_VTABLE: RawWakerVTable =
    RawWakerVTable::new(signal_clone, signal_wake, signal_wake_by_ref, signal_drop);

/// B. Build the same waker without the `Wake` trait. The waker takes over the
/// one reference that `signal` holds; it must not allocate.
pub fn waker_by_hand(signal: Arc<Signal>) -> Waker {
    todo!("B: turn the Arc into a raw pointer and wrap it; write the SAFETY comment")
}

/// B. `clone`: the caller keeps its reference; the returned `RawWaker` owns
/// a new one.
///
/// # Safety
/// `data` must come from `Arc::<Signal>::into_raw`, and the reference that
/// belongs to the calling waker must still be alive.
unsafe fn signal_clone(data: *const ()) -> RawWaker {
    todo!("B: add one reference, return the same data pointer and the same table")
}

/// B. `wake`: consumes the waker. Notify, and give back its reference.
///
/// # Safety
/// As for `signal_clone`. The caller gives up its reference: after this call
/// it must not use `data` again and must not call `signal_drop`.
unsafe fn signal_wake(data: *const ()) {
    todo!("B: notify, then release the reference that this waker owned")
}

/// B. `wake_by_ref`: notify; the waker stays usable and keeps its reference.
///
/// # Safety
/// As for `signal_clone`.
unsafe fn signal_wake_by_ref(data: *const ()) {
    todo!("B: notify without changing the reference count")
}

/// B. `drop`: give back the waker's reference without notifying.
///
/// # Safety
/// As for `signal_wake`.
unsafe fn signal_drop(data: *const ()) {
    todo!("B: release the reference that this waker owned")
}

/// C. A waker needs no `Arc` at all. This one wakes a counter that lives for
/// the whole program: each wake adds 1 to it. The data pointer is the address
/// of the counter, cloning copies the pointer, and dropping does nothing.
/// Add your own `static` vtable and its functions below.
pub fn static_waker(counter: &'static AtomicUsize) -> Waker {
    todo!("C: a second vtable; no reference counting at all")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::alloc::count;
    use crate::support::{guess, within};
    use std::sync::Weak;

    #[test]
    fn a_trait_waker_counts_each_wake() {
        let signal = Signal::new();
        let waker = waker_via_trait(Arc::clone(&signal));
        waker.wake_by_ref();
        waker.wake_by_ref();
        assert_eq!(signal.wakes(), 2, "each wake_by_ref must count once");
        let clone = waker.clone();
        clone.wake();
        assert_eq!(
            signal.wakes(),
            3,
            "a wake through a clone reaches the same signal"
        );
        waker.wake();
        assert_eq!(signal.wakes(), 4, "wake (by value) must count once");
    }

    #[test]
    fn b_hand_waker_counts_each_wake() {
        let signal = Signal::new();
        let waker = waker_by_hand(Arc::clone(&signal));
        waker.wake_by_ref();
        waker.wake_by_ref();
        assert_eq!(signal.wakes(), 2, "each wake_by_ref must count once");
        let clone = waker.clone();
        clone.wake();
        assert_eq!(
            signal.wakes(),
            3,
            "a wake through a clone reaches the same signal"
        );
        waker.wake();
        assert_eq!(signal.wakes(), 4, "wake (by value) must count once");
    }

    #[test]
    fn b_every_function_keeps_the_reference_count_balanced() {
        let signal = Signal::new();
        let waker = waker_by_hand(Arc::clone(&signal));
        assert_eq!(
            Arc::strong_count(&signal),
            2,
            "the waker owns the one reference that was passed in: one for `signal`, one for the waker"
        );
        let second = waker.clone();
        assert_eq!(
            Arc::strong_count(&signal),
            3,
            "clone must add exactly one reference"
        );
        second.wake_by_ref();
        assert_eq!(
            Arc::strong_count(&signal),
            3,
            "wake_by_ref must not give a reference back: the waker is still alive"
        );
        second.wake();
        assert_eq!(
            Arc::strong_count(&signal),
            2,
            "wake consumes the waker, so it must release that waker's reference"
        );
        drop(waker);
        assert_eq!(
            Arc::strong_count(&signal),
            1,
            "drop must release the waker's reference"
        );
        assert_eq!(signal.wakes(), 2, "drop must not notify");
    }

    #[test]
    fn b_nothing_is_leaked() {
        let signal = Signal::new();
        let weak: Weak<Signal> = Arc::downgrade(&signal);
        let waker = waker_by_hand(signal);
        let clones: Vec<Waker> = (0..5).map(|_| waker.clone()).collect();
        for (i, clone) in clones.into_iter().enumerate() {
            if i % 2 == 0 {
                clone.wake();
            } else {
                clone.wake_by_ref();
            }
        }
        assert!(
            weak.upgrade().is_some(),
            "the signal was freed although a waker is still alive: \
             a function gave back a reference that it did not own"
        );
        drop(waker);
        assert!(
            weak.upgrade().is_none(),
            "the signal is still alive after its last waker was dropped: \
             a clone was not paired with exactly one wake or drop"
        );
    }

    #[test]
    fn b_hand_waker_is_the_arc_pointer_and_does_not_allocate() {
        let signal = Signal::new();
        let (waker, allocations) = count(|| waker_by_hand(Arc::clone(&signal)));
        assert_eq!(
            allocations, 0,
            "waker_by_hand must reuse the Arc's allocation, not box anything"
        );
        assert_eq!(
            waker.data(),
            Arc::as_ptr(&signal).cast::<()>(),
            "the data pointer must be the pointer from Arc::into_raw"
        );
        assert!(
            std::ptr::eq(waker.vtable(), &SIGNAL_VTABLE),
            "waker_by_hand must build the waker from SIGNAL_VTABLE, not through the Wake trait"
        );
        let ((), allocations) = count(|| {
            let second = waker.clone();
            second.wake_by_ref();
            second.wake();
        });
        assert_eq!(
            allocations, 0,
            "cloning and waking must only change the reference count"
        );
        let second = waker.clone();
        assert!(
            waker.will_wake(&second),
            "a clone must carry the same data pointer and the same vtable address; \
             return `&SIGNAL_VTABLE` from clone"
        );
        assert!(
            !waker.will_wake(&waker_by_hand(Signal::new())),
            "wakers of two different signals must not look equal"
        );
    }

    #[test]
    fn b_hand_waker_works_from_another_thread() {
        let signal = Signal::new();
        let waker = waker_by_hand(Arc::clone(&signal));
        let clone = waker.clone();
        within(10, "waking from a second thread", move || {
            clone.wake_by_ref();
            clone.wake();
        });
        assert_eq!(signal.wakes(), 2, "both wakes from the other thread count");
        assert_eq!(
            Arc::strong_count(&signal),
            2,
            "the clone that moved to the other thread was consumed by wake"
        );
    }

    #[test]
    fn c_static_waker_counts_without_owning() {
        static HITS: AtomicUsize = AtomicUsize::new(0);
        let (waker, allocations) = count(|| static_waker(&HITS));
        assert_eq!(allocations, 0, "a waker for a static needs no allocation");
        assert_eq!(
            waker.data(),
            std::ptr::from_ref(&HITS).cast::<()>(),
            "the data pointer must be the address of the counter"
        );
        waker.wake_by_ref();
        let second = waker.clone();
        assert!(
            waker.will_wake(&second),
            "a clone must carry the same data pointer and the same vtable address"
        );
        second.wake();
        drop(waker);
        assert_eq!(
            HITS.load(Ordering::SeqCst),
            2,
            "one wake_by_ref and one wake; clone and drop must not count"
        );
        // The counter outlives every waker, so a new waker still works.
        static_waker(&HITS).wake();
        assert_eq!(HITS.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn c_static_wakers_of_two_counters_differ() {
        static FIRST: AtomicUsize = AtomicUsize::new(0);
        static SECOND: AtomicUsize = AtomicUsize::new(0);
        let first = static_waker(&FIRST);
        let second = static_waker(&SECOND);
        assert!(
            !first.will_wake(&second),
            "two counters have two addresses, so their wakers differ"
        );
        second.wake();
        assert_eq!(
            (FIRST.load(Ordering::SeqCst), SECOND.load(Ordering::SeqCst)),
            (0, 1),
            "(wakes of the first counter, wakes of the second)"
        );
    }

    /// A wake target for the predictions, so that they do not need your code.
    struct Bell(AtomicUsize);
    impl Wake for Bell {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. How large is a `Waker`, measured in pointers?
        let p1: usize = guess();
        assert_eq!(
            p1,
            size_of::<Waker>() / size_of::<usize>(),
            "P1: the size of a Waker in machine words"
        );

        // P2. `bell` is one reference. A waker is made from a clone of it, the
        // waker is cloned once, and the clone is woken by value. How many
        // strong references are there now?
        let bell = Arc::new(Bell(AtomicUsize::new(0)));
        let waker = Waker::from(Arc::clone(&bell));
        let second = waker.clone();
        second.wake();
        let p2: usize = guess();
        assert_eq!(
            p2,
            Arc::strong_count(&bell),
            "P2: references after wake by value"
        );

        // P3. `Bell` implements only `wake`, so `wake_by_ref` is the default
        // method of the trait. During the call below the reference count
        // changes. What is the count after the call, and how many wakes has
        // the bell counted in total (P2 included)?
        waker.wake_by_ref();
        let p3: (usize, usize) = guess();
        assert_eq!(
            p3,
            (Arc::strong_count(&bell), bell.0.load(Ordering::SeqCst)),
            "P3: (references, wakes) after wake_by_ref"
        );

        // P4. The waker is dropped without being woken. Count and wakes?
        drop(waker);
        let p4: (usize, usize) = guess();
        assert_eq!(
            p4,
            (Arc::strong_count(&bell), bell.0.load(Ordering::SeqCst)),
            "P4: (references, wakes) after dropping the last waker"
        );
    }
}
