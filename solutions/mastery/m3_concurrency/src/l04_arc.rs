//! LESSON 04 — `MyArc` and `MyWeak`
//! Run: cargo test l04_
//! Model: cargo model l04_
//! Read first: "Lesson 04" in CONCEPTS.md (same package).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A thread that clones a `MyArc` already holds one. Does the increment
//!    need to be ordered with anything? What about the last decrement?
//! 2. `upgrade` must never turn a strong count of 0 back into 1. Which single
//!    atomic operation guarantees that?
//!
//! IMPLEMENT the contract below. From this lesson on, the starter gives you
//! only the public names. The fields, the heap layout, the `Send`/`Sync`
//! impls and the `Drop` impls are your design. Import from `crate::sync`,
//! never from `std`. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Write down the two counters and what each one counts.
//! State when the value is dropped and when the allocation is freed. Then
//! justify the ordering of every atomic operation, and the bounds on your
//! `Send` and `Sync` impls.
//!
//! MODEL LAB (optional): Weaken the decrement in `Drop for MyArc` to `Relaxed`
//! and run `cargo model l04_`. Then restore it and remove only the fence.
//! Run the solved lesson under Miri (`cargo +nightly-2026-11-20 miri test l04_`), then
//! weaken the decrement in `Drop for MyWeak` and run Miri again with
//! `MIRIFLAGS=-Zmiri-many-seeds=0..32`. Which tool sees which mistake, and why?

use crate::sync::UnsafeCell;
use crate::sync::atomic::{AtomicUsize, Ordering, fence};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::ptr::NonNull;

// CONTRACT
//
// `MyArc<T>` is a pointer to a heap block that holds a `T` and two counters.
// Cloning a `MyArc` gives a second pointer to the same block. `MyWeak<T>`
// points to the same block but does not keep the `T` alive.
//
// 1. One heap allocation per `MyArc::new`. `clone`, `downgrade` and `upgrade`
//    never allocate. A `MyArc<T>` is one pointer wide, and so is
//    `Option<MyArc<T>>` (use `NonNull`).
// 2. The `T` is dropped exactly once: when the last `MyArc` is dropped, even
//    if `MyWeak`s still exist.
// 3. The heap block is freed exactly once: when the last `MyArc` and the last
//    `MyWeak` are both gone. Not earlier (a `MyWeak` must be able to read the
//    counter) and not later (no leak).
// 4. `MyWeak::upgrade` returns `Some` only while at least one `MyArc` exists.
//    It must not bring a count of zero back to life.
// 5. `strong_count` is the number of `MyArc`s. `weak_count` is the number of
//    `MyWeak`s. (If your design keeps one extra weak reference on behalf of
//    all the `MyArc`s, as the standard library does, do not report it.)
// 6. If a count would pass `usize::MAX / 2`, abort the process
//    (`std::process::abort()`), as the standard library does: a wrapped
//    counter would free memory that is still in use.
// 7. `MyArc<T>` and `MyWeak<T>` are `Send` and `Sync` exactly when `T` is both
//    `Send` and `Sync`. You must write these impls, and say why both bounds
//    are needed for each.
// 8. Every atomic operation uses the weakest ordering that your safety
//    argument needs. The model tests reject orderings that are too weak.

/// The heap block.
///
/// Invariants:
/// - `strong` is the number of `MyArc`s.
/// - `weak` is the number of `MyWeak`s, plus one while `strong > 0`. That one
///   extra reference belongs to all the `MyArc`s together.
/// - `value` is initialised while `strong > 0`. It is dropped by the thread
///   that takes `strong` from 1 to 0, and by nobody else.
/// - The block is freed by the thread that takes `weak` from 1 to 0.
struct Inner<T> {
    strong: AtomicUsize,
    weak: AtomicUsize,
    /// In a cell, because the last `MyArc` drops the value through a shared
    /// pointer while `MyWeak`s on other threads still hold `&Inner<T>`.
    /// In a `ManuallyDrop`, because the value is dropped earlier than the
    /// block is freed. A `ManuallyDrop` may hold a value that was already
    /// dropped, as long as nobody uses that value again.
    value: UnsafeCell<ManuallyDrop<T>>,
}

/// A count this high can only come from leaking handles in a loop.
const MAX_COUNT: usize = usize::MAX / 2;

/// A shared owner of a `T` on the heap.
pub struct MyArc<T> {
    ptr: NonNull<Inner<T>>,
    /// Records that a `MyArc<T>` owns an `Inner<T>`, and through it a `T`.
    /// A raw pointer alone does not say that.
    _owns: PhantomData<Inner<T>>,
}

