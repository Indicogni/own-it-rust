//! Two allocators that manage one caller-supplied region of memory.
//!
//! Neither type knows about `GlobalAlloc`, locks or interrupts. The kernel
//! wraps one of them in a lock; the host tests drive them directly.

use core::alloc::Layout;
use core::ptr;

/// Rounds `addr` up to a multiple of `align` (a power of two).
/// `None` means the result does not fit in `usize`.
fn align_up(addr: usize, align: usize) -> Option<usize> {
    debug_assert!(align.is_power_of_two());
    Some(addr.checked_add(align - 1)? & !(align - 1))
}

// ---------------------------------------------------------------------------
// Bump allocator
// ---------------------------------------------------------------------------

/// The simplest allocator: a pointer that only moves forward.
///
/// Freeing one block does nothing. When the *last* live block is freed the
/// pointer jumps back to the start, so a phase of work that frees everything
/// it allocated leaves the region reusable.
pub struct Bump {
    /// Start of the region. All returned pointers are derived from this one,
    /// so they carry its provenance.
    start: *mut u8,
    /// Address one past the end of the region.
    end: usize,
    /// Address of the next free byte. `start.addr() <= next <= end`.
    next: usize,
    /// Number of blocks handed out and not yet freed.
    live: usize,
}

// SAFETY: after `init` the allocator is the only owner of its region (that is
// the contract of `init`), and it has no thread affinity. Moving the value to
// another thread moves that ownership with it.
unsafe impl Send for Bump {}

impl Bump {
    /// An allocator without memory. Every `alloc` fails until `init`.
    pub const fn new() -> Self {
        Self {
            start: ptr::null_mut(),
            end: 0,
            next: 0,
            live: 0,
        }
    }

    /// Gives the allocator the region `start .. start + len`.
    ///
    /// # Safety
    ///
    /// The region must be valid for reads and writes, must not be used by
    /// anything else while this allocator or any block from it is in use, and
    /// `start + len` must not overflow.
    pub unsafe fn init(&mut self, start: *mut u8, len: usize) {
        self.start = start;
        self.next = start.addr();
        self.end = start.addr() + len;
        self.live = 0;
    }

    /// Returns a block for `layout`, or a null pointer when the region is
    /// exhausted.
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let Some(block) = align_up(self.next, layout.align()) else {
            return ptr::null_mut();
        };
        let Some(block_end) = block.checked_add(layout.size()) else {
            return ptr::null_mut();
        };
        if self.start.is_null() || block_end > self.end {
            return ptr::null_mut();
        }
        self.next = block_end;
        self.live += 1;
        // `with_addr` keeps the provenance of the region pointer; an integer
        // cast would lose it and Miri would reject the result.
        self.start.with_addr(block)
    }

    /// Marks one block as freed.
    ///
    /// # Safety
    ///
    /// `ptr` must come from `alloc` on this allocator and must not have been
    /// freed already.
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let _ = (ptr, layout);
        self.live -= 1;
        if self.live == 0 {
            self.next = self.start.addr();
        }
    }

    /// Bytes between the start of the region and the bump pointer.
    pub fn used(&self) -> usize {
        self.next - self.start.addr()
    }
}

impl Default for Bump {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Free-list allocator
// ---------------------------------------------------------------------------

/// Header written at the start of every *free* block. Allocated blocks have
/// no header: `dealloc` receives the layout, and that is enough to rebuild
/// the size.
#[repr(C)]
struct Node {
    /// Size of this free block in bytes, header included.
    size: usize,
    /// The next free block at a higher address, or null.
    next: *mut Node,
}

/// Every block starts at a multiple of `GRANULE` and its size is a multiple
/// of `GRANULE`. So any free block can hold a `Node`, and the gap left in
/// front of an over-aligned allocation is either empty or a valid block.
const GRANULE: usize = 16;
const _: () = assert!(size_of::<Node>() <= GRANULE && align_of::<Node>() <= GRANULE);

/// A first-fit allocator over a singly linked list of free blocks.
///
/// The list is sorted by address. That makes merging cheap: a freed block
/// only has to look at its two list neighbours to find adjacent free memory.
pub struct FreeList {
    /// The pointer given to `init`. Every pointer the allocator stores or
    /// returns is derived from this one (see `at`), so each of them may
    /// access the whole region.
    region: *mut u8,
    head: *mut Node,
    free: usize,
}

// SAFETY: as for `Bump`: the allocator exclusively owns its region and every
// node in the list lies inside that region.
unsafe impl Send for FreeList {}

impl FreeList {
    /// An allocator without memory. Every `alloc` fails until `init`.
    pub const fn new() -> Self {
        Self {
            region: ptr::null_mut(),
            head: ptr::null_mut(),
            free: 0,
        }
    }

