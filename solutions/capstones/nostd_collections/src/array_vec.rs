//! `ArrayVec<T, N>`: a vector with a fixed capacity and inline storage
//! (SPEC section 4).

use core::fmt;
use core::iter::FusedIterator;
use core::mem::{self, MaybeUninit};
use core::ops::{Bound, Deref, DerefMut, RangeBounds};
use core::ptr;

use crate::CapacityError;

/// A vector that stores at most `N` elements inside itself.
///
/// # Invariant
///
/// `len <= N`, and the slots `buf[..len]` hold initialised values that this
/// vector owns. The slots `buf[len..]` are never read as `T`: they are
/// uninitialised, or they hold the bytes of values that were moved out.
///
/// Every `SAFETY` comment in this file refers to this invariant.
pub struct ArrayVec<T, const N: usize> {
    len: usize,
    buf: [MaybeUninit<T>; N],
}

impl<T, const N: usize> ArrayVec<T, N> {
    /// Creates an empty vector.
    pub const fn new() -> Self {
        Self {
            len: 0,
            // An inline `const` block is evaluated once per array element,
            // so it also works when `T` is not `Copy`.
            buf: [const { MaybeUninit::uninit() }; N],
        }
    }

    /// The largest number of elements the vector can hold: `N`.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// The number of elements in the vector.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// `true` if the vector holds no element.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// `true` if the vector holds `N` elements.
    pub const fn is_full(&self) -> bool {
        self.len == N
    }

