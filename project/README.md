# Running project: `loglens`

[Course overview](../README.md) · [Hints](HINTS.md) · [Solutions](../solutions/project/)

The chapter lessons practice one topic at a time. This project is the other half of the course: **one realistic program that you grow from Chapter 3 to Chapter 24**. Each milestone adds a feature with the chapter you have just finished. Several milestones also *rewrite* earlier code with a better technique, while the earlier tests stay green. At the end you have a fast, tested command-line log analyzer, and you have practiced the work that fills most engineering days: changing code that already works without breaking it.

`loglens` reads structured log lines like this one:

```text
2026-10-23T17:03:00Z INFO auth user=ada latency_ms=42 msg="login ok"
```

It filters, counts, and summarizes them, across many files, in parallel, or while a service is still writing them:

```console
$ loglens grep --where 'level>=warn' fixtures/app.log fixtures/web.log
loglens: fixtures/app.log: line 9, column 1: invalid timestamp
2026-10-23T17:03:02Z WARN auth user=bob latency_ms=310 msg="slow password check"
2026-10-23T17:03:05Z ERROR auth user=bob latency_ms=97 msg="login failed: \"bad password\""
2026-10-23T17:03:21Z ERROR db latency_ms=5012 msg="connection reset"
2026-10-23T17:03:03Z WARN http method=POST path=/login status=401 latency_ms=330 user=bob
2026-10-23T17:03:07Z ERROR http method=GET path=/report status=500 latency_ms=5020 msg="upstream timeout"

$ loglens report --by user --top 3 --jobs 4 fixtures/app.log fixtures/web.log fixtures/worker.log
loglens: fixtures/app.log: line 9, column 1: invalid timestamp
loglens: fixtures/worker.log: line 4, column 77: unterminated quoted value
records: 19
levels: TRACE=1 DEBUG=1 INFO=11 WARN=3 ERROR=3
top 3 by user:
  -   count=10 errors=2 mean_latency_ms=3864.7 max_latency_ms=13000
  ada count=4 errors=0 mean_latency_ms=37.0 max_latency_ms=58
  bob count=3 errors=1 mean_latency_ms=245.7 max_latency_ms=330

$ loglens count -w 'target=http latency_ms>100' fixtures/web.log
3

$ loglens grep --follow -w 'level>=error' /var/log/myservice.log   # like tail -f
```

## What you are building, and why it is designed this way

You do not design `loglens` from a blank file. Every public type and function signature is given, with its contract, and you write the bodies. That is a deliberate choice for a project that runs beside the chapters: each milestone can then be tested the day you finish its chapter. The price is that the design decisions were made for you. So read them here first, and question them while you work. Each one is a decision you will make yourself in your own programs.

```text
bytes ───► lines ───► Record<'a> ───► Filter ───► Sink
file,      one        borrows         boxed       print, count,
stdin,     reused     from the        closures    or summarize
--follow   buffer     line
```

| Decision | Why | What it costs |
| --- | --- | --- |
| **A library with a thin binary.** `src/main.rs` only connects the real process to `cli::main_with`. | Everything can be tested without starting a process. Other programs can use the crate. | One more layer to read. |
| **The outside world is a parameter.** `cli` receives stdin, stdout, stderr, and the function that opens a file. | Tests run the whole program on in-memory buffers, give it a file that fails half-way, and see which thread reads which file. | Signatures are longer than `println!` and `File::open` would need. |
| **Records borrow, they do not copy.** A `Record<'a>` holds `&'a str` slices of its line. | Parsing a line allocates almost nothing, so a large file is cheap to scan. | Lifetimes appear in signatures, and a record cannot outlive the buffer its line is in. Code that keeps data longer (the report) copies exactly what it keeps. |
| **A malformed line is a value, not a crash.** It becomes a `ParseError` with a line and a column, is reported, and is skipped. | Real logs contain damaged lines. One of them must not hide the other million. | Every caller has to decide what to do with errors. |
| **A filter is compiled once.** `level>=warn user!=bob` becomes a list of boxed closures. | Mistakes in the filter are reported before any file is read, and matching a record does no parsing. | One indirect call per term and record. |
| **One trait for every output.** A `Sink` receives matching records: a list of lines, a counter, a writer, a `Summary`. | `process` is written once and serves `grep`, `count`, and `report`. | A trait to learn before the first output appears. |
| **Input is streamed.** One line buffer is reused for the whole input. | Memory use does not grow with the size of the file. | A record must be used before the next line is read. |
| **Parallel results are the sequential results.** Files are processed in any order, and merged in input order. | You can turn on `--jobs` without checking the output again. | Output is buffered per file instead of printed at once. |
| **The standard library only.** No argument parser, no async runtime, no error crate. | You see what those crates do for you. The [ecosystem track](../ecosystem/README.md) then shows the crates that professionals use instead. | More code than a real project would write by hand. |
| **Safe code first.** The only `unsafe` is in the last, optional milestone, after a safe version exists and both are measured. | `unsafe` moves a proof from the compiler to you. It should buy something. | Nothing. |

