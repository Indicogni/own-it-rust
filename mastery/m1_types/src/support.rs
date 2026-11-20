//! Support code for the lessons. You do not need to edit this file.
//!
//! - [`guess`] marks a prediction you have not written yet.
//! - [`implements!`](crate::implements) answers "does this type implement
//!   this trait?" with a `bool`, so that a test can report it in words.
//! - [`DropLog`] records the order in which values are dropped.
//! - [`alloc`] counts heap allocations, so that a test can check *how* a task
//!   was solved.

use std::cell::RefCell;
use std::rc::Rc;

/// Placeholder for a prediction. Replace the call with your answer before
/// running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

/// `implements!(Type: Bound)` is `true` when the concrete type `Type`
/// satisfies `Bound`, and `false` otherwise.
///
/// ```
/// use m1_types::implements;
/// use std::rc::Rc;
///
/// assert!(implements!(String: Send));
/// assert!(!implements!(Rc<String>: Send));
/// assert!(implements!(u8: Copy + Send + Sync));
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

/// A list of names, in the order in which they were written.
///
/// - `log.noisy("name")` makes a value that writes `"name"` into the log at
///   the moment it is dropped.
/// - `log.note("text")` writes `"text"` into the log immediately.
/// - `log.entries()` returns everything written so far, oldest first.
#[derive(Default, Clone)]
pub struct DropLog(Rc<RefCell<Vec<&'static str>>>);

/// A value that writes its name into a [`DropLog`] when it is dropped.
pub struct Noisy {
    name: &'static str,
    log: Rc<RefCell<Vec<&'static str>>>,
}

impl DropLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn noisy(&self, name: &'static str) -> Noisy {
        Noisy {
            name,
            log: Rc::clone(&self.0),
        }
    }

    pub fn note(&self, text: &'static str) {
        self.0.borrow_mut().push(text);
    }

    pub fn entries(&self) -> Vec<&'static str> {
        self.0.borrow().clone()
    }
}

impl Noisy {
    /// The name this value will write when it is dropped.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl Drop for Noisy {
    fn drop(&mut self) {
        self.log.borrow_mut().push(self.name);
    }
}

pub mod alloc {
    //! A global allocator that counts allocations per thread.
    //!
    //! The unit tests of this package install it in `lib.rs`. A program in
    //! `tests/cases/` that wants to count installs it for itself:
    //!
    //! ```
    //! use m1_types::support::alloc::{Counting, count};
    //!
    //! #[global_allocator]
    //! static ALLOC: Counting = Counting;
    //!
    //! let (text, allocations) = count(|| String::with_capacity(8));
    //! assert!(allocations <= 1);
    //! assert_eq!(text, "");
    //! ```
    //!
    //! Counts cover the calling thread only. The optimizer may remove
    //! allocations in release builds, so tests assert upper bounds.

    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static ACTIVE: Cell<bool> = const { Cell::new(false) };
        static COUNT: Cell<usize> = const { Cell::new(0) };
    }

    /// Forwards to the system allocator and counts allocations and growth.
    pub struct Counting;

    fn record() {
        let _ = ACTIVE.try_with(|active| {
            if active.get() {
                let _ = COUNT.try_with(|count| count.set(count.get() + 1));
            }
        });
    }

    // SAFETY: every method forwards unchanged to `System`, which upholds the
    // GlobalAlloc contract. Recording touches only const-initialized
    // thread-locals without destructors, so it never allocates or reenters.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record();
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which is forwarded unchanged.
            unsafe { System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record();
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which is forwarded unchanged.
            unsafe { System.alloc_zeroed(layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            record();
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which is forwarded unchanged.
            unsafe { System.realloc(ptr, layout, new_size) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which is forwarded unchanged.
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    /// Runs `f` and returns its result with the number of allocations and
    /// reallocations it made on this thread. Requires [`Counting`] to be the
    /// global allocator; otherwise the count is always zero.
    pub fn count<R>(f: impl FnOnce() -> R) -> (R, usize) {
        let before = COUNT.with(Cell::get);
        let was_active = ACTIVE.with(|active| active.replace(true));
        let result = f();
        ACTIVE.with(|active| active.set(was_active));
        (result, COUNT.with(Cell::get) - before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implements_reports_bounds() {
        assert!(implements!(String: Send));
        assert!(!implements!(Rc<u8>: Send));
        assert!(implements!(&'static str: Copy));
        assert!(!implements!(String: Copy));
        assert!(implements!(str: Sync));
        assert!(!implements!(std::cell::Cell<u8>: Sync));
        assert!(implements!(u8: Copy + Send + Sync + 'static));
    }

    #[test]
    fn drop_log_records_in_order() {
        let log = DropLog::new();
        {
            let _a = log.noisy("a");
            let _b = log.noisy("b");
            log.note("end of block");
        }
        assert_eq!(log.entries(), ["end of block", "b", "a"]);
    }

    #[test]
    fn count_sees_allocations_on_this_thread() {
        let (boxed, allocations) = alloc::count(|| Box::new([0_u8; 64]));
        assert_eq!(boxed.len(), 64);
        assert!(allocations <= 1);
        let (_, none) = alloc::count(|| 2 + 2);
        assert_eq!(none, 0);
    }

    #[test]
    #[should_panic(expected = "PREDICT")]
    fn guess_panics_until_replaced() {
        let _: u8 = guess();
    }
}
