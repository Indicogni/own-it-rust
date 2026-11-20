# Mastery M7 — Performance engineering

[All chapters](../../README.md) · [Concepts](CONCEPTS.md) · [Labs](LABS.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Rust gives you control over memory, dispatch and input/output, and it does not make a program fast by itself. This module teaches the work of making a program faster in a way that you can defend: measure, find where the cost is, change one thing, and prove that the change helped and that nothing broke. You build the core of a benchmark harness and a small profiler, so that you know what the real tools do. You remove heap allocations against a budget. You shrink types and reorder data so that it uses the cache well. You write loops that need no bounds checks. You choose between generics, trait objects and enums with their costs in mind. You buffer input and output correctly, including the error cases. At the end you tune a slow, correct program until it meets its budgets.

**Who this is for.** You have finished the chapters and can write correct Rust. You want to be the person on a team who can say why a program is slow, and who does not guess.

**Before you start:** chapters [1](../../exercises/chapter01/README.md) to [18](../../exercises/chapter18/README.md) and [chapter 23](../../exercises/chapter23/README.md). The ecosystem track's [E08](../../ecosystem/TOOLING.md) (tooling) is a gentle start. Mastery [M1](../m1_types/README.md) (lesson 05) and [M2](../m2_unsafe/README.md) help with lessons 06 and 05, but are not required. [CONCEPTS.md](CONCEPTS.md) names what each lesson assumes.

**Time:** about 22 to 25 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson.

## How this module tests performance

No test in this module reads a clock. A test that demands "less than 5 ms" fails on a slow or busy machine and passes on a fast one, whatever your code does. The tests count instead:

| What is counted | How | Lessons |
| --- | --- | --- |
| heap allocations, bytes requested, peak heap use | a counting allocator in `src/support.rs` | all |
| clock readings and what happens between them | a fake clock | 01 |
| comparisons and clones | values that count them | 02, 08 |
| cache misses | a small model of a cache | 05 |
| bounds checks | a slice type that counts them | 05 |
| calls through a trait object | a stage that counts its calls | 06 |
| calls of `read`, `write` and `flush` | a reader and a writer that count | 07, 08 |

A count is the same on every machine and in every run. A limit on a count is a **budget**, and a budget makes a good test. What a count is worth in time is a question for your machine: the benchmarks in `benches/` and the labs in [LABS.md](LABS.md) answer it.

## Setup

This is a standalone Cargo package with its own `Cargo.lock`. It is **not** part of the root package, so the chapter checks never download or build its one dependency, the benchmark harness [criterion](https://crates.io/crates/criterion) (pinned to 0.8.2, a development dependency). The lessons themselves use only the standard library. Rust 1.99.0 is pinned in `rust-toolchain.toml`, identical to the root one. Run commands from the root of the course repository:

```sh
# The first build downloads criterion; afterwards you can add --offline.
cargo test --manifest-path mastery/m7_perf/Cargo.toml --locked --all-targets --no-run
```

One of criterion's own dependencies (`alloca`) compiles a few lines of C in its build script. It uses the C compiler that a Rust installation needs for linking anyway: the Xcode command line tools on macOS, `gcc` or `clang` on Linux, the MSVC build tools on Windows. If the first build stops at `failed to run custom build command for alloca`, that compiler is missing.

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in. In lesson 06 (task D), in the code review lesson and in the capstone, the supplied code works, and it is the budget tests that fail.

## Work through the module

Every lesson uses the same loop as the chapters:

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. The predictions do not need your implementation.
3. **Implement** the tasks and run the lesson, for example `cargo test --manifest-path mastery/m7_perf/Cargo.toml l01_`. Run it with `--release` too.
4. **Check technique.** Most tests of this module are technique tests: they pass only when the count is within the budget, and the failure message says what was counted. Then run clippy and fix the warnings in code that you wrote.
5. **Do the lab** that the lesson header names, in [LABS.md](LABS.md). This is where you see times.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

The scaffolding fades. Lessons 01 to 04 give you signatures. Lesson 05 gives you signatures and leaves the structure of the solution to you. In lessons 06 and 07, a task is a contract in prose: the file holds only the names that the tests use, and the fields, helper types and private functions are your design. The capstone gives you a slow program and its budgets, and nothing else.

| Done | Lesson | Start after | Lab |
| --- | --- | --- | --- |
| [ ] | [01 · Benchmark design](src/l01_bench_design.rs) | chapters 14, 15; E08 helps | [Lab 1](LABS.md#lab-1--a-benchmark-that-measures-nothing) |
| [ ] | [02 · Reading a profile](src/l02_profile.rs) | lesson 01; chapters 9, 13 | [Lab 2](LABS.md#lab-2--record-and-read-a-real-profile) |
| [ ] | [03 · Allocation budgets](src/l03_allocations.rs) | lesson 02; chapters 13, 16, 17 | |
| [ ] | [04 · Data layout](src/l04_layout.rs) | lesson 03; chapters 9, 10 | [Lab 4](LABS.md#lab-4--an-array-of-structs-against-a-struct-of-arrays) |
| [ ] | [05 · Traversal and bounds checks](src/l05_traversal.rs) | lesson 04; chapters 15, 16 | [Lab 5](LABS.md#lab-5--traversal-order-in-time-and-bounds-checks-in-the-compilers-output) |
| [ ] | [06 · Dispatch, inlining and code size](src/l06_dispatch.rs) | lesson 05; chapters 11, 14 | [Lab 6](LABS.md#lab-6--how-many-copies-of-a-generic-function) |
| [ ] | [07 · Buffering](src/l07_io.rs) | lesson 03; chapters 13, 18 | |
| [ ] | [Compiler drills](../../drills/m7/) — `cargo drill m7` | lessons 03 to 07 | |
| [ ] | [08 · Capstone: tune a slow log summary](src/l08_capstone.rs) | lessons 01 to 07 | [Lab 8](LABS.md#lab-8--the-capstone-in-numbers) |
| [ ] | [C01 · Code review: "make it faster"](src/c01_code_review.rs) | lessons 03, 05, 07 | |

Lessons 01 to 06 build on each other. Lesson 07 needs only lesson 03. Do the code review lesson after the capstone, or between lessons as a change.

```sh
cargo test --manifest-path mastery/m7_perf/Cargo.toml l01_
cargo test --manifest-path mastery/m7_perf/Cargo.toml -- --list
cargo test --manifest-path mastery/m7_perf/Cargo.toml --lib
cargo test --manifest-path mastery/m7_perf/Cargo.toml --lib --release
cargo test --manifest-path mastery/m7_perf/Cargo.toml --doc
cargo clippy --manifest-path mastery/m7_perf/Cargo.toml --all-targets
cargo drill m7
cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l01_black_box
cargo run --release --manifest-path mastery/m7_perf/Cargo.toml --example usage
```

`--lib` runs only the lessons, and `--doc` compiles and runs every code block in CONCEPTS.md; plain `cargo test` does both. With `--all-targets`, Cargo also runs every benchmark once as a quick check. The benchmarks for lessons 04 and 05 stop at `todo!()` until those lessons are done.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

## The files

| Path | What it is |
| --- | --- |
| `src/lNN_*.rs`, `src/c01_code_review.rs` | the lessons |
| `src/support.rs` | `guess()`, the counting allocator, and `within()`; you do not edit it, and reading it is worth ten minutes |
| `benches/` | criterion benchmarks for lessons 01, 04, 05 and 08 |
| `examples/usage.rs` | prints the heap use of the capstone's baseline and of your version |
| `examples/profile_target.rs` | a slow program for the profiling lab |
| `labs/` | three small files that you compile with `rustc` in labs 1, 5 and 6 |
| `CONCEPTS.md`, `LABS.md`, `HINTS.md`, `NOTES.md` | teaching text, labs, hints, your workbook |

## Topic map

"Lab" means work with a real tool that you run by hand. No test checks it.

| Topic | Where you practise it |
| --- | --- |
| What a harness does: batches, calibration, setup outside the timed region | Lesson 01, tasks A to C |
| Robust statistics: median, median absolute deviation, a noise threshold | Lesson 01, tasks D and E |
| `black_box`, constant folding, dead code elimination | Lesson 01 (concepts); Lab 1 |
| Reading criterion output; saving and comparing baselines | Labs 1 and 8 |
| Writing a criterion benchmark; `iter_batched` for a routine that changes its input | Lab 1 |
| Self cost and total cost; folded stacks; flamegraphs | Lesson 02, task A; Lab 2 |
| Guards (`Drop`) that end a region on every way out | Lesson 02, task A |
| The cost of the measuring code itself | Lesson 02 (allocation test); lesson 01, task B |
| Amdahl's law: how much a change can gain | Lesson 02 (prediction P3) |
| Finding and removing a quadratic cost from a profile | Lesson 02, task B |
| Exact capacity; growth by doubling | Lesson 03, tasks A and B |
| Reusing a buffer; `clear` against `mem::take` | Lesson 03, tasks B and E; C01 |
| `Cow`: owning only when needed | Lesson 03, task C |
| Looking up with a borrowed key before making an owned one | Lesson 03, task D; capstone |
| Size, alignment, padding; `repr(C)` field order | Lesson 04, task A; drill d02 |
| Niches and `NonZero` types | Lesson 04, task B; drill d06 |
| Boxing a large, rare enum variant | Lesson 04, task C |
| Array of structs against struct of arrays | Lesson 04, task D; Lab 4 |
| `repr(packed)` and why references into it are refused | Lesson 04 (concepts); drill d10 |
| Cache lines, traversal order, loop fusion | Lesson 05, task A; Lab 5 |
| Blocking (tiling) a loop | Lesson 05, task B |
| Loops without bounds checks: iterators, a check in front of the loop | Lesson 05, tasks C and D; drills d03, d07, d08; Lab 5 |
| `get_unchecked` with a complete safety argument | Lesson 05 (concepts) |
| Static and dynamic dispatch; one virtual call per batch | Lesson 06, task A |
| A thin generic shell around a non-generic core; counting copies | Lesson 06, task B; Lab 6 |
| Enum dispatch for a closed set | Lesson 06, task C |
| Closures stored inline against boxed closures | Lesson 06, task D; drill d09 |
| `#[inline]`, `#[inline(never)]`, `#[cold]` | Lesson 06 (concepts); Labs 1 and 5 |
| Dyn compatibility | Lesson 06 (concepts); drill d05 |
| `BufWriter`, flushing, and errors that arrive late | Lesson 07, task A; drill d04; C01 |
| `BufReader`, `read_line` with one reused `String`, bounded memory | Lesson 07, task B |
| The contract of `Write::write`: short writes, `Interrupted`, `Ok(0)` | Lesson 07, task C |
| A buffer type with `Drop` and `into_inner` | Lesson 07, task C |
| The tuning loop; differential testing; a performance report | Lesson 08; Lab 8 |
| Keeping the best `k` of `n` items with a heap | Lesson 08 |
| Reviewing an "optimization" that changed behavior | C01 |

**Not in this module.** SIMD with `std::arch`, link-time and profile-guided optimization, allocator replacement, and parallel speed-up are named in CONCEPTS (lesson 08) with links, and are not practised here. Atomics and lock-free structures are mastery M3.

## Mastery check

Answer from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

1. A benchmark of a function that sums 10,000 numbers reports 0.3 ns. Give two possible causes and say how you would confirm each.
2. Why does a harness time a batch of calls with two clock readings, and how does it choose the batch size?
3. A profile shows `render` with a total cost of 70% and a self cost of 3%, and `escape` with a total and self cost of 55%. What do you look at first, and what is the most that a faster `escape` can gain?
4. Name four ways to remove a heap allocation, and one situation for each.
5. A `repr(C)` struct has the fields `u8`, `u64`, `u16`, `u32`, `u8` in this order. What is its size, and what is the smallest size you can reach by reordering?
6. Why is `Option<Box<T>>` as large as `Box<T>`, and `Option<u64>` larger than `u64`?
7. When is a struct of arrays faster than an array of structs, when is it slower, and what does it cost in code?
8. A loop reads a large grid column by column. Why can it be much slower than a loop that reads the same grid row by row?
9. Three ways to add two slices element by element: an index loop, an index loop after taking sub-slices, and `zip`. How do they differ in bounds checks, and how do they differ in meaning?
10. What does a call through `dyn Trait` cost, what does a generic function cost, and how can a trait be designed so that a `dyn` call is cheap per element?
11. A function writes through a `BufWriter` and returns `Ok(())` without flushing. What can go wrong, and why does `Drop` not save you?
12. Why can a buffered writer not use `write_all` to empty its buffer if it must be able to continue after an error?
13. You are asked to make a program faster. Describe the steps you take before you change the first line, and how you know when to stop.

## Solutions and validation

Model solutions are in `solutions/mastery/m7_perf/` at the top of the course repository, under the same paths as here (for example `solutions/mastery/m7_perf/src/l03_allocations.rs`), and the fixed drills are in `solutions/drills/m7/`. Each solution is the lesson file with only the task bodies, the redesigned types and the predictions changed; the tests are the same. `cargo progress solution m7 l03` prints where a solution is without showing it. Look at a solution only after your own tests pass, or after both hints did not help: comparing a working answer of your own with the model teaches far more than reading the model first.

The starters compile in debug and release with `--locked`. With the solutions in place, every test passes in both profiles and `cargo clippy --all-targets` is clean. Every technique test was checked against at least one plausible wrong solution, which it rejects.
