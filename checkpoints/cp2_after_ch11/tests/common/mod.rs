//! Helpers shared by the test files of this checkpoint.
//!
//! You do not need to read or edit this file to take the checkpoint. It
//! gives the tests these tools:
//!
//! - [`measure`] counts the heap allocations a piece of code makes, so that a
//!   test can check *how* a function works and not only what it returns;
//! - [`within`] runs code with a time limit, so that a loop that never ends
//!   becomes a failed test with a message instead of a test run that hangs;
//! - a memory limit: if the tests ever hold more than [`MEMORY_LIMIT`] bytes
//!   at once, the test program stops. Without the limit, a loop that never
//!   ends and adds to a collection each time could fill the memory of your
//!   machine.

// Each test file uses a different part of this module.
#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

/// The most memory, in bytes, that a test program may hold at one time.
pub const MEMORY_LIMIT: usize = 1 << 30;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static LIMIT_REPORTED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

/// Forwards to the system allocator. While [`measure`] is running on the
/// current thread, it also counts the requests.
pub struct Counting;

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn record(bytes: usize) {
    let _ = ACTIVE.try_with(|active| {
        if active.get() {
            let _ = CALLS.try_with(|calls| calls.set(calls.get() + 1));
            let _ = BYTES.try_with(|total| total.set(total.get() + bytes));
        }
    });
}

/// Adds `bytes` to the memory in use. Returns false, and takes nothing, when
/// that would pass the limit.
fn reserve(bytes: usize) -> bool {
    if LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes <= MEMORY_LIMIT {
        return true;
    }
    LIVE.fetch_sub(bytes, Ordering::Relaxed);
    if !LIMIT_REPORTED.swap(true, Ordering::Relaxed) {
        // Writing to standard error does not allocate.
        let _ = std::io::stderr().write_all(
            b"\nMEMORY LIMIT: the tests held more than 1 GiB at once. A loop is probably \
              adding to a collection without ever ending. The test program stops now.\n\n",
        );
    }
    false
}

// SAFETY: every method forwards unchanged to `System`, which upholds the
// GlobalAlloc contract, or returns null without allocating, which the
// contract allows as the report of a failed allocation. The accounting
// touches only atomics and const-initialized thread-locals without
// destructors, so it never allocates or reenters.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        if !reserve(layout.size()) {
            return std::ptr::null_mut();
        }
        // SAFETY: the caller upholds this method's GlobalAlloc contract,
        // which we forward unchanged.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        if !reserve(layout.size()) {
            return std::ptr::null_mut();
        }
        // SAFETY: as in `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(new_size);
        // The old block stays valid when this returns null, so its size is
        // released only after the new size was granted.
        if !reserve(new_size) {
            return std::ptr::null_mut();
        }
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: as in `alloc`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: as in `alloc`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

/// What a piece of code asked the heap for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heap {
    /// Number of allocations and reallocations (a `Vec` that grows counts
    /// once for each time it grows).
    pub calls: usize,
    /// Sum of the sizes that were requested, in bytes.
    pub bytes: usize,
}

/// Runs `f` and reports the heap requests it made on this thread.
///
/// Only the calling thread is counted, so tests that run in parallel do not
/// disturb each other. The optimizer may remove allocations in release
/// builds, so tests only assert upper bounds.
pub fn measure<R>(f: impl FnOnce() -> R) -> (R, Heap) {
    let calls = CALLS.with(Cell::get);
    let bytes = BYTES.with(Cell::get);
    let was_active = ACTIVE.with(|active| active.replace(true));
    let result = f();
    ACTIVE.with(|active| active.set(was_active));
    let heap = Heap {
        calls: CALLS.with(Cell::get) - calls,
        bytes: BYTES.with(Cell::get) - bytes,
    };
    (result, heap)
}

/// Runs `f` on a helper thread and returns its result, or fails the test
/// with a clear message if it has not finished after `secs` seconds.
#[track_caller]
pub fn within<R: Send + 'static>(
    secs: u64,
    what: &str,
    f: impl FnOnce() -> R + Send + 'static,
) -> R {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "{what} was still running after {secs} s. It is probably stuck: look for \
             a loop whose condition never becomes false, a lock that is never \
             released, or a thread or task that waits for something that never comes."
        ),
        Err(RecvTimeoutError::Disconnected) => {
            panic!("{what} panicked before finishing; see the panic message above")
        }
    }
}

/// True when `part` is a slice of `whole`: the same bytes in memory, not a
/// copy with equal contents.
pub fn is_slice_of(part: &str, whole: &str) -> bool {
    let start = whole.as_ptr() as usize;
    let end = start + whole.len();
    let part_start = part.as_ptr() as usize;
    part_start >= start && part_start + part.len() <= end
}