/// A pointer to the same block that does not keep the `T` alive.
pub struct MyWeak<T> {
    ptr: NonNull<Inner<T>>,
    _owns: PhantomData<Inner<T>>,
}

// SAFETY: sending a `MyArc<T>` to another thread lets that thread reach the
// `T` through `&T` while clones on the first thread do the same, so `T` must
// be `Sync`. The new thread may also be the one that drops the last handle
// and with it the `T`, which moves the `T` there, so `T` must be `Send`.
unsafe impl<T: Send + Sync> Send for MyArc<T> {}
// SAFETY: a thread with `&MyArc<T>` can clone it and then has a `MyArc<T>` of
// its own, so sharing must require everything that sending requires.
unsafe impl<T: Send + Sync> Sync for MyArc<T> {}
// SAFETY: a `MyWeak<T>` can be upgraded to a `MyArc<T>` on whatever thread
// holds it, so it needs the same bounds as `MyArc<T>`.
unsafe impl<T: Send + Sync> Send for MyWeak<T> {}
// SAFETY: `&MyWeak<T>` can be cloned or upgraded, as above.
unsafe impl<T: Send + Sync> Sync for MyWeak<T> {}

impl<T> MyArc<T> {
    pub fn new(value: T) -> Self {
        let inner = Box::new(Inner {
            strong: AtomicUsize::new(1),
            weak: AtomicUsize::new(1),
            value: UnsafeCell::new(ManuallyDrop::new(value)),
        });
        MyArc {
            ptr: NonNull::from(Box::leak(inner)),
            _owns: PhantomData,
        }
    }

    fn inner(&self) -> &Inner<T> {
        // SAFETY: this handle counts in `strong`, and the `MyArc`s together
        // hold one count in `weak`, so the block is allocated for as long as
        // `self` exists.
        unsafe { self.ptr.as_ref() }
    }

    /// A new `MyWeak` to the same block.
    pub fn downgrade(this: &Self) -> MyWeak<T> {
        // Relaxed: the caller already has access to the block through
        // `this`. Counting one more pointer orders nothing.
        let weak = this.inner().weak.fetch_add(1, Ordering::Relaxed);
        if weak > MAX_COUNT {
            std::process::abort();
        }
        MyWeak {
            ptr: this.ptr,
            _owns: PhantomData,
        }
    }

    /// How many `MyArc`s point to this block. Other threads may change the
    /// number at any time, so treat the answer as a hint.
    pub fn strong_count(this: &Self) -> usize {
        this.inner().strong.load(Ordering::Relaxed)
    }

    /// How many `MyWeak`s point to this block (a hint, as above).
    pub fn weak_count(this: &Self) -> usize {
        // `strong > 0` here, so the extra reference of the `MyArc`s is included.
        this.inner().weak.load(Ordering::Relaxed) - 1
    }

    /// Do `a` and `b` point to the same block?
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        a.ptr == b.ptr
    }
}

impl<T> Deref for MyArc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: `strong > 0` while `self` exists, so the value is
        // initialised and nobody drops it. Nothing ever hands out `&mut T`,
        // so shared references on any number of threads are fine (`T: Sync`
        // is required for the handle to reach another thread).
        unsafe { &*self.inner().value.with(|p| p.cast::<T>()) }
    }
}

impl<T> Clone for MyArc<T> {
    fn clone(&self) -> Self {
        // Relaxed: see `downgrade`. The new handle is created from an
        // existing one, so the count cannot be zero.
        let strong = self.inner().strong.fetch_add(1, Ordering::Relaxed);
        if strong > MAX_COUNT {
            std::process::abort();
        }
        MyArc {
            ptr: self.ptr,
            _owns: PhantomData,
        }
    }
}

impl<T> Drop for MyArc<T> {
    fn drop(&mut self) {
        // Release: whatever this thread did with the value through this
        // handle must happen before the drop of the value, which may run on
        // another thread.
        if self.inner().strong.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        // This thread took the count to zero. The Acquire fence pairs with
        // the Release decrements of all the other handles, so every use of
        // the value on every thread happens before the drop below.
        fence(Ordering::Acquire);
        // The weak reference that the `MyArc`s held together. It is given
        // back when this variable goes out of scope, and if no `MyWeak` is
        // left, that frees the block. Creating it first means that the block
        // is also released if the value's own `Drop` panics.
        let _shared_weak = MyWeak {
            ptr: self.ptr,
            _owns: PhantomData,
        };
        // SAFETY: `strong` is zero and can never rise again (`upgrade`
        // refuses zero), so no `&T` exists or can be created. The value is
        // still initialised, because only this thread saw the count go from
        // 1 to 0.
        self.inner()
            .value
            .with_mut(|p| unsafe { ManuallyDrop::drop(&mut *p) });
    }
}

