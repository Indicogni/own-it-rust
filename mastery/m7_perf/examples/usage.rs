//! Prints what the capstone's baseline and your version ask of the heap.
//!
//! Run: cargo run --release --manifest-path mastery/m7_perf/Cargo.toml --example usage
//!
//! The numbers are counts, not times: they are the same on every run. Until
//! you replace the bodies in `src/l08_capstone.rs`, both rows are equal.

use m7_perf::l08_capstone::{baseline, sample_log, summarize};
use m7_perf::support::alloc::{Counting, Usage, measure};
use std::io;

// This program wants the numbers, so it installs the counting allocator. The
// benchmarks do not: the counting would slow down what they measure.
#[global_allocator]
static ALLOC: Counting = Counting;

fn row(name: &str, lines: usize, usage: Usage) {
    println!(
        "{name:<10} {lines:>8} {:>12} {:>14} {:>12}",
        usage.allocations, usage.bytes, usage.peak
    );
}

fn main() -> io::Result<()> {
    println!(
        "{:<10} {:>8} {:>12} {:>14} {:>12}",
        "version", "lines", "allocations", "bytes asked", "peak bytes"
    );
    for lines in [10_000, 40_000] {
        let log = sample_log(lines, 1);
        let (result, usage) = measure(|| baseline::summarize(log.as_bytes(), io::sink(), 5));
        result?;
        row("baseline", lines, usage);
        let (result, usage) = measure(|| summarize(log.as_bytes(), io::sink(), 5));
        result?;
        row("yours", lines, usage);
    }
    Ok(())
}
