//! Original guided exercises; implement TODOs and answer the prompts.
#![allow(
    unused_variables,
    unused_mut,
    unused_imports,
    dead_code,
    unreachable_code
)]
mod c01_code_review;
mod l01_readers;
mod l02_buffered;
mod l03_writers;
mod l04_seek_binary;
mod l05_paths;
mod l06_directories;
mod l07_capstone;
mod l08_tcp_frames;
mod l09_processes;
mod r01_review;

// Support code for the tests below this line. You do not need to edit it.
//
// The chapter's allocator forwards every request to the course's counting
// allocator, so `practice::alloc::count` works as in other chapters. It also
// remembers the largest single request made on the current thread. Tests use
// that to check that a rejected frame never caused a large allocation.
use practice::alloc::Counting;
use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;

thread_local! {
    static LARGEST: Cell<usize> = const { Cell::new(0) };
}

struct Tracking;

fn note(size: usize) {
    let _ = LARGEST.try_with(|largest| largest.set(largest.get().max(size)));
}

// SAFETY: every method forwards unchanged to `Counting`, which upholds the
// GlobalAlloc contract. Recording touches only a const-initialized
// thread-local without a destructor, so it never allocates or reenters.
unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        // SAFETY: the caller upholds this method's contract; forwarded unchanged.
        unsafe { Counting.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        // SAFETY: the caller upholds this method's contract; forwarded unchanged.
        unsafe { Counting.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note(new_size);
        // SAFETY: the caller upholds this method's contract; forwarded unchanged.
        unsafe { Counting.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds this method's contract; forwarded unchanged.
        unsafe { Counting.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOC: Tracking = Tracking;

/// Runs `f` and returns its result with the size in bytes of the largest
/// single allocation it requested on this thread.
fn largest_allocation<R>(f: impl FnOnce() -> R) -> (R, usize) {
    let outer = LARGEST.with(|largest| largest.replace(0));
    let result = f();
    let inner = LARGEST.with(|largest| largest.replace(0));
    LARGEST.with(|largest| largest.set(outer.max(inner)));
    (result, inner)
}
