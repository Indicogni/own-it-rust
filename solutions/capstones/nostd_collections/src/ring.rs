//! `RingBuffer<T, N>`: a double-ended queue with a fixed capacity and
//! inline storage (SPEC section 5).

use core::fmt;
use core::iter::FusedIterator;
use core::mem::MaybeUninit;
use core::ops::Range;
use core::ptr;
use core::slice;

use crate::CapacityError;

/// A double-ended queue that stores at most `N` elements inside itself.
///
/// The elements occupy `len` slots of `buf` in a row, starting at slot
/// `head`. The row continues at slot 0 when it reaches the end of `buf`:
///
/// ```text
/// N = 8, head = 6, len = 5        front                 back
///                                   |                     |
/// slot      0   1   2   3   4   5   6   7                 |
///         +---+---+---+---+---+---+---+---+               |
/// buf     | c | d | e |   |   |   | a | b |   order: a b c d e
///         +---+---+---+---+---+---+---+---+
/// ```
///
/// # Invariant
///
/// `len <= N`. `head < N`, or `head == 0` when `N == 0`. The `len` slots
/// that start at `head` (wrapping at `N`) hold initialised values that this
/// buffer owns. No other slot is ever read as `T`.
///
/// Every `SAFETY` comment in this file refers to this invariant.
pub struct RingBuffer<T, const N: usize> {
    head: usize,
    len: usize,
    buf: [MaybeUninit<T>; N],
}

impl<T, const N: usize> RingBuffer<T, N> {
    /// Creates an empty buffer.
    pub const fn new() -> Self {
        Self {
            head: 0,
            len: 0,
            buf: [const { MaybeUninit::uninit() }; N],
        }
    }

    /// The largest number of elements the buffer can hold: `N`.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// The number of elements in the buffer.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// `true` if the buffer holds no element.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// `true` if the buffer holds `N` elements.
    pub const fn is_full(&self) -> bool {
        self.len == N
    }

    /// The slot that is `offset` places behind `head`. `offset` may be at
    /// most `N`.
    ///
    /// The obvious `(head + offset) % N` has two problems. It divides by
    /// zero when `N == 0`. And the sum can overflow when `T` is zero-sized,
    /// because `N` may then be as large as `usize::MAX`. Comparing `offset`
    /// with the room before the end of `buf` avoids both, and needs no
    /// division.
    const fn slot(&self, offset: usize) -> usize {
        let room = N - self.head;
        if offset < room {
            self.head + offset
        } else {
            offset - room
        }
    }

    /// The slots of the contents as two ranges: the part that starts at
    /// `head`, and the part that wrapped to the start of `buf` (empty if
    /// nothing wrapped).
    const fn ranges(&self) -> (Range<usize>, Range<usize>) {
        let room = N - self.head;
        if self.len <= room {
            (self.head..self.head + self.len, 0..0)
        } else {
            (self.head..N, 0..self.len - room)
        }
    }

    /// Appends `value` at the back, or gives it back if the buffer is full.
    pub fn push_back(&mut self, value: T) -> Result<(), CapacityError<T>> {
        if self.len == N {
            return Err(CapacityError::new(value));
        }
        // The slot behind the last element is free because `len < N`.
        let slot = self.slot(self.len);
        self.buf[slot].write(value);
        self.len += 1;
        Ok(())
    }

    /// Adds `value` at the front, or gives it back if the buffer is full.
    pub fn push_front(&mut self, value: T) -> Result<(), CapacityError<T>> {
        if self.len == N {
            return Err(CapacityError::new(value));
        }
        // `len < N` implies `N > 0`, so `N - 1` cannot underflow. The slot
        // before `head` is free for the same reason.
        let slot = if self.head == 0 { N - 1 } else { self.head - 1 };
        self.buf[slot].write(value);
        self.head = slot;
        self.len += 1;
        Ok(())
    }

    /// Appends `value` at the back. If the buffer is full, first removes the
    /// front element to make room, and returns it.
    ///
    /// A buffer with capacity 0 cannot keep anything: it returns `value`.
    pub fn push_overwrite(&mut self, value: T) -> Option<T> {
        if N == 0 {
            return Some(value);
        }
        let evicted = if self.len == N {
            self.pop_front()
        } else {
            None
        };
        // `N > 0`, and a full buffer has just given up its front element, so
        // there is room. Saying so with `unreachable!` is more honest than
        // code that pretends to handle a refusal.
        if self.push_back(value).is_err() {
            unreachable!("RingBuffer::push_overwrite: no room after making room");
        }
        evicted
    }