Designing from a blank file is a different skill, and this project does not practice it. The blank-file projects of the course are in [`capstones/`](../capstones/). Until then, each milestone below ends with a **design question** about a signature you were given.

## How the project works

`project/` is a **standalone Cargo package**. It has its own `Cargo.toml`, `Cargo.lock`, and toolchain file, it is not part of the repository's root package, and it uses only the standard library. Root commands such as `cargo test --test chapter04` never build it. Run its commands from inside `project/`, as all examples in this guide do, or from the repository root with `--manifest-path project/Cargo.toml`.

```sh
cd project
cargo test --test m03_types              # one milestone
cargo test --test m03_types p_           # only its predictions
cargo test --test m03_types --test m04_borrowing --test m07_errors   # everything done so far
cargo test --doc                         # the documentation examples (from m08 on)
cargo run -- report fixtures/app.log     # the program (from m18 on)
cargo test --all-targets --no-run        # compile everything without running it
```

`cargo progress` and `cargo drill` belong to the root package, so run them from the repository root, not from `project/`. `cargo progress all` also shows one row for this project.

```text
project/
├── Cargo.toml, Cargo.lock, rust-toolchain.toml
├── README.md          this guide
├── HINTS.md           two progressive hints per milestone
├── fixtures/          sample logs used by the tests and by you
├── src/
│   ├── lib.rs         module tree, re-exports, prelude
│   ├── main.rs        the loglens binary (m18)
│   └── num.rs, scan.rs, error.rs, level.rs, record.rs, sink.rs, filter.rs,
│       pipeline.rs, report.rs, text.rs, follow.rs, cli.rs, parallel.rs,
│       aio.rs, macros.rs, fast.rs
└── tests/
    ├── common/mod.rs  shared helpers: guess(), allocation counting, time limits
    └── mNN_name.rs    one test file per milestone (m18 has two)
```

**The contract is the doc comment.** Every public item has a doc comment that says what to return, which errors, which edge cases, and sometimes which technique to use. You replace `todo!()` bodies. Keep the public signatures, because all test files compile against them, and that is what lets you run milestones one at a time. Add private helpers, modules, and tests of your own freely. The starter compiles, and its tests fail at `todo!()` and `guess()`. That is the starting state, not a broken setup. Until you finish, one attribute in `src/lib.rs` silences the warnings about unused parameters, imports, and private fields that the `todo!()` bodies cause; delete that attribute at the end and clean up what the compiler reports.

**Milestones build on each other.** m09's `parse_record` calls m04's `split_word`; m18's program calls almost everything. Do them in table order. m20 (async) and m23 (unsafe) are optional; nothing later depends on them.

**Each test file has three kinds of tests:**

- *Behavior tests* check the contract, including edge cases (empty input, Unicode, overflow, ties).
- *Technique tests* check how you solved it, where the contract asks for a technique. They count heap allocations and allocated bytes (`common::count`, `common::count_bytes`), check that slices point into the original line (`common::borrows_from`), compare thread ids, or count polls. A solution that copies everything, collects where it should stream, or runs on one thread produces the right output and still fails them.
- *`p_predictions`*: before you run anything, replace each `guess()` with your prediction. A wrong prediction is the most useful feedback you can get. Write down why you were wrong.

The `m18_cli` and `m19_parallel` tests also run the **built binary** (`env!("CARGO_BIN_EXE_loglens")`) on the files in `fixtures/` and check stdout, stderr, and the exit status.

**No test hangs.** Code that can block forever when it is wrong (a channel whose sender is never dropped, an executor that is never woken, a `Follower::poll` that waits for more input, a program that never exits) runs under a time limit. After at most 30 seconds the test fails with a message that names the likely causes. A child process that a test started is killed when the test ends, also when the test fails. If the program panics, for example at a `todo!()` that is still there, the test shows what the program wrote to stderr.

### The loop for each milestone

