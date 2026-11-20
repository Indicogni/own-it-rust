# Final assessment: rgx, a regular-expression engine and search tool

[All chapters](../../README.md) · [Specification](SPEC.md) · [Rubric](RUBRIC.md)

This is the last project of the course. You write a regular-expression
engine from nothing: a parser for a small pattern language, a compiler from
patterns to a finite automaton, and a matcher whose running time is
proportional to the length of the text for every pattern, with no
exceptions. On top of the engine you write `rgx`, a command-line tool in
the style of `grep` that searches files and directory trees on several
threads and still prints the same bytes on every run.

It is an **assessment**. The other capstones come with a concepts text and
design hints. This one has neither. You get a precise specification and
106 acceptance tests, and every decision between the two is yours: the
modules, the types, the algorithm, the data structures, the way threads
share work. At the end you grade yourself against a rubric and defend your
design in writing.

| file | what it is |
|---|---|
| [SPEC.md](SPEC.md) | the complete specification: read all of it first |
| [RUBRIC.md](RUBRIC.md) | how to grade the result: pass, proficient, mastery; and the questions you answer to defend your design |
| `src/lib.rs` | the starter of the library: the five functions the tests call, each a `todo!()` |
| `src/main.rs` | the starter of the program: it prints one line and exits |
| `tests/` | the acceptance tests, one file per milestone |

There is no `HINTS.md` and no `CONCEPTS.md`, on purpose.

A reference solution, with a `DESIGN.md` that explains its decisions, is
in `solutions/capstones/final_assessment/` of the course repository. It is
there to compare with **after** you have graded your own work. Reading it
first turns the assessment into a reading exercise.

## Prerequisites

You should have finished chapters [1](../../exercises/chapter01/README.md) to [21](../../exercises/chapter21/README.md) and [23](../../exercises/chapter23/README.md), the four [checkpoints](../../checkpoints/README.md) (checkpoint 4 needs chapters 20 and 21),
and the mastery modules listed below. The table says where each skill that
this project needs was taught.

| from | you use it for |
|---|---|
| [Chapter 3](../../exercises/chapter03/README.md), Numbers, text and arrays | `char` against `u8`, integer widths, arithmetic that must not overflow (the cost of a pattern can exceed `u64`) |
| Chapters [4](../../exercises/chapter04/README.md) and [5](../../exercises/chapter05/README.md), Ownership and moves; Borrowing with references | who owns the pattern, the automaton and the scratch memory; lines that borrow from the text and outlive the `Regex` |
| [Chapter 7](../../exercises/chapter07/README.md), Handling errors | an error type of your own with a position; turning errors into messages and exit statuses in one place |
| [Chapter 8](../../exercises/chapter08/README.md), Modules, crates and Cargo | a library and a binary in one package; a public surface that is smaller than the crate |
| Chapters [9](../../exercises/chapter09/README.md) and [10](../../exercises/chapter10/README.md), Structs and methods; Enums and pattern matching | the syntax tree, the instructions or states of the automaton, exhaustive `match` |
| Chapters [11](../../exercises/chapter11/README.md) and [13](../../exercises/chapter13/README.md), Generic code with traits; Everyday traits | `Display`, `Debug`, `Error`, `Iterator`, `Send` and `Sync` on your own types |
| Chapters [14](../../exercises/chapter14/README.md) and [15](../../exercises/chapter15/README.md), Closures and captures; Iterators and adapters | `find_iter` and `matching_lines` as real, lazy iterators |
| [Chapter 16](../../exercises/chapter16/README.md), Standard collections | choosing the structure for sets of states and for results that wait their turn |
| [Chapter 17](../../exercises/chapter17/README.md), Working with text | UTF-8: character boundaries, byte offsets, decoding, why a class cannot be a table of bytes |
| [Chapter 18](../../exercises/chapter18/README.md), Files, streams and processes | reading files and directories, buffered output, standard streams, a closed pipe |
| [Chapter 19](../../exercises/chapter19/README.md), Threads and shared state | scoped threads, channels, `Mutex` and `Condvar`, sharing one immutable value among workers |
| [Chapter 23](../../exercises/chapter23/README.md), Unsafe Rust | only to decide that you do not need it, or to justify it and run Miri if you do |
| Mastery [M1](../../mastery/m1_types/README.md), the type system in depth | lifetimes in signatures: an iterator that borrows from two things and yields items that borrow from one |
| Mastery [M3](../../mastery/m3_concurrency/README.md), concurrency | reasoning about which thread may block where; ordered results from unordered work |
| Mastery [M7](../../mastery/m7_perf/README.md), performance engineering | allocation budgets, counting instead of timing, measuring before changing |
| Mastery [M8](../../mastery/m8_api/README.md), library design | a small facade that is hard to misuse; what to make public |

Helpful but not required: mastery [M2](../../mastery/m2_unsafe/README.md) (Miri), [M9](../../mastery/m9_toolbox/README.md) (lints, fuzzing and
property tests for your own test suite), and the capstone [`interpreter`](../interpreter/README.md),
which is a gentler first parser.

You do **not** need to know automata theory beforehand, but you will have
to learn some. Researching how linear-time matching is done is part of the
assessment; copying an existing engine is not (see "Rules" below).

Time: as a rough estimate, plan 35 to 60 hours: M1 5 to 8, M2 6 to 10, M3
8 to 14, M4 4 to 7, M5 5 to 8, M6 5 to 9, and 2 to 4 for the rubric and
the written defence. These are estimates, not targets.

