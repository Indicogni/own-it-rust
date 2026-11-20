# Chapter 8 — Modules, crates and Cargo

[All chapters](../../README.md) · [Previous chapter](../chapter07/README.md) · [Next chapter](../chapter09/README.md)

A program that grows needs walls. This chapter teaches how Rust code is divided into packages, crates and modules, and how you decide which parts other code may depend on. You will hide implementation details behind a small public surface, choose between `const` and `static`, switch code on and off at compile time, write the three kinds of tests that Cargo runs, and read dependency requirements. A checked lab then has you finish a real workspace: a library, two binaries, an optional dependency behind a feature, and a build script.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter08 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations or compare pointers, so a solution that copies everything fails. Then run `cargo clippy --test chapter08` and fix warnings in your code.
5. **Drill the compiler.** `cargo drill ch08` shows small programs; predict whether each compiles (and which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

After the numbered lessons, do the **lab** (a separate small workspace with its own checks), the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: fix a teammate's working-but-wrong code). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Crates, editions and profiles](l01_crates_profiles.rs) |
| [ ] | [02 · Nested modules and private implementation](l02_modules.rs) |
| [ ] | [03 · Paths, imports and reexports](l03_paths_exports.rs) |
| [ ] | [04 · Constants and statics](l04_const_static.rs) |
| [ ] | [05 · Attributes, cfg and included files](l05_attributes.rs) |
| [ ] | [06 · Tests and executable documentation](l06_tests_docs.rs) |
| [ ] | [07 · Dependency requirements and lockfiles](l07_dependencies.rs) |
| [ ] | [08 · Capstone: library facade and workspace](l08_capstone.rs) |
| [ ] | [Lab · A workspace you finish by hand](label_lab/README.md) — `cargo test --manifest-path exercises/chapter08/label_lab/Cargo.toml` |
| [ ] | [R01 · Review: Chapters 4, 6 and 7 from memory](r01_review.rs) |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) |
| [ ] | [Compiler drills](../../drills/ch08/) — `cargo drill ch08` |

```sh
cargo test --test chapter08 l01_
cargo test --test chapter08 -- --list
cargo test --test chapter08
cargo test --release --test chapter08
cargo clippy --test chapter08
cargo drill ch08
cargo run --example ch08_scratch
cargo test --manifest-path exercises/chapter08/label_lab/Cargo.toml
cargo test --manifest-path exercises/chapter08/label_lab/Cargo.toml --all-features
```

The starters compile; tests fail intentionally at `todo!()` and `guess()` until you fill them in. Work in lesson order and keep tests intact. Every task uses the standard library and needs no network. Free-form experiments belong in [the scratch example](../../examples/ch08_scratch.rs); restore it to compiling code after each intentional error. The lab is a separate workspace in [`label_lab/`](label_lab/README.md) with its own manifest, lockfile and tests, so nothing you do there can break the chapter's test target.

Use [progressive hints](HINTS.md) only after a first attempt.

**Time (estimate):** 9 to 11 hours for the whole chapter: about 5 hours for the eight lessons including the reading, 2 to 3 hours for the lab, and about 2.5 hours for the review, the code review and the drills. The optional labs add about 1 hour.

## Topic map

"Optional lab" means a manual experiment in [LABS.md](LABS.md) that no test checks.

| Topic | Where you practise it |
| --- | --- |
| Package, target and crate; crate names | 01: predictions about this repository's own crates; optional lab 1 |
| Editions, toolchain version and `rust-version` | 01: reading in CONCEPTS.md and the explanation task; optional lab 1 |
| Build profiles and `debug_assertions` | 01: a profile-dependent function, tested in debug and release; lab step 7 |
| Module tree, privacy, `pub(super)` and `pub(crate)` | 02 and 03; drills d01, d08; C01 |
| Modules in their own files | lab step 2 (checked by `lab_check`) |
| Paths, `use`, aliases, glob imports, the prelude | 02 and 03 predictions; drills d03, d09 |
| Reexports and a facade over private modules | 03, 08; lab step 2 with `compile_fail` doc-tests |
| Private fields and constructors | 03; lab step 2; drills d02, d11; C01 |
| `const`, `static`, `const fn`, atomics in statics | 04; drills d04, d05; C01; optional lab 3 |
| `#[cfg]` and `cfg!`; `env!` and `option_env!`; `include_str!` | 05; drills d06, d07, d10; optional lab 4 |
| Unit tests, integration tests, doc-tests | 06 (a tested public contract); lab steps 1 to 3 |
| Turning a program into a library plus thin binaries; `src/bin` | 08; lab step 4 (binaries tested as processes) |
| Version requirements and the pre-1.0 rule | 07 |
| `Cargo.lock`, path dependencies with a version | lab steps 5 and 8 (step 8 is an optional observation) |
| Workspaces and shared settings (`[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints]`) | lab step 5 |
| Cargo features, optional dependencies, feature unification | lab step 6, tested with and without the feature |
| Build scripts (`cargo::rustc-env`, `rerun-if-changed`) | lab step 7 |
| Package contents (`cargo package --list`) | lab step 8 (optional observation); never published |
| Earlier chapters: moves, `match` and `let else`, error conversion | R01 |

Not covered here: designing public APIs for the long term (API guidelines, which changes break users, feature design in depth, publishing). That is the subject of the mastery module [`mastery/m8_api`](../../mastery/m8_api/README.md).

## Solutions

Model answers for every lesson, the lab and the drills are in `solutions/exercises/chapter08/` and `solutions/drills/ch08/` of the course repository. Open them only after an honest attempt and after the hints: reading a solution first takes away the struggle that makes the knowledge stay. When you compare, look for differences in *approach*, not in spelling.

## Mastery check

- [ ] Explain package versus crate versus module, and name the crates that Cargo builds from one package with a library, a binary and an integration test.
- [ ] Control visibility with private modules, private fields and public reexports; say which kind of test can prove that something is hidden.
- [ ] Split modules into files without changing any path that callers use.
- [ ] Distinguish const from static, and compile-time cfg from a runtime if.
- [ ] Write unit tests, integration tests and doc-tests, and say what each kind can see.
- [ ] Explain a version requirement, the lockfile, and why a workspace has only one lockfile.
- [ ] Add a feature that switches on an optional dependency, and explain why features must only add.
- [ ] Say what a build script is, when it runs, and how it passes a value to the crate.
- [ ] Redo the final capstone after a break without looking at your implementation; invent a changed requirement and explain the tradeoff.

Model answers are at the end of [HINTS.md](HINTS.md). Tests cannot check everything: whether a program is *rejected* by the compiler, or what users of a crate can reach, is shown by the drills, by the `compile_fail` doc-tests in the lab, and by your own experiments in the optional labs.
