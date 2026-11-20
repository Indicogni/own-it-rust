//! LESSON 01 — UnsafeCell: building Cell and RefCell
//! Run: cargo test --manifest-path mastery/m2_unsafe/Cargo.toml l01_
//! Miri: cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml l01_
//! Read first: "Lesson 01" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `&T` promises that the value does not change. Which type removes that
//!    promise, and what does the compiler stop assuming when it sees that type?
//! 2. `Cell::get` needs `T: Copy`. What could go wrong with `T: Clone`?
//!
//! IMPLEMENT the tasks below. The fields and signatures are given. Every
//! `unsafe` block you write needs a `// SAFETY:` comment that names the facts
//! that make it sound; `cargo clippy` warns when one is missing. Keep the
//! supplied tests and add one edge case.
//! EXPLAIN afterward: State the invariant that makes `MyCell` sound without any
//! run-time check, and the invariant that `MyRefCell` checks at run time. Why
//! must `set` drop the old value only after the new value is in place?
//!
//! MIRI LAB: `examples/miri_l01_cell.rs` is a "cell" without `UnsafeCell`. Its
//! native run prints a plausible result. Predict what Miri reports, run
//! `cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l01_cell`,
//! compare with CONCEPTS.md, then repair the example until Miri is silent.

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

/// A. A value that can be replaced through a shared reference.
///
/// Invariant: no reference to the inside of the cell exists while a method that
/// takes `&self` runs, because no such method ever returns one, and no code of
/// `T` (a destructor, a `Clone` impl) runs while a method is reading or writing
/// the inside. `MyCell` is not `Sync`, so only one thread can be inside a method.
pub struct MyCell<T> {
    value: UnsafeCell<T>,
}

impl<T> MyCell<T> {
    /// Wraps a value. Adds no bytes: `MyCell<T>` is as large as `T`.
    pub const fn new(value: T) -> MyCell<T> {
        MyCell {
            value: UnsafeCell::new(value),
        }
    }

    /// Stores `value` and returns the old value. Runs no code of `T`.
    pub fn replace(&self, value: T) -> T {
        // SAFETY: `get()` points to the `T` inside `self`, so it is non-null,
        // aligned, initialized, and valid for reads and writes for the duration
        // of this call. By the type's invariant no reference to that `T` exists:
        // no `&self` method hands one out, and `MyCell` is `!Sync`, so no other
        // thread is in a method. `ptr::replace` only moves bytes; it runs no
        // code of `T`, so nothing can reenter the cell during the access.
        unsafe { std::ptr::replace(self.value.get(), value) }
    }

    /// Stores `value` and drops the old value. The old value's destructor may
    /// use this cell again, so it must run only after `value` is in place and
    /// no pointer into the cell is in use.
    pub fn set(&self, value: T) {
        // The old value is a plain local here: when its destructor runs, the
        // cell already holds `value` and no access to the inside is in progress.
        drop(self.replace(value));
    }

    /// Replaces the value with `T::default()` and returns the old value.
    pub fn take(&self) -> T
    where
        T: Default,
    {
        // `T::default()` runs before `replace` touches the inside.
        self.replace(T::default())
    }

    /// Exchanges the values of two cells. `other` may be the same cell as
    /// `self`; then nothing changes.
    pub fn swap(&self, other: &MyCell<T>) {
        // SAFETY: both pointers are valid for reads and writes and no reference
        // to either inside exists, for the reasons given in `replace`.
        // `ptr::swap` allows its two arguments to be the same location (unlike
        // `mem::swap`, which would need two `&mut T` to one value), and it runs
        // no code of `T`.
        unsafe { std::ptr::swap(self.value.get(), other.value.get()) }
    }

    /// Unwraps the value. No `unsafe` is needed: `self` is owned.
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    /// Borrows the inside. No `unsafe` is needed: `&mut self` proves that no
    /// other reference to the cell exists.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
}

impl<T: Copy> MyCell<T> {
    /// Returns a copy of the value.
    pub fn get(&self) -> T {
        // SAFETY: the pointer is valid for reads and no `&mut` to the inside
        // exists (see `replace`). `T: Copy`, so the read is a plain byte copy:
        // it runs no code of `T` and leaves the cell's value usable.
        unsafe { *self.value.get() }
    }
}

