//! MIRI LAB 07 — A promise to the compiler (lesson 07)
//!
//! Run it under Miri, from the repository root:
//!
//!   cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l07_sync
//!
//! 1. PREDICT. Read the program. Every `unsafe` block has a SAFETY comment.
//!    Decide which comment is false or leaves something out, and what Miri
//!    will report, before you run anything.
//! 2. RUN the command above. Read the first `error:` line and the source line
//!    that it points to.
//! 3. COMPARE with "What Miri says about the wrong version" in CONCEPTS.md,
//!    lesson 07.
//! 4. REPAIR the program until Miri finishes without a report. Do not change
//!    `main`. Correct the SAFETY comments too.
//!
//! A native `cargo run --example miri_l07_sync` may print the expected result, print
//! something else, or crash. That is what undefined behaviour means: only the
//! Miri run tells you whether the program is right.

use std::cell::UnsafeCell;

/// Counts events from several threads.
struct Hits {
    count: UnsafeCell<u64>,
}

// SAFETY: the counter is only ever incremented, and a u64 has no invalid
// values, so two threads cannot leave it in a broken state.
unsafe impl Sync for Hits {}

impl Hits {
    fn record(&self) {
        // SAFETY: the pointer comes from a live `UnsafeCell`, which allows
        // writes through a shared reference.
        unsafe { *self.count.get() += 1 };
    }

    fn total(&mut self) -> u64 {
        *self.count.get_mut()
    }
}

fn main() {
    let mut hits = Hits {
        count: Default::default(),
    };
    std::thread::scope(|scope| {
        for _ in 0..2 {
            scope.spawn(|| {
                for _ in 0..100 {
                    hits.record();
                }
            });
        }
    });
    println!("total: {}", hits.total());
    assert_eq!(hits.total(), 200);
}
