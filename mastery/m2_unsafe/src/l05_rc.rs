//! LESSON 05 — Shared ownership: building Rc and Weak
//! Run: cargo test --manifest-path mastery/m2_unsafe/Cargo.toml l05_
//! Miri: cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml l05_
//! Read first: "Lesson 05" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. The last `Rc` is dropped while a `Weak` still exists. What happens to
//!    the value at that moment, and what happens to the memory?
//! 2. The value's own destructor holds a `Weak` to its allocation and drops
//!    it. What keeps the allocation alive until the destructor has returned?
//!
//! IMPLEMENT the contract below. From this lesson on, you design the types:
//! this file contains no signatures. The tests are switched off until your
//! types exist. A good order of work: write both types with every function of
//! the contract as a `todo!()` body, change `off` to `on` in the `gate!` line
//! above the tests, make the file compile, then fill in the bodies. Write the
//! invariant of the shared allocation first. Add one edge-case test.
//! EXPLAIN afterward: Give the invariant that links the two counts to the
//! state of the value and of the allocation. Why does `get_mut` have to look
//! at the weak count, and why must the counts never wrap around?
//!
//! MIRI LAB: `examples/miri_l05_rc.rs` is a reference-counted pointer with a
//! weak pointer and one count too few. Predict what Miri reports, run
//! `cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l05_rc`,
//! compare with CONCEPTS.md, then repair the example until Miri is silent.
//!
//! CONTRACT
//!
//! Two public types, for sized `T`: `MyRc<T>` and `MyWeak<T>`.
//!
//! `MyRc<T>` is one of several owners of a value on the heap, on one thread.
//! - `MyRc::new(value: T) -> MyRc<T>` makes one allocation that holds the
//!   value and whatever you need to count.
//! - `Clone` gives another owner of the same allocation. It does not allocate
//!   and does not need `T: Clone`.
//! - `Deref<Target = T>`.
//! - The other functions are associated functions, called as
//!   `MyRc::strong_count(&rc)`, so that they cannot hide a method of `T`:
//!   - `strong_count(this: &Self) -> usize`: the number of `MyRc` owners.
//!   - `weak_count(this: &Self) -> usize`: the number of `MyWeak` pointers.
//!   - `ptr_eq(a: &Self, b: &Self) -> bool`: same allocation?
//!   - `get_mut(this: &mut Self) -> Option<&mut T>`: `Some` only if no other
//!     `MyRc` and no `MyWeak` points to the allocation.
//!   - `try_unwrap(this: Self) -> Result<T, Self>`: `Ok(value)` if `this` is
//!     the only `MyRc`. `MyWeak` pointers may exist; they stop upgrading.
//!   - `downgrade(this: &Self) -> MyWeak<T>`.
//! - When the last `MyRc` is dropped, the value is dropped at once, even if
//!   `MyWeak` pointers remain. The value's destructor may use `MyWeak`
//!   pointers to its own allocation.
//!
//! `MyWeak<T>` points to the allocation without owning the value.
//! - `upgrade(&self) -> Option<MyRc<T>>`: `None` once the last `MyRc` is
//!   gone. That is from the moment the value's destructor starts to run, not
//!   only after it has returned, and after `try_unwrap` has moved the value out.
//! - `Clone` and `Drop`.
//!
//! For both types:
//! - The allocation is freed when the last `MyRc` and the last `MyWeak` are
//!   gone, and not before. It is freed even if the value's destructor panics.
//! - Each is one pointer in size, and so is `Option<MyRc<T>>`.
//! - Neither is `Send` or `Sync`. `MyRc<T>` is covariant in `T`.
//! - A count never wraps around: panic (or abort) instead. No test can reach
//!   that case, but soundness needs it.
//!
//! You may allocate with `Box::new` and `Box::into_raw`, or with your `MyBox`,
//! or with `std::alloc`. Do not use `std::rc`.

