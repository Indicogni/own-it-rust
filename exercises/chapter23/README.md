# Chapter 23 — Unsafe Rust

[All chapters](../../README.md) · [Previous chapter](../chapter22/README.md) · [Next chapter](../chapter24/README.md)

Safe Rust rejects every program whose memory safety the compiler cannot prove. A few correct programs need operations that the compiler cannot check: reading through a raw pointer, building a slice from an address and a length, moving a value out of a slot by hand, or changing a value behind a shared reference. The keyword `unsafe` permits these operations and moves the proof from the compiler to you. In this chapter you learn to write that proof. You keep unsafe code small, you hide it behind safe functions that no caller can misuse, and you state for every `unsafe` block why its operation is valid. You build a checked split of a slice, a tagged pointer, a gap buffer, a small `Cell`, and a container over uninitialized memory. You also learn what your tests cannot see, and you use Miri to see more of it.

**Before you start:** Chapters 3 to 5 (types, ownership, references) and Chapter 13 (`Drop`, `Clone`, `Copy`). The review lesson also uses Chapters 17, 21 and 22. **Time:** about 11 to 12 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. Sessions of 45 to 60 minutes work well.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example, and shows the compiler errors you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run the lesson, for example `cargo test --test chapter23 l01_`. Write a `// SAFETY:` comment above every `unsafe` block *before* you run it. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations, compare pointers, or count drops, so a solution that copies instead of moving fails, and the failure message says what was expected. A few tests read the text of your function and look for the operation that the contract names (see "What the tests can and cannot check" below). Then run `cargo clippy --test chapter23` and fix the warnings in code that you wrote. Never change a function's signature unless its doc comment says that you may.
5. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

After lesson 09, **drill the compiler**: `cargo drill ch23` shows ten small programs. For each one, predict whether it compiles (and with which error code), runs, or panics; then fix it. After that, do the **capstone** (lesson 08), the **review** lesson (r01: Chapters 17, 21 and 22, from memory), the **code review** lesson (c01: repair unsafe code that looks finished but is wrong), and **Lab 1** in [LABS.md](LABS.md), which runs your finished chapter under Miri. Run `cargo progress` to see where you are and which chapters are due for spaced review.

Each lesson is independent, including the capstone, so an unfinished later task does not block an earlier lesson.