    /// The elements as a slice.
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: the slots `..len` are initialised (invariant), and they
        // cannot change while `self` is borrowed.
        unsafe { self.buf[..self.len].assume_init_ref() }
    }

    /// The elements as a mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: the slots `..len` are initialised (invariant). A caller
        // can replace elements through the slice but cannot uninitialise
        // one, so the invariant still holds afterwards.
        unsafe { self.buf[..self.len].assume_init_mut() }
    }

    /// Appends `value`.
    ///
    /// # Panics
    ///
    /// Panics if the vector is full.
    #[track_caller]
    pub fn push(&mut self, value: T) {
        if self.try_push(value).is_err() {
            panic!("ArrayVec::push: the capacity ({N}) is used up");
        }
    }

    /// Appends `value`, or gives it back if the vector is full.
    pub fn try_push(&mut self, value: T) -> Result<(), CapacityError<T>> {
        if self.len == N {
            return Err(CapacityError::new(value));
        }
        // `MaybeUninit::write` does not drop the old content of the slot,
        // which is right: slot `len` holds no value.
        self.buf[self.len].write(value);
        self.len += 1;
        Ok(())
    }

    /// Removes the last element and returns it.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // SAFETY: this slot was the last initialised one. `len` is already
        // lowered, so the vector no longer owns the value and it is moved
        // out exactly once.
        Some(unsafe { self.buf[self.len].assume_init_read() })
    }

    /// Inserts `value` at `index` and moves the later elements up by one.
    ///
    /// # Panics
    ///
    /// Panics if `index > len`, or if the vector is full.
    #[track_caller]
    pub fn insert(&mut self, index: usize, value: T) {
        if self.try_insert(index, value).is_err() {
            panic!("ArrayVec::insert: the capacity ({N}) is used up");
        }
    }

    /// Inserts `value` at `index`, or gives it back if the vector is full.
    ///
    /// # Panics
    ///
    /// Panics if `index > len`. The index is checked first, so this is true
    /// for a full vector as well.
    #[track_caller]
    pub fn try_insert(&mut self, index: usize, value: T) -> Result<(), CapacityError<T>> {
        let len = self.len;
        assert!(
            index <= len,
            "insertion index {index} is out of bounds (the length is {len})"
        );
        if len == N {
            return Err(CapacityError::new(value));
        }
        let base = self.buf.as_mut_ptr();
        // SAFETY: `index <= len < N`, so the source slots `index..len` and
        // the destination slots `index + 1..len + 1` are inside `buf`.
        // `ptr::copy` allows the ranges to overlap, and moving the bytes of
        // `MaybeUninit` slots is valid whatever they hold.
        unsafe { ptr::copy(base.add(index), base.add(index + 1), len - index) };
        // Slot `index` now holds a stale bitwise copy of its old value (now
        // owned by slot `index + 1`); `write` replaces it without a drop.
        // Nothing can panic between the copy and this line.
        self.buf[index].write(value);
        self.len = len + 1;
        Ok(())
    }

    /// Removes the element at `index` and moves the later elements down by
    /// one.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`.
    #[track_caller]
    pub fn remove(&mut self, index: usize) -> T {
        let len = self.len;
        assert!(
            index < len,
            "removal index {index} is out of bounds (the length is {len})"
        );
        // SAFETY: slot `index` is initialised because `index < len`. Its
        // value is moved out here and the slot is overwritten or given up
        // below, so the value is owned once.
        let value = unsafe { self.buf[index].assume_init_read() };
        let base = self.buf.as_mut_ptr();
        // SAFETY: the source slots `index + 1..len` and the destination
        // slots `index..len - 1` are inside `buf`; overlap is allowed.
        unsafe { ptr::copy(base.add(index + 1), base.add(index), len - index - 1) };
        self.len = len - 1;
        value
    }

    /// Removes the element at `index` and puts the last element in its
    /// place. Does not keep the order, but moves only one element.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`.
    #[track_caller]
    pub fn swap_remove(&mut self, index: usize) -> T {
        let len = self.len;
        assert!(
            index < len,
            "swap_remove index {index} is out of bounds (the length is {len})"
        );
        let last = len - 1;
        // Swapping two slots needs no unsafe code: a `MaybeUninit<T>` can be
        // moved whether it holds a value or not.
        self.buf.swap(index, last);
        self.len = last;
        // SAFETY: slot `last` holds the value that was at `index`, which
        // was initialised. `len` is already lowered, so it is moved out once.
        unsafe { self.buf[last].assume_init_read() }
    }

    /// Drops the elements from index `len` on. Does nothing if `len` is not
    /// smaller than the current length.
    pub fn truncate(&mut self, len: usize) {
        if len >= self.len {
            return;
        }
        let tail: *mut [T] = &raw mut self.as_mut_slice()[len..];
        // Give the tail up *before* any destructor runs. If a destructor
        // panics, the vector does not claim a dropped element, and its own
        // `Drop` cannot drop one a second time.
        self.len = len;
        // SAFETY: `tail` covers slots that were initialised and that the
        // vector no longer claims. `drop_in_place` on a slice drops every
        // element once; if one destructor panics, it still drops the rest
        // while unwinding.
        unsafe { ptr::drop_in_place(tail) };
    }

    /// Drops every element.
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Keeps the elements for which `keep` returns `true`, in their order,
    /// and drops the others.
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut keep: F) {
        /// Restores the invariant when `retain` ends, normally or by a panic
        /// of `keep` or of a destructor.
        ///
        /// While it lives: slots `..processed - deleted` hold the kept
        /// elements, slots `processed..original_len` hold the elements not
        /// yet looked at, and the slots between are holes.
        struct Guard<'a, T, const N: usize> {
            vec: &'a mut ArrayVec<T, N>,
            processed: usize,
            deleted: usize,
            original_len: usize,
        }

        impl<T, const N: usize> Drop for Guard<'_, T, N> {
            fn drop(&mut self) {
                let base = self.vec.buf.as_mut_ptr();
                let unseen = self.original_len - self.processed;
                // SAFETY: both ranges are inside `buf` because
                // `deleted <= processed <= original_len <= N`. This closes
                // the holes: the unseen elements move down next to the kept
                // ones. After a normal run `unseen` is 0.
                unsafe {
                    ptr::copy(
                        base.add(self.processed),
                        base.add(self.processed - self.deleted),
                        unseen,
                    );
                }
                self.vec.len = self.original_len - self.deleted;
            }
        }

        let original_len = self.len;
        // While the guard lives the vector claims nothing. If the guard were
        // leaked, elements would leak, but none could be dropped twice.
        self.len = 0;
        let mut guard = Guard {
            vec: self,
            processed: 0,
            deleted: 0,
            original_len,
        };

        while guard.processed < original_len {
            let slot = guard.processed;
            // SAFETY: slot `processed` is one of the unseen elements, so it
            // is initialised.
            let wanted = keep(unsafe { guard.vec.buf[slot].assume_init_ref() });
            // Count the element as handled before its destructor can run: a
            // panicking destructor must not leave it among the "unseen".
            guard.processed += 1;
            if wanted {
                // Moves the element down over the holes. Swapping the two
                // slots is safe code; what lands in slot `slot` is a hole.
                guard.vec.buf.swap(slot - guard.deleted, slot);
            } else {
                guard.deleted += 1;
                // SAFETY: the slot is initialised, and it was just counted
                // as deleted, so nothing reads or drops it again.
                unsafe { guard.vec.buf[slot].assume_init_drop() };
            }
        }
    }

    /// Removes the elements in `range` and returns them as an iterator.
    ///
    /// When the iterator is dropped, it drops the elements it did not yield
    /// and closes the gap. If it is leaked instead (`core::mem::forget`),
    /// the vector keeps only the elements before `range`; the others are
    /// leaked.
    ///
    /// # Panics
    ///
    /// Panics if the start of the range is greater than its end, or if the
    /// end is greater than the length.
    #[track_caller]
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, T, N> {
        let len = self.len;
        let start = match range.start_bound() {
            Bound::Included(&start) => Some(start),
            Bound::Excluded(&start) => start.checked_add(1),
            Bound::Unbounded => Some(0),
        };
        let end = match range.end_bound() {
            Bound::Included(&end) => end.checked_add(1),
            Bound::Excluded(&end) => Some(end),
            Bound::Unbounded => Some(len),
        };
        let (start, end) = match (start, end) {
            (Some(start), Some(end)) if start <= end && end <= len => (start, end),
            _ => panic!("the drain range is out of bounds (the length is {len})"),
        };
        // Leak amplification: from now on the vector claims only the
        // elements before the range. The iterator owns the range and is
        // responsible for the tail. If it is leaked, the tail is lost, which
        // is safe; no element can be reached twice.
        self.len = start;
        Drain {
            vec: self,
            front: start,
            back: end,
            tail_start: end,
            tail_len: len - end,
        }
    }

    /// Returns the elements as an array if the vector is full, and the
    /// unchanged vector otherwise.
    pub fn into_inner(mut self) -> Result<[T; N], Self> {
        if self.len < N {
            return Err(self);
        }
        // Take the storage and leave an empty vector behind for `Drop`.
        self.len = 0;
        let buf = mem::replace(&mut self.buf, [const { MaybeUninit::uninit() }; N]);
        // SAFETY: the vector was full, so all `N` slots are initialised, and
        // `[MaybeUninit<T>; N]` has the same size and layout as `[T; N]`.
        // `self` gave the elements up above, so they are owned once.
        Ok(unsafe { mem::transmute_copy::<[MaybeUninit<T>; N], [T; N]>(&buf) })
    }
}

