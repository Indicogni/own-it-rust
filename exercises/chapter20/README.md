# Chapter 20 — Writing async code

[All chapters](../../README.md) · [Previous chapter](../chapter19/README.md) · [Next chapter](../chapter21/README.md)

A program that talks to a network spends most of its time waiting. With threads, every wait holds a whole thread. Asynchronous code is a second way to wait: a function is written so that it can pause at a marked point, hand control back, and continue later from the same point, and a small number of threads can then serve a large number of waiting jobs. In Rust such a paused job is an ordinary value called a future, and the rules you already know (ownership, borrowing, `Drop`, `Send`) apply to it in ways that surprise people at first. In this chapter you learn those rules with the standard library alone: when an async body really runs, what a future borrows and what it owns, how traits and closures become async, what is dropped when a future is cancelled, and how to poll two futures by hand. Runtimes such as Tokio are built on exactly these rules; you meet one in the optional labs.

**Before you start:** Chapters 4, 5, 11, 13, 14 and 19. **Time:** about 7 to 8 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. Sessions of 45 to 60 minutes work well.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example, and shows the compiler errors you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run the lesson, for example `cargo test --test chapter20 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** The futures here are driven by tiny test executors, so the tests can count polls, log the order in which work starts, look at a lock while a future is suspended, and count heap allocations. A solution that runs work in the wrong order, holds a lock across `.await`, or boxes without need fails, and the failure message says what was expected. Then run `cargo clippy --test chapter20` and fix the warnings in code that you wrote. Warnings about functions that still contain `todo!()` disappear when you implement them. Do not change a function's signature.
5. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

After lesson 06, **drill the compiler**: `cargo drill ch20` shows eleven small programs that mix lessons 01 to 06. For each one, predict whether it compiles (and with which error code), runs, or panics; then fix it. After that, do the **capstone**, the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: repair code that works but is written badly). Run `cargo progress` to see where you are and which chapters are due for spaced review.

Test names start with the task letter (`a_`, `b_`, ...), so `cargo test --test chapter20 l06_racing::tests::a_` runs the tests of one task. Each lesson is independent, so an unfinished later lesson does not block an earlier one.

| Done | Lesson | Coding tasks |
| --- | --- | --- |
| [ ] | [01 · Lazy futures and await](l01_lazy.rs) | 2 |
| [ ] | [02 · Borrowed and owned futures](l02_lifetimes.rs) | 2 |
| [ ] | [03 · Async behavior in traits](l03_traits.rs) | 2 |
| [ ] | [04 · Lending async callbacks](l04_async_closures.rs) | 2 |
| [ ] | [05 · Cancellation by dropping](l05_cancellation.rs) | 2 |
| [ ] | [06 · Polling a race](l06_racing.rs) | 2 |
| [ ] | [Compiler drills](../../drills/ch20/) — `cargo drill ch20` | 11 |
| [ ] | [07 · Capstone: asynchronous graph traversal](l07_capstone.rs) | 2 |
| [ ] | [R01 · Review: Chapters 13 and 17–19 from memory](r01_review.rs) | 3 |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) | 3 |

```sh
cargo test --test chapter20 l01_
cargo test --test chapter20 -- --list
# After completing all lessons:
cargo test --test chapter20
cargo test --release --test chapter20
cargo clippy --test chapter20
cargo drill ch20
cargo run --example ch20_scratch
```

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, most tests fail because the supplied code has bugs. Keep the supplied tests as they are. Every lesson uses only the standard library and needs no network. Compiler experiments belong in [the scratch example](../../examples/ch20_scratch.rs); leave it compiling when you finish.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

## Topic map

"Optional lab" means work that you do by hand: either an experiment in the scratch example, or a project from [LABS.md](LABS.md) in a package of your own. No test in this repository checks an optional lab.

| Topic | Where you practise it |
| --- | --- |
| Calling an async function creates a future; the body runs only when the future is polled | Lesson 01; drills d02 and d06 |
| `.await`: where it is allowed, and that two awaits run one after the other | Lesson 01; drill d01 |
| Every async block has its own type | Lesson 02 (prediction); drill d04 |
| Futures that borrow their input, and the lifetime in their type | Lesson 02 task A; drill d03; R01 task B |
| Futures that own their input; `async move`; what `Send + 'static` demands | Lesson 02 task B; drills d05, d09 and d11; C01 |
| Async methods in traits through a generic bound, without boxing | Lesson 03; capstone |
| Why such a trait cannot be used as `dyn Trait`, and the boxed-future alternative | Drill d08; optional lab in the lesson 03 header |
| Async closures and the `AsyncFn`, `AsyncFnMut`, `AsyncFnOnce` bounds | Lesson 04; drill d10 |
| Cancellation: what is dropped with a future, and what is left half done | Lesson 05; R01 task C; C01 |
| Not holding a lock across `.await` | C01 |
| Calling `poll` by hand with `pin!` and `poll_fn`; a biased race of two futures | Lesson 06; drill d07 |
| Polling futures on threads | R01 task A |
| Async composition in one program: trait, generic function, errors through `.await?` | Lesson 07 (capstone, sequential on purpose) |
| Recall of Chapters 13 and 17 to 19: drop order, Unicode text, reading, threads and locks | R01 |
| Spawning tasks on a runtime, a bounded number of tasks in progress, collecting task results | Optional lab 1 in LABS.md (Tokio) |
| Blocking work and `spawn_blocking` | Optional lab 2 in LABS.md (Tokio) |
| `select!`, and state that is lost when a branch is cancelled | Optional lab 3 in LABS.md (Tokio) |
| Shutdown: abort, shutdown signals, cancellation tokens | Optional lab 4 in LABS.md (Tokio) |

