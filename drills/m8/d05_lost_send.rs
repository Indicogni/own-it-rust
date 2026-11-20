//! DRILL M8-05 · A private field changes its type
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m8`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which trait does `thread::spawn` demand of everything the closure owns, and who decides whether `Counter` has it?

/// Stands for version 1.0.1 of a library crate that you maintain. The release
/// notes say: "internal clean-up, no change of the interface".
mod metrics {
    use std::cell::Cell;
    use std::rc::Rc;

    /// Counts hits. Clones share one count.
    #[derive(Clone, Default)]
    pub struct Counter {
        // In 1.0.0 this field was an `Arc<AtomicU64>`.
        hits: Rc<Cell<u64>>,
    }

    impl Counter {
        pub fn hit(&self) {
            self.hits.set(self.hits.get() + 1);
        }

        pub fn hits(&self) -> u64 {
            self.hits.get()
        }
    }
}

fn main() {
    // A user's code, written for 1.0.0.
    let counter = metrics::Counter::default(); // KEEP
    let worker = counter.clone(); // KEEP
    std::thread::spawn(move || worker.hit()).join().expect("the worker runs"); // KEEP
    counter.hit(); // KEEP
    assert_eq!(counter.hits(), 2); // KEEP
}