impl<T, const N: usize> Drop for ArrayVec<T, N> {
    fn drop(&mut self) {
        // Drops the elements in index order.
        self.clear();
    }
}

impl<T, const N: usize> Default for ArrayVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Deref for ArrayVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> DerefMut for ArrayVec<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: Clone, const N: usize> Clone for ArrayVec<T, N> {
    fn clone(&self) -> Self {
        let mut copy = Self::new();
        for item in self {
            // If `T::clone` panics, `copy` is dropped while unwinding and
            // drops the clones made so far.
            copy.push(item.clone());
        }
        copy
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for ArrayVec<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_slice(), f)
    }
}

impl<T: PartialEq, const N: usize> PartialEq for ArrayVec<T, N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq, const N: usize> Eq for ArrayVec<T, N> {}

impl<T, const N: usize> From<[T; N]> for ArrayVec<T, N> {
    /// Builds a full vector from an array.
    fn from(array: [T; N]) -> Self {
        Self {
            len: N,
            buf: array.map(MaybeUninit::new),
        }
    }
}

impl<T, const N: usize> Extend<T> for ArrayVec<T, N> {
    /// Appends every item of `iter`.
    ///
    /// # Panics
    ///
    /// Panics when an item does not fit. The items before it stay in the
    /// vector.
    #[track_caller]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

impl<T, const N: usize> FromIterator<T> for ArrayVec<T, N> {
    /// Collects an iterator.
    ///
    /// # Panics
    ///
    /// Panics if the iterator yields more than `N` items.
    #[track_caller]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut vec = Self::new();
        vec.extend(iter);
        vec
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a ArrayVec<T, N> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut ArrayVec<T, N> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<T, const N: usize> IntoIterator for ArrayVec<T, N> {
    type Item = T;
    type IntoIter = IntoIter<T, N>;

    fn into_iter(mut self) -> IntoIter<T, N> {
        // Take the storage and leave an empty vector behind for `Drop`. This
        // needs no unsafe code; the compiler turns it into one copy of the
        // storage.
        let back = mem::take(&mut self.len);
        let buf = mem::replace(&mut self.buf, [const { MaybeUninit::uninit() }; N]);
        IntoIter {
            buf,
            front: 0,
            back,
        }
    }
}

/// The iterator of `ArrayVec::into_iter`: yields the elements by value.
///
/// # Invariant
///
/// `front <= back <= N`, and the slots `buf[front..back]` hold initialised
/// values that this iterator owns.
pub struct IntoIter<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    front: usize,
    back: usize,
}

impl<T, const N: usize> IntoIter<T, N> {
    /// The elements not yet yielded.
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: the slots `front..back` are initialised (invariant).
        unsafe { self.buf[self.front..self.back].assume_init_ref() }
    }
}