/// B. A value with borrow rules that are checked while the program runs.
///
/// `state` counts the guards that are alive: `0` means none, `n > 0` means `n`
/// [`Ref`] guards, and `-1` means one [`RefMut`] guard.
///
/// Invariant: a `&T` into `value` exists only while a `Ref` is alive, and a
/// `&mut T` only while the single `RefMut` is alive. `state` is changed only by
/// the `try_borrow*` methods and by the guards' destructors.
pub struct MyRefCell<T> {
    state: MyCell<isize>,
    value: UnsafeCell<T>,
}

/// A shared borrow of the value in a [`MyRefCell`]. Ends when it is dropped.
pub struct Ref<'b, T> {
    cell: &'b MyRefCell<T>,
}

/// The exclusive borrow of the value in a [`MyRefCell`]. Ends when it is dropped.
pub struct RefMut<'b, T> {
    cell: &'b MyRefCell<T>,
}

impl<T> MyRefCell<T> {
    pub const fn new(value: T) -> MyRefCell<T> {
        MyRefCell {
            state: MyCell::new(0),
            value: UnsafeCell::new(value),
        }
    }

    /// A shared borrow, or `None` while a `RefMut` is alive. The count must
    /// never wrap around: return `None` when it cannot grow any more (leaked
    /// guards can bring it that far).
    pub fn try_borrow(&self) -> Option<Ref<'_, T>> {
        let state = self.state.get();
        if state < 0 || state == isize::MAX {
            return None;
        }
        self.state.set(state + 1);
        Some(Ref { cell: self })
    }

    /// The exclusive borrow, or `None` while any guard is alive.
    pub fn try_borrow_mut(&self) -> Option<RefMut<'_, T>> {
        if self.state.get() != 0 {
            return None;
        }
        self.state.set(-1);
        Some(RefMut { cell: self })
    }

    /// Like `try_borrow`, but panics with a message that contains
    /// `already mutably borrowed`.
    pub fn borrow(&self) -> Ref<'_, T> {
        self.try_borrow().expect("already mutably borrowed")
    }

    /// Like `try_borrow_mut`, but panics with a message that contains
    /// `already borrowed`.
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.try_borrow_mut().expect("already borrowed")
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    /// Borrows the inside without touching `state`: `&mut self` already proves
    /// that no guard is alive.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
}

impl<T> Deref for Ref<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: the pointer is valid for reads for as long as the cell is
        // borrowed, which the guard's lifetime `'b` ensures. While this `Ref`
        // is alive, `state` is positive, so `try_borrow_mut` refuses and no
        // `&mut T` to the inside exists or can be created. The returned `&T`
        // cannot outlive `self`, so it ends before this guard's destructor
        // lowers the count.
        unsafe { &*self.cell.value.get() }
    }
}

impl<T> Drop for Ref<'_, T> {
    fn drop(&mut self) {
        self.cell.state.set(self.cell.state.get() - 1);
    }
}

impl<T> Deref for RefMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: the pointer is valid for reads while the cell is borrowed.
        // `state` is `-1` while this `RefMut` is alive, so no other guard
        // exists; the only other way to the inside is `deref_mut` on this same
        // guard, which needs `&mut self` and so cannot overlap with the `&self`
        // that the returned `&T` is tied to.
        unsafe { &*self.cell.value.get() }
    }
}

impl<T> DerefMut for RefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the pointer is valid for reads and writes while the cell is
        // borrowed. `state` is `-1` while this `RefMut` is alive, so every
        // `try_borrow*` call refuses and this guard is the only way to the
        // inside. The returned `&mut T` borrows `self` mutably, so no second
        // reference can be produced from this guard while it is in use.
        unsafe { &mut *self.cell.value.get() }
    }
}

