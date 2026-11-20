//! The kernel heap: makes `alloc::{boxed::Box, vec::Vec, string::String}` work.
//!
//! The allocation algorithm lives in `kcore::FreeList`, where it is tested
//! on the host. This file only adds what the algorithm must not know about:
//! where the memory is, and how to keep interrupt handlers out while the
//! free list is being changed.

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicBool, Ordering};

use kcore::FreeList;

use crate::sync::IrqLock;

// SAFETY: both symbols are defined by `link.ld` and only their addresses are
// used; the type `u8` is a placeholder.
unsafe extern "C" {
    static __heap_start: u8;
    static __heap_end: u8;
}

static HEAP: IrqLock<FreeList> = IrqLock::new(FreeList::new());

/// Set by the first call of [`init`]. A second call would hand out memory
/// that live blocks still use, so it is refused.
static INITIALISED: AtomicBool = AtomicBool::new(false);

struct KernelHeap;

#[global_allocator]
static ALLOCATOR: KernelHeap = KernelHeap;

// SAFETY: `GlobalAlloc` requires that `alloc` returns either null or a block
// that fits `layout` and overlaps no other live block, and that these
// functions never unwind. `FreeList` provides the first (its host tests and
// Miri check that), `IrqLock` serialises all calls, and this kernel aborts
// on panic.
unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP.lock(|heap| heap.alloc(layout))
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller of `GlobalAlloc::dealloc` guarantees that `ptr`
        // came from `alloc` above with the same layout and is freed once,
        // which is exactly the contract of `FreeList::dealloc`.
        HEAP.lock(|heap| unsafe { heap.dealloc(ptr, layout) });
    }
}

/// Hands the heap region from the linker script to the allocator. Before
/// this call every allocation fails.
///
/// # Panics
///
/// Panics when called a second time.
pub fn init() {
    assert!(
        !INITIALISED.swap(true, Ordering::Relaxed),
        "heap::init called twice"
    );
    let start = (&raw const __heap_start).addr();
    let end = (&raw const __heap_end).addr();
    // SAFETY: the linker script reserves `__heap_start..__heap_end` inside
    // RAM for this purpose and places nothing else there. The memory is
    // readable and writable for the whole run, and only the allocator uses
    // it from here on. The flag above proves that this is the first call,
    // so no block of an earlier `init` can be live.
    HEAP.lock(|heap| unsafe {
        heap.init(core::ptr::with_exposed_provenance_mut(start), end - start);
    });
}

/// Bytes of heap that are free now.
pub fn free_bytes() -> usize {
    HEAP.lock(|heap| heap.free_bytes())
}