impl<T, const N: usize> Iterator for IntoIter<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.front == self.back {
            return None;
        }
        let slot = self.front;
        self.front += 1;
        // SAFETY: `slot` was inside `front..back`, so it is initialised.
        // `front` has moved past it, so the value is moved out once.
        Some(unsafe { self.buf[slot].assume_init_read() })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.back - self.front;
        (left, Some(left))
    }
}

impl<T, const N: usize> DoubleEndedIterator for IntoIter<T, N> {
    fn next_back(&mut self) -> Option<T> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        // SAFETY: slot `back` was the last one inside `front..back`, so it
        // is initialised, and it is now outside the range.
        Some(unsafe { self.buf[self.back].assume_init_read() })
    }
}

impl<T, const N: usize> ExactSizeIterator for IntoIter<T, N> {}
impl<T, const N: usize> FusedIterator for IntoIter<T, N> {}

impl<T, const N: usize> Drop for IntoIter<T, N> {
    fn drop(&mut self) {
        // SAFETY: the slots `front..back` are initialised and owned
        // (invariant), and nothing uses the iterator after `drop`.
        unsafe { self.buf[self.front..self.back].assume_init_drop() };
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for IntoIter<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}

/// The iterator of [`ArrayVec::drain`].
///
/// # Invariant
///
/// `vec.len <= front <= back <= tail_start`, and
/// `tail_start + tail_len <= N`. The slots `front..back` hold the elements
/// not yet yielded, and the slots `tail_start..tail_start + tail_len` hold
/// the elements behind the drained range. This iterator owns both groups;
/// the vector claims only the slots `..vec.len`.
pub struct Drain<'a, T, const N: usize> {
    vec: &'a mut ArrayVec<T, N>,
    front: usize,
    back: usize,
    tail_start: usize,
    tail_len: usize,
}

impl<T, const N: usize> Drain<'_, T, N> {
    /// The elements not yet yielded.
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: the slots `front..back` are initialised (invariant).
        unsafe { self.vec.buf[self.front..self.back].assume_init_ref() }
    }
}

impl<T, const N: usize> Iterator for Drain<'_, T, N> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.front == self.back {
            return None;
        }
        let slot = self.front;
        self.front += 1;
        // SAFETY: `slot` was inside `front..back`, so it is initialised and
        // owned by this iterator. `front` has moved past it.
        Some(unsafe { self.vec.buf[slot].assume_init_read() })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.back - self.front;
        (left, Some(left))
    }
}

impl<T, const N: usize> DoubleEndedIterator for Drain<'_, T, N> {
    fn next_back(&mut self) -> Option<T> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        // SAFETY: slot `back` was the last one inside `front..back`, so it
        // is initialised, and it is now outside the range.
        Some(unsafe { self.vec.buf[self.back].assume_init_read() })
    }
}

impl<T, const N: usize> ExactSizeIterator for Drain<'_, T, N> {}
impl<T, const N: usize> FusedIterator for Drain<'_, T, N> {}

impl<T, const N: usize> Drop for Drain<'_, T, N> {
    fn drop(&mut self) {
        /// Moves the tail down to the end of the vector and gives it back.
        /// A guard, so that it also happens when a destructor panics below.
        struct MoveTail<'r, 'a, T, const N: usize>(&'r mut Drain<'a, T, N>);

        impl<T, const N: usize> Drop for MoveTail<'_, '_, T, N> {
            fn drop(&mut self) {
                let drain = &mut *self.0;
                let start = drain.vec.len;
                let base = drain.vec.buf.as_mut_ptr();
                // SAFETY: the source slots hold the tail and the destination
                // starts at `start <= tail_start`, so both ranges are inside
                // `buf` (invariant). Overlap is allowed.
                unsafe { ptr::copy(base.add(drain.tail_start), base.add(start), drain.tail_len) };
                drain.vec.len = start + drain.tail_len;
            }
        }

        let (front, back) = (self.front, self.back);
        // Give the range up first: after this line nothing reads it as `T`
        // except the `assume_init_drop` below.
        self.front = back;
        let guard = MoveTail(self);
        // SAFETY: the slots `front..back` held the elements not yet yielded
        // (invariant). They are dropped once: the iterator's range is empty
        // now, and the vector does not claim them.
        unsafe { guard.0.vec.buf[front..back].assume_init_drop() };
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for Drain<'_, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.as_slice()).finish()
    }
}
