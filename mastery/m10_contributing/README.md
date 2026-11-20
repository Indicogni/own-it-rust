# M10 · Working in real codebases and contributing

[All chapters](../../README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Most Rust you will ever touch was written by somebody else. This module is practice
for that work: finding your way in a project you did not write, turning a vague
report into a failing test, hunting a regression through history, making a change
that its maintainers can accept, reviewing and being reviewed, writing for people who
have little time, reporting unsound code, upgrading a dependency, and finding a way
into the Rust project itself.

Module [M9](../m9_toolbox/) is about the tools. This module is about the work that
the tools serve.

## Who it is for

You have finished chapters [1](../../exercises/chapter01/README.md) to [24](../../exercises/chapter24/README.md), or you write Rust at that level. You can read
traits, lifetimes and `unsafe` blocks without a reference at your side. You have used
git: commit, branch, diff, log. You do not need experience with open source; getting
it is the point.

Lesson 07 uses Miri, which [M2](../m2_unsafe/) introduces. Lesson 04 uses the
compatibility rules that [M8](../m8_api/) treats in depth. Both lessons explain what
they need, so you can do this module first.

## The project you work on: `fixture/`

`fixture/` is **tasklane**, a small task runner in three crates, written for this
module. It has what a real project has: a module tree to learn, a public API with
documentation, a test suite, a [CONTRIBUTING.md](fixture/CONTRIBUTING.md), a
[changelog](fixture/CHANGELOG.md), and an [issue list](fixture/issues/README.md) with
reports of every quality. It also has what real projects have and do not advertise:
bugs, a performance regression, an unsound function, an awkward API that cannot be
changed yet, and a test that fails every other run.

You change the fixture in most lessons. The tests of this package are the acceptance
tests for your changes; the fixture's own tests are the project's test suite, and
part of your work is to extend it.

```text
m10_contributing/
├── src/lNN_*.rs        the lessons: tasks, answers, tests
├── answers/            texts you write: commit messages, replies, an advisory
├── fixture/            tasklane, the project you work on
│   ├── crates/         tasklane-plugin, tasklane-core, tasklane-cli
│   ├── issues/         the issue tracker, as Markdown files
│   └── examples/       example Lanefiles
├── downstream/         two crates that use tasklane from outside (lessons 04, 08)
├── reviews/            a pull request to review, and a review of your own change
└── tests/downstream.rs builds the downstream crates against the fixture
```

## Setup

This is a standalone Cargo package with its own `Cargo.lock`. Rust 1.99.0 is pinned
in `rust-toolchain.toml`, identical to the one in the repository root. The only
dependency from crates.io is [bitflags](https://crates.io/crates/bitflags), which the
fixture uses in two versions on purpose (lesson 08). Run the commands from the
repository root:

```sh
# The first build downloads bitflags; afterwards you can add --offline.
cargo test --manifest-path mastery/m10_contributing/Cargo.toml --locked --all-targets --no-run
cargo test --manifest-path mastery/m10_contributing/Cargo.toml --locked --release --all-targets --no-run
```

The starter compiles. Most tests fail at `todo!()`, at `guess()`, or because the
fixture does not behave as the lesson wants yet. That is the starting state. Some
tests pass from the start: they describe behaviour that your changes must keep.

From lesson 08 on, leave out `--locked`: the lesson changes the lock file.

Three labs need more than the pinned toolchain. Each one says what it writes and
where; nothing in this module changes the git state of the course repository.

| Lesson | You need | What it writes outside this package |
| --- | --- | --- |
| 03 | `git` 2.32 or newer | a practice repository in your system's temporary directory (`make-history` prints the path, and refuses a directory inside a git repository); delete it when you are done |
| 07 | a nightly toolchain with Miri | `rustup toolchain install nightly-2026-11-20 --component miri --component rust-src` adds a toolchain to your rustup installation; `rustup toolchain uninstall nightly-2026-11-20` removes it again, if you have no other use for nightly |
| 09 | a network connection and about 8 GB of free disk space | a clone of Clippy in a directory you choose **outside** the course repository, and the nightly toolchain that Clippy pins, which rustup installs by itself (about 1 GB); the lab says how to remove both |

The nested builds of `--test downstream` write `downstream/Cargo.lock`, which this
package's `.gitignore` lists. Running cargo inside `fixture/` creates
`fixture/target/`, which is ignored too.

## Work through the module

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md), then the header of
   the lesson file: what to predict, what to do, what to explain.
2. **Predict.** Answer the `p_predictions` test first: replace each `guess()` with a
   value. Then run the lesson, for example
   `cargo test --manifest-path mastery/m10_contributing/Cargo.toml l02_`.
3. **Do the work.** Some tasks are `todo!()` bodies in the lesson file. Many are
   changes in `fixture/` or texts in `answers/`; the lesson header says which.
   Answers that are words (a module path, a test name, a commit subject) are checked
   by a digest, so you cannot read them from the test.
4. **Check the project's own rules.** After every change to the fixture, run its
   test suite and clippy, as its CONTRIBUTING.md asks:
   `cargo test --workspace` and `cargo clippy --workspace --all-targets` inside
   `mastery/m10_contributing/fixture`.
5. **Explain.** Write the EXPLAIN answer in [NOTES.md](NOTES.md), then compare with
   *Check your explanation* in [HINTS.md](HINTS.md).

| Done | Lesson | You practise | Estimate |
| --- | --- | --- | --- |
| [ ] | [01 · Orienting in an unfamiliar codebase](src/l01_orienting.rs) | building and running a strange project; mapping crates, modules and data flow; finding code through tests, docs and search | 2 h |
| [ ] | [02 · Reproducing a bug report](src/l02_reproduce.rs) | reducing a vague report to a minimal failing test; a fix that changes nothing else | 2 h |
| [ ] | [03 · Regression hunting with git bisect](src/l03_bisect.rs) | `git bisect run` with a probe test and a script that skips commits that do not build | 2 h |
| [ ] | [04 · Making a change the maintainers will accept](src/l04_change.rs) | a feature inside the existing design; tests, docs, changelog; no breaking change | 3 h |
| [ ] | [05 · Code review, both sides](src/l05_review.rs) | reviewing a patch series; judging review comments; answering a review of your own work | 3 h |
| [ ] | [06 · Writing for maintainers](src/l06_writing.rs) | commit message, pull request description, issue report with a minimal reproduction | 2 h |
| [ ] | [07 · Unsafe code and soundness reports](src/l07_soundness.rs) | finding an unsound function; showing it with Miri; fixing it; writing the advisory | 3 h |
| [ ] | [08 · Dependencies and upgrades](src/l08_dependencies.rs) | a duplicated dependency; an upgrade across a breaking release; what the compiler does not catch | 2–3 h |
| [ ] | [09 · Contributing to the wider ecosystem](src/l09_ecosystem.rs) | how the Rust project is run; building Clippy from source; a plan for a first contribution | 3–4 h |
| [ ] | [10 · Capstone: one contribution, from issue to merge](src/l10_capstone.rs) | all of it, on one issue of your choice | 4 h |

The times are estimates for a first pass, including reading. Do the lessons in
order: 05 and 06 build on your changes from 02 and 04, and the capstone uses
everything.

```sh
cargo test --manifest-path mastery/m10_contributing/Cargo.toml -- --list
cargo test --manifest-path mastery/m10_contributing/Cargo.toml l01_
cargo test --manifest-path mastery/m10_contributing/Cargo.toml --release l01_
cargo test --manifest-path mastery/m10_contributing/Cargo.toml --test downstream
cargo clippy --manifest-path mastery/m10_contributing/Cargo.toml --all-targets
cargo run --manifest-path mastery/m10_contributing/Cargo.toml --bin make-history

# The project itself, as its contributors work on it:
cd mastery/m10_contributing/fixture
cargo test --workspace
cargo clippy --workspace --all-targets
cargo run -p tasklane-cli -- -f examples/site.lane run site
```

Lesson filters (`l01_` … `l10_`) match the module path of the unit tests. A filtered
run also starts the `downstream` test binary, which then reports 0 tests. The one
exception is `l04_`: it also matches the downstream test of lesson 04, on purpose.

## Labs

A lab is a task with a tool that a test of this package cannot run for you. Each lab
states the exact command, what you should see, and a self-check.

| Lab | Lesson | Tool | Checked by |
| --- | --- | --- | --- |
| Bisecting a scripted history | 03 | `git bisect run` | your recorded results, by digest |
| Watching Miri find undefined behaviour | 07 | `cargo +nightly-2026-11-20 miri test` | the same test under `cargo test`, plus your own eyes |
| Building Clippy from source | 09 | `git`, `cargo` | the form of your lab notes |

## Topic map

| Topic | Lessons |
| --- | --- |
| Reading code: entry points, module tables, tests as documentation | 01, 02, 10 |
| Minimal reproductions and regression tests | 02, 03, 07, 10 |
| git: bisect, skip, untracked probe files, reading a commit | 03, 05 |
| Semantic versioning in practice: what breaks a user | 04, 05, 08 |
| `#[non_exhaustive]`, private fields, default methods, deprecation | 04, 05 |
| Review: severity, evidence, declining a request | 05 |
| Commit messages, pull requests, issue reports, advisories | 06, 07, 10 |
| Soundness, safety comments, Miri and its limits | 07 |
| Dependency trees, duplicates, changelogs, pinned versions | 08 |
| RFCs, FCP, stabilisation, editions, crater, triagebot, licences, LLM policy | 09 |

## Mastery check

You have mastered this module when you can do these without looking anything up:

1. Open a project you have never seen and, within an hour, say what each crate is
   for, follow one input from the entry point to the output, and name the test that
   pins a behaviour you were asked about.
2. Turn a report without a reproduction into a test of a few lines that fails for
   the reported reason, and say what a fix must not change.
3. Find the commit that introduced a regression with `git bisect run`, including in
   a history where some commits do not build, and explain the exit codes.
4. Look at a proposed API change and say whether it breaks a user, why, and which
   design would not.
5. Review a patch: separate what blocks a merge from what is a suggestion, check a
   claim before you post it, and answer a review of your own work, including a
   request you decline.
6. Write a commit message, a pull request description and an issue report that a
   maintainer can act on without asking back.
7. Explain what makes a safe function unsound, show it with a test under Miri, and
   say what Miri cannot see.
8. Upgrade a dependency across a breaking release: read its changelog, find the
   changes that the compiler reports and the ones it does not, and keep your own
   users unbroken.
9. Describe how a change reaches stable Rust (RFC or ACP, implementation behind a
   feature gate, FCP, stabilisation), where the rules are written, and what the
   project asks of contributors who use an LLM.
10. Redo the capstone with a different issue, after a break, without looking at
    your first solution.

## Capstone rubric

Grade your capstone yourself, one point per line. Eight or more is ready for a real
project; below that, repeat the weak step with another of the three issues.

| | Question |
| --- | --- |
| [ ] | Did you reproduce the problem with a failing test in the fixture *before* you changed any code? |
| [ ] | Is the test minimal: would deleting any line of it make it pass or stop compiling? |
| [ ] | Can you state the cause in one sentence that names a function and a line? |
| [ ] | Is the fix the smallest change that removes the cause, and nothing else? |
| [ ] | Do `cargo test --workspace` and `cargo clippy --workspace --all-targets` pass in the fixture? |
| [ ] | Do the `unchanged` tests of your issue and `--test downstream` still pass? |
| [ ] | Does the changelog entry say what a user saw, not what you edited? |
| [ ] | Does the commit message follow every rule of the fixture's CONTRIBUTING.md? |
| [ ] | Does the pull request say how you tested, in commands that a reviewer can run? |
| [ ] | Would you approve this pull request if somebody else had sent it? |

## Where the solutions are

`solutions/mastery/m10_contributing/` in the repository root mirrors this directory:
each file there is the solved version of the file with the same path here. That
includes the fixed fixture files, the model texts in `answers/`, and the lock files
after the upgrade of lesson 08. Look at a solution after you have a version of your
own; the texts in particular have many good answers, and the model is one of them.

## Validation scope

The starter compiles in debug and release with `--locked`, and `cargo clippy
--all-targets` is clean for this package, for the fixture and for the downstream
crates. With the solutions copied over the package, every test passes in both
profiles, including the two nested builds of `--test downstream`; the fixture's own
suite passes, and its flaky test no longer flakes.

Tool versions used for the labs: the Miri lab was run on the unfixed and on the fixed
fixture with `nightly-2026-11-19` (rustc 1.102.0-nightly). The scripted history of
lesson 03 was built and bisected with git 2.56; the generator needs git 2.32 or
newer. The Clippy lab was run in November 2026 on commit `ddecc859` of
`rust-lang/rust-clippy`, which pins `nightly-2026-11-17`, with rustup 1.29; its
commands come from the Clippy book, which is where to look if they stop working.
The external links of this module were checked in the same month.
