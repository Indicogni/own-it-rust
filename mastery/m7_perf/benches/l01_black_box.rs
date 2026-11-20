//! Lesson 01: what `black_box` changes in a benchmark. See LABS.md, "Lab 1".
//!
//! Run: cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l01_black_box
//!
//! Each group runs the same function on the same input twice. Only the first
//! benchmark of each group measures the function.

use criterion::{Criterion, criterion_group, criterion_main};
use m7_perf::l01_bench_design::{digit_sum, sum_of_squares};
use std::hint::black_box;

const INPUT: u64 = 1_234_567_890_123_456_789;

fn known_input(c: &mut Criterion) {
    let mut group = c.benchmark_group("l01_digit_sum");

    // Correct. The input passes through `black_box`, so the compiler must
    // build code that works for any number. `iter` passes the result of the
    // closure through `black_box` as well, so the result counts as used.
    group.bench_function("opaque_input", |b| b.iter(|| digit_sum(black_box(INPUT))));

    // Wrong: the compiler knows the input. It can run the loop while it
    // compiles and leave only the answer, 90, in the program.
    group.bench_function("constant_input", |b| b.iter(|| digit_sum(INPUT)));

    group.finish();
}

fn unused_result(c: &mut Criterion) {
    let values: Vec<u64> = (0..1_000).collect();
    let mut group = c.benchmark_group("l01_sum_of_squares");

    // Correct: the closure returns the sum, and `iter` treats it as used.
    group.bench_function("result_used", |b| {
        b.iter(|| sum_of_squares(black_box(&values)))
    });

    // Wrong: the sum is thrown away. The loop has no other effect, so the
    // compiler may remove all of it.
    group.bench_function("result_discarded", |b| {
        b.iter(|| {
            let _ = sum_of_squares(black_box(&values));
        })
    });

    group.finish();
}

criterion_group!(benches, known_input, unused_result);
criterion_main!(benches);
