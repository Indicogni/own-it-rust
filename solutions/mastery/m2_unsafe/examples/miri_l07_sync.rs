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

use std::sync::atomic::{AtomicU64, Ordering};

/// Counts events from several threads.
struct Hits {
    // FIX: `count += 1` is a read followed by a write. Two threads that do
    // this at the same time are a data race, whatever the values are. An
    // atomic integer makes each step one indivisible operation, and it is
    // `Sync` by itself, so the `unsafe impl` and the `unsafe` blocks are gone.
    count: AtomicU64,
}

impl Hits {
    fn record(&self) {
        self.count.fetch_add(1, Ordering::Relaxed);
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
