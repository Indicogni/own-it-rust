//! Milestone 6: the library never asks for heap memory (SPEC section 8.5).
//!
//! This file is a test binary of its own because it replaces the global
//! allocator with one that counts.
//!
//! ```text
//! cargo test --test m6_no_alloc
//! ```
//!
//! The real proof that the crate needs no heap is that it builds for a
//! target that has none; see "Stretch goals" in SPEC.md. This test is the
//! check you can run on any computer.

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use common::bounded;
use nostd_collections::{Arena, ArrayVec, RingBuffer};

thread_local! {
    /// Heap requests made by the current thread. The initialiser is a
    /// constant and the type has no destructor, so reading the counter never
    /// allocates by itself.
    static REQUESTS: Cell<usize> = const { Cell::new(0) };
}

/// The system allocator, with a counter in front of it.
struct Counting;

fn note_request() {
    // `try_with` fails only while a thread is shutting down; requests made
    // then are not part of any measurement.
    let _ = REQUESTS.try_with(|count| count.set(count.get() + 1));
}

// SAFETY: every method forwards to `System` with unchanged arguments, so the
// contract of `GlobalAlloc` is met exactly when `System` meets it.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note_request();
        // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note_request();
        // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc_zeroed`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note_request();
        // SAFETY: the caller upholds the contract of `GlobalAlloc::realloc`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds the contract of `GlobalAlloc::dealloc`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Runs `work` and returns its result with the number of heap requests that
/// the current thread made meanwhile.
fn count_requests<R>(work: impl FnOnce() -> R) -> (R, usize) {
    let before = REQUESTS.with(Cell::get);
    let result = work();
    (result, REQUESTS.with(Cell::get) - before)
}

#[test]
fn the_counting_allocator_counts() {
    bounded(|| {
        // `black_box` keeps an optimised build from removing the Box, and
        // with it the heap request that this test wants to see.
        let (boxed, requests) = count_requests(|| std::hint::black_box(Box::new(5u64)));
        assert_eq!(*boxed, 5);
        assert!(
            requests >= 1,
            "the test harness is broken: a Box was made and no heap request was counted"
        );
    });
}

#[test]
fn array_vec_never_allocates() {
    bounded(|| {
        let (sum, requests) = count_requests(|| {
            let mut v: ArrayVec<u32, 16> = ArrayVec::new();
            for n in 0..16 {
                v.push(n);
            }
            let _ = v.try_push(99);
            let last = v.pop().expect("the vector is full");
            v.insert(3, last);
            let moved = v.swap_remove(0);
            let removed = v.remove(5);
            v.push(moved);
            let _ = v.try_insert(1, removed);
            v.retain(|n| n % 3 != 0);
            let drained: u32 = v.drain(2..5).sum();
            let copy = v.clone();
            let collected: ArrayVec<u32, 16> = copy.into_iter().rev().collect();
            v.extend([1, 2]);
            v.truncate(4);
            drained + collected.iter().sum::<u32>() + v.len() as u32
        });
        assert!(sum > 0);
        assert_eq!(
            requests, 0,
            "ArrayVec made {requests} heap request(s); it must make none"
        );
    });
}

#[test]
fn ring_buffer_never_allocates() {
    bounded(|| {
        let (sum, requests) = count_requests(|| {
            let mut ring: RingBuffer<&'static str, 4> = RingBuffer::new();
            for word in ["a", "b", "c", "d", "e", "f"] {
                ring.push_overwrite(word);
            }
            let _ = ring.push_back("g");
            ring.pop_front();
            ring.push_front("h").unwrap();
            let lengths: usize = ring.iter().rev().map(|word| word.len()).sum();
            let copy = ring.clone();
            let same = usize::from(copy == ring);
            let (first, second) = ring.as_slices();
            let seen = first.len() + second.len();
            let by_value = copy.into_iter().count();
            ring.clear();
            lengths + same + seen + by_value + ring.len()
        });
        assert_eq!(sum, 4 + 1 + 4 + 4);
        assert_eq!(
            requests, 0,
            "RingBuffer made {requests} heap request(s); it must make none"
        );
    });
}

#[test]
fn arena_never_allocates() {
    bounded(|| {
        let (total, requests) = count_requests(|| {
            let mut arena: Arena<512> = Arena::new();
            let mut total = 0u64;
            for round in 0..3u64 {
                let number = arena.alloc(round).unwrap();
                let text = arena.alloc_str("no heap here").unwrap();
                let bytes = arena.alloc_slice_copy(&[1u8, 2, 3]).unwrap();
                let squares = arena.alloc_slice_fill_with(8, |i| (i * i) as u64).unwrap();
                let too_big = arena.alloc([0u8; 1024]);
                total += *number
                    + text.len() as u64
                    + u64::from(bytes[2])
                    + squares[7]
                    + u64::from(too_big.is_err());
                arena.reset();
            }
            total
        });
        assert_eq!(total, 3 + 3 * (12 + 3 + 49 + 1));
        assert_eq!(
            requests, 0,
            "Arena made {requests} heap request(s); it must make none"
        );
    });
}
