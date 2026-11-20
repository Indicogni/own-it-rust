//! DRILL M6-03 · An out-parameter
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0381
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: the C function only writes. Does the Rust compiler know that?

use std::mem::MaybeUninit;

#[repr(C)]
struct Stats {
    len: usize,
    capacity: usize,
}

/// Stands in for a C function that fills `*out`: `void get_stats(stats *out)`.
unsafe extern "C" fn get_stats(out: *mut Stats) {
    // SAFETY: the caller passes a pointer that is valid for one write.
    unsafe {
        out.write(Stats {
            len: 2,
            capacity: 8,
        })
    }
}

// FIX: `&mut stats` is a reference, and a reference must point to an
// initialised value; the compiler cannot know that the callee only writes.
// `MaybeUninit<Stats>` is memory for a `Stats` that may be uninitialised:
// `as_mut_ptr` gives the raw pointer for the C function, and `assume_init`
// is the promise, made after the call, that the function filled it.
fn main() {
    let mut stats = MaybeUninit::<Stats>::uninit();
    // SAFETY: the pointer is valid for one write.
    unsafe { get_stats(stats.as_mut_ptr()) };
    // SAFETY: get_stats wrote every field.
    let stats = unsafe { stats.assume_init() };
    assert_eq!((stats.len, stats.capacity), (2, 8)); // KEEP
}
