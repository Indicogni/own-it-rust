//! DRILL M2-03 · An unsafe operation inside an unsafe fn
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: in edition 2024, is the body of an `unsafe fn` an `unsafe` block? And if it is not, is that an error or a warning?

/// Reads the byte behind `byte` two times and adds the results.
///
/// # Safety
///
/// `byte` must point to a live, initialized `u8`.
unsafe fn read_twice(byte: *const u8) -> u16 {
    let first = *byte;
    let second = unsafe { *byte };
    u16::from(first) + u16::from(second)
}

fn main() {
    let byte = 21u8;
    // SAFETY: the pointer comes from a reference to a live local.
    let total = unsafe { read_twice(&byte) };
    assert_eq!(total, 42); // KEEP
}
