//! LESSON 09 — Retry loops, orderings, and one-time initialization
//! Run: cargo test --test chapter19 l09_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. What can another thread do between your atomic `load` and the `store` that follows it?
//! 2. A reader sees `ready == true`. Why does that alone not prove that it reads the published value?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Explain why a retry loop continues with the value that the failed exchange returned, and what a Release store followed by an Acquire load guarantees that two Relaxed operations do not.
//! COMPILER LAB (examples/ch19_scratch.rs): Write `flag.load(Ordering::Release)` on an AtomicBool and read the lint. Then let two threads call `get_or_init` on one `static OnceLock<String>` and print how often the closure ran.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// A. Claim a free slot. `owner` holds 0 while the slot is free, otherwise the
/// id of its owner. If the slot is free, make `id` the owner and return Ok(()).
/// Otherwise change nothing and return Err(current owner). When several
/// threads call this at the same time, exactly one may win. Use one
/// `compare_exchange` (the strong form, no loop) with Relaxed for both
/// orderings: this number protects no other memory. `id` is never 0.
fn claim(owner: &AtomicUsize, id: usize) -> Result<(), usize> {
    todo!("A: implement the contract")
}

/// B. Your own `try_update`. Load the value, ask `next` what the new value
/// should be, and install it with `compare_exchange_weak`. If the exchange
/// fails (another thread changed the value in between, or the weak exchange
/// failed for no reason), try again with the value that the failed exchange
/// reported. Return Ok(previous value) after a successful exchange. Return
/// Err(current value) as soon as `next` answers None, without writing.
/// Write the loop yourself: do not call `fetch_update`, `try_update` or
/// `update`. Relaxed is enough for every ordering here.
fn update_with(cell: &AtomicU64, next: impl Fn(u64) -> Option<u64>) -> Result<u64, u64> {
    todo!("B: implement the contract")
}

/// A boolean flag for tasks C and D. Use it exactly like an `AtomicBool`: it
/// has the same `load` and `store`. It also notes whether you passed
/// `Ordering::Relaxed`, so that a test can check your choice of ordering,
/// which no test could observe from the outside. And a test can ask `store`
/// to wait, right after it has set the flag, until a reader has looked; that
/// shows a reader exactly what is visible at the moment the flag appears.
/// (Given code; do not edit.)
struct Flag {
    bit: AtomicBool,
    relaxed_store: AtomicBool,
    relaxed_load: AtomicBool,
    /// Set by a test: `store` then waits for `reader_done` before it returns.
    hold_after_store: AtomicBool,
    reader_done: AtomicBool,
}
impl Flag {
    fn new() -> Self {
        Flag {
            bit: AtomicBool::new(false),
            relaxed_store: AtomicBool::new(false),
            relaxed_load: AtomicBool::new(false),
            hold_after_store: AtomicBool::new(false),
            reader_done: AtomicBool::new(false),
        }
    }
    fn store(&self, value: bool, order: Ordering) {
        if order == Ordering::Relaxed {
            self.relaxed_store.store(true, Ordering::Relaxed);
        }
        self.bit.store(value, order);
        if self.hold_after_store.load(Ordering::SeqCst) {
            while !self.reader_done.load(Ordering::SeqCst) {
                std::thread::yield_now();
            }
        }
    }
    fn load(&self, order: Ordering) -> bool {
        if order == Ordering::Relaxed {
            self.relaxed_load.store(true, Ordering::Relaxed);
        }
        self.bit.load(order)
    }
}
/// A value that one writer publishes once: the value first, then the flag.
/// (Given code; do not edit.)
struct Announced {
    value: AtomicU64,
    ready: Flag,
}
impl Announced {
    fn new() -> Self {
        Announced {
            value: AtomicU64::new(0),
            ready: Flag::new(),
        }
    }
}

/// C. Publish: store `value` in `slot.value`, then set `slot.ready`. Called at
/// most once per slot. Together with D it must guarantee this: a reader that
/// sees the flag also sees the value. Choose the weakest ordering that is
/// correct for each of the two stores. One of them may be Relaxed.
fn announce(slot: &Announced, value: u64) {
    todo!("C: implement the contract")
}

