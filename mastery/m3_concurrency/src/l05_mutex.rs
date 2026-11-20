//! LESSON 05 — A mutex that puts waiting threads to sleep
//! Run: cargo test l05_
//! Model: cargo model l05_
//! Read first: "Lesson 05" in CONCEPTS.md (same package).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A thread finds the mutex locked and decides to sleep. The owner unlocks
//!    between that decision and the call to `park`. Who wakes the sleeper?
//! 2. `unlock` wakes one waiting thread. Is that thread guaranteed to get the
//!    mutex next?
//!
//! IMPLEMENT the contract below. You design the fields. You may use your
//! `SpinLock` from lesson 02 (`crate::l02_spinlock::SpinLock`) to protect a
//! queue of waiting threads; everything else comes from `crate::sync`. Keep
//! the supplied tests and add one edge case.
//! EXPLAIN afterward: Describe your waiting protocol as two lists of steps,
//! one for a thread that must wait and one for a thread that unlocks. Then
//! show, for every way these steps can interleave, that a waiting thread is
//! woken. Say which step makes spurious wake-ups harmless.
//!
//! MODEL LAB (optional): In your `lock`, remove the second attempt to take the
//! mutex (the one made after the thread has registered itself as a waiter)
//! and run `cargo model l05_`. Loom reports a deadlock. Write the interleaving
//! in NOTES.md as a numbered list of steps.

use crate::l02_spinlock::SpinLock;
use crate::sync::UnsafeCell;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::sync::thread::{self, Thread};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

// The imports above are the ones the reference solution uses. Remove what
// your design does not need.

// CONTRACT
//
// `Mutex<T>` owns a `T` and gives access to it to one thread at a time.
//
// 1. `lock` returns a guard as soon as the mutex is free. If it is not free,
//    the calling thread **sleeps** with `thread::park()` until an unlock wakes
//    it. It must not spin while it waits for the mutex. (A short spin lock
//    around your own queue of waiters is fine: nobody holds that for long.)
// 2. No lost wake-up: if a thread is waiting and the mutex becomes free, some
//    waiting thread is woken and gets the chance to take it. This must hold
//    for every interleaving; the model tests check it.
// 3. `park` may return although nobody called `unpark`, and `unpark` may be
//    called for a thread that is not parked. Both must be harmless.
// 4. `try_lock` never waits. It returns `Some` when the mutex is free.
// 5. The guard dereferences to `T`, mutably too. Dropping the guard unlocks,
//    also when the thread is unwinding from a panic. There is no poisoning.
// 6. `into_inner` returns the value without locking.
// 7. Locking and unlocking a free mutex makes no heap allocation.
// 8. `Mutex<T>` is `Sync` exactly when `T: Send`. A guard must not be `Sync`
//    unless `T: Sync`, because `&MutexGuard<T>` gives `&T`. You must make
//    both true. (A guard made of `&Mutex<T>` and the marker of lesson 02 is
//    `Sync` for `T: Send + Sync`. That is stricter than necessary, and
//    harmless.)
// 9. Every atomic operation uses the weakest ordering that your argument needs.

/// A mutual-exclusion lock that parks waiting threads.
///
/// Replace the placeholder field with your own, and write the `unsafe impl
/// Sync` that the contract asks for, with a SAFETY comment.
pub struct Mutex<T> {
    _replace_me: PhantomData<T>,
}

