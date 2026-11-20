# Authoring guide

[Course overview](README.md)

How "Own It: Hands-On Mastery of Rust" is built, for anyone who adds or changes a unit. Learners do not need this file. [Chapter 4](exercises/chapter04/) is the reference implementation of a chapter; copy its patterns. The mastery modules, capstones and checkpoints each have a pattern of their own, described below.

## Rules for every file

- **Title.** The course is called "Own It: Hands-On Mastery of Rust". Use exactly that name where a title is needed, and no other or earlier name.
- **Standalone and original.** The course teaches by itself. Do not refer to, quote, closely paraphrase, or copy the structure of any book, paid or free, or of other courses and tutorials. Write your own text and your own examples. Link freely to the official documentation under `https://doc.rust-lang.org/` (the standard library, the Rust Reference, the Rustonomicon, the Cargo book, the Edition Guide, the rustc book).
- **Accuracy first.** The toolchain is pinned to Rust 1.99.0, edition 2024. Check every claim, code block, diagnostic and error code with that compiler. Never invent a diagnostic; paste a real one, trimmed, in a `text` block.
- **Licences.** Code is MIT ([LICENSE](LICENSE)); prose and figures are CC BY-NC-SA 4.0 ([LICENSE-TEXT](LICENSE-TEXT)); [NOTICE.md](NOTICE.md) explains the split. The copyright holder is Indicogni LLC. Until a contributor agreement is published, outside corrections arrive as issues, not pull requests; see [CONTRIBUTING.md](CONTRIBUTING.md). Do not add a personal name or an e-mail address anywhere. The repository URL appears only in the root README; refer to "the course repository" elsewhere, because the URL will change.
- **Nightly.** Where a unit needs nightly (Miri, fuzzing, sanitizers), write the course's one dated nightly, `cargo +nightly-2026-11-20 ...`, never a bare `+nightly`. It is the toolchain of the CI Miri jobs; the README's [nightly section](README.md#the-nightly-toolchain) says why it is dated. Move it forward only on purpose, in CI and every unit at once.
- **Style.** Short sentences in plain English that a non-native speaker can read. Define each term when you first use it. Use "you". No hype, no jokes that need cultural context, no emojis.

## Learning design

Each element exists to build long-term, usable skill rather than short-term recognition:

| Element | Why |
| --- | --- |
| `CONCEPTS.md` | The teaching text, read before each lesson: a mental model, a worked example, the real mistakes. |
| `p_predictions` tests | Retrieval and prediction before feedback. The learner writes a value, and the test checks it at once. |
| Compiler drills (`cargo drill`) | Day-to-day Rust is reading diagnostics. Learners predict the outcome, see the verdict, then fix the code. |
| Technique tests | A green test should mean that the intended technique was used, not just that the output is right. |
| `r01_review.rs` | Spaced, interleaved retrieval of earlier chapters, done from memory. |
| `c01_code_review.rs` | Working-but-wrong code: reviewing, refactoring and judging API design are core work skills. |
| `NOTES.md` | Written explanations are the main retention artefact and the basis for spaced review. |
| *Check your explanation* hints | Feedback on explanations, revealed only after the learner writes their own. |
| `cargo progress` | Status plus spaced reviews at 1, 3, 7, 21 and 60 days, each one different. |
| Checkpoints, capstones, final assessment | Transfer: building from a specification without step-by-step guidance. |

## CONCEPTS.md

Every chapter and every mastery module has a `CONCEPTS.md`, the instruction a learner reads **before** each lesson.

- Start with a short overview: what the learner can do afterwards, the prerequisites, and an estimate of the time per lesson, labelled as an estimate.
- One `## Lesson NN · Title` section per lesson, 400 to 900 words. A capstone gets a short section; R01 and C01 need none.
- Each section gives the mental model in plain words, building only on earlier lessons and chapters (say what is assumed); one **worked example that differs from the exercise** (never give away the solution), with a memory or ownership sketch in a `text` block where it helps; the two or three mistakes people really make, with the real diagnostic and the fix; "when to use which" guidance where there is a choice; and "Read more" deep links into the official documentation.
- Code blocks: ```` ```rust ```` for code that compiles and runs as a doctest; ```` ```rust,compile_fail,E0382 ```` for code that must not compile (with the error code where it is stable); ```` ```rust,should_panic ````; ```` ```rust,no_run ````; ```` ```text ```` for diagnostics and diagrams. The file must pass `rustdoc --edition 2024 --test path/CONCEPTS.md` with the pinned toolchain. Note that rustdoc checks the error code of a `compile_fail` block only on nightly, so compile those blocks once with `rustc` and check the code yourself.
- In a package with dependencies, mark a block that needs a crate `rust,ignore` only when it cannot be avoided, and compile that code another way (for example as a doc test of the package).
- Each lesson file's header has the line `Read first: "Lesson NN" in CONCEPTS.md (same directory).`

## Chapters (`exercises/chapterNN/`)

A chapter has `README.md`, `CONCEPTS.md`, `HINTS.md`, `NOTES.md`, `mod.rs`, lesson files `lNN_*.rs`, usually `r01_review.rs` and `c01_code_review.rs`, a drill set in `drills/chNN/`, and solutions in `solutions/exercises/chapterNN/` and `solutions/drills/chNN/`. Each chapter is one test target in the root `Cargo.toml` (`[[test]] name = "chapterNN"`).

### Lesson files (`lNN_*.rs`)

- Keep the existing contracts and tests. Add tests; do not weaken them.
- When you add a lesson, use the next free number, add its `mod` line to `mod.rs`, a row in the README lesson table in the recommended study order (which may differ from the numbers), entries in HINTS.md, a CONCEPTS.md section, and the solution file. A capstone keeps its file name and must contain the word `capstone`.
- Header: the `Read first` line, then the `PREDICT before running:` line, which starts with ``first answer `p_predictions` at the bottom (replace each `guess()` with your answer). Then consider:``, then IMPLEMENT and EXPLAIN prompts.
- **`p_predictions`**: every `lNN_*.rs` and `rNN_*.rs` file except the capstone has one, at the end of `mod tests`, with two to four predictions about *observable run-time behaviour* (values, drop order, pointer equality, allocation counts, `size_of`, iteration order, reference counts, overflow results).
  - Pattern: show the code, ask in a `// P1.` comment, `let p1: T = guess();`, then `assert_eq!(p1, <actual expression>, "P1: what it means")`.
  - The answer must not be readable from the source; the learner must evaluate the expression.
  - Every prediction must be deterministic on 64-bit Linux, macOS and Windows and in both profiles. Avoid hash iteration order, capacities after growth and thread interleavings.
  - Conceptual or "does it compile" questions belong in drills or NOTES, not here.
  - Chapter 4's `mod.rs` has a `droplog` helper (`DropLog::new()`, `log.noisy(name)`, `log.note(text)`, `log.entries()`) for drop-order predictions; copy it when you need one.
- **Technique tests** where the contract states a technique ("no clones", "borrow", "in place", "one allocation", "use threads"). Use `practice::alloc::count(|| ...)`, which returns `(result, allocations)` and counts only the current thread, and assert upper bounds (`<= n` or `== 0`), never an exact non-zero count: release builds can remove allocations. Other tools: `as_ptr()` and `ptr::eq`, `Rc::ptr_eq`, distinct `thread::current().id()` values, `size_of`, drop logs.
- **Anything that could block forever** under a plausible wrong solution runs inside `practice::within(secs, "what", move || ...)`, which runs the closure on a helper thread and fails with a clear message instead of hanging. Count allocations inside that closure.
- Every assertion that compares pointers, counts or predictions has a message that tells a newcomer what it means, for example `"handoff must return the same buffer, not a copy"`.
- Beware panic payloads: `panic!("literal")` gives a `&'static str` payload, a formatted value gives a `String`.
- A chapter that uses `count` needs this once in its `mod.rs`:

  ```rust,ignore
  #[global_allocator]
  static ALLOC: practice::alloc::Counting = practice::alloc::Counting;
  ```

- `guess` becomes an unused import once all of a file's predictions are answered. Every chapter's `mod.rs` allows `unused_imports` with a comment that says so; lesson files carry no `#[allow(unused_imports)]` of their own. Learners delete the `guess` import when they have answered every prediction, and every solution deletes it (the solution check ignores that one difference).

### Review lesson (`r01_review.rs`)

Three tasks plus a `p_predictions` test of earlier-chapter facts. Each task combines one earlier idea with this chapter's material: draw from chapter N−1, from N−2 or N−3, and from one chapter at least four back when one exists. Label each task with its chapters, for example `/// A. (Ch5 borrows + Ch9 structs)`. The header opens with the "do this WITHOUT reopening earlier chapters" instruction from Chapter 4. Chapters 1 to 3 have no review lesson.

### Code-review lesson (`c01_code_review.rs`)

- Ships *working-looking* code (no `todo!()`) written the way a hurried teammate might: two or three functions, two real bugs, plus avoidable clones, needless allocation, wrong ownership in signatures, `unwrap()` where errors should propagate, non-idiomatic loops, missed standard-library APIs, or concurrency or unsafe hazards that suit the chapter.
- Tests: behaviour tests that expose the bugs (some pass at the start, which is realistic) and technique tests that expose the smells.
- If a better signature is part of the fix, write the tests so that they compile with both the starting and the improved signature, and say in the doc comment that the type may change.
- Some clippy lints are deny-level. Seeded smells must stay at warning level so that `cargo clippy` still builds; use a manual implementation or a targeted `#[allow]` with a comment.

### Drills (`drills/<key>/dNN_name.rs`)

- Eight to eleven standalone programs per chapter (`fn main`, standard library only, edition 2024). Mix the outcomes: mostly compile errors with error codes, at least one that compiles (`ok`), and a run-time `panic` where the topic allows. Pick misconceptions people actually hit.
- Use the header of `drills/ch04/d01_use_after_move.rs`, with a one-line "Ask:" nudge and exactly one `EXPECT: ?` line. The example error code in the header (`e.g. E0308`) must not be the answer of any drill in that set; choose another code if it is.
- `main` asserts the intended behaviour. Lines that must survive the fix end with `// KEEP` (at least one per drill), so that the drill cannot be "fixed" by deleting the problem.
- Verify each outcome with `rustc --edition 2024 -A warnings`, and check that a reasonable fix compiles and runs. Keep the set of error codes small, ideally one.
- Drill sets live in `drills/<key>` with key `chNN` for chapters and `mN` for mastery modules.
- Add a `## Drills` section to the unit's HINTS.md: one collapsed `<details>` per drill with the outcome, why it happens, and the idiomatic fix.

### HINTS.md

For every lesson, two hints and then `<details><summary>Check your explanation (after writing your own)</summary>`: a model answer of three to six sentences to the lesson's EXPLAIN prompt. Then sections for R01 and C01 (two hints each; C01 also gets *Check your review*), and the Drills section. List the hint sections in the study order. Add the unit to the root [hint directory](HINTS.md).

### README.md

Each chapter README has a short introduction, "Before you start" with prerequisites and a time estimate, the lesson table in study order with R01, C01, the drills and the capstone, the run commands (`cargo test --test chapterNN`, `cargo clippy --test chapterNN`, `cargo drill chNN`), a coverage table that maps topics to lessons and drills, a "Mastery check" list of questions, and a note on where the solutions are. Checkpoint chapters (7, 11, 16, 21) end by pointing to their checkpoint.

### NOTES.md

Generated from the lesson headers and the README's "Mastery check" by `scripts/make_notes.py`:

- `python3 scripts/make_notes.py NN` writes a missing NOTES.md;
- `NN --force` regenerates one that holds no answers;
- `--all` must be given explicitly to touch every chapter;
- a workbook that holds answers is refused unless `--discard-my-notes` is given;
- every replaced file is first saved as `NOTES.md.bak` (then `.bak.2`, ...), which Git ignores;
- `--all --check` exits with 1 when a workbook is missing or stale; CI runs it.

The generator knows only `exercises/chapterNN`. The NOTES.md of mastery modules are written by hand in the same format; keep them in step with their lesson headers.

### Chapters 1 and 2 (the on-ramp)

Three hints per lesson, the last headed "Hint 3 (nearly the answer)". Lessons have a learner-owned `mod my_tests` above a `// ---- supplied tests` line, and supplied tests may count the learner's `#[test]` lines through `include_str!`. There is no R01 in chapters 1 and 2, and a C01 only in Chapter 2. Chapter 1 lesson 03 ships wrong code on purpose. Chapter 1 reads `CARGO_PKG_RUST_VERSION` and checks that `Cargo.lock` exists, so keep `rust-version = "1.99"` and the committed lock file.

### Labs with their own manifests

`exercises/chapter08/label_lab` (a checked workspace), `exercises/chapter16/random-lab`, `exercises/chapter17/regex-lab` and `exercises/chapter22/export_lab` are packages of their own. They are run with `cargo test --manifest-path <lab>/Cargo.toml` and are covered by the solution check like the standalone packages.

## solutions/

`solutions/` mirrors repository paths: `solutions/<path>` is the solved version of `<path>`. Examples: `solutions/exercises/chapter04/l01_scope.rs`, `solutions/drills/ch04/d01_use_after_move.rs`, `solutions/project/src/record.rs`, `solutions/mastery/m1_types/src/l01_variance.rs`. The check script copies the course, lays `solutions/` over the copy, and runs everything.

- A solution is the starter with only the learner-editable parts changed: bodies, `guess()` answers, the C01 code, the fields of `_replace_me` structs. Its `mod tests` block (and every other module compiled only for tests, or the whole file under a `tests/` directory) must equal the starter's, except for the prediction answers, a `//` comment on an answer's line, and dropping the unused `guess` import. If you change a starter's tests, update its solution.
- A package can list path prefixes whose tests may differ (`editable-tests`, see below).
- Solutions are model answers: idiomatic, clippy-clean, with a short comment where a choice is not obvious.
- Every drill set has `solutions/drills/<key>/expected.tsv`: one line per drill, `<file name><TAB><outcome>`, where the outcome is `ok`, `panic`, `error` (a compile error without a code) or the error codes joined by commas (`E0382` or `E0499,E0502`), exactly as `rustc --edition 2024 -A warnings` on the pinned toolchain produces them. Every solved drill must compile, exit with 0, and keep every `// KEEP` line.
- Capstones and checkpoints add a `DESIGN.md` (capstones) or commentary that explains the solution's decisions.

## The `practice` support library

The root package's library (`src/lib.rs`, crate name `practice`) offers `guess()`, `alloc::Counting` with `alloc::count(|| ...) -> (R, usize)`, and `within(secs, "what", move || ...) -> R`. Standalone packages carry their own `support.rs` with the same three tools, so that they do not depend on the root package.

## Mastery modules (`mastery/mN_name/`)

Each module is a standalone Cargo package with its own `Cargo.toml`, `Cargo.lock` and `rust-toolchain.toml` (a copy of the root one), and `README.md`, `CONCEPTS.md`, `HINTS.md` and a hand-written `NOTES.md`. Lessons are `src/lNN_*.rs` (procedural-macro lessons live in the macro crate), with `p_predictions`, technique tests, a C01 code review and a capstone lesson; `src/support.rs` holds the test tools. The README has "Who it is for", a prerequisites table that names the chapter for every skill, a time estimate, setup commands, the lesson table and a mastery check. Modules M1 to M8 have a drill set `drills/mN`.

A package can tune the solution check in its `Cargo.toml`:

```toml
[package.metadata.course]
test-args = ["--no-default-features"]
skip-os = ["windows"]          # linux, macos or windows
editable-tests = ["fixture/"]  # tests under these paths may differ from the starter
```

Module-specific conventions:

- **M2 (unsafe).** Design-from-contract lessons switch their tests with `gate! { off|on, "NN" => ... }` (predictions sit in a separate, ungated `predictions` module). Miri labs are examples `examples/miri_lNN_*.rs` with solved copies under `solutions/`. The counting allocator is disabled under `cfg(miri)`.
- **M3 (concurrency).** Code imports atomics and locks from `crate::sync`, which switches to loom's types under `cfg(loom)`. Each lesson has a `model` module under `#[cfg(all(test, loom))]`, run with the module's `cargo model lNN_` alias. Models with three threads use a preemption bound.
- **M5 (procedural macros).** trybuild cases live in `tests/ui/`; record `.stderr` files with `TRYBUILD=overwrite` on the pinned toolchain only, keep compiler-authored text in snapshots to a minimum, name learner-written cases `own_*`, and re-record every `.stderr` file when `rust-toolchain.toml` changes.

## Capstones (`capstones/<name>/`) and checkpoints (`checkpoints/cpN_after_chNN/`)

Both are standalone packages and blank-file projects: the learner designs the types and modules.

- A **capstone** has `README.md` (what you build, prerequisites table, time estimate per milestone, tools to install), `SPEC.md` (the contract), `CONCEPTS.md` (ideas outside Rust that the project needs, with compiled examples that are not the project), `HINTS.md` (design hints per milestone, no code), `RUBRIC.md` (pass, proficient, mastery), and acceptance tests `tests/mN_*.rs` per milestone with shared tools in `tests/common/`. The model solution and its `DESIGN.md` live in `solutions/capstones/<name>/`.
- The **final assessment** follows the capstone pattern without `CONCEPTS.md` and `HINTS.md`, on purpose.
- A **checkpoint** has `SPEC.md`, `RUBRIC.md` (which maps every test to the lessons behind it), a starter `src/lib.rs` with public signatures, `todo!()` bodies and `_replace_me` fields, `tests/behaviour.rs` (what), `tests/technique.rs` (how) and `tests/common/`. Tests run under time limits and a memory limit. There are no hints. Checkpoints follow chapters 7, 11, 16 and 21; [checkpoints/README.md](checkpoints/README.md) has the rules a learner follows.

## Validation (required before committing)

The scripts need Python 3.9 or newer. Use a private target directory when several people or agents build at once (`CARGO_TARGET_DIR=...`).

1. Formatting and lints: `rustfmt --edition 2024 --check` on the files you changed, and `cargo clippy` on the targets you changed. Warnings caused by `todo!()` bodies are acceptable in starters.
2. Starters compile: `cargo test --locked --offline --all-targets --no-run`, in debug and `--release`; for a standalone package, the same with `--manifest-path`.
3. Starter state: `python3 scripts/check_starters.py --strict --run` (structure, forbidden phrases, every prediction fails at `guess()`, nothing crashes or hangs).
4. Solutions: `python3 scripts/check_solutions.py --only chNN` (or `--only m1`, `--only capstones/kvstore`, ...); add `--clippy --repeat 3` before you commit. Confirm by hand that at least one plausible wrong technique fails each new technique test.
5. Concepts: `python3 scripts/check_concepts.py chNN` runs every std-only `CONCEPTS.md`; check the error codes of `compile_fail` blocks with `rustc`.
6. Links: `python3 scripts/check_links.py`.
7. Notes: `python3 scripts/make_notes.py --all --check`.
8. Drills: `cargo drill chNN` works; predict one drill, fix it, see the check mark, then delete `.progress/` and restore the drill.

The "Course checks" workflow in [`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs these checks on Linux, macOS and Windows, plus Miri on the solved unsafe code and loom on the solved concurrency module.