    /// A pointer to address `addr` with the provenance of the whole region.
    fn at(&self, addr: usize) -> *mut u8 {
        self.region.with_addr(addr)
    }

    /// Gives the allocator the region `start .. start + len`. The ends are
    /// trimmed to multiples of 16 bytes.
    ///
    /// # Safety
    ///
    /// The region must be valid for reads and writes, must not be used by
    /// anything else while this allocator or any block from it is in use, and
    /// `start + len` must not overflow.
    pub unsafe fn init(&mut self, start: *mut u8, len: usize) {
        self.region = start;
        self.head = ptr::null_mut();
        self.free = 0;
        let first = start.addr().next_multiple_of(GRANULE);
        let last = (start.addr() + len) & !(GRANULE - 1);
        if first >= last {
            return;
        }
        let node = self.at(first).cast::<Node>();
        // SAFETY: `first .. last` lies inside the caller's region, is at
        // least `GRANULE` bytes long and starts at a multiple of `GRANULE`,
        // so a `Node` fits there and is aligned.
        unsafe {
            node.write(Node {
                size: last - first,
                next: ptr::null_mut(),
            });
        }
        self.head = node;
        self.free = last - first;
    }

    /// The block size and alignment actually used for `layout`.
    fn rounded(layout: Layout) -> Option<(usize, usize)> {
        let size = align_up(layout.size().max(GRANULE), GRANULE)?;
        Some((size, layout.align().max(GRANULE)))
    }

    /// Returns a block for `layout`, or a null pointer when no free block is
    /// large enough.
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let Some((size, align)) = Self::rounded(layout) else {
            return ptr::null_mut();
        };
        // `link` points at the pointer that leads to the block under test:
        // first `self.head`, then the `next` field of each node in turn.
        // Unlinking a block is then one write through `link`, with no special
        // case for the head of the list.
        let mut link: *mut *mut Node = &raw mut self.head;
        loop {
            // SAFETY: `link` points either at `self.head` or at the `next`
            // field of a node in the list. Both are valid and initialised.
            let node = unsafe { *link };
            if node.is_null() {
                return ptr::null_mut();
            }
            // SAFETY: every non-null pointer in the list refers to an
            // initialised `Node` inside the region (type invariant).
            let (block_size, next) = unsafe { ((*node).size, (*node).next) };
            let block_start = node.addr();
            let block_end = block_start + block_size;

            let fits = align_up(block_start, align)
                .and_then(|start| Some((start, start.checked_add(size)?)))
                .filter(|&(_, end)| end <= block_end);
            let Some((alloc_start, alloc_end)) = fits else {
                // SAFETY: `node` is a valid node, so its `next` field is a
                // valid place.
                link = unsafe { &raw mut (*node).next };
                continue;
            };

            // Split the block into up to three parts:
            //   [front gap][allocation][back gap]
            // Both gaps are multiples of GRANULE, so each one is either
            // empty or big enough to stay in the list as a free block.
            let mut tail = next;
            if alloc_end < block_end {
                let back = self.at(alloc_end).cast::<Node>();
                // SAFETY: `alloc_end .. block_end` is inside the free block
                // we are splitting, is at least GRANULE bytes long and is
                // GRANULE-aligned.
                unsafe {
                    back.write(Node {
                        size: block_end - alloc_end,
                        next,
                    });
                }
                tail = back;
            }
            if alloc_start > block_start {
                // The front gap keeps the existing node; only its fields change.
                // SAFETY: `node` is a valid node that we own exclusively.
                unsafe {
                    (*node).size = alloc_start - block_start;
                    (*node).next = tail;
                }
            } else {
                // SAFETY: see the comment on `link` above.
                unsafe { *link = tail };
            }
            self.free -= size;
            return self.at(alloc_start);
        }
    }

    /// Returns a block to the allocator and merges it with free neighbours.
    ///
    /// # Safety
    ///
    /// `ptr` must come from `alloc` on this allocator with the same `layout`
    /// and must not have been freed already.
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let Some((size, _)) = Self::rounded(layout) else {
            return; // `alloc` never succeeds for such a layout
        };

