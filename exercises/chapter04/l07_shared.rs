//! LESSON 07 — Rc and Arc
//! Run: cargo test --test chapter04 l07_
//! Read first: "Lesson 07" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Does `Rc::clone` make a second copy of the String's text?
//! 2. Can two `Rc` owners give you a `&mut String`?
//! 3. Which of the two pointer types may cross into another thread?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain strong counts, what happens when the last owner
//! is dropped, what `Arc` costs compared with `Rc`, and why two values that
//! own each other through `Rc` are never freed.
//!
//! COMPILER / TRANSFER LAB (optional): Try to send an `Rc<String>` into
//! `thread::spawn` and read the error; then use `Arc`. Compare `Rc::clone(&a)`
//! with `(*a).clone()`: which one allocates? On paper, sketch two nodes that
//! own each other and say why neither count reaches zero. You will build and
//! repair such a cycle with `Weak` in Chapter 9.
//! Use examples/ch04_scratch.rs for this, and leave it compiling afterward.

use std::rc::Rc;
use std::sync::Arc;
use std::thread::ThreadId;
/// A. Return two owners of one allocation, containing input text.
fn shared_text(text: &str) -> (Rc<String>, Rc<String>) {
    todo!("Implement this task")
}
/// B. Read shared text on another thread.
///
/// Spawn one thread and join it. Give the worker thread its own `Arc` (clone
/// the `Arc`, not its `String`) and keep `text` alive in the calling thread
/// until the worker has finished. The worker reports three things, and this
/// function returns them in this order:
///
/// 1. the byte length of the text,
/// 2. `Arc::strong_count`, read inside the worker,
/// 3. the worker's own `std::thread::current().id()`.
fn worker_report(text: Arc<String>) -> (usize, usize, ThreadId) {
    todo!("Implement this task")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, guess};
    use std::thread;
    #[test]
    fn a_pointers_share_one_allocation() {
        let (a, b) = shared_text("harbor");
        assert!(
            Rc::ptr_eq(&a, &b),
            "both Rc values must point at the same allocation: clone the Rc, do not \
             build a second one"
        );
        assert_eq!(
            Rc::strong_count(&a),
            2,
            "exactly two owners must be alive: the two Rc values that were returned"
        );
        drop(b);
        assert_eq!(Rc::strong_count(&a), 1, "dropping one owner leaves one");
        assert_eq!(a.as_str(), "harbor");
    }
    #[test]
    fn b_arc_crosses_a_thread_boundary() {
        let text = Arc::new(String::from("🦀"));
        let (length, _, _) = worker_report(Arc::clone(&text));
        assert_eq!(length, 4, "the crab is four bytes of UTF-8");
        assert_eq!(
            Arc::strong_count(&text),
            1,
            "after worker_report returns, only the test's own Arc may be left: \
             join the worker and let every other Arc be dropped"
        );
        let (length, _, _) = worker_report(Arc::new(String::new()));
        assert_eq!(length, 0);
    }

    #[test]
    fn b_the_worker_is_another_thread_with_its_own_arc() {
        let text = Arc::new(String::from("harbor"));
        let (_, owners, worker) = worker_report(Arc::clone(&text));
        assert_ne!(
            worker,
            thread::current().id(),
            "the report must come from a spawned thread, but it carries the id of the \
             calling thread"
        );
        assert_eq!(
            owners, 3,
            "inside the worker there must be 3 owners: this test, the `text` parameter kept by \
             the calling thread, and the worker's own clone. A smaller number means that `text` \
             itself was moved into the thread, that the String was cloned instead of the Arc, \
             or that the count was read outside the worker"
        );
    }

    #[test]
    fn a_shared_text_makes_one_rc_allocation() {
        let (_, allocations) = count(|| shared_text("harbor"));
        assert!(
            allocations <= 2,
            "shared_text made {allocations} heap allocations; it needs at most 2 \
             (one String buffer, one Rc block). More means the text was stored twice"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        let a = Rc::new(String::from("config"));
        // P1. Allocations made by Rc::clone.
        let (b, allocations) = count(|| Rc::clone(&a));
        let p1: usize = guess();
        assert_eq!(
            p1, allocations,
            "P1: allocations made by Rc::clone (left: your prediction, right: actual)"
        );

        // P2. Strong count after the clone, and after dropping it.
        let p2: (usize, usize) = guess();
        let after_clone = Rc::strong_count(&a);
        drop(b);
        assert_eq!(
            p2,
            (after_clone, Rc::strong_count(&a)),
            "P2: strong counts (left: your prediction, right: actual)"
        );

        // P3. `(*a).clone()` instead: allocations made, and the count afterwards.
        let (text, allocations) = count(|| (*a).clone());
        let p3: (usize, usize) = guess();
        assert_eq!(
            p3,
            (allocations, Rc::strong_count(&a)),
            "P3: (allocations, strong count) after `(*a).clone()` (left: your prediction, \
             right: actual)"
        );
        assert_eq!(text, "config");
    }
}
