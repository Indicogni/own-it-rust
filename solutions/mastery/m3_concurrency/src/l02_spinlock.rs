//! LESSON 02 — A spin lock with a guard
//! Run: cargo test l02_
//! Model: cargo model l02_
//! Read first: "Lesson 02" in CONCEPTS.md (same package).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `unlock` is one store. Which memory ordering must it have, and which
//!    operation in `lock` is its partner?
//! 2. The guard below holds only `&SpinLock<T>`. For which `T` does the
//!    compiler make such a guard `Sync`, and is that right?
//!
//! IMPLEMENT the tasks below. Import from `crate::sync`, never from `std`.
//! Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: State the invariant that connects `locked` and `value`.
//! Use it to justify the two `unsafe` blocks in the guard, the orderings in
//! `lock` and in the guard's `Drop`, and the bounds on `Sync` for the lock
//! and for the guard.
//!
//! MODEL LAB (optional): Change the store in the guard's `Drop` to `Relaxed`
//! and run `cargo model l02_`. Then restore it and weaken the success
//! ordering of the compare-exchange in `lock` instead. Both reports name the
//! same two accesses. Why does either change break the same chain?

use crate::sync::UnsafeCell;
use crate::sync::atomic::{AtomicBool, Ordering};
use crate::sync::hint;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// A lock that waits by trying again in a loop.
///
/// Invariant: while `locked` is `false`, nobody accesses `value`. While it is
/// `true`, exactly one `SpinGuard` exists, and only that guard accesses `value`.
pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

// SAFETY: sharing `&SpinLock<T>` lets several threads take the lock one after
// another, and each of them gets `&mut T`. That moves access to the `T` from
// thread to thread, which is what `T: Send` allows. `T: Sync` is not needed,
// because the lock never lets two threads reach the value at the same time.
unsafe impl<T: Send> Sync for SpinLock<T> {}

/// Proof that the lock is held. Dereferences to the value; unlocks on drop.
///
/// A. The fields as given make the compiler derive the wrong `Sync` rule for
/// the guard. Find out which types `T` make `SpinGuard<'_, T>` `Sync` today,
/// decide what the rule should be, and change the guard so that the compiler
/// derives the right one. (You may add a field. You do not need `unsafe impl`.)
pub struct SpinGuard<'a, T> {
    lock: &'a SpinLock<T>,
    // The guard behaves like `&'a mut T`: it gives `&T` through a shared
    // guard and `&mut T` through an exclusive one. With this marker the
    // compiler asks for `T: Sync` before it makes the guard `Sync`, as it
    // does for `&mut T`. Without it the guard would be `Sync` whenever the
    // lock is, that is for every `T: Send`, and two threads could share a
    // `&Cell<_>`. (Because of the `lock` field the full rule is now
    // `T: Send + Sync`. Only `T: Sync` is necessary; the stricter rule
    // refuses nothing but types that are `Sync` and not `Send`.)
    _borrow: PhantomData<&'a mut T>,
}

impl<T> SpinLock<T> {
    pub fn new(value: T) -> Self {
        SpinLock {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    /// B. Waits until the lock is free, takes it, and returns the guard.
    ///
    /// Use a compare-exchange in a loop, and call `hint::spin_loop()` (from
    /// `crate::sync`) each time it fails. A weak compare-exchange is fine
    /// here, because the loop tries again anyway. (A loop around `swap` is
    /// also a correct spin lock, but it writes on every attempt, and loom
    /// then cannot tell a waiting thread from a working one: the model stops
    /// with "exceeded maximum number of branches".)
    pub fn lock(&self) -> SpinGuard<'_, T> {
        // Acquire on success pairs with the Release store in the guard's
        // `Drop`: everything the previous holder did to `value` happens
        // before everything this holder does. A failed attempt learns
        // nothing that matters, so Relaxed is enough for it.
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }
        SpinGuard {
            lock: self,
            _borrow: PhantomData,
        }
    }

    /// B. Takes the lock if it is free right now; never waits.
    ///
    /// Must return `Some` whenever the lock is free. A weak compare-exchange
    /// is allowed to fail for no reason, so it is the wrong tool here.
    pub fn try_lock(&self) -> Option<SpinGuard<'_, T>> {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| SpinGuard {
                lock: self,
                _borrow: PhantomData,
            })
    }

    /// B. Returns the value. Needs no locking: `self` by value proves that no
    /// guard and no other reference exists.
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T> Deref for SpinGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: this guard exists, so `locked` is true and no other guard
        // exists. The only other access to the value could come through this
        // same guard, and the borrow of `self` rules out a `deref_mut`
        // for as long as the returned reference lives.
        unsafe { &*self.lock.value.with(|p| p) }
    }
}

