//! Lesson 05: the same work in two traversal orders, on real memory. Run it
//! after you have finished tasks A and B of lesson 05; before that it stops
//! at `todo!()`.
//!
//! Run: cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l05_traversal
//!
//! The grid has 2,048 rows and 2,048 columns of `u32`: 16 MiB, more than the
//! inner caches of most processors hold.

use criterion::{Criterion, criterion_group, criterion_main};
use m7_perf::l05_traversal::{column_sums, column_sums_by_column, transpose};
use std::hint::black_box;

const N: usize = 2_048;

/// The transpose with two plain loops: along the rows of `src`, down the
/// columns of `dst`.
fn transpose_plain(src: &[u32], dst: &mut [u32], n: usize) {
    for row in 0..n {
        for column in 0..n {
            dst[column * n + row] = src[row * n + column];
        }
    }
}

fn traversal(c: &mut Criterion) {
    let grid: Vec<u32> = (0..N * N).map(|i| i as u32 % 1_000).collect();
    assert_eq!(
        column_sums(&grid[..], N),
        column_sums_by_column(&grid[..], N)
    );

    let mut group = c.benchmark_group("l05_column_sums");
    group.sample_size(20);
    group.bench_function("column_by_column", |b| {
        b.iter(|| column_sums_by_column(black_box(&grid[..]), N))
    });
    group.bench_function("yours", |b| b.iter(|| column_sums(black_box(&grid[..]), N)));
    group.finish();

    let mut target = vec![0u32; N * N];
    let mut group = c.benchmark_group("l05_transpose");
    group.sample_size(20);
    group.bench_function("plain_loops", |b| {
        b.iter(|| transpose_plain(black_box(&grid), black_box(&mut target), N))
    });
    group.bench_function("yours", |b| {
        b.iter(|| transpose(black_box(&grid[..]), black_box(&mut target[..]), N))
    });
    group.finish();
}

criterion_group!(benches, traversal);
criterion_main!(benches);
