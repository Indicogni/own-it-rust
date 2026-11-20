//! DRILL M2-07 · A compile-time check inside a generic function
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0080
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: when is the `const` block evaluated: when `park` is defined, or for each type it is used with?

use std::mem::needs_drop;

/// Moves a value to the heap and never frees it. Refuses types that own
/// resources, because their destructor would never run.
fn park<T>(value: T) -> &'static mut T {
    const { assert!(!needs_drop::<T>(), "park would leak this type") };
    Box::leak(Box::new(value))
}

// FIX: the `const` block is evaluated once for each `T` that `park` is used
// with, during code generation. `park::<String>` fails that check, so the
// build stops (and `cargo check`, which generates no code, would not have
// noticed). A `&'static str` owns nothing and has no destructor.
fn main() {
    let number = park(7u32);
    let name = park("ada");
    assert_eq!(*number, 7); // KEEP
    assert_eq!(name.len(), 3); // KEEP
}