impl<T> DerefMut for SpinGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as in `deref`, and `&mut self` makes this the only use of
        // the guard for as long as the returned reference lives.
        unsafe { &mut *self.lock.value.with_mut(|p| p) }
    }
}

impl<T> Drop for SpinGuard<'_, T> {
    fn drop(&mut self) {
        // Release: the next thread whose compare-exchange reads this `false`
        // with Acquire sees every write this holder made to the value.
        self.lock.locked.store(false, Ordering::Release);
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

    #[test]
    fn b_lock_gives_access_to_the_value() {
        // `within`: a lock that is never released would spin here forever.
        within(
            10,
            "locking the same spin lock three times in a row",
            || {
                let lock = SpinLock::new(vec![1, 2]);
                lock.lock().push(3);
                assert_eq!(*lock.lock(), [1, 2, 3]);
                {
                    let mut guard = lock.lock();
                    guard.clear();
                    assert!(guard.is_empty());
                }
                assert_eq!(lock.into_inner(), []);
            },
        );
    }

    #[test]
    fn b_try_lock_fails_only_while_a_guard_exists() {
        let lock = SpinLock::new(0_u32);
        let guard = lock.try_lock().expect("the lock is free");
        assert!(
            lock.try_lock().is_none(),
            "a second guard must not exist while the first one is alive"
        );
        drop(guard);
        assert!(
            lock.try_lock().is_some(),
            "dropping the guard must release the lock"
        );
    }

    #[test]
    fn b_try_lock_never_fails_on_a_free_lock() {
        let lock = SpinLock::new(0_u32);
        for attempt in 0..200 {
            assert!(
                lock.try_lock().is_some(),
                "attempt {attempt}: `try_lock` failed although the lock was free. \
                 `compare_exchange_weak` may fail for no reason; `try_lock` has no \
                 loop to hide that, so it needs the strong version"
            );
        }
    }

    #[test]
    fn b_a_panic_while_locked_releases_the_lock() {
        let lock = SpinLock::new(1_u32);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut guard = lock.lock();
            *guard = 2;
            panic!("deliberate panic while the lock is held");
        }));
        assert!(result.is_err());
        let guard = lock
            .try_lock()
            .expect("unwinding dropped the guard, which must release the lock");
        assert_eq!(*guard, 2);
    }

    #[test]
    fn b_threads_do_not_lose_updates() {
        let rounds: u64 = if cfg!(miri) { 30 } else { 5_000 };
        let total = within(30, "the spin lock stress test", move || {
            let lock = Arc::new(SpinLock::new(0_u64));
            let workers: Vec<_> = (0..4)
                .map(|_| {
                    let lock = Arc::clone(&lock);
                    std::thread::spawn(move || {
                        for _ in 0..rounds {
                            // Read, then write: two separate steps that only
                            // a working lock keeps together.
                            let mut guard = lock.lock();
                            let seen = *guard;
                            *guard = seen + 1;
                        }
                    })
                })
                .collect();
            for worker in workers {
                worker.join().unwrap();
            }
            *lock.lock()
        });
        assert_eq!(
            total,
            4 * rounds,
            "some increments were lost: two threads were inside the lock together"
        );
    }

    #[test]
    fn a_guard_is_sync_only_for_sync_values() {
        assert!(
            implements!(SpinGuard<'static, u32>: Sync),
            "a guard for a `Sync` value may be shared between threads"
        );
        assert!(
            !implements!(SpinGuard<'static, Cell<u32>>: Sync),
            "`&SpinGuard<Cell<u32>>` gives `&Cell<u32>`. If the guard is `Sync`, two \
             threads can call `Cell::set` on the same cell at once, which is a data race"
        );
    }

    #[test]
    fn a_lock_is_sync_for_send_values() {
        let _ = SpinLock::new(0_u8);
        assert!(
            implements!(SpinLock<Cell<u32>>: Sync),
            "`Cell<u32>` is `Send`, and the lock lets only one thread at a time \
             reach it, so the lock may be shared"
        );
        assert!(
            !implements!(SpinLock<Rc<u32>>: Sync),
            "a shared lock hands the value to other threads, so `T` must be `Send`"
        );
        assert!(implements!(SpinLock<u32>: Send));
    }

    #[test]
    fn b_lock_is_one_flag_and_never_allocates() {
        assert_eq!(
            std::mem::size_of::<SpinLock<u8>>(),
            2,
            "a spin lock around a `u8` is one flag byte and one value byte"
        );
        let (value, allocations) = count(|| {
            let lock = SpinLock::new(41_u64);
            *lock.lock() += 1;
            let seen = lock.try_lock().map(|guard| *guard);
            (seen, lock.into_inner())
        });
        assert_eq!(value, (Some(42), 42));
        assert_eq!(
            allocations, 0,
            "creating, locking and unlocking made {allocations} heap allocation(s); \
             a spin lock needs none"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// These predictions use `std::sync::Mutex` on one thread. Its `try_lock`
    /// returns `Err` when the mutex is locked, also when the current thread
    /// is the one that holds it.
    #[test]
    fn p_predictions() {
        use std::sync::Mutex;
        use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

        // P1. Is the mutex free at the two marked points?
        let mutex = Mutex::new(0);
        let first = mutex.try_lock();
        let while_first_lives = mutex.try_lock().is_ok();
        drop(first);
        let after_drop = mutex.try_lock().is_ok();
        let p1: (bool, bool) = (false, true);
        assert_eq!(
            p1,
            (while_first_lives, after_drop),
            "P1: (free while `first` is alive, free after `drop(first)`)"
        );

        // P2. Three ways to write "lock it". Is the mutex free on the next line?
        let mutex = Mutex::new(0);
        *mutex.lock().unwrap() += 1;
        let after_statement = mutex.try_lock().is_ok();
        // The compiler rejects the next line by default, because it is almost
        // always a mistake. Here it is the point of the question.
        #[allow(let_underscore_lock)]
        let _ = mutex.lock().unwrap();
        let after_underscore = mutex.try_lock().is_ok();
        let _guard = mutex.lock().unwrap();
        let after_named = mutex.try_lock().is_ok();
        let p2: (bool, bool, bool) = (true, true, false);
        assert_eq!(
            p2,
            (after_statement, after_underscore, after_named),
            "P2: free after (a temporary guard, `let _ =`, `let _guard =`)"
        );

        // P3. The operation at the heart of every lock, twice in a row.
        let locked = AtomicBool::new(false);
        let first = locked.compare_exchange(false, true, SeqCst, SeqCst);
        let second = locked.compare_exchange(false, true, SeqCst, SeqCst);
        let p3: (Result<bool, bool>, Result<bool, bool>) = (Ok(false), Err(true));
        assert_eq!(
            p3,
            (first, second),
            "P3: results of two compare_exchange(false, true) calls in a row"
        );
    }
}

#[cfg(all(test, loom))]
mod model {
    use super::*;
    use crate::support::{Tracked, model, model_bounded};
    use crate::sync::{Arc, thread};

    /// Adds one to the value in two separate tracked steps.
    fn add_one(lock: &SpinLock<Tracked<u32>>) {
        let mut guard = lock.lock();
        let seen = guard.get();
        guard.set(seen + 1);
    }

    #[test]
    fn b_two_threads_take_turns() {
        model(|| {
            let lock = Arc::new(SpinLock::new(Tracked::new(0_u32)));
            let other = {
                let lock = Arc::clone(&lock);
                thread::spawn(move || add_one(&lock))
            };
            add_one(&lock);
            other.join().unwrap();
            assert_eq!(
                lock.lock().get(),
                2,
                "an increment was lost: both threads were inside the lock together"
            );
        });
    }

    /// Three threads. Loom follows two spinning threads that keep handing the
    /// processor to each other for as long as it is allowed to, so this model
    /// is limited to runs with at most one forced thread switch.
    #[test]
    fn b_three_threads_take_turns() {
        model_bounded(1, || {
            let lock = Arc::new(SpinLock::new(Tracked::new(0_u32)));
            let others: Vec<_> = (0..2)
                .map(|_| {
                    let lock = Arc::clone(&lock);
                    thread::spawn(move || add_one(&lock))
                })
                .collect();
            add_one(&lock);
            for other in others {
                other.join().unwrap();
            }
            assert_eq!(lock.lock().get(), 3);
        });
    }

    #[test]
    fn b_try_lock_admits_one_thread_at_a_time() {
        model(|| {
            let lock = Arc::new(SpinLock::new(Tracked::new(0_u32)));
            let attempt = |lock: &SpinLock<Tracked<u32>>| match lock.try_lock() {
                Some(mut guard) => {
                    let seen = guard.get();
                    guard.set(seen + 1);
                    1
                }
                None => 0,
            };
            let other = {
                let lock = Arc::clone(&lock);
                thread::spawn(move || attempt(&lock))
            };
            let mine = attempt(&lock);
            let theirs = other.join().unwrap();
            assert!(mine + theirs >= 1, "the lock was free for the first caller");
            assert_eq!(lock.lock().get(), mine + theirs);
        });
    }
}
