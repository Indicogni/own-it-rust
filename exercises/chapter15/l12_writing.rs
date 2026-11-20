//! LESSON 12 — Writing iterators: return types, both ends, and your own adapter
//! Run: cargo test --test chapter15 l12_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: When a function returns `impl Iterator`,
//! which part of its body runs at the call, and which part runs once per item? What must
//! always hold between next, next_back, and len?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: When do you return `impl Iterator`, and when `Box<dyn Iterator>`?
//! Why does adding a method such as `.dedup_runs()` to every iterator need a new trait?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch15_scratch.rs):
//! Copy your `ordered` and change its return type to `impl Iterator<Item = i32>`; read
//! the diagnostic about the two branches. Then write `Box<dyn Iterator<Item = i32>>`
//! without the lifetime and read that diagnostic too.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, and explanation are done.

use std::iter::Peekable;

/// A. Lazily yield the whitespace-separated words of `text` that have more than
/// `min` chars, in order. The words borrow from `text`. Do not build a Vec.
#[allow(unreachable_code)]
fn long_words(text: &str, min: usize) -> impl Iterator<Item = &str> {
    // Replace both lines. The second one only gives the unfinished function an
    // iterator type, so that the file compiles.
    todo!("Task 1: implement the documented contract");
    std::iter::empty()
}

/// B. Yield the values in slice order, or in reverse order when `newest_first`
/// is true. The two directions are two different iterator types, so return a
/// boxed trait object. Box the iterator itself; do not collect the values.
fn ordered(values: &[i32], newest_first: bool) -> Box<dyn Iterator<Item = i32> + '_> {
    todo!("Task 2: implement the documented contract")
}

/// Neighbouring pairs of a slice that do not overlap: `[a, b, c, d, e]` gives
/// `(&a, &b)` and then `(&c, &d)`. A last element without a partner is ignored,
/// from whichever end you start.
///
/// C. Design this type and write its trait impls yourself. Nothing is
/// pre-written, and the tests reach your work only through `pairs` below.
/// 1. Choose the fields. The one below is a starting point; change it freely.
/// 2. Write `impl<'a, T> Iterator for Pairs<'a, T>` with
///    `Item = (&'a T, &'a T)` and a `size_hint` whose two bounds are exact.
/// 3. Write `impl DoubleEndedIterator`: `next_back` yields the last complete
///    pair. The two ends must never yield the same pair twice.
/// 4. Write `impl ExactSizeIterator`.
struct Pairs<'a, T> {
    rest: &'a [T],
}

/// D. Build your `Pairs` over `values`. The return type lists what you promise
/// to callers: an iterator that also works from the back and knows its length.
/// Your impls from task C are what make `Pairs` fit that promise.
#[allow(unreachable_code)]
fn pairs<T>(values: &[T]) -> impl DoubleEndedIterator<Item = (&T, &T)> + ExactSizeIterator {
    // Replace both lines, as in task A.
    todo!("Task 4: implement the documented contract");
    std::iter::empty()
}

/// A lazy adapter that shortens runs: of several equal items in a row, only the
/// first is yielded. `[1, 1, 2, 2, 2, 1]` becomes `1, 2, 1`.
struct DedupRuns<I: Iterator> {
    inner: Peekable<I>,
}
impl<I> Iterator for DedupRuns<I>
where
    I: Iterator,
    I::Item: PartialEq,
{
    type Item = I::Item;
    /// E. Yield the first item of the next run and discard the equal items that
    /// follow it directly. Look at most one item past the end of the run, and
    /// do not clone items.
    fn next(&mut self) -> Option<I::Item> {
        todo!("Task 5: implement the documented contract")
    }
    /// F. Bounds on the number of items still to come. Lower bound: 1 if the
    /// inner iterator certainly has an item left, otherwise 0. Upper bound: the
    /// inner upper bound, because every inner item may start a run.
    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!("Task 6: implement the documented contract")
    }
}

