//! Support code for the lessons. You do not need to edit this file.
//!
//! - [`guess`] marks a prediction you have not written yet.
//! - [`alloc`] measures heap use, so that tests can check a budget.
//! - [`within`] turns a test that would hang into a test that fails with a message.

// The size and layout predictions in this module are written for 64-bit
// targets. On another target this line stops the build with a clear message,
// instead of letting correct answers fail later.
const _: () = assert!(
    size_of::<usize>() == 8,
    "module M7 assumes a 64-bit target: its size and layout predictions use 8-byte pointers"
);

/// Placeholder for a prediction. Replace the call with your answer before
/// running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

// This module implements `GlobalAlloc`, which is an `unsafe` trait. It is the
// only place in the package that needs `unsafe`.
#[allow(unsafe_code)]
pub mod alloc {
    //! A global allocator that measures heap use per thread.
    //!
    //! The library installs it for its own unit tests only (see `lib.rs`).
    //! Benchmarks use the system allocator, because the counting here would
    //! slow down exactly the thing a benchmark measures. A program that wants
    //! the numbers opts in, as `examples/usage.rs` does:
    //!
    //! ```ignore
    //! #[global_allocator]
    //! static ALLOC: m7_perf::support::alloc::Counting = m7_perf::support::alloc::Counting;
    //! ```
    //!
    //! All numbers cover the calling thread only, so tests that run in
    //! parallel do not disturb each other. For the same reason, measure
    //! *inside* a closure that runs on another thread, not around it.
    //!
    //! The numbers are the calls that reach this allocator. A compiler is
    //! allowed to remove an allocation whose memory is never used, so a
    //! release build could report fewer calls than a debug build. The tests
    //! therefore assert upper bounds, and where a prediction needs an exact
    //! number, the value passes through `std::hint::black_box`.

    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    /// What one run of [`measure`] asked of the heap.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Usage {
        /// Calls that handed out a block: `alloc`, `alloc_zeroed` and `realloc`.
        pub allocations: usize,
        /// The sizes of those blocks, added up. A `Vec` that grows by doubling
        /// asks for about twice the bytes it ends up using; one made with the
        /// right capacity asks for them once.
        pub bytes: usize,
        /// The largest number of bytes that were allocated during the run and
        /// not yet freed, at any one moment. Memory that was allocated before
        /// the run does not count, even if the run frees it.
        pub peak: usize,
    }

    struct Tally {
        active: Cell<bool>,
        allocations: Cell<usize>,
        bytes: Cell<usize>,
        /// Bytes allocated minus bytes freed since the run began. It goes
        /// below zero when the run frees memory that is older than the run.
        live: Cell<isize>,
        peak: Cell<isize>,
    }

    thread_local! {
        static TALLY: Tally = const {
            Tally {
                active: Cell::new(false),
                allocations: Cell::new(0),
                bytes: Cell::new(0),
                live: Cell::new(0),
                peak: Cell::new(0),
            }
        };
    }

    /// Forwards to the system allocator and records what passes through.
    pub struct Counting;

    /// Records one block of `new` bytes that replaces a block of `old` bytes
    /// (`old` is 0 for a fresh allocation).
    fn handed_out(old: usize, new: usize) {
        // `try_with` instead of `with`: the allocator also runs while a thread
        // is being torn down, when its thread-locals may be gone already.
        let _ = TALLY.try_with(|tally| {
            if tally.active.get() {
                tally
                    .allocations
                    .set(tally.allocations.get().saturating_add(1));
                tally.bytes.set(tally.bytes.get().saturating_add(new));
                // A block is never larger than `isize::MAX` bytes, so these
                // conversions are exact.
                let live = tally
                    .live
                    .get()
                    .saturating_add(new as isize)
                    .saturating_sub(old as isize);
                tally.live.set(live);
                tally.peak.set(tally.peak.get().max(live));
            }
        });
    }

    fn freed(size: usize) {
        let _ = TALLY.try_with(|tally| {
            if tally.active.get() {
                tally
                    .live
                    .set(tally.live.get().saturating_sub(size as isize));
            }
        });
    }

    // SAFETY: `GlobalAlloc` requires that the methods behave like an
    // allocator and never unwind. Every method passes its arguments unchanged
    // to `System`, which fulfils the contract, and returns what `System`
    // returns; so every pointer, size and alignment that callers see is one
    // that `System` produced for exactly that request.
    //
    // The counting in between cannot break the contract:
    // - it does not allocate and does not call back into the allocator. It
    //   only reads and writes `Cell`s inside one `thread_local!` value that is
    //   const-initialized and has no destructor. On the targets this course
    //   supports (64-bit Linux, macOS and Windows, which have native
    //   thread-local storage) such a value needs no lazy setup and no
    //   registration, so touching it is a plain memory access. (On macOS the
    //   system may set up a thread's storage on the first access, with its
    //   own `malloc`. That call does not go through this type, so it cannot
    //   come back here.) On a target that emulates thread-locals with keys
    //   that Rust allocates through the global allocator this would not hold,
    //   and this allocator must not be used there;
    // - it cannot panic: the arithmetic saturates, `Cell::get`/`set` do not
    //   panic, and `try_with` reports a destroyed thread-local as an error
    //   that is ignored.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            handed_out(0, layout.size());
            // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc`
            // for `layout` (non-zero size); it is forwarded unchanged.
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            handed_out(0, layout.size());
            // SAFETY: as for `alloc`; the layout is forwarded unchanged.
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            handed_out(layout.size(), new_size);
            // SAFETY: the caller guarantees that `ptr` came from this
            // allocator with `layout`, and that `new_size` is valid for it.
            // Every block of this allocator is a block of `System`, so the
            // same guarantees hold for the forwarded call.
            unsafe { System.realloc(ptr, layout, new_size) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            freed(layout.size());
            // SAFETY: the caller guarantees that `ptr` came from this
            // allocator with `layout`; every such block is a block of
            // `System` with that layout.
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    /// The saved numbers of an enclosing [`measure`] call. Dropping it puts
    /// them back and adds the inner run to them, also when the closure panics.
    struct Restore {
        active: bool,
        allocations: usize,
        bytes: usize,
        live: isize,
        peak: isize,
    }

    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = TALLY.try_with(|tally| {
                tally.active.set(self.active);
                tally
                    .allocations
                    .set(self.allocations.saturating_add(tally.allocations.get()));
                tally
                    .bytes
                    .set(self.bytes.saturating_add(tally.bytes.get()));
                let inner_peak = self.live.saturating_add(tally.peak.get());
                tally.peak.set(self.peak.max(inner_peak));
                tally.live.set(self.live.saturating_add(tally.live.get()));
            });
        }
    }

    /// The numbers of the innermost running [`measure`] call so far. All zero
    /// when no measurement is running on this thread.
    pub fn so_far() -> Usage {
        TALLY.with(|tally| {
            if tally.active.get() {
                Usage {
                    allocations: tally.allocations.get(),
                    bytes: tally.bytes.get(),
                    peak: tally.peak.get().max(0) as usize,
                }
            } else {
                Usage::default()
            }
        })
    }

    /// Runs `f` and returns its result with the heap use of this thread
    /// during the run. Requires [`Counting`] to be the global allocator;
    /// otherwise every number is zero.
    pub fn measure<R>(f: impl FnOnce() -> R) -> (R, Usage) {
        let restore = TALLY.with(|tally| Restore {
            active: tally.active.replace(true),
            allocations: tally.allocations.replace(0),
            bytes: tally.bytes.replace(0),
            live: tally.live.replace(0),
            peak: tally.peak.replace(0),
        });
        let result = f();
        let usage = so_far();
        drop(restore);
        (result, usage)
    }

    /// Runs `f` and returns its result with the number of allocations and
    /// reallocations it made on this thread.
    pub fn count<R>(f: impl FnOnce() -> R) -> (R, usize) {
        let (result, usage) = measure(f);
        (result, usage.allocations)
    }
}

