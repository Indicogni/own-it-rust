# Production Rust: the ecosystem track

[All chapters](../README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Tooling workday](TOOLING.md)

The chapter exercises use only the standard library, on purpose. Professional Rust code also uses a small set of crates and tools almost everywhere. This track gives you guided, tested practice with them: serde, thiserror, anyhow, clap, tracing, proptest, tokio, axum, reqwest, and the Cargo tooling around them. The aim is that you can sit down at a real Rust code base and recognise what you see.

Each lesson tells you which chapter to finish first. Each lesson also ends with **judgment**: why this crate, what the alternatives are, and when you would choose differently. Knowing the API is half of the work; choosing well is the other half.

This track covers the core production crates. It does not cover databases, authentication, metrics, or deployment. Those build on what you learn here, and the mastery modules of the course continue from this point: [M11 · Rust as a service](../mastery/m11_services/README.md) builds on E01, E02, E04, E06 and E09, and [M7 · Performance engineering](../mastery/m7_perf/README.md) continues the timing and profiling tasks of E08.

## Setup

This is a standalone Cargo package with its own `Cargo.lock` and pinned versions. It is **not** part of the root package, so the chapter checks never download or build these crates. Rust 1.99.0 is pinned in `ecosystem/rust-toolchain.toml`. Run commands from the repository root:

```sh
# The first build downloads the pinned crates; afterwards you can add --offline.
cargo test --manifest-path ecosystem/Cargo.toml --locked --all-targets --no-run
cargo test --manifest-path ecosystem/Cargo.toml --locked --release --all-targets --no-run
```

The starters compile; tests fail at `todo!()` and `guess()` until you fill them in. That is the starting state, not a broken setup.

`cargo progress` and `cargo drill` belong to the root package, so run them from the repository root, not from inside `ecosystem/`. `cargo progress all` also shows one row for this track.

No test should hang. A test that is still running after 60 seconds fails with a message that says so. That almost always means your code is waiting for something that never happens: a queue that is never closed, a loop with no exit, a lock taken twice. The helpers for this are in `src/test_support.rs`; you do not need to edit that file.

## Work through the track

Same loop as the chapters, with one step before it:

1. **Study** the lesson's section in [CONCEPTS.md](CONCEPTS.md): what the crate is for, the mental model, a worked example, the usual mistakes, and the alternatives.
2. **Read** the lesson file's header: what to predict, what to implement, what to explain, and the judgment notes.
3. **Predict.** Answer the `p_predictions` test first: replace each `crate::guess()` call with a value. The prediction tests don't need your implementation.
4. **Implement** the `todo!()` bodies from their contracts. Some tasks are attributes on supplied types (serde, clap, thiserror, `#[instrument]`). Run only that lesson, for example `cargo test --manifest-path ecosystem/Cargo.toml e01_`.
5. **Check technique.** Some tests check *how*: borrowed pointers in E02, the job handed back on timeout in E06, mutants your properties must catch in E05, a request that must survive shutdown in E09. Then run clippy and fix warnings in your code.
6. **Explain** the EXPLAIN prompt in your notes, add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

| Done | Lesson | Start after | Crates | Time (estimate) |
| --- | --- | --- | --- | --- |
| [ ] | [E01 · Library and application errors](src/e01_errors.rs) | [Chapter 7](../exercises/chapter07/README.md) | thiserror, anyhow | 2–3 h |
| [ ] | [E02 · serde: derive, attributes, borrowing, validation](src/e02_serde.rs) | Chapters [11](../exercises/chapter11/README.md) and [13](../exercises/chapter13/README.md) | serde, serde_json | 3–4 h |
| [ ] | [E03 · Command-line interfaces](src/e03_clap.rs) | [Chapter 18](../exercises/chapter18/README.md) | clap | 2–3 h |
| [ ] | [E04 · Structured logging](src/e04_tracing.rs) | [Chapter 18](../exercises/chapter18/README.md) | tracing, tracing-subscriber | 2–3 h |
| [ ] | [E05 · Property-based testing](src/e05_proptest.rs) | Chapters [15](../exercises/chapter15/README.md) and [16](../exercises/chapter16/README.md) | proptest | 4–5 h |
| [ ] | [E06 · Async services](src/e06_tokio.rs) | Chapters [20](../exercises/chapter20/README.md) and [21](../exercises/chapter21/README.md) | tokio | 5–6 h |
| [ ] | [E07 · Capstone: `jobq` job runner](src/e07_capstone.rs) | E01–E06 | all of the above | 6–8 h |
| [ ] | [E08 · Tooling workday](TOOLING.md) ([code](src/e08_tooling.rs)) | E01; tasks 6–7 after [Chapter 15](../exercises/chapter15/README.md) | cargo, clippy, rustdoc | 4–6 h |
| [ ] | [E09 · An HTTP JSON service and its client](src/e09_http.rs) | E01, E02, and E06 | axum, reqwest, tokio-util | 5–6 h |

