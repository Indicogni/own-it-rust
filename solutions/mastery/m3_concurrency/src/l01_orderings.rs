//! LESSON 01 — Memory orderings and happens-before
//! Run: cargo test l01_
//! Model: cargo model l01_
//! Read first: "Lesson 01" in CONCEPTS.md (same package).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer), then `p_model_predictions` in the `model`
//! module. Then consider:
//! 1. One thread writes plain data and then sets an atomic flag. Another
//!    thread sees the flag set. What exactly allows it to read the data?
//! 2. Which of the three tasks below needs no ordering stronger than `Relaxed`?
//!
//! IMPLEMENT the tasks below. Import atomics from `crate::sync`, never from
//! `std`, and give every atomic operation the weakest ordering that your
//! argument needs. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Define happens-before. Then say, for each task, which
//! pair of operations must be ordered by it, and which ordering arguments
//! create that order.
//!
//! MODEL LAB (optional): In `SetOnce::set`, change the last store to `Relaxed`
//! and run `cargo model l01_`. Read the "Causality violation" report: it names
//! the two accesses that are not ordered. Change it back. Then make
//! `try_enter` use `SeqCst` on the store and on the load, without a fence,
//! and run the model again. Loom 0.7.3 reports a failure that a real
//! processor can never produce; CONCEPTS.md explains why, and why the tests
//! therefore ask for the fence.

use crate::sync::UnsafeCell;
use crate::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering, fence};
use std::mem::MaybeUninit;

/// A. A fixed number of permits, for example "at most 8 uploads at once".
///
/// The counter protects no other memory: a caller only learns "you got one"
/// or "none left".
pub struct Permits {
    used: AtomicUsize,
    limit: usize,
}

impl Permits {
    pub fn new(limit: usize) -> Self {
        Permits {
            used: AtomicUsize::new(0),
            limit,
        }
    }

    /// Takes one permit and returns `true`, or returns `false` when all
    /// `limit` permits are taken.
    ///
    /// `used` must never be greater than `limit`, not even for a moment, and
    /// the call must not wait for another thread. Use a compare-exchange loop.
    pub fn try_acquire(&self) -> bool {
        // Relaxed is enough: the counter is the only shared state, and a
        // compare-exchange always works on the latest value. No other memory
        // is published through it, so there is nothing to order.
        let mut used = self.used.load(Ordering::Relaxed);
        loop {
            if used >= self.limit {
                return false;
            }
            match self.used.compare_exchange_weak(
                used,
                used + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(now) => used = now,
            }
        }
    }

    /// Gives one permit back. Only call this after a successful `try_acquire`.
    pub fn release(&self) {
        self.used.fetch_sub(1, Ordering::Relaxed);
    }

    /// How many permits are taken right now. The answer may be out of date
    /// by the time the caller looks at it.
    pub fn in_use(&self) -> usize {
        self.used.load(Ordering::Relaxed)
    }
}

const EMPTY: u8 = 0;
const WRITING: u8 = 1;
const READY: u8 = 2;

/// B. A cell that can be written once, by whichever thread comes first, and
/// read by any thread afterwards. (The standard library's `OnceLock` is the
/// full version of this idea.)
///
/// `state` is `EMPTY`, then `WRITING` while one `set` call writes the value,
/// then `READY` forever. `value` is initialised exactly when `state == READY`.
pub struct SetOnce<T> {
    state: AtomicU8,
    value: UnsafeCell<MaybeUninit<T>>,
}

// SAFETY: sharing `&SetOnce<T>` lets any thread call `get`, which hands out
// `&T` (so `T: Sync`), and call `set`, which moves a `T` in from that thread
// to be dropped by whichever thread drops the cell (so `T: Send`). The state
// machine makes sure that the one write of `value` is finished, and published
// with Release/Acquire, before any read.
unsafe impl<T: Send + Sync> Sync for SetOnce<T> {}