/// The extension trait. Its one method becomes available on every iterator
/// through the blanket impl below.
trait DedupRunsExt: Iterator + Sized {
    /// G. Wrap `self` in a `DedupRuns`. Do not pull any item yet.
    fn dedup_runs(self) -> DedupRuns<Self>
    where
        Self::Item: PartialEq,
    {
        todo!("Task 7: implement the documented contract")
    }
}
impl<I: Iterator> DedupRunsExt for I {}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, guess, within};
    use std::cell::Cell;

    #[test]
    fn a_long_words_filters_by_chars_and_keeps_order() {
        let got: Vec<&str> = long_words("a quick  brown fox", 3).collect();
        assert_eq!(got, ["quick", "brown"]);
        assert_eq!(
            long_words("éé ééé", 2).collect::<Vec<_>>(),
            ["ééé"],
            "count chars, not bytes: éé is two chars in four bytes"
        );
        assert_eq!(long_words("   ", 0).count(), 0);
    }
    #[test]
    fn a_long_words_is_a_lazy_pipeline() {
        let text = "one three five seven nine eleven ".repeat(50);
        let (n, allocations) = count(|| long_words(&text, 4).count());
        assert_eq!(n, 150);
        assert_eq!(
            allocations, 0,
            "long_words made {allocations} heap allocation(s); return the adapter \
             chain itself instead of a Vec turned back into an iterator"
        );
    }

    #[test]
    fn b_ordered_runs_in_either_direction() {
        let v = [1, 2, 3];
        assert_eq!(ordered(&v, false).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(ordered(&v, true).collect::<Vec<_>>(), [3, 2, 1]);
        assert_eq!(ordered(&[], true).count(), 0);
    }
    #[test]
    fn b_ordered_boxes_the_iterator_not_a_copy_of_the_data() {
        let v: Vec<i32> = (0..1000).collect();
        let (total, allocations) = count(|| ordered(&v, true).map(i64::from).sum::<i64>());
        assert_eq!(total, 499_500);
        assert!(
            allocations <= 1,
            "one Box is the only allocation that ordered needs; it made {allocations}. \
             Box the borrowing iterator instead of collecting the values first"
        );
    }

    #[test]
    fn c_pairs_walk_from_the_front_and_ignore_an_unpaired_element() {
        // `take(10)` bounds each check, so a `next` that never ends cannot hang.
        let v = [1, 2, 3, 4, 5];
        let got: Vec<(&i32, &i32)> = pairs(&v).take(10).collect();
        assert_eq!(got, [(&1, &2), (&3, &4)], "5 has no partner");
        assert_eq!(pairs::<i32>(&[]).take(10).count(), 0);
        assert_eq!(pairs(&[7]).take(10).count(), 0);
    }
    #[test]
    fn c_pairs_borrow_the_original_elements() {
        let v = vec![String::from("left"), String::from("right")];
        let (pair, allocations) = count(|| pairs(&v).next());
        let (left, right) = pair.expect("two elements make one pair");
        assert!(
            std::ptr::eq(left, &v[0]) && std::ptr::eq(right, &v[1]),
            "the items must be references into the slice, not references to copies"
        );
        assert_eq!(
            allocations, 0,
            "walking a slice needs no heap allocation; pairs made {allocations}"
        );
    }
    #[test]
    fn c_pairs_from_the_back_are_the_same_pairs_reversed() {
        let v = [1, 2, 3, 4, 5];
        let got: Vec<(&i32, &i32)> = pairs(&v).rev().take(10).collect();
        assert_eq!(
            got,
            [(&3, &4), (&1, &2)],
            "5 has no partner from either end: next_back must not yield (4, 5)"
        );
    }
    #[test]
    fn c_both_ends_share_one_remaining_range_and_len_stays_exact() {
        let v = [1, 2, 3, 4, 5, 6, 7];
        let mut it = pairs(&v);
        assert_eq!(it.len(), 3, "seven elements make three complete pairs");
        assert_eq!(it.next_back(), Some((&5, &6)));
        assert_eq!(it.len(), 2, "len shrinks after next_back");
        assert_eq!(it.next(), Some((&1, &2)));
        assert_eq!(
            it.size_hint(),
            (1, Some(1)),
            "both bounds of size_hint must equal the number of pairs left"
        );
        assert_eq!(it.next_back(), Some((&3, &4)));
        assert_eq!(
            it.next(),
            None,
            "the back already took (3, 4); the front must not yield it again"
        );
        assert_eq!(it.next_back(), None);
        assert_eq!(it.len(), 0);
    }
    #[test]
    fn d_pairs_returns_your_own_type() {
        let name = std::any::type_name_of_val(&pairs(&[1, 2]));
        assert!(
            name.contains("Pairs"),
            "pairs must return your Pairs type, not a chain of std adapters; \
             it returned {name}"
        );
    }

    // The DedupRuns tests run inside `within`: a `next` that peeks but never
    // advances would otherwise loop forever. Allocations are counted per
    // thread, so `count` is called inside the closure.
    const DEDUP: &str = "dedup_runs, which must advance the inner iterator in every loop turn,";
    #[test]
    fn e_runs_collapse_to_their_first_item() {
        let (numbers, letters, none, one) = within(5, DEDUP, || {
            (
                [1, 1, 2, 2, 2, 1]
                    .into_iter()
                    .dedup_runs()
                    .take(20)
                    .collect::<Vec<i32>>(),
                "aabbbcaa".chars().dedup_runs().take(20).collect::<String>(),
                std::iter::empty::<u8>().dedup_runs().take(20).count(),
                std::iter::once(9).dedup_runs().take(20).count(),
            )
        });
        assert_eq!(
            numbers,
            [1, 2, 1],
            "equal items are dropped only when they are direct neighbours"
        );
        assert_eq!(letters, "abca");
        assert_eq!((none, one), (0, 1));
    }
    #[test]
    fn e_the_first_item_of_a_run_is_moved_out_not_cloned() {
        let (kept, first_is_original, last_is_original, allocations) = within(5, DEDUP, || {
            let words = vec![String::from("x"), String::from("x"), String::from("y")];
            let (first, last) = (words[0].as_ptr(), words[2].as_ptr());
            let (items, allocations) = count(|| {
                let mut it = words.into_iter().dedup_runs();
                [it.next(), it.next(), it.next()]
            });
            let first_is_original = items[0].as_ref().is_some_and(|s| s.as_ptr() == first);
            let last_is_original = items[1].as_ref().is_some_and(|s| s.as_ptr() == last);
            (items, first_is_original, last_is_original, allocations)
        });
        assert_eq!(
            kept,
            [Some("x".to_string()), Some("y".to_string()), None],
            "x, x, y has two runs"
        );
        assert!(
            first_is_original && last_is_original,
            "yield the first String of each run itself (same heap buffer), not a clone \
             and not a later duplicate"
        );
        assert_eq!(
            allocations, 0,
            "the adapter made {allocations} heap allocation(s); it needs no clone and \
             no buffer"
        );
    }
    #[test]
    fn e_dedup_runs_pulls_only_what_it_needs() {
        let (before, first, after_first, more, after_more) = within(5, DEDUP, || {
            let pulled = Cell::new(0);
            // 0, 0, 0, 1, 1, 1, 2, 2, 2, ...
            let source = (0..3000)
                .map(|n| n / 3)
                .inspect(|_| pulled.set(pulled.get() + 1));
            let mut it = source.dedup_runs();
            let before = pulled.get();
            let first = it.next();
            let after_first = pulled.get();
            let more: Vec<i32> = it.by_ref().take(3).collect();
            (before, first, after_first, more, pulled.get())
        });
        assert_eq!(
            before, 0,
            "creating the adapter must not pull any item; it pulled {before}"
        );
        assert_eq!(first, Some(0));
        assert!(
            after_first <= 4,
            "the first run is 0, 0, 0; one more item shows that it has ended, so 4 \
             items are enough, but {after_first} were pulled"
        );
        assert_eq!(more, [1, 2, 3]);
        assert!(
            after_more <= 13,
            "four runs of three items plus one look ahead is 13 items; {after_more} \
             were pulled"
        );
    }
    #[test]
    fn f_size_hint_never_lies() {
        let (three_equal, empty, after_one, filtered) = within(5, DEDUP, || {
            let mut partly = [1, 1, 2].into_iter().dedup_runs();
            partly.next();
            (
                [7, 7, 7].into_iter().dedup_runs().size_hint(),
                std::iter::empty::<i32>().dedup_runs().size_hint(),
                partly.size_hint(),
                (0..10).filter(|n| n % 2 == 0).dedup_runs().size_hint(),
            )
        });
        assert_eq!(
            three_equal,
            (1, Some(3)),
            "three inner items give at least 1 and at most 3 items: they may all be equal"
        );
        assert_eq!(empty, (0, Some(0)));
        assert_eq!(
            after_one,
            (1, Some(1)),
            "after the run of 1s, the 2 has not been yielded yet and must be counted"
        );
        assert_eq!(
            filtered,
            (0, Some(10)),
            "an inner lower bound of 0 stays 0; the inner upper bound is kept"
        );
    }
    #[test]
    fn g_the_method_chains_like_a_std_adapter() {
        let (shout, changes, allocations) = within(5, DEDUP, || {
            let shout: String = "mississippi"
                .chars()
                .dedup_runs()
                .map(|c| c.to_ascii_uppercase())
                .take(20)
                .collect();
            let readings = [3, 3, 3, 4, 4, 3];
            let (changes, allocations) = count(|| readings.iter().dedup_runs().take(20).count());
            (shout, changes, allocations)
        });
        assert_eq!(shout, "MISISIPI");
        assert_eq!(changes, 3, "the runs are 3, 4, 3");
        assert_eq!(
            allocations, 0,
            "dedup_runs over a slice iterator needs no heap allocation; it made \
             {allocations}"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. The value of `log`: (right after the call, after one next(), at the end).
        fn traced(log: &Cell<u32>, items: &[u32]) -> impl Iterator<Item = u32> {
            log.set(log.get() + 100);
            items.iter().map(move |&item| {
                log.set(log.get() + 1);
                item
            })
        }
        let log = Cell::new(0);
        let mut it = traced(&log, &[5, 6, 7]);
        let after_call = log.get();
        it.next();
        let after_one = log.get();
        let rest: u32 = it.sum();
        let p1: (u32, u32, u32) = guess();
        assert_eq!(
            p1,
            (after_call, after_one, log.get()),
            "P1: log (after the call, after one next, after the sum)"
        );
        assert_eq!(rest, 13);

        // P2. A slice iterator behind map: (next_back, next, len afterwards).
        let mut tens = [1, 2, 3, 4].iter().map(|x| x * 10);
        let ends = (tens.next_back(), tens.next(), tens.len());
        let p2: (Option<i32>, Option<i32>, usize) = guess();
        assert_eq!(p2, ends, "P2: (next_back, next, len afterwards)");

        // P3. Five values in chunks of two, starting from the back:
        // (chunks(2).next_back(), rchunks(2).next()).
        let v = [1, 2, 3, 4, 5];
        let from_front = v.chunks(2).next_back();
        let from_back = v.rchunks(2).next();
        let p3: (Option<&[i32]>, Option<&[i32]>) = guess();
        assert_eq!(
            p3,
            (from_front, from_back),
            "P3: (chunks(2).next_back(), rchunks(2).next())"
        );
    }
}