The times are estimates for a learner who has finished the chapters named in "Start after". They include reading the concepts section and writing your explanation. The whole track is about 33 to 44 hours.

E01–E06 are independent of each other; do each one when you have finished its chapters. E07 builds on all of them, and its tracing test also uses your E04 `capture`. E08 can be done in parts whenever you have a free afternoon. E09 reuses the error style of E01, the serde attributes of E02, and the cancellation ideas of E06.

```sh
cargo test --manifest-path ecosystem/Cargo.toml -- --list
cargo test --manifest-path ecosystem/Cargo.toml e01_
cargo test --manifest-path ecosystem/Cargo.toml --release e01_
cargo test --manifest-path ecosystem/Cargo.toml --test e07_capstone --test e07_tracing
cargo run --manifest-path ecosystem/Cargo.toml --bin jobq -- -v run ecosystem/jobs.example.json
cargo run --manifest-path ecosystem/Cargo.toml --example e09_serve
cargo test --manifest-path ecosystem/Cargo.toml --doc
cargo clippy --manifest-path ecosystem/Cargo.toml --all-targets
cargo fmt --manifest-path ecosystem/Cargo.toml --all --check
```

Lesson filters (`e01_` … `e09_`) match the module path of the unit tests. A filtered run also starts the integration-test binaries, which then report 0 tests. That is expected.

`cargo test --doc` also compiles and runs every Rust code block in [CONCEPTS.md](CONCEPTS.md), so the examples you read there are checked against the pinned crates.

## The lessons

### E01 · Library errors with thiserror, application errors with anyhow

**Teaches:** an error enum whose variants callers can match; Display messages that don't repeat their source; `#[source]` and `#[from]`; `?` conversions; anyhow's `context`, `{:#}` chains, and `downcast_ref`; mapping errors to exit codes by walking `chain()`.

