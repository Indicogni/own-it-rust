//! DRILL M6-03 · An out-parameter
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: the C function only writes. Does the Rust compiler know that?

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

fn main() {
    let mut stats: Stats;
    // SAFETY: the pointer is valid for one write.
    unsafe { get_stats(&mut stats) };
    assert_eq!((stats.len, stats.capacity), (2, 8)); // KEEP
}
