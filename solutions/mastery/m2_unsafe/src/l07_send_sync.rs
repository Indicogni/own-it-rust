//! LESSON 07 — Send and Sync: writing the proof
//! Run: cargo test --manifest-path mastery/m2_unsafe/Cargo.toml l07_
//! Miri: cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml l07_
//! Read first: "Lesson 07" in CONCEPTS.md (one directory up).
//! Needs: your types from lessons 01 to 04.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `Mutex<T>` is `Sync` when `T` is `Send`. Why is `T: Sync` not needed,
//!    and why is `T: Send` needed although the mutex itself never moves?
//! 2. A lock guard contains `&Lock<T>`. For which `T` does the compiler then
//!    make the guard `Sync` by itself, and for which `T` is that wrong?
//!
//! IMPLEMENT the contract below. You design the types; the tests are switched
//! off until they exist (see lesson 05 for the order of work). Every
//! `unsafe impl` needs a SAFETY comment that is a short proof: say what a
//! second thread can do with the type, and why the bound makes that harmless.
//! EXPLAIN afterward: For each `unsafe impl` you wrote, give the proof in two
//! sentences. For each bound, name a concrete `T` that would cause a data race
//! or a wrong-thread destructor if the bound were missing.
//!
//! MIRI LAB: `examples/miri_l07_sync.rs` promises the compiler that a counter
//! is `Sync`. Predict what Miri reports, run
//! `cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l07_sync`,
//! compare with CONCEPTS.md, then repair the example until Miri is silent.
//!
//! CONTRACT
//!
//! Part A. Two public types in this file: `Spin<T>`, a lock that waits by
//! spinning, and its guard `SpinGuard<'a, T>`.
//! - `Spin::new(value: T) -> Spin<T>` is a `const fn`.
//! - `lock(&self) -> SpinGuard<'_, T>` waits until the lock is free and takes
//!   it. Call `std::hint::spin_loop()` in the waiting loop.
//! - `try_lock(&self) -> Option<SpinGuard<'_, T>>` never waits.
//! - `get_mut(&mut self) -> &mut T` and `into_inner(self) -> T` do not lock.
//! - `SpinGuard` implements `Deref<Target = T>` and `DerefMut`. Dropping it
//!   releases the lock, also when it is dropped by a panic (no poisoning).
//! - Keep the state in an `AtomicBool`. Take the lock with
//!   `compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)` and
//!   release it with `store(false, Ordering::Release)`. Module M3 explains
//!   these orderings; here you may take them as given.
//! - The compiler must accept exactly these, and no more:
//!
//!   | Type               | `Send` when | `Sync` when |
//!   | ------------------ | ----------- | ----------- |
//!   | `Spin<T>`          | `T: Send`   | `T: Send`   |
//!   | `SpinGuard<'_, T>` | `T: Send`   | `T: Sync`   |
//!
//!   Some cells of this table are what the compiler derives from your fields.
//!   Find the ones that are not, and write an `unsafe impl` for each.
//!
//! Part B. Go back to your own types and make them usable across threads,
//! with `unsafe impl`s next to each type and a proof in each SAFETY comment:
//!
//!   | Type           | `Send` when | `Sync` when |
//!   | -------------- | ----------- | ----------- |
//!   | `MyBox<T>`     | `T: Send`   | `T: Sync`   |
//!   | `MyVec<T>`     | `T: Send`   | `T: Sync`   |
//!   | `IntoIter<T>`  | `T: Send`   | `T: Sync`   |
//!   | `Drain<'_, T>` | `T: Send`   | `T: Sync`   |
//!
//!   `MyCell<T>` and `MyRefCell<T>` must stay as they are: `Send` when
//!   `T: Send`, and never `Sync`. Do not add impls for them; say why in NOTES.md.

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

/// A lock that waits by spinning.
///
/// Invariant: `locked` is `true` from the moment a `lock` or `try_lock` call
/// succeeds until the guard it returned is dropped, and at most one guard
/// exists at a time. Only that guard touches `value`.
///
/// `Send` is derived by the compiler: `AtomicBool` is `Send`, and
/// `UnsafeCell<T>` is `Send` exactly when `T` is, which is the right answer,
/// because moving the lock moves the `T`.
pub struct Spin<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

