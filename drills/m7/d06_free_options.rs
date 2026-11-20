//! DRILL M7-06 · Which `Option`s are free?
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which of these types has a bit pattern that it never uses?

use std::num::NonZeroU32;

struct Id(NonZeroU32);

fn main() {
    assert_eq!(size_of::<Option<Id>>(), size_of::<u32>()); // KEEP
    assert_eq!(size_of::<Option<Box<u8>>>(), size_of::<usize>()); // KEEP
    assert!(size_of::<Option<u32>>() > size_of::<u32>()); // KEEP
    let id = Id(NonZeroU32::new(7).unwrap());
    assert_eq!(id.0.get(), 7); // KEEP
}
