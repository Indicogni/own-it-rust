//! Host acceptance tests for `kcore::Bump` and `kcore::FreeList` (SPEC 5.2).
//!
//! ```text
//! cargo test --test host_heap
//! cargo +nightly-2026-11-20 miri test --test host_heap
//! ```
//!
//! Each test hands the allocator a region of ordinary host memory. Under
//! Miri the same tests also check that the allocator only touches memory
//! it was given and never mixes up pointers.

use std::alloc::{self, Layout};

use harness::within;
use kcore::{Bump, FreeList};

/// A block of host memory that plays the role of the kernel's heap region.
struct Region {
    start: *mut u8,
    layout: Layout,
}

impl Region {
    fn new(len: usize) -> Self {
        let layout = Layout::from_size_align(len, 4096).expect("valid region layout");
        // SAFETY: `len` is never zero in these tests.
        let start = unsafe { alloc::alloc(layout) };
        assert!(!start.is_null(), "the host is out of memory");
        Self { start, layout }
    }

    fn len(&self) -> usize {
        self.layout.size()
    }

    fn contains(&self, block: *mut u8, size: usize) -> bool {
        let start = self.start.addr();
        block.addr() >= start && block.addr() + size <= start + self.len()
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        // SAFETY: `start` came from `alloc::alloc` with this layout.
        unsafe { alloc::dealloc(self.start, self.layout) };
    }
}

fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).expect("valid test layout")
}

/// Fills a block with one byte value.
fn fill(block: *mut u8, size: usize, value: u8) {
    // SAFETY: the caller passes a block of `size` bytes that an allocator
    // under test just returned as non-null. If the allocator is wrong, Miri
    // reports the bad write here.
    unsafe { block.write_bytes(value, size) };
}

/// Checks that a block still holds the byte value it was filled with.
fn holds(block: *mut u8, size: usize, value: u8) -> bool {
    // SAFETY: as for `fill`; the block is still allocated.
    (0..size).all(|i| unsafe { block.add(i).read() } == value)
}

// ---------------------------------------------------------------------------
// Bump
// ---------------------------------------------------------------------------

#[test]
fn bump_without_memory_returns_null() {
    let mut bump = Bump::new();
    assert!(
        bump.alloc(layout(8, 8)).is_null(),
        "before `init` there is no memory: alloc must return null, not panic"
    );
}

#[test]
fn bump_blocks_are_aligned_inside_the_region_and_disjoint() {
    within(10, "bump allocation", || {
        let region = Region::new(8192);
        let mut bump = Bump::new();
        // SAFETY: the region is valid and used by nothing else.
        unsafe { bump.init(region.start, region.len()) };

        let layouts = [
            layout(1, 1),
            layout(24, 8),
            layout(3, 2),
            layout(100, 64),
            layout(4096, 16),
        ];
        let mut blocks = Vec::new();
        for (index, &request) in layouts.iter().enumerate() {
            let block = bump.alloc(request);
            assert!(!block.is_null(), "request {index} fits in the region");
            assert!(
                block.addr().is_multiple_of(request.align()),
                "request {index}: the block must be aligned to {}",
                request.align()
            );
            assert!(
                region.contains(block, request.size()),
                "request {index}: the block must lie inside the region"
            );
            fill(block, request.size(), index as u8 + 1);
            blocks.push(block);
        }
        for (index, (&block, &request)) in blocks.iter().zip(&layouts).enumerate() {
            assert!(
                holds(block, request.size(), index as u8 + 1),
                "request {index}: a later block overlaps this one"
            );
        }
    });
}

#[test]
fn bump_reports_exhaustion_and_keeps_working() {
    let region = Region::new(4096);
    let mut bump = Bump::new();
    // SAFETY: the region is valid and used by nothing else.
    unsafe { bump.init(region.start, 256) };
    assert!(
        !bump.alloc(layout(200, 1)).is_null(),
        "200 of 256 bytes fit"
    );
    assert!(
        bump.alloc(layout(100, 1)).is_null(),
        "100 more bytes do not fit in 256: alloc must return null"
    );
    assert!(
        !bump.alloc(layout(56, 1)).is_null(),
        "a failed request must not consume memory: the last 56 bytes are still free"
    );
    assert!(bump.alloc(layout(1, 1)).is_null(), "now the region is full");
    assert!(
        bump.alloc(layout(usize::MAX / 4, 4096)).is_null(),
        "an absurd size must give null, not an arithmetic overflow"
    );
}