## Rules

- **The standard library only.** No dependencies (SPEC section 9).
- **Your own design and your own code.** You may read papers, articles and
  documentation about regular-expression matching, and you should. You may
  not copy code from an existing engine, and you may not open the
  reference solution before you have graded yourself.
- **Do not change the tests.** If you think a test contradicts the SPEC,
  the SPEC wins; write the contradiction down and go on.
- Take as long as you need. There is no clock.

## Setting up

The package stands alone. Work inside `capstones/final_assessment/`; it
has its own `Cargo.toml`, `Cargo.lock` and `rust-toolchain.toml` (Rust
1.99.0, the same as the rest of the course). There is nothing to install.

```text
cargo build
cargo test --test m1_syntax
```

Every test fails at first. The library tests stop at a `todo!()` that
names a SPEC section; the program tests show the line that the starter
prints. That is the starting line.

## How to run

The tests, one milestone at a time (SPEC section 10 says what each file
covers):

```text
cargo test --test m1_syntax
cargo test --test m2_matching
cargo test --test m3_linear_time
cargo test --test m4_lines_alloc
cargo test --test m5_cli
cargo test --test m6_parallel
```

Everything, in both profiles:

```text
cargo test
cargo test --release
```

Your program by hand:

```text
cargo run -- -n 'fn \w+\(' src
cargo run --release -- -c -i todo .
cargo run -- --help
```

The measurement that the rubric uses (it prints numbers and cannot fail
because of them):

```text
cargo test --release --test m6_parallel -- --ignored --nocapture speed_report
```

What to expect from the tests:

- **A test never hangs.** Every library test and every run of your binary
  has a limit of 30 seconds. A search that does not end fails one test
  with a message that starts with `TIMEOUT`. The reference solution needs
  less than two seconds for each test in a debug build, so a timeout
  always means that something is wrong, not that your machine is slow.
- **A test never fills your memory.** The two test files that work with
  very large patterns stop the test program when it holds more than one
  gibibyte.
- **Nothing is left behind.** Temporary directories are removed and
  started programs are ended, also when a test fails. The one exception:
  if you interrupt a test run (Ctrl-C), or the test program itself is
  killed, it cannot tidy up. Then delete the directories named
  `rgx-test-*` in the temporary directory of your system, and end any
  `rgx` process that is still running.
- **A failure says what was expected.** Program tests print the command,
  the expected and the actual output, the first line that differs, and
  what your program wrote on standard error. Most messages name the SPEC
  section that they check.
- If a whole test program ends with `stack overflow` or with a signal, one
  of your functions recursed too deeply (SPEC 4.5). Run the tests of that
  file one by one to find which: `cargo test --test m3_linear_time -- --test-threads=1`.

## How to work

1. Read SPEC.md from start to end once. It is long because it is complete.
2. **Design before you type.** Decide what the stages of the engine are,
   what each one takes and gives, and what the one data structure is that
   the matcher works on. Write half a page. You will change it; the half
   page gives you something to change.
3. Work through the milestones in order. M2 can be passed by a matcher
   that is correct and slow, and M3 cannot. Plan for that: either build the
   final matcher at once, or build a simple one first, keep it, and use it
   later to test the fast one.
4. Write your own tests as you go. The acceptance tests see only the
   facade and the command line. A parser, a set of character ranges and a
   line splitter are much easier to get right with ten small tests next to
   each.
5. Keep `cargo clippy --all-targets` clean from the first day.

## How to assess your work

Do these in order. The first three are quick and decide whether the rest
is worth doing yet.

1. **The gate.** `cargo test` and `cargo test --release` pass, three times
   in a row each. `cargo clippy --all-targets` prints no warnings.
   `[dependencies]` is empty.
2. **Unsafe.** `grep -rn "unsafe" src/` finds at most a
   `#![forbid(unsafe_code)]` line. If it finds more, the library tests
   must pass under Miri, which checks them with smaller inputs:

   ```text
   cargo +nightly-2026-11-20 miri test --lib --test m1_syntax --test m2_matching --test m4_lines_alloc
   ```

   (This needs the course's dated nightly toolchain, `nightly-2026-11-20`,
   with the `miri` component; see the course README. Miri is
   slow: plan ten minutes or more.)
3. **By hand.** Run your program on a large directory tree, for example
   the course repository, with `-j 1` and `-j 8`, and compare the two
   outputs with a diff tool. Pipe a large output into a program that stops
   reading early. Give it a file that is not text, a directory you may not
   read, and a pattern of your own invention that you expect to be slow.
4. **Measure.** Run `speed_report` and write the numbers down.
5. **Grade.** Go through [RUBRIC.md](RUBRIC.md) row by row. Each row has
   three levels with a check you can carry out. Be strict: the point is to
   find what to improve.
6. **Defend.** Answer the questions at the end of the rubric in writing,
   in a `DESIGN.md` of your own, without looking at any other solution.
7. **Compare.** Only now read the reference solution and its `DESIGN.md`.
   Different is fine. Look for decisions that you made without noticing
   that there was a decision to make, and for claims in your defence that
   the comparison shows to be wrong.

## Licence

The code, the specification, the pattern language and the tests in this
package are original work for this course and are covered by the MIT
licence of the course repository.
