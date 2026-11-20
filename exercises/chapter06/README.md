# Chapter 6 — Expressions and control flow

[All chapters](../../README.md) · [Previous chapter](../chapter05/README.md) · [Next chapter](../chapter07/README.md)

Rust is an expression language: a block, an `if`, a `match` and even a `loop` produce a value, and one semicolon can change the type of a whole function body. In this chapter you practise that style until you can read it without effort. You compute values with blocks and branches, take data apart with `if let` and `let else`, leave loops early with `break`, `continue` and labels, and work out by hand what operators, casts and small closures do. You also learn when temporary values are dropped, which edition 2024 changed, and what `const` and `unsafe` blocks really mean. The teaching text is in [CONCEPTS.md](CONCEPTS.md); the lessons, drills and reviews in this directory are where you prove that you can use it.

**Time.** About 9 to 11 hours for a careful first pass (an estimate). [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter06 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations or compare pointers, so a solution that copies everything fails. When a task names the construct to use (a labeled block, `let else`, an inline const block, no `unsafe`), one test reads your source file and looks for it inside the body of that task's function. Then run `cargo clippy --test chapter06` and fix warnings in your code.
5. **Drill the compiler.** `cargo drill ch06` shows small programs; predict whether each compiles (and which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

After the numbered lessons, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: fix a teammate's working-but-wrong code). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Expression blocks and declarations](l01_blocks.rs) |
| [ ] | [02 · if and match as values](l02_branching.rs) |
| [ ] | [03 · if let and let else](l03_let_else.rs) |
| [ ] | [04 · for, while, break and continue](l04_loops.rs) |
| [ ] | [05 · Loop values and labeled blocks](l05_labels.rs) |
| [ ] | [06 · const and unsafe expression blocks](l06_const_unsafe.rs) |
| [ ] | [07 · Operators, calls, fields and assignment](l07_operators.rs) |
| [ ] | [08 · Casts and closure expressions](l08_casts_closures.rs) |
| [ ] | [09 · Capstone: command expression engine](l09_capstone.rs) |
| [ ] | [R01 · Review: Chapters 3–5 from memory](r01_review.rs) |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) |
| [ ] | [Compiler drills](../../drills/ch06/) — `cargo drill ch06` |

```sh
cargo test --test chapter06 l01_
cargo test --test chapter06 -- --list
cargo test --test chapter06
cargo test --release --test chapter06
cargo clippy --test chapter06
cargo drill ch06
cargo run --example ch06_scratch
```

The starters compile; tests fail intentionally at `todo!()` and `guess()` until you fill them in. Work in lesson order and keep tests intact. Every task uses only the standard library and needs no network. The tests of the loop tasks fail with a message after 10 seconds if a loop never ends, so a mistake cannot hang the whole run.

Each lesson header also has a **COMPILER / TRANSFER LAB** line. These labs are optional: you break code on purpose in [the scratch example](../../examples/ch06_scratch.rs) and read what the compiler says. No test checks them. Restore the scratch file to compiling code after each experiment.

Use [progressive hints](HINTS.md) only after a first attempt.

## Topic map

| Topic | Where you practise it |
| --- | --- |
| Block values, tail expressions, the unit type, shadowing | Lesson 01 (task A, P1–P3); drill d02 |
| Items declared inside a function | Lesson 01 (task B); optional lab in the Lesson 01 header |
| When temporaries are dropped: tail expressions, `if let`, `match` (edition 2024) | Lesson 01 (P4); Lesson 03 (P4, P5); drill d11 |
| `if` and `match` as values; range, `\|` and `@` patterns; guards; exhaustiveness | Lesson 02; C01 (`classify`); drills d01, d03, d09 |
| `if let`, `let else`, `while let` | Lesson 03; Lesson 09; R01 (task A); drill d04 |
| `for`, `while`, ranges, `break`, `continue` | Lesson 04; Lesson 09; R01 (task B) |
| `loop` with a value, labels, labeled blocks, the never type | Lesson 05; drill d05. `return` compared with `break`: optional lab only |
| Inline `const` blocks | Lesson 06 (task A, P1); optional lab |
| `unsafe` blocks and raw pointers | Lesson 06 (P2 and P3: you read given code, you do not write unsafe code); optional lab. Writing unsafe code is Chapter 23 |
| Operator precedence, bit operators, short-circuit `&&` and `\|\|` | Lesson 07; drill d06 |
| Method calls, field access, indexing, reference operators, the type of an assignment | Optional lab in the Lesson 07 header only; no graded task |
| Integer limits in loops and sums: widening, checked addition, unsigned subtraction | Lesson 04 (task A); Lesson 05 (task A); Lesson 09; R01 (task C); C01 (`total_size`); drill d07 |
| Numeric casts with `as` | Lesson 08 (task A, P1, P2); R01 (task C); drill d08 |
| Closures: what they capture, `move` | Lesson 08 (task B is a one-line body; P3, P4); drill d10. Closures in depth are Chapter 14 |
| Reviewing code written in statement style | C01 |

Three stated constraints are not checked by any test, so check them yourself: in Lesson 01, task A asks for a block expression and task B asks you to put the nested function after its first call; in C01, `end_line` should stop at the first match.

## Solutions

Model answers for every lesson, the review, the code review and the drills are in [`solutions/exercises/chapter06/`](../../solutions/exercises/chapter06/) and [`solutions/drills/ch06/`](../../solutions/drills/ch06/). Open one only after an honest attempt of your own, including the hints. A solution you read before you struggled teaches you much less than one you compare with your own work.

## Mastery check

- [ ] Explain how semicolons and tail expressions change a block type.
- [ ] Choose match, if let, and let else according to exhaustiveness and early exit needs.
- [ ] Say when a temporary value is dropped in a tail expression, in the condition of an if let, and in the scrutinee of a match.
- [ ] Trace return, labeled break, continue, and loop values; explain the never type.
- [ ] Separate const evaluation from runtime and explain the proof obligation at an unsafe block.
- [ ] Explain operator precedence, short-circuiting, cast truncation, and closure capture.
- [ ] Redo the final capstone after a break without looking at your implementation; invent a changed requirement and explain the tradeoff.

Model answers are at the end of [HINTS.md](HINTS.md). Write your own in [NOTES.md](NOTES.md) first.
