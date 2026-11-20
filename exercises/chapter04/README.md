# Chapter 4 — Ownership and moves

[All chapters](../../README.md) · [Previous chapter](../chapter03/README.md) · [Next chapter](../chapter05/README.md)

Rust frees memory without a garbage collector and without asking you to free it by hand. It can do this because every value has exactly one owner, and the compiler knows at every line who that owner is. In this chapter you learn to see ownership as the compiler sees it: when a value is created, when it changes hands, and when it is destroyed. You move values through functions, branches, loops and vectors. You take values out of places that must never be left empty. You choose between a move, a copy and a clone, and you share one value among several owners with `Rc` and `Arc`. Many of the compiler errors that people meet in their first weeks of Rust come from these rules, so the chapter also trains you to read those errors and to repair the cause.

**Before you start:** Chapters 1 to 3. **Time:** about 7 to 8 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. Sessions of 45 to 60 minutes work well.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example, and shows the compiler errors you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run the lesson, for example `cargo test --test chapter04 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations or compare pointers, so a solution that clones everything fails, and the failure message says what was expected. Then run `cargo clippy --test chapter04` and fix the warnings in code that you wrote. Warnings about functions that still contain `todo!()` disappear when you implement them. Never change a function's signature unless its doc comment says that you may.
5. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

After lesson 07, **drill the compiler**: `cargo drill ch04` shows nine small programs that mix lessons 01 to 07. For each one, predict whether it compiles (and with which error code), runs, or panics; then fix it. After that, do the **capstone**, the **review** lesson (r01: Chapter 3, from memory) and the **code review** lesson (c01: repair code that works but is written badly). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Ownership scopes](l01_scope.rs) |
| [ ] | [02 · Moves across assignments and fields](l02_move_paths.rs) |
| [ ] | [03 · Moves and control flow](l03_control_flow.rs) |
| [ ] | [04 · Moving from indexed collections](l04_indexed.rs) |
| [ ] | [05 · Leaving a replacement behind](l05_replace.rs) |
| [ ] | [06 · Copy and explicit cloning](l06_copy_clone.rs) |
| [ ] | [07 · Rc and Arc](l07_shared.rs) |
| [ ] | [Compiler drills](../../drills/ch04/) — `cargo drill ch04` |
| [ ] | [08 · Capstone: owned dispatch queue](l08_capstone.rs) |
| [ ] | [R01 · Review: Chapter 3 from memory](r01_review.rs) |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) |

```sh
cargo test --test chapter04 l01_
cargo test --test chapter04 -- --list
cargo test --test chapter04
cargo test --release --test chapter04
cargo clippy --test chapter04
cargo drill ch04
cargo run --example ch04_scratch
```

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, most tests fail because the supplied code has bugs. Keep the supplied tests as they are. Every task uses only the standard library and needs no network. Compiler experiments belong in [the scratch example](../../examples/ch04_scratch.rs); leave it compiling when you finish.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

## Topic map

"Optional lab" means an experiment that you run by hand in the scratch example. No test checks it.

| Topic | Where you practise it |
| --- | --- |
| One owner per value, scopes, and the order in which values are dropped | Lesson 01 |
| What a move does to the header and to the heap buffer | Lessons 01 and 02 (pointer and allocation predictions) |
| Moves through assignment, function arguments, return values and `push` | Lesson 02; drill d01 |
| Moving one field out of a tuple, and taking a tuple apart with a pattern | Lesson 02; drill d05 |
| Moves inside branches and loops; giving a moved variable a new value | Lesson 03; drills d03 and d04 |
| Drop flags: what is dropped when a move happens on only one path | Lesson 03 (predictions) |
| Taking elements out of a vector with `remove` and `swap_remove`; index checks at run time | Lesson 04; drills d02 and d08; R01 task C |
| Exchanging a value through `&mut` with `mem::replace`, `mem::take` and `Option::take` | Lesson 05 |
| `Copy` types, deriving `Copy`, and the cost of an explicit `clone` | Lesson 06; drill d06 |
| `Rc`: shared ownership, strong counts, read-only access | Lesson 07 task A; drill d07; capstone |
| `Arc`: giving a second owner to another thread | Lesson 07 task B; drill d09 |
| Choosing between move, clone and share in one small design | Lesson 08 (capstone) |
| Finding needless clones and poor parameter types in existing code | C01 |
| Recall of Chapter 3: overflow, UTF-8, slices | R01 |
| Reference cycles and `Weak` | Explained in CONCEPTS, lesson 07, with a paper sketch as an optional lab. You build and repair a cycle in Chapter 9. |
| Reading the compiler's rejection of your own experiments | Optional labs in each lesson header |

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Draw the stack and the heap before and after a move, a clone, and a drop of a `String`.
- [ ] Say which operations leave the source variable without a value, and why such a variable can be given a new value.
- [ ] Take an owned element out of a vector, and an owned value out of a place behind `&mut`, without cloning and without leaving the place empty.
- [ ] Explain why `Copy` cannot be derived for a type with a `String` field, and how cloning an `Rc` differs from cloning the value inside it.
- [ ] Say when to use `Rc` and when `Arc`, why neither lets you change the shared value, and why two values that own each other through `Rc` are never freed.
- [ ] Redo the capstone after a break without looking at your first solution. Then invent a changed requirement and explain what it changes in the ownership design.

## Solutions

Model solutions are in [solutions/exercises/chapter04/](../../solutions/exercises/chapter04/) (one file per lesson, with the predictions filled in) and [solutions/drills/ch04/](../../solutions/drills/ch04/) (each drill repaired, plus the expected outcome of every drill). Use them after an honest attempt and after the hints: compare your solution with the model, and note in NOTES.md what differs and why. Reading a solution first removes the struggle that makes the lesson stay in memory.
