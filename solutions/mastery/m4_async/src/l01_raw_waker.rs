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
        self.notify();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.notify();
    }
}

/// A. Build a waker from the `Wake` implementation above.
pub fn waker_via_trait(signal: Arc<Signal>) -> Waker {
    Waker::from(signal)
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
    let data = Arc::into_raw(signal).cast::<()>();
    // SAFETY: `data` comes from `Arc::<Signal>::into_raw`, which is what the
    // four functions in `SIGNAL_VTABLE` expect, and it carries one strong
    // reference that the new waker now owns. The functions keep the
    // RawWaker contract: `clone` adds one reference and returns the same
    // table, `wake` and `drop` each give one reference back, `wake_by_ref`
    // gives none back. `Signal` is `Send + Sync`, so the functions may be
    // called from any thread, as `Waker: Send + Sync` requires.
    unsafe { Waker::from_raw(RawWaker::new(data, &SIGNAL_VTABLE)) }
}

/// B. `clone`: the caller keeps its reference; the returned `RawWaker` owns
/// a new one.
///
/// # Safety
/// `data` must come from `Arc::<Signal>::into_raw`, and the reference that
/// belongs to the calling waker must still be alive.
unsafe fn signal_clone(data: *const ()) -> RawWaker {
    // SAFETY: by this function's contract `data` is a live
    // `Arc<Signal>` pointer, so its count can be raised by one. The new
    // reference belongs to the RawWaker that is returned.
    unsafe { Arc::increment_strong_count(data.cast::<Signal>()) };
    RawWaker::new(data, &SIGNAL_VTABLE)
}

/// B. `wake`: consumes the waker. Notify, and give back its reference.
///
/// # Safety
/// As for `signal_clone`. The caller gives up its reference: after this call
/// it must not use `data` again and must not call `signal_drop`.
unsafe fn signal_wake(data: *const ()) {
    // SAFETY: `data` is a live `Arc<Signal>` pointer and the caller hands
    // over its reference, so rebuilding the `Arc` takes exactly that
    // reference. It is released when `signal` goes out of scope.
    let signal = unsafe { Arc::from_raw(data.cast::<Signal>()) };
    signal.notify();
}

/// B. `wake_by_ref`: notify; the waker stays usable and keeps its reference.
///
/// # Safety
/// As for `signal_clone`.
unsafe fn signal_wake_by_ref(data: *const ()) {
    // SAFETY: `data` points to a `Signal` inside a live `Arc`, and the
    // calling waker keeps its reference for the whole call, so the shared
    // borrow is valid. Nothing is taken from the count.
    let signal = unsafe { &*data.cast::<Signal>() };
    signal.notify();
}

/// B. `drop`: give back the waker's reference without notifying.
///
/// # Safety
/// As for `signal_wake`.
unsafe fn signal_drop(data: *const ()) {
    // SAFETY: `data` is a live `Arc<Signal>` pointer and the caller hands
    // over its reference; the rebuilt `Arc` releases it at once.
    drop(unsafe { Arc::from_raw(data.cast::<Signal>()) });
}

/// C. A waker needs no `Arc` at all. This one wakes a counter that lives for
/// the whole program: each wake adds 1 to it. The data pointer is the address
/// of the counter, cloning copies the pointer, and dropping does nothing.
/// Add your own `static` vtable and its functions below.
pub fn static_waker(counter: &'static AtomicUsize) -> Waker {
    let data = std::ptr::from_ref(counter).cast::<()>();
    // SAFETY: `data` points to an `AtomicUsize` that lives for the rest of
    // the program, so every function in `COUNTER_VTABLE` may read it at any
    // time, also after every waker is gone. No function owns anything, so
    // `clone` can copy the pointer and `drop` has nothing to release.
    // `AtomicUsize` is `Sync`, so the calls may come from any thread.
    unsafe { Waker::from_raw(RawWaker::new(data, &COUNTER_VTABLE)) }
}

static COUNTER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(counter_clone, counter_wake, counter_wake, counter_drop);

/// # Safety
/// `data` must point to an `AtomicUsize` with the `'static` lifetime.
unsafe fn counter_clone(data: *const ()) -> RawWaker {
    RawWaker::new(data, &COUNTER_VTABLE)
}

/// Serves as both `wake` and `wake_by_ref`: there is no reference to consume.
///
/// # Safety
/// `data` must point to an `AtomicUsize` with the `'static` lifetime.
unsafe fn counter_wake(data: *const ()) {
    // SAFETY: by this function's contract `data` points to an
    // `AtomicUsize` that is never freed.
    let counter = unsafe { &*data.cast::<AtomicUsize>() };
    counter.fetch_add(1, Ordering::SeqCst);
}

/// # Safety
/// None: the pointer is not used.
unsafe fn counter_drop(_data: *const ()) {}

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
        let p1: usize = 2;
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
        let p2: usize = 2;
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
        let p3: (usize, usize) = (2, 2);
        assert_eq!(
            p3,
            (Arc::strong_count(&bell), bell.0.load(Ordering::SeqCst)),
            "P3: (references, wakes) after wake_by_ref"
        );

        // P4. The waker is dropped without being woken. Count and wakes?
        drop(waker);
        let p4: (usize, usize) = (1, 2);
        assert_eq!(
            p4,
            (Arc::strong_count(&bell), bell.0.load(Ordering::SeqCst)),
            "P4: (references, wakes) after dropping the last waker"
        );
    }
}
