//! DRILL M7-05 · A generic method behind `dyn`
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many versions of `put` would the one vtable of `Count` have to hold?

use std::fmt::Display;

trait Collector {
    fn put<T: Display>(&mut self, item: T);
}

struct Count(usize);

impl Collector for Count {
    fn put<T: Display>(&mut self, item: T) {
        self.0 += item.to_string().len();
    }
}

fn main() {
    let mut count = Count(0);
    {
        // The caller wants one copy of its code, whatever the collector is.
        let collector: &mut dyn Collector = &mut count; // KEEP
        collector.put(12345);
        collector.put("ab");
    }
    assert_eq!(count.0, 7); // KEEP
}