#[test]
fn bump_used_grows_and_resets_when_the_last_block_is_freed() {
    let region = Region::new(4096);
    let mut bump = Bump::new();
    // SAFETY: the region is valid and used by nothing else.
    unsafe { bump.init(region.start, region.len()) };
    assert_eq!(bump.used(), 0, "nothing is used right after init");

    let small = layout(10, 1);
    let first = bump.alloc(small);
    let second = bump.alloc(small);
    assert!(!first.is_null() && !second.is_null());
    assert!(
        bump.used() >= 20,
        "two blocks of 10 bytes use at least 20 bytes"
    );
    let used_with_two = bump.used();

    // SAFETY: `first` came from this allocator with this layout.
    unsafe { bump.dealloc(first, small) };
    assert_eq!(
        bump.used(),
        used_with_two,
        "a bump allocator cannot reuse one freed block while another is live"
    );
    // SAFETY: `second` came from this allocator with this layout.
    unsafe { bump.dealloc(second, small) };
    assert_eq!(
        bump.used(),
        0,
        "freeing the last live block resets the allocator"
    );
    assert!(
        !bump.alloc(layout(4096, 1)).is_null(),
        "after the reset the whole region can be allocated again"
    );
}

// ---------------------------------------------------------------------------
// FreeList
// ---------------------------------------------------------------------------

#[test]
fn free_list_without_memory_returns_null() {
    let mut heap = FreeList::new();
    assert_eq!(heap.free_bytes(), 0, "before `init` nothing is free");
    assert!(
        heap.alloc(layout(8, 8)).is_null(),
        "before `init` there is no memory: alloc must return null, not panic"
    );
}

#[test]
fn free_list_offers_almost_the_whole_region() {
    let region = Region::new(4096);
    let mut heap = FreeList::new();
    // SAFETY: the region is valid and used by nothing else.
    unsafe { heap.init(region.start, region.len()) };
    assert!(
        heap.free_bytes() <= region.len() && heap.free_bytes() >= region.len() - 64,
        "after init at most 64 bytes may be lost to alignment and bookkeeping, \
         but free_bytes() is {} of {}",
        heap.free_bytes(),
        region.len()
    );

    // A region that starts and ends at odd addresses must work too.
    let mut odd = FreeList::new();
    // SAFETY: a sub-range of the same valid region; `heap` is not used again.
    unsafe { odd.init(region.start.add(1), region.len() - 3) };
    assert!(
        odd.free_bytes() >= region.len() - 3 - 64,
        "an unaligned region may lose a few bytes at each end, not more"
    );
    let block = odd.alloc(layout(64, 16));
    assert!(
        !block.is_null() && block.addr().is_multiple_of(16),
        "blocks are aligned anyway"
    );
    assert!(
        block.addr() > region.start.addr()
            && block.addr() + 64 <= region.start.addr() + region.len() - 2,
        "blocks stay inside the unaligned region"
    );
}

#[test]
fn free_list_blocks_are_aligned_inside_the_region_and_disjoint() {
    within(10, "free-list allocation", || {
        let region = Region::new(64 * 1024);
        let mut heap = FreeList::new();
        // SAFETY: the region is valid and used by nothing else.
        unsafe { heap.init(region.start, region.len()) };

        let layouts = [
            layout(1, 1),
            layout(3, 2),
            layout(24, 8),
            layout(17, 16),
            layout(100, 64),
            layout(64, 32),
            layout(4096, 4096),
            layout(5000, 8),
            layout(9, 1),
            layout(256, 256),
        ];
        let mut blocks = Vec::new();
        for (index, &request) in layouts.iter().enumerate() {
            let before = heap.free_bytes();
            let block = heap.alloc(request);
            assert!(!block.is_null(), "request {index} fits in the region");
            assert!(
                block.addr().is_multiple_of(request.align()),
                "request {index}: the block must be aligned to {}",
                request.align()
            );
            assert!(
                region.contains(block, request.size()),
                "request {index}: the block must lie inside the region"
            );
            assert!(
                before - heap.free_bytes() >= request.size(),
                "request {index}: free_bytes() must shrink by at least the block size"
            );
            fill(block, request.size(), index as u8 + 1);
            blocks.push(block);
        }
        for (index, (&block, &request)) in blocks.iter().zip(&layouts).enumerate() {
            assert!(
                holds(block, request.size(), index as u8 + 1),
                "request {index}: another block, or the allocator's own bookkeeping, \
                 overwrote this live block"
            );
        }
    });
}

