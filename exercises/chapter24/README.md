# Chapter 24 — Calling C

[All chapters](../../README.md) · [Previous chapter](../chapter23/README.md)

Most of the software that already exists is not written in Rust, and much of it can be reached through the C calling rules. This chapter teaches you to cross that border without losing what Rust gives you. You describe C data to Rust byte for byte, move strings between the two worlds, and call functions that report failure with a status code and return results through pointers. You pass callbacks to a library, own a library object so that it is freed exactly once, and give a borrowed foreign pointer a lifetime that the compiler can check. Then you call functions from the real C library, where no compiler checks your declarations. The aim is always the same: a small amount of `unsafe` code with a written reason, behind a safe interface that its users cannot misuse.

**Before you start:** Chapter 23 (unsafe code), and Chapters 4, 5, 7 and 13. **Time:** about 8 to 9 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. Sessions of 45 to 60 minutes work well.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example, and shows the compiler messages that you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run the lesson, for example `cargo test --test chapter24 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations, compare pointers, or count how often the foreign function was called. So a wrapper that copies a buffer it could borrow, or that computes in Rust what the library should compute, fails, and the failure message says what was expected. Then run `cargo clippy --test chapter24` and fix the warnings in code that you wrote. Write a `// SAFETY:` comment for every `unsafe` block.
5. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