impl<T> Drop for RefMut<'_, T> {
    fn drop(&mut self) {
        self.cell.state.set(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{guess, impls};
    use std::cell::{Cell, RefCell};
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

    #[test]
    fn a_cell_replaces_through_a_shared_reference() {
        let cell = MyCell::new(5);
        let (first, second) = (&cell, &cell);
        first.set(6);
        assert_eq!(second.get(), 6);
        assert_eq!(second.replace(7), 6);
        assert_eq!(cell.take(), 7);
        assert_eq!(cell.get(), 0, "take leaves the default value behind");
        let mut cell = cell;
        *cell.get_mut() += 3;
        assert_eq!(cell.into_inner(), 3);
    }

    #[test]
    fn a_cell_moves_values_that_are_not_copy() {
        let cell = MyCell::new(String::from("old"));
        assert_eq!(cell.replace(String::from("new")), "old");
        cell.set(String::from("newer"));
        assert_eq!(cell.take(), "newer");
        assert_eq!(cell.into_inner(), "");
    }

    #[test]
    fn a_swap_exchanges_two_cells_and_tolerates_one() {
        let (a, b) = (
            MyCell::new(String::from("a")),
            MyCell::new(String::from("b")),
        );
        a.swap(&b);
        assert_eq!((a.take(), b.take()), ("b".to_string(), "a".to_string()));
        let same = MyCell::new(vec![1, 2, 3]);
        same.swap(&same);
        assert_eq!(
            same.take(),
            [1, 2, 3],
            "swapping a cell with itself must leave the value alone"
        );
    }

    #[test]
    fn a_cell_costs_no_memory_and_is_not_sync() {
        assert_eq!(MyCell::new(1u64).get(), 1);
        assert_eq!(
            size_of::<MyCell<u64>>(),
            size_of::<u64>(),
            "a MyCell<u64> must be exactly as large as a u64"
        );
        assert!(
            impls!(MyCell<u8>: Send),
            "moving a whole cell to another thread is fine when its value may move"
        );
        assert!(
            !impls!(MyCell<u8>: Sync),
            "MyCell must not be Sync: two threads could call `set` at the same time"
        );
        assert!(
            !impls!(MyRefCell<u8>: Sync),
            "MyRefCell must not be Sync: its borrow count is not atomic"
        );
        assert!(
            !impls!(Ref<'static, u8>: Send),
            "a Ref must stay on its thread: its destructor changes the borrow count"
        );
    }

    /// A value whose destructor writes to the cell it used to live in.
    struct Reentrant<'a> {
        home: *const MyCell<Option<Reentrant<'a>>>,
        drops: &'a Cell<u32>,
    }

    impl Drop for Reentrant<'_> {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
            // Disarm first, so that a second run of this destructor on the same
            // bytes (a bug in the cell) cannot recurse without end.
            let home = std::mem::replace(&mut self.home, std::ptr::null());
            if !home.is_null() {
                // SAFETY: the test keeps the cell alive and in place while a
                // value with a non-null `home` exists.
                unsafe { (*home).set(None) };
            }
        }
    }

    #[test]
    fn a_set_drops_the_old_value_after_the_write() {
        let (old_drops, new_drops) = (Cell::new(0), Cell::new(0));
        let cell = MyCell::new(None);
        cell.set(Some(Reentrant {
            home: &raw const cell,
            drops: &old_drops,
        }));
        // Dropping the old value calls `cell.set(None)` from its destructor.
        cell.set(Some(Reentrant {
            home: std::ptr::null(),
            drops: &new_drops,
        }));
        assert_eq!(
            old_drops.get(),
            1,
            "the old value's destructor ran {} times: `set` must move the old value out \
             of the cell before dropping it",
            old_drops.get()
        );
        assert_eq!(
            new_drops.get(),
            1,
            "the destructor's own `set(None)` runs after the new value is stored, so it \
             replaces (and drops) the new value"
        );
        assert!(cell.take().is_none(), "the cell must end up holding `None`");
    }

    #[test]
    fn b_many_readers_or_one_writer() {
        let cell = MyRefCell::new(vec![1]);
        {
            let (r1, r2) = (cell.borrow(), cell.borrow());
            assert_eq!(r1.len() + r2.len(), 2);
            assert!(
                cell.try_borrow_mut().is_none(),
                "no RefMut may exist while a Ref is alive"
            );
        }
        {
            let mut writer = cell.borrow_mut();
            writer.push(2);
            assert_eq!(writer.len(), 2);
            assert!(
                cell.try_borrow().is_none(),
                "no Ref may exist while the RefMut is alive"
            );
            assert!(
                cell.try_borrow_mut().is_none(),
                "only one RefMut may exist at a time"
            );
        }
        assert_eq!(*cell.borrow(), [1, 2]);
        assert_eq!(cell.into_inner(), [1, 2]);
    }

    #[test]
    fn b_each_guard_gives_back_exactly_its_own_borrow() {
        let cell = MyRefCell::new(0u8);
        let (r1, r2) = (cell.borrow(), cell.borrow());
        drop(r1);
        assert!(
            cell.try_borrow_mut().is_none(),
            "one Ref is still alive, so the cell must still refuse a RefMut"
        );
        drop(r2);
        let writer = cell.try_borrow_mut();
        assert!(
            writer.is_some(),
            "all Refs are gone, so a RefMut must be granted"
        );
        drop(writer);
        assert!(
            cell.try_borrow().is_some(),
            "the RefMut is gone, so a Ref must be granted"
        );
    }

    #[test]
    #[should_panic(expected = "already mutably borrowed")]
    fn b_borrow_panics_while_a_writer_is_alive() {
        let cell = MyRefCell::new(1);
        let _writer = cell.borrow_mut();
        let _reader = cell.borrow();
    }

    #[test]
    #[should_panic(expected = "already borrowed")]
    fn b_borrow_mut_panics_while_a_reader_is_alive() {
        let cell = MyRefCell::new(1);
        let _reader = cell.borrow();
        let _writer = cell.borrow_mut();
    }

    #[test]
    fn b_guards_point_into_the_cell() {
        let mut cell = MyRefCell::new(7u64);
        let inside: *const u64 = cell.get_mut();
        assert!(
            std::ptr::eq(&*cell.borrow(), inside),
            "a Ref must point at the value inside the cell, not at a copy"
        );
        assert!(
            std::ptr::eq(&*cell.borrow_mut(), inside),
            "a RefMut must point at the value inside the cell, not at a copy"
        );
        *cell.borrow_mut() += 1;
        assert_eq!(*cell.get_mut(), 8);
    }

    #[test]
    fn b_a_panic_releases_the_borrow() {
        let cell = MyRefCell::new(0);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let mut writer = cell.borrow_mut();
            *writer = 1;
            // Unwinds like a panic, without printing a message.
            resume_unwind(Box::new("stop"));
        }));
        assert!(outcome.is_err());
        let reader = cell
            .try_borrow()
            .expect("a guard that is dropped by unwinding must release its borrow in Drop");
        assert_eq!(*reader, 1);
    }

    #[test]
    fn b_a_leaked_guard_keeps_the_cell_locked() {
        let mut cell = MyRefCell::new(5);
        std::mem::forget(cell.borrow_mut());
        assert!(
            cell.try_borrow().is_none(),
            "a leaked RefMut never ran its destructor, so the cell stays locked"
        );
        // `&mut self` still proves that no guard can be *used*.
        *cell.get_mut() += 1;
        assert_eq!(cell.into_inner(), 6);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Sizes in bytes on a 64-bit target: (Option<&u8>, Option<Cell<&u8>>).
        // A reference is never null, and `Option` can use the null value as `None`.
        // May it do the same when the reference is inside a `Cell`?
        let p1: (usize, usize) = (8, 16);
        assert_eq!(
            p1,
            (size_of::<Option<&u8>>(), size_of::<Option<Cell<&u8>>>()),
            "P1: niches through UnsafeCell (left: your prediction, right: actual)"
        );

        // P2. The standard RefCell: what do the two `is_ok()` calls return?
        let cell = RefCell::new(5);
        let reader = cell.borrow();
        let while_reading = cell.try_borrow_mut().is_ok();
        drop(reader);
        let after_reading = cell.try_borrow_mut().is_ok();
        let p2: (bool, bool) = (false, true);
        assert_eq!(
            p2,
            (while_reading, after_reading),
            "P2: run-time borrow state (left: your prediction, right: actual)"
        );

        // P3. The standard Cell with a value that is not Copy.
        let cell = Cell::new(vec![1, 2]);
        let taken = cell.take();
        cell.set(vec![9]);
        let old = cell.replace(Vec::new());
        let p3: (usize, usize, usize) = (2, 1, 0);
        assert_eq!(
            p3,
            (taken.len(), old.len(), cell.into_inner().len()),
            "P3: take, set and replace (left: your prediction, right: actual)"
        );
    }
}
