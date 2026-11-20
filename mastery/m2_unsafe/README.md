# Mastery M2 — Unsafe Rust by rebuilding the standard library

[All chapters](../../README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Reading a SAFETY comment is not the same as writing one that is true. In this module you build the types that every Rust program rests on, `Cell`, `RefCell`, `Box`, `Vec`, its iterators, `Rc` and `Weak`, from raw allocation, and then a linked queue, a spin lock and a bump arena. For each one you write down the invariant that makes it sound, justify every `unsafe` block against that invariant, and let Miri check what your tests cannot see. The module ends with a code review of a ring buffer that "works" and is unsound.

**Who it is for.** You have finished the chapters, in particular [chapter 23](../../exercises/chapter23/README.md), and you can use raw pointers in small functions. Now you want to own an unsafe abstraction: to design it, to argue that no safe caller can break it, and to review someone else's.

**Before you start:**

| Chapter | What this module uses from it |
| --- | --- |
| [4](../../exercises/chapter04/README.md) and [5](../../exercises/chapter05/README.md) | Ownership, moves, references and lifetimes |
| [9](../../exercises/chapter09/README.md) | `Cell` and `RefCell` as a user |
| [13](../../exercises/chapter13/README.md) | `Drop`, `Deref`, `Clone`, `Default` |
| [15](../../exercises/chapter15/README.md) | `Iterator`, `DoubleEndedIterator`, `ExactSizeIterator` |
| [19](../../exercises/chapter19/README.md) | Threads, `Send` and `Sync` as a user, atomics |
| [23](../../exercises/chapter23/README.md) | Raw pointers, `unsafe fn`, SAFETY comments, `ptr::read`/`write`, `MaybeUninit`, a first Miri run |

Variance and the drop checker are explained where they are needed; module [M1](../m1_types/README.md) treats them in full. Memory orderings are given where they are needed; module [M3](../m3_concurrency/README.md) treats them in full.

**Time:** about 40 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. This is slow material. One lesson a week is a good pace.

## Setup

This is a standalone Cargo package with no dependencies. It is not part of the root package, so the chapter checks never build it. Rust 1.99.0 is pinned in `rust-toolchain.toml`, identical to the root one. Run every command from the root of the course repository:

```sh
cargo test --manifest-path mastery/m2_unsafe/Cargo.toml --all-targets --no-run
cargo test --manifest-path mastery/m2_unsafe/Cargo.toml --release --all-targets --no-run
```

The starters compile; tests fail at `todo!()` and `guess()` until you fill them in. That is the starting state, not a broken setup.

### Miri

Miri is an interpreter that runs your tests and checks every memory access. It is the second half of every lesson in this module. It needs a nightly toolchain; the package itself stays on the pinned Rust 1.99.0. The course uses one dated nightly, `nightly-2026-11-20`, everywhere, the same one its CI uses (see [The nightly toolchain](../../README.md#the-nightly-toolchain)).

```sh
rustup toolchain install nightly-2026-11-20 --component miri --component rust-src
cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml l01_
```

The first run builds a copy of the standard library for Miri and takes a few minutes. The reports quoted in this module were produced with that toolchain, which reports `rustc 1.102.0-nightly (6905905bb 2026-11-19)`. It is pinned because Miri's wording, and now and then a verdict, changes between nightlies; with another nightly your output may differ in more than numbers.

| Command prefix | What it changes |
| --- | --- |
| *(none)* | Stacked Borrows, the default aliasing model, and in most cases the stricter one |
| `MIRIFLAGS=-Zmiri-tree-borrows` | Tree Borrows, the second aliasing model (lesson 06) |
| `MIRIFLAGS=-Zmiri-many-seeds=0..16` | Sixteen different thread schedules (lesson 07) |
| `MIRIFLAGS=-Zmiri-strict-provenance` | Rejects integer-to-pointer casts |
| `MIRIFLAGS=-Zmiri-ignore-leaks` | Does not report leaked memory (lab 04 only) |

Three things to know before the first surprise:

- **A wrong unsafe program can crash the whole test run** instead of failing one test: the process dies and Cargo prints `process didn't exit successfully`. When that happens, run that one test under Miri. It will name the line.
- **Miri stops at the first undefined behaviour.** Fix it and run again.
- **A clean Miri run is evidence, not proof.** Miri checks the paths that your tests run. It cannot tell you that no other safe program breaks your type. The invariant and the SAFETY comments do that.

Under Miri the tests' audit allocator is switched off, so that Miri's own, stricter allocator checks apply. Natively, the tests of this module replace the system allocator with one that counts allocations and compares the layout of every `dealloc` with the layout of the matching `alloc`, so some layout mistakes fail a native test with a message.

## Work through the module

Every lesson uses the same loop as the chapters, with one step added:

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Answer the `p_predictions` test in the lesson file: replace each `guess()` with a value. Do not run anything yet.
3. **Write the invariant** of the type in its doc comment, before any code.
4. **Implement**, with a `// SAFETY:` comment on every `unsafe` block that points back to the invariant. Run the lesson natively, in both profiles, then under Miri.
5. **Do the Miri lab** named in the lesson header: predict what Miri says about a wrong version of this lesson's type, run it, read the report, repair it.
6. **Check technique.** `cargo clippy` warns about every `unsafe` block without a SAFETY comment. Fix the warnings in code that you wrote.
7. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

The lessons give you less as you go. Lessons 01 and 02 supply the fields and the signatures. Lessons 03, 04 and 06 supply the signatures; the fields and the invariant are yours. Lessons 05, 07 and 08 supply a contract in prose and nothing else: you design the types. Their tests name types that do not exist yet, so they are switched off in the starter. One failing test, `tests_are_switched_off`, tells you so. When your types exist (start with `todo!()` bodies), change `off` to `on` in the `gate!` line above the tests.

| Done | Lesson | Needs | You build |
| --- | --- | --- | --- |
| [ ] | [01 · UnsafeCell](src/l01_unsafe_cell.rs) | Chapter 23 | `MyCell`, `MyRefCell` and its guards |
| [ ] | [02 · Layout, alloc, NonNull](src/l02_box.rs) | 01 | `MyBox` |
| [ ] | [03 · A growable buffer](src/l03_vec.rs) | 02 | `MyVec`: growth, zero-sized types, `insert`, `remove`, `Deref` |
| [ ] | [04 · Panic safety and leak safety](src/l04_iterators.rs) | your `MyVec` | `IntoIter`, `Drain` |
| [ ] | [Compiler drills](../../drills/m2/) — `cargo drill m2` | 01 to 04 | ten predictions |
| [ ] | [05 · Shared ownership](src/l05_rc.rs) | 01, 02 | `MyRc`, `MyWeak` |
| [ ] | [06 · The aliasing rules](src/l06_aliasing.rs) | 01 to 05 | `pair_mut`, a linked `Queue`, eight Miri verdicts |
| [ ] | [07 · Send and Sync](src/l07_send_sync.rs) | your types from 01 to 04 | `Spin`, `SpinGuard`, and the impls for your own types |
| [ ] | [08 · Capstone: a bump arena](src/l08_capstone.rs) | all | `Arena` |
| [ ] | [C01 · Code review: a teammate's ring buffer](src/c01_code_review.rs) | all | a review, your exploits, a repair |

```sh
cargo test --manifest-path mastery/m2_unsafe/Cargo.toml l01_
cargo test --manifest-path mastery/m2_unsafe/Cargo.toml --release l01_
cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml l01_
cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example miri_l01_cell
cargo clippy --manifest-path mastery/m2_unsafe/Cargo.toml --all-targets
cargo test --manifest-path mastery/m2_unsafe/Cargo.toml --doc
cargo drill m2
```

Lesson filters (`l01_` to `l08_`, `c01_`) match the module path of the tests. `cargo test --doc` compiles and runs every example in CONCEPTS.md, and the three compiler checks in the header of lesson 08. One of those three fails until your `Arena` exists; that is the starting state too.

When the whole module is done, this is the full check:

```sh
cargo test --manifest-path mastery/m2_unsafe/Cargo.toml
cargo test --manifest-path mastery/m2_unsafe/Cargo.toml --release
cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml
MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly-2026-11-20 miri test --manifest-path mastery/m2_unsafe/Cargo.toml
```

### The Miri labs

Each program in `examples/` is a small, wrong version of the type of its lesson, with SAFETY comments that sound right and are not. Predict, run under Miri, compare with the report in CONCEPTS.md, repair. A native run of these programs proves nothing: some print the expected result, some print garbage, some crash.

| Lab | The mistake |
| --- | --- |
| [miri_l01_cell](examples/miri_l01_cell.rs) | Writing through a pointer made from `&T` |
| [miri_l02_box](examples/miri_l02_box.rs) | Moving a value out and letting the destructor run as well |
| [miri_l03_vec](examples/miri_l03_vec.rs) | Assigning into memory that holds no value |
| [miri_l04_drain](examples/miri_l04_drain.rs) | Depending on a destructor that `mem::forget` can skip |
| [miri_l05_rc](examples/miri_l05_rc.rs) | Freeing an allocation that weak pointers still read |
| [miri_l06_queue](examples/miri_l06_queue.rs) | A `Box` and a raw pointer to the same node (not repaired: task B is the repair) |
| [aliasing](examples/aliasing.rs) | Ten specimens for lesson 06 (not repaired: you record Miri's verdicts) |
| [miri_l07_sync](examples/miri_l07_sync.rs) | An `unsafe impl Sync` with a comment instead of a proof |
| [miri_l08_arena](examples/miri_l08_arena.rs) | A bump allocator that forgets alignment |

## Topic map

| Topic | Where you practise it |
| --- | --- |
| What `&T` promises, and what `UnsafeCell` changes | Lesson 01; lab 01; drills d01, d02, d10 |
| Interior mutability without checks (`Cell`) and with run-time checks (`RefCell`) | Lesson 01 |
| Why foreign code (`Drop`, `Clone`) must not run while a pointer into a cell is in use | Lesson 01 task A (`set`) |
| `Layout`, `alloc`, `dealloc`, `handle_alloc_error` | Lessons 02 and 03; C01 |
| `NonNull`, niches, covariance, `PhantomData` | Lessons 02, 03, 05, 06 |
| Zero-sized types: dangling pointers, no allocation, unlimited capacity | Lessons 02, 03, 04, 08; C01; drill d08 |
| `ptr::read`, `ptr::write`, assignment, `ptr::copy`, `drop_in_place` | Lesson 03; lab 03 |
| Growth with `realloc`, amortized cost, capacity overflow | Lesson 03 |
| `unsafe fn` contracts between modules (`set_len`, `from_raw_parts`) | Lesson 03 task D, used by lesson 04 |
| Stopping a destructor: `ManuallyDrop`, `mem::forget` | Lessons 02, 03, 05; lab 02; drill d04 |
| Panic safety and drop guards | Lessons 03, 04, 05, 08 |
| Leak safety and leak amplification | Lesson 04; lab 04; drill d06 |
| Reference counting, the shared weak reference, `get_mut`, overflow of counts | Lesson 05; lab 05; drill d09 |
| Borrowing one field through a raw pointer | Lesson 05 |
| Stacked Borrows and Tree Borrows; reading a Miri report | Lesson 06; lab 06; task M |
| Linked structures with raw pointers only | Lesson 06 task B |
| `Send` and `Sync` proofs; narrowing a derived auto trait | Lesson 07; lab 07; C01; drill d05 |
| A lock with a guard on `UnsafeCell` and `AtomicBool` | Lesson 07 |
| Alignment arithmetic, chunks, provenance-preserving bumps | Lesson 08; lab 08 |
| Type-erased destructors; why `'static` can be a safety argument | Lesson 08; drill d07 |
| Safety that lives in a signature (`T: 'static`, `&mut self`), checked by programs that must not compile | Lesson 08, compiler checks |
| Finding soundness holes in code that passes its tests; writing an exploit | C01 |
| `unsafe_op_in_unsafe_fn` in edition 2024 | The package's lint settings; drill d03 |

Not covered here, on purpose: `Arc`, atomics in depth and lock-free structures (module M3); pinning (module M4); `repr` attributes, `transmute` and unions (chapter 23); foreign functions (chapter 24 and module M6).

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Say what a `&T` promises, what `UnsafeCell` changes about that promise and what it does not change, and why `Cell::get` needs `T: Copy`.
- [ ] Give the three rules of the allocator that cause most bugs, and say how a box and a vector handle zero-sized types.
- [ ] For a vector slot, say when to use `write`, `read`, an assignment, `copy` and `copy_nonoverlapping`, and what goes wrong with the wrong choice.
- [ ] Explain panic safety, leak safety, a drop guard and leak amplification, with `Drain` as the example.
- [ ] Explain the two counters of `Rc`: what each counts, when the value is dropped, when the allocation is freed, why `get_mut` looks at both, and why the counts must not wrap.
- [ ] Name the three parts of a Miri borrow report, describe the two aliasing models in two sentences each, and give the rules of thumb that both accept.
- [ ] Derive the `Send` and `Sync` bounds for a lock and for its guard, and name a type that breaks each bound if it is missing.
- [ ] For the arena, give the argument for `&self -> &mut T`, the reason `alloc` runs no destructors, and the reason `alloc_with_drop` needs `'static`.
- [ ] Say what a clean Miri run shows and what it does not show.
- [ ] Redo the capstone after a break, from an empty file and without your notes. Then change one requirement (for example: the arena must be `Send`) and say what has to change.

## Solutions

Model solutions are in [solutions/mastery/m2_unsafe/](../../solutions/mastery/m2_unsafe/): one file per lesson under `src/`, with the predictions filled in, and the repaired Miri labs under `examples/`. The repaired drills and their expected outcomes are in [solutions/drills/m2/](../../solutions/drills/m2/). Every solution passes its tests natively in both profiles and under Miri with both aliasing models.

Use them after an honest attempt and after the hints. For this module there is a better use than checking your result: when your version passes, read the model's SAFETY comments next to yours. Where the model names a fact that your comment does not, ask whether your code relies on that fact. Write what you find in NOTES.md.
