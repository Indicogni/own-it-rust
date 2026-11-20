//! LESSON 06 — Pointer types encode different policies
//! Run: cargo test --test chapter03 l06_
//! Read first: CONCEPTS.md, section "Lesson 06".
//!
//! PREDICT: first answer `p_predictions` at the bottom (replace each `guess()`
//! with your answer). Then consider: Which types borrow and which own an
//! allocation: `&T`, `&mut T`, `Box<T>`, `Rc<T>`, `Arc<T>`? When does each
//! owned allocation get freed? Does cloning an Rc clone its string contents?
//! What does Arc add to Rc? Arc's count is atomic; it does not by itself
//! synchronize mutation of its contents.
//!
//! EXPLAIN: Why are nested Rust tuples values stored together rather than
//! automatically separate heap objects? Why use `Option<&T>` for absence?
//! How do raw pointers differ in nullability and dereferencing requirements?
//! Recognizing these policies is enough here; full borrowing comes later.
//!
//! COMPILER LAB: try mutating through `&i32`, then through `&mut i32`.
//! Separately, try reading through a `*const i32` in safe code. Repair that
//! experiment by using a reference; no exercise calls for an unsafe block.
//! Compare `size_of::<&i32>()` with `size_of::<Option<&i32>>()`, and then with
//! `size_of::<&[i32]>()`. Explain absence without null references and slice metadata.

use std::rc::Rc;
use std::sync::Arc;

/// A. Add one to the borrowed integer and return its new value.
/// Inputs are below i32::MAX.
fn increment(value: &mut i32) -> i32 {
    *value += 1;
    *value
}

/// B. Allocate one pair on the heap and return its owning box.
fn boxed_point(x: i32, y: i32) -> Box<(i32, i32)> {
    Box::new((x, y))
}

/// C. Return TWO Rc pointers to the SAME owned String allocation.
fn share_label(label: &str) -> (Rc<String>, Rc<String>) {
    let first = Rc::new(label.to_string());
    // Cloning the Rc copies the pointer and adds one to the count. The String
    // is not copied.
    let second = Rc::clone(&first);
    (first, second)
}

/// D. Return TWO Arc pointers to the SAME u32 allocation.
/// The tests supply the thread code; your task is just construction and sharing.
fn share_count(value: u32) -> (Arc<u32>, Arc<u32>) {
    let first = Arc::new(value);
    let second = Arc::clone(&first);
    (first, second)
}

/// E. Borrow the supplied value only when visible is true; otherwise use None.
/// Do not copy the integer into a new allocation.
fn visible_value(value: &i32, visible: bool) -> Option<&i32> {
    if visible { Some(value) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;

    #[test]
    fn a_mutable_borrow_changes_the_original() {
        let mut value = 41;
        assert_eq!(increment(&mut value), 42);
        assert_eq!(value, 42);
        let mut negative = -1;
        assert_eq!(increment(&mut negative), 0);
    }

    #[test]
    fn b_box_owns_the_pair() {
        let point = boxed_point(3, -8);
        assert_eq!(*point, (3, -8));
    }

    #[test]
    fn c_rc_pointers_share_one_allocation() {
        let (first, second) = share_label("sensor north");
        assert_eq!(first.as_str(), "sensor north");
        assert!(
            Rc::ptr_eq(&first, &second),
            "both Rc pointers must point at one allocation: clone the Rc, \
             do not build a second String"
        );
        assert_eq!(
            Rc::strong_count(&first),
            2,
            "two owners are alive, so the strong count must be 2"
        );
        drop(first);
        assert_eq!(
            Rc::strong_count(&second),
            1,
            "dropping one owner must leave a strong count of 1"
        );
        assert_eq!(second.as_str(), "sensor north");
    }

    #[test]
    fn d_arc_keeps_the_shared_value_alive_across_threads() {
        let (first, second) = share_count(42);
        assert!(
            Arc::ptr_eq(&first, &second),
            "both Arc pointers must point at one allocation: clone the Arc, \
             do not call Arc::new twice"
        );
        assert_eq!(
            Arc::strong_count(&first),
            2,
            "two owners are alive, so the strong count must be 2"
        );
        let worker = std::thread::spawn(move || *second);
        assert_eq!(worker.join().expect("worker panicked"), 42);
        assert_eq!(*first, 42);
        assert_eq!(
            Arc::strong_count(&first),
            1,
            "the worker thread dropped its Arc, so one owner is left"
        );
    }

    #[test]
    fn e_optional_reference_preserves_identity() {
        let value = 123;
        let borrowed = visible_value(&value, true).expect("visible value missing");
        assert!(
            std::ptr::eq(borrowed, &value),
            "return the reference you were given, not a reference to a copy"
        );
        assert_eq!(visible_value(&value, false), None);
    }

    #[test]
    fn b_c_d_each_pointer_allocates_once() {
        let (_, allocations) = count(|| boxed_point(1, 2));
        assert!(
            allocations <= 1,
            "one Box, one allocation; got {allocations}"
        );
        let (_, allocations) = count(|| share_label("sensor north"));
        assert!(
            allocations <= 2,
            "one String buffer and one Rc allocation; got {allocations}"
        );
        let (_, allocations) = count(|| share_count(7));
        assert!(allocations <= 1, "one Arc allocation; got {allocations}");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Box<i32> size in machine words, and whether Option<&i32> is
        // the same size as &i32.
        let p1: (usize, bool) = (1, true);
        assert_eq!(
            p1,
            (
                size_of::<Box<i32>>() / size_of::<usize>(),
                size_of::<&i32>() == size_of::<Option<&i32>>()
            ),
            "P1: pointer sizes. Left = your prediction, right = what Rust computes."
        );

        // P2. Strong counts inside the block, and after it ends.
        let a = Rc::new(String::from("x"));
        let b = Rc::clone(&a);
        let inside;
        {
            let c = Rc::clone(&b);
            inside = Rc::strong_count(&a);
        }
        let p2: (usize, usize) = (3, 2);
        assert_eq!(
            p2,
            (inside, Rc::strong_count(&a)),
            "P2: Rc strong counts. Left = your prediction, right = what Rust computes."
        );

        // P3. Moving a Box: does the heap value move to a new address?
        let boxed = Box::new(5);
        let address: *const i32 = &*boxed;
        let moved = boxed;
        let p3: bool = true; // true if `moved` points at the same address
        assert_eq!(
            p3,
            std::ptr::eq(address, &*moved),
            "P3: the heap address after moving a Box. Left = your prediction, right = what Rust computes."
        );

        // P4. Is each of these raw pointers null?
        let value = 5;
        let raw: *const i32 = &value;
        let p4: (bool, bool) = (false, true);
        assert_eq!(
            p4,
            (raw.is_null(), std::ptr::null::<i32>().is_null()),
            "P4: raw pointer nullability. Left = your prediction, right = what Rust computes."
        );
    }
}
