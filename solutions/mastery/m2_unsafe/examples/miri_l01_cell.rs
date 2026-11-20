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

use std::cell::Cell;

/// Counts events. `record` takes `&self`, so that several parts of a program
/// can share one counter.
struct Counter {
    // FIX: a value that changes behind `&self` must live in a type built on
    // `UnsafeCell`. `Cell` is the safe one for `Copy` values, so the `unsafe`
    // block is gone altogether. A `&Counter` no longer promises the compiler
    // that `hits` stays the same.
    hits: Cell<u32>,
}

impl Counter {
    fn new() -> Counter {
        Counter { hits: Cell::new(0) }
    }

    fn record(&self) {
        self.hits.set(self.hits.get() + 1);
    }

    fn hits(&self) -> u32 {
        self.hits.get()
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
