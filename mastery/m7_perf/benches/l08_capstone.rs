//! Lesson 08: the baseline against your version, in time.
//!
//! Run: cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l08_capstone
//!
//! Until you replace the bodies in `src/l08_capstone.rs`, "yours" is the
//! baseline and both lines report the same time. Criterion remembers the last
//! run: after a change it prints how much "yours" has moved.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use m7_perf::l08_capstone::{baseline, sample_log, summarize, top_k};
use std::hint::black_box;
use std::io;

fn summaries(c: &mut Criterion) {
    let log = sample_log(50_000, 1);

    let mut group = c.benchmark_group("l08_summarize");
    // Lets criterion also report bytes per second.
    group.throughput(Throughput::Bytes(log.len() as u64));
    group.sample_size(30);
    group.bench_function("baseline", |b| {
        b.iter(|| baseline::summarize(black_box(log.as_bytes()), io::sink(), 5))
    });
    group.bench_function("yours", |b| {
        b.iter(|| summarize(black_box(log.as_bytes()), io::sink(), 5))
    });
    group.finish();
}

fn largest(c: &mut Criterion) {
    // 100,000 different numbers in a fixed scrambled order.
    let items: Vec<u32> = (0..100_000u32)
        .map(|i| (i * 7_919 + 13) % 100_000)
        .collect();

    let mut group = c.benchmark_group("l08_top_k");
    group.bench_function("baseline", |b| {
        b.iter(|| baseline::top_k(black_box(&items).iter().copied(), 8))
    });
    group.bench_function("yours", |b| {
        b.iter(|| top_k(black_box(&items).iter().copied(), 8))
    });
    group.finish();
}

criterion_group!(benches, summaries, largest);
criterion_main!(benches);
