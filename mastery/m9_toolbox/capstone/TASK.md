# Capstone: turn `tally` into a well-run workspace

[Module guide](../README.md) · [Concepts](../CONCEPTS.md) · [Hints](../HINTS.md)

`tally` adds up a ledger file by category. It works on the author's machine, on the
author's files. Two crates sit next to each other, each with its own manifest, no
tests, no lints, no CI, and a parser that panics on bad input. Your job is everything
a maintainer would do before inviting contributors.

Check your work from the repository root:

```sh
cargo test --manifest-path mastery/m9_toolbox/Cargo.toml --test l10_capstone
```

The tests run real Cargo commands inside `capstone/`. The first run after you add
`insta` downloads it, unless it is already in your Cargo cache.

## What the program must do

Input: one entry per line, `YYYY-MM-DD category amount`, separated by spaces or tabs.
Blank lines and lines that start with `#` are skipped.

- The date is four digits, a dash, a month `01`-`12`, a dash, a day `01`-`31`.
- The category is any text without white space.
- The amount is not negative: digits, optionally a dot and one or two more digits.
  `12`, `12.5` and `12.50` all mean twelve units and fifty hundredths. The digits are
  ASCII digits. An amount is stored in hundredths as a 64-bit integer; an amount that
  does not fit is an error like any other bad line, never a panic and never a wrapped
  number.

`tally report <file>` prints one row per category, sorted by name, then a `total` row.
Names are padded on the right to the longest name (`total` counts). Amounts have two
decimals and are padded on the left to the widest amount. Two spaces separate the
columns:

```text
books        31.98
groceries    58.30
rent       1200.00
total      1290.28
```

`tally report --json <file>` prints the entries in input order as one line of JSON:

```text
[{"date":"2026-11-02","category":"rent","cents":120000},{"date":"2026-11-03","category":"groceries","cents":1250}]
```

A category may contain any character that is not white space, so `"` and `\` in it
must be escaped in the JSON output (`\"` and `\\`).

Sums are exact. One amount fits in 64 bits, but a category sum or the total may not;
the report must still show the right number (add in a wider type).

Errors: a line that cannot be parsed stops the program with exit code 1 and a message
on stderr that starts with `line N:` (lines are counted from 1, including skipped
ones). A file that cannot be read also gives exit code 1. Wrong arguments give exit
code 2: no arguments, no file, or `--json` without a file. The program never panics,
whatever the file contains.

## What the project must look like

1. **Workspace.** `capstone/Cargo.toml` is a virtual workspace with resolver 3 and the
   members `tally-core` and `tally`. Version `0.4.0`, edition 2024, licence MIT and
   `rust-version = "1.99"` are set once and inherited. Dependencies (`tally-core`,
   `insta`) are declared once in `[workspace.dependencies]`. A `rust-toolchain.toml`
   pins Rust 1.99.0 with rustfmt and clippy.
2. **Lints.** `[workspace.lints]`: `unsafe_code` forbidden, `missing_docs` warned,
   Clippy's `pedantic` group warned with a negative priority. Both members inherit
   them. No `allow(warnings)` anywhere. `cargo fmt --all --check` and
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
   pass.
3. **Features.** JSON output is optional. `tally-core` has a feature `json`, off by
   default, that adds `to_json`. `tally` has a feature `json`, on by default, that
   forwards to it. Built with `--no-default-features`, `tally report --json <file>`
   exits with code 2 and says that this build has no JSON support.
4. **Tests.** `cargo test --workspace --all-features --locked` passes and includes
   unit tests for the parser, at least one snapshot test with `insta` for the report
   (its `.snap` file is committed), and a doc test: at least five tests in total.
   `Cargo.lock` is committed; the commands with `--locked` need it.
5. **Fuzzing.** `capstone/fuzz/` is a cargo-fuzz project with one target that feeds
   arbitrary text to the line parser. It is not a member of the workspace.
6. **Documentation.** Every public item is documented.
   `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` passes.
   `README.md` explains the input format (`YYYY-MM-DD category amount`) and shows
   the `tally report` command. `CHANGELOG.md` has an `## [Unreleased]` section and
   a section for `0.4.0`.
7. **Policy.** `capstone/deny.toml`: yanked crates denied, an `allow` list of
   licences that contains MIT and no GPL-family licence, duplicate versions denied,
   unknown registries and git sources denied.
8. **CI.** `capstone/.github/workflows/ci.yml` runs on pushes and pull requests with
   a read-only token, the pinned 1.99.0 toolchain (name the version, or let rustup
   read `rust-toolchain.toml`), pinned actions and a build cache. It runs fmt,
   clippy with `-D warnings`, the tests on Linux, macOS and Windows, the
   documentation build with `RUSTDOCFLAGS: -D warnings`, and `cargo deny check`.
   Every cargo command that builds uses `--locked`.

You may restructure the code as you like. Keep the two package names and the
directory names `tally-core/` and `tally/`.