impl<T> MyWeak<T> {
    fn inner(&self) -> &Inner<T> {
        // SAFETY: this handle counts in `weak`, so the block is allocated
        // for as long as `self` exists. The value inside may already be
        // dropped, or another thread may be dropping it right now. Both are
        // fine for a `&Inner<T>`: the value is in an `UnsafeCell`, its
        // `ManuallyDrop` wrapper may hold a dropped value, and no `MyWeak`
        // method ever looks at it.
        unsafe { self.ptr.as_ref() }
    }

    /// A new `MyArc`, if at least one still exists.
    pub fn upgrade(&self) -> Option<MyArc<T>> {
        let mut strong = self.inner().strong.load(Ordering::Relaxed);
        loop {
            if strong == 0 {
                return None;
            }
            if strong > MAX_COUNT {
                std::process::abort();
            }
            // The compare-exchange makes "is it non-zero?" and "add one" a
            // single step. Relaxed: a non-zero count means that some `MyArc`
            // keeps the value alive, and the thread that will drop the value
            // synchronizes through its own Release/Acquire pair.
            match self.inner().strong.compare_exchange_weak(
                strong,
                strong + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return Some(MyArc {
                        ptr: self.ptr,
                        _owns: PhantomData,
                    });
                }
                Err(now) => strong = now,
            }
        }
    }
}

impl<T> Clone for MyWeak<T> {
    fn clone(&self) -> Self {
        let weak = self.inner().weak.fetch_add(1, Ordering::Relaxed);
        if weak > MAX_COUNT {
            std::process::abort();
        }
        MyWeak {
            ptr: self.ptr,
            _owns: PhantomData,
        }
    }
}

