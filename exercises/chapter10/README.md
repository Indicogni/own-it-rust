# Chapter 10 — Enums and pattern matching

[All chapters](../../README.md) · [Previous chapter](../chapter09/README.md) · [Next chapter](../chapter11/README.md)

A struct groups values that exist together. An enum describes a value that is exactly one of several alternatives, and each alternative can carry its own data. Patterns are the other half of the same idea: a pattern takes a value apart, tests its shape and gives names to its parts in one step. Together they let you write down which states a program can be in, and they let the compiler check that every one of those states is handled. In this chapter you define enums with and without data, build recursive and generic ones, and match owned, borrowed and mutably borrowed values without unwanted moves. You use slice patterns, ranges, guards, alternatives and `@` bindings, and you learn which patterns each part of the language accepts. The chapter ends with an ordered tree that you build from an enum, a `Box` and three recursive functions.

**Before you start:** Chapters 3 to 7 and 9. **Time:** about 8 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. Sessions of 45 to 60 minutes work well.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example, and shows the compiler errors you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter10 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations, compare pointers or count comparisons, so a solution that clones everything or searches the whole tree fails, and the failure message says what was expected. Then run `cargo clippy --test chapter10` and fix the warnings in code that you wrote. Warnings about functions that still contain `todo!()` go away when you implement them.
5. **Drill the compiler.** `cargo drill ch10` shows ten small programs; for each one, predict whether it compiles (and with which error code), runs or panics, then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

After the numbered lessons, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: repair a teammate's code that works but is written badly). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Variants and payloads](l01_variants.rs) |
| [ ] | [02 · Recursive data with enums](l02_recursive.rs) |
| [ ] | [03 · Generic enums](l03_generic.rs) |
| [ ] | [04 · Literal, tuple and struct patterns](l04_structural.rs) |
| [ ] | [05 · Matching references and mutating payloads](l05_reference_patterns.rs) |
| [ ] | [06 · Array and slice patterns](l06_slice_patterns.rs) |
| [ ] | [07 · Guards, alternatives and @ bindings](l07_guards_or_at.rs) |
| [ ] | [08 · Pattern contexts and public compatibility](l08_contexts_public.rs) |
| [ ] | [09 · Capstone: generic ordered tree](l09_capstone.rs) |
| [ ] | [R01 · Review: Chapters 3, 7 and 9 from memory](r01_review.rs) |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) |
| [ ] | [Compiler drills](../../drills/ch10/) — `cargo drill ch10` |

```sh
cargo test --test chapter10 l01_
cargo test --test chapter10 -- --list
cargo test --test chapter10
cargo test --release --test chapter10
cargo clippy --test chapter10
cargo drill ch10
cargo run --example ch10_scratch
```

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, most tests fail because the supplied code has bugs. Work in lesson order and keep the supplied tests as they are. Every task uses only the standard library and needs no network. Single-file experiments belong in [the scratch example](../../examples/ch10_scratch.rs); leave it compiling when you finish.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

**Rules about the form of a pattern are on your honour.** Some contracts ask for a particular form: "use one slice pattern", "use `@` and a range, not an `if`", "no wildcard arm". A test can observe results, allocations, pointers and comparisons. It cannot see which syntax you typed. These rules are the practice itself, so follow them even though a different form would also turn the tests green.

## Topic map

"Optional lab" means an experiment that you run by hand. No test checks it.

| Topic | Where you practise it |
| --- | --- |
| Unit-like, tuple-like and struct-like variants; a `match` that covers every variant | Lesson 01 tasks A and B; drill d01 |
| Designing an enum yourself: one variant per state, with only the data that state needs | Lesson 01 task C |
| Moving a payload out of an owned enum without copying | Lesson 01 tasks B and C; drill d08 |
| Explicit discriminants and `as`, derived ordering, `mem::discriminant`, the size of `Option<Box<T>>` | Lesson 01 (predictions) |
| Measuring the size and alignment of your own enums | Lesson 01, optional lab |
| Recursive enums with `Box`, functions that follow the shape of the type, drop order of nested values | Lesson 02; drill d06 |
| Generic enums; moving and borrowing payloads without trait bounds; ordering of `Option` and `Result` | Lesson 03 |
| Literal, tuple and struct patterns; order of arms; patterns in parameters | Lesson 04 |
| A plain name in a pattern: new binding or comparison; `_` versus `_name` | Lesson 04 (predictions); drill d03; C01 |
| Matching `&T` and `&mut T`: borrowed bindings, changing a payload in place | Lesson 05; drills d05 and d07 |
| The explicit `ref` and `&` spellings and their edition 2024 error | Explained in CONCEPTS, Lesson 05; trying it out is an optional lab |
| Array patterns and slice patterns, `rest @ ..` | Lesson 06; R01 task B; drill d09 |
| Ranges, `@` bindings, or-patterns, guards, `matches!` | Lesson 07; R01 task C; drills d04 and d10 |
| `while let`, `let ... else` and let chains | Lesson 08; drill d02 |
| `#[non_exhaustive]` on public enums and structs | Explained in CONCEPTS, Lesson 08, with a two-package optional lab |
| An ordered tree: recursive insert through `&mut`, ordered search, traversal that returns borrows; how insertion order decides the shape | Lesson 09 (capstone) |
| Reviewing pattern bugs: a wildcard arm that hides a variant, a binding that was meant to compare, needless clones | C01 |
| Recall of `Cell`, `?` with `From`, and Unicode scalar values | R01 |

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Represent mutually exclusive states with variants and choose payload ownership deliberately.
- [ ] Explain recursive indirection and avoid assumptions about default enum layout.
- [ ] Destructure owned, borrowed, and mutably borrowed variants and track moves.
- [ ] Use slice patterns, guards, or-patterns and @ without introducing accidental catch-all bindings.
- [ ] Explain exhaustiveness, the role of wildcard arms, and non_exhaustive public API evolution.
- [ ] Build and query a recursive binary tree using patterns and compare its worst-case shape.
- [ ] Redo the final capstone after a break without looking at your implementation; invent a changed requirement and explain the tradeoff.

## Solutions

Model solutions are in [solutions/exercises/chapter10/](../../solutions/exercises/chapter10/) (one file per lesson, with the predictions filled in) and [solutions/drills/ch10/](../../solutions/drills/ch10/) (each drill repaired, plus the expected outcome of every drill in `expected.tsv`). Use them after an honest attempt and after the hints: compare your code with the model and write in NOTES.md what differs and why. If you read a solution first, you skip the effort that makes the lesson stay in memory.
