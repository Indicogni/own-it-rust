//! LESSON 07 — Atomic counters and local state
//! Run: cargo test --test chapter19 l07_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. Does Relaxed synchronize unrelated data?
//! 2. Do two threads share the contents of one thread_local variable?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Distinguish atomicity, ordering, and per-thread storage. Explain why a mutable static would need an invariant beyond these counters.
//! COMPILER LAB (examples/ch19_scratch.rs): Try a mutable static counter in safe code; replace it with explicit per-run state or a synchronized OnceLock. Do not execute a data race.

use std::{
    cell::Cell,
    sync::atomic::{AtomicUsize, Ordering},
    thread::{self, LocalKey},
};
/// A. Atomically reserve one ID by incrementing the counter; return the previous
/// value. At usize::MAX return None without wrapping. Relaxed is sufficient:
/// the counter allocates IDs, not publication of other memory.
fn reserve(counter: &AtomicUsize) -> Option<usize> {
    todo!("A: implement the contract")
}
/// B. `counter` is a thread-local: every thread has its own copy, which starts
/// at the initial value written in its `thread_local!` declaration. Spawn n
/// threads. Each one reads its own copy, adds 1 to it twice, and returns
/// (value before, value after). Return the results in spawn order. The calling
/// thread's copy must not change.
fn local_counters(n: usize, counter: &'static LocalKey<Cell<u32>>) -> Vec<(u32, u32)> {
    todo!("B: implement the contract")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;
    use std::sync::{Barrier, atomic::AtomicU8};
    #[test]
    fn a_reservation_never_wraps() {
        let c = AtomicUsize::new(4);
        assert_eq!(reserve(&c), Some(4));
        assert_eq!(reserve(&c), Some(5));
        assert_eq!(c.load(Ordering::Relaxed), 6);
        let max = AtomicUsize::new(usize::MAX);
        assert_eq!(
            reserve(&max),
            None,
            "at usize::MAX no ID is left: return None instead of wrapping to 0"
        );
        assert_eq!(
            max.load(Ordering::Relaxed),
            usize::MAX,
            "a refused reservation must leave the counter unchanged"
        );
        let near = AtomicUsize::new(usize::MAX - 1);
        assert_eq!(reserve(&near), Some(usize::MAX - 1));
        assert_eq!(reserve(&near), None, "the last ID was just handed out");
    }
    #[test]
    fn a_concurrent_reservations_are_unique() {
        // A race needs luck to show up, so this test tries hard: eight workers
        // start together from a barrier, and the whole experiment is repeated.
        // A correct `reserve` passes every time. A `reserve` that loads and
        // stores in two steps fails almost every time; if it ever passes, that
        // is luck, not correctness.
        const WORKERS: usize = 8;
        const PER_WORKER: usize = 5_000;
        for round in 0..10 {
            let counter = AtomicUsize::new(0);
            let start = Barrier::new(WORKERS);
            let mut ids: Vec<usize> = thread::scope(|s| {
                let workers: Vec<_> = (0..WORKERS)
                    .map(|_| {
                        s.spawn(|| {
                            start.wait();
                            (0..PER_WORKER)
                                .map(|_| reserve(&counter).unwrap())
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                workers
                    .into_iter()
                    .flat_map(|w| w.join().unwrap())
                    .collect()
            });
            ids.sort_unstable();
            assert!(
                ids.iter().copied().eq(0..WORKERS * PER_WORKER),
                "round {round}: two threads received the same ID, so the increment \
                 is not one atomic step (a separate load and store can interleave)"
            );
        }
    }
    #[test]
    fn b_each_worker_starts_fresh() {
        thread_local! {
            static HITS: Cell<u32> = const { Cell::new(0) };
        }
        // The calling thread's copy holds 40. Fresh threads start at 0.
        HITS.set(40);
        assert_eq!(
            local_counters(3, &HITS),
            vec![(0, 2); 3],
            "every worker must count in its own copy, which starts at 0; a result \
             such as (40, 42) means the counting ran on the calling thread"
        );
        assert_eq!(
            HITS.get(),
            40,
            "the calling thread's copy changed: only spawned workers may count"
        );
        assert!(local_counters(0, &HITS).is_empty());
        assert_eq!(local_counters(16, &HITS), vec![(0, 2); 16]);
        assert_eq!(HITS.get(), 40);
    }
    #[test]
    fn b_initial_value_comes_from_the_declaration() {
        thread_local! {
            static START_AT_7: Cell<u32> = const { Cell::new(7) };
        }
        assert_eq!(
            local_counters(2, &START_AT_7),
            [(7, 9), (7, 9)],
            "read the starting value from `counter`; do not assume it is 0"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. fetch_add returns which value? What does the atomic hold afterwards?
        let a = AtomicUsize::new(5);
        let returned = a.fetch_add(3, Ordering::Relaxed);
        let p1: (usize, usize) = guess();
        assert_eq!(p1, (returned, a.load(Ordering::Relaxed)), "P1: fetch_add");

        // P2. Atomic arithmetic in a debug build: panic, saturate, or wrap?
        let small = AtomicU8::new(250);
        small.fetch_add(10, Ordering::Relaxed);
        let p2: u8 = guess();
        assert_eq!(p2, small.load(Ordering::Relaxed), "P2: atomic overflow");

        // P3. compare_exchange expects 1, but the value is 8.
        let c = AtomicUsize::new(8);
        let p3: Result<usize, usize> = guess();
        assert_eq!(
            p3,
            c.compare_exchange(1, 9, Ordering::Relaxed, Ordering::Relaxed),
            "P3: a failed compare_exchange"
        );

        // P4. This thread sets its thread-local to 5. What does a new thread see?
        thread_local! {
            static SEEN: Cell<u32> = const { Cell::new(0) };
        }
        SEEN.set(5);
        let other = thread::spawn(|| SEEN.get()).join().unwrap();
        let p4: (u32, u32) = guess();
        assert_eq!(p4, (SEEN.get(), other), "P4: one thread_local per thread");
    }
}