crate::gate! { off, "05" =>
#[cfg(test)]
mod tests {
    use super::*;
    use crate::impls;
    use crate::support::alloc::{audit, counting};
    use std::cell::RefCell;
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

    /// Writes its name to a shared log when dropped.
    struct Noisy<'a>(char, &'a RefCell<String>);

    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(self.0);
        }
    }

    /// A log with room for every test's entries, so that writing to it inside
    /// an `audit` does not count as an allocation of the code under test.
    fn new_log() -> RefCell<String> {
        RefCell::new(String::with_capacity(64))
    }

    /// Not `Clone`: cloning a `MyRc<Plain>` must still work.
    struct Plain(u32);

    /// Compiles only if `MyRc<T>` is covariant in `T`, as `Rc<T>` is.
    fn _covariant<'a>(long: MyRc<&'static str>) -> MyRc<&'a str> {
        long
    }

    #[test]
    fn a_clones_share_one_value() {
        let (first, heap) = audit(|| MyRc::new(Plain(7)));
        assert!(
            heap.allocations <= 1,
            "MyRc::new made {} allocations; the counts and the value share one",
            heap.allocations
        );
        let (second, heap) = audit(|| first.clone());
        assert_eq!(heap.allocations, 0, "cloning a MyRc must not allocate");
        assert_eq!(second.0, 7);
        assert!(
            std::ptr::eq::<Plain>(&*first, &*second),
            "both owners must point at the same value, not at copies"
        );
        assert!(MyRc::ptr_eq(&first, &second));
        assert!(!MyRc::ptr_eq(&first, &MyRc::new(Plain(7))));
        assert_eq!(MyRc::strong_count(&first), 2);
        drop(second);
        assert_eq!(MyRc::strong_count(&first), 1);
        assert_eq!(MyRc::weak_count(&first), 0);
    }

    #[test]
    fn a_the_last_owner_drops_the_value_and_frees_the_allocation() {
        let log = new_log();
        let ((), heap) = audit(|| {
            let first = MyRc::new(Noisy('v', &log));
            let second = first.clone();
            let third = second.clone();
            drop(first);
            drop(third);
            assert_eq!(*log.borrow(), "", "an owner is still alive");
            drop(second);
            assert_eq!(*log.borrow(), "v", "the last owner drops the value");
        });
        assert_eq!(*log.borrow(), "v", "the value must be dropped exactly once");
        assert_eq!(heap.live, 0, "the allocation must be freed");
        assert_eq!(
            heap.bad_frees, 0,
            "the allocation must be freed with the layout it was allocated with"
        );
    }

    #[test]
    fn a_pointers_are_one_word_and_stay_on_their_thread() {
        assert_eq!(size_of::<MyRc<[u64; 4]>>(), size_of::<usize>());
        assert_eq!(size_of::<MyWeak<[u64; 4]>>(), size_of::<usize>());
        assert_eq!(
            size_of::<Option<MyRc<u8>>>(),
            size_of::<usize>(),
            "a NonNull pointer lets Option use the null value for None"
        );
        assert!(
            !impls!(MyRc<u8>: Send) && !impls!(MyRc<u8>: Sync),
            "MyRc must be neither Send nor Sync: its counts are not atomic"
        );
        assert!(
            !impls!(MyWeak<u8>: Send) && !impls!(MyWeak<u8>: Sync),
            "MyWeak must be neither Send nor Sync: its counts are not atomic"
        );
    }

    #[test]
    fn b_get_mut_needs_the_only_pointer() {
        let mut first = MyRc::new(String::from("a"));
        MyRc::get_mut(&mut first)
            .expect("the only MyRc must give mutable access")
            .push('b');
        let second = first.clone();
        assert!(
            MyRc::get_mut(&mut first).is_none(),
            "a second MyRc could read the value while it is changed"
        );
        drop(second);
        let weak = MyRc::downgrade(&first);
        assert!(
            MyRc::get_mut(&mut first).is_none(),
            "a MyWeak could be upgraded and then read the value while it is changed"
        );
        drop(weak);
        MyRc::get_mut(&mut first)
            .expect("all other pointers are gone")
            .push('c');
        assert_eq!(*first, "abc");
    }

    #[test]
    fn b_try_unwrap_moves_the_value_out_of_the_only_owner() {
        let log = new_log();
        let first = MyRc::new((Noisy('v', &log), String::from("payload")));
        let text = first.1.as_ptr();
        let second = first.clone();
        let first = MyRc::try_unwrap(first)
            .err()
            .expect("two owners: try_unwrap must give the MyRc back");
        assert_eq!(MyRc::strong_count(&first), 2);
        drop(second);
        let (value, heap) = audit(|| MyRc::try_unwrap(first).ok());
        let value = value.expect("one owner: try_unwrap must give the value");
        assert_eq!(*log.borrow(), "", "try_unwrap must not drop the value");
        assert!(
            std::ptr::eq(value.1.as_ptr(), text),
            "try_unwrap must move the value out, not clone it"
        );
        if counting() {
            assert_eq!(
                heap.live, -1,
                "no MyWeak exists, so try_unwrap must free the allocation"
            );
        }
        drop(value);
        assert_eq!(*log.borrow(), "v");
    }

    #[test]
    fn c_weak_pointers_do_not_keep_the_value_alive() {
        let log = new_log();
        let strong = MyRc::new(Noisy('v', &log));
        let weak = MyRc::downgrade(&strong);
        let weak_too = weak.clone();
        assert_eq!(
            (MyRc::strong_count(&strong), MyRc::weak_count(&strong)),
            (1, 2)
        );
        let upgraded = weak.upgrade().expect("the value is alive");
        assert!(MyRc::ptr_eq(&strong, &upgraded));
        assert_eq!(MyRc::strong_count(&strong), 2);
        drop(upgraded);
        drop(weak_too);
        assert_eq!(MyRc::weak_count(&strong), 1);

        let ((), heap) = audit(|| drop(strong));
        assert_eq!(
            *log.borrow(),
            "v",
            "the last MyRc drops the value at once, even while a MyWeak exists"
        );
        assert_eq!(
            heap.live, 0,
            "the allocation must stay while a MyWeak points to it"
        );
        assert!(
            weak.upgrade().is_none(),
            "a MyWeak must not upgrade after the value was dropped"
        );
        assert!(weak.clone().upgrade().is_none());
        let ((), heap) = audit(|| drop(weak));
        if counting() {
            assert_eq!(heap.live, -1, "the last MyWeak must free the allocation");
        }
        assert_eq!(*log.borrow(), "v", "the value must be dropped exactly once");
    }

    #[test]
    fn c_try_unwrap_leaves_weak_pointers_dangling_but_safe() {
        let strong = MyRc::new(String::from("moved out"));
        let weak = MyRc::downgrade(&strong);
        let value = MyRc::try_unwrap(strong)
            .ok()
            .expect("MyWeak pointers do not stop try_unwrap");
        assert_eq!(value, "moved out");
        assert!(
            weak.upgrade().is_none(),
            "after try_unwrap there is no value to upgrade to"
        );
        let ((), heap) = audit(|| drop(weak));
        if counting() {
            assert_eq!(heap.live, -1, "the last MyWeak must free the allocation");
        }
    }

    /// A value that keeps a weak pointer to its own allocation and uses it in
    /// its destructor.
    struct Selfish<'a> {
        me: RefCell<Option<MyWeak<Selfish<'a>>>>,
        log: &'a RefCell<String>,
    }

    impl Drop for Selfish<'_> {
        fn drop(&mut self) {
            if let Some(me) = self.me.borrow_mut().take() {
                let alive = me.upgrade().is_some();
                self.log.borrow_mut().push(if alive { '!' } else { 'x' });
                // `me` is dropped here: the last MyWeak goes away while the
                // value's destructor is still running.
            }
        }
    }

    #[test]
    fn c_a_destructor_may_use_weak_pointers_to_its_own_allocation() {
        let log = new_log();
        let ((), heap) = audit(|| {
            let strong = MyRc::new(Selfish {
                me: RefCell::new(None),
                log: &log,
            });
            *strong.me.borrow_mut() = Some(MyRc::downgrade(&strong));
            assert_eq!(MyRc::weak_count(&strong), 1);
            drop(strong);
        });
        assert_eq!(
            *log.borrow(),
            "x",
            "while the value is being dropped, upgrade must return None"
        );
        assert_eq!(
            heap.live, 0,
            "the allocation must be freed after the destructor has returned"
        );
        assert_eq!(heap.bad_frees, 0);
    }

    struct Tree {
        parent: RefCell<Option<MyWeak<Tree>>>,
        children: RefCell<Vec<MyRc<Tree>>>,
    }

    #[test]
    fn c_a_parent_link_through_weak_does_not_leak() {
        let ((), heap) = audit(|| {
            let root = MyRc::new(Tree {
                parent: RefCell::new(None),
                children: RefCell::new(Vec::new()),
            });
            let leaf = MyRc::new(Tree {
                parent: RefCell::new(Some(MyRc::downgrade(&root))),
                children: RefCell::new(Vec::new()),
            });
            root.children.borrow_mut().push(leaf.clone());
            let parent = leaf.parent.borrow().as_ref().and_then(MyWeak::upgrade);
            assert!(parent.is_some_and(|parent| MyRc::ptr_eq(&parent, &root)));
            assert_eq!((MyRc::strong_count(&leaf), MyRc::weak_count(&root)), (2, 1));
        });
        assert_eq!(heap.live, 0, "a tree with weak parent links must be freed");
    }

    #[test]
    fn c_the_allocation_is_freed_even_if_the_destructor_panics() {
        struct Bomb;
        impl Drop for Bomb {
            fn drop(&mut self) {
                // Unwinds like a panic, without printing a message.
                resume_unwind(Box::new("destructor failed"));
            }
        }
        let (panicked, heap) = audit(|| {
            let strong = MyRc::new((Bomb, 1u64));
            catch_unwind(AssertUnwindSafe(move || drop(strong))).is_err()
        });
        assert!(panicked, "the destructor's panic must reach the caller");
        assert_eq!(
            heap.live, 0,
            "the allocation must be freed even when the value's destructor panics"
        );
    }
}
}

