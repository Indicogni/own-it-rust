//! MIRI LAB 01 — A cell without UnsafeCell (lesson 01)
//!
//! Run it under Miri, from the repository root:
//!
//!   cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l01_cell
//!
//! 1. PREDICT. Read the program. Every `unsafe` block has a SAFETY comment.
//!    Decide which comment is false or leaves something out, and what Miri
//!    will report, before you run anything.
//! 2. RUN the command above. Read the first `error:` line and the source line
//!    that it points to.
//! 3. COMPARE with "What Miri says about the wrong version" in CONCEPTS.md,
//!    lesson 01.
//! 4. REPAIR the program until Miri finishes without a report. Do not change
//!    `main`. Correct the SAFETY comments too.
//!
//! A native `cargo run --example miri_l01_cell` may print the expected result, print
//! something else, or crash. That is what undefined behaviour means: only the
//! Miri run tells you whether the program is right.

/// A pointer to the same value that can be written through.
fn writable<T>(shared: &T) -> *mut T {
    std::ptr::from_ref(shared).cast_mut()
}

/// Counts events. `record` takes `&self`, so that several parts of a program
/// can share one counter.
struct Counter {
    hits: u32,
}

impl Counter {
    fn new() -> Counter {
        Counter { hits: 0 }
    }

    fn record(&self) {
        let hits = writable(&self.hits);
        // SAFETY: `hits` comes from a reference to a live field, so it is
        // non-null, aligned and points to an initialized u32. The counter is
        // not shared between threads.
        unsafe { *hits += 1 };
    }

    fn hits(&self) -> u32 {
        self.hits
    }
}

fn main() {
    let counter = Counter::new();
    let (first, second) = (&counter, &counter);
    first.record();
    second.record();
    println!("hits: {}", counter.hits());
    assert_eq!(counter.hits(), 2);
}
