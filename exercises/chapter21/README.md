# Chapter 21 — Futures by hand

[All chapters](../../README.md) · [Previous chapter](../chapter20/README.md) · [Next chapter](../chapter22/README.md)

In Chapter 20 you wrote `async` functions and let a runtime drive them. This chapter opens the machine. A future is an ordinary value with one method, `poll`, and everything else in async Rust is built around three questions: who calls `poll`, when is it called again, and what must not happen between two calls. You answer them by building the parts yourself, with the standard library only: futures that finish at once and futures that wait, a waker, a cell that another thread completes without losing a wakeup, an executor that sleeps while there is nothing to do, the registration pattern that I/O reactors use, and a future that does a bounded amount of work for each poll. The capstone wraps a future that must never move, which is where pinning stops being a word in a signature and becomes a tool that you use. When you finish, a hanging async program is no longer a mystery to you: you can name the wake that was lost and say where it should have been arranged.

**Before you start:** Chapters 11, 13, 14, 19 and 20. **Time:** about 8 to 9 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. Sessions of 45 to 60 minutes work well.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example, and shows the mistakes and compiler errors you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run the lesson, for example `cargo test --test chapter21 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** The tests drive your futures by hand with wakers that count their calls or look at a lock. They check that `Pending` always arranges a wake and `Ready` does not, that the newest waker is the one that is woken, that a wake happens after the lock is released, and that polling does not allocate. In lesson 04 they watch your executor from another thread and count its polls. A solution that spins, sleeps and tries again, wakes an old waker, or clones instead of moving fails with a message that says what was expected. A solution that blocks forever fails after a few seconds instead of hanging the test run. Then run `cargo clippy --test chapter21` and fix the warnings in code that you wrote. Warnings about functions that still contain `todo!()` disappear when you implement them.
5. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

After lesson 06, **drill the compiler**: `cargo drill ch21` shows ten small programs. For each one, predict whether it compiles (and with which error code), runs, or panics; then fix it. After that, do the **capstone**, the **review** lesson (r01: Chapters 16, 19 and 20, from memory) and the **code review** lesson (c01: repair code that looks right and is not). Run `cargo progress` to see where you are and which chapters are due for spaced review.

Test names start with the letters of the tasks they exercise (`a_`, `a_b_`, ...), so you can run the tests of one task, for example `cargo test --test chapter21 l01_future_trait::tests::a_`. Each lesson is independent, so an unfinished later lesson does not block an earlier one.

| Done | Lesson | Coding tasks |
| --- | --- | --- |
| [ ] | [l01 · Implement Future](l01_future_trait.rs) | 2 |
| [ ] | [l02 · Pending and wakeups](l02_wake.rs) | 3 |
| [ ] | [l03 · A completion cell](l03_completion.rs) | 3 |
| [ ] | [l04 · A parker block_on](l04_executor.rs) | 2 |
| [ ] | [l05 · Readiness registration model](l05_reactor.rs) | 2 |
| [ ] | [l06 · Cooperative work budgets](l06_cooperation.rs) | 2 |
| [ ] | [Compiler drills](../../drills/ch21/) — `cargo drill ch21` | 10 |
| [ ] | [l07 · Capstone: pinned future adapter](l07_capstone.rs) | 2 |
| [ ] | [R01 · Review: Chapters 16, 19, and 20 from memory](r01_review.rs) | 3 |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) | 3 |

```sh
cargo test --test chapter21 l01_
cargo test --test chapter21 -- --list
# After completing all lessons:
cargo test --test chapter21
cargo test --release --test chapter21
cargo clippy --test chapter21
cargo drill ch21
cargo run --example ch21_scratch
```

Run the commands from the root of the course repository, with the pinned Rust 1.99.0 toolchain. The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, most tests fail because the supplied code has bugs. Keep the supplied tests as they are. Every lesson uses only the standard library and needs no network. Compiler experiments belong in [the scratch example](../../examples/ch21_scratch.rs); leave it compiling when you finish.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

## Topic map

"Optional lab" means a project or experiment that you do by hand. No test in this repository checks it.

| Topic | Where you practise it |
| --- | --- |
| The `Future` trait: `Output`, `poll`, `Poll::Ready` and `Poll::Pending` | Lesson 01; drills d03 and d04 |
| Polling a future by hand; what a second poll after `Ready` does | Lesson 01 (task A and predictions); drills d01 and d07 |
| `Pin<&mut Self>` as the receiver of `poll`, and what `Unpin` allows | Lesson 01; drills d01, d03 and d10 |
| What `Pending` promises; waking as a request for another poll | Lesson 02 |
| Writing a waker with the `Wake` trait; `wake`, `wake_by_ref`, and the `Arc` inside a `Waker` | Lesson 02 (tasks B and C, predictions); drill d09 |
| A future that another party completes: shared state, storing the newest waker | Lesson 03; drill d05; C01 |
| Lost wakeups, and waking only after the lock is released | Lessons 03 and 05 (lock-probing tests); drill d08; R01 task B; C01 |
| Handing work to a thread and waiting for its result as a future | R01 task B. A complete adapter with panics and disconnection is optional lab 1 in [LABS.md](LABS.md). |
| An executor for one future: `block_on` with `thread::park` and `unpark` | Lesson 04 |
| What a runtime adds to that executor (task queue, spawning, timers) | Explained in CONCEPTS, lesson 04. Built in the mastery track, [`mastery/m4_async`](../../mastery/m4_async/). |
| Readiness and registration, modelled with a flag and one waiter | Lesson 05 |
| Readiness is not data: short reads and end of input | Lesson 05 (predictions) |
| A reactor over real nonblocking sockets | Optional lab 2 in [LABS.md](LABS.md) |
| Bounded work for each poll, and yielding with a self-wake | Lesson 06; lesson 04 task B; C01 |
| How a real runtime shares one thread among tasks | Optional lab 3 in [LABS.md](LABS.md) |
| Pinning: why a started future must not move, `Pin<Box<F>>`, moving the handle | Lesson 07 (capstone); drills d02 and d06 |
| Which moves the compiler rejects for a `!Unpin` value | Optional lab 4 in [LABS.md](LABS.md) |
| Running several futures at once inside one future | R01 task A |
| A first-in, first-out queue of waiting tasks | R01 task C |
| Pin projection, streams, and a multi-task executor | Not in this chapter. See [`mastery/m4_async`](../../mastery/m4_async/). |

## Optional labs

[LABS.md](LABS.md) has four larger projects: a blocking-work adapter, a small reactor over loopback sockets, an experiment with a real runtime, and a pinning audit. Each one lives in its own package outside this repository and is not checked by the course tests.

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

1. What promises accompany Poll::Pending, and why must futures not block in poll?
2. How do wakeups schedule another poll without carrying the result themselves?
3. Where must waker registration and completion state be synchronized to avoid missed wakeups?
4. How does thread::unpark avoid a wake-before-park race?
5. Which responsibilities belong to an executor, a reactor, and a future?
6. Why does pinning matter after polling begins, and which values can safely move through Unpin?
7. How can a future cooperate by bounding work per poll?

Redo the capstone from its contract in a later session. Explain one design tradeoff and create a case the supplied tests do not exercise.

## Solutions

Model solutions are in [solutions/exercises/chapter21/](../../solutions/exercises/chapter21/) (one file per lesson, with the predictions filled in) and [solutions/drills/ch21/](../../solutions/drills/ch21/) (each drill repaired, plus the expected outcome of every drill). Use them after an honest attempt and after the hints: compare your solution with the model, and note in NOTES.md what differs and why. Reading a solution first removes the struggle that makes the lesson stay in memory.

## Next

Take checkpoint 4 (`checkpoints/cp4_after_ch21`) at least two days after you finish this chapter. It tests chapters 1 to 21 without hints. Read the [checkpoint guide](../../checkpoints/README.md#when-to-take-one) first, and open the checkpoint's `SPEC.md` only when you start the timer. Until then, continue with [chapter 22](../chapter22/README.md).