#[cfg(test)]
mod predictions {
    use crate::guess;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. The standard Rc: (strong count, weak count) at the end.
        let first = Rc::new(5);
        let second = Rc::clone(&first);
        let third = Rc::clone(&second);
        let weak = Rc::downgrade(&third);
        drop(second);
        let p1: (usize, usize) = guess();
        assert_eq!(
            p1,
            (Rc::strong_count(&first), Rc::weak_count(&first)),
            "P1: counts (left: your prediction, right: actual)"
        );
        drop((third, weak));

        // P2. One Rc and one Weak: does get_mut give access? And after the
        // Weak is dropped?
        let mut only = Rc::new(String::from("x"));
        let weak = Rc::downgrade(&only);
        let with_weak = Rc::get_mut(&mut only).is_some();
        drop(weak);
        let without_weak = Rc::get_mut(&mut only).is_some();
        let p2: (bool, bool) = guess();
        assert_eq!(
            p2,
            (with_weak, without_weak),
            "P2: get_mut and weak pointers (left: your prediction, right: actual)"
        );

        // P3. `Noisy` writes 'd' to the log when it is dropped. What is in the
        // log at the end, and can the weak pointer still be upgraded?
        struct Noisy<'a>(&'a RefCell<String>);
        impl Drop for Noisy<'_> {
            fn drop(&mut self) {
                self.0.borrow_mut().push('d');
            }
        }
        let log = RefCell::new(String::new());
        let weak = {
            let strong = Rc::new(Noisy(&log));
            let weak = Rc::downgrade(&strong);
            log.borrow_mut().push('a');
            weak
        };
        log.borrow_mut().push('b');
        let p3: (&str, bool) = guess();
        assert_eq!(
            p3,
            (log.borrow().as_str(), weak.upgrade().is_some()),
            "P3: when the value is dropped (left: your prediction, right: actual)"
        );
    }
}