    /// Removes the front element and returns it.
    pub fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let slot = self.head;
        self.head = self.slot(1);
        self.len -= 1;
        // SAFETY: `slot` was the first of the `len` initialised slots.
        // `head` and `len` no longer cover it, so the value is moved out
        // once.
        Some(unsafe { self.buf[slot].assume_init_read() })
    }

    /// Removes the back element and returns it.
    pub fn pop_back(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let slot = self.slot(self.len);
        // SAFETY: `slot` was the last of the initialised slots. `len` no
        // longer covers it, so the value is moved out once.
        Some(unsafe { self.buf[slot].assume_init_read() })
    }

    /// The element `index` places behind the front, if there is one.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        // SAFETY: `index < len`, so this is one of the initialised slots.
        Some(unsafe { self.buf[self.slot(index)].assume_init_ref() })
    }

    /// Like [`RingBuffer::get`], but mutable.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        let slot = self.slot(index);
        // SAFETY: `index < len`, so this is one of the initialised slots.
        Some(unsafe { self.buf[slot].assume_init_mut() })
    }

    /// The front element: the one `pop_front` would return.
    pub fn front(&self) -> Option<&T> {
        self.get(0)
    }

    /// The back element: the one `pop_back` would return.
    pub fn back(&self) -> Option<&T> {
        self.get(self.len.checked_sub(1)?)
    }

    /// The contents as two slices. The first followed by the second is the
    /// contents from front to back. The second is empty unless the contents
    /// wrap around the end of the storage.
    pub fn as_slices(&self) -> (&[T], &[T]) {
        let (first, second) = self.ranges();
        // SAFETY: both ranges cover initialised slots only (invariant).
        unsafe {
            (
                self.buf[first].assume_init_ref(),
                self.buf[second].assume_init_ref(),
            )
        }
    }

    /// Like [`RingBuffer::as_slices`], but mutable.
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        let (first, second) = self.ranges();
        // `first` starts at `head` and `second` ends at or before `head`,
        // so splitting at `head` separates them without unsafe code.
        let (low, high) = self.buf.split_at_mut(first.start);
        let first = &mut high[..first.len()];
        let second = &mut low[second];
        // SAFETY: both slices cover initialised slots only (invariant).
        unsafe { (first.assume_init_mut(), second.assume_init_mut()) }
    }

    /// An iterator over the elements, from front to back.
    pub fn iter(&self) -> Iter<'_, T> {
        let (first, second) = self.as_slices();
        Iter {
            first: first.iter(),
            second: second.iter(),
        }
    }

    /// An iterator over mutable references to the elements, from front to
    /// back.
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        let (first, second) = self.as_mut_slices();
        IterMut {
            first: first.iter_mut(),
            second: second.iter_mut(),
        }
    }

    /// Drops every element, from front to back.
    pub fn clear(&mut self) {
        /// Drops a slice of elements when it goes out of scope, also during
        /// unwinding. Whoever builds one promises that the pointer covers
        /// initialised elements that nothing else reads or drops afterwards.
        struct DropSlice<T>(*mut [T]);

        impl<T> Drop for DropSlice<T> {
            fn drop(&mut self) {
                // SAFETY: the promise made when the guard was built. It is
                // built in one place, a few lines below, where the buffer
                // has just given the elements up.
                unsafe { ptr::drop_in_place(self.0) };
            }
        }

        let (first, second) = self.as_mut_slices();
        let (first, second): (*mut [T], *mut [T]) = (first, second);
        // Give every element up *before* any destructor runs. If one
        // panics, the buffer is empty and valid, and its own `Drop` cannot
        // drop an element a second time.
        self.head = 0;
        self.len = 0;
        // The guard drops the second slice even if a destructor in the
        // first one panics.
        let _second = DropSlice(second);
        // SAFETY: both pointers cover slots that were initialised and that
        // the buffer no longer claims. The writes to `head` and `len` above
        // do not touch `buf`, so the pointers are still valid. Each element
        // is dropped once: `first` here, `second` by the guard.
        unsafe { ptr::drop_in_place(first) };
    }
}

