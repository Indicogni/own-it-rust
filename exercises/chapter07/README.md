# Chapter 7 — Handling errors

[All chapters](../../README.md) · [Previous chapter](../chapter06/README.md) · [Next chapter](../chapter08/README.md)

Programs fail in two different ways, and Rust gives each its own tool. When the program itself is wrong, it panics and stops. When the world is wrong (bad input, a missing file, a full disk), a function returns a `Result`, and the caller decides what to do. In this chapter you practise both: you choose between them, pass errors up with `?`, reshape them with `Result`'s methods, design error types that keep their cause, and decide what a finished program prints and which exit status it returns. The chapter teaches by itself: [CONCEPTS.md](CONCEPTS.md) explains each idea with a worked example before you meet it in a lesson. You need chapters 3 to 6. Allow roughly 8 to 10 hours for a first pass; CONCEPTS.md has an estimate for each lesson.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter07 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations, compare pointers, or record which thread did the work, so a solution that copies everything or skips the technique fails. Then run `cargo clippy --test chapter07` and fix warnings in your code.
5. **Drill the compiler.** `cargo drill ch07` shows small programs; predict whether each compiles (and which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

After the numbered lessons, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: fix a teammate's working-but-wrong code). Run `cargo progress` to see where you are and which chapters are due for spaced review.

The table is in study order. Lesson 09 comes before the capstone (lesson 08), which builds on it.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Panics and invariants](l01_panic.rs) |
| [ ] | [02 · Catching Result errors and printing context](l02_catching.rs) |
| [ ] | [03 · Propagating failures with question mark](l03_propagation.rs) |
| [ ] | [04 · Result methods and aliases](l04_methods_aliases.rs) |
| [ ] | [05 · Multiple error types at a boundary](l05_multiple_errors.rs) |
| [ ] | [06 · Custom errors with a source chain](l06_custom.rs) |
| [ ] | [07 · Impossible errors and deliberate recovery](l07_invariants_ignoring.rs) |
| [ ] | [09 · Thread-safe boxed errors and what main returns](l09_send_errors_main.rs) |
| [ ] | [08 · Capstone: fallible batch report](l08_capstone.rs) |
| [ ] | [R01 · Review: Chapters 3, 5 and 6 from memory](r01_review.rs) |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) |
| [ ] | [Compiler drills](../../drills/ch07/) — `cargo drill ch07` |
| [ ] | [Optional labs](LABS.md) — whole-program behaviour that tests cannot check |

```sh
cargo test --test chapter07 l01_
cargo test --test chapter07 -- --list
cargo test --test chapter07
cargo test --release --test chapter07
cargo clippy --test chapter07
cargo drill ch07
cargo run --example ch07_scratch
```

The starters compile; tests fail intentionally at `todo!()` and `guess()` until you fill them in. Keep the tests intact. Every lesson uses only the standard library and needs no network. Experiments that are meant to fail to compile belong in [the scratch example](../../examples/ch07_scratch.rs), not in the lessons; put working code back afterwards.

Use the [progressive hints](HINTS.md) only after a first attempt.

## Solutions

Model answers for every lesson, the review and the code review are in [`solutions/exercises/chapter07/`](../../solutions/exercises/chapter07/), and fixed drills are in [`solutions/drills/ch07/`](../../solutions/drills/ch07/). Open one only after an honest attempt of your own: compare, note what differs, and then close it and repair your version from memory. Reading a solution first removes the struggle that makes the lesson stay with you.

## Topic map

"Optional lab" means a guided experiment in [LABS.md](LABS.md) that you run and observe yourself. No test checks it.

| Topic | Where you practise it |
| --- | --- |
| Panic for a broken invariant, `Result` for an expected failure | Lesson 01, tasks A and B |
| Unwinding runs destructors; what a panic payload is | Lesson 01, predictions |
| Abort instead of unwinding; the panic hook; `assert!`, `unreachable!`, `todo!` and related macros | Optional labs 1 and 2 |
| Handling a `Result` with `match`; fallback policies; stable messages with context | Lesson 02; drills d04 and d05 |
| What `parse` accepts and which error kinds it reports | Lesson 02, predictions |
| `Display` and `Debug` of an error; standard output and standard error | Lesson 02, task B; optional lab 3 |
| `?` on `Result` and `Option`; early return; the first error wins | Lesson 03; drills d01 and d02; R01, task A |
| `map`, `map_err`, `and_then`, `transpose`; eager and lazy fallbacks; type aliases | Lesson 04 (the tests check results, not which method you called) |
| `Box<dyn Error>`: conversion by `?`, downcasting, size and allocation cost | Lesson 05; drill d06 |
| Your own error enum: `Display`, `Error::source`, source chains | Lesson 06; drill d07; C01 |
| `From` impls that let `?` convert into your error | Explained in CONCEPTS.md (Lesson 06) and read in drills d02 and d09; this chapter does not ask you to write one |
| `expect` with a stated reason; `let _ =`; recovery that reports what it dropped | Lesson 07; drill d03; C01 |
| `Box<dyn Error + Send + Sync + 'static>`: errors that cross threads; converting between the two boxed forms | Lesson 09, tasks A to C; drill d10 |
| What `main` returns: `Termination`, `ExitCode`, the message on standard error | Lesson 09, task D and predictions (through `Termination::report`, inside the test process); a real process and its exit status in optional lab 5 |
| Typed errors that carry a position; no partial success; no allocation | Lesson 08 (capstone) |
| Earlier chapters inside error handling: loops that stop early, borrowed results, `try_from` | R01 |
| Matching on a borrowed `Result` | Drill d08 |
| The `anyhow` crate | Optional lab 4 (a separate package; needs network access) |

## Mastery check

- [ ] Choose Result for expected failures and panic for a documented broken invariant.
- [ ] Explain unwinding versus aborting and why catch_unwind is not general error handling.
- [ ] Propagate errors with ? and preserve contextual information and source chains.
- [ ] Choose between a typed error enum, `Box<dyn Error>`, and `Box<dyn Error + Send + Sync + 'static>` for a given boundary, and say what each choice costs the caller.
- [ ] Explain what ignoring an error means and make main return a useful exit status.
- [ ] Redo the final capstone after a break without looking at your implementation; invent a changed requirement and explain the tradeoff.

Model answers are at the end of [HINTS.md](HINTS.md). Write your own first.

For language details, the authorities are the [Rust Reference](https://doc.rust-lang.org/reference/) and the [standard library documentation](https://doc.rust-lang.org/std/); CONCEPTS.md links to the relevant pages.

## Next

Take checkpoint 1 (`checkpoints/cp1_after_ch07`) at least two days after you finish this chapter. It tests chapters 1 to 7 without hints. Read the [checkpoint guide](../../checkpoints/README.md#when-to-take-one) first, and open the checkpoint's `SPEC.md` only when you start the timer. Until then, continue with [chapter 8](../chapter08/README.md).
