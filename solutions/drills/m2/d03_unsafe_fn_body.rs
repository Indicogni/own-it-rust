//! DRILL M2-03 · An unsafe operation inside an unsafe fn
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ok
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: in edition 2024, is the body of an `unsafe fn` an `unsafe` block? And if it is not, is that an error or a warning?

// NOTE: this drill compiles and runs. In edition 2024 the body of an
// `unsafe fn` is no longer an `unsafe` block, but the missing block is only a
// warning (`unsafe_op_in_unsafe_fn`), and the drill is built with warnings
// off. The version below is how the function should be written: one block
// for each operation, with the reason why it is sound. The course package
// `mastery/m2_unsafe` turns this warning into an error.
/// Reads the byte behind `byte` two times and adds the results.
///
/// # Safety
///
/// `byte` must point to a live, initialized `u8`.
unsafe fn read_twice(byte: *const u8) -> u16 {
    // SAFETY: the caller promises that `byte` points to a live `u8`.
    let first = unsafe { *byte };
    // SAFETY: as above.
    let second = unsafe { *byte };
    u16::from(first) + u16::from(second)
}

fn main() {
    let byte = 21u8;
    // SAFETY: the pointer comes from a reference to a live local.
    let total = unsafe { read_twice(&byte) };
    assert_eq!(total, 42); // KEEP
}