**Judgment:** A *library* returns an error type callers can inspect, because they may recover differently from `MissingKey` than from `Io`. An *application* mostly reports errors to a human, so one opaque type with context at each layer is better. thiserror writes exactly the `Display`/`Error`/`From` impls you would write by hand, so it adds nothing to your public API. Never put `anyhow::Error` in a public library signature. Alternatives: hand-written impls (no dependency; fine for two or three variants), [snafu](https://crates.io/crates/snafu) (context selectors, good for large libraries), [eyre](https://crates.io/crates/eyre)/color-eyre (anyhow-style with customizable reports), [miette](https://crates.io/crates/miette) (diagnostics with source spans, for compilers and linters). Don't create a variant nobody will match: a variant is a promise to your callers.

### E02 · serde

**Teaches:** derive with `rename_all`, `default`, `skip`, `skip_serializing_if`, `deny_unknown_fields`; internally tagged enums; zero-copy deserialization with `&str` and `Cow` + `#[serde(borrow)]`; a hand-written `Serialize`/`Deserialize` that keeps a type's invariant; round-trip tests.

**Judgment:** serde is the standard: one data model and many formats (JSON, TOML, YAML, MessagePack, [postcard](https://crates.io/crates/postcard)...). Your wire format is a public API. `deny_unknown_fields` suits config files, where typos should fail loudly. It is risky for messages between services, where an old reader must accept a newer writer's extra fields. `#[serde(default)]` is how a format evolves. A derived `Deserialize` never calls your constructor, so validated types need `#[serde(try_from = "...")]` or a hand-written impl. Borrowing matters on hot paths; elsewhere owned `String`s are simpler. Alternatives: hand parsing for tiny formats; [miniserde](https://crates.io/crates/miniserde) (much smaller, less flexible); schema-first formats like protobuf ([prost](https://crates.io/crates/prost)) when other languages share the schema; [rkyv](https://crates.io/crates/rkyv) for zero-copy archives. One format you will still see in older code is bincode. Its maintainers have stopped development, so do not start a new project on it; postcard is a maintained compact binary format.

### E03 · clap

**Teaches:** derive-based parsers with subcommands, global flags, counted flags, defaults, ranges, `ValueEnum`, and custom value parsers. Testing them with `try_parse_from` and `Command::debug_assert`. Routing help and version output to stdout with exit code 0, and usage errors to stderr with exit code 2.

**Judgment:** For a CLI that people use, clap's help, suggestions, validation, and shell completions are worth its compile time. Validation belongs in the parser (`value_parser`): the error then names the flag, and nothing downstream sees invalid input. Keep `parse()` in `main` and test `try_parse_from`. Alternatives: [lexopt](https://crates.io/crates/lexopt) or [pico-args](https://crates.io/crates/pico-args) (tiny, manual), [argh](https://crates.io/crates/argh) (small derive), [bpaf](https://crates.io/crates/bpaf) (combinators), or plain `std::env::args` for a two-flag internal tool.

### E04 · tracing

**Teaches:** events vs spans; fields recorded with Display (`%`) and Debug; `#[instrument]` with `skip_all` so secrets never reach logs; fields recorded after the span is created (`field::Empty` + `record`); a custom `MakeWriter` that captures output in tests; `with_default` vs a global subscriber; level filtering, which skips argument evaluation.

**Judgment:** Libraries *emit* tracing data; only the binary installs a subscriber, once, in `main`. Spans carry context across `.await` points and threads, which is why async services choose tracing over [log](https://crates.io/crates/log). log + [env_logger](https://crates.io/crates/env_logger) is simpler and fine for small synchronous tools, and tracing-subscriber can collect `log` records. Production setups add the `env-filter` feature (`RUST_LOG=info,my_crate=debug`), `json` output, or [tracing-opentelemetry](https://crates.io/crates/tracing-opentelemetry). They are left out here to keep the build small. A span filtered out by its level also disappears from the context of the events inside it. E07 makes you deal with that.

### E05 · Property-based testing with proptest

**Teaches:** writing generators (`prop_oneof!`, `prop_map`, `collection::vec`, weights, `.boxed()`); round-trip, invariant, and model-based properties; `prop_assert!` inside property functions; why generators must reach the edge cases (runs longer than 255, empty input); how shrinking turns a random 700-byte failure into a minimal one. The supplied code is correct. The `m_` tests check your properties against deliberately broken mutants, and show that a round-trip property alone misses real bugs.

**Judgment:** proptest's integrated shrinking (each value remembers how to simplify itself) usually gives better counterexamples than [quickcheck](https://crates.io/crates/quickcheck), which is smaller and simpler. Coverage-guided fuzzing ([cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) with [arbitrary](https://crates.io/crates/arbitrary)) explores deeper and runs for hours, outside `cargo test`. [cargo-mutants](https://mutants.rs) automates what the `m_` tests do by hand. Keep example tests for documented behavior and properties for searching. A property is only as good as its generator. When proptest finds a failure, it saves the seed under `ecosystem/proptest-regressions/`. Real projects commit those files; this package ignores them.

proptest is a dev-dependency, so this lesson's module exists only in test builds (`#[cfg(test)] mod e05_proptest;`).

### E06 · tokio

**Teaches:** `JoinSet` with a concurrency limit and per-task panic reporting; load shedding with `try_send`; backpressure with a deadline, using `reserve()` so a timeout hands the job back; a `select!` batcher whose state and deadline survive cancelled branches; `spawn_blocking` for CPU work. The tests run on a runtime with paused time (what `#[tokio::test(start_paused = true)]` gives you), so time is virtual and exact: no real sleeps, no flaky timing.

**Judgment:** tokio is the default runtime for network services, and most of that ecosystem (hyper, axum, reqwest, tonic, sqlx) builds on it. Async pays off when you have many concurrent waits (sockets, timers). For CPU-bound work, use threads or [rayon](https://crates.io/crates/rayon); for a handful of blocking tasks, plain threads and channels are simpler. Enable only the features you use: E06 and E07 need `macros`, `rt`, `sync`, and `time`, plus `test-util` for tests. E09 adds what a server needs: `net` for sockets, `rt-multi-thread` for its end-to-end test and its example binary, and `signal` for Ctrl-C in that binary. `full` is convenient, but it compiles everything. Alternatives: [smol](https://crates.io/crates/smol) (small, composable), [embassy](https://embassy.dev) (embedded). async-std is discontinued. Helpers you will meet next: `tokio-util`'s `CancellationToken` (used in E09), and [async-channel](https://crates.io/crates/async-channel) for multi-consumer queues.

### E07 · Capstone: `jobq`

A small job runner that combines E01 to E06: a JSON config (serde, with validation), a clap CLI (`check`, `run`, `--workers`, `--json`, `-v`), a thiserror `ConfigError` with anyhow context in the application layer, tracing spans per job, and a tokio worker pool with a bounded queue, per-job timeouts, and `spawn_blocking`. `main` stays thin. The library has everything testable. The integration tests drive the library with virtual time and also run the real binary. Exit codes:

| Code | Meaning |
| --- | --- |
| 0 | every job succeeded (or `check` passed) |
| 1 | at least one job failed or timed out |
| 2 | usage error (clap) |
| 66 | config file not found (`EX_NOINPUT`) |
| 78 | invalid config (`EX_CONFIG`) |

The tracing test lives in its own test binary, `tests/e07_tracing.rs`. tracing caches each callsite's interest process-wide. Parallel tests that hit the same callsites while another test installs a scoped subscriber can silently lose events. One test per process avoids that. `cargo nextest` does it for every test.

### E08 · Tooling workday

A guide with checkable tasks, in [TOOLING.md](TOOLING.md): `cargo doc` and doc tests, `cargo tree` and feature unification, clippy configuration through `[lints]`, macro debugging without cargo-expand, release profiles, honest timing with `Instant`, and reading a flamegraph from folded stacks. Optional tools are listed with install commands; none is required.

Going further: mastery module M7 turns tasks 6 and 7 into full lessons, [benchmark design](../mastery/m7_perf/CONCEPTS.md#lesson-01--benchmark-design) (lesson 01) and [reading a profile](../mastery/m7_perf/CONCEPTS.md#lesson-02--reading-a-profile) (lesson 02).

### E09 · An HTTP JSON service and its client

**Teaches:** an axum router with shared state, path and JSON extractors, and one error type that becomes a JSON error response; a per-request deadline as middleware; graceful shutdown with a `CancellationToken`; a typed reqwest client with a timeout that separates "no such item", "the server refused", "timed out", and "could not connect"; testing a router with no socket (`oneshot`) and a whole service over `127.0.0.1:0`. Everything runs on your machine, with no outside network.

**Judgment:** axum is a thin layer over hyper (the HTTP implementation) and tower (middleware as `Service`s), and it is the usual choice on tokio. [actix-web](https://crates.io/crates/actix-web) is a mature alternative with its own runtime setup (one single-threaded runtime per worker thread). Using [hyper](https://crates.io/crates/hyper) directly is for proxies and unusual protocols. For gRPC use [tonic](https://crates.io/crates/tonic). reqwest is the usual client; [ureq](https://crates.io/crates/ureq) is a small blocking client for scripts and CLIs with no async runtime. This lesson writes its deadline middleware by hand so that you see how it works; real services usually take [tower-http](https://crates.io/crates/tower-http)'s `TimeoutLayer`, `TraceLayer`, and request-size limits instead. Test at two levels: most cases against the router with no socket, and a few against a real listener, because only those exercise the connection handling and shutdown.

Run it by hand with the supplied example, `examples/e09_serve.rs`, and `curl`; the commands are at the top of that file.

Going further: mastery module [M11 · Rust as a service](../mastery/m11_services/README.md) starts where this lesson stops, with tower middleware, a SQL database, authentication, observability and a binary protocol.

## Pinned versions

| Crate | Version | Features |
| --- | --- | --- |
| serde | 1.0.229 | `derive` |
| serde_json | 1.0.154 | default |
| thiserror | 2.0.21 | default |
| anyhow | 1.0.104 | default |
| clap | 4.6.7 | default + `derive` |
| tracing | 0.1.44 | default |
| tracing-subscriber | 0.3.23 | `fmt`, `std` only (no defaults) |
| tokio | 1.54.1 | `macros`, `net`, `rt`, `rt-multi-thread`, `signal`, `sync`, `time`; dev: `test-util` |
| tokio-util | 0.7.20 | default (`CancellationToken` needs no feature) |
| axum | 0.8.9 | `http1`, `json`, `tokio` only (no defaults) |
| reqwest | 0.13.5 | `json` only (no defaults: no TLS, no HTTP/2) |
| proptest (dev) | 1.12.0 | `std` only (no defaults) |
| tower (dev) | 0.5.3 | `util` only (no defaults) |

Versions are pinned with `=` so that error messages, help text, and log formats match the tests. To upgrade a crate in a real project: read its changelog, run `cargo update -p NAME --precise VERSION`, check `cargo tree --duplicates`, and run the full test suite.

reqwest is built without TLS here because every request stays on loopback. A client that talks to the internet needs TLS: keep reqwest's default features, or choose `rustls` or `native-tls` yourself.

## Solutions and validation

The starters compile in debug and release with `--locked`; `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean.

Model solutions for every file are in `solutions/ecosystem/` at the top of the course repository, under the same paths as here (for example `solutions/ecosystem/src/e01_errors.rs`). Each one is the lesson file with only the task bodies and the predictions filled in; the tests are unchanged. They pass every test and clippy in both profiles. Look at a solution only after your own tests pass, or after both [hints](HINTS.md) did not help: comparing a working answer of your own with the model teaches far more than reading the model first.
