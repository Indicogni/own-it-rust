//! LESSON 08 — Capstone: initialized-prefix storage
//! Run: cargo test --test chapter23 l08_
//! Read first: "Lesson 08" in CONCEPTS.md (same directory).
//!
//! PREDICT before running (this lesson has no `p_predictions` test; write
//! your answers in NOTES.md):
//! 1. May a `MaybeUninit<T>` go out of scope before it contains a `T`?
//! 2. Why must `len` change before `pop` returns the value it moved out?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Write the invariant (`len <= N`, and exactly
//! `data[..len]` is initialized). Then audit `push`, `pop`, and the supplied
//! `Drop` for a normal return and for a panic.
//!
//! COMPILER / TRANSFER LAB (optional): In the scratch example, call a closure
//! that panics BEFORE a push, catch the panic with `catch_unwind`, and check
//! that every stored value is still owned exactly once. Never put a panic
//! between a raw move and the update of `len`.
//! Use examples/ch23_scratch.rs for this, and leave it compiling afterward.
//!
//! If a test run of this lesson ends with "signal: 6, SIGABRT", "SIGSEGV" or
//! "pointer being freed was not allocated" instead of a test summary, a value
//! was dropped twice. Check that `pop` decrements `len`.

use std::mem::MaybeUninit;
struct Slots<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: usize,
}
impl<T, const N: usize> Slots<T, N> {
    fn new() -> Self {
        Self {
            data: [const { MaybeUninit::uninit() }; N],
            len: 0,
        }
    }
    /// A. Push into the next slot or return the unchanged value if full.
    /// Write before incrementing len, and run no user callback between them.
    fn push(&mut self, value: T) -> Result<(), T> {
        if self.len == N {
            return Err(value);
        }
        // Write first, count second. `MaybeUninit::write` cannot panic and
        // runs no user code, so the invariant holds again on the next line.
        self.data[self.len].write(value);
        self.len += 1;
        Ok(())
    }
    /// B. Decrement len, then assume_init_read the former last initialized slot.
    /// Empty returns None. No clones or default placeholders.
    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // SAFETY: slot `len` (after the decrement) was the last slot of the
        // initialized prefix. `len` no longer counts it, so neither `Drop`
        // nor `as_slice` will touch it again: the caller is its only owner.
        Some(unsafe { self.data[self.len].assume_init_read() })
    }
    /// C. Borrow only the initialized prefix as a slice. Never include unused
    /// slots, even when they happen to hold old representation bytes.
    fn as_slice(&self) -> &[T] {
        // SAFETY: `MaybeUninit<T>` has the layout of `T`, so the cast pointer is
        // aligned for `T`. Exactly `data[..len]` is initialized (invariant) and
        // `len <= N`, so the range is in bounds. The result borrows `self`, so
        // no `&mut self` method can run while the slice exists.
        unsafe { std::slice::from_raw_parts(self.data.as_ptr().cast::<T>(), self.len) }
    }
}
// The cleanup is supplied, so that an unfinished task cannot cause a second
// `todo!()` panic while a failing test unwinds. Keep this implementation.
impl<T, const N: usize> Drop for Slots<T, N> {
    fn drop(&mut self) {
        while self.len > 0 {
            self.len -= 1;
            // SAFETY: the invariant says that this slot of the former prefix
            // is initialized. `len` no longer counts it, so nothing else will
            // read or drop it after this call.
            unsafe {
                self.data[self.len].assume_init_drop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_b_c_capacity_and_reuse_preserve_sequence() {
        let mut s = Slots::<String, 2>::new();
        s.push("a".into()).unwrap();
        s.push("b".into()).unwrap();
        assert_eq!(s.push("c".into()), Err("c".into()));
        assert_eq!(s.as_slice(), ["a", "b"]);
        assert_eq!(s.pop().as_deref(), Some("b"));
        s.push("d".into()).unwrap();
        assert_eq!(s.as_slice(), ["a", "d"]);
        assert_eq!(s.pop().as_deref(), Some("d"));
        assert_eq!(s.pop().as_deref(), Some("a"));
        assert!(s.pop().is_none());
        let mut zero = Slots::<u8, 0>::new();
        assert_eq!(zero.push(7), Err(7));
        assert!(zero.as_slice().is_empty());
    }
    #[test]
    fn a_b_every_owned_value_drops_once() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        #[derive(Debug)]
        struct Spy(Arc<AtomicUsize>);
        impl Drop for Spy {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }
        let count = Arc::new(AtomicUsize::new(0));
        let mut s = Slots::<Spy, 3>::new();
        s.push(Spy(Arc::clone(&count))).unwrap();
        s.push(Spy(Arc::clone(&count))).unwrap();
        let out = s.pop().unwrap();
        assert_eq!(
            count.load(Ordering::Relaxed),
            0,
            "drops so far: push and pop move values; they drop nothing"
        );
        drop(s);
        assert_eq!(
            count.load(Ordering::Relaxed),
            1,
            "drops after dropping the container: only the value still inside it"
        );
        drop(out);
        assert_eq!(
            count.load(Ordering::Relaxed),
            2,
            "drops in total: each of the two values exactly once"
        );
    }
    #[test]
    fn a_b_values_move_in_and_out_without_copies() {
        let mut s = Slots::<String, 2>::new();
        let a = String::from("alpha");
        let pa = a.as_ptr();
        let (pushed, allocations) = count(|| s.push(a));
        assert_eq!(
            (pushed, allocations),
            (Ok(()), 0),
            "(result, allocations): push moves the String into its slot"
        );
        s.push("beta".into()).unwrap();
        let rejected = String::from("gamma");
        let pr = rejected.as_ptr();
        let back = s.push(rejected).unwrap_err();
        assert_eq!(back.as_ptr(), pr, "a rejected value comes back unchanged");
        assert_eq!(s.pop().as_deref(), Some("beta"));
        let (out, allocations) = count(|| s.pop());
        assert_eq!(allocations, 0, "pop moves the value out");
        let out = out.unwrap();
        assert_eq!(
            out.as_ptr(),
            pa,
            "pop must return the String that was pushed, with its original buffer"
        );
    }
    #[test]
    fn c_slice_borrows_the_initialized_prefix() {
        let mut s = Slots::<u32, 4>::new();
        assert!(s.as_slice().is_empty());
        s.push(10).unwrap();
        s.push(20).unwrap();
        s.push(30).unwrap();
        assert_eq!(s.pop(), Some(30));
        let (len, allocations) = count(|| s.as_slice().len());
        assert_eq!(
            (len, allocations),
            (2, 0),
            "(length, allocations): as_slice borrows the prefix; it never copies"
        );
        assert_eq!(s.as_slice(), [10, 20], "the popped slot is not included");
        assert_eq!(
            s.as_slice().as_ptr(),
            s.data.as_ptr().cast::<u32>(),
            "the slice must point at the container's own storage"
        );
    }
}
