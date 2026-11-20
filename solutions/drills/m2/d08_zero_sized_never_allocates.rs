//! DRILL M2-08 · A thousand values of size zero
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ok
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many bytes does this program ask the allocator for?

// NOTE: this drill compiles and runs. A value of type `()` has no bytes, so
// a vector of them never allocates: it only counts. Its capacity is reported
// as `usize::MAX`, and a `Box<()>` holds a dangling pointer that is non-null
// and aligned but points to no allocation. Lessons 02 and 03 make your own
// box and vector behave the same way.
fn main() {
    let mut units: Vec<()> = Vec::new();
    assert_eq!(units.capacity(), usize::MAX); // KEEP
    for _ in 0..1000 {
        units.push(());
    }
    assert_eq!(units.len(), 1000); // KEEP
    assert_eq!(units.pop(), Some(())); // KEEP

    let boxed = Box::new(());
    let address = &raw const *boxed;
    assert!(!address.is_null()); // KEEP
}
