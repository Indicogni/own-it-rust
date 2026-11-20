# Chapter 2 — A first tour

[All chapters](../../README.md) · [Previous chapter](../chapter01/README.md) · [Next chapter](../chapter03/README.md)

This chapter shows you the whole everyday language once, at first contact: variables and types, functions, `if` and loops, text, lists, your own types with `struct` and `enum`, `match`, the two types `Option` and `Result` that replace "null" and exceptions, and tests that you write yourself. Each lesson is small, and the teaching text has more worked examples than in later chapters, because this is where you learn to read and write Rust at all. At the end you write a small program, a text statistics tool, with a design of your own. Chapters 3 to 24 then return to every topic and go deep.

**Before you start:** Chapter 1. **Time:** about ten hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. Sessions of 45 to 60 minutes work well.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows worked examples, and shows the compiler errors you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run the lesson, for example `cargo test --test chapter02 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Run `cargo clippy --test chapter02` and repair the warnings in code that you wrote. Clippy is a second checker that knows simpler and safer ways to write many things. The warnings in `c01_code_review.rs` are planted for the code review lesson; leave them until you get there. Never change a function's signature unless its doc comment says that you may.
5. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

After lesson 09, **drill the compiler**: `cargo drill ch02` shows ten small programs that mix lessons 01 to 08. For each one, predict whether it compiles (and with which error code), runs, or panics; then repair it. After that, do the **capstone** and the **code review** lesson (c01: repair code that works badly and is written badly). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Variables, mutability and basic types](l01_variables.rs) |
| [ ] | [02 · Functions, expressions and statements](l02_functions.rs) |
| [ ] | [03 · if, loops and ranges](l03_control_flow.rs) |
| [ ] | [04 · Text: &str, String and formatting](l04_text.rs) |
| [ ] | [05 · Vectors: a list that can grow](l05_vectors.rs) |
| [ ] | [06 · Structs and methods](l06_structs.rs) |
| [ ] | [07 · Enums and match](l07_enums_match.rs) |
| [ ] | [08 · Option and Result at first contact](l08_option_result.rs) |
| [ ] | [09 · Writing your own tests](l09_tests.rs) |
| [ ] | [Compiler drills](../../drills/ch02/) — `cargo drill ch02` |
| [ ] | [10 · Capstone: a text statistics tool](l10_capstone.rs) |
| [ ] | [C01 · Code review: a teammate's first pull request](c01_code_review.rs) |

```sh
cargo test --test chapter02 l01_
cargo test --test chapter02 -- --list
cargo test --test chapter02
cargo test --release --test chapter02
cargo clippy --test chapter02
cargo drill ch02
cargo run --example ch02_scratch
```

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, some tests fail because the supplied code has bugs. Keep the supplied tests as they are. In lessons 09 and 10 you also write tests of your own, in a section named `mod my_tests`. Every task uses only the standard library and needs no network. Compiler experiments belong in [the scratch example](../../examples/ch02_scratch.rs); leave it compiling when you finish.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has three hints. Each says more than the one before, and the third is nearly the answer. From Chapter 3 on there are two hints per lesson.

## Topic map

"Optional lab" means an experiment that you run by hand in the scratch example. No test checks it.

| Topic | Where you practise it |
| --- | --- |
| `let`, `mut`, shadowing, and the life of a variable inside a block | Lesson 01; drills d01, d04 and d10 |
| Number types by use (`i32`, `u32`, `i64`, `usize`, `f64`), `bool` and `char`; converting between number types | Lesson 01; drill d03 |
| Whole-number division and float division | Lesson 01 (task C and predictions); C01 |
| Functions, signatures, and the value of the last expression | Lesson 02 |
| Expressions, statements, blocks as values, the unit type `()` | Lesson 02 (predictions); drill d04 |
| Tuples for two results | Lesson 02 (task D) |
| `if` as an expression | Lesson 03 (task A); drills d02 and d10 |
| `for` over ranges, `..` and `..=`, `while`, `loop`, `break`, `continue` | Lesson 03; drill d06 |
| `&str` and `String`; lending a `String` with `&` | Lesson 04; drill d05 |
| `format!` and `println!`: decimals, width, alignment, `{:?}` | Lesson 04 (tasks B and D, predictions) |
| `Vec`: `push`, `len`, indexes, walking with `for`; slices as parameters | Lesson 05; drills d01 and d06 |
| References to elements and `*` | Lesson 05 (tasks C and D) |
| Structs, fields, methods with `&self` and `&mut self`, `new` | Lesson 06 |
| `#[derive(Debug, PartialEq)]` | Lesson 06 (predictions); drill d09 |
| Enums, variants with data, `match`, exhaustiveness, patterns | Lesson 07; drill d07 |
| `Option` and `Result`: building them and taking them apart; `parse`; `?` | Lesson 08; drill d08 |
| Writing tests: `#[test]`, the assert macros, `#[should_panic]`, choosing cases | Lesson 09; your own tests in lesson 10 |
| Checks that catch wrong versions of a function; functions as values | Lesson 09 (task C) |
| Designing a small program from a description | Lesson 10 (capstone) |
| Turning tested functions into a program with arguments and standard input | Lesson 10 (optional lab) |
| Finding bugs and long-winded code in a teammate's work; clippy | C01 |
| Ownership, borrowing and lifetimes | Only met here (`&`, `&mut self`, error `E0382`). Chapters 4 and 5 teach them. |

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Explain `let`, `let mut` and shadowing with one example each, and say when you use which.
- [ ] Say what a function returns, what a semicolon on the last line changes, and what `()` means in a compiler message.
- [ ] Write from memory a `for` loop over `1..=n`, a `while` loop, and a `loop` with `break`. Say how many rounds `0..5` and `0..=5` make.
- [ ] Explain the difference between `&str` and `String`, and which of them a function should take and which it should return.
- [ ] Define a struct with one method that reads and one that changes, and an enum with a variant that carries data. Write a `match` over the enum.
- [ ] Explain what `Option` and `Result` are for, and show three ways to get the value out.
- [ ] Write a test module from memory, with one ordinary test and one `#[should_panic]` test. Name the kinds of test case that you look for.
- [ ] Redo the capstone after a break without looking at your first solution. Then add a fifth command of your own.

## Solutions

Model solutions are in [solutions/exercises/chapter02/](../../solutions/exercises/chapter02/) (one file per lesson, with the predictions filled in) and [solutions/drills/ch02/](../../solutions/drills/ch02/) (each drill repaired, plus the expected outcome of every drill). Use them after an honest attempt and after the hints: compare your solution with the model, and note in NOTES.md what differs and why. Reading a solution first removes the struggle that makes the lesson stay in memory. The capstone has many good designs; the model is one of them, not the only right one.
