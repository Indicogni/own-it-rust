//! The building blocks for every lesson. You do not need to edit this file.
//!
//! **Import atomics, cells, threads, `Arc`, `Mutex` and `Condvar` from here,
//! never from `std` directly.**
//!
//! In a normal build this module hands you the standard library's own types.
//! With `--cfg loom` (the `cargo model` alias) it hands you loom's versions,
//! which have the same methods but report every operation to the model checker.
//! Code that uses `std::sync::atomic` directly still compiles under loom, but
//! loom cannot see what it does, so the model tests would check nothing.
//!
//! Three things differ from `std`:
//!
//! - [`UnsafeCell`] has `with` and `with_mut` instead of `get`. Each call marks
//!   the start and the end of one access, which is what loom needs in order to
//!   notice two threads touching the same cell without synchronization.
//! - [`thread`] holds only the items that the lessons need. In a normal build
//!   its `park` also counts calls, so that a test can see that a blocked
//!   thread really went to sleep.
//! - Under loom, `new` functions are not `const`, so the lessons never put
//!   these types in a `static`.

#[cfg(not(loom))]
mod imp {
    pub use std::hint;
    pub use std::sync::atomic;
    pub use std::sync::{Arc, Condvar, Mutex, MutexGuard};

    /// `std::cell::UnsafeCell` with the closure-based interface of loom's cell.
    ///
    /// ```
    /// use m3_concurrency::sync::UnsafeCell;
    ///
    /// let cell = UnsafeCell::new(5);
    /// // SAFETY: nothing else accesses the cell during either call.
    /// cell.with_mut(|p| unsafe { *p += 1 });
    /// assert_eq!(cell.with(|p| unsafe { *p }), 6);
    /// ```
    #[derive(Debug, Default)]
    #[repr(transparent)]
    pub struct UnsafeCell<T>(std::cell::UnsafeCell<T>);

    impl<T> UnsafeCell<T> {
        pub const fn new(value: T) -> Self {
            UnsafeCell(std::cell::UnsafeCell::new(value))
        }

        /// Calls `f` with a pointer for reading the value.
        ///
        /// Getting the pointer is safe; using it is not. The caller must make
        /// sure that no thread writes the value while it is being read.
        #[inline]
        pub fn with<R>(&self, f: impl FnOnce(*const T) -> R) -> R {
            f(self.0.get())
        }

        /// Calls `f` with a pointer for writing the value.
        ///
        /// The caller must make sure that no other access to the value
        /// happens while it is being written.
        #[inline]
        pub fn with_mut<R>(&self, f: impl FnOnce(*mut T) -> R) -> R {
            f(self.0.get())
        }

        pub fn into_inner(self) -> T {
            self.0.into_inner()
        }
    }

    pub mod thread {
        //! The parts of `std::thread` that the lessons use.

        pub use std::thread::{JoinHandle, Thread, ThreadId, current, spawn, yield_now};

        use std::cell::RefCell;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        thread_local! {
            static PARK_COUNTER: RefCell<Option<Arc<AtomicUsize>>> = const { RefCell::new(None) };
        }

        /// `std::thread::park`. If the calling thread registered a counter
        /// with [`count_parks_in`], the counter goes up by one first.
        pub fn park() {
            PARK_COUNTER.with(|slot| {
                if let Some(counter) = slot.borrow().as_ref() {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            });
            std::thread::park();
        }

        /// From now on, every [`park`] call of the current thread adds one to
        /// `counter`. Tests use this to wait until a thread is really blocked.
        pub fn count_parks_in(counter: &Arc<AtomicUsize>) {
            PARK_COUNTER.with(|slot| *slot.borrow_mut() = Some(Arc::clone(counter)));
        }
    }
}

#[cfg(loom)]
mod imp {
    pub use loom::cell::UnsafeCell;
    pub use loom::hint;
    pub use loom::sync::atomic;
    pub use loom::sync::{Arc, Condvar, Mutex, MutexGuard};

    pub mod thread {
        //! The parts of `loom::thread` that the lessons use.

        pub use loom::thread::{ThreadId, park, yield_now};

        /// loom's `Thread` handle, with one change to `unpark`.
        ///
        /// In loom 0.7.3 a thread can lose the processor only immediately
        /// before an atomic operation, a lock operation or a similar step,
        /// and `unpark` is not such a step. Loom also treats `unpark` as if
        /// it synchronized the two threads at once. Together this means that
        /// "store the state, then unpark" would always run as one step, and
        /// the woken thread would never be seen reading the state in between.
        /// This wrapper lets the other threads run first, so that the models
        /// also cover that case.
        #[derive(Clone, Debug)]
        pub struct Thread(loom::thread::Thread);

        impl Thread {
            pub fn id(&self) -> ThreadId {
                self.0.id()
            }

            pub fn unpark(&self) {
                loom::thread::yield_now();
                self.0.unpark();
            }
        }

        pub fn current() -> Thread {
            Thread(loom::thread::current())
        }

        /// loom's `JoinHandle`, with the behaviour of the standard library's:
        /// `join` returns `Err` if the thread panicked.
        pub struct JoinHandle<T>(loom::thread::JoinHandle<std::thread::Result<T>>);

        impl<T> JoinHandle<T> {
            pub fn join(self) -> std::thread::Result<T> {
                self.0.join()?
            }
        }

        /// `loom::thread::spawn` with the signature of `std::thread::spawn`
        /// (loom itself does not ask for `Send`). A panic in the new thread
        /// is caught and reported through `join`, as in the standard library.
        pub fn spawn<F, T>(f: F) -> JoinHandle<T>
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static,
        {
            JoinHandle(loom::thread::spawn(move || {
                crate::support::catch_in_model(f)
            }))
        }
    }
}

pub use imp::*;

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn cell_reads_and_writes() {
        let cell = UnsafeCell::new(String::from("a"));
        // SAFETY: this test is the only user of the cell.
        cell.with_mut(|p| unsafe { (&mut *p).push('b') });
        // SAFETY: as above.
        assert_eq!(cell.with(|p| unsafe { (&*p).len() }), 2);
        assert_eq!(cell.into_inner(), "ab");
    }

    #[test]
    fn park_is_counted_per_thread() {
        let counter = std::sync::Arc::new(AtomicUsize::new(0));
        thread::count_parks_in(&counter);
        // A token is already waiting, so `park` returns at once.
        thread::current().unpark();
        thread::park();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}

#[cfg(all(test, loom))]
mod model {
    use super::atomic::{AtomicUsize, Ordering};
    use super::{Arc, thread};

    /// The example model from CONCEPTS.md, "How the model tests work".
    #[test]
    fn concepts_example_two_increments() {
        loom::model(|| {
            let hits = Arc::new(AtomicUsize::new(0));
            let worker = {
                let hits = Arc::clone(&hits);
                thread::spawn(move || {
                    hits.fetch_add(1, Ordering::Relaxed);
                })
            };
            hits.fetch_add(1, Ordering::Relaxed);
            worker.join().unwrap();
            assert_eq!(hits.load(Ordering::Relaxed), 2);
        });
    }
}