impl<T, const N: usize> Drop for RingBuffer<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone, const N: usize> Clone for RingBuffer<T, N> {
    /// The clone holds equal elements in the same order. Where they sit in
    /// its storage is not copied: the clone starts at slot 0.
    fn clone(&self) -> Self {
        let mut copy = Self::new();
        for item in self {
            // The copy has the same capacity, so there is room. If
            // `T::clone` panics, `copy` is dropped while unwinding and drops
            // the clones made so far.
            let pushed = copy.push_back(item.clone());
            debug_assert!(pushed.is_ok());
        }
        copy
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for RingBuffer<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self).finish()
    }
}

impl<T: PartialEq, const N: usize> PartialEq for RingBuffer<T, N> {
    /// Two buffers are equal when they hold equal elements in the same
    /// order. Where the elements sit in the storage does not matter.
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.iter().eq(other)
    }
}

impl<T: Eq, const N: usize> Eq for RingBuffer<T, N> {}

impl<'a, T, const N: usize> IntoIterator for &'a RingBuffer<T, N> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut RingBuffer<T, N> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<T, const N: usize> IntoIterator for RingBuffer<T, N> {
    type Item = T;
    type IntoIter = IntoIter<T, N>;

    fn into_iter(self) -> IntoIter<T, N> {
        IntoIter { ring: self }
    }
}

/// The iterator of [`RingBuffer::iter`].
///
/// It is two slice iterators, one after the other. `core::iter::Chain`
/// would do the same, but `Chain` is not an `ExactSizeIterator`, because the
/// sum of two lengths can overflow in general. Here it cannot: both slices
/// lie in one buffer.
pub struct Iter<'a, T> {
    first: slice::Iter<'a, T>,
    second: slice::Iter<'a, T>,
}

// Written by hand: `#[derive(Clone)]` would add the bound `T: Clone`, but
// copying two slice iterators needs no bound.
impl<T> Clone for Iter<'_, T> {
    fn clone(&self) -> Self {
        Self {
            first: self.first.clone(),
            second: self.second.clone(),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        self.first.next().or_else(|| self.second.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.first.len() + self.second.len();
        (left, Some(left))
    }
}

impl<T> DoubleEndedIterator for Iter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.second.next_back().or_else(|| self.first.next_back())
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {}
impl<T> FusedIterator for Iter<'_, T> {}

impl<T: fmt::Debug> fmt::Debug for Iter<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// The iterator of [`RingBuffer::iter_mut`].
pub struct IterMut<'a, T> {
    first: slice::IterMut<'a, T>,
    second: slice::IterMut<'a, T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        self.first.next().or_else(|| self.second.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.first.len() + self.second.len();
        (left, Some(left))
    }
}

impl<T> DoubleEndedIterator for IterMut<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.second.next_back().or_else(|| self.first.next_back())
    }
}

impl<T> ExactSizeIterator for IterMut<'_, T> {}
impl<T> FusedIterator for IterMut<'_, T> {}

impl<T: fmt::Debug> fmt::Debug for IterMut<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.first.as_slice())
            .entries(self.second.as_slice())
            .finish()
    }
}

/// The iterator of `RingBuffer::into_iter`: yields the elements by value.
///
/// It owns the buffer and pops from it, so it needs no unsafe code and no
/// `Drop` of its own: the buffer drops what is left.
pub struct IntoIter<T, const N: usize> {
    ring: RingBuffer<T, N>,
}

impl<T, const N: usize> Iterator for IntoIter<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.ring.pop_front()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.ring.len, Some(self.ring.len))
    }
}

impl<T, const N: usize> DoubleEndedIterator for IntoIter<T, N> {
    fn next_back(&mut self) -> Option<T> {
        self.ring.pop_back()
    }
}

impl<T, const N: usize> ExactSizeIterator for IntoIter<T, N> {}
impl<T, const N: usize> FusedIterator for IntoIter<T, N> {}

impl<T: fmt::Debug, const N: usize> fmt::Debug for IntoIter<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter").field(&self.ring).finish()
    }
}
