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

// The imports above are the ones the reference solution uses. Remove what
// your design does not need.

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

/// A shared owner of a `T` on the heap.
///
/// Replace the placeholder field with your own. Write the `Drop` impl, and the
/// `unsafe impl Send` and `unsafe impl Sync` with a SAFETY comment each.
pub struct MyArc<T> {
    _replace_me: PhantomData<T>,
}

/// A pointer to the same block that does not keep the `T` alive.
///
/// Replace the placeholder field with your own. This type needs `Drop`, `Send`
/// and `Sync` impls too.
pub struct MyWeak<T> {
    _replace_me: PhantomData<T>,
}

impl<T> MyArc<T> {
    pub fn new(value: T) -> Self {
        todo!("Implement this task")
    }

    /// A new `MyWeak` to the same block.
    pub fn downgrade(this: &Self) -> MyWeak<T> {
        todo!("Implement this task")
    }

    /// How many `MyArc`s point to this block. Other threads may change the
    /// number at any time, so treat the answer as a hint.
    pub fn strong_count(this: &Self) -> usize {
        todo!("Implement this task")
    }

    /// How many `MyWeak`s point to this block (a hint, as above).
    pub fn weak_count(this: &Self) -> usize {
        todo!("Implement this task")
    }

    /// Do `a` and `b` point to the same block?
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        todo!("Implement this task")
    }
}

impl<T> Deref for MyArc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        todo!("Implement this task")
    }
}

impl<T> Clone for MyArc<T> {
    fn clone(&self) -> Self {
        todo!("Implement this task")
    }
}

impl<T> MyWeak<T> {
    /// A new `MyArc`, if at least one still exists.
    pub fn upgrade(&self) -> Option<MyArc<T>> {
        todo!("Implement this task")
    }
}

impl<T> Clone for MyWeak<T> {
    fn clone(&self) -> Self {
        todo!("Implement this task")
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
        let p1: (usize, usize) = guess();
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
        let p2: (bool, bool) = guess();
        assert_eq!(
            p2,
            (with_weak, without_weak),
            "P2: does `get_mut` succeed (while a Weak exists, after it is dropped)?"
        );

        // P3. What a Weak reports after every Arc is gone.
        drop(first);
        drop(second);
        let p3: (bool, usize, usize) = guess();
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
