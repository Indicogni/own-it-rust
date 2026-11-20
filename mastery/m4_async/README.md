# Mastery M4 — An async runtime from scratch, then a real service

[All chapters](../../README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Chapter 21 ended with an executor that runs one future. A real runtime runs thousands, puts them to sleep, wakes them when a timer fires or a socket has data, and stops them cleanly. In this module you build every one of those parts with the standard library only: a waker made from raw pointers, a task queue with `spawn` and `JoinHandle`, a timer driver, a reactor over non-blocking sockets, adapters that keep a future pinned in place, streams, `select`, and cancellation that loses no data. Then you put the parts together: the capstone is a chat server that runs on your own runtime and is tested over real loopback connections. The last lesson builds the same server on tokio and adds what a production service needs: backpressure, a connection limit, and graceful shutdown.

**Who this is for.** You have finished the core course and want to know what `async` code does when it runs, well enough to debug a service that hangs, loses a message, or will not shut down. The module assumes that you write `unsafe` only with a written safety argument, and it asks you for those arguments.

**Before you start.**

| You need | Because |
| --- | --- |
| [Chapter 19](../../exercises/chapter19/README.md) (threads, `Arc`, `Mutex`, `Condvar`, atomics) | The runtime is shared between the executor thread and two driver threads. |
| [Chapter 20](../../exercises/chapter20/README.md) (`async`/`.await`, cancellation by dropping, racing) | You implement what you used there. |
| [Chapter 21](../../exercises/chapter21/README.md), all lessons (the `Future` trait, wakers, `block_on`, pinning with `Box::pin`) | Lesson 01 starts where its lesson 04 stops. |
| [Chapter 23](../../exercises/chapter23/README.md) (raw pointers, `unsafe`, SAFETY comments) | Lessons 01, 05 and 06 contain `unsafe` code that you write. |
| [Chapter 18](../../exercises/chapter18/README.md) (TCP on loopback) | Lessons 04 and 08 use `std::net`. |
| Ecosystem lesson [E06](../../ecosystem/README.md#e06--tokio) (tokio basics) | Only for lesson 09. |

**Time.** About 26 to 29 hours in total, as a rough estimate. [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson.

## Setup

This is a standalone Cargo package with its own `Cargo.lock`. It is not part of the root package, so the chapter checks never build it. Rust 1.99.0 is pinned in `rust-toolchain.toml`, identical to the root one. Lessons 01 to 08 use only the standard library. Lesson 09 uses tokio 1.54.1 and tokio-util 0.7.20, pinned with `=`. Run all commands from the root of the course repository:

```sh
# The first build downloads the pinned crates; afterwards you can add --offline.
cargo test --manifest-path mastery/m4_async/Cargo.toml --locked --no-run
cargo test --manifest-path mastery/m4_async/Cargo.toml --locked --release --no-run
```

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, most tests fail because the supplied code has bugs. No test needs the network: sockets are bound to `127.0.0.1` on a port that the system chooses.

## Work through the module

Every lesson uses the same loop as the chapters:

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example that is not the exercise, and shows the mistakes and compiler errors you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. The predictions do not need your implementation.
3. **Implement** from the contracts and run the lesson, for example `cargo test --manifest-path mastery/m4_async/Cargo.toml l01_`.
4. **Check technique.** The tests look at *how* you solved a task: reference counts after each waker call, allocation counts, which thread made a wake, whether a pinned future was moved, whether a lock was still held during a wake. A solution that blocks for ever fails after a few seconds with a message instead of hanging the run. Then run clippy and fix the warnings in code that you wrote; it also reports every `unsafe` block without a `// SAFETY:` comment.
5. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

The scaffolding fades as you go. Lessons 01 and 02 give you every type and signature. Lessons 03 and 04 fix the public signatures and leave the fields to you. Lesson 05 gives the fields again, because "which field is pinned" is its topic. In lessons 06 and 07 you design the types behind a few fixed function signatures. The capstone gives you a protocol in prose and four method names. Lesson 09 gives you one function signature.

The lessons build on each other in ideas, but lessons 03 to 07 are tested with a small executor from `src/support.rs`, so a gap in an earlier lesson does not block a later one. The capstone is different: it runs on your own code from lessons 02 to 07.

| Done | Lesson | You build | Start after |
| --- | --- | --- | --- |
| [ ] | [l01 · RawWaker and its vtable, by hand](src/l01_raw_waker.rs) | a waker from raw pointers, three ways | [Chapter 21](../../exercises/chapter21/README.md) |
| [ ] | [l02 · Tasks, a run queue, spawn and JoinHandle](src/l02_tasks.rs) | a single-threaded executor | l01 |
| [ ] | [l03 · A timer heap and sleep](src/l03_timers.rs) | a timer driver and `Sleep` | l02 |
| [ ] | [l04 · A reactor over non-blocking sockets](src/l04_reactor.rs) | a reactor, `AsyncListener`, `AsyncStream` | l03 |
| [ ] | [l05 · Pin projection by hand](src/l05_projection.rs) | `Timeout`, `Fuse`, a two-state `Chain` | l02 |
| [ ] | [l06 · A Stream trait, a channel, and adapters](src/l06_streams.rs) | a bounded channel, `map`, `filter`, `merge` | l05 |
| [ ] | [l07 · Select and cancellation safety](src/l07_select.rs) | `select`, `CancelToken`, a line reader | l06 |
| [ ] | [Compiler drills](../../drills/m4/) — `cargo drill m4` | 10 predictions | l07 |
| [ ] | [C01 · Code review: a teammate's runtime helpers](src/c01_code_review.rs) | three repairs | l07 |
| [ ] | [l08 · Capstone: a chat server on your own runtime](src/l08_capstone_chat.rs) | the server | l02 to l07 |
| [ ] | [l09 · The same service on tokio](src/l09_tokio_service.rs) | backpressure, limits, graceful shutdown | l08, [E06](../../ecosystem/README.md#e06--tokio) |

```sh
cargo test --manifest-path mastery/m4_async/Cargo.toml -- --list
cargo test --manifest-path mastery/m4_async/Cargo.toml l01_
cargo test --manifest-path mastery/m4_async/Cargo.toml l02_tasks::tests::a_
# After completing all lessons:
cargo test --manifest-path mastery/m4_async/Cargo.toml
cargo test --manifest-path mastery/m4_async/Cargo.toml --release
cargo clippy --manifest-path mastery/m4_async/Cargo.toml --all-targets
cargo drill m4
cargo run --manifest-path mastery/m4_async/Cargo.toml --example l09_shutdown
```

Test names start with the letters of the tasks they exercise (`a_`, `b_c_`, ...). A filtered run also starts the documentation tests of `CONCEPTS.md` and reports them as filtered out; `cargo test --doc` runs them.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

**After this module.** Lesson 09 ends with one small service on tokio. Mastery module [M11 · Rust as a service](../m11_services/README.md) continues from there: an HTTP API on axum and tower, a SQL database, login, and the logs, metrics and shutdown behaviour that a service in production needs. M11 does not need this module, but this module explains what the runtime under M11 does.

### Miri

Lessons 01, 05 and 06 ask you to write `unsafe` code, and the code review C01 has you repair some. The ordinary tests can miss a use after free or a leak. [Miri](https://github.com/rust-lang/miri) is an interpreter that reports them. It needs the course's dated nightly toolchain, `nightly-2026-11-20` (see [The nightly toolchain](../../README.md#the-nightly-toolchain)), which this module does not require otherwise:

```sh
rustup toolchain install nightly-2026-11-20 --component miri --component rust-src
cargo +nightly-2026-11-20 miri test --manifest-path mastery/m4_async/Cargo.toml l01_
cargo +nightly-2026-11-20 miri test --manifest-path mastery/m4_async/Cargo.toml l05_
cargo +nightly-2026-11-20 miri test --manifest-path mastery/m4_async/Cargo.toml l06_
cargo +nightly-2026-11-20 miri test --manifest-path mastery/m4_async/Cargo.toml c01_
```

The model solutions pass these runs. Do not run Miri on lessons 04, 08 and 09: they use sockets and real time, which Miri does not model well, and the runs take very long.

## Topic map

| Topic | Where you practise it |
| --- | --- |
| The layout of a `Waker`: data pointer and vtable | Lesson 01 (predictions); drill d07 |
| The four vtable functions and who owns the reference in each | Lesson 01 (task B); C01 (task A) |
| A waker without reference counting | Lesson 01 (task C); the worked example in CONCEPTS |
| Tasks, a run queue, and waking as "put on the queue" | Lesson 02 (tasks A and B) |
| Queueing a task at most once | Lesson 02 (`a_many_wakes_give_one_poll`) |
| `spawn`, `JoinHandle`, detaching, aborting | Lesson 02 (tasks C and D) |
| Why spawned futures must be `Send + 'static` | Lesson 02 (CONCEPTS); drills d03 and d04 |
| Shutting an executor down: who drops the futures, and outside which lock | Lesson 02 (task D); lesson 08 |
| A priority queue of deadlines; lazy deletion | Lesson 03 (task A); drills d05 and d06 |
| A driver thread that sleeps until the next deadline | Lesson 03 (task B) |
| The lock rule: no wake and no drop of a waker under a lock | Lessons 03, 04, 06, 07; C01 (task B) |
| Non-blocking system calls and `WouldBlock` | Lesson 04 (predictions, task B) |
| Registering interest and being woken; why no wakeup is lost | Lesson 04 (tasks A and B); drill d10 |
| Partial writes, end of stream, non-blocking mode of accepted sockets | Lesson 04 (task C) |
| What epoll, kqueue and IOCP add to a polling reactor | Lesson 04 (CONCEPTS, EXPLAIN) |
| Structural pinning and its four duties | Lesson 05; lesson 06 (adapters); drill d01 |
| `Pin::set`, `Option::as_pin_mut`, dropping in place | Lesson 05 (tasks B and C) |
| A state machine like the one the compiler writes for `async` | Lesson 05 (task C) |
| `Unpin`, `PhantomPinned`, `pin!` | Lesson 05; drill d02 |
| The `Stream` trait and its three answers | Lesson 06 (predictions) |
| A bounded channel that ends when its senders are gone | Lesson 06 (task A) |
| Lazy adapters and fairness in `merge` | Lesson 06 (tasks B and C) |
| `select`: first future wins, the other is dropped | Lesson 07 (task A) |
| Cancellation safety: state in the owner, not in the future | Lesson 07 (task C); drill d09 |
| Cleanup that also runs when a future is cancelled | C01 (task C); lesson 08 |
| A cancellation token with many waiters | Lesson 07 (task B) |
| `impl Trait` captures in edition 2024 and `use<>` | Lesson 07 (task B); drill d08 |
| Fairness between tasks: yield after a unit of work | Lesson 08 (rule 7) |
| Bounded queues per client; dropping a slow client | Lesson 08 (rule 6) |
| Backpressure: making the sender wait | Lesson 09 |
| Connection limits with a semaphore | Lesson 09 |
| Graceful shutdown with a grace period and abort | Lessons 08 and 09 |
| Backpressure meets shutdown: the wait for room must have an exit | Lesson 09 (`d_shutdown_reaches_a_client_whose_line_waits_for_room`) |
| Testing a service with in-memory pipes and paused time | Lesson 09 |
| A multi-threaded, work-stealing executor | Not in this module. Lesson 09 uses tokio's. Module M3 has the building blocks. |
| A reactor on epoll, kqueue or IOCP | Not in this module (it needs platform calls or a crate such as mio). Lesson 04 explains the difference. |

## Solutions

Model solutions are in [`solutions/mastery/m4_async/`](../../solutions/mastery/m4_async/), in files with the same names, and the solved drills are in [`solutions/drills/m4/`](../../solutions/drills/m4/). `cargo progress solution m4` lists them. They are checked by the course: with the solutions in place, every test of this package passes in debug and release builds, clippy is clean, and the `unsafe` lessons pass under Miri. Use them after your own attempt. The value of this module is in the hours you spend finding out why your runtime hangs.

## Mastery check

Answer from memory, in writing, before you call the module finished. Model answers are at the end of [HINTS.md](HINTS.md).

1. A `Waker` is cloned on thread A and woken by value on thread B. List what each of the four vtable functions must do to the reference count, and say what goes wrong if `wake` forgets its part and if `wake_by_ref` does too much.
2. A task returns `Pending` and is never polled again. Name five different places in a runtime where the wakeup can have been lost, one per lesson from 02 to 06, and the rule that prevents each.
3. Why does `Spawner::spawn` demand `Send + 'static`, although your executor polls every task on one thread?
4. State the lock rule of this module, and give one deadlock that breaking it causes.
5. What exactly does `Pin<&mut T>` promise, and which four duties do you take on when you project it to a field? Which of them does `Option::take` on a pinned field break?
6. Your reactor wakes every waiting task once per millisecond. What does a reactor on epoll or kqueue do instead, and what stays the same for the code that calls `poll_io`?
7. Define cancellation safety. Then say why `next_line` of lesson 07 has it and `write_all` of lesson 04 does not, and what the capstone does about the second one.
8. A client stops reading. Describe what happens in lesson 08 and in lesson 09, and give one reason to prefer each policy.
9. In which order must a runtime drop its executor, its timer driver and its reactor, and why?
10. Lesson 09 tests most behaviour with in-memory pipes and a paused clock. What can such a test prove that a test over real sockets cannot, and what can it not prove?

## Validation scope

The starters compile in debug and release with `--locked`. With the model solutions in place, all tests pass in both profiles, `cargo clippy --all-targets` is clean, every code block in `CONCEPTS.md` is compiled and run, and each technique test was checked against at least one plausible wrong solution. The tests of lessons 03, 07, 08 and 09 also pass with a second solution of a different design (an ordered map in place of the heap, a hand-written `Select` future, two tasks per connection, one `select!` loop with `reserve` in place of `join!`), so they check the contract and not one way to meet it. Tests that involve real time assert lower bounds exactly and upper bounds only with a margin of a second or more; the few sleeps in the tests measure a quiet period or pace a client that talks, they do not synchronise threads.
