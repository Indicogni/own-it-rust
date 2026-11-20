//! Support code for the lessons. You do not need to edit this file.
//!
//! - [`guess`] marks a prediction you have not written yet.
//! - [`within`] turns a test that would hang into a test that fails with a message.
//! - [`implements!`](crate::implements) answers "does this type implement
//!   this trait?" with a `bool`, so that a test can report it in words.
//! - [`alloc`] counts heap allocations and frees, so that tests can check *how*
//!   you solved a task.
//! - [`DropCount`] and [`Counted`] check that a value is dropped exactly once.
//! - `model`, `outcomes` and `Tracked` (only with `--cfg loom`) are the helpers
//!   of the model tests.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Placeholder for a prediction. Replace the call with your answer before
/// running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

/// Runs `f` on a helper thread and returns its result, or fails the test with a
/// clear message if it has not finished after `secs` seconds.
///
/// Tests use this around code that would otherwise block forever when a
/// solution has a deadlock or a lost wake-up. `what` names the thing being run
/// and appears in the failure message. The time limit is only a safety net:
/// no test in this module passes or fails because of how fast something ran.
#[track_caller]
pub fn within<R: Send + 'static>(
    secs: u64,
    what: &str,
    f: impl FnOnce() -> R + Send + 'static,
) -> R {
    use std::sync::mpsc::{self, RecvTimeoutError};
    // Miri interprets the program and is far slower than native code, and its
    // clock advances with the work done. Give it much more room.
    let secs = if cfg!(miri) { secs * 600 } else { secs };
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(std::time::Duration::from_secs(secs)) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "{what} was still running after {secs} s. It is probably blocked forever: \
             look for a lock that is never released, a thread that is parked and never \
             unparked, or a condition variable that nobody notifies."
        ),
        Err(RecvTimeoutError::Disconnected) => {
            panic!("{what} panicked before finishing; see the panic message above")
        }
    }
}

/// `implements!(Type: Bound)` is `true` when the concrete type `Type`
/// satisfies `Bound`, and `false` otherwise.
///
/// ```
/// use m3_concurrency::implements;
/// use std::cell::Cell;
/// use std::rc::Rc;
///
/// assert!(implements!(String: Send + Sync));
/// assert!(implements!(Cell<u8>: Send));
/// assert!(!implements!(Cell<u8>: Sync));
/// assert!(!implements!(Rc<u8>: Send));
/// ```
///
/// A bound that is not met is normally a compile error, and a test cannot
/// describe a compile error. This macro turns the question into a value.
///
/// How it works: `Probe<T>` has an inherent associated constant `YES` that
/// exists only when `T` meets the bound, and a trait constant `YES` that
/// always exists and is `false`. A path such as `<Probe<T>>::YES` prefers the
/// inherent constant when there is one, and falls back to the trait.
/// The type must be written out in full; the macro cannot be used on a
/// generic parameter, because the choice is made where the macro is written.
#[macro_export]
macro_rules! implements {
    ($ty:ty : $($bound:tt)+) => {{
        struct Probe<T: ?Sized>(::core::marker::PhantomData<T>);
        trait Fallback {
            const YES: bool = false;
        }
        impl<T: ?Sized> Fallback for Probe<T> {}
        #[allow(dead_code)]
        impl<T: ?Sized + $($bound)+> Probe<T> {
            const YES: bool = true;
        }
        <Probe<$ty>>::YES
    }};
}