/// D. Read: Some(value) if the flag is set, None otherwise. Choose the weakest
/// ordering that is correct for each of the two loads.
fn peek(slot: &Announced) -> Option<u64> {
    todo!("D: implement the contract")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{guess, within};
    use std::{
        cell::Cell,
        collections::HashSet,
        sync::{Barrier, LazyLock, OnceLock},
        thread,
    };

    #[test]
    fn a_first_claim_wins() {
        let (first, second, third, owner) = within(
            10,
            "claim on a slot that is already taken (return Err at once; do not wait for the slot)",
            || {
                let owner = AtomicUsize::new(0);
                let first = claim(&owner, 7);
                let second = claim(&owner, 9);
                let third = claim(&owner, 7);
                (first, second, third, owner.into_inner())
            },
        );
        assert_eq!(first, Ok(()));
        assert_eq!(
            second,
            Err(7),
            "a later claim must fail and report the current owner"
        );
        assert_eq!(third, Err(7));
        assert_eq!(owner, 7, "a failed claim must not change the owner");
    }
    #[test]
    fn a_exactly_one_racing_claim_wins() {
        // Four threads race for 2000 slots, one slot after another. For each
        // slot they first line up: every thread reports its arrival and then
        // spins until the last one to arrive opens the gate. So all of them
        // call `claim` within a few nanoseconds of each other.
        // A correct `claim` passes every time. A `claim` that loads, compares
        // and stores in separate steps lets two threads win some slot in
        // practically every run; if it ever passes, that is luck.
        const RACERS: usize = 4;
        const SLOTS: usize = 2000;
        // results[id - 1][slot] is what thread `id` was told about `slot`;
        // owners[slot] is what the slot holds at the end.
        let (results, owners): (Vec<Vec<Result<(), usize>>>, Vec<usize>) = within(
            60,
            "the race for 2000 slots (claim must return at once, also when it loses)",
            || {
                let slots: Vec<AtomicUsize> = (0..SLOTS).map(|_| AtomicUsize::new(0)).collect();
                let arrived = AtomicUsize::new(0);
                let gate = AtomicUsize::new(0); // number of slots opened so far
                let results = thread::scope(|s| {
                    let racers: Vec<_> = (1..=RACERS)
                        .map(|id| {
                            let (slots, arrived, gate) = (&slots, &arrived, &gate);
                            s.spawn(move || {
                                let mut told = Vec::with_capacity(SLOTS);
                                for (index, slot) in slots.iter().enumerate() {
                                    let before_me = arrived.fetch_add(1, Ordering::SeqCst);
                                    if before_me + 1 == RACERS * (index + 1) {
                                        // Everybody else is already spinning below.
                                        gate.store(index + 1, Ordering::SeqCst);
                                    }
                                    let mut spins = 0_u32;
                                    while gate.load(Ordering::SeqCst) <= index {
                                        spins += 1;
                                        if spins.is_multiple_of(128) {
                                            thread::yield_now();
                                        } else {
                                            std::hint::spin_loop();
                                        }
                                    }
                                    told.push(claim(slot, id));
                                }
                                told
                            })
                        })
                        .collect();
                    racers.into_iter().map(|r| r.join().unwrap()).collect()
                });
                let owners = slots
                    .iter()
                    .map(|slot| slot.load(Ordering::SeqCst))
                    .collect();
                (results, owners)
            },
        );
        for index in 0..SLOTS {
            let winners: Vec<usize> = (1..=RACERS)
                .filter(|id| results[id - 1][index].is_ok())
                .collect();
            assert_eq!(
                winners.len(),
                1,
                "slot {index}: threads {winners:?} were all told Ok; the check and \
                 the write must be one compare_exchange"
            );
            let winner = winners[0];
            assert_eq!(
                owners[index], winner,
                "slot {index}: the stored owner is not the thread that was told Ok"
            );
            assert!(
                (1..=RACERS).all(|id| id == winner || results[id - 1][index] == Err(winner)),
                "slot {index}: every loser must be told that thread {winner} owns the slot"
            );
        }
    }
    #[test]
    fn b_update_applies_the_function_once() {
        let cell = AtomicU64::new(20);
        assert_eq!(update_with(&cell, |v| Some(v + 1)), Ok(20));
        assert_eq!(cell.load(Ordering::Relaxed), 21);
        assert_eq!(
            update_with(&cell, |v| v.checked_sub(100)),
            Err(21),
            "when `next` answers None, return Err(current value)"
        );
        assert_eq!(
            cell.load(Ordering::Relaxed),
            21,
            "nothing may be written when `next` answers None"
        );
    }
    #[test]
    fn b_a_change_between_load_and_exchange_forces_a_retry() {
        let (result, final_value, calls) = within(
            10,
            "update_with (after a failed exchange, continue with the value that the \
             exchange returned; a loop that keeps its old value never succeeds)",
            || {
                let cell = AtomicU64::new(1);
                let calls = Cell::new(0_u32);
                let result = update_with(&cell, |seen| {
                    calls.set(calls.get() + 1);
                    if calls.get() == 1 {
                        // This plays the part of another thread that gets in
                        // between your load and your exchange.
                        cell.fetch_add(100, Ordering::Relaxed);
                    }
                    Some(seen * 2)
                });
                (result, cell.load(Ordering::Relaxed), calls.get())
            },
        );
        assert_eq!(
            final_value, 202,
            "the value was 1, then another thread added 100, so doubling must give \
             202. A result of 2 means the other thread's update was overwritten: \
             write with compare_exchange_weak, not with store"
        );
        assert_eq!(
            result,
            Ok(101),
            "return the value that the successful exchange replaced"
        );
        assert!(
            calls >= 2,
            "`next` must be asked again after the failed exchange"
        );
    }
    #[test]
    fn b_a_retry_can_end_in_none() {
        let (result, final_value) = within(
            10,
            "update_with (after a failed exchange, continue with the value that the \
             exchange returned)",
            || {
                let cell = AtomicU64::new(1);
                let first_call = Cell::new(true);
                let result = update_with(&cell, |seen| {
                    if first_call.replace(false) {
                        cell.fetch_add(100, Ordering::Relaxed);
                    }
                    (seen < 50).then_some(seen + 1)
                });
                (result, cell.load(Ordering::Relaxed))
            },
        );
        assert_eq!(
            result,
            Err(101),
            "the second look found 101, which `next` refuses: return Err(101)"
        );
        assert_eq!(
            final_value, 101,
            "nothing may be written after `next` answers None"
        );
    }
    #[test]
    fn b_concurrent_updates_are_not_lost() {
        let (total, granted) = within(
            30,
            "update_with under contention (a failed exchange must lead to a new \
             attempt with the reported value)",
            || {
                let total = AtomicU64::new(0);
                let limited = AtomicU64::new(0);
                let granted = AtomicU64::new(0);
                thread::scope(|s| {
                    for _ in 0..4 {
                        s.spawn(|| {
                            for _ in 0..20_000 {
                                update_with(&total, |v| Some(v + 1)).unwrap();
                            }
                            for _ in 0..100 {
                                // At most ten of these 400 attempts may succeed.
                                if update_with(&limited, |v| (v < 10).then_some(v + 1)).is_ok() {
                                    granted.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        });
                    }
                });
                (total.into_inner(), granted.into_inner())
            },
        );
        assert_eq!(total, 80_000, "an increment was lost");
        assert_eq!(
            granted, 10,
            "the limit was 10, but {granted} updates reported success"
        );
    }
    #[test]
    fn c_d_nothing_is_visible_before_announce() {
        let slot = Announced::new();
        assert_eq!(peek(&slot), None);
        announce(&slot, 9);
        assert_eq!(peek(&slot), Some(9));
        assert_eq!(peek(&slot), Some(9), "peek must not consume the value");
        let zero = Announced::new();
        announce(&zero, 0);
        assert_eq!(
            peek(&zero),
            Some(0),
            "0 is a value too: the flag decides, not the value"
        );
    }
    #[test]
    fn c_d_a_reader_that_sees_the_flag_sees_the_value() {
        // A reader polls while the writer announces. If the flag were set
        // before the value is stored, the reader could get the initial 0.
        // This check needs luck to catch that; the next test is exact.
        within(
            30,
            "announce and peek (the reader polls until peek returns Some, so \
             announce must set the flag)",
            || {
                for round in 1..=300_u64 {
                    let slot = Announced::new();
                    thread::scope(|s| {
                        let reader = s.spawn(|| {
                            loop {
                                if let Some(value) = peek(&slot) {
                                    break value;
                                }
                                thread::yield_now();
                            }
                        });
                        announce(&slot, round);
                        assert_eq!(
                            reader.join().unwrap(),
                            round,
                            "the reader saw the flag but not the value: store the \
                             value first, then set the flag"
                        );
                    });
                }
            },
        );
    }
    #[test]
    fn c_d_the_flag_is_set_after_the_value() {
        // The writer is held inside the store that sets the flag until the
        // reader has called `peek`. So the reader sees exactly what a fast
        // reader could see at the moment the flag appears.
        /// Sets a signal when it is dropped, also during a panic, so that
        /// neither thread waits for the other forever.
        struct Raise<'a>(&'a AtomicBool);
        impl Drop for Raise<'_> {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let seen = within(30, "announce with a reader that waits for the flag", || {
            let slot = Announced::new();
            let writer_returned = AtomicBool::new(false);
            slot.ready.hold_after_store.store(true, Ordering::SeqCst);
            thread::scope(|s| {
                let reader = s.spawn(|| {
                    let _done = Raise(&slot.ready.reader_done);
                    while !slot.ready.bit.load(Ordering::SeqCst)
                        && !writer_returned.load(Ordering::SeqCst)
                    {
                        thread::yield_now();
                    }
                    peek(&slot)
                });
                {
                    let _returned = Raise(&writer_returned);
                    announce(&slot, 41);
                }
                reader.join().expect("peek panicked")
            })
        });
        assert_eq!(
            seen,
            Some(41),
            "a reader looked at the moment the flag appeared (or, if the flag \
             never appeared, when announce returned) and did not find 41: \
             store the value first, then set the flag"
        );
    }
    #[test]
    fn c_d_the_flag_uses_release_and_acquire() {
        let slot = Announced::new();
        announce(&slot, 5);
        assert_eq!(peek(&slot), Some(5));
        assert!(
            !slot.ready.relaxed_store.load(Ordering::Relaxed),
            "`ready` was stored with Relaxed. A Relaxed store promises nothing \
             about the earlier store to `value`; Release does"
        );
        assert!(
            !slot.ready.relaxed_load.load(Ordering::Relaxed),
            "`ready` was loaded with Relaxed. Only an Acquire load of the flag \
             guarantees that the writer's earlier store to `value` is visible"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. The same compare_exchange twice in a row.
        let state = AtomicU64::new(8);
        let first = state.compare_exchange(8, 9, Ordering::Relaxed, Ordering::Relaxed);
        let second = state.compare_exchange(8, 9, Ordering::Relaxed, Ordering::Relaxed);
        let p1: (Result<u64, u64>, Result<u64, u64>) = guess();
        assert_eq!(p1, (first, second), "P1: what compare_exchange returns");

        // P2. A OnceLock is set twice, then asked to initialize itself.
        let cell: OnceLock<u32> = OnceLock::new();
        let before = cell.get().copied();
        let _ = cell.set(1);
        let second_set = cell.set(2);
        let later = *cell.get_or_init(|| 3);
        let p2: (Option<u32>, Result<(), u32>, u32) = guess();
        assert_eq!(
            p2,
            (before, second_set, later),
            "P2: a OnceLock is written once"
        );

        // P3. How often has the closure run before the first use of `table`,
        // and after two uses?
        let runs = Cell::new(0_u32);
        let table = LazyLock::new(|| {
            runs.set(runs.get() + 1);
            vec![10, 20, 30]
        });
        let runs_before_use = runs.get();
        let total = table.len() + table.len();
        let p3: (u32, u32) = guess();
        assert_eq!(total, 6);
        assert_eq!(
            p3,
            (runs_before_use, runs.get()),
            "P3: LazyLock is lazy, and runs once"
        );

        // P4. Four threads start together. Each offers its own number to
        // get_or_init. How many closures run, and how many different values do
        // the four threads get back?
        let cell: OnceLock<usize> = OnceLock::new();
        let runs = AtomicUsize::new(0);
        let start = Barrier::new(4);
        let seen: Vec<usize> = thread::scope(|s| {
            let threads: Vec<_> = (0..4)
                .map(|number| {
                    let (cell, runs, start) = (&cell, &runs, &start);
                    s.spawn(move || {
                        start.wait();
                        *cell.get_or_init(|| {
                            runs.fetch_add(1, Ordering::Relaxed);
                            number
                        })
                    })
                })
                .collect();
            threads.into_iter().map(|t| t.join().unwrap()).collect()
        });
        let distinct = seen.iter().collect::<HashSet<_>>().len();
        let p4: (usize, usize) = guess();
        assert_eq!(p4, (runs.into_inner(), distinct), "P4: racing initializers");
    }
}