#[test]
fn free_list_reports_exhaustion_and_recovers() {
    within(10, "free-list exhaustion", || {
        let region = Region::new(4096);
        let mut heap = FreeList::new();
        // SAFETY: the region is valid and used by nothing else.
        unsafe { heap.init(region.start, region.len()) };

        let request = layout(1024, 8);
        let mut blocks = Vec::new();
        loop {
            let block = heap.alloc(request);
            if block.is_null() {
                break;
            }
            blocks.push(block);
            assert!(
                blocks.len() <= 4,
                "4 KiB cannot hold more than four blocks of 1 KiB"
            );
        }
        assert!(
            blocks.len() >= 3,
            "at least three blocks of 1 KiB fit in 4 KiB"
        );
        assert!(
            heap.alloc(layout(usize::MAX / 4, 4096)).is_null(),
            "an absurd size must give null, not an arithmetic overflow"
        );

        let freed = blocks.pop().expect("at least three blocks");
        // SAFETY: `freed` came from this allocator with this layout.
        unsafe { heap.dealloc(freed, request) };
        assert!(
            !heap.alloc(request).is_null(),
            "after one block is freed, a block of the same size must fit again"
        );
    });
}

/// Allocates `count` small blocks, frees them in the given order and checks
/// that the memory has merged back into one piece.
fn merges_back(order: impl Fn(usize) -> Vec<usize> + Send + 'static, what: &'static str) {
    within(10, what, move || {
        const COUNT: usize = 48;
        let region = Region::new(8192);
        let mut heap = FreeList::new();
        // SAFETY: the region is valid and used by nothing else.
        unsafe { heap.init(region.start, region.len()) };
        let free_at_start = heap.free_bytes();

        let request = layout(40, 8);
        let blocks: Vec<*mut u8> = (0..COUNT).map(|_| heap.alloc(request)).collect();
        assert!(
            blocks.iter().all(|block| !block.is_null()),
            "48 blocks of 40 bytes fit in 8 KiB"
        );

        let order = order(COUNT);
        assert_eq!(order.len(), COUNT);
        for index in order {
            // SAFETY: each index appears once, so each block is freed once,
            // with the layout it was allocated with.
            unsafe { heap.dealloc(blocks[index], request) };
        }

        assert_eq!(
            heap.free_bytes(),
            free_at_start,
            "{what}: after every block is freed, free_bytes() must be back at its value after init"
        );
        let big = layout(region.len() / 4 * 3, 16);
        assert!(
            !heap.alloc(big).is_null(),
            "{what}: all blocks are free, so a block of three quarters of the region must fit. \
             It does not: the freed blocks were not merged with their neighbours."
        );
    });
}

#[test]
fn free_list_merges_blocks_freed_in_address_order() {
    merges_back(|count| (0..count).collect(), "freeing in address order");
}

#[test]
fn free_list_merges_blocks_freed_in_reverse_order() {
    merges_back(
        |count| (0..count).rev().collect(),
        "freeing in reverse order",
    );
}

#[test]
fn free_list_merges_blocks_freed_in_scattered_order() {
    // Evens first, then odds from the top: every odd block is freed between
    // two free neighbours and has to merge with both.
    merges_back(
        |count| {
            (0..count)
                .step_by(2)
                .chain((1..count).step_by(2).rev())
                .collect()
        },
        "freeing every second block, then the rest",
    );
}