impl<T> SetOnce<T> {
    pub fn new() -> Self {
        SetOnce {
            state: AtomicU8::new(EMPTY),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Stores `value` if the cell is still empty. If another call got there
    /// first (even if it has not finished), gives `value` back as `Err`.
    pub fn set(&self, value: T) -> Result<(), T> {
        // Claim the cell. Relaxed: this step publishes nothing, and a
        // compare-exchange cannot succeed for two threads.
        if self
            .state
            .compare_exchange(EMPTY, WRITING, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return Err(value);
        }
        // SAFETY: the compare-exchange succeeded for this call only, so this
        // is the one write of the cell. No reader touches the cell before it
        // sees READY, which is stored below.
        self.value
            .with_mut(|p| unsafe { p.cast::<T>().write(value) });
        // Release: everything this thread did so far, including the write
        // above, happens before any Acquire load that reads READY.
        self.state.store(READY, Ordering::Release);
        Ok(())
    }

    /// The stored value, or `None` while no `set` call has finished.
    pub fn get(&self) -> Option<&T> {
        // Acquire pairs with the Release store in `set`.
        if self.state.load(Ordering::Acquire) != READY {
            return None;
        }
        // SAFETY: READY was stored after the value was written, and the
        // Release/Acquire pair orders that write before this read. Nothing
        // writes the cell again while `&self` is alive, so the reference may
        // live as long as `&self`.
        Some(unsafe { &*self.value.with(|p| p.cast::<T>()) })
    }
}

impl<T> Default for SetOnce<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for SetOnce<T> {
    fn drop(&mut self) {
        // `&mut self` means that no other thread uses the cell any more, and
        // that whatever gave this thread exclusive access also ordered every
        // earlier store before this load. Relaxed therefore reads the final
        // state.
        if self.state.load(Ordering::Relaxed) == READY {
            // SAFETY: READY means the value is initialised, and nothing can
            // read it after this point.
            self.value
                .with_mut(|p| unsafe { p.cast::<T>().drop_in_place() });
        }
    }
}

/// C. Two threads each own one flag. Each calls
/// `try_enter(&its_own_flag, &the_other_flag)` at most once.
///
/// Raise `mine`, then look at `theirs`, and return `true` if `theirs` is not
/// raised. Requirement: when both threads call at the same time, **at most
/// one** of them may get `true`. (Both may get `false`.)
///
/// Release/Acquire cannot give this guarantee. Use `Relaxed` for the store
/// and for the load, with a `fence(Ordering::SeqCst)` between them. In
/// production code you would more often write `SeqCst` on both operations,
/// which gives the same guarantee, but loom 0.7.3 cannot check that form.
pub fn try_enter(mine: &AtomicBool, theirs: &AtomicBool) -> bool {
    mine.store(true, Ordering::Relaxed);
    // The two fences (one per thread) are totally ordered. The thread whose
    // fence comes second must see the other thread's store.
    fence(Ordering::SeqCst);
    !theirs.load(Ordering::Relaxed)
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use crate::support::{DropCount, alloc::count, guess, within};
    use std::sync::Arc;

    #[test]
    fn a_permits_stop_at_the_limit() {
        let permits = Permits::new(2);
        assert!(permits.try_acquire());
        assert!(permits.try_acquire());
        assert!(
            !permits.try_acquire(),
            "both permits are taken, so a third request must fail"
        );
        assert_eq!(permits.in_use(), 2);
        permits.release();
        assert_eq!(permits.in_use(), 1);
        assert!(
            permits.try_acquire(),
            "a released permit can be taken again"
        );
    }

    #[test]
    fn a_zero_permits_never_succeed() {
        let permits = Permits::new(0);
        assert!(!permits.try_acquire());
        assert_eq!(
            permits.in_use(),
            0,
            "a failed request must leave the counter unchanged"
        );
    }

    #[test]
    fn a_threads_never_hold_more_than_the_limit() {
        let rounds = if cfg!(miri) { 20 } else { 2_000 };
        let granted = within(20, "the permit stress test", move || {
            let permits = Arc::new(Permits::new(3));
            let holders = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let workers: Vec<_> = (0..6)
                .map(|_| {
                    let (permits, holders) = (Arc::clone(&permits), Arc::clone(&holders));
                    std::thread::spawn(move || {
                        let mut granted = 0_u32;
                        for _ in 0..rounds {
                            if permits.try_acquire() {
                                let now = holders.fetch_add(1, Ordering::SeqCst) + 1;
                                assert!(
                                    now <= 3,
                                    "{now} threads hold a permit, but the limit is 3"
                                );
                                holders.fetch_sub(1, Ordering::SeqCst);
                                permits.release();
                                granted += 1;
                            }
                        }
                        granted
                    })
                })
                .collect();
            let granted: u32 = workers.into_iter().map(|w| w.join().unwrap()).sum();
            assert_eq!(
                permits.in_use(),
                0,
                "every permit was released, so none is in use at the end"
            );
            granted
        });
        assert!(granted > 0, "no thread ever got a permit");
    }

    #[test]
    fn b_set_once_keeps_the_first_value() {
        let cell = SetOnce::new();
        assert_eq!(cell.get(), None, "a new cell is empty");
        assert_eq!(cell.set(String::from("first")), Ok(()));
        assert_eq!(cell.get().map(String::as_str), Some("first"));
        assert_eq!(
            cell.set(String::from("second")),
            Err(String::from("second")),
            "a second `set` must hand its value back"
        );
        assert_eq!(cell.get().map(String::as_str), Some("first"));
    }

    #[test]
    fn b_get_returns_the_stored_value_itself() {
        let cell = SetOnce::new();
        cell.set(vec![1_u8, 2, 3]).unwrap();
        let first = cell.get().unwrap().as_ptr();
        let second = cell.get().unwrap().as_ptr();
        assert_eq!(
            first, second,
            "`get` must return a reference to the stored value, not to a copy"
        );
        let (_, allocations) = count(|| cell.get().map(Vec::len));
        assert_eq!(
            allocations, 0,
            "`get` made {allocations} heap allocation(s); reading needs none"
        );
    }

    #[test]
    fn b_value_is_dropped_exactly_once() {
        let drops = DropCount::new();
        {
            let cell = SetOnce::new();
            assert!(cell.set(drops.value(1)).is_ok());
            assert!(cell.set(drops.value(2)).is_err());
            assert_eq!(
                drops.get(),
                1,
                "the rejected value was returned and dropped by the test; \
                 the stored value must still be alive"
            );
        }
        assert_eq!(
            drops.get(),
            2,
            "dropping the cell must drop the stored value exactly once"
        );
        drop(SetOnce::<String>::new());
    }

    #[test]
    fn b_one_of_many_setters_wins() {
        let rounds = if cfg!(miri) { 5 } else { 200 };
        within(20, "the SetOnce race", move || {
            for _ in 0..rounds {
                let cell = Arc::new(SetOnce::new());
                let workers: Vec<_> = (0..4_usize)
                    .map(|id| {
                        let cell = Arc::clone(&cell);
                        std::thread::spawn(move || {
                            let won = cell.set(vec![id; 3]).is_ok();
                            // Whoever lost may see the cell while the winner is
                            // still writing. It must then see `None`, never a
                            // half-written value.
                            if let Some(value) = cell.get() {
                                assert_eq!(value.len(), 3);
                            }
                            won
                        })
                    })
                    .collect();
                let winners = workers
                    .into_iter()
                    .map(|w| w.join().unwrap())
                    .filter(|won| *won)
                    .count();
                assert_eq!(winners, 1, "exactly one `set` call may succeed");
                assert_eq!(cell.get().map(Vec::len), Some(3));
            }
        });
    }

    #[test]
    fn c_a_single_caller_enters() {
        let (mine, theirs) = (AtomicBool::new(false), AtomicBool::new(false));
        assert!(
            try_enter(&mine, &theirs),
            "nobody else raised a flag, so the caller may enter"
        );
        assert!(
            mine.load(Ordering::SeqCst),
            "`try_enter` must raise the caller's own flag"
        );
        assert!(
            !try_enter(&theirs, &mine),
            "the first caller's flag is raised, so the second caller must stay out"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// These predictions use the standard library's atomics on one thread, so
    /// each has exactly one right answer.
    #[test]
    fn p_predictions() {
        use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering::SeqCst};

        // P1. What `fetch_add` returns, and what is stored afterwards.
        let small = AtomicU8::new(250);
        let returned = small.fetch_add(10, SeqCst);
        let p1: (u8, u8) = (250, 4);
        assert_eq!(
            p1,
            (returned, small.load(SeqCst)),
            "P1: (value returned by fetch_add, value stored afterwards)"
        );

        // P2. Two compare-exchange calls in a row on a value that starts at 5.
        let number = AtomicUsize::new(5);
        let first = number.compare_exchange(4, 9, SeqCst, SeqCst);
        let second = number.compare_exchange(5, 9, SeqCst, SeqCst);
        let p2: (Result<usize, usize>, Result<usize, usize>) = (Err(5), Ok(5));
        assert_eq!(
            p2,
            (first, second),
            "P2: results of compare_exchange(4, 9) and then compare_exchange(5, 9)"
        );

        // P3. `swap` and `fetch_and` on a flag that starts as `false`.
        let flag = AtomicBool::new(false);
        let swapped = flag.swap(true, SeqCst);
        let anded = flag.fetch_and(false, SeqCst);
        let p3: (bool, bool, bool) = (false, true, false);
        assert_eq!(
            p3,
            (swapped, anded, flag.load(SeqCst)),
            "P3: (result of swap(true), result of fetch_and(false), final value)"
        );
    }
}

#[cfg(all(test, loom))]
mod model {
    use super::*;
    use crate::support::{DropCount, Tracked, guess, model, outcomes};
    use crate::sync::{Arc, thread};

    #[test]
    fn a_two_threads_one_permit() {
        model(|| {
            let permits = Arc::new(Permits::new(1));
            let other = {
                let permits = Arc::clone(&permits);
                thread::spawn(move || permits.try_acquire())
            };
            let mine = permits.try_acquire();
            assert!(
                permits.in_use() <= 1,
                "`used` went above the limit of 1, even if only for a moment"
            );
            let theirs = other.join().unwrap();
            assert!(
                !(mine && theirs),
                "both threads got the only permit: the check and the increment \
                 must be one atomic step"
            );
            assert!(
                mine || theirs,
                "one permit was free, so one of the two requests must succeed"
            );
            assert_eq!(permits.in_use(), 1);
        });
    }

    #[test]
    fn b_reader_sees_a_complete_value() {
        model(|| {
            let cell = Arc::new(SetOnce::new());
            let writer = {
                let cell = Arc::clone(&cell);
                thread::spawn(move || {
                    assert!(cell.set(Tracked::new(7_u32)).is_ok());
                })
            };
            // If `get` returns the value, reading it must be ordered after
            // the write in the other thread. Loom reports a causality
            // violation here when the orderings in `set` or `get` are too weak.
            if let Some(value) = cell.get() {
                assert_eq!(value.get(), 7);
            }
            writer.join().unwrap();
            assert_eq!(cell.get().map(Tracked::get), Some(7));
        });
    }

    #[test]
    fn b_two_setters_one_winner() {
        model(|| {
            let drops = DropCount::new();
            let cell = Arc::new(SetOnce::new());
            let other = {
                let cell = Arc::clone(&cell);
                thread::spawn(move || cell.set(Tracked::counted(1_u32, &drops)).is_ok())
            };
            let mine = cell.set(Tracked::counted(2_u32, &drops)).is_ok();
            let theirs = other.join().unwrap();
            assert!(
                mine != theirs,
                "exactly one of two `set` calls must succeed"
            );
            let stored = cell.get().expect("a `set` call finished").get();
            assert_eq!(stored, if mine { 2 } else { 1 });
            drop(cell);
            assert_eq!(
                drops.get(),
                2,
                "the stored value and the rejected value must each be dropped once"
            );
        });
    }

    #[test]
    fn c_at_most_one_thread_enters() {
        model(|| {
            let flags = Arc::new((AtomicBool::new(false), AtomicBool::new(false)));
            let other = {
                let flags = Arc::clone(&flags);
                thread::spawn(move || try_enter(&flags.1, &flags.0))
            };
            let mine = try_enter(&flags.0, &flags.1);
            let theirs = other.join().unwrap();
            assert!(
                !(mine && theirs),
                "both threads entered: each read the other's flag before the \
                 other's store became visible (store buffering)"
            );
        });
    }

    /// Message passing. The writer stores 7 into `data` and then sets `flag`,
    /// using `flag_store` for the second store. The reader loads `flag` with
    /// `flag_load` and, only if it is set, loads `data`. The result is `None`
    /// when the flag was not set, and `Some(what the reader saw in data)`
    /// otherwise.
    fn message_passing(flag_store: Ordering, flag_load: Ordering) -> Vec<Option<u32>> {
        use crate::sync::atomic::AtomicU32;
        outcomes(move || {
            let shared = Arc::new((AtomicU32::new(0), AtomicBool::new(false)));
            let writer = {
                let shared = Arc::clone(&shared);
                thread::spawn(move || {
                    shared.0.store(7, Ordering::Relaxed);
                    shared.1.store(true, flag_store);
                })
            };
            let seen = if shared.1.load(flag_load) {
                Some(shared.0.load(Ordering::Relaxed))
            } else {
                None
            };
            writer.join().unwrap();
            seen
        })
    }

    /// Store buffering. Each thread stores 1 into its own variable and then
    /// loads the other thread's variable. With `fenced`, each thread runs
    /// `fence(SeqCst)` between its store and its load. The result is
    /// `(what the spawned thread loaded, what the main thread loaded)`.
    fn store_buffering(order: (Ordering, Ordering), fenced: bool) -> Vec<(u32, u32)> {
        use crate::sync::atomic::AtomicU32;
        let (store, load) = order;
        outcomes(move || {
            let shared = Arc::new((AtomicU32::new(0), AtomicU32::new(0)));
            let other = {
                let shared = Arc::clone(&shared);
                thread::spawn(move || {
                    shared.0.store(1, store);
                    if fenced {
                        fence(Ordering::SeqCst);
                    }
                    shared.1.load(load)
                })
            };
            shared.1.store(1, store);
            if fenced {
                fence(Ordering::SeqCst);
            }
            let mine = shared.0.load(load);
            (other.join().unwrap(), mine)
        })
    }

    /// P. Predict BEFORE running `cargo model l01_`: replace each `guess()`
    /// with your answer.
    ///
    /// Read `message_passing` and `store_buffering` above. Each prediction is
    /// the complete list of results that the memory model allows, in
    /// ascending order (`None` sorts before `Some(_)`; tuples sort by their
    /// first field, then their second). Loom tries every allowed run and
    /// collects the results.
    #[test]
    fn p_model_predictions() {
        // M1. Message passing with `Relaxed` on the flag.
        let m1: Vec<Option<u32>> = vec![None, Some(0), Some(7)];
        assert_eq!(
            m1,
            message_passing(Ordering::Relaxed, Ordering::Relaxed),
            "M1: every result of message passing with a Relaxed flag \
             (left: your prediction, right: found by loom)"
        );

        // M2. Message passing with `Release` on the store and `Acquire` on the load.
        let m2: Vec<Option<u32>> = vec![None, Some(7)];
        assert_eq!(
            m2,
            message_passing(Ordering::Release, Ordering::Acquire),
            "M2: every result of message passing with Release/Acquire \
             (left: your prediction, right: found by loom)"
        );

        // M3. Store buffering with `Release` stores and `Acquire` loads, no fence.
        let m3: Vec<(u32, u32)> = vec![(0, 0), (0, 1), (1, 0), (1, 1)];
        assert_eq!(
            m3,
            store_buffering((Ordering::Release, Ordering::Acquire), false),
            "M3: every result of store buffering with Release/Acquire \
             (left: your prediction, right: found by loom)"
        );

        // M4. Store buffering with `Relaxed` stores and loads and a SeqCst fence.
        let m4: Vec<(u32, u32)> = vec![(0, 1), (1, 0), (1, 1)];
        assert_eq!(
            m4,
            store_buffering((Ordering::Relaxed, Ordering::Relaxed), true),
            "M4: every result of store buffering with a SeqCst fence \
             (left: your prediction, right: found by loom)"
        );
    }
}