1. **Study** the course chapter that the milestone follows: its `CONCEPTS.md` and its lessons. The milestone table links them.
2. **Predict.** Open `tests/mNN_*.rs`, read the header, and answer `p_predictions`.
3. **Implement** the tasks in the files listed below. Run the milestone's tests until they pass, in both profiles: `cargo test --test mNN_name` and `cargo test --release --test mNN_name`.
4. **Refactor** where the milestone says so, then run every earlier milestone again. Run `cargo clippy --all-targets` and fix the warnings in your code.
5. **Explain.** Answer the header's EXPLAIN prompt and the milestone's design question in your own notes, and add one edge-case test. Open [HINTS.md](HINTS.md) only if you are stuck, and the [solution](#solutions) only when your own version is green.

### Refactoring with a safety net

At work you rarely write code once. You change it: for speed, for a new requirement, or because you learned a better tool. Tests are what make that safe. Several milestones below have a **Refactor** step that rewrites code you already finished. The rule is always the same:

1. Run the earlier milestones' tests and see them pass *before* you touch anything.
2. Change one thing.
3. Run the same tests again. If they pass, the behavior is unchanged. If not, the failure tells you what you broke. Undo or fix before you go on.

The equivalence tests are deliberate: `split_field` must agree with `scan_field` (m07), `process_reader` with `process` (m18), the parallel results with the sequential ones (m19), `process_async` with `process` (m20), and both versions of `find_byte` with a plain search (m23).

### Solutions

A complete model solution is in [`solutions/project/`](../solutions/project/). The directory mirrors this one: `solutions/project/src/scan.rs` is the solved `project/src/scan.rs`, and `solutions/project/tests/m04_borrowing.rs` is the same test file with the predictions answered. Only the parts that you edit differ (function bodies, private helpers, and `guess()` answers), so a plain `diff` shows exactly what the solution added:

```sh
diff src/scan.rs ../solutions/project/src/scan.rs
```

To run the solved program, copy `project/` to a directory outside the repository, copy the contents of `solutions/project/` over the copy, and run `cargo test --tests` there.

Use the solution the way you would use a colleague's review. Finish the milestone first. Then compare: where the solution is shorter, find out which standard-library function made it shorter; where yours is shorter or clearer, keep yours. The solution is one good answer. It is not the only one, and the tests accept every program that keeps the contracts.

## The log format

```text
TIMESTAMP LEVEL TARGET [KEY=VALUE]...
2026-10-23T17:03:00Z INFO auth user=ada latency_ms=42 msg="login ok"
```

- `TIMESTAMP` is exactly `YYYY-MM-DDTHH:MM:SSZ` (UTC). In this format, string order is time order.
- `LEVEL` is `TRACE`, `DEBUG`, `INFO`, `WARN` (or `WARNING`), or `ERROR`, in any case.
- `TARGET` is the component that wrote the line.
- A value is bare (`user=ada`) or in double quotes (`msg="login ok"`), where `\"`, `\\`, `\n`, `\r`, and `\t` are escapes.
- Words and fields are separated by ASCII whitespace. Other Unicode spaces, such as a no-break space, are ordinary characters.
- Blank lines and lines whose first character after leading whitespace is `#` are ignored. "Blank" and "whitespace" mean ASCII whitespace here too, so a line that holds only a no-break space is not blank. Any other line that does not parse is *malformed*: it is reported with its line and column and skipped. It never stops processing.

**Filters** (m14) are terms separated by spaces, all of which must match: `level>=warn`, `target=auth`, `target~au` (contains), `user=ada`, `user!=bob`, `msg~timeout`, `latency_ms>100`, `ts>=2026-10-23T17:00:00Z`. The full grammar is in [`src/filter.rs`](src/filter.rs).

**The program** (m18) has three commands, `grep`, `count`, and `report`, and the options in `loglens --help` ([`src/cli.rs`](src/cli.rs)). Matching output goes to stdout; warnings and errors go to stderr. The exit status is 0 if a record matched, 1 if none did, and 2 on an error (bad arguments, a file that cannot be opened or read, an invalid filter, or a malformed line under `--strict`), like `grep`.

## Milestones

There are 15 milestones and 16 test files (m18 has two). **Follows** names the course chapters to finish first; each link opens that chapter's teaching text. The times are estimates for a learner who has just finished those chapters. They include the predictions, the refactor step, and your notes, and they will vary a lot from person to person.

| Done | Milestone | Follows | You write | Run | Estimate |
| --- | --- | --- | --- | --- | --- |
| [ ] | [m03 · Numbers with an overflow policy](#m03--numbers-with-an-overflow-policy) | [Ch 3](../exercises/chapter03/CONCEPTS.md) | `num.rs` | `cargo test --test m03_types` | 1–2 h |
| [ ] | [m04 · Zero-copy fields](#m04--zero-copy-fields) | [Ch 4](../exercises/chapter04/CONCEPTS.md), [5](../exercises/chapter05/CONCEPTS.md) | `scan.rs` | `cargo test --test m04_borrowing` | 2–3 h |
| [ ] | [m07 · Errors with positions](#m07--errors-with-positions) | [Ch 6](../exercises/chapter06/CONCEPTS.md), [7](../exercises/chapter07/CONCEPTS.md) | `scan.rs`, `error.rs` | `cargo test --test m07_errors` | 2–3 h |
| [ ] | [m08 · Modules, a public API, doc tests](#m08--modules-a-public-api-doc-tests) | [Ch 8](../exercises/chapter08/CONCEPTS.md) | `error.rs`, doc comments | `cargo test --test m08_modules` | 1–2 h |
| [ ] | [m09 · The Level enum and the Record struct](#m09--the-level-enum-and-the-record-struct) | [Ch 9](../exercises/chapter09/CONCEPTS.md), [10](../exercises/chapter10/CONCEPTS.md) | `level.rs`, `record.rs` | `cargo test --test m09_records` | 2–3 h |
| [ ] | [m11 · Traits: Sink, Display, FromStr, TryFrom](#m11--traits-sink-display-fromstr-tryfrom) | [Ch 11](../exercises/chapter11/CONCEPTS.md), [12](../exercises/chapter12/CONCEPTS.md), [13](../exercises/chapter13/CONCEPTS.md) | `level.rs`, `record.rs`, `sink.rs` | `cargo test --test m11_traits` | 2–3 h |
| [ ] | [m14 · A filter language compiled to closures](#m14--a-filter-language-compiled-to-closures) | [Ch 14](../exercises/chapter14/CONCEPTS.md) | `filter.rs` | `cargo test --test m14_filters` | 3–4 h |
| [ ] | [m15 · Iterator pipelines](#m15--iterator-pipelines) | [Ch 15](../exercises/chapter15/CONCEPTS.md) | `pipeline.rs` | `cargo test --test m15_pipelines` | 2–3 h |
| [ ] | [m16 · Aggregation and top-k](#m16--aggregation-and-top-k) | [Ch 12](../exercises/chapter12/CONCEPTS.md), [16](../exercises/chapter16/CONCEPTS.md) | `report.rs` | `cargo test --test m16_reports` | 3–4 h |
| [ ] | [m17 · Quoted values and Unicode](#m17--quoted-values-and-unicode) | [Ch 17](../exercises/chapter17/CONCEPTS.md) | `text.rs`, `scan.rs`, `record.rs`, `filter.rs` | `cargo test --test m17_text` | 2–3 h |
| [ ] | [m18 · Streaming input and the CLI](#m18--streaming-input-and-the-cli) | [Ch 18](../exercises/chapter18/CONCEPTS.md) | `pipeline.rs`, `follow.rs`, `cli.rs`, `main.rs` | `cargo test --test m18_io --test m18_cli` | 5–7 h |
| [ ] | [m19 · Parallel files, identical results](#m19--parallel-files-identical-results) | [Ch 19](../exercises/chapter19/CONCEPTS.md) | `parallel.rs`, `cli.rs` | `cargo test --test m19_parallel` | 3–4 h |
| [ ] | [m20 · Optional: a std-only async reader](#m20--optional-a-std-only-async-reader) | [Ch 20](../exercises/chapter20/CONCEPTS.md), [21](../exercises/chapter21/CONCEPTS.md) | `aio.rs` | `cargo test --test m20_async` | 3–4 h |
| [ ] | [m22 · Macros: field! and log_line!](#m22--macros-field-and-log_line) | [Ch 22](../exercises/chapter22/CONCEPTS.md) | `macros.rs` | `cargo test --test m22_macros` | 2–3 h |
| [ ] | [m23 · Optional: a fast byte search, and when unsafe is worth it](#m23--optional-a-fast-byte-search-and-when-unsafe-is-worth-it) | [Ch 23](../exercises/chapter23/CONCEPTS.md), [24](../exercises/chapter24/CONCEPTS.md) | `fast.rs` | `cargo test --test m23_unsafe` | 2–4 h |

Together that is roughly 35 to 50 hours. Each section names the tasks by the letters used in the source (`m07 A`, `m07 B`, ...); search for them.

### m03 · Numbers with an overflow policy

**Goal.** Read numeric fields such as `latency_ms=42` without ever wrapping or panicking, and make the overflow behavior a visible choice.

**Tasks** in [`src/num.rs`](src/num.rs):

- A. `parse_u64(text, policy)`: digits only, the byte index of the first bad byte, and `Overflow::Reject` or `Overflow::Saturate` for values above `u64::MAX`. Accumulate with `checked_mul`/`checked_add`; do not call `str::parse`.
- B. `mean(total, count)`: integer-to-float conversion with `as`.

**Run** `cargo test --test m03_types`. **Done when** both profiles pass and you can say which `u64` values `as f64` rounds.

**Design question.** Why is the overflow policy a parameter of type `Overflow`, and not a `bool` or two functions?

### m04 · Zero-copy fields

**Goal.** Split a line into words and `key=value` fields without copying a single byte: every result is a `&str` into the line.

**Tasks** in [`src/scan.rs`](src/scan.rs): A. `split_word`; B. `split_field` (bare and quoted values, with `\"` not closing a quote); C. `find_field`, whose signature promises a value that outlives the vector of fields.

**Run** `cargo test --test m04_borrowing`. The technique tests compare pointer ranges: a result that is equal but copied fails. Another test checks that only ASCII whitespace separates words; `str::trim` and `char::is_whitespace` also accept Unicode spaces, which this format treats as ordinary characters. **Done when** you can draw the line, the `Field` structs, and the arrows between them.

**Design question.** `find_field` returns `Option<&'a str>` with the lifetime of the *line*, not of the `fields` slice. What could callers no longer do if it returned a reference tied to `fields`?

### m07 · Errors with positions

**Goal.** Replace "it didn't parse" (`None`) with "line 3, column 17: unterminated quoted value".

**Tasks:** in [`src/scan.rs`](src/scan.rs), A. `scan_field` (an `ErrorKind` per problem), B. `column_of` (characters, not bytes), C. `parse_fields` (use `?` and `map_err`); in [`src/error.rs`](src/error.rs), D. `Display` for `ErrorKind` and E. `Display` for `ParseError`.

**Refactor.** `split_field` and `scan_field` now contain the same scanner twice. Make `split_field` the one-liner `scan_field(s).ok()` and delete the duplicate. Run `cargo test --test m04_borrowing --test m07_errors`; the m04 tests, plus `a_split_field_agrees_with_scan_field`, prove that nothing changed.

**Run** `cargo test --test m07_errors`.

**Design question.** `ErrorKind` is a small `Copy` enum and `ParseError` adds the position. Why two types instead of one enum whose variants carry the position?

### m08 · Modules, a public API, doc tests

**Goal.** Treat the crate as a library that someone else uses: one error type for everything, documented paths, and examples that are tested.

**Tasks:** in [`src/error.rs`](src/error.rs), A. `Display` for `Error`, B. `source` (so error chains can be walked), C. the `From` conversions that let `?` turn an `io::Error`, `ParseError`, or `FilterError` into `loglens::Error`. Then read [`src/lib.rs`](src/lib.rs): the module tree, the `pub use` re-exports, and the `prelude`.

**Doc tests.** Run `cargo test --doc -- num:: scan::`: the examples on `parse_u64`, `split_word`, and `scan_field` must pass. Add a runnable example (a fenced code block in the `///` comment) to two more functions you wrote, and run `cargo doc --no-deps --open` to read your crate as a user would. The crate-level example in `lib.rs` needs m09 and m14; run plain `cargo test --doc` again after m14.

**Run** `cargo test --test m08_modules`.

**Design question.** `lib.rs` re-exports `Record`, `Filter`, and a few more names at the crate root, but not `split_word`. What rule would you use to decide which names get a short path?

### m09 · The Level enum and the Record struct

**Goal.** Turn a line into a typed value: a `Level` enum whose derived ordering means severity, and a `Record<'a>` struct that borrows everything from its line.

**Tasks:** in [`src/level.rs`](src/level.rs), A. `Level::from_name` (a `match` with guards), B. `as_str`; in [`src/record.rs`](src/record.rs), C. `is_timestamp` (one slice pattern), D. `parse_record` (`let ... else`, `?`, and the column table in its contract), E. `Record::get`, F. `Record::number` (`Option` and `Result` combined with `transpose`).

**Run** `cargo test --test m09_records`. **Done when** you can explain the `'a` in `Record<'a>` and what an owned record would cost per line.

**Design question.** The fields are a `Vec<Field<'a>>` in line order, not a `HashMap`. Which operations does that make cheaper, and which more expensive?

### m11 · Traits: Sink, Display, FromStr, TryFrom

**Goal.** Give the types the standard traits that users expect, and define the crate's own trait: a [`Sink`](src/sink.rs) receives matching records.

**Tasks:** in [`src/level.rs`](src/level.rs), A. `FromStr`, B. `Display` that honors `{:<5}`, C. `TryFrom<u8>` and `From<Level> for u8`; in [`src/record.rs`](src/record.rs), D. `Display` for `Record` (the canonical line, written without allocating); in [`src/sink.rs`](src/sink.rs), E–I. the sinks: `Vec<String>`, `Count`, `LineWriter`, `Tee`, and the blanket impls for `&mut S` and `Box<S>`.

**Run** `cargo test --test m11_traits`. The allocation tests fail if `Display` or `LineWriter` builds a `String` per record.

**Design question.** `impl From<Level> for u8` implements a standard-library trait for a standard-library type, and the compiler accepts it. Which part of the impl is local to this crate, and why is that enough?

### m14 · A filter language compiled to closures

**Goal.** `level>=warn user!=bob latency_ms>100` is parsed **once** into a list of boxed closures. Matching a record then parses nothing and, with one exception that m17 introduces, allocates nothing.

**Tasks** in [`src/filter.rs`](src/filter.rs): A. `Filter::parse` (find the operator, validate the value, build a `move` closure per term, with the exact error reasons in the contract); B. `Filter::matches`.

**Run** `cargo test --test m14_filters`. Technique tests: the filter must be `Send + Sync + 'static` (so it owns its values) and matching must not allocate. After this milestone, `cargo test --doc` should pass completely.

**Design question.** The terms are `Box<dyn Fn(&Record<'_>) -> bool + Send + Sync>`. What would the type of `Filter` look like with a generic closure parameter instead, and why can a list of different terms not use one?

### m15 · Iterator pipelines

**Goal.** Process a whole text as a lazy pipeline: lines → records → matching records → a sink, with line numbers and error reports.

**Tasks** in [`src/pipeline.rs`](src/pipeline.rs): A. `Iterator for Records` (skip ignored lines, number the errors); B. `AddAssign for Tally`; C. `matching` (adapters only, no `collect`); D. `slowest` (the *first* maximum, which `max_by_key` does not give you); E. `process` (count, filter, forward to the sink, report errors through a closure).

**Refactor.** Rewrite the loops you wrote in m04, m07, and m09 (`find_field`, `parse_fields`) with iterator adapters or `std::iter::from_fn` where that reads better, and keep loops where it does not. `cargo test --test m04_borrowing --test m07_errors --test m09_records` must stay green.

**Run** `cargo test --test m15_pipelines`. `c_matching_is_lazy` fails if `matching` parses the whole text before it returns the first record. `a_only_ascii_whitespace_makes_a_line_blank` fails if you find blank lines with `str::trim_start` or `str::trim`, which also remove Unicode spaces.

**Design question.** `process` reports malformed lines through `&mut dyn FnMut(ParseError)` instead of returning a `Vec<ParseError>`. What does the caller gain, for a file with ten million damaged lines?

### m16 · Aggregation and top-k

**Goal.** Turn records into a report: counts per level, per-group counts and latency statistics, the top groups, and merging partial results (which m19 needs).

**Tasks** in [`src/report.rs`](src/report.rs): A. `Group::key_of`; B. `From<&str> for Group`; C. `GroupStats::mean_latency`; D. `+=` for `GroupStats`; E. `Summary::add` (allocate a group name only the first time it appears); F. `Summary::top` (a `BinaryHeap` that never holds more than k entries); G. `+=` for `Summary`; H. `Sink for Summary`; I. `render`, the exact text shown by `loglens report`.

**Refactor.** Anywhere you aggregate by hand (for instance in a test of your own), pass a `Summary` to `process` instead: it is a `Sink`.

**Run** `cargo test --test m16_reports`. Two technique tests: `add` must not call `to_string` for every record, and `top(3)` of 5000 groups must not store all 5000 before it chooses.

**Design question.** `levels` is a `BTreeMap` and `groups` is a `HashMap`. What does each choice give the report, and what would the other choice cost?

### m17 · Quoted values and Unicode

**Goal.** Decode quoted values correctly, quote values for output, and shorten text without splitting characters, borrowing whenever nothing changes (`Cow<str>`).

**Tasks** in [`src/text.rs`](src/text.rs): A. `unescape`, B. `quote`, E. `truncate`; in [`src/scan.rs`](src/scan.rs), C. `Field::text`; in [`src/record.rs`](src/record.rs), D. `Record::text`.

**Refactor.** m14's filter compares `=`, `!=`, and `~` against the raw value, so `path=C:\temp` does not match `path="C:\\temp"`. Switch those comparisons to `record.text(key)`. Run `cargo test --test m14_filters` and `cargo test --test m17_text`. The zero-allocation check of m14 keeps passing, because `text` returns `Cow::Borrowed` for a bare value and for a quoted value without escapes. A quoted value *with* an escape has to be decoded into a new `String` first, so matching it costs one allocation for that record. That is the honest price of comparing decoded text; the module documentation of `filter.rs` says so.

**Run** `cargo test --test m17_text`.

**Design question.** `unescape` returns `Cow<'_, str>`. Compare it with returning `String` always, and with returning `&str` always. Which callers does each choice hurt?

### m18 · Streaming input and the CLI

**Goal.** Make it a real program: read files or stdin line by line without loading them into memory, follow a growing file, parse arguments by hand, and use stdout, stderr, and the exit status like a well-behaved Unix tool.

**Part 1, `m18_io`:** in [`src/pipeline.rs`](src/pipeline.rs), A. `process_reader` (one reused buffer, `read_until`, invalid UTF-8 as a malformed line); in [`src/follow.rs`](src/follow.rs), B. `Follower::poll` (complete lines only; keep a partial line for the next poll; return at the end of the input, never wait there) and C. `Follower::finish`.

**Refactor.** `process` and `process_reader` now do the same per-line work. Extract one private helper that both call (m20 will reuse it). Run `cargo test --test m15_pipelines --test m18_io`; the equivalence test `a_process_reader_equals_process` checks every line of both.

**Part 2, `m18_cli`:** in [`src/cli.rs`](src/cli.rs), D. `parse_args` (by hand, no crates; file names stay `OsString`), E. `run_with`, F. `main_with`; in [`src/main.rs`](src/main.rs), G. `main` (lock the streams, buffer stdout, return the `ExitCode`).

`run_with` receives an **opener**: the function it must call to open a FILE. The provided `run` passes `open_file`, which opens real files. The tests pass their own openers, so they can serve a file from memory or one that fails after two lines. The tests also run the built binary on `fixtures/`, including a `--follow` test that appends to a file while the program runs.

**Run** `cargo test --test m18_io --test m18_cli`, then try it: `cargo run -- report --by user fixtures/app.log`, `cat fixtures/web.log | cargo run -- grep -w status=500`, and `cargo run -- grep fixtures/app.log | head -1` (a closed pipe must not print an error).

**Design question.** `main.rs` is a few lines, and `cli` never names `std::io::stdout` or `File::open` outside `open_file`. Name two tests in `tests/m18_cli.rs` that could not exist otherwise.

### m19 · Parallel files, identical results

**Goal.** `loglens report --jobs 8 *.log` reads several files at the same time, and its output is byte-for-byte the output of `--jobs 1`.

**Tasks** in [`src/parallel.rs`](src/parallel.rs): A. `map_ordered` (scoped threads, dynamic work distribution through an atomic index, results sent back over a channel and put in input order; no threads at all for one worker); B. `summarize_files` (per-file summaries merged in path order with m16's `+=`).

**Refactor** `run_with` in [`src/cli.rs`](src/cli.rs): with `--jobs` other than 1 and several files, process the files with `map_ordered`. Each worker calls the opener for its file, reads it, and buffers that file's stdout and stderr; then you write the buffers in input order. `cargo test --test m18_cli` must stay green.

**Run** `cargo test --test m19_parallel`. Technique tests check that several threads really run at once, that a slow item does not stall the others, that one worker spawns nothing, and that the program itself opens files on several threads when `--jobs` asks for it. Equal output alone would also pass for a program that ignores `--jobs`, so `c_cli_jobs_reads_files_on_several_threads` looks at the threads.

**Design question.** `map_ordered` requires `T: Sync`, `R: Send`, and `F: Sync`. For each bound, name the place in your implementation that would not compile without it.

The course goes deeper into this topic (memory orderings, locks, and channels built by hand) in the mastery track, [`mastery/m3_concurrency`](../mastery/m3_concurrency/README.md).

### m20 · Optional: a std-only async reader

**Goal.** See what an async runtime does by writing the smallest one: a future that reads one line from a source that is often "not ready", and an executor that parks the thread until it is woken.

**Tasks** in [`src/aio.rs`](src/aio.rs): A. `Future for NextLine`; B. `block_on` (a `Waker` built from `std::task::Wake` that unparks the thread); C. `process_async`, which reuses your m18 per-line helper. `Trickle`, the simulated source, is provided; read its `poll_read` first.

**Run** `cargo test --test m20_async`. A busy-polling executor fails `b_block_on_sleeps_until_woken`. An executor whose waker does nothing sleeps forever; the tests report that after their time limit.

**Design question.** `Trickle::poll_read` calls `cx.waker().wake_by_ref()` before it returns `Pending`. What is the rule behind that call, and what happens to your `block_on` if a source breaks it?

For real programs, use an async runtime such as Tokio; the [ecosystem track](../ecosystem/README.md) has a lesson on it. This milestone is about understanding what a runtime does. The mastery track, [`mastery/m4_async`](../mastery/m4_async/README.md), builds a fuller one.

### m22 · Macros: field! and log_line!

**Goal.** Remove repetition at call sites with two `macro_rules!` macros: `field!(record, user)` / `field!(record, latency_ms as u64)`, and `log_line!(ts, Info, "auth", user = "ada")`, which builds a correctly quoted line.

**Tasks** in [`src/macros.rs`](src/macros.rs): A. the expansions of `field!`; B. the expansion of `log_line!`. The patterns are given, so that the tests compile. The expansions must use `$crate::` paths, evaluate each argument once, and be hygienic, which the tests check with a caller that defines its own `quote`, `text`, and `Level`.

**Refactor (optional).** Build the log lines of your own new tests with `log_line!`. It quotes values correctly, which hand-written test lines often do not.

**Run** `cargo test --test m22_macros`.

**Design question.** `field!(record, user)` saves a few characters over `record.get("user")`. When is a macro like this worth having, and when is the plain method call the better API?

### m23 · Optional: a fast byte search, and when unsafe is worth it

**Goal.** Make the byte search of the scanner faster, in **safe** code first. Then write `unsafe` code behind a safe function, prove it sound in a comment, check it with Miri, and let a measurement decide whether the unsafe version stays.

The scanner looks for single bytes (`=`, `"`, `\`, spaces). A classic speed-up looks at eight bytes per step: load them into one `u64` and test all eight with a few arithmetic operations. For a long time, loading eight bytes from a slice at any address was done with a raw pointer read. Since `slice::as_chunks` is in the standard library, it needs no `unsafe` at all.

**Tasks** in [`src/fast.rs`](src/fast.rs):

- A. `find_byte`: the word-at-a-time search with `as_chunks::<8>()` and `u64::from_ne_bytes`. The function carries `#[forbid(unsafe_code)]`, so the compiler rejects `unsafe` inside it.
- B. `split_ascii`: split a `&str` at an ASCII byte with `get_unchecked`. Here `unsafe` skips a bounds check and a `char`-boundary check that you can prove are unnecessary. The proof is the task: write the `// SAFETY:` comment *before* you run the code.
- C (optional comparison). `find_byte_unaligned`: the same search with `ptr.add(i).cast::<u64>().read_unaligned()`. Its tests are marked `#[ignore]`, so the milestone is green without it: `cargo test --test m23_unsafe -- --ignored c_`.

**Measure, then decide.** The test file contains a small measurement. Run it in release mode:

```sh
cargo test --release --test m23_unsafe -- --ignored --nocapture d_measure
```

One run of the model solution on one machine (an Apple M-series laptop) printed:

```text
                         1 MiB, no match       20000 short lines
plain loop                       263.2µs                 318.0µs
find_byte                         38.6µs                  84.4µs
find_byte_unaligned               37.7µs                  89.8µs
```

Your numbers will differ, and they move by a few percent from run to run. The pattern is what matters. The word-at-a-time idea made the search several times faster. Writing it with `unsafe` added nothing that the measurement can show. So the safe `find_byte` is the one to keep: it costs no proof, and no later change to it can cause undefined behavior. Keep an `unsafe` version only when a measurement on realistic input shows a gain that matters to the program, and then keep the measurement next to it. The same question applies to `split_ascii`: the checks it skips are cheap, and in a real code base you would measure before you accept the `unsafe` block. It is in this milestone because its safety argument is a good one to learn to write.

The tests cannot tell a word-at-a-time `find_byte` from a plain loop, because both return the same results. The measurement can.

**Refactor (optional).** Use the version you chose in `scan.rs` to look for `=`, `"`, and `\`, and write the reason for the choice next to the call. Every milestone must stay green: `cargo test --tests`.

**Miri.** Tests show that the code works on the inputs they try. Miri is an interpreter for Rust programs that detects undefined behavior, such as an out-of-bounds or misaligned read, while the tests run. It needs a nightly toolchain. From the `project/` directory:

```sh
rustup toolchain install nightly-2026-11-20 --component miri
cargo +nightly-2026-11-20 miri test --test m23_unsafe
cargo +nightly-2026-11-20 miri test --test m23_unsafe -- --include-ignored    # with task C
```

Miri interprets every instruction, so it is slow. On the machine that produced the numbers above, the second command took about one minute and the third two to three minutes.

To see what Miri adds, break the optional version on purpose: change its loop condition from `i + 8 <= haystack.len()` to `i < haystack.len()`, so that the last read runs past the end of the slice. Under `cargo test`, `c_unaligned_returns_the_first_match` still passes, because the bytes after the slice happen to be readable. Under Miri the same test stops at the read (trimmed):

```text
error: Undefined Behavior: memory access failed: attempting to access 8 bytes, but got alloc58170+0x50 which is only 1 byte from the end of the allocation
   --> src/fast.rs:119:29
    |
119 |         let word = unsafe { haystack.as_ptr().add(i).cast::<u64>().read_unaligned() };
    |                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Undefined Behavior occurred here
```

(The allocation name and the line number will differ in your code.)

Neither the tests nor Miri prove soundness for every input. Miri checks only the executions that the tests produce; your written argument covers the rest.

**Run** `cargo test --test m23_unsafe`.

**Design question.** `split_ascii` panics for a non-ASCII `byte` instead of being an `unsafe fn` that requires an ASCII byte. Who has to read and maintain the safety argument in each design?

**Going further.** Chapter 24 covers calling C. As a manual extension, declare C's `memchr` in an `unsafe extern "C"` block, wrap it in a safe function, and add it to the measurement in a scratch test. Keep it out of the committed code, because it depends on the platform's C library. The mastery track builds `Vec`, `Rc`, and other containers from raw memory ([`mastery/m2_unsafe`](../mastery/m2_unsafe/README.md)) and treats benchmarking and profiling properly ([`mastery/m7_perf`](../mastery/m7_perf/README.md)).

## After the last milestone

- Delete the `#![allow(dead_code, unused_variables, unused_imports)]` line in `src/lib.rs` and fix every warning. Then run `cargo clippy --all-targets` and the whole suite in both profiles: `cargo test --tests && cargo test --release --tests`.
- Compare your code with [`solutions/project/`](../solutions/project/), file by file, as described under [Solutions](#solutions).
- Point the program at real logs. Many services can write `key=value` lines; for others, write a converter, or change the parser (with the tests as your safety net).
- Change a requirement, predict what breaks, then make it pass: fractional seconds in timestamps (careful: string order is then no longer time order), a 95th-percentile latency in the report, `--since 10m`, JSON lines as input, or log rotation in `--follow`.
- **Design something yourself.** These extensions come without signatures. Write the contract first, as doc comments and tests, then the code: an `--output json` mode (which trait does it implement?), a `--sample N` option that keeps every Nth matching record (where does the state live?), or a second input format behind a trait that `process_reader` accepts. Then move on to the blank-file projects in [`capstones/`](../capstones/).
- Redo one milestone from its contract after a break, without looking at your old code.

## Validation (for maintainers)

The starter must compile without warnings and stay formatted. From the repository root:

```sh
cargo test --manifest-path project/Cargo.toml --locked --offline --all-targets --no-run
cargo test --manifest-path project/Cargo.toml --locked --offline --release --all-targets --no-run
cargo fmt --manifest-path project/Cargo.toml --all --check
cargo clippy --manifest-path project/Cargo.toml --locked --offline --all-targets
```

The model solution in `solutions/project/` must stay in step with the starter. Every file there is the starter file with only bodies, private helpers, and `guess()` answers changed; the tests are otherwise byte-identical. After you change a contract or a test, copy `project/` to a scratch directory, copy `solutions/project/` over it, and check there that:

- `cargo test --tests` and `cargo test --release --tests` pass, three times each (several tests use threads and timing);
- `cargo test --test m23_unsafe -- --include-ignored` and `cargo test --doc` pass;
- `cargo clippy --all-targets` prints no warning;
- `cargo +nightly-2026-11-20 miri test --test m23_unsafe -- --include-ignored` reports no undefined behavior;
- each technique test still fails for the wrong technique it was written to catch (see [AUTHORING.md](../AUTHORING.md)).
