//! DRILL M5-04 · Copy without Clone
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m5`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does the impl that `#[derive(Copy)]` writes look like, and what does the trait `Copy` require?

// `Copy` has `Clone` as a supertrait, and each derive writes only its own impl.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Id(u32);

fn main() {
    let first = Id(7); // KEEP
    let second = first; // KEEP
    assert_eq!(first, second); // KEEP
}
