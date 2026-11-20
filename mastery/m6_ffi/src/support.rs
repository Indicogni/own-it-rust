//! Support code for the lessons. You do not need to edit this file.
//!
//! - [`guess`] marks a prediction you have not written yet.
//! - [`within`] turns a test that would hang into a test that fails with a message.
//! - [`alloc`] counts heap allocations made by Rust code on the calling thread.
//! - [`implements!`](crate::implements) answers "does this type implement
//!   this trait?" with a `bool`.

/// Placeholder for a prediction. Replace the call with your answer before
/// running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

/// Runs `f` on a helper thread and returns its result, or fails the test with a
/// clear message if it has not finished after `secs` seconds.
///
/// `what` names the thing being run and appears in the failure message.
#[track_caller]
pub fn within<R: Send + 'static>(
    secs: u64,
    what: &str,
    f: impl FnOnce() -> R + Send + 'static,
) -> R {
    use std::sync::mpsc::{self, RecvTimeoutError};
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(std::time::Duration::from_secs(secs)) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "{what} was still running after {secs} s. It is probably blocked forever: \
             look for a lock that is never released or a thread that is never joined."
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
/// use m6_ffi::implements;
///
/// assert!(implements!(String: Send));
/// assert!(!implements!(*mut u8: Send));
/// assert!(!implements!(std::rc::Rc<u8>: Send + Sync));
/// ```
///
/// A bound that is not met is normally a compile error, and a test cannot
/// report a compile error in words. This macro turns the question into a
/// value. It works because an inherent associated constant wins over a trait
/// constant of the same name: `Probe<T>` has the inherent `YES = true` only
/// when `T` meets the bound, and otherwise the path falls back to the trait's
/// `YES = false`. The type must be written out in full; the macro cannot be
/// used on a generic parameter.
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
    //! A global allocator that counts allocations per thread.
    //!
    //! The crate installs it for test builds only (see `lib.rs`). Tests call
    //! [`count`]. Only allocations made through Rust's global allocator are
    //! counted: `malloc` calls inside the C library are invisible here.
    //! The optimizer may remove allocations in release builds, so tests only
    //! assert upper bounds.

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
            // which we forward unchanged.
            unsafe { System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record();
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which we forward unchanged.
            unsafe { System.alloc_zeroed(layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            record();
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which we forward unchanged.
            unsafe { System.realloc(ptr, layout, new_size) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: the caller upholds this method's GlobalAlloc contract,
            // which we forward unchanged.
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    /// Runs `f` and returns its result with the number of allocations and
    /// reallocations that Rust code made on this thread meanwhile.
    pub fn count<R>(f: impl FnOnce() -> R) -> (R, usize) {
        let before = COUNT.with(Cell::get);
        let was_active = ACTIVE.with(|a| a.replace(true));
        let result = f();
        ACTIVE.with(|a| a.set(was_active));
        (result, COUNT.with(Cell::get) - before)
    }
}

#[cfg(test)]
mod tests {
    use super::within;

    #[test]
    fn within_returns_the_value() {
        assert_eq!(within(5, "addition", || 2 + 2), 4);
    }

    #[test]
    fn implements_tells_send_from_not_send() {
        assert!(implements!(Vec<u8>: Send + Sync));
        assert!(!implements!(*const u8: Send));
        assert!(!implements!(std::cell::Cell<u8>: Sync));
    }
}
