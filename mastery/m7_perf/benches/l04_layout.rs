//! Lesson 04: an array of structs against a struct of arrays. Run it after
//! you have finished task D of lesson 04; before that it stops at `todo!()`.
//!
//! Run: cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l04_layout
//!
//! Both benchmarks add up `price_cents * quantity` over the open orders. The
//! first one walks over 48 bytes per order, the second one over 9.

use criterion::{Criterion, criterion_group, criterion_main};
use m7_perf::l04_layout::{Order, OrderTable, open_value_of};
use std::hint::black_box;

const ORDERS: u32 = 500_000;

fn order(i: u32) -> Order {
    Order {
        id: u64::from(i),
        price_cents: i % 10_000 + 1,
        quantity: i % 9 + 1,
        // Mostly open, in a pattern that a branch predictor cannot learn
        // by heart in a few steps.
        open: i.wrapping_mul(2_654_435_761) >> 29 != 0,
        note: [0; 24],
    }
}

fn open_value(c: &mut Criterion) {
    let orders: Vec<Order> = (0..ORDERS).map(order).collect();
    let mut table = OrderTable::with_capacity(orders.len());
    for &order in &orders {
        table.push(order);
    }
    assert_eq!(table.open_value(), open_value_of(&orders));

    let mut group = c.benchmark_group("l04_open_value");
    group.bench_function("array_of_structs", |b| {
        b.iter(|| open_value_of(black_box(&orders)))
    });
    group.bench_function("struct_of_arrays", |b| {
        b.iter(|| black_box(&table).open_value())
    });
    group.finish();
}

criterion_group!(benches, open_value);
criterion_main!(benches);
