//! DRILL M5-07 · A derive and a hand-written impl
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m5`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: a derive adds an ordinary impl block next to the type. How many impls of `Clone` does `Counter` have?

// `#[derive(Clone)]` writes `impl Clone for Counter`. With the impl below, that
// would make two. The hand-written one has the behaviour that `main` expects.
#[derive(Debug)]
struct Counter {
    count: u32,
}

/// A clone starts counting from zero.
impl Clone for Counter {
    fn clone(&self) -> Counter {
        Counter { count: 0 } // KEEP
    }
}

fn main() {
    let counter = Counter { count: 5 }; // KEEP
    assert_eq!(counter.clone().count, 0); // KEEP
    assert_eq!(format!("{counter:?}"), "Counter { count: 5 }"); // KEEP
}
