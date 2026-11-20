//! DRILL M1-02 · One lifetime for the value and for the borrow
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: for how long does the first call borrow `counter`?

struct Counter<'a> {
    names: Vec<&'a str>,
}

impl<'a> Counter<'a> {
    fn add(&'a mut self, name: &'a str) -> usize {
        self.names.push(name);
        self.names.len()
    }
}

fn main() {
    let mut counter = Counter { names: Vec::new() };
    assert_eq!(counter.add("ada"), 1); // KEEP
    assert_eq!(counter.add("grace"), 2); // KEEP
}
