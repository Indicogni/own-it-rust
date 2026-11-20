//! Support code for the exercises. You do not need to edit this crate.
//!
//! - [`guess`] marks a prediction you have not written yet.
//! - [`alloc`] counts heap allocations so tests can check *how* you solved a task.

/// Placeholder for a prediction. Replace the call with your answer before
/// running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

pub mod alloc {
    //! A global allocator that counts allocations per thread.
    //!
    //! A chapter opts in once in its `mod.rs`:
    //!
    //! ```ignore
    //! #[global_allocator]
    //! static ALLOC: practice::alloc::Counting = practice::alloc::Counting;
    //! ```
    //!
    //! Tests then call [`count`]. Counts only cover the calling thread, so
    //! tests running in parallel do not disturb each other. The optimizer may
    //! remove allocations in release builds, so tests only assert upper bounds.

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
    /// reallocations it made on this thread. Requires [`Counting`] to be the
    /// global allocator; otherwise the count is always zero.
    pub fn count<R>(f: impl FnOnce() -> R) -> (R, usize) {
        let before = COUNT.with(Cell::get);
        let was_active = ACTIVE.with(|a| a.replace(true));
        let result = f();
        ACTIVE.with(|a| a.set(was_active));
        (result, COUNT.with(Cell::get) - before)
    }
}

/// Runs `f` on a helper thread and returns its result, or fails the test
/// with a clear message if it has not finished after `secs` seconds.
///
/// Tests use this around code that would otherwise block forever when a
/// solution has a deadlock, never closes a channel, or never wakes a task.
/// `what` names the thing being run and appears in the failure message.
#[track_caller]
pub fn within<R: Send + 'static>(
    secs: u64,
    what: &str,
    f: impl FnOnce() -> R + Send + 'static,
) -> R {
    use std::sync::mpsc::{self, RecvTimeoutError};
    let (tx, rx) = mpsc::channel();
    // The helper thread carries `what` as its name, so a crash on it (for
    // example a stack overflow) names the operation under test.
    let spawned = std::thread::Builder::new()
        .name(what.to_string())
        .spawn(move || {
            let _ = tx.send(f());
        });
    if let Err(error) = spawned {
        panic!("could not start a thread to run {what}: {error}");
    }
    match rx.recv_timeout(std::time::Duration::from_secs(secs)) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "{what} was still running after {secs} s. It probably never finishes: \
             look for a loop that never ends, a function that calls itself without \
             end, a lock that is never released, a channel sender that is never \
             dropped, or a task that is never woken."
        ),
        Err(RecvTimeoutError::Disconnected) => {
            panic!("{what} panicked before finishing; see the panic message above")
        }
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
    #[should_panic(expected = "still running after 1 s")]
    fn within_reports_a_hang() {
        within(1, "sleeper", || {
            std::thread::sleep(std::time::Duration::from_secs(30))
        });
    }

    #[test]
    #[should_panic(expected = "panicked before finishing")]
    fn within_reports_a_panic() {
        within(5, "panicker", || panic!("boom"));
    }
}
