//! DRILL M7-05 · A generic method behind `dyn`
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0038
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many versions of `put` would the one vtable of `Count` have to hold?

// FIX: a trait object has one vtable entry per method, and a generic method
// would need one entry for every type `T`, so the trait was not dyn compatible
// (E0038). A `&dyn Display` parameter gives one method that serves every type.
use std::fmt::Display;

trait Collector {
    // A trait object has one vtable entry per method, so the method cannot be
    // generic. A `&dyn Display` parameter gives one method for every type.
    fn put(&mut self, item: &dyn Display);
}

struct Count(usize);

impl Collector for Count {
    fn put(&mut self, item: &dyn Display) {
        self.0 += item.to_string().len();
    }
}

fn main() {
    let mut count = Count(0);
    {
        // The caller wants one copy of its code, whatever the collector is.
        let collector: &mut dyn Collector = &mut count; // KEEP
        collector.put(&12345);
        collector.put(&"ab");
    }
    assert_eq!(count.0, 7); // KEEP
}
