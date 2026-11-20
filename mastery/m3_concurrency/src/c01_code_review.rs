//! CODE REVIEW 01 — A teammate's lock-free slot pool
//! Run: cargo test c01_
//! Model: cargo model c01_
//! Then: cargo clippy --all-targets
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! it is entirely safe Rust, it "passed the stress test on my machine", and
//! some of its tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md.
//!    For each problem, write the interleaving (or the input) that shows it.
//!    There are four real bugs. Only one of them shows itself with a single
//!    thread on every machine; the others need a second thread that runs at
//!    the wrong moment.
//! 2. Run the tests, then the model tests. Compare what they found with your
//!    review. A model test prints the failing assertion; work out the
//!    interleaving that leads to it before you change anything.
//! 3. Fix the code. Keep the public names and signatures. You may change the
//!    private fields and their types.
//! 4. Run clippy and resolve its warnings for this file.

use crate::sync::atomic::{AtomicUsize, Ordering};

/// Marks the end of the free list.
const NIL: usize = usize::MAX;

/// Hands out the slot numbers `0..slots`, for example indices into a table of
/// connections. A slot number is held by at most one caller at a time.
///
/// The free slots form a linked stack: `head` is a free slot (or `NIL`), and
/// `next[i]` is the free slot below slot `i`. `acquire` pops the top slot and
/// `release` pushes a slot back. Any number of threads may call both.
pub struct SlotPool {
    head: AtomicUsize,
    next: Vec<AtomicUsize>,
}

impl SlotPool {
    /// A pool with the slots `0..slots`, all free. `slots` may be 0.
    pub fn new(slots: usize) -> SlotPool {
        let mut next = Vec::new();
        for i in 0..slots {
            next.push(AtomicUsize::new((i + 1) % slots));
        }
        SlotPool {
            head: AtomicUsize::new(0),
            next: next,
        }
    }

    /// Takes a free slot, or returns `None` if every slot is in use.
    pub fn acquire(&self) -> Option<usize> {
        loop {
            let head = self.head.load(Ordering::Relaxed);
            if head == NIL {
                return None;
            }
            let next = self.next[head].load(Ordering::Relaxed);
            if self
                .head
                .compare_exchange(head, next, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return Some(head);
            }
        }
    }

    /// Gives `slot` back. The caller must hold `slot` (it came from `acquire`
    /// and was not released since).
    pub fn release(&self, slot: usize) {
        let head = self.head.load(Ordering::Relaxed);
        self.next[slot].store(head, Ordering::Relaxed);
        // If another thread got in between, never mind: it is only a pool.
        let _ = self
            .head
            .compare_exchange_weak(head, slot, Ordering::Relaxed, Ordering::Relaxed);
    }

