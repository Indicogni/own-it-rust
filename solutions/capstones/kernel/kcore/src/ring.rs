//! A fixed-capacity FIFO queue that never allocates.

use core::mem::MaybeUninit;

/// A first-in, first-out queue that holds at most `N` values.
///
/// The storage is an inline array, so the queue works before a heap exists
/// and inside interrupt handlers.
///
/// Invariant: exactly the `len` slots starting at `head` (wrapping at `N`)
/// hold initialised values; every other slot is uninitialised.
pub struct RingBuffer<T, const N: usize> {
    slots: [MaybeUninit<T>; N],
    head: usize,
    len: usize,
}

impl<T, const N: usize> RingBuffer<T, N> {
    /// Creates an empty queue. Usable in a `static`.
    pub const fn new() -> Self {
        Self {
            slots: [const { MaybeUninit::uninit() }; N],
            head: 0,
            len: 0,
        }
    }

    /// The largest number of values the queue can hold.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// The number of values in the queue now.
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn is_full(&self) -> bool {
        self.len == N
    }

    /// Adds `value` at the back. A full queue hands the value back unchanged.
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.len == N {
            return Err(value);
        }
        // `len < N` here, so `N > 0` and the remainder is defined.
        let tail = (self.head + self.len) % N;
        self.slots[tail].write(value);
        self.len += 1;
        Ok(())
    }

    /// Removes and returns the oldest value.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        // SAFETY: `len > 0`, so by the type invariant the slot at `head` is
        // initialised. Moving `head` forward right after the read takes the
        // slot out of the initialised range, so the value is never read twice.
        let value = unsafe { self.slots[self.head].assume_init_read() };
        self.head = (self.head + 1) % N;
        self.len -= 1;
        Some(value)
    }
}

impl<T, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Drop for RingBuffer<T, N> {
    fn drop(&mut self) {
        // `MaybeUninit` never drops its content, so the values that are still
        // queued must be dropped by hand. `pop` does exactly that.
        while self.pop().is_some() {}
    }
}