| Done | Lesson | Coding tasks |
| --- | --- | --- |
| [ ] | [l01 · A safe invariant boundary](l01_ascii.rs) | 2 |
| [ ] | [l02 · Raw pointer reads with contracts](l02_pointers.rs) | 2 |
| [ ] | [l03 · Raw slice reconstruction](l03_raw_slices.rs) | 2 |
| [ ] | [l04 · A flagged borrow](l04_tagged.rs) | 2 |
| [ ] | [l05 · Layout, unsafe traits, unions](l05_representations.rs) | 3 |
| [ ] | [l06 · Moving values and initialization](l06_moves.rs) | 2 |
| [ ] | [l07 · A fixed-capacity byte gap buffer](l07_gap_buffer.rs) | 3 |
| [ ] | [l09 · Interior mutability and layout attributes](l09_cells_layout.rs) | 3 |
| [ ] | [Compiler drills](../../drills/ch23/) — `cargo drill ch23` | 10 |
| [ ] | [l08 · Capstone: initialized-prefix storage](l08_capstone.rs) | 3 |
| [ ] | [R01 · Review: Chapters 17, 21, and 22 from memory](r01_review.rs) | 3 |
| [ ] | [C01 · Code review: a teammate's unsafe pull request](c01_code_review.rs) | 3 |
| [ ] | [Lab 1 · Run your unsafe code under Miri](LABS.md#lab-1--run-your-unsafe-code-under-miri) | — |

```sh
cargo test --test chapter23 l01_
cargo test --test chapter23 -- --list
cargo test --test chapter23
cargo test --release --test chapter23
cargo clippy --test chapter23
cargo drill ch23
cargo run --example ch23_scratch
cargo +nightly-2026-11-20 miri test --test chapter23
```

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, most tests fail because the supplied code has bugs. Keep the supplied tests as they are. Every task uses only the standard library and needs no network. The last command needs a nightly toolchain with Miri; Lab 1 explains the setup.

The first build prints one compiler warning, `warning[E0133]: call to unsafe function ... is unsafe and requires unsafe block`, for `c01_code_review.rs`. It is part of the code review lesson and disappears when you repair that code.

Compiler experiments belong in [the scratch example](../../examples/ch23_scratch.rs); leave it compiling when you finish.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

## What the tests can and cannot check

The tests check behavior, pointer identity, allocation counts and drop counts. They cannot check that an `unsafe` block is sound, and a green test run does not mean that it is.

- **A safe shortcut often passes the behavior tests.** `values.pop()` behaves like the raw pop of lesson 06. For that reason, some technique tests read the text of your function and look for the operation named in the contract (for example `set_len`, `map_addr` or `read_unaligned`). This is a simple text search. It reminds you of the contract; it proves nothing.
- **Undefined behavior often passes too.** A misaligned read or an overlapping `copy_nonoverlapping` usually "works" in a release build on a desktop machine. Debug builds catch a few such mistakes with built-in checks, and Miri catches many more (Lab 1). Your written SAFETY argument is the only check that covers every input.
- **A double drop kills the test program.** If a run ends with `signal: 6, SIGABRT`, `SIGSEGV`, `SIGTRAP`, or a message such as `pointer being freed was not allocated`, and prints no `test result:` line, then a value was dropped twice or invalid memory was read. The whole test program stops, so the results of the other lessons are missing too. Find the lesson by running one at a time on one thread:

  ```sh
  cargo test --test chapter23 l06_ -- --test-threads=1
  ```

  The last test name that was printed is the one that crashed. The usual cause is a raw move (`ptr::read`, `assume_init_read`) without the matching change of the length.

## Topic map

"Optional lab" means a manual experiment from a lesson header or from [LABS.md](LABS.md). No test checks it.

| Topic | Where you practise it |
| --- | --- |
| What `unsafe` permits and what it leaves unchanged; undefined behavior | Lesson 01; drills d01, d03 and d05; CONCEPTS, lesson 01 |
| A safe type whose private invariant justifies an unchecked operation | Lesson 01 (an ASCII string wrapper); R01 task C |
| `unsafe fn`: stating caller obligations and discharging them in a safe wrapper | Lesson 02; R01 tasks A and B |
| Raw pointers: creating them, null and dangling pointers, when a read is valid | Lesson 02; drills d01, d02 and d06 |
| Pointer arithmetic with `add` and `wrapping_add`; rebuilding slices with `from_raw_parts_mut` | Lesson 03; lesson 02 predictions; drill d02 |
| Two `&mut` to one buffer: proving that they do not overlap | Lesson 03 |
| Alignment and spare address bits; address and provenance; `map_addr`; `PhantomData` for a lifetime | Lesson 04; optional Lab 5 |
| Size, alignment, padding, `repr(C)`, `offset_of!`; valid bit patterns | Lesson 05; drills d07 and d09 |
| Unsafe traits and `unsafe impl` | Lesson 05; lesson 09 (supplied impls to read); drills d04 and d05 |
| Unions | Lesson 05 task C |
| Raw moves with `ptr::read`, `set_len` and `MaybeUninit`; avoiding a double drop | Lessons 06 and 08; drill d10; C01 |
| Overlapping copies with `ptr::copy`; a gap buffer for bytes | Lesson 07 |
| A gap buffer for values that are not `Copy` | Optional Lab 3 |
| Panic safety: no panic between a raw move and the length update | Lessons 06 and 08 (written argument); C01; optional Lab 4 (tested with `catch_unwind`) |
| `UnsafeCell`: mutation through `&self`, and why such a type is not `Sync` | Lesson 09 task A |
| `repr(transparent)` and pointer casts between a wrapper and its field | Lesson 09 task B |
| `repr(packed)`, `&raw const`, and `read_unaligned` | Lesson 09 task C |
| `static mut` and why edition 2024 rejects references to it | Drill d08 and its hint |
| A container over uninitialized memory, with its invariant written down | Lesson 08 (capstone) |
| A hand-built `RawWaker` with manual reference counting | R01 task B |
| Finding unnecessary and unsound `unsafe` in existing code | C01 |
| Running tests under Miri and reading its reports | Lab 1 |
| Writing the full safety argument for every block | Optional Lab 2 |

Not in this chapter: building `Box`, `Vec`, `Rc` and `RefCell` from raw allocation, the aliasing model in detail, variance and drop check. These belong to the mastery track (the modules on unsafe Rust and on the type system).

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

1. Which guarantees remain your responsibility inside an `unsafe` block, and which checks does the compiler still perform there?
2. How does the contract of an unsafe function differ from the invariant of a safe abstraction?
3. What makes a read through a raw pointer valid? Name provenance, bounds, alignment, initialization, lifetime and aliasing.
4. When must a raw move change the record of which slots are initialized, so that no value is dropped twice?
5. How do size, alignment, padding and valid bit patterns limit the reinterpretation of memory?
6. How does a gap buffer keep its two initialized regions disjoint?
7. What must stay true if an operation supplied by the user panics?
8. What do tests and Miri show, and which general safety claims still need a written proof?
9. Why does interior mutability need `UnsafeCell`, why is a cell without a lock not `Sync`, and what do `repr(transparent)` and `repr(packed)` change?

Redo the capstone from its contract in a later session. Explain one design tradeoff and create a case that the supplied tests do not exercise.

## Solutions

Model solutions are in [solutions/exercises/chapter23/](../../solutions/exercises/chapter23/) (one file per lesson, with the predictions filled in and a SAFETY comment on every unsafe block) and [solutions/drills/ch23/](../../solutions/drills/ch23/) (each drill repaired, plus the expected outcome of every drill). Use them after an honest attempt and after the hints: compare your solution with the model, and note in NOTES.md what differs and why. In this chapter, compare the SAFETY comments as carefully as the code. Reading a solution first removes the struggle that makes the lesson stay in memory.