impl<T> Drop for MyWeak<T> {
    fn drop(&mut self) {
        // Release, then an Acquire fence in the thread that reaches zero:
        // every access to the block by every other handle must happen before
        // the block is freed.
        if self.inner().weak.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        fence(Ordering::Acquire);
        // SAFETY: `weak` is zero, so no `MyWeak` exists and (because the
        // `MyArc`s hold one weak reference) no `MyArc` either. The pointer
        // came from `Box::leak` in `new` and nothing else frees it. The
        // value is `ManuallyDrop` and was already dropped by the last
        // `MyArc`, so freeing the box does not drop it again.
        drop(unsafe { Box::from_raw(self.ptr.as_ptr()) });
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use crate::implements;
    use crate::support::{
        DropCount,
        alloc::{count, frees},
        guess, within,
    };
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::MutexGuard;

    #[test]
    fn a_clones_share_one_value() {
        let first = MyArc::new(String::from("shared"));
        let second = first.clone();
        assert_eq!(*first, "shared");
        assert_eq!(
            first.as_ptr(),
            second.as_ptr(),
            "a clone must point to the same String, not to a copy"
        );
        assert!(MyArc::ptr_eq(&first, &second));
        assert!(!MyArc::ptr_eq(&first, &MyArc::new(String::from("shared"))));
        assert_eq!(MyArc::strong_count(&first), 2);
        drop(second);
        assert_eq!(MyArc::strong_count(&first), 1);
        assert_eq!(first.len(), 6);
    }

    #[test]
    fn a_value_is_dropped_with_the_last_strong_handle() {
        let drops = DropCount::new();
        let first = MyArc::new(drops.value(1));
        let second = first.clone();
        let third = second.clone();
        drop(first);
        drop(third);
        assert_eq!(
            drops.get(),
            0,
            "one handle is left, so the value must be alive"
        );
        assert_eq!(second.id, 1);
        drop(second);
        assert_eq!(
            drops.get(),
            1,
            "dropping the last handle must drop the value exactly once"
        );
    }

    #[test]
    fn b_weak_does_not_keep_the_value_alive() {
        let drops = DropCount::new();
        let strong = MyArc::new(drops.value(7));
        let weak = MyArc::downgrade(&strong);
        assert_eq!(MyArc::strong_count(&strong), 1);
        assert_eq!(MyArc::weak_count(&strong), 1);

        let upgraded = weak.upgrade().expect("a strong handle exists");
        assert_eq!(upgraded.id, 7);
        assert!(MyArc::ptr_eq(&strong, &upgraded));
        assert_eq!(MyArc::strong_count(&strong), 2);
        drop(upgraded);

        let second_weak = weak.clone();
        assert_eq!(MyArc::weak_count(&strong), 2);
        drop(second_weak);

        drop(strong);
        assert_eq!(
            drops.get(),
            1,
            "the value must be dropped with the last MyArc, although a MyWeak exists"
        );
        assert!(
            weak.upgrade().is_none(),
            "`upgrade` must fail once the last MyArc is gone"
        );
        assert!(weak.upgrade().is_none(), "and it must keep failing");
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn b_block_is_freed_with_the_last_handle_of_either_kind() {
        let strong = MyArc::new(5_u64);
        let weak = MyArc::downgrade(&strong);
        let ((), freed) = frees(|| drop(strong));
        assert_eq!(
            freed, 0,
            "a MyWeak still points to the block, so dropping the last MyArc must not free it"
        );
        let ((), freed) = frees(|| drop(weak));
        assert_eq!(
            freed, 1,
            "the last handle of any kind is gone, so the block must be freed now"
        );

        let strong = MyArc::new(5_u64);
        let weak = MyArc::downgrade(&strong);
        let ((), freed) = frees(|| drop(weak));
        assert_eq!(freed, 0, "a MyArc still points to the block");
        let ((), freed) = frees(|| drop(strong));
        assert_eq!(
            freed, 1,
            "without any MyWeak, the last MyArc frees the block"
        );
    }

    #[test]
    fn c_one_allocation_one_pointer() {
        let (handle, allocations) = count(|| MyArc::new(std::hint::black_box(9_u64)));
        assert!(
            allocations <= 1,
            "`MyArc::new` made {allocations} heap allocations; the counters and the \
             value belong in one block"
        );
        let (parts, allocations) = count(|| {
            let clone = handle.clone();
            let weak = MyArc::downgrade(&handle);
            let upgraded = weak.upgrade();
            (clone, weak, upgraded)
        });
        assert_eq!(
            allocations, 0,
            "clone, downgrade and upgrade made {allocations} heap allocation(s); \
             they only change a counter"
        );
        drop(parts);
        assert_eq!(
            std::mem::size_of::<MyArc<[u8; 64]>>(),
            std::mem::size_of::<usize>(),
            "a MyArc is one pointer, whatever the size of the value"
        );
        assert_eq!(
            std::mem::size_of::<Option<MyArc<u8>>>(),
            std::mem::size_of::<usize>(),
            "with `NonNull`, `None` is the null pointer and `Option<MyArc<T>>` needs no extra space"
        );
        assert_eq!(
            std::mem::size_of::<Option<MyWeak<u8>>>(),
            std::mem::size_of::<usize>()
        );
    }

    #[test]
    fn c_send_and_sync_need_both_bounds() {
        let _ = MyArc::new(0_u8);
        assert!(implements!(MyArc<u32>: Send + Sync));
        assert!(implements!(MyWeak<u32>: Send + Sync));
        assert!(
            !implements!(MyArc<Cell<u32>>: Send),
            "`Cell<u32>` is `Send` but not `Sync`. Two MyArcs on two threads would \
             share `&Cell<u32>`, so MyArc<Cell<u32>> must not be `Send`"
        );
        assert!(!implements!(MyArc<Cell<u32>>: Sync));
        assert!(
            !implements!(MyArc<MutexGuard<'static, u32>>: Sync),
            "`MutexGuard` is `Sync` but not `Send`. A thread with `&MyArc` can clone \
             it and later drop the last handle, which drops the guard on the wrong thread"
        );
        assert!(!implements!(MyArc<MutexGuard<'static, u32>>: Send));
        assert!(!implements!(MyArc<Rc<u32>>: Send));
        assert!(
            !implements!(MyWeak<Cell<u32>>: Send),
            "a MyWeak can be upgraded, so it needs the same bounds as MyArc"
        );
        assert!(!implements!(MyWeak<MutexGuard<'static, u32>>: Sync));
    }

    #[test]
    fn d_threads_clone_and_drop() {
        let rounds = if cfg!(miri) { 20 } else { 2_000 };
        let drops = DropCount::new();
        within(30, "the clone and drop stress test", move || {
            let shared = MyArc::new(drops.value(3));
            let workers: Vec<_> = (0..4)
                .map(|_| {
                    let mine = shared.clone();
                    std::thread::spawn(move || {
                        for _ in 0..rounds {
                            let extra = mine.clone();
                            assert_eq!(extra.id, 3);
                        }
                    })
                })
                .collect();
            drop(shared);
            for worker in workers {
                worker.join().unwrap();
            }
        });
        assert_eq!(
            drops.get(),
            1,
            "after all threads are done, the value must have been dropped exactly once"
        );
    }

    #[test]
    fn d_upgrade_races_with_the_last_drop() {
        let rounds = if cfg!(miri) { 20 } else { 2_000 };
        within(30, "the upgrade stress test", move || {
            for _ in 0..rounds {
                let drops = DropCount::new();
                let strong = MyArc::new(drops.value(1));
                let weak = MyArc::downgrade(&strong);
                let upgrader = std::thread::spawn(move || {
                    // Either the value is still alive and stays alive while
                    // we look at it, or we get `None`.
                    weak.upgrade().map(|alive| alive.id)
                });
                drop(strong);
                let seen = upgrader.join().unwrap();
                assert!(seen.is_none() || seen == Some(1));
                assert_eq!(drops.get(), 1, "the value must be dropped exactly once");
            }
        });
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// These predictions use the standard library's `Arc` and `Weak`.
    #[test]
    fn p_predictions() {
        use std::sync::Arc;

        // P1. Counts after one downgrade and one clone.
        let first = Arc::new(5);
        let weak = Arc::downgrade(&first);
        let second = Arc::clone(&first);
        let p1: (usize, usize) = (2, 1);
        assert_eq!(
            p1,
            (Arc::strong_count(&first), Arc::weak_count(&first)),
            "P1: (strong count, weak count) with two Arcs and one Weak"
        );

        // P2. `get_mut` gives `&mut T` only to a unique owner.
        let mut only = Arc::new(5);
        let observer = Arc::downgrade(&only);
        let with_weak = Arc::get_mut(&mut only).is_some();
        drop(observer);
        let without_weak = Arc::get_mut(&mut only).is_some();
        let p2: (bool, bool) = (false, true);
        assert_eq!(
            p2,
            (with_weak, without_weak),
            "P2: does `get_mut` succeed (while a Weak exists, after it is dropped)?"
        );

        // P3. What a Weak reports after every Arc is gone.
        drop(first);
        drop(second);
        let p3: (bool, usize, usize) = (false, 0, 0);
        assert_eq!(
            p3,
            (
                weak.upgrade().is_some(),
                weak.strong_count(),
                weak.weak_count()
            ),
            "P3: (upgrade succeeds, strong count, weak count) of a Weak without any Arc"
        );
    }
}

#[cfg(all(test, loom))]
mod model {
    use super::*;
    use crate::support::{DropCount, Tracked, model, model_bounded};
    use crate::sync::thread;

    #[test]
    fn a_last_drop_on_either_thread() {
        model(|| {
            let drops = DropCount::new();
            let mine = MyArc::new(Tracked::counted(5_u32, &drops));
            let theirs = mine.clone();
            let other = thread::spawn(move || {
                assert_eq!(theirs.get(), 5);
                // Possibly the last handle: then this thread drops the value.
            });
            assert_eq!(mine.get(), 5);
            // Possibly the last handle. Whichever thread drops the value,
            // the drop must be ordered after the read on the other thread.
            drop(mine);
            other.join().unwrap();
            assert_eq!(drops.get(), 1, "the value must be dropped exactly once");
        });
    }

    #[test]
    fn b_upgrade_never_revives_a_dropped_value() {
        model(|| {
            let drops = DropCount::new();
            let strong = MyArc::new(Tracked::counted(5_u32, &drops));
            let weak = MyArc::downgrade(&strong);
            let other = thread::spawn(move || drop(strong));
            if let Some(alive) = weak.upgrade() {
                // The upgrade succeeded, so the value must stay alive here.
                assert_eq!(alive.get(), 5);
            }
            other.join().unwrap();
            assert_eq!(
                drops.get(),
                1,
                "the value must be dropped exactly once: `upgrade` either sees a \
                 live value and keeps it alive, or returns `None`"
            );
            assert!(weak.upgrade().is_none());
        });
    }

    #[test]
    fn b_two_upgrades_and_a_drop() {
        model_bounded(2, || {
            let drops = DropCount::new();
            let strong = MyArc::new(Tracked::counted(5_u32, &drops));
            let weak = MyArc::downgrade(&strong);
            let upgraders: Vec<_> = (0..2)
                .map(|_| {
                    let weak = weak.clone();
                    thread::spawn(move || weak.upgrade().map(|alive| alive.get()))
                })
                .collect();
            drop(strong);
            for upgrader in upgraders {
                let seen = upgrader.join().unwrap();
                assert!(seen.is_none() || seen == Some(5));
            }
            assert_eq!(drops.get(), 1);
        });
    }
}