This chapter does not make you write code in which several futures make progress at the same time, apart from the two-future race in lesson 06. CONCEPTS.md shows a join of two futures as a worked example. Tested practice with a real runtime is in unit `e06_tokio` of the [ecosystem package](../../ecosystem/README.md). Chapter 21 builds wakers and an executor by hand, and the mastery track ([`mastery/m4_async`](../../mastery/m4_async/README.md)) goes on to pin projection, streams, and cancellation in a real service.

## Optional labs

- [LABS.md](LABS.md): four projects on the Tokio runtime. They are optional and unchecked.

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

1. Why is calling an async function different from executing its body?
2. What may a future borrow, and what must it own? What do `Send` and `'static` ask of a task that is spawned on a runtime?
3. How does a trait offer an async method, and what does each form (generic bound, `+ Send`, boxed future) cost?
4. How do AsyncFn, AsyncFnMut, and AsyncFnOnce express lending asynchronous callbacks?
5. What happens to a future's state when it is dropped, and what follows from that for the code you write around an await?
6. What must a hand-written combinator such as a race do on each poll?
7. Is the capstone traversal concurrent? What would a concurrent version need, and what evidence would show that it bounds its work and reports every error?
8. After the optional labs: which work may block an executor thread, and what belongs in spawn_blocking?
9. After the optional labs: what state is lost when a future is dropped in a select! branch?
10. After the optional labs: how do abort, cooperative shutdown signals, and cancellation tokens differ?

Redo the capstone from its contract in a later session, without looking at your first solution. Explain one design tradeoff and create a case that the supplied tests do not exercise.

## Solutions

Model solutions are in [solutions/exercises/chapter20/](../../solutions/exercises/chapter20/) (one file per lesson, with the predictions filled in) and [solutions/drills/ch20/](../../solutions/drills/ch20/) (each drill repaired, plus the expected outcome of every drill). Use them after an honest attempt and after the hints: compare your solution with the model, and note in NOTES.md what differs and why. Reading a solution first removes the struggle that makes the lesson stay in memory. The optional labs have no model solution.

## Validation scope

Use the course's pinned Rust 1.99.0 toolchain and run commands from the repository root (see the [setup instructions](../../README.md#fresh-clone-setup)). To check that the starter compiles without running any test, use `cargo test --offline --test chapter20 --no-run`. The optional labs are outside these checks.
