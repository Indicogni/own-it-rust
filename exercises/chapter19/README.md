# Chapter 19 — Threads and shared state

[All chapters](../../README.md) · [Previous chapter](../chapter18/README.md) · [Next chapter](../chapter20/README.md)

A program with several threads does several things at the same time, and the hard part is the data that those threads share. In most languages a mistake there shows up as a rare, random failure. In Rust most of these mistakes do not compile: the ownership and borrowing rules that you already know also decide what may cross from one thread to another. In this chapter you start threads and collect their results, let threads borrow local data, move values through channels, and share state behind locks, condition variables and atomics. You also learn the limits of the compiler's help. Deadlocks, lost wake-ups and wrong memory orderings still compile, so the chapter trains you to reason about them, and its tests are built to catch them: they check which thread did the work, whether workers really ran at the same time, and they turn a program that would wait forever into a failed test with a message.

**Before you start:** Chapters 4, 5, 7, 9, 13, 14 and 15; the review lesson also uses Chapters 16 to 18. **Time:** about 10 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. Sessions of 45 to 60 minutes work well.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example, and shows the compiler errors you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run the lesson, for example `cargo test --test chapter19 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Many tasks receive a function (`work`, `measure`, `read`, ...) that the tests supply. Through it a test sees which thread ran the work, and it can make one worker wait for another. So a solution that does everything on the calling thread, or that starts and joins its workers one after another, fails, and the message says what was expected. A solution that blocks forever is stopped after a time limit and reported as a failure. The [topic map](#topic-map) says what the tests cannot see. Then run `cargo clippy --test chapter19` and fix the warnings in code that you wrote. Never change a function's signature unless its doc comment says that you may.
5. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

Do lesson 09 after lesson 07. Then **drill the compiler**: `cargo drill ch19` shows ten small programs. For each one, predict whether it compiles (and with which error code), runs, or panics; then fix it. After that, do the **capstone**, the **review** lesson (r01: Chapters 14 to 18, from memory) and the **code review** lesson (c01: repair code that works in simple cases but is wrong). Run `cargo progress` to see where you are and which chapters are due for spaced review.

Test names start with the task letter (`a_`, `b_`, ...), so `cargo test --test chapter19 l06_coordination::tests::a_` runs the tests of one task.

| Done | Lesson | Coding tasks |
| --- | --- | --- |
| [ ] | [l01 · Fork, join, and worker failures](l01_fork_join.rs) | 2 |
| [ ] | [l02 · Borrowed parallel work](l02_scoped.rs) | 2 |
| [ ] | [l03 · Ownership and channel closure](l03_channels.rs) | 2 |
| [ ] | [l04 · Send and Sync contracts](l04_send_sync.rs) | 2 |
| [ ] | [l05 · Mutex invariants and poisoning](l05_mutexes.rs) | 2 |
| [ ] | [l06 · RwLock and condition predicates](l06_coordination.rs) | 3 |
| [ ] | [l07 · Atomic counters and local state](l07_atomics_tls.rs) | 2 |
| [ ] | [l09 · Retry loops, orderings, and one-time initialization](l09_cas_orderings.rs) | 4 |
| [ ] | [Compiler drills](../../drills/ch19/) — `cargo drill ch19` | 10 |
| [ ] | [l08 · Capstone: bounded parallel pipeline](l08_capstone.rs) | 2 |
| [ ] | [R01 · Review: Chapters 14–18 from memory](r01_review.rs) | 3 |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) | 2 |

```sh
cargo test --test chapter19 l01_
cargo test --test chapter19 -- --list
# After completing all lessons:
cargo test --test chapter19
cargo test --release --test chapter19
cargo clippy --test chapter19
cargo drill ch19
cargo run --example ch19_scratch
```

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, all tests but one fail because the supplied code has bugs. Keep the supplied tests as they are. Every lesson is independent, including the capstone, so an unfinished later task does not block an earlier lesson. Every task uses only the standard library and needs no network. Compiler experiments belong in [the scratch example](../../examples/ch19_scratch.rs); leave it compiling when you finish. Use the pinned Rust 1.99.0 toolchain and run all commands from the root of the course repository.

A test that fails with "was still running after 10 s" (or 20, 30 or 60 s) found a deadlock or an endless wait in your code. It is not a slow machine. Read the text in parentheses in that message; it names the usual cause.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

## Topic map

"Optional lab" means a project that you build by hand, following [LABS.md](LABS.md). No test in this repository checks it.

| Topic | Where you practise it |
| --- | --- |
| Starting threads, moving data into them, joining in input order | Lesson 01 task A; R01 tasks A and C; drill d01 |
| A worker's own error versus a worker's panic; panic payloads; detached and named threads | Lesson 01 task B and predictions |
| Scoped threads that borrow local data; splitting a slice for two writers | Lesson 02; R01 task B; drill d04; C01 |
| What a scope does with a worker that panicked | C01 (test with several failing workers); explained in CONCEPTS, lesson 02 |
| Channels: moving values to a worker, closing by dropping senders, order of messages | Lesson 03 task A; capstone. The tests see that a worker thread did the work and that the function returns; they cannot see the channel itself |
| Not blocking: `try_recv`, empty versus disconnected, a failed `send` | Lesson 03 task B and predictions; drill d10 |
| Bounded channels and backpressure | Lesson 03 predictions; capstone (a deadlock is reported as a timeout). The tests cannot see the capacity that you chose. Optional lab 2 varies it |
| `Send` and `Sync`: which types may cross threads, and why | Lesson 04; drills d02 and d07 |
| Sharing one allocation with `Arc`, and getting it back | Lesson 04 task B and predictions; drill d03 |
| `Mutex`: guards, how long a lock is held, keeping it short | Lesson 05 task A (the tests check that the lock is free while a worker computes; they cannot see which kind of lock you used); R01 task B; drills d05 and d06 |
| Temporary guards in `if let` and `match` (edition 2024) | Lesson 05 prediction P4 |
| Poisoning and deliberate recovery | Lesson 05 task B and predictions; drill d09 |
| Deadlock by lock order | Explained in CONCEPTS, lesson 05; a paper exercise in the lesson 05 header; optional lab 3 |
| `RwLock`: many readers or one writer | Lesson 06 task A; C01 (`refresh`) |
| `Condvar`: state under a mutex, a predicate loop, lost and spurious wake-ups | Lesson 06 tasks B and C; drill d08 |
| `Barrier` | Lesson 06 prediction P3; the tests of this chapter use it throughout |
| Atomic counters without lost updates; overflow | Lesson 07 task A (a probabilistic test, repeated to make a race show) and predictions |
| Thread-local values | Lesson 07 task B and predictions |
| Globals: why `static mut` is rejected, and what to use | Explained in CONCEPTS, lesson 07; drill d07; lesson 07 compiler lab (optional lab) |
| Compare-and-exchange, once and in a retry loop | Lesson 09 tasks A and B (task B has an exact test for the retry) |
| Memory orderings: a flag that needs `Release` and `Acquire` | Lesson 09 tasks C and D. A wrong ordering cannot be observed by running the code, so the lesson's flag type records the ordering that you pass and a test checks it. Another test holds the writer at the moment the flag appears, so a flag that is set before the value fails every time |
| One-time initialization with `OnceLock` and `LazyLock` | Lesson 09 predictions P2 to P4; compiler lab in the lesson header (optional lab) |
| A pipeline with several producers, one bounded channel and ordered results; stopping at the first error | Lesson 08 (capstone) |
| Reviewing concurrent code: thread limits, panics turned into errors, locks held too long | C01 |
| Recall of Chapters 14 to 18: boxed closures, iterators, maps, text, buffered readers | R01 |
| Thread pools and data-parallel libraries (Rayon) | Optional lab 1 |
| A worker queue with several consumers; shutdown | Optional labs 2 and 3 |
| Building locks, channels and `Arc` yourself; lock-free structures; `loom` | Not in this chapter. See the mastery track, [`mastery/m3_concurrency`](../../mastery/m3_concurrency/README.md) |

## Extension labs

- [LABS.md](LABS.md): four optional projects. They are manual practice outside the test target.

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

1. When can scoped threads borrow data that spawn cannot?
2. How do join errors differ from a worker Result error?
3. Which values must be Send, and when does Arc<T> require Sync?
4. How do channel closure, bounded backpressure, and a forgotten sender affect termination?
5. What invariant is a mutex protecting; why does a Condvar wait in a loop?
6. Which guarantees does relaxed atomic arithmetic provide, and which does it not provide?
7. How can you design concurrent tests around completion signals rather than sleeps?
8. What does a retry loop do after a failed compare-exchange, and which pair of orderings lets a flag announce other data?

Redo the capstone from its contract in a later session. Explain one design tradeoff and create a case the supplied tests do not exercise.

## Solutions

Model solutions are in [solutions/exercises/chapter19/](../../solutions/exercises/chapter19/) (one file per lesson, with the predictions filled in) and [solutions/drills/ch19/](../../solutions/drills/ch19/) (each drill repaired, plus the expected outcome of every drill). Use them after an honest attempt and after the hints: compare your solution with the model, and note in NOTES.md what differs and why. Reading a solution first removes the struggle that makes the lesson stay in memory.
