# Mastery M5 — Procedural macros

[All chapters](../../README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

A procedural macro is a Rust function that the compiler runs while it compiles someone else's code. `#[derive(Debug)]`, `#[test]` and `println!` have this power built in; with a procedural macro you write such things yourself. In this module you write all three kinds: derives, an attribute macro and function-like macros. You begin with nothing but the compiler's own `proc_macro` crate, so that you see what a macro really receives. Then you move to `syn` and `quote`, the crates that nearly every published macro is built on. The second half of the module is about quality, because a macro is a small compiler extension that other people have to live with: errors that point at the user's mistake, tests for code that must *not* compile, and generated code that keeps its meaning in every module it lands in. The capstone is a `#[derive(Builder)]`.

**Who this is for.** You have finished the chapters and want to write macros that you would publish, or you maintain a codebase that uses derives and want to understand what they do to your code.

**Before you start.**

| You need | From |
| --- | --- |
| Crates, Cargo, workspaces, features | [Chapter 8](../../exercises/chapter08/README.md) |
| Structs, enums, patterns | Chapters [9](../../exercises/chapter09/README.md) and [10](../../exercises/chapter10/README.md) |
| Traits, generic parameters, bounds, where clauses | Chapters [11](../../exercises/chapter11/README.md) and [13](../../exercises/chapter13/README.md) |
| `Result`, `?`, and your own error types | [Chapter 7](../../exercises/chapter07/README.md) |
| `Drop` and the order in which local variables are dropped | [Chapter 13](../../exercises/chapter13/README.md) |
| `macro_rules!`, including hygiene | [Chapter 22](../../exercises/chapter22/README.md) |

[Chapter 22](../../exercises/chapter22/README.md)'s optional labs 2 and 3 are a first contact with procedural macros. They are not required: this module starts from the beginning.

**Time:** about 17 hours in total, as a rough estimate. [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson. Sessions of 60 to 90 minutes work well; the capstone is better done in two or three sittings.

## Setup

This is a standalone Cargo workspace with its own `Cargo.lock`. It is **not** part of the root package, so the chapter checks never download or build these crates. Rust 1.99.0 is pinned in `rust-toolchain.toml`, identical to the root one. The dependencies are pinned to exact versions: syn 3.0.10, quote 1.0.47, proc-macro2 1.0.107 and trybuild 1.0.125 (a test tool).

```sh
cd mastery/m5_proc_macros
# The first build downloads the pinned crates; afterwards you can add --offline.
cargo test --locked --no-run
cargo test --locked --release --no-run
```

The starters compile. Their tests fail at `todo!()` and `guess()` until you fill them in. That is the starting state, not a broken setup.

### The three crates

A crate that exports procedural macros can export nothing else, so a trait and its derive are always in two crates. This workspace has the shape of a real macro library:

| Directory | Crate | What it is |
| --- | --- | --- |
| `.` | `m5_proc_macros` | The crate a user depends on: the traits, the run-time helpers that generated code calls, and re-exports of all macros. You do not edit it. Its `tests/` directory holds the end-to-end tests. |
| `bare/` | `m5_bare` | The macro crate for lesson 01. It has no dependencies. |
| `macros/` | `m5_macros` | The macro crate for lessons 02 to 08 and the code review. One file per lesson, with its unit tests. |

### Two kinds of test

- **Unit tests**, next to your code in `macros/src/lNN_*.rs`. They call your function with tokens and look at the tokens that come back: the shape of the generated code, the error messages and where they point. They run in milliseconds.
- **End-to-end tests**, in `tests/`. They compile the small programs in `tests/ui/lNN/` against your macros, the way a user would: a `pass_*.rs` program must compile and run, and a `fail_*.rs` program must be rejected with exactly the compiler output in the `.stderr` file next to it. Read those programs. They are the precise contract.

When an end-to-end test fails, read the compiler output printed above the test summary. `proc-macro derive panicked ... not yet implemented` means that the macro still contains a `todo!()`.

## Work through the module

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example that is different from the exercise, and shows the compiler errors you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet. (Lesson 01's predictions are in `tests/l01_raw_tokens.rs`.)
3. **Implement** the `todo!()` bodies from their contracts and run the lesson, for example `cargo test l02_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Many tests check *how*: that an error points at the right token, that a bound is on the right type, that a name in the generated code cannot collide with the user's names. The failure message says what was expected and why. Then run `cargo clippy --all-targets` and fix the warnings in code that you wrote.
5. **Explain** in [NOTES.md](NOTES.md), add one edge case of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md).

The lessons give you less and less. Lessons 01 to 04 give every signature. Lesson 05 gives the types and leaves the helpers to you. Lessons 06 and 07 give a contract in prose and one entry point per macro; the types, the parser and the checks are your design. The capstone gives a contract and nothing else.

After lesson 03, **drill the compiler**: from the repository root, `cargo drill m5` shows nine small programs about what the standard derives generate. For each one, predict whether it compiles (and with which error code), runs, or panics; then fix it. Do the **code review** lesson (C01: repair a derive that works on one struct and fails on the rest) after lesson 04, or after lesson 07 if you want it to be a real test.

| Done | Lesson | Macro you write | Start after |
| --- | --- | --- | --- |
| [ ] | [01 · A derive with nothing but `proc_macro`](bare/src/l01_raw_tokens.rs) | `#[derive(TypeName)]`, `#[derive(FieldNames)]` | Chapter 22 |
| [ ] | [02 · `syn` and `quote` for structs with named fields](macros/src/l02_syn_quote.rs) | `#[derive(Changes)]`, `#[derive(Setters)]` | Lesson 01 |
| [ ] | [03 · Generics, where clauses and helper attributes](macros/src/l03_generics_attrs.rs) | `#[derive(Redacted)]` | Lesson 02 |
| [ ] | [Compiler drills](../../drills/m5/) — `cargo drill m5` | | Lesson 03 |
| [ ] | [04 · Errors that point at the right place, and a compile-fail suite](macros/src/l04_errors.rs) | `#[derive(Label)]` | Lesson 03 |
| [ ] | [C01 · Code review: a teammate's derive](macros/src/c01_code_review.rs) | `#[derive(Reset)]` | Lesson 04 |
| [ ] | [05 · An attribute macro that keeps the item intact](macros/src/l05_attribute.rs) | `#[logged]` | Lesson 04 |
| [ ] | [06 · A function-like macro with its own syntax](macros/src/l06_function_like.rs) | `flags!` | Lesson 04 |
| [ ] | [07 · Hygiene, crate paths, and how to test a macro](macros/src/l07_hygiene.rs) | `attempt!`, `#[derive(Named)]` | Lessons 05 and 06 |
| [ ] | [08 · Capstone: `#[derive(Builder)]`](macros/src/l08_capstone.rs) | `#[derive(Builder)]` | Lesson 07 |

Run these in `mastery/m5_proc_macros`:

```sh
cargo test l02_                       # one lesson: unit tests and end-to-end tests
cargo test -p m5_macros l02_          # only the fast unit tests of the lesson
cargo test --test ui l02_             # only the end-to-end tests of the lesson
cargo test -- --list
cargo test
cargo test --release
cargo clippy --all-targets
cargo test --doc                      # the std-only code block of CONCEPTS.md
TRYBUILD=overwrite cargo test l04_d   # record the compiler output of your own case (lesson 04)
```

And this one in the repository root:

```sh
cargo drill m5
```

A filtered run such as `cargo test l02_` starts every test binary of the workspace, and most of them then report 0 tests. That is expected. The end-to-end tests start `cargo` themselves, so the first run of each session takes a few seconds longer.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

## Topic map

| Topic | Where you practise it |
| --- | --- |
| What a procedural macro receives and returns; the three kinds | Lesson 01; CONCEPTS lesson 01 |
| Token trees: identifiers, punctuation, literals, groups; why `<` is not a bracket | Lesson 01 (tasks A and D, predictions) |
| Reporting an error with `compile_error!`, built by hand with a span | Lesson 01 task B |
| The split into a macro crate and a crate for traits | This package's layout; lesson 07 |
| `DeriveInput`, `Data`, `Fields`: the syntax tree of a derive's input | Lessons 02 to 04; C01 |
| `quote!`: interpolation, repetition, what each type turns into | Lesson 02 (tasks and predictions) |
| New identifiers with `format_ident!`; raw identifiers | Lesson 02 task C |
| The three pieces of an impl header; `split_for_impl` | Lesson 03 task C; lessons 04, 07, 08; C01 |
| Which bounds a derive adds: per type parameter or per field type | Lesson 03 tasks B and C; drills d01 and d09 |
| Helper attributes and `parse_nested_meta` | Lesson 03 task A; lessons 07 and 08; C01 |
| Spans for new tokens with `quote_spanned!` | Lesson 03 task B; lesson 08; C01 |
| `syn::Error`: spans, `new_spanned`, combining several errors | Lesson 04; every later lesson |
| A macro must never panic | Lesson 04 task C; every later lesson; C01 |
| Compile-fail tests with trybuild; `.stderr` files; `TRYBUILD=overwrite` | Lesson 04 task D; capstone |
| Attribute macros: arguments, the item, keeping the item on an error | Lesson 05 |
| Running code at every exit of a function with a guard | Lesson 05 task B |
| Writing a parser: `Parse`, `ParseStream`, `Punctuated`, `braced!` | Lesson 05 task A; lesson 06 |
| Parse, then check, then generate; reporting all problems | Lessons 04, 06 and 08 |
| Functions that must be usable in a `const` | Lesson 06 |
| Hygiene of local variables: `Span::mixed_site` | Lesson 07 task A; lesson 08 |
| Absolute paths; names in patterns that collide with items | Lesson 07 task A; lessons 06 and 08 (hostile modules) |
| The path to your own crate: `extern crate self`, a `crate = path` argument | Lesson 07 task B |
| Choosing tests: tokens, pass cases, compile-fail cases, hostile modules | Lesson 07 task C; CONCEPTS lesson 07 |
| Deciding from syntax, and the limits of that | Lesson 08; CONCEPTS lessons 04 and 08 |
| What the standard derives generate | Compiler drills |
| Reviewing a macro that "works" | C01 |
| Looking at an expansion while debugging | Optional labs in lessons 01 and 07; CONCEPTS lesson 07 |

Not covered here, on purpose: build scripts and code generation outside macros ([`mastery/m6_ffi`](../m6_ffi/README.md) covers build scripts), and unstable features of `proc_macro` such as `Span::def_site` and diagnostics with several spans, which need a nightly compiler.

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Describe the life of a derive macro: which crate it is in and why, when it runs, what it receives, what it returns, and what the compiler does with the result. Say what a macro cannot know about the types it sees.
- [ ] For `struct View<'a, T: Clone = u8> where T: Send { .. }`, write the header of the impl that a derive must generate. Compare "one bound per type parameter" with "one bound per field type", and give a struct for which each one is wrong.
- [ ] A user misspells an argument of your helper attribute. Say exactly what your macro returns, how the compiler knows where to underline, why a panic is the wrong reaction, and which two kinds of test check the message and its position.
- [ ] Explain why an attribute macro returns the item even when it reports an error. Give two ways to run code at the end of a function, and say for each what it misses or changes.
- [ ] Name the three kinds of names in generated code, and the protection for each. Explain why a name in a pattern needs more than hygiene, and why a unit test on tokens cannot show that a macro is hygienic.
- [ ] Redo the capstone after a break without looking at your first solution. Then invent a changed requirement (for example: a missing required field must be a compile error, not a run-time error) and explain what it changes in the generated code and in your tests.

## Solutions

Model solutions are in [solutions/mastery/m5_proc_macros/](../../solutions/mastery/m5_proc_macros/): one file per lesson at the same path as the lesson file, with the predictions filled in, and the model answers for the test cases that you write yourself (`tests/ui/l04/own_*`, `tests/ui/l08/own_*`). The repaired drills and the expected outcome of every drill are in [solutions/drills/m5/](../../solutions/drills/m5/). Use them after an honest attempt and after the hints: compare your solution with the model, and note in NOTES.md what differs and why. Reading a solution first removes the struggle that makes the lesson stay in memory. A macro has more than one good design, so your types and helper functions will differ from the model's; the contract and the tests are what counts.
