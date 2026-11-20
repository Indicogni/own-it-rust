# Chapter 15 — Iterators and adapters

[All chapters](../../README.md) · [Previous chapter](../chapter14/README.md) · [Next chapter](../chapter16/README.md)

Most Rust code that touches more than one value uses an iterator: a `for` loop, a search, a sum, a conversion from one collection into another. This chapter starts with the one method that every iterator has, `next`, and builds up from there. You learn the ways to iterate over a collection and what each one does to ownership. You combine adapters into pipelines that do no work until a consumer asks, and you learn to predict how many items such a pipeline reads. At the end you write iterators yourself: types that can be walked from both ends, functions that return iterators, and a new adapter method that works on every iterator.

This chapter has **12 lessons with 42 implementation tasks**, a review lesson, a code-review lesson, and 10 compiler drills, with **84 automated checks**. The teaching text for every lesson is in [CONCEPTS.md](CONCEPTS.md).

The starter functions contain `todo!()`: they compile, and their tests fail until you implement the contracts. Keep the supplied tests intact. In most lessons the types and signatures are given, and you write the bodies. In Lesson 12 you also design a type and write its trait impls from nothing.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter15 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations, compare pointers, or count how many items your code pulls from its input. A solution that collects intermediate vectors, clones items, or reads more input than it needs fails them. Then run `cargo clippy --test chapter15` and fix warnings in your code.
5. **Drill the compiler.** `cargo drill ch15` shows small programs; predict whether each compiles (and which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your predictions* and *Check your explanation* in [HINTS.md](HINTS.md). The lab in each lesson header is optional extra practice in [the scratch example](../../examples/ch15_scratch.rs).

Do the lessons in the order of the table: Lesson 12 comes before the capstone, which uses it. After the numbered lessons, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: fix a teammate's working-but-wrong code). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson | Check |
| --- | --- | --- |
| [ ] | [01 · Borrowed, mutable, owned, and draining sources](l01_sources.rs) | `cargo test --test chapter15 l01_` |
| [ ] | [02 · Generating finite sequences](l02_generated.rs) | `cargo test --test chapter15 l02_` |
| [ ] | [03 · Mapping, filtering, and flattening](l03_mapping.rs) | `cargo test --test chapter15 l03_` |
| [ ] | [04 · Stopping, peeking, and resuming](l04_boundaries.rs) | `cargo test --test chapter15 l04_` |
| [ ] | [05 · Reversal, pairing, inspection, and repetition](l05_composition.rs) | `cargo test --test chapter15 l05_` |
| [ ] | [06 · Accumulators, extrema, and sequence comparisons](l06_aggregation.rs) | `cargo test --test chapter15 l06_` |
| [ ] | [07 · Searching and iterator positions](l07_search.rs) | `cargo test --test chapter15 l07_` |
| [ ] | [08 · Folds, short circuits, and side effects](l08_folds.rs) | `cargo test --test chapter15 l08_` |
| [ ] | [09 · Collections, extension, and partition](l09_collecting.rs) | `cargo test --test chapter15 l09_` |
| [ ] | [10 · Implementing a double-ended iterator](l10_custom.rs) | `cargo test --test chapter15 l10_` |
| [ ] | [12 · Writing iterators: return types, both ends, and your own adapter](l12_writing.rs) | `cargo test --test chapter15 l12_` |
| [ ] | [11 · Capstone: a lazy measurement report](l11_capstone.rs) | `cargo test --test chapter15 l11_` |
| [ ] | [R01 · Review: Chapters 10–14 from memory](r01_review.rs) | `cargo test --test chapter15 r01_` |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) | `cargo test --test chapter15 c01_` |
| [ ] | [Compiler drills](../../drills/ch15/) | `cargo drill ch15` |

Run commands from the repository root:

```sh
cargo test --test chapter15 l01_
cargo test --test chapter15 -- --list
cargo test --test chapter15
cargo test --release --test chapter15
cargo clippy --test chapter15
cargo drill ch15
cargo run --example ch15_scratch
```

The starters compile; tests fail intentionally at `todo!()` and `guess()` until you fill them in. Keep the supplied tests intact. Use [progressive hints](HINTS.md) only after a first attempt; passing tests alone does not demonstrate mastery.

Some optional labs deliberately fail to compile or panic. Keep them in the scratch example and restore a compiling example afterward. Do not put intentional compilation failures in the test modules. Tests are deterministic and need only the standard library. A test that could run forever with a wrong solution is bounded with `take`, or runs under a time limit and fails with a message.

