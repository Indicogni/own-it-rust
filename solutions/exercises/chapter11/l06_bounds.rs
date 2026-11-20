//! LESSON 06 — Opaque results, lifetimes, and precise capture
//! Run: cargo test --test chapter11 l06_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does impl Iterator permit returning two
//! different iterator types from branches? Which inputs may the opaque result borrow?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Explain every bound in these signatures. Why does use<'a> list
//! captures while + 'a expresses an outlives requirement?
//!
//! COMPILER / MANUAL LAB (use examples/ch11_scratch.rs):
//! Copy your finished words into the scratch file. In main, create the scratch String in
//! an inner block, call words there, and consume the iterator after the block ends.
//! Then remove `+ use<'a>` and read the error: which borrow does the iterator now hold?
//! Separately remove Clone from duplicated and use the diagnostic to rebuild the
//! smallest bound that works.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, lab, and explanation are done.

/// A. Return borrowed whitespace-separated words; precisely capture only 'a.
/// (The unnamed lifetime of `_scratch` is in scope too; `use<'a>` leaves it out.)
fn words<'a>(text: &'a str, _scratch: &str) -> impl Iterator<Item = &'a str> + use<'a> {
    text.split_whitespace()
}
/// B. Return a closure that borrows prefix. Neither building it nor calling it may
/// allocate.
fn starts_with<'a>(prefix: &'a str) -> impl Fn(&str) -> bool + 'a {
    // `prefix` is a `&'a str`, which is `Copy`. `move` copies that reference into
    // the closure, so the closure holds a pointer and a length and nothing else.
    move |text: &str| text.starts_with(prefix)
}
/// C. Return two independently owned T values using only the necessary Clone bound.
/// Clone exactly once.
fn duplicated<T: Clone>(value: T) -> (T, T) {
    // One clone is enough: the original moves into the second position.
    (value.clone(), value)
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::cell::Cell;
    use std::hint::black_box;
    use std::rc::Rc;
    #[test]
    fn a_precise_capture_does_not_borrow_scratch() {
        let text = String::from(" air\tsea\n🦀 ");
        let iter;
        {
            let temporary = String::from("scratch");
            iter = words(&text, &temporary);
        }
        assert_eq!(iter.collect::<Vec<_>>(), ["air", "sea", "🦀"]);
        assert_eq!(words("", "ignored").count(), 0);
    }
    #[test]
    fn b_opaque_closure_borrows_the_prefix() {
        let prefix = String::from("α");
        let p = starts_with(&prefix);
        assert!(p("αβ"));
        assert!(!p("a"));
        assert!(starts_with("")("anything"));
    }
    #[test]
    fn c_clone_bound_supports_non_copy_values() {
        let (mut a, b) = duplicated(vec![1, 2]);
        a.push(3);
        assert_eq!(b, [1, 2]);
    }

    #[test]
    fn a_words_split_on_unicode_whitespace() {
        let text = "a\u{2003}b\u{3000}c";
        assert_eq!(words(text, "").collect::<Vec<_>>(), ["a", "b", "c"]);
        let first = words(text, "").next().unwrap();
        assert_eq!(
            first.as_ptr(),
            text.as_ptr(),
            "each word must be a slice of text itself, not a copy of it"
        );
    }

    #[test]
    fn b_predicate_does_not_allocate_per_call() {
        let prefix = String::from("ab");
        let p = starts_with(&prefix);
        let (hits, allocations) = count(|| {
            ["abc", "xab", "ab", ""]
                .into_iter()
                .filter(|t| p(black_box(*t)))
                .count()
        });
        assert_eq!(hits, 2);
        assert_eq!(
            allocations, 0,
            "calling the closure must not allocate: compare against the borrowed prefix"
        );
    }

    #[test]
    fn b_predicate_borrows_the_prefix_instead_of_owning_a_copy() {
        let prefix = String::from("a rather long prefix that would need the heap");
        let (p, allocations) = count(|| starts_with(black_box(&prefix)));
        assert!(p("a rather long prefix that would need the heap, and more"));
        assert_eq!(
            allocations, 0,
            "building the closure must not copy prefix to the heap: capture the &str"
        );
        assert_eq!(
            size_of_val(&p),
            size_of::<&str>(),
            "the closure should hold exactly one &str (a pointer and a length)"
        );
    }

    #[test]
    fn c_duplicated_clones_exactly_once() {
        // Every clone of a Tally adds one to the shared counter.
        struct Tally<'a>(&'a Cell<u32>);
        impl Clone for Tally<'_> {
            fn clone(&self) -> Self {
                self.0.set(self.0.get() + 1);
                Tally(self.0)
            }
        }
        let clones = Cell::new(0);
        let _pair = duplicated(Tally(&clones));
        assert_eq!(
            clones.get(),
            1,
            "two owned values need one clone: the original can be moved into the pair"
        );
    }

    fn twice<T: Clone>(value: &T) -> (T, T) {
        (value.clone(), value.clone())
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. A returned closure is a value made of its captures. Sizes in machine words?
        let prefix: &str = "ab";
        let none = |t: &str| t.is_empty();
        let borrowing = move |t: &str| t.starts_with(prefix);
        let p1: (usize, usize) = (0, 2);
        assert_eq!(
            (p1.0 * size_of::<usize>(), p1.1 * size_of::<usize>()),
            (size_of_val(&none), size_of_val(&borrowing)),
            "P1: closure sizes in machine words (no captures, one &str capture)"
        );

        // P2. `twice` (above) needs T: Clone. First T is an Rc; `a` and `b` are still
        // alive when the count is read. Then T is the reference type &String.
        let shared = Rc::new(vec![1, 2, 3]);
        let (a, b) = twice(&shared);
        let text = String::from("owned");
        let borrowed: &String = &text;
        let (_, allocations) = count(|| twice(black_box(&borrowed)));
        let p2: (usize, usize) = (3, 0);
        assert_eq!(
            p2,
            (Rc::strong_count(&shared), allocations),
            "P2: (Rc strong count, heap allocations when T is a reference)"
        );
        drop((a, b));
    }
}