/// Proof that the mutex is locked by this guard. Unlocks when dropped.
///
/// Replace the placeholder field with your own, and write the `Drop` impl.
pub struct MutexGuard<'a, T> {
    _replace_me: PhantomData<&'a mut T>,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        todo!("Implement this task")
    }

    /// Takes the mutex, sleeping until it is free.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        todo!("Implement this task")
    }

    /// Takes the mutex if it is free right now.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        todo!("Implement this task")
    }

    /// Returns the value. `self` by value proves that no guard exists.
    pub fn into_inner(self) -> T {
        todo!("Implement this task")
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        todo!("Implement this task")
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        todo!("Implement this task")
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use crate::implements;
    use crate::support::{alloc::count, guess, within};
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn a_lock_gives_access_to_the_value() {
        within(10, "locking a free mutex three times in a row", || {
            let mutex = Mutex::new(vec![1, 2]);
            mutex.lock().push(3);
            assert_eq!(*mutex.lock(), [1, 2, 3]);
            {
                let mut guard = mutex.lock();
                guard.clear();
            }
            assert_eq!(mutex.into_inner(), []);
        });
    }

    #[test]
    fn a_try_lock_fails_only_while_a_guard_exists() {
        let mutex = Mutex::new(0_u32);
        let guard = mutex.try_lock().expect("the mutex is free");
        assert!(
            mutex.try_lock().is_none(),
            "a second guard must not exist while the first one is alive"
        );
        drop(guard);
        for attempt in 0..200 {
            assert!(
                mutex.try_lock().is_some(),
                "attempt {attempt}: `try_lock` failed on a free mutex \
                 (a weak compare-exchange may fail for no reason)"
            );
        }
    }

    #[test]
    fn a_a_panic_while_locked_unlocks() {
        let mutex = Mutex::new(1_u32);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut guard = mutex.lock();
            *guard = 2;
            panic!("deliberate panic while the mutex is held");
        }));
        assert!(result.is_err());
        let guard = mutex
            .try_lock()
            .expect("unwinding dropped the guard, which must unlock the mutex");
        assert_eq!(*guard, 2);
    }

    #[test]
    fn b_threads_do_not_lose_updates() {
        let rounds: u64 = if cfg!(miri) { 25 } else { 5_000 };
        let total = within(60, "the mutex stress test", move || {
            let mutex = Arc::new(Mutex::new(0_u64));
            let workers: Vec<_> = (0..6)
                .map(|_| {
                    let mutex = Arc::clone(&mutex);
                    std::thread::spawn(move || {
                        for _ in 0..rounds {
                            let mut guard = mutex.lock();
                            let seen = *guard;
                            *guard = seen + 1;
                        }
                    })
                })
                .collect();
            for worker in workers {
                worker.join().unwrap();
            }
            *mutex.lock()
        });
        assert_eq!(
            total,
            6 * rounds,
            "some increments were lost: two threads were inside the mutex together"
        );
    }

    /// Starts a thread that locks `mutex`, adds one, and unlocks. Returns
    /// after the thread has called `park` at least once, which it only does
    /// if it found the mutex locked.
    fn blocked_locker(mutex: &Arc<Mutex<u32>>) -> std::thread::JoinHandle<()> {
        let parks = Arc::new(AtomicUsize::new(0));
        let handle = {
            let (mutex, parks) = (Arc::clone(mutex), Arc::clone(&parks));
            std::thread::spawn(move || {
                crate::sync::thread::count_parks_in(&parks);
                *mutex.lock() += 1;
            })
        };
        // A thread that spins instead of sleeping never calls `park`. The
        // test then stays in this loop, and `within` reports it.
        while parks.load(Ordering::SeqCst) == 0 {
            std::thread::yield_now();
        }
        handle
    }

    #[test]
    fn b_a_waiting_thread_sleeps_and_is_woken() {
        within(
            20,
            "waiting for a blocked `lock` call to park (it must call `thread::park` from \
             `crate::sync`, and must not spin) and then to be woken by the unlock",
            || {
                let mutex = Arc::new(Mutex::new(0_u32));
                let guard = mutex.lock();
                let waiter = blocked_locker(&mutex);
                assert_eq!(
                    *guard, 0,
                    "the waiter must not get in while the guard is alive"
                );
                drop(guard);
                waiter.join().unwrap();
                assert_eq!(*mutex.lock(), 1);
            },
        );
    }

    #[test]
    fn b_a_waiter_sleeps_again_after_an_early_wake_up() {
        within(
            20,
            "waiting for a blocked `lock` call to park a second time after its first \
             `park` returned early (the mutex is still locked, so the thread must go back \
             to sleep with `thread::park` from `crate::sync`, and must not spin)",
            || {
                let mutex = Arc::new(Mutex::new(0_u32));
                let guard = mutex.lock();
                let parks = Arc::new(AtomicUsize::new(0));
                let waiter = {
                    let (mutex, parks) = (Arc::clone(&mutex), Arc::clone(&parks));
                    std::thread::spawn(move || {
                        crate::sync::thread::count_parks_in(&parks);
                        // Leave a wake-up token behind, as earlier code on
                        // this thread might. The first `park` in `lock` then
                        // returns at once, although nobody unlocked.
                        std::thread::current().unpark();
                        *mutex.lock() += 1;
                    })
                };
                while parks.load(Ordering::SeqCst) < 2 && !waiter.is_finished() {
                    std::thread::yield_now();
                }
                assert!(
                    !waiter.is_finished(),
                    "`lock` returned while another guard was alive. A `park` that returns \
                     does not mean that the mutex is free: try to take it again, and park \
                     again if that fails"
                );
                assert_eq!(
                    *guard, 0,
                    "the waiter must not get in while the guard is alive"
                );
                drop(guard);
                waiter.join().unwrap();
                assert_eq!(*mutex.lock(), 1);
            },
        );
    }

    #[test]
    fn b_every_waiting_thread_gets_its_turn() {
        within(
            30,
            "four threads waiting for one mutex (each must be woken in the end)",
            || {
                let mutex = Arc::new(Mutex::new(0_u32));
                let guard = mutex.lock();
                let waiters: Vec<_> = (0..4).map(|_| blocked_locker(&mutex)).collect();
                drop(guard);
                for waiter in waiters {
                    waiter.join().unwrap();
                }
                assert_eq!(*mutex.lock(), 4);
            },
        );
    }

    #[test]
    fn c_a_free_mutex_costs_no_allocation() {
        // `within`: a guard that does not unlock would make the second
        // `lock` wait for ever.
        within(10, "locking a free mutex twice in a row", || {
            let mutex = Mutex::new(41_u64);
            let (seen, allocations) = count(|| {
                *mutex.lock() += 1;
                let seen = mutex.try_lock().map(|guard| *guard);
                *mutex.lock() += 1;
                seen
            });
            assert_eq!(seen, Some(42));
            assert_eq!(
                allocations, 0,
                "locking and unlocking a free mutex made {allocations} heap allocation(s); \
                 the fast path must not touch the heap"
            );
            assert_eq!(mutex.into_inner(), 43);
        });
    }

    #[test]
    fn c_sync_bounds() {
        let _ = Mutex::new(0_u8);
        assert!(
            implements!(Mutex<Cell<u32>>: Sync),
            "`Cell<u32>` is `Send`, and the mutex lets one thread at a time reach it, \
             so `Mutex<Cell<u32>>` must be `Sync`"
        );
        assert!(
            !implements!(Mutex<Rc<u32>>: Sync),
            "a shared mutex hands the value to other threads, so `T` must be `Send`"
        );
        assert!(implements!(Mutex<u32>: Send + Sync));
        assert!(implements!(MutexGuard<'static, u32>: Sync));
        assert!(
            !implements!(MutexGuard<'static, Cell<u32>>: Sync),
            "`&MutexGuard<Cell<u32>>` gives `&Cell<u32>`; sharing the guard between \
             threads would share the cell"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// These predictions use `std::sync::Mutex`, which poisons itself when a
    /// thread panics while holding it. (Your mutex does not.)
    #[test]
    fn p_predictions() {
        use std::sync::Mutex;

        // P1. A temporary guard lives until the end of its statement.
        let mutex = Mutex::new(10);
        let sum = *mutex.lock().unwrap() + mutex.try_lock().map(|guard| *guard).unwrap_or(100);
        let p1: i32 = guess();
        assert_eq!(
            p1, sum,
            "P1: 10 + (the value if `try_lock` succeeds, else 100)"
        );

        // P2. A thread panics while it holds the mutex.
        let mutex = Arc::new(Mutex::new(1));
        let worker = {
            let mutex = Arc::clone(&mutex);
            std::thread::spawn(move || {
                let mut guard = mutex.lock().unwrap();
                *guard = 2;
                panic!("deliberate panic in a prediction (this message is expected)");
            })
        };
        let worker_failed = worker.join().is_err();
        let lock_is_err = mutex.lock().is_err();
        let value = *mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let p2: (bool, bool, i32) = guess();
        assert_eq!(
            p2,
            (worker_failed, lock_is_err, value),
            "P2: (join reports a panic, a later `lock` returns Err, the value inside)"
        );
    }
}

#[cfg(all(test, loom))]
mod model {
    use super::*;
    use crate::support::{Tracked, model, model_bounded};
    use crate::sync::{Arc, thread};

    // In these models only spawned threads ever wait for the mutex. The main
    // thread locks it only while it is free (before it spawns the others, or
    // after it has joined them). The reason is special to loom 0.7.3: its
    // `unpark` wakes a thread out of any blocking call, also out of `join`,
    // which real threads never do. A main thread that is never a waiter is
    // never unparked.

    fn add_one(mutex: &Mutex<Tracked<u32>>) {
        let mut guard = mutex.lock();
        let seen = guard.get();
        guard.set(seen + 1);
    }

    fn lockers(count: u32) -> impl Fn() {
        move || {
            let mutex = Arc::new(Mutex::new(Tracked::new(0_u32)));
            let workers: Vec<_> = (0..count)
                .map(|_| {
                    let mutex = Arc::clone(&mutex);
                    thread::spawn(move || add_one(&mutex))
                })
                .collect();
            // A deadlock report means that a thread went to sleep and no
            // unlock woke it.
            for worker in workers {
                worker.join().unwrap();
            }
            assert_eq!(
                mutex.lock().get(),
                count,
                "an increment was lost: two threads were inside the mutex together"
            );
        }
    }

    #[test]
    fn b_two_threads_take_turns() {
        model(lockers(2));
    }

    #[test]
    fn b_three_threads_take_turns() {
        model_bounded(1, lockers(3));
    }

    /// A thread that was woken does not own the mutex yet. Here a third
    /// thread takes the mutex with `try_lock` at the moment between the
    /// wake-up and the woken thread's next attempt. The woken thread must
    /// then sleep again, and the next unlock must still find it.
    ///
    /// The main thread takes the mutex while it is free, so it never sleeps
    /// in `lock` and nobody unparks it. It calls `yield_now` to let the other
    /// threads run first; after that, one forced thread switch is enough to
    /// reach the interesting run.
    #[test]
    fn b_a_woken_thread_that_loses_the_race_is_woken_again() {
        model_bounded(1, || {
            let mutex = Arc::new(Mutex::new(Tracked::new(0_u32)));
            let guard = mutex.lock();
            let sleeper = {
                let mutex = Arc::clone(&mutex);
                thread::spawn(move || add_one(&mutex))
            };
            let barger = {
                let mutex = Arc::clone(&mutex);
                thread::spawn(move || match mutex.try_lock() {
                    Some(mut guard) => {
                        let seen = guard.get();
                        guard.set(seen + 1);
                        1
                    }
                    None => 0,
                })
            };
            thread::yield_now();
            drop(guard);
            // A deadlock report means: the sleeper was woken, found the mutex
            // taken again, went back to sleep, and the unlock after that did
            // not wake it (for example because it was no longer in any queue).
            sleeper.join().unwrap();
            let barged: u32 = barger.join().unwrap();
            assert_eq!(
                mutex.lock().get(),
                1 + barged,
                "an increment was lost: two threads were inside the mutex together"
            );
        });
    }

    #[test]
    fn a_try_lock_admits_one_thread_at_a_time() {
        model(|| {
            let mutex = Arc::new(Mutex::new(Tracked::new(0_u32)));
            let workers: Vec<_> = (0..2)
                .map(|_| {
                    let mutex = Arc::clone(&mutex);
                    thread::spawn(move || match mutex.try_lock() {
                        Some(mut guard) => {
                            let seen = guard.get();
                            guard.set(seen + 1);
                            1
                        }
                        None => 0,
                    })
                })
                .collect();
            let entered: u32 = workers.into_iter().map(|w| w.join().unwrap()).sum();
            assert!(entered >= 1, "the mutex was free for the first caller");
            assert_eq!(mutex.lock().get(), entered);
        });
    }
}
