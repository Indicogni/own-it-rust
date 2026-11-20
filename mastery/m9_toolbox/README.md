# M9 · The professional toolbox

[All chapters](../../README.md) · [Concepts](CONCEPTS.md) · [Labs](LABS.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

This module is about the work around the code: Cargo in depth, the editor, lints, test tooling, fuzzing, debugging, documentation, supply-chain checks, CI and releases. It is what a developer needs to be productive in any Rust project on the first day, and to run one well.

## Who it is for

You can write Rust. You have finished chapters [1](../../exercises/chapter01/README.md) to [22](../../exercises/chapter22/README.md), or you work with Rust already and want the tooling to stop being a mystery. You are about to contribute to an existing project, or to maintain one.

## Prerequisites

- Chapters [1](../../exercises/chapter01/README.md) to [22](../../exercises/chapter22/README.md) of the course.
- The [ecosystem track](../../ecosystem/README.md), above all its [tooling workday](../../ecosystem/TOOLING.md) (E08) and property testing ([E05](../../ecosystem/README.md#e05--property-based-testing-with-proptest)). This module starts where they stop.
- Rust 1.99.0 with `rustfmt` and `clippy`. `rust-toolchain.toml` in this directory selects it, and rustup installs it on first use.
- For the labs only: the course's dated nightly toolchain, `nightly-2026-11-20` (see [The nightly toolchain](../../README.md#the-nightly-toolchain)), and some external tools. The lessons do not need them.

Other mastery modules cover neighbouring tools, and this module links to them instead of repeating them: Miri ([M2](../m2_unsafe/README.md)), loom ([M3](../m3_concurrency/README.md)), trybuild ([M5](../m5_proc_macros/README.md)), build scripts for C code ([M6](../m6_ffi/README.md)), criterion and profiling ([M7](../m7_perf/README.md)), API design and semantic versioning ([M8](../m8_api/README.md)). Reading an unfamiliar codebase and getting a change merged is the subject of [M10](../m10_contributing/README.md), which uses the tools from this module.

## How this module works

It is a standalone Cargo package with its own `Cargo.lock`. It is not part of the root package, so the chapter checks never build it. Run every command from the repository root.

```sh
# The first build downloads the pinned crates; afterwards you can add --offline.
cargo test --manifest-path mastery/m9_toolbox/Cargo.toml --locked --no-run
```

Tooling is learned on real projects, so most lessons give you one: a small crate or workspace under `fixtures/` that is in a bad state. You repair its manifests, configuration files and code. The lesson's tests then run real commands on it (`cargo metadata`, `cargo clippy`, `cargo doc`, `cargo package`, ...) and read the files you wrote. A few lessons also have ordinary Rust tasks with `todo!()` bodies.

The starters compile. Tests fail at `todo!()`, at `guess()`, or with a message that says what the fixture still lacks. That is the starting state, not a broken setup.

**Labs** are tasks that need a tool this package cannot ship: a fuzzer, a debugger, a coverage tool. They are in [LABS.md](LABS.md). Each lab gives the exact command, what you should see, and a self-check. Four labs end in a test marked `#[ignore]`, where you record what the tool showed you.

### Work through a lesson

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md), then the header of the lesson file: what to predict, the tasks, what to explain.
2. **Predict.** Answer `p_predictions` first: replace each `guess()` with a value. Many predictions are about what a Cargo command will do on a small fixture; read the fixture, decide, then let the test run the command.
3. **Do the tasks.** Run only that lesson, for example `cargo test --manifest-path mastery/m9_toolbox/Cargo.toml l03_`. Read the failure messages: they name the file and the rule.
4. **Check technique.** Some tests check *how*: that a build script really reruns, that a Clippy ban really fires, that a temporary directory is really private, that your test cases really catch seeded bugs.
5. **Explain** the EXPLAIN prompt in [NOTES.md](NOTES.md), add one edge case, then compare with *Check your explanation* in [HINTS.md](HINTS.md).
6. **Do the labs** of the lesson if you have the tools.

| Done | Lesson | You work on | Labs | Time (estimate) |
| --- | --- | --- | --- | --- |
| [ ] | [01 · Cargo in depth](src/l01_cargo.rs) | `fixtures/l01_ws/` (manifests, build script) | 1.1, 1.2 | 3 h |
| [ ] | [02 · Toolchains, the editor, generated code](src/l02_editor.rs) | one function; `fixtures/l02_setup/` | 2.1 to 2.3 | 1.5 h |
| [ ] | [03 · Lints and formatting as policy](src/l03_lints.rs) | `fixtures/l03_lints/` | | 2 h |
| [ ] | [04 · Testing at scale](src/l04_testing.rs) | six functions; `fixtures/l04_doctests/` | 4.1 to 4.4 | 3 h + 1.5 h |
| [ ] | [05 · Fuzzing and sanitizers](src/l05_fuzz.rs) | codec, `Arbitrary`, harness | 5.1 to 5.4 | 2.5 h + 1.5 h |
| [ ] | [06 · Debugging](src/l06_debug.rs) ([process tests](tests/l06_process.rs)) | one function; `src/bin/stockroom.rs` | 6.1 | 2 h + 1 h |
| [ ] | [07 · Documentation](src/l07_docs.rs) | `fixtures/l07_docs/` | 7.1 | 2 h |
| [ ] | [08 · Supply chain and policy](src/l08_supply.rs) | three functions; `fixtures/l08_policy/` | 8.1 to 8.5 | 2.5 h + 1 h |
| [ ] | [09 · CI and release](src/l09_release.rs) | `fixtures/l09_ship/` | 9.1, 9.2 | 3 h |
| [ ] | [10 · Capstone](tests/l10_capstone.rs) ([task](capstone/TASK.md)) | `capstone/` | | 5 to 8 h |

Times are estimates for the lesson plus, after the `+`, its labs. Lessons 01 to 09 are independent, with two small exceptions: lesson 09 uses the commands of 03, 04 and 08, and the capstone needs all of them. The suggested order is the numbered one.

### Commands

```sh
cargo test --manifest-path mastery/m9_toolbox/Cargo.toml -- --list
cargo test --manifest-path mastery/m9_toolbox/Cargo.toml l01_
cargo test --manifest-path mastery/m9_toolbox/Cargo.toml l06_          # also runs tests/l06_process.rs
cargo test --manifest-path mastery/m9_toolbox/Cargo.toml --test l10_capstone
cargo test --manifest-path mastery/m9_toolbox/Cargo.toml --release l04_
cargo test --manifest-path mastery/m9_toolbox/Cargo.toml lab_ -- --ignored   # lab answers
cargo run  --manifest-path mastery/m9_toolbox/Cargo.toml --bin stockroom
cargo clippy --manifest-path mastery/m9_toolbox/Cargo.toml --all-targets
cargo fmt --manifest-path mastery/m9_toolbox/Cargo.toml --all --check
rustdoc --edition 2024 --test mastery/m9_toolbox/CONCEPTS.md
```

Notes on running the tests:

- A filtered run also starts the other test executables, which then report 0 tests.
- The tests start Cargo themselves, on the fixtures. Those nested builds go to a directory inside this package's own `target/` (`target/m9-nested/`), never into the fixtures. The first run of a lesson compiles its fixture and takes a few seconds; later runs are fast.
- Nested commands run with `--offline` behaviour, except in the capstone, whose workspace depends on `insta`.
- The fixtures are separate projects. You can `cd` into one and run Cargo there by hand; that is often the fastest way to see a full diagnostic. (It creates a `target/` directory inside the fixture, which git ignores.) Cargo reads `.cargo/config.toml` from the directory you are in, so the alias of lesson 01 and the linker setting of lesson 09 only apply when you run Cargo from inside their fixture.
- The lessons and their tests install nothing and never run git. They build inside `target/` and use the system's temporary directory. In the source tree, two things can change besides your own edits. When you change the manifests of a fixture, Cargo brings that fixture's `Cargo.lock` up to date (lesson 01). And a snapshot test that fails in lesson 04 leaves a `*.snap.new` file next to the snapshot; git ignores it, and you can delete it (lab 4.2 shows what it is for). The labs install tools; [LABS.md](LABS.md#setup) says exactly what each command changes on your machine and how to undo it.
- A test that could wait forever on a wrong answer (a deadlock, a loop that never advances) stops after 20 to 60 seconds with a message that says so.
- The tests also pass under `cargo nextest run`.

## Topic map

| Topic | Lesson | Tasks and predictions |
| --- | --- | --- |
| Workspace inheritance: package fields, dependencies, lints | 01 | A, B, C |
| Resolver 3, feature design, `dep:`, unification | 01 | A, D, P1 to P3 |
| Target-specific dependencies | 01 | E |
| Profiles and where they are read | 01, 09 | 01 F; 09 C, P1 to P5 |
| Build scripts, custom cfg, rerun rules | 01 | G |
| xtask and Cargo aliases | 01 | H |
| `cargo metadata`, `cargo tree`, `--timings`, `[patch]`, vendoring, offline | 01 | tests use them; labs 1.1, 1.2 |
| Build times: what makes a build slow and how to measure it; see also the guide [Taming compile times](../../guides/COMPILE_TIMES.md) | 01 | reading only |
| rustup toolchains, components, overrides | 02 | A, B; lab 2.1 |
| rust-analyzer needs and settings | 02 | C; lab 2.3 |
| Reading expanded code | 02 | P1 to P3; lab 2.2 |
| Lint levels and groups, `[lints]`, priority | 03 | A, P1 to P3 |
| `#[expect]` with reasons | 03 | B |
| `clippy.toml`, banning APIs | 03 | C |
| `rustfmt.toml`, stable options | 03 | D |
| Warnings as errors without breakage | 03, 09 | 03 B, E; 09 A |
| Test organisation | 04 | P1 to P3 |
| Snapshot tests, redaction | 04 | A, B; lab 4.2 |
| Doc tests as contract | 04, 07 | 04 C; 07 A, B |
| Fixtures, temporary directories, flaky tests | 04 | D |
| Coverage and its limits, mutation testing | 04 | E; labs 4.3, 4.4 |
| cargo-nextest | 04 | lab 4.1 |
| Fuzz targets, `arbitrary`, minimising | 05 | A, B, C; labs 5.1, 5.2 |
| Sanitizers | 05 | labs 5.3, 5.4 |
| Backtraces, exit codes, `dbg!`, panic hooks | 06 | A, P1 to P5 |
| Symptom to root cause; a debugger | 06 | C; lab 6.1 |
| rustdoc, intra-doc links, README as crate docs | 07 | A, B |
| `doc(cfg)`, docs.rs metadata | 07 | C, P3; lab 7.1 |
| Examples directory, changelog | 07 | D, F |
| Lock files, version requirements | 08 | A, C, P1 to P5 |
| Advisories, cargo-audit | 08 | B; lab 8.1 |
| cargo-deny: licences, bans, sources | 08 | D; lab 8.2 |
| MSRV and how to test it; minimal versions; cargo-vet | 08 | E; labs 8.3 to 8.5 |
| A CI pipeline: matrix, caching, pins, MSRV job | 09 | A |
| Packaging and publishing, yanking | 09 | B; lab 9.1 |
| Cross-compilation, target cfg | 09 | D, P6; lab 9.2 |
| Everything together | 10 | capstone |

## Tool versions

The lessons need only what `Cargo.toml` pins. The labs use external tools. These are the versions the outputs in [CONCEPTS.md](CONCEPTS.md) and [LABS.md](LABS.md) were produced with. Tools change; when a flag differs on your machine, trust `--help` and the tool's own documentation.

| Tool | Version checked | Used in |
| --- | --- | --- |
| rustc, cargo, clippy, rustfmt | 1.99.0 | everything |
| rustup | 1.29.3 | lesson 02 |
| nightly toolchain | `nightly-2026-11-20`, the course's dated nightly: rustc 1.102.0-nightly (6905905bb 2026-11-19) | labs 5.x, 7.1, 8.4 |
| cargo-expand | 1.0.127 | lab 2.2 |
| cargo-nextest | 0.9.152 | lab 4.1 |
| cargo-insta | 1.49.0 | lab 4.2 |
| cargo-llvm-cov | 0.9.1 | lab 4.3 |
| cargo-mutants | 27.1.0 | lab 4.4 |
| cargo-fuzz | 0.13.2 | labs 5.1, 5.2 |
| LLDB (through `rust-lldb`) | 2103.0 (Xcode) | lab 6.1 |
| cargo-audit | 0.23.0 | lab 8.1 |
| cargo-deny | 0.22.0 | lab 8.2 |
| cargo-vet | 0.10.3 | lab 8.5 |
| Rust 1.85.0 (the MSRV of two fixtures) | 1.85.0 | lab 8.3 |
| GitHub actions in the model workflows | `actions/checkout` v7 (7.0.1), `Swatinem/rust-cache` v2 (2.9.3), `EmbarkStudios/cargo-deny-action` v2 (2.1.3) | lessons 09, 10 |

| Crate (pinned with `=`) | Version | Why |
| --- | --- | --- |
| toml | 1.2.1 | tests read manifests and configuration files |
| serde_json | 1.0.154 | tests read `cargo metadata` and compiler messages |
| serde_norway | 0.9.42 | tests read workflow files (YAML) |
| semver | 1.0.28 | lesson 08 |
| arbitrary | 1.5.0 | lesson 05 |
| insta (dev) | 1.49.0 | lesson 04 |
| tempfile (dev) | 3.27.0 | lessons 04 and 10 |

## Mastery check

Answer from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

1. Two members of a workspace depend on the same crate with different features. What gets built when you run `cargo build -p one-member`, and what when you run `cargo build --workspace`? Why must features be additive?
2. Your manifest says `unsafe_code = "forbid"` and CI runs `cargo clippy -- -D warnings`. A new stable compiler adds a lint that fires in your code. What happens to users of your published crate, and what happens in your CI? What decides the answer?
3. A test suite has 100% line coverage. Give two different reasons why a bug in the covered code can still pass, and name the tool that would find each.
4. A fuzzer runs your parser for an hour and reports nothing. List three things that must be true before that result means anything.
5. A program panics with "index out of bounds" at a line that looks correct. Describe your next three steps, and the mistake to avoid.
6. What does `Cargo.lock` guarantee, and what does it not? Who should commit it, and which flag makes CI respect it?
7. Your crate says `rust-version = "1.85"`. Name two independent ways the promise can be broken without any test failing, and the check that catches each.
8. List everything in a CI run that can change without a commit, and how you pin each.

## Where the solutions are

Solved versions of every file you edit are in `solutions/mastery/m9_toolbox/`, at the same relative paths: lesson files, fixture files, the capstone. Compare after you have a green test, not before. For configuration files there is often more than one good answer; the tests accept any that meets the stated rules.

## Validation scope

Checked in November 2026 with Rust 1.99.0 on macOS (Apple silicon), twice: by the author, and by an independent reviewer who worked through every lesson, lab and the capstone from the starter files without looking at the solutions.

- **Starter:** compiles in debug and release; every lesson test fails at `todo!()`, at `guess()`, or with a message about the fixture.
- **Solutions:** with `solutions/` applied, every test passes in both profiles, three runs each, including the lab checks (`--include-ignored`), under `cargo test` and under `cargo nextest`; `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean. `cargo xtask ci` passes in the solved lesson 01 workspace, and `cargo deny check` passes in the solved lesson 09 crate and capstone.
- **A second, independent set of answers** (the reviewer's) passes the same tests, so the tests do not depend on the model answers' choices. Where the two sets differ in a way the text allows (for example the toolchain version in the workflow, or in `rust-toolchain.toml`), the tests accept both.
- **Wrong answers:** plausible wrong answers were run against the technique tests of every lesson, and are caught. That includes answers that used to hang the test run; they now stop with a message.
- **Labs:** every lab was run with the tool versions above, except lab 2.3 (rust-analyzer in an editor) and the `cross` step of lab 9.2. Lab 8.3 was run with a real 1.85.0 toolchain.
- **Git:** the package-content test of lesson 09 was run inside a git repository and outside one, because Cargo lists different files in the two cases.
- **Links:** every external link in this module was fetched on 2026-11-20.

Not checked: Linux and Windows. The tests avoid shell tools and platform-specific paths, and CI should confirm them there. The two model workflow files are parsed by the tests but were never executed on GitHub.
