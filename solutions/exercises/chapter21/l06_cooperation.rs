//! LESSON 06 — Cooperative work budgets
//! Run: cargo test --test chapter21 l06_
//! Read first: "Lesson 06" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. Can a future starve peers even though it returns a Future type?
//! 2. Does a runtime preempt arbitrary CPU loops inside poll?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Explain bounded work per poll and why a self-wake after progress is appropriate here.
//! COMPILER LAB (optional; examples/ch21_scratch.rs): Compare budgets 1, 3, and 100 by counting poll calls for the same input. Explain why a poll count is not a timing benchmark.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
struct BudgetSum<'a> {
    values: &'a [u32],
    position: usize,
    total: u64,
    budget: usize,
}
impl Future for BudgetSum<'_> {
    type Output = u64;
    /// A. Process at most budget elements per poll, widening before addition.
    /// If unfinished wake and return Pending; else return Ready(total).
    /// Constructor guarantees budget > 0 and total fits u64.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u64> {
        // `BudgetSum` is `Unpin`, so the pin can be turned into a plain
        // `&mut`. That lets the loop read `values` and update `total` at the
        // same time, which is not possible through the `Pin` itself.
        let this = &mut *self;
        let end = this
            .position
            .saturating_add(this.budget)
            .min(this.values.len());
        for &value in &this.values[this.position..end] {
            this.total += u64::from(value);
        }
        this.position = end;
        if end < this.values.len() {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(this.total)
        }
    }
}
/// B. Construct with a nonzero budget, rejecting zero as None.
fn budget_sum(values: &[u32], budget: usize) -> Option<BudgetSum<'_>> {
    (budget > 0).then_some(BudgetSum {
        values,
        position: 0,
        total: 0,
        budget,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Counter(AtomicUsize);
    impl std::task::Wake for Counter {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn counted() -> (Arc<Counter>, std::task::Waker) {
        let c = Arc::new(Counter(AtomicUsize::new(0)));
        let w = std::task::Waker::from(Arc::clone(&c));
        (c, w)
    }
    /// Polls until Ready; returns (output, polls, wakes requested).
    fn run(mut f: BudgetSum<'_>) -> (u64, usize, usize) {
        let (c, w) = counted();
        let mut cx = Context::from_waker(&w);
        for polls in 1..=10_000 {
            if let Poll::Ready(total) = Pin::new(&mut f).poll(&mut cx) {
                return (total, polls, c.0.load(Ordering::Relaxed));
            }
        }
        panic!("BudgetSum never finished")
    }
    #[test]
    fn a_b_work_is_bounded_and_wakes_only_when_needed() {
        let mut f = budget_sum(&[1, 2, 3, 4, 5], 2).unwrap();
        let (c, w) = counted();
        let mut cx = Context::from_waker(&w);
        assert_eq!(Pin::new(&mut f).poll(&mut cx), Poll::Pending);
        assert_eq!(
            f.position, 2,
            "a poll with budget 2 must process 2 elements"
        );
        assert_eq!(Pin::new(&mut f).poll(&mut cx), Poll::Pending);
        assert_eq!(
            f.position, 4,
            "the second poll continues where the first stopped"
        );
        assert_eq!(Pin::new(&mut f).poll(&mut cx), Poll::Ready(15));
        assert_eq!(
            c.0.load(Ordering::Relaxed),
            2,
            "wake once for each Pending, and not for Ready"
        );
    }
    #[test]
    fn a_b_empty_and_invalid_budget() {
        assert!(budget_sum(&[], 0).is_none());
        assert!(budget_sum(&[1, 2], 0).is_none());
        let mut f = budget_sum(&[], 1).unwrap();
        assert_eq!(
            Pin::new(&mut f).poll(&mut Context::from_waker(std::task::Waker::noop())),
            Poll::Ready(0)
        );
    }
    #[test]
    fn a_b_large_values_widen_before_adding() {
        let values = [u32::MAX; 5];
        let (total, _, _) = run(budget_sum(&values, 2).unwrap());
        assert_eq!(total, 5 * u64::from(u32::MAX));
    }
    #[test]
    fn a_b_poll_count_follows_the_budget() {
        let values: Vec<u32> = (1..=10).collect();
        // Each tuple is (sum, polls, wakes).
        assert_eq!(
            run(budget_sum(&values, 3).unwrap()),
            (55, 4, 3),
            "10 elements, budget 3: chunks of 3, 3, 3 and 1"
        );
        assert_eq!(
            run(budget_sum(&values, 10).unwrap()),
            (55, 1, 0),
            "a budget equal to the length finishes in one poll, without a wake"
        );
        assert_eq!(
            run(budget_sum(&values, 100).unwrap()),
            (55, 1, 0),
            "a budget above the length must not read past the end"
        );
        assert_eq!(
            run(budget_sum(&values, 1).unwrap()),
            (55, 10, 9),
            "budget 1: one element per poll, no wake after the last one"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. The same two values summed after widening, and with wrapping u32 math.
        let values = [u32::MAX, 1];
        let widened: u64 = values.iter().map(|&v| u64::from(v)).sum();
        let wrapped = values.iter().fold(0u32, |acc, &v| acc.wrapping_add(v));
        let p1: (u64, u32) = (4_294_967_296, 0);
        assert_eq!(p1, (widened, wrapped), "P1: widen before adding");

        // P2. Position 4 in a 5-element slice with budget 2: how many elements
        // does this poll process?
        let (len, position, budget) = (5usize, 4usize, 2usize);
        let end = (position + budget).min(len);
        let p2: usize = 1;
        assert_eq!(p2, end - position, "P2: the last chunk");
    }
}