// SAFETY: a `&Spin<T>` on several threads lets each of them obtain a guard,
// and a guard gives `&mut T`. By the invariant only one guard exists at a
// time, and the Acquire/Release pair on `locked` makes everything the previous
// holder wrote visible to the next one. So the threads never access `value`
// at the same time; they take turns with exclusive access. Exclusive access
// that passes from thread to thread is what `T: Send` permits: a thread can
// even move the `T` out with `mem::replace` and drop it there. `T: Sync` is
// not needed, because no two threads ever hold a `&T` at the same moment.
unsafe impl<T: Send> Sync for Spin<T> {}

/// Exclusive access to the value in a [`Spin`]. Releases the lock when dropped.
///
/// `Send` is derived by the compiler from the field: `&Spin<T>` is `Send` when
/// `Spin<T>` is `Sync`, that is when `T: Send`. That is the right answer: the
/// thread that receives the guard gets the exclusive access, and a spin lock
/// may be released by any thread.
pub struct SpinGuard<'a, T> {
    lock: &'a Spin<T>,
}

// SAFETY: a `&SpinGuard<'_, T>` gives only `&T` (`deref_mut` and `drop` need
// more than a shared reference to the guard). Sharing a guard between threads
// therefore shares a `&T`, which is sound exactly when `T: Sync`. Without this
// impl the compiler would derive `Sync` from the field under the weaker
// condition `T: Send`, which is wrong: two threads could then use
// `&Cell<i32>` at the same time. Writing the impl replaces the derived one.
unsafe impl<T: Sync> Sync for SpinGuard<'_, T> {}

