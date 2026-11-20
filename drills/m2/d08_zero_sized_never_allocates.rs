//! DRILL M2-08 · A thousand values of size zero
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many bytes does this program ask the allocator for?

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