Do the lessons in the order of the table. Lesson 08 comes before the capstone, which keeps its file name `l07`. After lesson 08, **drill the compiler**: `cargo drill ch24` shows ten small programs. For each one, predict whether it compiles (and with which error code), runs, or panics; then fix it. After that, do the **capstone**, the **review** lesson (r01: Chapters 18, 22 and 23, from memory) and the **code review** lesson (c01: repair a wrapper that works on the happy path only). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson | Coding tasks |
| --- | --- | --- |
| [ ] | [01 · C representations](l01_representations.rs) | 2 |
| [ ] | [02 · C strings and ownership](l02_c_strings.rs) | 3 |
| [ ] | [03 · ABI calls and out-parameters](l03_abi_status.rs) | 2 |
| [ ] | [04 · Callbacks and userdata](l04_callbacks.rs) | 2 |
| [ ] | [05 · Opaque handle ownership](l05_handles.rs) | 2 |
| [ ] | [06 · Owner-bound foreign borrows](l06_borrows.rs) | 2 |
| [ ] | [08 · Calling the real C library](l08_libc.rs) | 5 |
| [ ] | [Compiler drills](../../drills/ch24/) — `cargo drill ch24` | 10 |
| [ ] | [07 · Capstone: checked foreign transaction](l07_capstone.rs) | 2 |
| [ ] | [R01 · Review: Chapters 18, 22, and 23 from memory](r01_review.rs) | 3 |
| [ ] | [C01 · Code review: a teammate's FFI wrapper](c01_code_review.rs) | 3 |

```sh
cargo test --test chapter24 l01_
cargo test --test chapter24 -- --list
cargo test --test chapter24
cargo test --release --test chapter24
cargo clippy --test chapter24
cargo drill ch24
cargo run --example ch24_scratch
```

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, most tests fail because the supplied code has bugs. Keep the supplied tests as they are. Every task uses only the standard library, and needs no network and no C compiler. Each lesson is independent, so an unfinished later lesson does not block an earlier one. Compiler experiments belong in [the scratch example](../../examples/ch24_scratch.rs); leave it compiling when you finish.

In lessons 01 to 07 the foreign functions are written in Rust with the C calling rules, so that the tests can observe them. In lesson 08 they are real compiled C code from the C library that every Rust program links on Linux, macOS and Windows. One warning for that lesson: a panic inside an `extern "C"` function stops the whole test run, not one test. The lesson and its hints tell you what to look for.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

## Topic map

"Optional lab" means a project or experiment that you do by hand. No test checks it.

| Topic | Where you practise it |
| --- | --- |
| C type aliases (`c_int`, `c_char`, ...) and why their sizes depend on the target | Lesson 01 |
| `#[repr(C)]` structs: field order, alignment, padding, offsets | Lesson 01; R01 task C |
| Enum representations at the boundary | Lesson 01 (predictions) |
| `CString` and `CStr`: who owns the bytes, the NUL at the end, UTF-8 checks | Lesson 02; drills d05, d06, d08, d10 |
| Status codes and out-parameters with `MaybeUninit` | Lesson 03; capstone; C01 |
| `extern "C"` function pointers and their types | Lesson 03 task B; drill d07 |
| Callbacks with a userdata pointer; how long the pointer must live | Lesson 04; C01 |
| Nullable function pointers as `Option<extern "C" fn>` | Lesson 04 task B; drill d09 |
| Why a panic must not leave an `extern "C"` function | Lesson 04 (concepts and explanation); lesson 08 task D |
| Owning an opaque handle: `NonNull`, `Drop`, exactly one free | Lesson 05 |
| Giving a borrowed foreign pointer a lifetime with `PhantomData` | Lesson 06 |
| Writing an `unsafe extern "C"` block from C prototypes | Lesson 08 task A; drills d01 to d03 |
| Calling real C library functions (`strlen`, `abs`, `qsort`) behind safe wrappers | Lesson 08 tasks B to E |
| A comparator that C calls back, and passing count and element size | Lesson 08 tasks D and E |
| Exporting a symbol with `#[unsafe(no_mangle)]` | Drill d04; optional lab 1 |
| Copying a foreign error message before the next call; stopping at the first error | Lesson 07 (capstone); C01 |
| Writing a record as explicit bytes instead of dumping a struct | R01 task C |
| Filling a `Vec` from a foreign function without placeholder values | R01 task A |
| Reviewing a wrapper for wrong status checks, hidden truncation and missing SAFETY comments | C01 |
| Compiling and linking your own C code; a `catch_unwind` trampoline | Optional lab 1 in [LABS.md](LABS.md) |
| Binding a system library (SQLite) with a raw and a safe layer | Optional lab 2 in [LABS.md](LABS.md) |

Not covered in this chapter: generating declarations with `bindgen` or `cbindgen`, `-sys` crates and build scripts, static against dynamic linking, variadic functions, unions, and `Send`/`Sync` for handles. The mastery module [`mastery/m6_ffi`](../../mastery/m6_ffi/README.md) treats these with a real C library.

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

1. How do `#[repr(C)]`, the C type aliases, `usize` for `size_t`, pointers and ABI declarations make two languages agree on the same bytes?
2. Why do `CString` and `CStr` exist as two types, and what is the lifetime of the pointer that `as_ptr` returns?
3. When is a foreign output parameter initialized, and what may you read after a failure?
4. Which allocator frees each foreign object, and how does a wrapper make sure that cleanup happens exactly once, also on an early return?
5. How does a borrowed foreign object keep the lifetime of its owner?
6. How long must callback userdata live, and what happens when a panic reaches the edge of an `extern "C"` function?
7. Who checks an `unsafe extern` block, and what does each of the words `unsafe extern`, `safe fn` and `unsafe { ... }` promise?
8. Which facts about a C library must a safe wrapper find out and write down (initialization and shutdown, threads, error messages, versions, linking)?

Redo the capstone from its contract in a later session, without looking at your first solution. Explain one design choice, and add a test case that the supplied tests do not have.

## Solutions

Model solutions are in [solutions/exercises/chapter24/](../../solutions/exercises/chapter24/) (one file per lesson, with the predictions filled in) and [solutions/drills/ch24/](../../solutions/drills/ch24/) (each drill repaired, plus the expected outcome of every drill). Use them after an honest attempt and after the hints: compare your solution with the model, and note in NOTES.md what differs and why. Reading a solution first removes the struggle that makes the lesson stay in memory. The optional labs have no model solution.

## Extension labs

- [LABS.md](LABS.md): two optional projects with real C code and a real system library.

## Validation scope

Use the pinned Rust 1.99.0 toolchain and run commands from the root of the course repository. `cargo test --offline --test chapter24 --no-run` checks that the starters compile without running them. Lesson 08 calls only functions of the standard C library with the same prototype on Linux, macOS and Windows. The optional labs are manual and are not part of any automated check.

## Next

This is the last chapter of the core course. From here you can go on with the [ecosystem track](../../ecosystem/README.md) (the crates and tools that production code uses), finish the [running project](../../project/README.md), and start the mastery modules with [M1 · The type system in depth](../../mastery/m1_types/README.md). The [course overview](../../README.md) shows the whole map, including the capstones.