pub mod alloc {
    //! A global allocator that counts allocations and frees per thread.
    //!
    //! `lib.rs` installs it for test builds. Tests then call [`count`] or
    //! [`frees`]. The numbers only cover the calling thread, so tests that run
    //! in parallel do not disturb each other. The optimizer may remove
    //! allocations in release builds, so tests assert upper bounds for
    //! allocations.

    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static ACTIVE: Cell<bool> = const { Cell::new(false) };
        static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
        static FREES: Cell<usize> = const { Cell::new(0) };
    }

    /// Forwards to the system allocator and counts what passes through.
    pub struct Counting;

    fn record(counter: &'static std::thread::LocalKey<Cell<usize>>) {
        let _ = ACTIVE.try_with(|active| {
            if active.get() {
                let _ = counter.try_with(|count| count.set(count.get() + 1));
            }
        });
    }

    // SAFETY: every method forwards unchanged to `System`, which upholds the
    // GlobalAlloc contract. Recording touches only const-initialized
    // thread-locals without destructors, so it never allocates or reenters.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record(&ALLOCATIONS);
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which we forward unchanged.
            unsafe { System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record(&ALLOCATIONS);
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which we forward unchanged.
            unsafe { System.alloc_zeroed(layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            record(&ALLOCATIONS);
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which we forward unchanged.
            unsafe { System.realloc(ptr, layout, new_size) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            record(&FREES);
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which we forward unchanged.
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    fn measure<R>(
        counter: &'static std::thread::LocalKey<Cell<usize>>,
        f: impl FnOnce() -> R,
    ) -> (R, usize) {
        let before = counter.with(Cell::get);
        let was_active = ACTIVE.with(|a| a.replace(true));
        let result = f();
        ACTIVE.with(|a| a.set(was_active));
        (result, counter.with(Cell::get) - before)
    }

    /// Runs `f` and returns its result with the number of allocations and
    /// reallocations it made on this thread.
    pub fn count<R>(f: impl FnOnce() -> R) -> (R, usize) {
        measure(&ALLOCATIONS, f)
    }

    /// Runs `f` and returns its result with the number of heap blocks it
    /// freed on this thread.
    pub fn frees<R>(f: impl FnOnce() -> R) -> (R, usize) {
        measure(&FREES, f)
    }
}

/// Counts how many [`Counted`] values made from it have been dropped.
///
/// ```
/// use m3_concurrency::support::DropCount;
///
/// let drops = DropCount::new();
/// let a = drops.value(1);
/// let b = drops.value(2);
/// assert_eq!(a.id, 1);
/// drop(a);
/// assert_eq!(drops.get(), 1);
/// drop(b);
/// assert_eq!(drops.get(), 2);
/// ```
///
/// The counter lives for the rest of the program. A `Counted` value therefore
/// owns nothing: if a wrong solution drops one twice, the test reports "2
/// drops" instead of crashing on a double free.
#[derive(Clone, Copy)]
pub struct DropCount(&'static AtomicUsize);

/// A value that reports to its [`DropCount`] when it is dropped.
#[derive(Debug)]
pub struct Counted {
    /// A number that lets a test tell the values apart.
    pub id: usize,
    drops: &'static AtomicUsize,
}

/// Keeps every counter reachable, so that leak checkers do not report them.
static COUNTERS: Mutex<Vec<&'static AtomicUsize>> = Mutex::new(Vec::new());

impl DropCount {
    pub fn new() -> Self {
        let counter: &'static AtomicUsize = Box::leak(Box::new(AtomicUsize::new(0)));
        COUNTERS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(counter);
        DropCount(counter)
    }

    /// A new value with the given `id` that counts its own drop here.
    pub fn value(&self, id: usize) -> Counted {
        Counted { id, drops: self.0 }
    }

    /// How many values made by [`DropCount::value`] have been dropped so far.
    pub fn get(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

impl Default for DropCount {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[cfg(loom)]
pub use self::checker::{Tracked, catch_in_model, model, model_bounded, outcomes};

#[cfg(loom)]
mod checker {
    //! Helpers for the model tests. They exist only with `--cfg loom`.

    use super::DropCount;
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    // (`Arc` and `Mutex` here are the real ones: `outcomes` uses them outside
    // the model, where loom's versions do not work.)

    use std::any::Any;
    use std::cell::RefCell;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    thread_local! {
        /// The message of the first panic in the model that is running on
        /// this test thread. Loom runs all the threads of a model on the one
        /// operating-system thread that started it, so they all see this.
        static FIRST_FAILURE: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    fn message_of(payload: &(dyn Any + Send)) -> String {
        if let Some(text) = payload.downcast_ref::<&str>() {
            (*text).to_owned()
        } else if let Some(text) = payload.downcast_ref::<String>() {
            text.clone()
        } else {
            "a thread panicked".to_owned()
        }
    }

    /// Runs `f`. If it panics, remembers the message as the model's failure
    /// and returns the panic payload instead of unwinding further.
    ///
    /// Why: when a panic unwinds out of a loom thread, loom tears the model
    /// down while other threads still hold loom objects, and that second
    /// failure aborts the whole test program. Stopping the panic here lets
    /// the other threads of this run finish; [`model`] then reports the
    /// first failure in the normal way.
    pub fn catch_in_model<R>(f: impl FnOnce() -> R) -> Result<R, Box<dyn Any + Send>> {
        catch_unwind(AssertUnwindSafe(f)).inspect_err(|payload| {
            let message = message_of(payload.as_ref());
            FIRST_FAILURE.with(|slot| {
                slot.borrow_mut().get_or_insert(message);
            });
        })
    }

    fn first_failure() -> Option<String> {
        FIRST_FAILURE.with(|slot| slot.borrow().clone())
    }

    fn report(message: &str) -> ! {
        panic!(
            "the model found a run that fails: {message}\n\
             (the first panic message above shows where it happened)"
        );
    }

    fn check(builder: loom::model::Builder, f: impl Fn() + Sync + Send + 'static) {
        FIRST_FAILURE.with(|slot| slot.borrow_mut().take());
        builder.check(move || {
            // A run failed, and all its threads have finished. Stop the
            // model here, at the start of the next run, where no other
            // thread exists yet.
            if let Some(message) = first_failure() {
                report(&message);
            }
            let _ = catch_in_model(&f);
        });
        // The failing run was the last one.
        if let Some(message) = first_failure() {
            report(&message);
        }
    }

    /// Runs `f` once for every interleaving and every stale read that the
    /// memory model allows, and fails if any of those runs fails.
    pub fn model(f: impl Fn() + Sync + Send + 'static) {
        check(loom::model::Builder::new(), f);
    }

    /// Like [`model`], but explores only the runs in which the scheduler
    /// takes the processor away from a running thread at most `preemptions`
    /// times. Models with three threads use this, because the number of runs
    /// grows very fast with each thread. Most real bugs need only one or two
    /// such switches.
    pub fn model_bounded(preemptions: usize, f: impl Fn() + Sync + Send + 'static) {
        let mut builder = loom::model::Builder::new();
        builder.preemption_bound = Some(preemptions);
        check(builder, f);
    }

    /// Runs `f` under [`model`] and returns every distinct result that some
    /// run produced, in ascending order.
    pub fn outcomes<T>(f: impl Fn() -> T + Sync + Send + 'static) -> Vec<T>
    where
        T: Ord + Send + 'static,
    {
        let seen = Arc::new(Mutex::new(BTreeSet::new()));
        let sink = Arc::clone(&seen);
        model(move || {
            let result = f();
            sink.lock().unwrap().insert(result);
        });
        let mut seen = seen.lock().unwrap();
        std::mem::take(&mut *seen).into_iter().collect()
    }

    /// A value whose every access the model checker can see.
    ///
    /// A model test sends a `Tracked` value through your primitive. Creating
    /// it, [`Tracked::set`] and dropping it count as writes; [`Tracked::get`]
    /// counts as a read. If two of these accesses happen on different threads
    /// and your primitive did not establish a happens-before relation between
    /// them, loom stops with "Causality violation". That is how a model test
    /// notices an ordering that is too weak, whatever fields you chose.
    pub struct Tracked<T> {
        cell: loom::cell::UnsafeCell<T>,
        drops: Option<&'static AtomicUsize>,
    }

    // SAFETY: `get` only reads the value and `set` needs `&mut self`, so the
    // cell adds no shared mutation: `Tracked<T>` is as thread-safe as `T`.
    // The cell is there only so that loom sees each access.
    unsafe impl<T: Send> Send for Tracked<T> {}
    // SAFETY: see above; sharing `&Tracked<T>` only allows reads of `T`.
    unsafe impl<T: Send + Sync> Sync for Tracked<T> {}

    impl<T: Copy> Tracked<T> {
        /// A tracked value. Creation counts as a write by the calling thread.
        pub fn new(value: T) -> Self {
            Tracked {
                cell: loom::cell::UnsafeCell::new(value),
                drops: None,
            }
        }

        /// A tracked value that also reports its drop to `drops`.
        pub fn counted(value: T, drops: &DropCount) -> Self {
            Tracked {
                cell: loom::cell::UnsafeCell::new(value),
                drops: Some(drops.0),
            }
        }

        /// Reads the value. Loom records a read by the calling thread.
        pub fn get(&self) -> T {
            // SAFETY: the pointer comes from a live cell, and `&self` rules
            // out a `set` at the same time.
            self.cell.with(|p| unsafe { *p })
        }

        /// Replaces the value. Loom records a write by the calling thread.
        pub fn set(&mut self, value: T) {
            // SAFETY: the pointer comes from a live cell, and `&mut self`
            // gives exclusive access.
            self.cell.with_mut(|p| unsafe { *p = value });
        }
    }

    impl<T> std::fmt::Debug for Tracked<T> {
        /// Does not print the value: reading it here would count as an
        /// access by whichever thread formats a failure message.
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Tracked(..)")
        }
    }

    impl<T> Drop for Tracked<T> {
        fn drop(&mut self) {
            // Dropping counts as a write, so loom also checks that the drop
            // is ordered after every other access. (Not while a panic is
            // unwinding: a second failure there would abort the program.)
            if !std::thread::panicking() {
                self.cell.with_mut(|_| ());
            }
            if let Some(drops) = self.drops {
                drops.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn within_returns_the_value() {
        assert_eq!(within(5, "addition", || 2 + 2), 4);
    }

    #[test]
    #[should_panic(expected = "panicked before finishing")]
    fn within_reports_a_panic() {
        within(5, "panicker", || panic!("boom"));
    }

    #[test]
    fn implements_reports_bounds() {
        assert!(implements!(String: Send));
        assert!(!implements!(Rc<u8>: Send));
        assert!(implements!(Cell<u8>: Send));
        assert!(!implements!(Cell<u8>: Sync));
        assert!(implements!(u8: Copy + Send + Sync + 'static));
    }

    #[test]
    fn alloc_counts_allocations_and_frees() {
        // `black_box` hides the pointer from the optimizer. Without it, a
        // release build may remove the allocation and the free together.
        let (boxed, allocations) = alloc::count(|| std::hint::black_box(Box::new(7_u64)));
        assert_eq!(allocations, 1, "creating a Box makes one heap allocation");
        let ((), frees) = alloc::frees(|| drop(boxed));
        assert_eq!(frees, 1, "dropping a Box frees one heap block");
    }

    #[test]
    fn drop_count_counts() {
        let drops = DropCount::new();
        let a = drops.value(1);
        assert_eq!(drops.get(), 0);
        drop(a);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    #[should_panic(expected = "PREDICT")]
    fn guess_panics_until_replaced() {
        let _: u8 = guess();
    }
}
