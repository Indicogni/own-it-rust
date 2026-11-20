# Mastery M1 — The type system in depth

[All chapters](../../README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

You can write Rust that compiles. This module is about the parts of the type system that decide *why* it compiles, and about using those parts on purpose: to reject wrong programs before they run, at no run-time cost. You study variance, lifetime bounds, higher-ranked bounds, generic associated types, `PhantomData`, trait objects, zero-sized and uninhabited types, sealed traits, typestate, const generics and coherence. These are the topics behind the compiler errors that experienced Rust programmers still find hard to read, and behind most of the design choices in a well-made library.

**Who it is for.** A programmer who has finished the chapters of this course, or who writes Rust at work and wants to stop guessing at lifetime and trait errors. You do not need to have written `unsafe` code or a public library before.

**Before you start.** These chapters, with the lessons that depend on them:

| You need | For |
| --- | --- |
| [Chapter 5](../../exercises/chapter05/README.md) (references), [Chapter 9](../../exercises/chapter09/README.md) (structs with lifetimes) | every lesson |
| [Chapter 8](../../exercises/chapter08/README.md) (crates, modules, privacy) | lessons 06, 07, 08 |
| [Chapter 10](../../exercises/chapter10/README.md) (enums and patterns) | lesson 06 |
| Chapters [11](../../exercises/chapter11/README.md) and [13](../../exercises/chapter13/README.md) (traits, generics, trait objects, utility traits) | lessons 02 to 09 |
| [Chapter 14](../../exercises/chapter14/README.md) (closures) | lesson 02 |
| [Chapter 15](../../exercises/chapter15/README.md) (iterators) | lesson 03 |
| [Chapter 19](../../exercises/chapter19/README.md) (`Send` and `Sync`), [Chapter 23](../../exercises/chapter23/README.md) (raw pointers) | lesson 04 |

**Time.** About 20 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. The lessons are dense. One lesson per session is a good pace.

## Setup

This is a standalone Cargo package with its own `Cargo.lock`. It uses only the standard library, so it needs no network. Rust 1.99.0 is pinned in `rust-toolchain.toml`, identical to the one in the repository root. Run the commands from the repository root:

```sh
cargo test --manifest-path mastery/m1_types/Cargo.toml --no-run
cargo test --manifest-path mastery/m1_types/Cargo.toml --release --no-run
```

Or change into `mastery/m1_types/` and leave out `--manifest-path`. The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in.

## How this module checks your work

Many rules in this module are about programs that must **not** compile. An ordinary test cannot check that. So there are two kinds of checks:

- **Unit tests** in each lesson file, as in the chapters. Some of them use `implements!(Type: Trait)`, a small macro in `src/support.rs` that turns "does this type implement this trait?" into a `bool`, so that a test can fail with a sentence and not with a wall of compiler output.
- **Cases** in `tests/cases/`. Each case is a small program that uses your library the way another crate would. Its first line says what must happen: `//@ expect: run` (it compiles, runs and exits normally), an error code such as `//@ expect: E0599` (the compiler rejects it with exactly this code), `//@ expect: error` (rejected with an error that has no code), or a list such as `//@ expect: E0308 | E0277` (rejected, and every code that the compiler reports is in the list; used where two sound designs give different messages). `tests/contracts.rs` builds your library, compiles every case with `rustc`, and compares. When a case fails, the test prints what the case means and the compiler's output.

Read the cases of a lesson before you write code. They are the exact contract. Do not edit them.

## Work through the module

Every lesson uses the same loop.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet.
3. **Implement.** In lessons 01 to 05 you fill in `todo!()` bodies, and in some tasks you repair a signature or a type that is given as a first attempt. From lesson 06 on, more of the design is yours: lessons 07 to 09 give you a contract in prose and an empty file. Run the lesson, for example `cargo test --manifest-path mastery/m1_types/Cargo.toml --no-fail-fast l03_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** The cases that must not compile are the technique tests of this module: a design that checks at run time what the contract wants checked by the compiler fails them. Other tests compare pointers, count allocations and check auto traits. Then run clippy and fix the warnings in code that you wrote.
5. **Explain** in [NOTES.md](NOTES.md), add one edge case of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

Do the **compiler drills** as you go: each drill names its lesson in [HINTS.md](HINTS.md), and `cargo drill m1` walks through all twenty (run it from the repository root; d19 and d20 belong to lessons 04 and 05). For each one, predict whether it compiles (and with which error code), runs, or panics; then fix it. Do the **code review** lesson (C01) after lesson 05 and the **capstone** last.

| Done | Lesson | What you build |
| --- | --- | --- |
| [ ] | [01 · Subtyping and variance](src/l01_variance.rs) | Conversions that the compiler allows, refuses, or reverses; a signature that stops borrowing forever |
| [ ] | [02 · Lifetime bounds and higher-ranked trait bounds](src/l02_bounds_hrtb.rs) | A `for<'a>` bound on a reference type, a lexer whose words outlive its borrow, a closure that returns a borrow |
| [ ] | [03 · Generic associated types and a lending iterator](src/l03_lending.rs) | `WindowsMut`, a `Take` adapter, a line reader that reuses one buffer |
| [ ] | [04 · PhantomData](src/l04_phantom.rs) | A typed id, a raw-pointer slice iterator with `unsafe impl Send`, a thread-bound handle |
| [ ] | [05 · Trait objects](src/l05_trait_objects.rs) | Upcasts to `dyn Any`, a registry with the right object lifetime, a forwarding impl, a trait made dyn compatible |
| [ ] | [C01 · Code review: a generic API that asks for too much](src/c01_code_review.rs) | Repairs to needless `'static` bounds, a leaked iterator type and two bugs |
| [ ] | [06 · Zero-sized and uninhabited types](src/l06_zst_never.rs) | An infallible parser, a proof token |
| [ ] | [07 · Sealed traits, typestate and const generics](src/l07_sealed_typestate.rs) | An `Envelope` that can only be sent when complete; a `Matrix` with shapes in its type |
| [ ] | [08 · Coherence and the orphan rule](src/l08_coherence.rs) | A `Describe` trait that leaves room for its users; conversions for a local type |
| [ ] | [Compiler drills](../../drills/m1/) — `cargo drill m1` | Twenty predictions |
| [ ] | [09 · Capstone: units of measure](src/l09_capstone.rs) | `Quantity<U>`: misuse of a unit is a compile error |

```sh
# From the repository root. Inside mastery/m1_types/ leave out --manifest-path.
cargo test --manifest-path mastery/m1_types/Cargo.toml -- --list
cargo test --manifest-path mastery/m1_types/Cargo.toml --no-fail-fast l01_
cargo test --manifest-path mastery/m1_types/Cargo.toml --test contracts l07_
cargo test --manifest-path mastery/m1_types/Cargo.toml --no-fail-fast
cargo test --manifest-path mastery/m1_types/Cargo.toml --release --no-fail-fast
cargo test --manifest-path mastery/m1_types/Cargo.toml --doc
cargo clippy --manifest-path mastery/m1_types/Cargo.toml --all-targets
cargo drill m1
```

Notes on these commands:

- A filter such as `l01_` matches the unit tests of the lesson (`l01_variance::tests::...`) and its cases (`l01_record_twice`).
- `cargo test` stops after the first test binary with a failure. The unit tests run first, so without `--no-fail-fast` you may not see the cases. `--test contracts` runs only the cases.
- `cargo test --doc` compiles and runs every code block of [CONCEPTS.md](CONCEPTS.md).
- `cargo drill m1` is a command of the course's root package. It works from the repository root, not from inside `mastery/m1_types/`.
- The cases call `rustc` from your `PATH` (or from the `RUSTC` environment variable). With `rustup`, that is the pinned toolchain.
- Lesson 04 contains `unsafe` code. If you have the course's dated nightly toolchain with Miri (see [The nightly toolchain](../../README.md#the-nightly-toolchain)), check your solution with `cargo +nightly-2026-11-20 miri test --manifest-path mastery/m1_types/Cargo.toml --lib l04_`. Miri runs the unit tests in an interpreter that reports invalid pointer use. The model solution passes it.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

## Topic map

| Topic | Where you practise it |
| --- | --- |
| Lifetime subtyping; covariance, contravariance, invariance | Lesson 01; drills d01 to d04 |
| Why `&mut T`, `Cell<T>` and `*mut T` are invariant | Lesson 01 tasks B and D; drills d01 and d02 |
| `&'a mut Thing<'a>`: a borrow that never ends | Lesson 01 task D; drill d02 |
| `'a: 'b`, `T: 'a`, `T: 'static` | Lesson 02 task D and prediction P1 |
| `for<'a>` bounds on closures and on reference types | Lesson 02 tasks A, B and E; drill d06 |
| Closures that return a borrow, and why a `let` breaks them | Lesson 02 task E and the compiler lab; drill d05 |
| Tying a result to the data, not to `&mut self` | Lesson 02 task C |
| Generic associated types, `where Self: 'a` | Lesson 03; drill d08 |
| Lending iterators: overlapping windows, reused buffers, adapters | Lesson 03 tasks A, C and D; drill d07 |
| The `'static` limitation of higher-ranked bounds over GATs | CONCEPTS, lesson 03, and the lesson's compiler lab |
| `PhantomData` markers and what each one says | Lesson 04; drill d10 |
| Derives that add unwanted bounds | Lesson 04 task A; capstone |
| `unsafe impl Send`/`Sync` with the right bound, and SAFETY comments | Lesson 04 task B |
| Drop check, and the one case where a marker changes it | CONCEPTS, lesson 04; drills d09 and d19 |
| Fat pointers and vtables, observed with `size_of` and `size_of_val` | Lesson 05 predictions; CONCEPTS, lesson 05 (a vtable by hand) |
| Dyn compatibility and `where Self: Sized` | Lesson 05 task E; drill d11 |
| Upcasting to a supertrait and to `dyn Any`; the `type_id` trap | Lesson 05 tasks A and B, prediction P3; drill d13 |
| Object lifetime defaults (`Box<dyn Trait>` in a field or signature is `'static`; in a `let` it is inferred) | Lesson 05 task C; drills d12 and d20; C01 |
| Forwarding impls with `?Sized` | Lesson 05 task D; lesson 08 |
| Needless `'static`, `Send` and `Sync` bounds; leaked implementation types | C01 |
| Zero-sized types, alignment, `Vec<()>` | Lesson 06 predictions |
| Uninhabited types, `Infallible`, irrefutable patterns | Lesson 06 tasks A and B; drill d14 |
| Proof tokens with private constructors | Lesson 06 task C |
| Typestate: states as types, transitions that consume `self` | Lesson 07 contract A; drill d15 |
| Sealed traits | Lesson 07 contract A; capstone |
| Const generics as typestate for sizes | Lesson 07 contract B |
| The orphan rule, uncovered parameters, fundamental types | Lesson 08 contract B; drills d16 and d18 |
| Overlap, blanket impls, and what they forbid | Lesson 08 contract A; drill d17 |
| Method resolution: which impl a call finds | Lesson 08 predictions |
| Associated types as compile-time facts; operator impls for foreign types | Lesson 09 (capstone) |

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Give the variance of `&'a T`, `&'a mut T`, `Cell<T>`, `Vec<T>` and `fn(T) -> U` in each parameter. For `&'a mut T`, describe the unsound program that covariance in `T` would allow.
- [ ] Explain what `T: 'static` does and does not promise, and write the desugared form of `F: Fn(&str) -> &str`. Say who chooses the lifetime in each form.
- [ ] Explain why an iterator that lends overlapping `&mut` windows cannot implement `Iterator`, and how a generic associated type solves it. Name one thing you lose.
- [ ] For a type that holds a raw pointer, list the three things the compiler cannot work out for itself, and the `PhantomData` marker for "owns a `T`", "borrows a `T` for `'a`" and "is only labelled with `T`".
- [ ] Draw a `Box<dyn Trait>`: the two words and what the vtable holds. Give three reasons why a trait may not be dyn compatible, and the repair for each.
- [ ] State the default lifetime of the trait object in `Box<dyn Trait>` and in `&'a dyn Trait` when the type is written in a field or a signature, what changes in a `let` inside a function body, and the error you get when the default is wrong.
- [ ] Explain how a zero-sized token with a private field proves that code ran, and what `Result<T, Infallible>` tells a caller.
- [ ] Describe the three ingredients of the typestate pattern and how a trait is sealed. Say when typestate is the wrong tool.
- [ ] State the orphan rule, including uncovered type parameters. Explain what a blanket impl over `T: Display` takes away from the users of a library.
- [ ] Redo the capstone after a break without looking at your first solution. Then add a third dimension (mass) and a unit for it, and explain which impls you had to touch and why the others did not change.

## Solutions

Model solutions are in [solutions/mastery/m1_types/](../../solutions/mastery/m1_types/) (one file per lesson under `src/`, with the predictions filled in) and [solutions/drills/m1/](../../solutions/drills/m1/) (each drill repaired, plus the expected outcome of every drill in `expected.tsv`). Every `unsafe` block in the lesson 04 solution has a SAFETY comment; read those even if your own solution passed. Use the solutions after an honest attempt and after the hints: compare your design with the model, and note in NOTES.md what differs and why. For lessons 07 to 09 your design may differ from the model and still be right; the cases decide.
