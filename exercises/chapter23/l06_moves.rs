//! LESSON 06 — Moving values and initialization
//! Run: cargo test --test chapter23 l06_
//! Read first: "Lesson 06" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. After `ptr::read`, does the compiler still treat the source as an owner?
//! 2. What happens if a `String` that was moved out with `ptr::read` is also
//!    dropped in its old place?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain why no code that can panic may run between a
//! raw move and the update of the length that records which slots are
//! initialized.
//!
//! COMPILER / TRANSFER LAB (optional): On paper, write an ownership ledger for
//! `raw_pop`: one line per statement, saying who owns the last element after
//! it. Do not run a double-drop experiment. Then read the source of `Vec::pop`
//! in the std documentation and compare it with your version.
//! Use examples/ch23_scratch.rs for this, and leave it compiling afterward.
//!
//! If a test run of this lesson ends with "signal: 6, SIGABRT", "SIGSEGV" or
//! "pointer being freed was not allocated" instead of a test summary, a value
//! was dropped twice. Check how task A updates the length.

use std::mem::MaybeUninit;
/// A. Implement pop without Vec::pop: reduce the vector length and ptr::read
/// its former last initialized element. Empty returns None. Never expose a
/// reference to the now-uninitialized former slot or run fallible code mid-move.
// The `&mut Vec<T>` parameter is part of the contract: you need `set_len`.
#[allow(clippy::ptr_arg)]
fn raw_pop<T>(values: &mut Vec<T>) -> Option<T> {
    todo!("A: implement the contract")
}
/// B. Move two owned Strings into MaybeUninit slots, then return a fully
/// initialized array. No default placeholders, clones, or raw byte copies.
fn initialized_pair(first: String, second: String) -> [String; 2] {
    todo!("B: implement the contract")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, guess};
    use std::cell::RefCell;
    #[test]
    fn a_moved_values_drop_once() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        struct Spy(Arc<AtomicUsize>);
        impl Drop for Spy {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }
        let count = Arc::new(AtomicUsize::new(0));
        let mut v = vec![Spy(Arc::clone(&count)), Spy(Arc::clone(&count))];
        let last = raw_pop(&mut v).unwrap();
        assert_eq!(v.len(), 1, "the Vec must no longer count the popped slot");
        assert_eq!(
            count.load(Ordering::Relaxed),
            0,
            "drops so far: a pop moves the value out; it drops nothing"
        );
        drop(v);
        assert_eq!(
            count.load(Ordering::Relaxed),
            1,
            "drops after dropping the Vec: only the element that stayed in it"
        );
        drop(last);
        assert_eq!(
            count.load(Ordering::Relaxed),
            2,
            "drops in total: each of the two values exactly once"
        );
        assert!(raw_pop::<String>(&mut vec![]).is_none());
    }
    #[test]
    fn a_pops_the_last_element_in_lifo_order() {
        let mut v = vec![String::from("a"), String::from("b"), String::from("c")];
        let pc = v[2].as_ptr();
        let (c, allocations) = count(|| raw_pop(&mut v));
        assert_eq!(
            (c.as_deref(), allocations),
            (Some("c"), 0),
            "(popped value, allocations): a pop moves a String header; it never allocates"
        );
        let c = c.unwrap();
        assert_eq!(c.as_ptr(), pc, "moved out, not cloned");
        assert_eq!(v, ["a", "b"]);
        assert_eq!(raw_pop(&mut v).as_deref(), Some("b"));
        assert_eq!(raw_pop(&mut v).as_deref(), Some("a"));
        assert_eq!(raw_pop(&mut v), None);
        assert!(v.is_empty());
    }
    #[test]
    fn b_initialized_values_are_owned_by_returned_array() {
        assert_eq!(initialized_pair("a".into(), "β".into()), ["a", "β"]);
    }
    #[test]
    fn b_pair_moves_the_strings_in() {
        let first = String::from("first");
        let second = String::from("second");
        let (p1, p2) = (first.as_ptr(), second.as_ptr());
        let (pair, allocations) = count(|| initialized_pair(first, second));
        assert_eq!(allocations, 0, "no clones or placeholders");
        assert_eq!(
            (pair[0].as_ptr(), pair[1].as_ptr()),
            (p1, p2),
            "the array must own the two original text buffers, in order"
        );
    }
    #[test]
    fn a_b_technique_raw_moves() {
        let file = include_str!("l06_moves.rs");
        let pop = crate::source::body_of(file, "fn raw_pop");
        assert!(
            pop.contains("set_len") && !pop.contains(".pop("),
            "task A re-creates `Vec::pop`: shrink the length with `set_len`, then move the \
             last element out with `ptr::read`"
        );
        let pair = crate::source::body_of(file, "fn initialized_pair");
        assert!(
            pair.contains("MaybeUninit"),
            "task B practises initializing an array slot by slot: write both Strings into \
             `MaybeUninit<String>` slots before you claim that the array is initialized"
        );
    }

    struct Noisy<'a>(&'static str, &'a RefCell<Vec<&'static str>>);
    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(self.0);
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// `left` in a failure message is your prediction; `right` is the real value.
    #[test]
    fn p_predictions() {
        // P1. ptr::read makes a bitwise copy; then the source is forgotten.
        // How many times does "a" get dropped?
        let log = RefCell::new(Vec::new());
        let a = Noisy("a", &log);
        // SAFETY: `a` is valid and initialized; it is forgotten right after,
        // so exactly one owner (`b`) remains.
        let b = unsafe { std::ptr::read(&a) };
        std::mem::forget(a);
        drop(b);
        let p1: usize = guess();
        assert_eq!(p1, log.borrow().len(), "P1: read + forget");

        // P2. A MaybeUninit holding a value goes out of scope. What is logged?
        let log = RefCell::new(Vec::new());
        {
            let _slot = MaybeUninit::new(Noisy("m", &log));
        }
        let p2: Vec<&str> = guess();
        assert_eq!(p2, *log.borrow(), "P2: MaybeUninit and Drop");

        // P3. set_len shrinks the length without dropping (this leaks "y",
        // which is safe). What is logged once the vector itself is dropped?
        let log = RefCell::new(Vec::new());
        let mut v = vec![Noisy("x", &log), Noisy("y", &log)];
        // SAFETY: 1 <= len, and element 0 stays initialized.
        unsafe { v.set_len(1) };
        drop(v);
        let p3: Vec<&str> = guess();
        assert_eq!(p3, *log.borrow(), "P3: set_len and ownership");
    }
}
