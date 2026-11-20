# Mastery module M3 — Concurrency by building the primitives

[All chapters](../../README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Chapter 19 taught you to *use* `Mutex`, `Arc`, channels and atomics. In this module you *build* them: a spin lock, a one-shot channel, `Arc` with `Weak`, a mutex that puts threads to sleep, a bounded channel, a lock-free stack and a wait-free queue. Each one is small. What makes them hard is that the compiler cannot check the part that matters. You have to state an invariant, choose the memory ordering of every atomic operation, and write the argument for every `unsafe` block yourself. Two tools then check your argument against the machine: **loom** runs a small scenario under every interleaving, and **Miri** detects undefined behaviour.

**Who this is for:** a learner who has finished the chapters and wants to understand concurrent Rust at the level of the people who write the standard library and the async runtimes. If you only want to use these types well, [Chapter 19](../../exercises/chapter19/README.md) is enough.

**Before you start:**

| You need | From |
| --- | --- |
| Threads, `Mutex`, `Condvar`, channels, `Arc`, atomics and `compare_exchange` loops | [Chapter 19](../../exercises/chapter19/README.md) |
| `unsafe`, raw pointers, `UnsafeCell`, SAFETY comments, a first Miri run | [Chapter 23](../../exercises/chapter23/README.md) |
| `Drop`, `Deref`, `Send` and `Sync` as marker traits | [Chapter 13](../../exercises/chapter13/README.md) |
| `Rc`, `Weak`, reference cycles | [Chapter 9](../../exercises/chapter09/README.md) |
| `MaybeUninit`, `NonNull`, `PhantomData` (helpful, explained again here) | Mastery module [M2](../m2_unsafe/README.md) |

**Time:** about 30 to 40 hours in total, as a rough estimate. [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. This material is dense: plan sessions of 60 to 90 minutes and expect to read each concepts section twice.

## Setup

This is a standalone Cargo package with its own `Cargo.lock`. It is **not** part of the root package, so the chapter checks never build it. Rust 1.99.0 is pinned in `rust-toolchain.toml`. Run every command of this module **from this directory**:

```sh
cd mastery/m3_concurrency
cargo test --no-run             # the starter compiles, in debug
cargo test --release --no-run   # and in release
```

The ordinary tests use only the standard library. The model tests need one crate, [loom](https://docs.rs/loom/0.7.3/loom/), pinned to version 0.7.3. It is downloaded the first time you run `cargo model`, and never built otherwise. Cargo needs a network connection one time before that: the first command in this package reads the crates.io index to check `Cargo.lock` (it prints `Updating crates.io index` and downloads no crate). After that, the ordinary tests also work offline. Miri needs the course's dated nightly toolchain with the `miri` component (`rustup toolchain install nightly-2026-11-20 --component miri --component rust-src`; [The nightly toolchain](../../README.md#the-nightly-toolchain) says why it is dated); it is optional but strongly recommended. loom does not need nightly: `cargo model` runs on the pinned Rust 1.99.0.

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, most tests fail because the supplied code has bugs.

## Three ways to check your work

```sh
cargo test l02_                    # 1. ordinary tests of lesson 02, on real threads
cargo test --release l02_          #    the same, optimised
cargo model l02_                   # 2. the model tests of lesson 02, under loom
cargo +nightly-2026-11-20 miri test l02_      # 3. the ordinary tests again, under Miri
```

1. **Ordinary tests** check behaviour, and *how* you solved the task: they count allocations and frees, compare sizes, and ask whether your types are `Send` and `Sync` for the right `T`. Tests that would hang with a wrong solution are run with a time limit and fail with a message instead. The time limit is only a safety net: no test passes or fails because of how fast something ran.
2. **Model tests** live in the `model` module at the bottom of each lesson. `cargo model` is an alias (see `.cargo/config.toml`) that does the same as `RUSTFLAGS="--cfg loom" cargo test --release --lib`, with its own target directory. Loom runs each model under every interleaving that the memory model allows. This is the check that rejects an ordering that is too weak, a lost wake-up, or a missing notification. An ordinary test almost never finds those.
3. **Miri** finds what the other two cannot see: reads of freed or uninitialised memory, data races on memory that loom does not track, and leaks. It is slow, so the tests do less work under Miri.

"How the model tests work" in [CONCEPTS.md](CONCEPTS.md) explains how to read a loom failure, and what loom 0.7.3 cannot check. Read it before lesson 01.

One rule makes the model tests meaningful: **import atomics, cells, threads, `Arc`, `Mutex` and `Condvar` from `crate::sync`, never from `std`.** `src/sync.rs` hands you the standard library's types in a normal build and loom's types in a model build.

A warning that is special to this module: your code contains `unsafe`. A wrong solution can crash the test program (a segmentation fault, or `SIGABRT`) instead of failing a test politely. That is a failed test too. Run the lesson under Miri to find out which access was wrong.

## Work through the module

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It gives the model, a worked example (never the exercise itself), and the mistakes people really make.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Lesson 01 has a second set in its `model` module.
3. **Implement** from the contract. Run the ordinary tests, then the model tests. A failed prediction or a model failure is feedback: write in [NOTES.md](NOTES.md) what you had assumed.
4. **Check technique.** Read the failure messages of the technique tests; they say what was expected and why. Then run `cargo clippy --all-targets` and fix the warnings in code that you wrote. Every `unsafe` block needs a `// SAFETY:` comment (clippy reports the ones without). Write a real argument there, not a description of the code. Before you start a lesson, clippy also warns in its tests about `drop` of "a value that does not implement `Drop`"; those warnings go away when your types own something.
5. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

The lessons give you less and less:

- **Lessons 01 to 03** give the types, their fields and the signatures. You write the bodies, the orderings and the safety arguments.
- **Lessons 04 to 07** give a contract in prose and only the public names, so that the tests compile. The fields, the helper types, the `Send`/`Sync` impls and the `Drop` impls are your design.
- **Lesson 08**, the capstone, is the same with nothing else: no hints in the code, and you add a model test of your own.

| Done | Lesson | Builds on | Time (estimate) |
| --- | --- | --- | --- |
| [ ] | [01 · Memory orderings and happens-before](src/l01_orderings.rs) | [Chapter 19](../../exercises/chapter19/README.md) | 3–4 h |
| [ ] | [02 · A spin lock with a guard](src/l02_spinlock.rs) | 01 | 2–3 h |
| [ ] | [03 · A one-shot channel](src/l03_oneshot.rs) | 01 | 3–4 h |
| [ ] | [04 · `MyArc` and `MyWeak`](src/l04_arc.rs) | 01, 03 | 4–5 h |
| [ ] | [05 · A mutex that puts waiting threads to sleep](src/l05_mutex.rs) | 02, 03 (uses your `SpinLock`) | 4–5 h |
| [ ] | [06 · A bounded channel with `Mutex` and `Condvar`](src/l06_channel.rs) | [Chapter 19](../../exercises/chapter19/README.md) | 3–4 h |
| [ ] | [07 · A lock-free stack, and the ABA problem](src/l07_treiber.rs) | 01, 04 | 4–5 h |
| [ ] | [Compiler drills](../../drills/m3/) — `cargo drill m3` (from the repository root) | 01–07 | 1 h |
| [ ] | [C01 · Code review: a teammate's lock-free slot pool](src/c01_code_review.rs) | 07 | 2–3 h |
| [ ] | [08 · Capstone: a wait-free ring buffer](src/l08_capstone.rs) | all | 4–6 h |

Lesson 05 calls the `SpinLock` that you wrote in lesson 02, so finish 02 first. All other lessons are independent of each other's code, but they are ordered so that each idea is used again later.

```sh
cargo test -- --list               # every ordinary test
cargo test l01_                    # one lesson
cargo test --release l01_
cargo model l01_                   # its model tests (downloads loom the first time)
cargo model                        # every model test
cargo +nightly-2026-11-20 miri test l04_      # one lesson under Miri
MIRIFLAGS="-Zmiri-many-seeds=0..16" cargo +nightly-2026-11-20 miri test l07_   # 16 different schedules
cargo test --doc                   # the code blocks of CONCEPTS.md
cargo clippy --all-targets
cargo fmt --check
cargo drill m3                     # from the repository root
```

If you prefer not to use the alias, the model tests are `RUSTFLAGS="--cfg loom" cargo test --release --lib l01_` (in PowerShell: `$env:RUSTFLAGS="--cfg loom"; cargo test --release --lib l01_`). 

**Check that the model tests really ran.** The names in the output of `cargo model` must contain `::model::`, for example `l01_orderings::model::a_two_threads_one_permit`. If they contain `::tests::` instead, the `--cfg loom` flag did not reach the compiler, and you ran the ordinary tests a second time. This happens when your shell sets the environment variable `RUSTFLAGS`: Cargo then uses that value and ignores the alias's flag. Use the explicit form above in that case, with your own flags added inside the quotes.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

## Topic map

"Optional lab" means an experiment that you run by hand. No test checks it.

| Topic | Where you practise it |
| --- | --- |
| Data races, happens-before, and what each `Ordering` adds | Lesson 01; every later lesson |
| Which results a small concurrent program can have (message passing, store buffering) | Lesson 01 (model predictions) |
| `Relaxed` counters and `compare_exchange` loops; check-then-act | Lesson 01 task A; drill d10 |
| Publication with `Release`/`Acquire` | Lesson 01 task B; lessons 02, 03, 07, 08 |
| When only `SeqCst` (or a `SeqCst` fence) is enough | Lesson 01 task C |
| `UnsafeCell`, an invariant, and SAFETY comments that use it | Lessons 02 to 08 |
| Guards: `Deref`, `DerefMut`, unlock in `Drop`, unlock during a panic | Lessons 02 and 05; drills d03, d04, d05 |
| `unsafe impl Send`/`Sync` with the right bounds; `PhantomData` markers | Lessons 02, 04, 05, 07, 08; drills d06, d07 |
| `MaybeUninit` and a state machine that tracks initialisation | Lessons 01 (task B), 03, 08 |
| `park`/`unpark`, the token, spurious wake-ups, lost wake-ups | Lessons 03 and 05; drill d09 |
| Reference counting: two counters, `Weak`, upgrade, overflow | Lesson 04; drill d08 |
| A waiting protocol with a queue: register, check again, sleep | Lesson 05 |
| Monitors: `Mutex` + `Condvar`, predicates, `notify_one` and `notify_all`, closing | Lesson 06 |
| Lock-free and wait-free; `AtomicPtr`; release sequences | Lessons 07 and 08 |
| The ABA problem and memory reclamation; version counters | Lesson 07 (predictions); C01 |
| Index arithmetic for ring buffers | Lesson 08 (predictions) |
| Reading loom reports; the limits of loom 0.7.3 | CONCEPTS "How the model tests work"; model labs in each lesson |
| Miri on concurrent code; many seeds | Optional labs in lessons 04, 07 and 08 |
| Reviewing lock-free code written by somebody else | C01 |
| Not covered here: hazard pointers and epoch-based reclamation in code, `RwLock`, futexes, async wakers (mastery module M4), cache effects (mastery module M7) | — |

## Mastery check

Answer from memory, in writing, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

1. Define "data race" and "happens-before". Name four ways in which two operations in different threads become ordered.
2. A thread stores a flag with `Relaxed` after writing a buffer; another thread loads the flag with `Acquire` and then reads the buffer. Is this correct? What exactly can go wrong, and which single change repairs it?
3. Give a program for which `Release`/`Acquire` is not enough and `SeqCst` is. Say why.
4. Why is `SpinLock<T>: Sync` for `T: Send`, but its guard `Sync` only for `T: Sync`?
5. Why must the waker change the state before it calls `unpark`, and why must the waiter call `park` in a loop?
6. Why does `Arc<T>` need `T: Send + Sync` to be `Send`? Give one type that shows the need for each bound.
7. Which ordering does `Arc`'s decrement need, and what is the `Acquire` fence for? Why is `Relaxed` enough for the increment?
8. In a mutex that parks its waiters: in which order must a waiting thread "register" and "try again", and in which order must the unlocking thread "free" and "look for waiters"? What happens if either pair is swapped?
9. State the three-part rule for condition variables. Give a reason for the loop that has nothing to do with spurious wake-ups.
10. Describe the ABA problem in a lock-free stack with a concrete interleaving. Name two ways to prevent it, and say which of them also prevents a use after free.
11. In a single-producer, single-consumer ring buffer, which two accesses does the `Release` store of `head` order, and why can each side load its own index with `Relaxed`?
12. What can loom check that Miri cannot, and what can Miri check that loom cannot? Name one thing that loom 0.7.3 gets wrong.

## Solutions

Model solutions are in the course repository under `solutions/mastery/m3_concurrency/` (the same paths as here), and solved drills under `solutions/drills/m3/`. A solution is the lesson file with the bodies filled in; its tests are the same as yours. Look at a solution only after your own version passes all three checks, or after you have spent real effort and the second hint did not help. Then compare the *safety comments* first: in this module the argument is the product, and the code is the easy part.
