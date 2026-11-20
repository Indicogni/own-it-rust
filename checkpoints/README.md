# Checkpoints

[All chapters](../README.md)

A checkpoint is a test of what you can do **without help**. The lessons of
a chapter give you a file with signatures, contracts and hints. A
checkpoint gives you a specification and an almost empty file. You decide
which types hold the data, which helper functions exist and in which order
to build things. Tests then tell you whether the result is correct, and a
rubric tells you what to review if it is not.

There are four checkpoints. Each one covers everything up to its point in
the course, with the weight on the chapters since the last checkpoint.

| checkpoint | take it after (prerequisites) | main topics | what you build |
|---|---|---|---|
| [`cp1_after_ch07`](cp1_after_ch07/SPEC.md) | chapters [1](../exercises/chapter01/README.md) to [7](../exercises/chapter07/README.md) | types, ownership, borrowing, expressions, errors | `splitpot`: a ledger of shared expenses that reads text, keeps balances and plans who pays whom |
| [`cp2_after_ch11`](cp2_after_ch11/SPEC.md) | chapters [1](../exercises/chapter01/README.md) to [11](../exercises/chapter11/README.md) | modules, structs, enums and patterns, traits and generics | `postage`: validated parcels and carriers that price them |
| [`cp3_after_ch16`](cp3_after_ch16/SPEC.md) | chapters [1](../exercises/chapter01/README.md) to [16](../exercises/chapter16/README.md) | operators, utility traits, closures, iterators, collections | `matchday`: a league table with rankings, form and fixtures |
| [`cp4_after_ch21`](cp4_after_ch21/SPEC.md) | chapters [1](../exercises/chapter01/README.md) to [21](../exercises/chapter21/README.md) | strings, input and output, threads, async basics | `seal`: a file integrity checker with a parallel check and its own small executor |

Each checkpoint is a small Cargo package of its own. It uses only the
standard library and the same pinned toolchain as the rest of the course.

## When to take one

Take a checkpoint when all of this is true:

- You have finished every lesson, the review file and the capstone of the
  chapters it covers, and `cargo progress` shows them as complete.
- You have done the drills of those chapters at least once.
- At least **two days** have passed since you finished the last of those
  chapters. A checkpoint measures what stayed, not what you read this
  morning.
- You have a block of four hours without interruptions.

If you are not sure, take it anyway. A checkpoint that you do not pass
costs you an afternoon and tells you exactly what to repeat. Skipping it
tells you nothing.

## Rules

These rules are what makes the result mean something. Nobody checks them
but you.

1. **One sitting, at most 4 hours.** Start a timer when you open
   `SPEC.md`. Reading the specification is part of the time. Short breaks
   are fine, and the timer keeps running. When the timer shows 4 hours,
   stop typing, even in the middle of a line. A well-prepared learner needs
   2 to 3 hours.
2. **Closed notes.** While the timer runs you do not open:
   - any `CONCEPTS.md`, `HINTS.md`, `NOTES.md` or lesson file of the
     course, and none of your own earlier solutions;
   - anything under `solutions/`;
   - web searches, forums, question-and-answer sites, books;
   - an AI assistant or AI code completion of any kind.
3. **What you may use:**
   - `SPEC.md` and the files of the checkpoint itself;
   - the compiler, its error messages and `rustc --explain E....`;
   - `cargo test`, `cargo clippy`, and tests that you write yourself;
   - the API documentation of the standard library, offline with
     `rustup doc --std` or at <https://doc.rust-lang.org/std/>. Looking up
     the exact name or signature of a method is not what a checkpoint
     tests. Stay inside the `std` documentation: the Rust Reference and
     other guides are closed, like the course text;
   - your editor with its normal help: go to definition, type hints,
     completion of names.
4. **Do not change the tests.** The files under `tests/` and `Cargo.toml`
   stay as they are. Do not change a public name, a public field or a
   signature in `src/lib.rs`. Everything else in `src/` is yours: fields,
   private items, new files, derives.
5. **No hints exist.** If the specification seems unclear, read the
   sentence again, then look at the assertion of the failing test: test
   messages say what was expected. That is all the help there is.

## How to take one

1. Save your work in the course repository first (for example, commit it),
   so that you can get the starter back later.
2. Go into the directory of the checkpoint. All commands below run there.
   The course commands `cargo drill` and `cargo progress` belong to the
   repository root and do not work inside a checkpoint.

   ```text
   cd checkpoints/cp1_after_ch07
   cargo test --no-run          # builds the starter; nothing is solved yet
   ```