impl<T> Spin<T> {
    pub const fn new(value: T) -> Spin<T> {
        Spin {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn try_lock(&self) -> Option<SpinGuard<'_, T>> {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| SpinGuard { lock: self })
    }

    pub fn lock(&self) -> SpinGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_lock() {
                return guard;
            }
            std::hint::spin_loop();
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        // `&mut self` proves that no guard exists and no other thread can lock.
        self.value.get_mut()
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T> Deref for SpinGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: the pointer is valid while the lock is borrowed. This guard
        // exists, so by the invariant no other guard does, and nothing else
        // touches `value`. The only other access is `deref_mut` on this same
        // guard, which cannot overlap with the `&self` borrow of the result.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for SpinGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as in `deref`; the result borrows the only guard mutably, so
        // it is the only reference to `value` on any thread.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for SpinGuard<'_, T> {
    fn drop(&mut self) {
        // No reference into `value` made from this guard can still be alive:
        // each one borrowed the guard, and the guard is being dropped.
        self.lock.locked.store(false, Ordering::Release);
    }
}

crate::gate! { on, "07" =>
#[cfg(test)]
mod tests {
    use super::*;
    use crate::l01_unsafe_cell::{MyCell, MyRefCell};
    use crate::l02_box::MyBox;
    use crate::l03_vec::MyVec;
    use crate::l04_iterators::{Drain, IntoIter};
    use crate::{impls, within};
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
    use std::rc::Rc;
    use std::sync::MutexGuard;

    /// Compiles only if `Spin::new` is a `const fn` and `Spin<u32>` is `Sync`.
    static SHARED: Spin<u32> = Spin::new(0);

    #[test]
    fn a_lock_gives_exclusive_access() {
        let mut lock = Spin::new(String::from("a"));
        {
            let mut guard = lock.lock();
            guard.push('b');
            assert_eq!(*guard, "ab");
            assert_eq!(guard.len(), 2, "method calls must reach the value");
            assert!(
                lock.try_lock().is_none(),
                "try_lock must fail while a guard exists"
            );
        }
        lock.try_lock()
            .expect("the guard was dropped, so the lock must be free")
            .push('c');
        lock.get_mut().push('d');
        assert_eq!(lock.into_inner(), "abcd");
        *SHARED.lock() += 1;
        assert!(*SHARED.lock() >= 1);
    }

    #[test]
    fn a_guard_points_into_the_lock() {
        let mut lock = Spin::new(5u64);
        let inside: *const u64 = lock.get_mut();
        assert!(
            std::ptr::eq(&*lock.lock(), inside),
            "the guard must point at the value inside the lock, not at a copy"
        );
    }

    #[test]
    fn a_threads_take_turns() {
        let threads: u64 = 4;
        let rounds: u64 = if cfg!(miri) { 40 } else { 20_000 };
        let total = within(120, "four threads sharing one Spin", move || {
            let lock = Spin::new(0u64);
            std::thread::scope(|scope| {
                for _ in 0..threads {
                    scope.spawn(|| {
                        for _ in 0..rounds {
                            let mut guard = lock.lock();
                            // Read, wait, write: a second thread inside the
                            // lock at the same time would lose an update.
                            let seen = *guard;
                            std::hint::spin_loop();
                            *guard = seen + 1;
                            // Passing the guard to a function by value is
                            // the hard case for a guard type: see HINTS.md.
                            drop(guard);
                        }
                    });
                }
            });
            lock.into_inner()
        });
        assert_eq!(
            total,
            threads * rounds,
            "updates were lost: two threads were inside the lock at the same time"
        );
    }

    #[test]
    fn a_a_panic_releases_the_lock() {
        let released = within(10, "locking again after a panic", || {
            let lock = Spin::new(0);
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                let mut guard = lock.lock();
                *guard = 1;
                // Unwinds like a panic, without printing a message.
                resume_unwind(Box::new("stop"));
            }));
            assert!(outcome.is_err());
            *lock.lock()
        });
        assert_eq!(
            released, 1,
            "a guard dropped by unwinding must release the lock and keep the value"
        );
    }

    #[test]
    fn a_spin_and_its_guard_cross_threads_exactly_when_it_is_sound() {
        // Spin<T>: Send when T: Send, Sync when T: Send.
        assert!(impls!(Spin<u8>: Send + Sync));
        assert!(
            impls!(Spin<Cell<u8>>: Send + Sync),
            "Spin<Cell<u8>> must be Sync: the lock lets only one thread at a time use \
             the Cell, so T: Send is enough"
        );
        assert!(
            !impls!(Spin<Rc<u8>>: Send) && !impls!(Spin<Rc<u8>>: Sync),
            "Spin<Rc<u8>> must be neither Send nor Sync: another thread could clone or \
             drop the Rc"
        );
        assert!(
            !impls!(Spin<MutexGuard<'static, u8>>: Sync),
            "Spin<T> must not be Sync for a T that is Sync but not Send: another thread \
             could take the T out of the lock and drop it there"
        );

        // SpinGuard<'_, T>: Send when T: Send, Sync when T: Sync.
        assert!(impls!(SpinGuard<'static, u8>: Send + Sync));
        assert!(
            impls!(SpinGuard<'static, Cell<u8>>: Send),
            "a guard for a Send value may be handed to another thread"
        );
        assert!(
            !impls!(SpinGuard<'static, Cell<u8>>: Sync),
            "SpinGuard<Cell<u8>> must not be Sync: two threads with a shared reference \
             to the guard would both get &Cell<u8>"
        );
        assert!(
            !impls!(SpinGuard<'static, Rc<u8>>: Send) && !impls!(SpinGuard<'static, Rc<u8>>: Sync)
        );
        assert!(
            impls!(SpinGuard<'static, MutexGuard<'static, u8>>: Sync),
            "a shared guard only gives &T, so T: Sync is enough for SpinGuard: Sync"
        );
        assert!(!impls!(SpinGuard<'static, MutexGuard<'static, u8>>: Send));
    }

    #[test]
    fn b_owning_containers_cross_threads_like_their_elements() {
        // T = u8 is Send and Sync; Rc<u8> is neither; Cell<u8> is Send only;
        // MutexGuard<u8> is Sync only.
        assert!(
            impls!(MyBox<u8>: Send + Sync),
            "MyBox<u8> must be Send and Sync"
        );
        assert!(!impls!(MyBox<Rc<u8>>: Send) && !impls!(MyBox<Rc<u8>>: Sync));
        assert!(impls!(MyBox<Cell<u8>>: Send) && !impls!(MyBox<Cell<u8>>: Sync));
        assert!(
            !impls!(MyBox<MutexGuard<'static, u8>>: Send)
                && impls!(MyBox<MutexGuard<'static, u8>>: Sync)
        );

        assert!(
            impls!(MyVec<u8>: Send + Sync),
            "MyVec<u8> must be Send and Sync"
        );
        assert!(!impls!(MyVec<Rc<u8>>: Send) && !impls!(MyVec<Rc<u8>>: Sync));
        assert!(impls!(MyVec<Cell<u8>>: Send) && !impls!(MyVec<Cell<u8>>: Sync));
        assert!(
            !impls!(MyVec<MutexGuard<'static, u8>>: Send)
                && impls!(MyVec<MutexGuard<'static, u8>>: Sync)
        );

        assert!(
            impls!(IntoIter<u8>: Send + Sync),
            "IntoIter<u8> must be Send and Sync"
        );
        assert!(!impls!(IntoIter<Rc<u8>>: Send) && !impls!(IntoIter<Rc<u8>>: Sync));
        assert!(impls!(IntoIter<Cell<u8>>: Send) && !impls!(IntoIter<Cell<u8>>: Sync));

        assert!(
            impls!(Drain<'static, u8>: Send + Sync),
            "Drain<u8> must be Send and Sync"
        );
        assert!(!impls!(Drain<'static, Rc<u8>>: Send) && !impls!(Drain<'static, Rc<u8>>: Sync));
        assert!(impls!(Drain<'static, Cell<u8>>: Send) && !impls!(Drain<'static, Cell<u8>>: Sync));
    }

    #[test]
    fn b_cells_stay_on_one_thread_at_a_time() {
        assert!(impls!(MyCell<u8>: Send) && impls!(MyRefCell<u8>: Send));
        assert!(
            !impls!(MyCell<u8>: Sync) && !impls!(MyRefCell<u8>: Sync),
            "a cell changes its value through &self without any synchronization, so it \
             must never be Sync"
        );
        assert!(!impls!(MyCell<Rc<u8>>: Send) && !impls!(MyRefCell<Rc<u8>>: Send));
    }

    #[test]
    fn b_a_vector_really_moves_to_another_thread() {
        let mut vec = MyVec::new();
        for word in ["a", "b", "c"] {
            vec.push(MyBox::new(word.to_string()));
        }
        let joined = within(10, "a thread that owns a MyVec", move || {
            let shared = &vec;
            let lengths = std::thread::scope(|scope| {
                let reader = scope.spawn(|| shared.iter().map(|word| word.len()).sum::<usize>());
                reader.join().unwrap() + shared.len()
            });
            let text: String = vec.into_iter().map(MyBox::into_inner).collect();
            (lengths, text)
        });
        assert_eq!(joined, (6, "abc".to_string()));
    }
}
}

#[cfg(test)]
mod predictions {
    use crate::{guess, impls};
    use std::cell::Cell;
    use std::marker::PhantomData;
    use std::ptr::NonNull;
    use std::rc::Rc;
    use std::sync::atomic::AtomicPtr;
    use std::sync::{Arc, Mutex, MutexGuard, RwLock};

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// `impls!(Type: Trait)` is `true` if the type implements the trait.
    #[test]
    fn p_predictions() {
        // P1. Shared ownership.
        let p1: (bool, bool, bool) = (false, true, false);
        assert_eq!(
            p1,
            (
                impls!(Rc<i32>: Send),
                impls!(Arc<i32>: Send),
                impls!(Arc<Cell<i32>>: Send)
            ),
            "P1: Send for Rc<i32>, Arc<i32>, Arc<Cell<i32>> \
             (left: your prediction, right: actual)"
        );

        // P2. References.
        let p2: (bool, bool, bool) = (false, true, false);
        assert_eq!(
            p2,
            (
                impls!(&Cell<i32>: Send),
                impls!(&mut Cell<i32>: Send),
                impls!(Cell<i32>: Sync)
            ),
            "P2: &Cell<i32>: Send, &mut Cell<i32>: Send, Cell<i32>: Sync \
             (left: your prediction, right: actual)"
        );

        // P3. Locks and their guards.
        let p3: (bool, bool, bool, bool) = (true, false, false, false);
        assert_eq!(
            p3,
            (
                impls!(Mutex<Cell<i32>>: Sync),
                impls!(RwLock<Cell<i32>>: Sync),
                impls!(MutexGuard<'static, Cell<i32>>: Sync),
                impls!(MutexGuard<'static, i32>: Send)
            ),
            "P3: Mutex<Cell<i32>>: Sync, RwLock<Cell<i32>>: Sync, \
             MutexGuard<Cell<i32>>: Sync, MutexGuard<i32>: Send \
             (left: your prediction, right: actual)"
        );

        // P4. Raw pointers and their relatives.
        let p4: (bool, bool, bool, bool) = (false, false, false, true);
        assert_eq!(
            p4,
            (
                impls!(*const i32: Send),
                impls!(NonNull<i32>: Sync),
                impls!(PhantomData<*const i32>: Send),
                impls!(AtomicPtr<i32>: Sync)
            ),
            "P4: *const i32: Send, NonNull<i32>: Sync, PhantomData<*const i32>: Send, \
             AtomicPtr<i32>: Sync (left: your prediction, right: actual)"
        );
    }
}
