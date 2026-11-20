# Chapter 1 — Start here

[All chapters](../../README.md) · [Next chapter](../chapter02/README.md)

This is the first chapter of the course, and it assumes that you have never used Rust. You install the tools, check that they work, and learn to read what they print: the report of a failed test and the message of the compiler. You also learn the loop that every later lesson uses: predict, implement, explain. The chapter teaches very little of the language itself; that starts in Chapter 2. When you finish, no tool in this course is new to you any more, and you can give your whole attention to Rust.

**Before you start:** you have written a little code in some programming language (variables, functions, `if`). Nothing else. **Time:** about four and a half hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson.

## First steps

1. Read "Lesson 01" in [CONCEPTS.md](CONCEPTS.md). It tells you how to install Rust and an editor.
2. Open a terminal in the course directory and run `cargo progress` once, before you change any file. It shows every chapter and lesson, and it keeps an untouched copy of each lesson so that you can get it back later.
3. Run `cargo test --test chapter01 l01_`. Four tests fail. That is the correct starting state: the tasks are not done yet.

## Work through the chapter

Every lesson uses the same loop. Lesson 05 explains why it is built this way.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example, and shows the messages you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet.
3. **Implement** the `todo!()` bodies from the text above each function, and run the lesson, for example `cargo test --test chapter01 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Explain** in [NOTES.md](NOTES.md), then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

After lesson 04, **drill the compiler**: `cargo drill ch01` shows ten small programs. For each one, predict whether it compiles (and with which error code), runs, or panics; then repair it. After lesson 05, do the **capstone** on your own. Run `cargo progress` to see where you are.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Install, verify, and a first green test](l01_first_test.rs) |
| [ ] | [02 · Cargo: new, run, check, build, test](l02_cargo.rs) |
| [ ] | [03 · Reading a failing test](l03_failing_test.rs) |
| [ ] | [04 · Reading a compiler error](l04_compiler_error.rs) |
| [ ] | [Compiler drills](../../drills/ch01/) — `cargo drill ch01` |
| [ ] | [05 · The lesson loop: predict, implement, explain](l05_lesson_loop.rs) |
| [ ] | [06 · Capstone: the parking meter](l06_capstone.rs) |

```sh
cargo test --test chapter01 l01_
cargo test --test chapter01 -- --list
cargo test --test chapter01
cargo test --release --test chapter01
cargo drill ch01
cargo run --example ch01_scratch
cargo progress
```

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in. In lesson 03, and in one task of the capstone, the tests fail because the supplied code is wrong, and repairing it is the task. Keep the supplied tests as they are. Every task uses only the standard library and needs no network. Experiments belong in [the scratch example](../../examples/ch01_scratch.rs); leave it compiling when you finish.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has three hints. Each says more than the one before, and the third is nearly the answer.

## Topic map

"Optional lab" means an experiment that you run by hand. No test checks it.

| Topic | Where you practise it |
| --- | --- |
| Installing Rust with rustup; the pinned toolchain file; checking the version | Lesson 01 (task B) |
| An editor with rust-analyzer | Lesson 01 (set-up, no test) |
| Running one lesson; what `todo!()` and `guess()` do | Lesson 01 |
| `cargo new`, `check`, `build`, `run`, `test` in a package of your own | Lesson 02 (lab and tasks A to D) |
| What `target/` and `Cargo.lock` are | Lesson 02 (tasks B and C, prediction P2) |
| Reading `Cargo.toml` | Lesson 02 (prediction P1) |
| The parts of a failed test's report; `left` and `right` | Lesson 03 (tasks A, B and D); drill d09 |
| A panic inside the function under test; backtraces | Lesson 03 (task C and its optional lab) |
| The parts of a compiler message: code, position, labels, help | Lesson 04 (tasks A to D) |
| Several errors and warnings in one output; which to repair first | Lesson 04 (predictions) |
| The most frequent first errors: unknown name, wrong type, missing `mut`, missing semicolon, wrong number of arguments | Drills d01 to d08 |
| A first look at the ownership error `E0382` | Drill d10 (Chapter 4 teaches the rule) |
| The loop of predict, implement and explain; writing a test of your own | Lesson 05 |
| Whole-number division and remainder; the order of `*` and `/` | Lesson 05 (task C and predictions) |
| `cargo progress` and `cargo drill` | Lessons 04 and 05 |
| `if`/`else` as a value | Lesson 03 (tasks C and D); Lesson 06 (capstone) |
| Subtraction on `u32` | Lesson 06 (capstone) |

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Say how you find out which Rust version is used in a directory, and why the course fixes one version.
- [ ] Name the Cargo command that only looks for errors, the one that builds, the one that builds and runs, and the one that runs the tests. Say what `target/` and `Cargo.lock` are and which of them you may delete.
- [ ] In the report of a failed test, show where the name of the test, the file and line, and the expected and the actual value stand. Name the three kinds of failure and how you tell them apart.
- [ ] Name the parts of a compiler message in the order in which you read them, and say which error you repair first and why.
- [ ] Describe the lesson loop, and explain why the prediction comes before the first run.
- [ ] Redo the capstone after a break without looking at your first solution.

## Solutions

Model solutions are in [solutions/exercises/chapter01/](../../solutions/exercises/chapter01/) (one file per lesson, with the predictions filled in) and [solutions/drills/ch01/](../../solutions/drills/ch01/) (each drill repaired, plus the expected outcome of every drill). Use them after an honest attempt and after the hints: compare your solution with the model, and note in NOTES.md what differs and why. Reading a solution first removes the struggle that makes the lesson stay in memory.
