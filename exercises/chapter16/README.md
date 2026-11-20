# Chapter 16 — Standard collections

[All chapters](../../README.md) · [Previous chapter](../chapter15/README.md) · [Next chapter](../chapter17/README.md)

Almost every program keeps many values of the same kind: lines, readings, users, tasks. A *collection* owns such a group, and the choice of collection decides what is cheap and what is slow. In this chapter you work with the standard collections one by one: the growable array `Vec`, the double-ended queue `VecDeque`, the priority queue `BinaryHeap`, the maps `HashMap` and `BTreeMap`, and the sets `HashSet` and `BTreeSet`. You edit vectors in place, and you learn why the compiler refuses a change while a reference into the collection is alive. You sort and search, keep the best few values of a long stream, count with one lookup per item, and make output that does not depend on hash order. You make `Eq` and `Hash` agree, and you write a small hash function and plug it into a map. In the capstone you design a scheduler that needs several collections at once, and you decide which one holds the truth.

**Before you start:** Chapters 1 to 15; iterators (Chapter 15) and the traits `Ord`, `Eq`, `Hash` and `Borrow` (Chapters 11 and 13) are used all the time. **Time:** about 10 to 11 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. Sessions of 45 to 60 minutes work well.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example, and shows the compiler errors you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run the lesson, for example `cargo test --test chapter16 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations, compare buffer addresses, or count how often a key is hashed or compared. A solution that copies into a new collection where the contract says *in place*, or that looks a key up twice, fails, and the failure message says what was expected. Then run `cargo clippy --test chapter16` and fix the warnings in code that you wrote. Warnings about functions that still contain `todo!()` disappear when you implement them; in particular, ignore the advice to change `&mut Vec<i32>` into a slice on such a function. Never change a function's signature unless its doc comment says that you may.
5. **Drill the compiler.** `cargo drill ch16` shows ten small programs; for each one, predict whether it compiles (and with which error code), runs, or panics; then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

After the numbered lessons, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: repair a teammate's code that works but is written badly). Run `cargo progress` to see where you are and which chapters are due for spaced review.

Open [HINTS.md](HINTS.md) one hint at a time, and only after a real attempt. Passing tests alone does not demonstrate mastery.

| Done | Lesson | Check |
| --- | --- | --- |
| [ ] | [01 · Vector access and structural editing](l01_vectors.rs) | `cargo test --test chapter16 l01_` |
| [ ] | [02 · Growth, removal, and borrow validity](l02_validity.rs) | `cargo test --test chapter16 l02_` |
| [ ] | [03 · Sorting, binary search, and slices](l03_sort_search.rs) | `cargo test --test chapter16 l03_` |
| [ ] | [04 · Queues and ring buffers](l04_deques.rs) | `cargo test --test chapter16 l04_` |
| [ ] | [05 · Priority queues and heap ordering](l05_heaps.rs) | `cargo test --test chapter16 l05_` |
| [ ] | [06 · Hash maps, tree maps, and entries](l06_maps.rs) | `cargo test --test chapter16 l06_` |
| [ ] | [07 · Set algebra and equal-but-distinct values](l07_sets.rs) | `cargo test --test chapter16 l07_` |
| [ ] | [08 · Hashing and custom builders](l08_hashing.rs) | `cargo test --test chapter16 l08_` |
| [ ] | [09 · Capstone: a task scheduler and audit](l09_capstone.rs) | `cargo test --test chapter16 l09_` |
| [ ] | [R01 · Review: Chapters 7, 10, 13–15 from memory](r01_review.rs) | `cargo test --test chapter16 r01_` |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) | `cargo test --test chapter16 c01_` |
| [ ] | [Compiler drills](../../drills/ch16/) | `cargo drill ch16` |

Run commands from the repository root:

```sh
cargo test --test chapter16 l01_
cargo test --test chapter16 -- --list
cargo test --test chapter16
cargo test --release --test chapter16
cargo clippy --test chapter16
cargo drill ch16
cargo run --example ch16_scratch
```

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, most tests fail because the supplied code has bugs. Keep the supplied tests as they are. Every task in the table uses only the standard library and needs no network. A test that could wait forever because of a loop that never ends stops after some seconds with a message instead. Compiler experiments belong in [the scratch example](../../examples/ch16_scratch.rs); leave it compiling when you finish.

## Topic map

"Optional lab" means an experiment that you run by hand in the scratch example; the lesson header describes it, and no test checks it. The random lab is optional too, but it is a package with its own tests.

| Topic | Where you practise it |
| --- | --- |
| Reading an element or a range without a panic (`get`) | Lesson 01 task A and prediction P3 |
| Length and capacity: `reserve`, `with_capacity`, `clear` | Lesson 01 P1; lesson 02 P2 and P3; optional lab in lesson 01 |
| Editing a vector's structure: `split_off`, `swap`, `fill`, `reverse` | Lesson 01 tasks B and C |
| Why a vector cannot change while a reference into it is alive | Drill d01; optional lab in lesson 02 |
| Two mutable references into one slice: `split_at_mut` and `get_disjoint_mut` | Lesson 02 task A and P4; drill d05 |
| Removing in place: `retain`, `swap_remove`, `drain`, `dedup` | Lesson 02 tasks B and C, P1; lesson 03 task C and P3; drill d08 |
| Stable and unstable sorting; sorting by a key or by several fields | Lesson 03 task A; drill d06; R01 task A; C01 |
| Sorting floating-point numbers | Drill d03 |
| Binary search and the exact range of equal elements | Lesson 03 task B and P1 |
| Comparing slices | Lesson 03 P2 |
| `VecDeque` as a queue and as a bounded history; rotation; `make_contiguous` | Lesson 04 |
| `BinaryHeap`: which element is on top, `Reverse`, ties through tuples, `peek` before `pop` | Lesson 05; drill d07 |
| Keeping the k largest values of a long stream in a small heap | Lesson 05 tasks A and B (allocation tests) |
| Counting with the entry API, one lookup per item | Lesson 06 task A (a test counts hash computations); R01 task A; C01 |
| Telling a vacant entry from an occupied one | R01 task C |
| Output that does not depend on hash order | Lesson 06 task B; R01 task A; C01 |
| `BTreeMap`: key order and range queries | Lesson 06 task C and P3; R01 task B; capstone |
| Looking up a `String` key with a `&str`, and why `Vec::contains` does not accept one | Lesson 06 P4; drills d09 and d10 |
| What a map needs from its key type, and what indexing a map can do | Drills d02 and d04 |
| Set operations and set relations | Lesson 07 task A and P3 |
| Equal but not identical values: `insert` and `replace`; `Hash` in agreement with `Eq` | Lesson 07 tasks B and C, P1 and P2 |
| Writing a `Hasher`, using it through `BuildHasherDefault`, and what collisions do | Lesson 08 |
| A design with several collections: a source of truth, removal from a priority order, a bounded history, an ordered audit | Lesson 09 (capstone) |
| Finding wrong output order, an off-by-one limit, needless clones, double lookups and a repeated linear search in existing code | C01 |
| Recall of earlier chapters: lazy iterators, closures as parameters, error enums, slice patterns | R01 |
| Views of a slice in pieces (`windows`, `chunks`) and joining (`concat`, `join`) | Shown in CONCEPTS, lesson 01. Not exercised in this chapter. |
| Wrap-around in a `VecDeque` (`as_slices`), `peek_mut`, a rolling hasher, when to look for a collection outside the standard library | Optional labs in the headers of lessons 04, 05, 08 and 09 |
| Random choice, shuffling and sampling with the `rand` crate; testing code that uses random numbers | Optional lab: [random-lab](random-lab/README.md) |

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Choose between `Vec`, `VecDeque`, `BinaryHeap`, `HashMap`, `BTreeMap` and the two set types for a task, from the operations that the task needs and what they cost.
- [ ] Say which operations keep the order of elements, which may move the elements to a new buffer, and why the compiler rejects a change to a collection while a reference into it is alive.
- [ ] Get two mutable references into one slice without `unsafe`, in two different ways.
- [ ] Sort by several fields, explain stable versus unstable sorting, and find the exact range of equal elements in a sorted slice.
- [ ] Update a map with one lookup, and produce the same output from a `HashMap` on every run.
- [ ] State the rule that connects `Eq` and `Hash`, and say what `insert` and `replace` do with a value that equals a stored one.
- [ ] Explain what `Hash`, `Hasher` and `BuildHasher` each do, why a collision cannot make a map wrong, and when a weak hash function is dangerous.
- [ ] Redo the capstone after a break without looking at your first solution. Then explain what changes in your design if a second `submit` may also lower a task's priority.

## Optional lab: random elements

The standard library has no random number generator. [random-lab](random-lab/README.md) is a small separate package that uses the `rand` crate to choose, shuffle and sample. Do it after lesson 03 if you like. It has its own manifest and commands, so `cargo test --test chapter16` does not run it. Downloading its pinned dependency needs network access once; everything else in this chapter works offline.

## Solutions

Model solutions are in [solutions/exercises/chapter16/](../../solutions/exercises/chapter16/) (one file per lesson with the predictions filled in, the repaired code review, and the solved random lab) and [solutions/drills/ch16/](../../solutions/drills/ch16/) (each drill repaired, plus the expected outcome of every drill). Use them after an honest attempt and after the hints: compare your solution with the model, and note in NOTES.md what differs and why. Reading a solution first removes the struggle that makes the lesson stay in memory. For the capstone the model is one of several good designs; a different choice of collections that passes the tests and meets the cost limits in the contract is just as correct.

## Next

Take checkpoint 3 (`checkpoints/cp3_after_ch16`) at least two days after you finish this chapter. It tests chapters 1 to 16 without hints. Read the [checkpoint guide](../../checkpoints/README.md#when-to-take-one) first, and open the checkpoint's `SPEC.md` only when you start the timer. Until then, continue with [chapter 17](../chapter17/README.md).
