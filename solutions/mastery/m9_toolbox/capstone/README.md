# tally

Adds up a ledger file by category.

```sh
cargo run -p tally -- report ledger.txt
cargo run -p tally -- report --json ledger.txt
```

## Input

One entry per line: `YYYY-MM-DD category amount`, separated by spaces or tabs. The
amount is `12`, `12.5` or `12.50`. Blank lines and lines that start with `#` are
skipped.

```text
2026-11-02 rent 1200
2026-11-03 groceries 12.50
```

## As a library

The crate `tally-core` holds the parser and the report:

```rust
let entries = tally_core::parse("2026-11-02 rent 1200\n2026-11-03 food 12.5\n")?;
assert_eq!(tally_core::report(&entries), "food     12.50\nrent   1200.00\ntotal  1212.50\n");
# Ok::<(), tally_core::ParseError>(())
```

JSON output is behind the feature `json` (on by default in the `tally` binary, off
by default in `tally-core`).

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo +nightly-2026-11-20 fuzz run parse_line      # needs cargo-fuzz
```

See [CHANGELOG.md](CHANGELOG.md) for the release history.
