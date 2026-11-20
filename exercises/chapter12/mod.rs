//! Chapter 12: Operators for your own types. Original guided practice.
//! Run one lesson: cargo test --test chapter12 l01_
// `unused_imports` is allowed because `guess` is no longer used by a lesson once
// you have answered all of its predictions. Then delete the lesson's `guess`
// import, as the solutions do.
#![allow(dead_code, unused_variables, unused_mut, unused_imports)]

mod c01_code_review;
mod l01_arithmetic;
mod l02_equality;
mod l03_ordering;
mod l04_indexing;
mod l05_capstone;
mod r01_review;

// Lets tests count heap allocations, so they can check how you solved a task.
#[global_allocator]
static ALLOC: practice::alloc::Counting = practice::alloc::Counting;

/// Test helpers for this chapter. You do not need to edit them.
///
/// Why they exist: an operator method that uses its own operator calls itself
/// without end. `fn add(self, rhs: Self) -> Self { self + rhs }` is the classic
/// case, because that `+` *is* `add`. A debug build then stops with
/// "has overflowed its stack". A release build can turn the same code into an
/// endless loop, and the test run would never finish. These helpers run the
/// operator on a second thread and give up after ten seconds, so you get a
/// failed test with a message instead.
mod guard {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    /// Runs `f` on a helper thread and returns its result. Fails the test if `f`
    /// has not finished after ten seconds. `what` names the code under test.
    #[track_caller]
    pub fn run_op<R: Send + 'static>(what: &str, f: impl FnOnce() -> R + Send + 'static) -> R {
        let what = format!(
            "{what} [chapter 12 hint: an operator method that uses its own operator, \
             such as `self + rhs` inside `add`, calls itself forever]"
        );
        practice::within(10, &what, f)
    }

    /// Runs `f`, which is expected to panic, with the same ten-second limit.
    /// Returns the panic message, or `None` if `f` returned without panicking.
    #[track_caller]
    pub fn panic_message<R>(what: &str, f: impl FnOnce() -> R + Send + 'static) -> Option<String> {
        run_op(what, move || {
            let payload = catch_unwind(AssertUnwindSafe(|| drop(f()))).err()?;
            let text = match payload.downcast_ref::<&str>() {
                Some(text) => Some((*text).to_string()),
                None => payload.downcast_ref::<String>().cloned(),
            };
            Some(text.unwrap_or_else(|| "a panic whose message is not text".to_string()))
        })
    }
}