## Topic map

"Optional lab" means an unchecked experiment that a lesson header suggests for the scratch example. Everything else is checked by tests or drills.

| Topic | Where you practise it |
| --- | --- |
| `next` and `IntoIterator`; `iter`, `iter_mut`, `into_iter`; `drain` | 01 |
| Laziness and pipelines that do not allocate | 01 (technique test, prediction), 03 (prediction), drill d01 |
| `from_fn` and `successors`; ending a generated sequence; checked arithmetic at the last step | 02 |
| `repeat_n`, `once`, bounding an endless source | 02 optional lab |
| `map`, `filter`, `filter_map`, `flat_map`, `flatten` | 03 |
| `skip_while`, `take_while`, `take`, `by_ref`; `peekable` with `peek` or `next_if` | 04 |
| `fuse` | 04 (prediction and a supplied test); writing a source that needs it is an optional lab |
| `enumerate`, `rev`, `zip`, `cycle`, `chain`, `copied`, `cloned`, `inspect` | 05 |
| `sum`, `product`, widening, `min_by_key`, `any`, `all`, `Iterator::cmp` | 06 |
| `min`, `max`, `max_by_key` and ties; `max_by` with your own comparison | 06 (predictions); optional lab |
| `position`, `rposition`, `find_map`, `nth`, `nth_back`, `rfind` | 07 |
| `fold`, `try_fold`, `try_for_each`; what is left in an iterator after an early stop | 08 |
| `rfold` and `try_rfold` | 08 (prediction for `rfold`); optional lab |
| `collect` into `Result`, `partition`, `extend` and `size_hint` | 09 |
| Collecting into sets and strings; implementing `Extend` | 09 (prediction); optional lab |
| Implementing `Iterator`, `size_hint`, `DoubleEndedIterator`; the promises of `ExactSizeIterator` and `FusedIterator` | 10 |
| `IntoIterator` for your own collection; `FromIterator` for your own type | worked examples in CONCEPTS.md; 10 optional lab |
| Returning `impl Iterator`; `Box<dyn Iterator>` when the types differ | 12 (also 02 and 11) |
| Designing a borrowing iterator with a lifetime and writing all of its impls | 12 |
| A custom adapter, its `size_hint`, and an extension trait | 12 |
| One lazy pipeline over a generic source, with a limit on how much input is read | 11 capstone |
| Stateful `move` closures in adapters; `TryFrom` with `collect`; `try_fold` with enum patterns | R01 |
| Reviewing iterator code: needless collections, index loops, missing early exit | C01 |
| Reading iterator diagnostics: laziness, item types, missing traits, borrows during iteration | [drills](../../drills/ch15/) d01–d10 |

Not in this chapter: iterators whose items borrow from the iterator itself (lending iterators) are in the mastery track, [`mastery/m1_types`](../../mastery/m1_types/README.md). Measuring and tuning iterator code is in [`mastery/m7_perf`](../../mastery/m7_perf/README.md).

## Time

About 11 to 13 hours in total, as an estimate: 35 to 50 minutes for each of Lessons 01–10, about 90 minutes for Lesson 12, about an hour for the capstone, and about two hours for the review, the code review, and the drills together. [CONCEPTS.md](CONCEPTS.md) lists an estimate for each lesson.

## Solutions

Model answers are in [solutions/exercises/chapter15/](../../solutions/exercises/chapter15/), one file for each lesson, and the fixed drills are in [solutions/drills/ch15/](../../solutions/drills/ch15/). Open a solution only after an honest attempt and after the hints: reading an answer feels like learning, but writing one is what makes the knowledge stay. When your tests pass, compare your code with the model answer and note one difference in NOTES.md.

## Mastery check

Model answers are at the end of [HINTS.md](HINTS.md#mastery-check-model-answers). Write your own first.

- [ ] Explain laziness and the precise ownership of Item for iter, iter_mut, and into_iter.
- [ ] Choose adapters and consumers while predicting short-circuit and consumption behavior.
- [ ] Implement Iterator, DoubleEndedIterator, ExactSizeIterator, and FusedIterator consistently.
- [ ] Use by_ref and peekable without accidentally consuming delimiters.
- [ ] Explain why iterator pipelines need not allocate intermediate collections.
- [ ] Choose between returning impl Iterator, a boxed trait object, and a named type, and add an adapter method to every iterator.
- [ ] Redo the capstone from its contract after a few days without consulting your earlier implementation.
- [ ] Present your predictions, failed compiler experiments, and one new edge case for review.
