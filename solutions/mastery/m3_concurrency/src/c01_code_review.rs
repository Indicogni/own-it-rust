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

use crate::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Marks the end of the free list.
const NIL: u32 = u32::MAX;

/// Hands out the slot numbers `0..slots`, for example indices into a table of
/// connections. A slot number is held by at most one caller at a time.
///
/// The free slots form a linked stack: `head` names a free slot (or `NIL`),
/// and `next[i]` is the free slot below slot `i`. `acquire` pops the top slot
/// and `release` pushes a slot back. Any number of threads may call both.
///
/// Invariants:
/// - Following `head` and then the `next` links visits every free slot exactly
///   once and ends at `NIL`.
/// - `head` holds a version number next to the slot. Every successful
///   `acquire` adds one to the version. So if `head` still has the value that
///   a thread read earlier, no slot was acquired in between, and therefore no
///   slot on the list was released again with a different link.
pub struct SlotPool {
    /// Version in the high 32 bits, top slot in the low 32 bits.
    head: AtomicU64,
    next: Box<[AtomicU32]>,
}

fn pack(version: u32, slot: u32) -> u64 {
    (u64::from(version) << 32) | u64::from(slot)
}

fn unpack(head: u64) -> (u32, u32) {
    // The casts keep the high half and the low half; nothing else is lost.
    ((head >> 32) as u32, head as u32)
}

impl SlotPool {
    /// A pool with the slots `0..slots`, all free. `slots` may be 0.
    pub fn new(slots: usize) -> SlotPool {
        let slots = u32::try_from(slots)
            .ok()
            .filter(|&slots| slots < NIL)
            .expect("a slot pool holds fewer than u32::MAX slots");
        // Slot `i` links to slot `i + 1`; the last slot ends the list.
        let next = (0..slots)
            .map(|i| AtomicU32::new(if i + 1 == slots { NIL } else { i + 1 }))
            .collect();
        let top = if slots == 0 { NIL } else { 0 };
        SlotPool {
            head: AtomicU64::new(pack(0, top)),
            next,
        }
    }

    /// Takes a free slot, or returns `None` if every slot is in use.
    pub fn acquire(&self) -> Option<usize> {
        // Acquire: pairs with the Release in the `release` that pushed the top
        // slot, so that the link it wrote into `next` is visible below. Every
        // change of `head` is a compare-exchange, so the load synchronizes
        // with every earlier `release` as well.
        let mut head = self.head.load(Ordering::Acquire);
        loop {
            let (version, slot) = unpack(head);
            if slot == NIL {
                return None;
            }
            // Relaxed: the load above already ordered the write of this link
            // before us. If the slot was acquired and released again since
            // then, the link may be newer, but then the version differs and
            // the exchange below fails.
            let next = self.next[slot as usize].load(Ordering::Relaxed);
            // Comparing version and slot together is what rules out ABA: the
            // same slot on top with a different list below it has a different
            // version. On success nothing new is published (Relaxed). On
            // failure the next iteration follows a link of the new top slot,
            // so the new value must be loaded with Acquire.
            match self.head.compare_exchange_weak(
                head,
                pack(version.wrapping_add(1), next),
                Ordering::Relaxed,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(slot as usize),
                Err(now) => head = now,
            }
        }
    }

    /// Gives `slot` back. The caller must hold `slot` (it came from `acquire`
    /// and was not released since).
    pub fn release(&self, slot: usize) {
        assert!(
            slot < self.next.len(),
            "slot {slot} is not part of this pool"
        );
        let slot = slot as u32;
        // Relaxed: this thread does not follow the link of the current top
        // slot; it only stores the slot number.
        let mut head = self.head.load(Ordering::Relaxed);
        loop {
            let (version, top) = unpack(head);
            // The caller holds `slot`, so no other thread writes its link. A
            // slow `acquire` that saw `slot` on top earlier may still load
            // the link (it is an atomic, so that is no data race), but the
            // version has changed since then and its exchange will fail.
            self.next[slot as usize].store(top, Ordering::Relaxed);
            // Release: the link stored above happens before any Acquire load
            // that sees `slot` on top. A failed exchange means that another
            // thread changed the list; try again with the new top. The slot
            // must never be dropped on the floor.
            match self.head.compare_exchange_weak(
                head,
                pack(version, slot),
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(now) => head = now,
            }
        }
    }

    /// Is every slot in use at this moment?
    pub fn is_exhausted(&self) -> bool {
        unpack(self.head.load(Ordering::Relaxed)).1 == NIL
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
