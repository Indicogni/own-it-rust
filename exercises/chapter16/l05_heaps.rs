//! LESSON 05 — Priority queues and heap ordering
//! Run: cargo test --test chapter16 l05_
//! Read first: "Lesson 05" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does BinaryHeap iteration produce sorted order? Is the default
//! heap a min-heap or a max-heap? Which value must sit on top of a heap that keeps "the k largest"?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why should you pop to obtain priority order? How does Reverse adapt
//! ordering without writing a second collection?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch16_scratch.rs):
//! Build a heap, compare iter(), into_vec(), and repeated pop(), recording only documented
//! guarantees. Use peek_mut to lower the top element and inspect reheapification after the
//! guard drops.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, lab, and explanation are done.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
/// A. `top` holds the largest values seen so far, at most `k` of them, as a
/// min-heap: the smallest value it holds is on top. Offer one more value.
/// Afterwards `top` holds the `k` largest of (its old content plus `value`).
/// Equal values count separately. With k == 0 the heap stays empty.
fn offer(top: &mut BinaryHeap<Reverse<i32>>, value: i32, k: usize) {
    todo!("Task 1: implement the documented contract")
}
/// B. Return the k largest numbers of a stream, largest first; keep duplicates.
/// A stream with fewer than k numbers gives all of them. The stream may be far
/// longer than k, so do not collect it: hold at most k + 1 numbers at any
/// moment (task A does exactly that).
fn largest(values: impl IntoIterator<Item = i32>, k: usize) -> Vec<i32> {
    todo!("Task 2: implement the documented contract")
}
/// C. A timer queue. Each entry is (due time, name), and the entry with the
/// earliest due time is on top. Remove every entry whose due time is <= now and
/// return the names, earliest first; equal times come out in ascending name
/// order. Entries that are due later stay in the queue.
fn pop_due(queue: &mut BinaryHeap<Reverse<(u32, String)>>, now: u32) -> Vec<String> {
    todo!("Task 3: implement the documented contract")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, guess, within};
    #[test]
    fn a_offer_keeps_only_the_k_largest() {
        // `within` fails the test with a message if a loop never ends.
        within(10, "the loop in offer", || {
            let mut top = BinaryHeap::new();
            for (value, expected_len) in [(5, 1), (1, 2), (9, 3), (7, 3), (0, 3), (9, 3)] {
                offer(&mut top, value, 3);
                assert_eq!(
                    top.len(),
                    expected_len,
                    "after offering {value}: the heap holds every value until it has k = 3, \
                     then never more than 3"
                );
            }
            assert_eq!(
                top.peek(),
                Some(&Reverse(7)),
                "the top of the min-heap is the smallest of the three largest values"
            );
            let mut kept: Vec<i32> = top.into_iter().map(|Reverse(v)| v).collect();
            kept.sort_unstable();
            assert_eq!(kept, [7, 9, 9], "equal values count separately");
            let mut none = BinaryHeap::new();
            offer(&mut none, 4, 0);
            assert!(none.is_empty(), "with k == 0 nothing is kept");
        });
    }
    #[test]
    fn a_offer_never_grows_the_heap_past_k() {
        let (len, allocations) = within(10, "the loop in offer", || {
            let mut top = BinaryHeap::with_capacity(8);
            // Allocations are counted per thread, so `count` runs inside `within`.
            let ((), allocations) = count(|| {
                for value in 0..10_000 {
                    offer(&mut top, (value * 37) % 1009, 4);
                }
            });
            (top.len(), allocations)
        });
        assert_eq!(len, 4, "10 000 offers with k = 4 leave exactly 4 values");
        assert_eq!(
            allocations, 0,
            "a heap with room for 8 values that never holds more than k + 1 = 5 \
             never needs a new buffer; got {allocations} allocations"
        );
    }
    #[test]
    fn b_largest_come_out_in_descending_order() {
        within(10, "the loop in largest", || {
            assert_eq!(largest([3, 9, 2, 9], 3), [9, 9, 3]);
            assert_eq!(largest([1], 8), [1], "a short stream gives all it has");
            assert!(largest([1], 0).is_empty());
            assert!(largest([], 3).is_empty());
            assert_eq!(largest([-5, -1, -3], 2), [-1, -3]);
            assert_eq!(largest(vec![4, 4, 4], 2), [4, 4]);
        });
    }
    #[test]
    fn b_largest_holds_only_k_values_of_a_long_stream() {
        // A filtered range cannot say in advance how many items it has, so a
        // vector or heap that takes in the whole stream must grow many times.
        fn stream() -> impl Iterator<Item = i32> {
            (0..100_000)
                .filter(|n| n % 7 != 0)
                .map(|n| (n * 7919) % 100_003)
        }
        let mut expected: Vec<i32> = stream().collect();
        expected.sort_unstable_by(|a, b| b.cmp(a));
        expected.truncate(4);
        let (top, allocations) =
            within(20, "the loop in largest", || count(|| largest(stream(), 4)));
        assert_eq!(top, expected);
        assert!(
            allocations <= 8,
            "a heap of at most k + 1 = 5 values and the result need only a few \
             allocations; {allocations} means the whole stream was collected"
        );
    }

    fn timers(entries: &[(u32, &str)]) -> BinaryHeap<Reverse<(u32, String)>> {
        entries
            .iter()
            .map(|&(due, name)| Reverse((due, name.to_string())))
            .collect()
    }
    #[test]
    fn c_pop_due_takes_due_entries_in_time_order() {
        // `within` fails the test with a message if the loop never ends, for
        // example because it keeps looking at an entry that is not due yet.
        within(10, "the loop in pop_due", || {
            let mut queue = timers(&[(30, "c"), (10, "b"), (20, "z"), (10, "a"), (40, "d")]);
            assert!(
                pop_due(&mut queue, 5).is_empty(),
                "nothing is due at time 5"
            );
            assert_eq!(queue.len(), 5);
            assert_eq!(
                pop_due(&mut queue, 20),
                ["a", "b", "z"],
                "earliest first; equal times in ascending name order"
            );
            assert_eq!(queue.len(), 2, "entries that are due later stay queued");
            assert_eq!(
                queue.peek().map(|Reverse((due, _))| *due),
                Some(30),
                "the earliest remaining entry is on top"
            );
            assert_eq!(pop_due(&mut queue, 30), ["c"], "due time == now is due");
            assert_eq!(pop_due(&mut queue, u32::MAX), ["d"]);
            assert!(pop_due(&mut queue, u32::MAX).is_empty());
            assert!(pop_due(&mut BinaryHeap::new(), 7).is_empty());
        });
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        let heap = BinaryHeap::from(vec![3, 1, 4, 1, 5]);
        // P1. What does peek see?
        let p1: Option<i32> = guess();
        assert_eq!(p1, heap.peek().copied(), "P1: default heap order");

        // P2. into_sorted_vec: which direction?
        let p2: Vec<i32> = guess();
        assert_eq!(p2, heap.into_sorted_vec(), "P2: into_sorted_vec");

        // P3. The first pop from a heap of Reverse values.
        let mut heap = BinaryHeap::from([Reverse(3), Reverse(1), Reverse(2)]);
        let p3: Option<i32> = guess();
        assert_eq!(p3, heap.pop().map(|Reverse(x)| x), "P3: Reverse");

        // P4. Tuples compare field by field. Which pops first?
        let mut heap = BinaryHeap::from([(2, "a"), (1, "z"), (2, "b")]);
        let p4: Option<(i32, &str)> = guess();
        assert_eq!(p4, heap.pop(), "P4: ties in a max-heap");

        // P5. into_sorted_vec sorts ascending by the element type's own order.
        // The elements here are Reverse values. In which order do the numbers
        // inside them come out?
        let heap = BinaryHeap::from([Reverse(2), Reverse(9), Reverse(5)]);
        let p5: Vec<i32> = guess();
        assert_eq!(
            p5,
            heap.into_sorted_vec()
                .into_iter()
                .map(|Reverse(x)| x)
                .collect::<Vec<_>>(),
            "P5: ascending Reverse values are descending numbers"
        );
    }
}