#[test]
fn free_list_uses_only_the_address_of_a_freed_pointer() {
    // `Box<T>` and `Vec<T>` give a block back through a pointer that went
    // through a reference to their content. Such a pointer may access
    // `layout.size()` bytes and no more. The free block that the allocator
    // manages is usually larger: sizes are rounded up and neighbours merge.
    // So `dealloc` must take the address from `ptr` and build the pointer it
    // keeps from the region pointer (SPEC 5.2). An ordinary test run cannot
    // see the difference; Miri reports the access that is not allowed.
    within(10, "free-list reuse of narrowed pointers", || {
        let region = Region::new(4096);
        let mut heap = FreeList::new();
        // SAFETY: the region is valid and used by nothing else.
        unsafe { heap.init(region.start, region.len()) };
        let free_at_start = heap.free_bytes();

        let byte = layout(1, 1);
        let blocks: Vec<*mut u8> = (0..8)
            .map(|_| {
                let block = heap.alloc(byte);
                assert!(!block.is_null(), "eight blocks of one byte fit in 4 KiB");
                // SAFETY: a live block of one byte that nothing else uses.
                let content: &mut u8 = unsafe { &mut *block };
                *content = 0xa5;
                // From here on the pointer is valid for this one byte only,
                // like the pointer inside a `Box<u8>`.
                std::ptr::from_mut(content)
            })
            .collect();
        for index in [1, 0, 3, 2, 7, 5, 6, 4] {
            // SAFETY: each block is freed once, with its layout.
            unsafe { heap.dealloc(blocks[index], byte) };
        }
        assert_eq!(
            heap.free_bytes(),
            free_at_start,
            "after every block is freed, free_bytes() must be back at its value after init"
        );

        let big = layout(region.len() / 4 * 3, 16);
        let block = heap.alloc(big);
        assert!(
            !block.is_null(),
            "all blocks are free, so a block of three quarters of the region must fit"
        );
        // Under Miri this write fails if the new block's pointer was derived
        // from one of the one-byte pointers above.
        fill(block, big.size(), 0x5a);
        assert!(
            holds(block, big.size(), 0x5a),
            "the new block must be usable in full"
        );
        // SAFETY: `block` came from this allocator with this layout.
        unsafe { heap.dealloc(block, big) };
        assert_eq!(
            heap.free_bytes(),
            free_at_start,
            "freeing the large block returns every byte"
        );
    });
}

#[test]
fn free_list_survives_a_long_random_workload() {
    // Miri runs this about a hundred times slower than native code.
    let steps = if cfg!(miri) { 300 } else { 20_000 };
    within(60, "free-list random workload", move || {
        let region = Region::new(32 * 1024);
        let mut heap = FreeList::new();
        // SAFETY: the region is valid and used by nothing else.
        unsafe { heap.init(region.start, region.len()) };
        let free_at_start = heap.free_bytes();

        // Live blocks with the byte value each one was filled with.
        let mut live: Vec<(*mut u8, Layout, u8)> = Vec::new();
        let mut state: u32 = 0x9e37_79b9;
        let mut next = |bound: u32| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) % bound
        };

        for step in 0..steps {
            let allocate = live.len() < 4 || (live.len() < 48 && next(5) < 3);
            if allocate {
                let request = layout(1 + next(300) as usize, 1 << next(8));
                let block = heap.alloc(request);
                if block.is_null() {
                    continue; // fragmented or full: allowed
                }
                assert!(
                    block.addr().is_multiple_of(request.align()),
                    "step {step}: misaligned block"
                );
                assert!(
                    region.contains(block, request.size()),
                    "step {step}: block outside the region"
                );
                let value = (step % 251) as u8;
                fill(block, request.size(), value);
                live.push((block, request, value));
            } else {
                let (block, request, value) = live.swap_remove(next(live.len() as u32) as usize);
                assert!(
                    holds(block, request.size(), value),
                    "step {step}: a live block was overwritten while it was allocated"
                );
                // SAFETY: the block is live, came from this allocator with
                // this layout, and is removed from `live` so it is freed once.
                unsafe { heap.dealloc(block, request) };
            }
        }

        for (block, request, value) in live {
            assert!(
                holds(block, request.size(), value),
                "a live block was overwritten"
            );
            // SAFETY: as above.
            unsafe { heap.dealloc(block, request) };
        }
        assert_eq!(
            heap.free_bytes(),
            free_at_start,
            "after the workload every byte must be free again: the allocator leaks or loses memory"
        );
        assert!(
            !heap.alloc(layout(region.len() / 4 * 3, 16)).is_null(),
            "after the workload the free memory must be one piece again"
        );
    });
}