/// Runs `f` on a helper thread and returns its result, or fails the test with
/// a clear message if it has not finished after `secs` seconds.
///
/// `what` names the thing being run and appears in the failure message. Heap
/// use is measured per thread, so call [`alloc::measure`] inside `f`.
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
            "{what} was still running after {secs} s. It is probably in a loop that never \
             ends: check the condition that is meant to stop it."
        ),
        Err(RecvTimeoutError::Disconnected) => {
            panic!("{what} panicked before finishing; see the panic message above")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::alloc::{Usage, count, measure, so_far};
    use super::within;
    use std::hint::black_box;

    #[test]
    fn measure_reports_calls_bytes_and_peak() {
        let (_, usage) = measure(|| {
            let first: Vec<u8> = black_box(Vec::with_capacity(100));
            drop(first);
            let second: Vec<u8> = black_box(Vec::with_capacity(40));
            let third: Vec<u8> = black_box(Vec::with_capacity(30));
            drop((second, third));
        });
        assert_eq!(
            usage,
            Usage {
                allocations: 3,
                bytes: 170,
                peak: 100
            }
        );
    }

    #[test]
    fn growth_counts_the_new_block() {
        let (_, usage) = measure(|| {
            let mut v: Vec<u8> = black_box(Vec::with_capacity(16));
            v.extend_from_slice(&[0; 16]);
            v.reserve_exact(16);
            black_box(v);
        });
        assert_eq!(
            usage,
            Usage {
                allocations: 2,
                bytes: 16 + 32,
                peak: 32
            }
        );
    }

    #[test]
    fn nested_measurements_add_up() {
        let (inner, outer) = measure(|| {
            let keep: Vec<u8> = black_box(Vec::with_capacity(10));
            let (_, inner) = measure(|| {
                let v: Vec<u8> = black_box(Vec::with_capacity(20));
                assert_eq!(so_far().allocations, 1);
                drop(v);
            });
            drop(keep);
            inner
        });
        assert_eq!(
            inner,
            Usage {
                allocations: 1,
                bytes: 20,
                peak: 20
            }
        );
        assert_eq!(
            outer,
            Usage {
                allocations: 2,
                bytes: 30,
                peak: 30
            }
        );
    }

    #[test]
    fn freeing_older_memory_does_not_underflow() {
        let old: Vec<u8> = black_box(Vec::with_capacity(64));
        let (_, usage) = measure(|| {
            drop(old);
            let v: Vec<u8> = black_box(Vec::with_capacity(8));
            drop(v);
        });
        assert_eq!(usage.allocations, 1);
        assert_eq!(
            usage.peak, 0,
            "live bytes never rose above the starting point"
        );
    }

    #[test]
    fn nothing_is_counted_outside_a_measurement() {
        let v: Vec<u8> = black_box(Vec::with_capacity(8));
        assert_eq!(so_far(), Usage::default());
        drop(v);
        assert_eq!(count(|| ()).1, 0);
    }

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
}