        // Find the first free block above `ptr`, remembering the block
        // before it (if any).
        let mut prev: *mut Node = ptr::null_mut();
        let mut link: *mut *mut Node = &raw mut self.head;
        // SAFETY: `link` always points at `self.head` or at the `next` field
        // of a valid node, and every non-null list pointer is a valid node.
        unsafe {
            while !(*link).is_null() && (*link).addr() < ptr.addr() {
                prev = *link;
                link = &raw mut (*prev).next;
            }
        }

        // Only the *address* of `ptr` is used. The caller's pointer may have
        // passed through a reference (a `Box<u8>` hands back a pointer that
        // is valid for one byte), so it may not be allowed to access the
        // whole block of `size` bytes, let alone a neighbour that this block
        // is merged with later. A pointer derived from the region pointer is.
        let node = self.at(ptr.addr()).cast::<Node>();
        // SAFETY: the caller returns a block of at least `size >= GRANULE`
        // bytes that starts at a multiple of GRANULE (that is what `alloc`
        // hands out), so a `Node` fits and is aligned. `link` is valid as
        // explained above.
        unsafe {
            let next = *link;
            node.write(Node { size, next });
            *link = node;

            // Merge with the block above when the two touch.
            if !next.is_null() && node.addr() + size == next.addr() {
                (*node).size += (*next).size;
                (*node).next = (*next).next;
            }
            // Merge with the block below when the two touch.
            if !prev.is_null() && prev.addr() + (*prev).size == node.addr() {
                (*prev).size += (*node).size;
                (*prev).next = (*node).next;
            }
        }
        self.free += size;
    }

    /// Total number of bytes in free blocks.
    pub fn free_bytes(&self) -> usize {
        self.free
    }
}

impl Default for FreeList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the free list's *internal* invariants. The acceptance tests
    //! in `tests/host_heap.rs` only see the public behaviour; these walk the
    //! list after every operation.

    extern crate std;

    use std::alloc::{alloc as host_alloc, dealloc as host_dealloc};
    use std::vec::Vec;

    use super::*;

    impl FreeList {
        /// Checks every invariant of the list and returns the number of
        /// free blocks.
        fn check_invariants(&self) -> usize {
            let mut blocks = 0;
            let mut total = 0;
            let mut previous_end = 0;
            let mut node = self.head;
            while !node.is_null() {
                // SAFETY: the test only calls this between operations, when
                // every list pointer refers to an initialised node.
                let (size, next) = unsafe { ((*node).size, (*node).next) };
                assert!(
                    node.addr().is_multiple_of(GRANULE),
                    "block start is a multiple of GRANULE"
                );
                assert!(
                    size >= GRANULE && size.is_multiple_of(GRANULE),
                    "block size is a multiple of GRANULE"
                );
                assert!(
                    blocks == 0 || node.addr() > previous_end,
                    "blocks are sorted by address and two free blocks never touch"
                );
                previous_end = node.addr() + size;
                total += size;
                blocks += 1;
                node = next;
            }
            assert_eq!(total, self.free, "the counter matches the list");
            blocks
        }
    }

    #[test]
    fn invariants_hold_after_every_operation() {
        const LEN: usize = 16 * 1024;
        let steps = if cfg!(miri) { 150 } else { 4000 };
        let region_layout = Layout::from_size_align(LEN, 4096).unwrap();
        // SAFETY: the layout has a non-zero size.
        let region = unsafe { host_alloc(region_layout) };
        assert!(!region.is_null());

        let mut heap = FreeList::new();
        // SAFETY: the region is valid and owned by this test.
        unsafe { heap.init(region, LEN) };
        assert_eq!(heap.check_invariants(), 1, "a fresh heap is one block");

        let mut live: Vec<(*mut u8, Layout)> = Vec::new();
        let mut state: u32 = 0xdead_beef;
        let mut next = |bound: u32| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) % bound
        };
        for _ in 0..steps {
            if live.is_empty() || (live.len() < 40 && next(5) < 3) {
                let layout = Layout::from_size_align(1 + next(200) as usize, 1 << next(8)).unwrap();
                let block = heap.alloc(layout);
                if !block.is_null() {
                    live.push((block, layout));
                }
            } else {
                let (block, layout) = live.swap_remove(next(live.len() as u32) as usize);
                // SAFETY: the block is live and came from `heap` with this layout.
                unsafe { heap.dealloc(block, layout) };
            }
            heap.check_invariants();
        }
        for (block, layout) in live {
            // SAFETY: as above.
            unsafe { heap.dealloc(block, layout) };
        }
        assert_eq!(
            heap.check_invariants(),
            1,
            "everything merged back into one block"
        );

        // SAFETY: allocated above with the same layout; no block is in use.
        unsafe { host_dealloc(region, region_layout) };
    }
}