    /// Is every slot in use at this moment?
    pub fn is_exhausted(&self) -> bool {
        if self.head.load(Ordering::Relaxed) == NIL {
            return true;
        } else {
            return false;
        }
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use crate::support::{alloc::count, within};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn a_each_slot_is_handed_out_once() {
        let pool = SlotPool::new(3);
        let mut slots: Vec<usize> = (0..3)
            .map(|_| pool.acquire().expect("three slots are free"))
            .collect();
        slots.sort_unstable();
        assert_eq!(
            slots,
            [0, 1, 2],
            "the pool must hand out exactly the slots 0, 1 and 2"
        );
        assert!(pool.is_exhausted());
        assert_eq!(
            pool.acquire(),
            None,
            "all three slots are in use, so a fourth `acquire` must return None"
        );
    }

    #[test]
    fn a_an_empty_pool_has_nothing_to_give() {
        let pool = SlotPool::new(0);
        assert!(pool.is_exhausted());
        assert_eq!(pool.acquire(), None);
    }

    #[test]
    fn b_a_released_slot_can_be_acquired_again() {
        let pool = SlotPool::new(2);
        let first = pool.acquire().unwrap();
        let second = pool.acquire().unwrap();
        assert_ne!(first, second, "two callers must never hold the same slot");
        pool.release(first);
        assert!(!pool.is_exhausted());
        assert_eq!(
            pool.acquire(),
            Some(first),
            "the only free slot is the one that was just released"
        );
    }

    #[test]
    fn b_no_slot_is_lost_over_many_rounds() {
        let pool = SlotPool::new(4);
        for round in 0..50 {
            let held: Vec<usize> = std::iter::from_fn(|| pool.acquire()).take(5).collect();
            assert_eq!(
                held.len(),
                4,
                "round {round}: the pool has 4 slots, and all were released in the round before"
            );
            for slot in held {
                pool.release(slot);
            }
        }
    }

    #[test]
    fn c_acquire_and_release_do_not_allocate() {
        let pool = SlotPool::new(8);
        let (slot, allocations) = count(|| {
            let slot = pool.acquire();
            if let Some(slot) = slot {
                pool.release(slot);
            }
            slot
        });
        assert!(slot.is_some());
        assert_eq!(
            allocations, 0,
            "acquire and release made {allocations} heap allocation(s); they only \
             change a few atomics"
        );
    }

    #[test]
    fn d_threads_never_share_a_slot() {
        let rounds = if cfg!(miri) { 30 } else { 20_000 };
        within(60, "the slot pool stress test", move || {
            let pool = Arc::new(SlotPool::new(3));
            let in_use: Arc<Vec<AtomicBool>> =
                Arc::new((0..3).map(|_| AtomicBool::new(false)).collect());
            let workers: Vec<_> = (0..4)
                .map(|_| {
                    let (pool, in_use) = (Arc::clone(&pool), Arc::clone(&in_use));
                    std::thread::spawn(move || {
                        for _ in 0..rounds {
                            if let Some(slot) = pool.acquire() {
                                assert!(
                                    !in_use[slot].swap(true, Ordering::SeqCst),
                                    "slot {slot} was handed to two threads at the same time"
                                );
                                in_use[slot].store(false, Ordering::SeqCst);
                                pool.release(slot);
                            }
                        }
                    })
                })
                .collect();
            for worker in workers {
                worker.join().unwrap();
            }
            let mut left: Vec<usize> = std::iter::from_fn(|| pool.acquire()).take(4).collect();
            left.sort_unstable();
            assert_eq!(
                left,
                [0, 1, 2],
                "after every thread released its slot, all three slots must be free"
            );
        });
    }
}

#[cfg(all(test, loom))]
mod model {
    use super::*;
    use crate::support::model;
    use crate::sync::{Arc, thread};

    #[test]
    fn a_two_releases_at_once_lose_nothing() {
        model(|| {
            let pool = Arc::new(SlotPool::new(2));
            let first = pool.acquire().expect("two slots are free");
            let second = pool.acquire().expect("one slot is free");
            let other = {
                let pool = Arc::clone(&pool);
                thread::spawn(move || pool.release(second))
            };
            pool.release(first);
            other.join().unwrap();
            let mut free = vec![pool.acquire(), pool.acquire()];
            free.sort_unstable();
            assert_eq!(
                free,
                [Some(0), Some(1)],
                "both slots were released, so both must be available again"
            );
            assert_eq!(pool.acquire(), None);
        });
    }

    #[test]
    fn b_a_slow_acquire_does_not_hand_out_a_held_slot() {
        model(|| {
            let pool = Arc::new(SlotPool::new(2));
            // The other thread takes both slots, gives the first one back and
            // keeps the second one.
            let other = {
                let pool = Arc::clone(&pool);
                thread::spawn(move || {
                    let first = pool.acquire();
                    let kept = pool.acquire();
                    if let Some(first) = first {
                        pool.release(first);
                    }
                    kept
                })
            };
            let mine = pool.acquire();
            let kept = other.join().unwrap();
            // Whatever is still free now:
            let rest = [pool.acquire(), pool.acquire()];
            let mut held: Vec<usize> = [mine, kept, rest[0], rest[1]]
                .into_iter()
                .flatten()
                .collect();
            held.sort_unstable();
            assert_eq!(
                held,
                [0, 1],
                "the slots held at the end must be exactly 0 and 1, each held once: \
                 a slot was handed out twice, or a slot disappeared"
            );
        });
    }

    #[test]
    fn c_an_acquire_sees_the_links_of_the_release_before_it() {
        model(|| {
            let pool = Arc::new(SlotPool::new(2));
            let first = pool.acquire().expect("two slots are free");
            let kept = pool.acquire().expect("one slot is free");
            let releaser = {
                let pool = Arc::clone(&pool);
                thread::spawn(move || pool.release(first))
            };
            let taker = {
                let pool = Arc::clone(&pool);
                thread::spawn(move || (pool.acquire(), pool.acquire()))
            };
            releaser.join().unwrap();
            let (one, two) = taker.join().unwrap();
            for slot in [one, two].into_iter().flatten() {
                assert_ne!(
                    slot, kept,
                    "slot {kept} was never released, but `acquire` handed it out again"
                );
            }
            assert!(
                !(one.is_some() && two.is_some()),
                "only one slot was free, but two `acquire` calls succeeded"
            );
        });
    }
}