3. Start the timer and read `SPEC.md` from the first line to the last.
   Plan 15 to 20 minutes for it. Read the section about cost with special
   care: it tells you which operations must be cheap, and that decides
   your data structures.
4. Open `src/lib.rs`. Every `todo!()` is yours to replace, and every
   struct with a `_replace_me` field is yours to design.
5. Work in small steps, and run the tests often:

   ```text
   cargo test --test behaviour             # is it correct?
   cargo test --test behaviour amount_     # only the tests whose name starts with amount_
   cargo test --test technique             # is it correct for the right reasons?
   cargo test --release                    # everything again, optimized
   cargo clippy --all-targets
   ```

   Make the behaviour tests pass first. Then turn to the technique tests.
   Write tests of your own for your helper functions as you go; the rubric
   asks for them, and they save time.
6. When you are done, or when the timer shows 4 hours, stop.
7. Open `RUBRIC.md` and go through it line by line. It ends with what to
   do next.
8. Only now read the reference solution and its commentary:
   `solutions/checkpoints/<name>/` in the course repository. From the
   repository root, `cargo progress solution cp1` lists its files.

A possible plan for the time:

| minutes | what |
|---|---|
| 0 to 20 | read `SPEC.md`; note the data structures you intend to use |
| 20 to 150 | implement section by section; behaviour tests after each one |
| 150 to 200 | technique tests, `--release`, clippy |
| 200 to 240 | reserve; your own tests; comments for the rubric |

## What the files are

| file | purpose |
|---|---|
| `SPEC.md` | the contract: what every public item must do, and what it may cost |
| `src/lib.rs` | the starter: public names and signatures with `todo!()` bodies, so that the tests compile |
| `tests/behaviour.rs` | acceptance tests for *what* the code does. All of them must pass for the level "pass" |
| `tests/technique.rs` | tests for *how* the code does it. Together with the checks in the rubric they decide the level "proficient" |
| `tests/common/mod.rs` | tools of the tests; you do not need to read it |
| `RUBRIC.md` | how to grade yourself, and which lesson to review for each test that failed |

## How the tests look at technique

A wrong idea often gives right answers. The technique tests therefore
measure a few things besides the results.

- **Allocations.** The tests install a counting allocator. A test can ask
  how many times, and for how many bytes, a call requested heap memory. So
  "must not allocate" and "at most 1024 bytes" in a specification are
  checked literally. Only the thread that runs the measured call is
  counted.
- **Identity.** Some tests compare addresses: is this `&str` the caller's
  own string, or a copy with the same text?
- **Call counts.** Some tests hand your code a closure or a reader that
  counts how often it is used.
- **Time limits.** Anything that could wait for ever runs with a limit. If
  your code hangs, the test fails with "was still running after N s" and
  the test run goes on. The thread that hangs stays behind until the test
  program ends; that is expected.
- **A memory limit.** If the test program holds more than 1 GiB at once,
  it prints "MEMORY LIMIT" and stops. That happens when a loop never ends
  and stores something on each round.
- **Both profiles.** Integer overflow panics in a debug build and wraps
  around in a release build, and the optimizer can hide or reveal a race.
  The rubric therefore asks for `cargo test` and `cargo test --release`.

In checkpoint 4, where threads are involved, run each command three times.
A test that passes two times out of three has found a bug.

## The two levels

- **Pass:** all behaviour tests pass in both profiles, inside the rules.
  You can build a correct program of this size from a specification.
- **Proficient:** the technique tests pass as well, the compiler and
  clippy are silent, and the reading checks of the rubric hold. Your
  program is correct for reasons that you chose on purpose.

Not passing is normal on a first attempt. The rubric maps every test to
the lessons behind it. Review those, wait at least a week, and take the
checkpoint again from the starter. To get the starter back, restore the
package from version control, for example:

```text
git stash push -u -- checkpoints/cp1_after_ch07    # keeps your attempt in the stash
```

Keep your attempts. Reading your first attempt after you passed is one of
the more useful things you can do with it.

## After a checkpoint

Reaching "proficient" in all four checkpoints means that you can design
and build small Rust programs by yourself, with ownership, traits,
collections, I/O, threads and futures. That is the entry ticket for the
mastery modules under `mastery/` and for the large blank-file projects
under `capstones/`.
