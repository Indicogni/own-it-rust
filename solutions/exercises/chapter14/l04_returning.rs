//! LESSON 04 — Returning borrowed and erased closures
//! Run: cargo test --test chapter14 l04_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Can a returned closure borrow a local String
//! created inside its factory? Why can a boxed callable return different closure types from
//! branches?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Compare the allocation and dispatch choices of impl Fn and Box<dyn
//! Fn>. What is the captured-data lifetime of each API?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch14_scratch.rs):
//! Remove move from a returned closure that uses a parameter; inspect the error. Return a
//! closure that borrows a parameter, then try to use it after the referenced String is
//! dropped. Add `+ use<>` to the return type and read why the compiler rejects it. Change
//! the return type to Box<dyn Fn(&str) -> bool> and find the bound that makes it compile.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete when its tests, predictions, and explanation are done. The lab is optional.

/// A. Return a test for "text starts with prefix". Borrow prefix for as long as the
/// returned callable is used; do not copy it.
///
/// No lifetime is written here on purpose. In edition 2024 a returned `impl Trait`
/// may use every lifetime that is in scope, so the closure can hold `prefix` and the
/// borrow checker still stops callers from using it after the prefix is gone.
fn prefix_test(prefix: &str) -> impl Fn(&str) -> bool {
    // `move` copies the `&str` (a pointer and a length) into the closure. The
    // text itself is not copied.
    move |text: &str| -> bool { text.starts_with(prefix) }
}
/// B. If double, multiply by 2; otherwise add 1. Inputs are in -1000..=1000.
/// Return different closure types behind one boxed interface: each branch boxes its
/// own closure, and neither closure captures `double`.
fn choose(double: bool) -> Box<dyn Fn(i32) -> i32> {
    // Decide once, here, instead of capturing `double` and testing it on
    // every call. Each closure captures nothing, so each box is zero-sized.
    if double {
        Box::new(|x| x * 2)
    } else {
        Box::new(|x| x + 1)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::hint::black_box;
    use std::mem::size_of_val;
    #[test]
    fn a_returned_closure_keeps_a_borrow() {
        let p = String::from("β");
        let test = prefix_test(&p);
        assert!(test("βeta"));
        assert!(!test("beta"));
        assert!(!test(""));
        assert!(prefix_test("")("anything"));
    }
    #[test]
    fn a_returned_closure_borrows_without_copying() {
        let p = String::from("log:");
        let (hit, allocations) = count(|| {
            let test = prefix_test(&p);
            test("log: ready") && !test("lo")
        });
        assert!(hit);
        assert_eq!(allocations, 0, "borrow the prefix; do not copy it");
        assert_eq!(
            size_of_val(&prefix_test(&p)),
            size_of::<&str>(),
            "the returned closure should hold just the borrowed &str"
        );
    }
    #[test]
    fn b_erased_closures_support_multiple_branches() {
        assert_eq!(choose(true)(6), 12);
        assert_eq!(choose(false)(6), 7);
        assert_eq!(choose(true)(-4), -8);
        assert_eq!(choose(false)(-1), 0);
    }
    #[test]
    fn b_each_branch_boxes_a_capture_free_closure() {
        let (f, allocations) = count(|| choose(true));
        assert_eq!(f(3), 6);
        assert_eq!(
            allocations, 0,
            "a closure that captures nothing is zero-sized, so boxing it does not allocate"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Machine words of captured state in a closure that borrows a &str.
        // (No lifetime bound is written on `impl Fn`: edition 2024 lets the returned
        // closure hold the borrow anyway.)
        fn starts(prefix: &str) -> impl Fn(&str) -> bool {
            move |text| text.starts_with(prefix)
        }
        let test = starts("api/");
        let p1: usize = 2;
        assert_eq!(
            p1 * size_of::<usize>(),
            size_of_val(&test),
            "P1: sizes in bytes (your words x word size, actual); an impl Fn is the \
             closure itself, stored inline"
        );

        // P2. Heap allocations made by boxing each closure: (captures nothing, captures i32).
        // (`black_box` only stops the optimizer from removing an allocation.)
        let offset = 5;
        let (plain, plain_allocs) =
            count(|| -> Box<dyn Fn(i32) -> i32> { black_box(Box::new(|x: i32| x * 2)) });
        let (shifted, shifted_allocs) =
            count(|| -> Box<dyn Fn(i32) -> i32> { black_box(Box::new(move |x: i32| x + offset)) });
        let p2: (usize, usize) = (0, 1);
        assert_eq!(
            p2,
            (plain_allocs, shifted_allocs),
            "P2: (allocations for the capture-free box, for the i32-capturing box); \
             Box allocates only for non-zero-sized values"
        );
        assert_eq!(plain(1) + shifted(1), 8);

        // P3. Each call to a factory makes a new closure with its own state.
        fn counter() -> impl FnMut() -> u32 {
            let mut n = 0;
            move || {
                n += 1;
                n
            }
        }
        let mut a = counter();
        let mut b = counter();
        a();
        a();
        let p3: (u32, u32) = (3, 1);
        assert_eq!(p3, (a(), b()), "P3: factories return independent state");
    }
}
