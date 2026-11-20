//! MIRI LAB 04 — A drain that trusts its destructor (lesson 04)
//!
//! Run it under Miri, from the repository root:
//!
//!   cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l04_drain
//!
//! 1. PREDICT. Read the program. Every `unsafe` block has a SAFETY comment.
//!    Decide which comment is false or leaves something out, and what Miri
//!    will report, before you run anything.
//! 2. RUN the command above. Read the first `error:` line and the source line
//!    that it points to.
//! 3. COMPARE with "What Miri says about the wrong version" in CONCEPTS.md,
//!    lesson 04.
//! 4. REPAIR the program. Do not change `main`. Correct the SAFETY comments
//!    too. After a correct repair Miri no longer reports undefined behaviour.
//!    It reports a memory leak instead: the forgotten iterator still owned
//!    two strings. A leak is safe, and it is the price of leak safety here.
//!    Confirm that nothing else is wrong by running the command again with
//!    `MIRIFLAGS=-Zmiri-ignore-leaks` in front of it.
//!
//! A native `cargo run --example miri_l04_drain` may print the expected result, print
//! something else, or crash. That is what undefined behaviour means: only the
//! Miri run tells you whether the program is right.

/// Moves the first `count` elements out of a vector, one by one. When it is
/// dropped, it removes those slots from the vector.
struct TakeFront<'a, T> {
    vec: &'a mut Vec<T>,
    /// How many of the `count` elements have been returned so far.
    taken: usize,
    count: usize,
    /// The length of the vector before `take_front` shortened it.
    old_len: usize,
}

fn take_front<T>(vec: &mut Vec<T>, count: usize) -> TakeFront<'_, T> {
    assert!(
        count <= vec.len(),
        "cannot take more elements than there are"
    );
    // FIX: `mem::forget` is safe, so the destructor below may never run. The
    // vector must already be in a state that is valid without it. From here
    // on it claims no element at all; the destructor gives the tail back.
    // (A leaked `TakeFront` now leaks the elements instead of dropping one of
    // them twice.) The old length is kept in `TakeFront::old_len`.
    let old_len = vec.len();
    // SAFETY: 0 is at most the capacity, and a length of 0 claims no element.
    unsafe { vec.set_len(0) };
    TakeFront {
        vec,
        taken: 0,
        count,
        old_len,
    }
}

impl<T> Iterator for TakeFront<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.taken == self.count {
            return None;
        }
        // SAFETY: `taken < count <= old_len`, so the slot is inside the
        // vector's block and initialized. The vector's length is 0, so the
        // vector does not count the slot, and `taken` moves on, so the copy
        // made by `read` is the only owner.
        let value = unsafe { self.vec.as_ptr().add(self.taken).read() };
        self.taken += 1;
        Some(value)
    }
}

impl<T> Drop for TakeFront<'_, T> {
    fn drop(&mut self) {
        let len = self.old_len;
        // SAFETY: the vector's length is 0, so this destructor owns every
        // element in `taken..len`. Slots `taken..count` hold the elements that
        // were not returned; they are dropped here. The `len - count` elements
        // after them are then moved to the front (the ranges may overlap, so
        // `copy`), and the new length covers exactly those elements.
        unsafe {
            let base = self.vec.as_mut_ptr();
            std::ptr::slice_from_raw_parts_mut(base.add(self.taken), self.count - self.taken)
                .drop_in_place();
            std::ptr::copy(base.add(self.count), base, len - self.count);
            self.vec.set_len(len - self.count);
        }
    }
}

fn main() {
    let mut words = vec![String::from("a"), String::from("b"), String::from("c")];
    let mut front = take_front(&mut words, 2);
    let first = front.next();
    // Forgetting a value is safe: no `unsafe` is needed for this line.
    std::mem::forget(front);
    println!(
        "took {first:?}, the vector now holds {} element(s)",
        words.len()
    );
    drop(first);
    drop(words);
}
