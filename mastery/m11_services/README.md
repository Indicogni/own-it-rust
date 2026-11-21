# Mastery M11 — Rust as a service

[All chapters](../../README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Notes workbook](NOTES.md) · [Capstone brief](capstone/README.md) · [Container lab](lab/README.md)

In this module you build network services the way they are built at work: an HTTP API on axum and tower, a SQL database behind sqlx, password login, CPU-heavy work beside async I/O, a binary protocol between two programs, and the logs, metrics, health checks and shutdown behaviour that let other people run what you wrote. Every lesson is checked by tests. All of them run on your machine, on loopback addresses and SQLite, without any external server.

## Who this is for

You have finished the core course and can write async Rust with tokio without looking up the basics. You want to be able to build, review and operate a backend service in Rust, and to say why it is built the way it is.

**Prerequisites**

- The core chapters, in particular [7](../../exercises/chapter07/README.md) (errors), [11](../../exercises/chapter11/README.md) and [13](../../exercises/chapter13/README.md) (traits), [18](../../exercises/chapter18/README.md) (I/O), [19](../../exercises/chapter19/README.md) (threads) and [20](../../exercises/chapter20/README.md) (async).
- From the ecosystem track: [E01](../../ecosystem/README.md#e01--library-errors-with-thiserror-application-errors-with-anyhow) (thiserror and anyhow), [E02](../../ecosystem/README.md#e02--serde) (serde), [E04](../../ecosystem/README.md#e04--tracing) (tracing) and [E06](../../ecosystem/README.md#e06--tokio) (tokio). This module uses those crates without introducing them again. The ecosystem track also has one short HTTP lesson; this module starts where that one ends.
- Basic SQL: `SELECT`, `INSERT`, `UPDATE`, `JOIN`. No database administration.
- A C compiler on your machine. sqlx compiles SQLite from its C source, so nothing has to be installed separately, but the build needs `cc`, `gcc`, `clang` or MSVC.

Mastery module [M4](../m4_async/README.md) builds an async runtime from first principles. You do not need it here, and this module does not repeat it: M4 is about how a runtime works, M11 about what you build on top of one.

## Setup

This is a standalone Cargo workspace with its own `Cargo.lock` and pinned versions. It is **not** part of the course's root package, so the chapter checks never download or build these crates. Rust 1.99.0 is pinned in `rust-toolchain.toml`, identical to the root one. Run commands from the repository root:

```sh
# The first build downloads about 200 crates and compiles SQLite. Allow a few minutes.
cargo test --manifest-path mastery/m11_services/Cargo.toml --locked --all-targets --no-run
cargo test --manifest-path mastery/m11_services/Cargo.toml --locked --release --all-targets --no-run
```

After the first build you can add `--offline`. The starters compile; tests fail at `todo!()` and `guess()` until you fill them in. That is the starting state, not a broken setup.

The workspace has three packages:

| Package | Directory | Holds |
| --- | --- | --- |
| `m11_services` | `src/`, `tests/` | lessons 01 to 09, C01, and supplied test helpers (`src/support.rs`) |
| `beacon_wire` | `wire/` | lesson 07, part 1: the protocol crate that two programs share |
| `beacon` | `capstone/` | the capstone: two binaries and their acceptance tests |

## Work through the module

Every lesson uses the same loop as the chapters.

1. **Read** the section for the lesson in [CONCEPTS.md](CONCEPTS.md), then the lesson file's header.
2. **Predict.** Answer the `p_predictions` test first: replace each `guess()` with a value. Do not run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run only that lesson, for example `cargo test --manifest-path mastery/m11_services/Cargo.toml -p m11_services l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Many tests check *how*: they count SQL statements, need two calls in flight at the same time, cancel a request half way, or let a job wait for a message that only a free runtime can send. Then run clippy and fix the warnings in your code.
5. **Explain** the EXPLAIN prompt in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

The scaffolding gets thinner as you go. Lessons 01 to 04 give you signatures. From lesson 05 you design state and handlers yourself against an HTTP contract. In lesson 09 you write the tests. The capstone gives you a brief and acceptance tests, and nothing else.

| Done | Lesson | Start after | Main crates | Estimated time |
| --- | --- | --- | --- | --- |
| [ ] | [01 · Service anatomy](src/l01_anatomy.rs) | E01, E02, E06 | axum | 2.5 h |
| [ ] | [02 · Middleware with tower](src/l02_middleware.rs) | 01 | tower, tower-http | 3 h |
| [ ] | [03 · Persistence](src/l03_persistence.rs) | 01 | sqlx (SQLite) | 3.5 h |
| [ ] | [04 · Authentication](src/l04_auth.rs) | 01, 02 | argon2, hmac, sha2 | 3 h |
| [ ] | [05 · Validation and API design](src/l05_api_design.rs) | 01 | axum, serde | 4 h |
| [ ] | [06 · CPU-bound work in a service](src/l06_cpu_work.rs) | 01 | rayon, tokio | 3 h |
| [ ] | 07 · A binary protocol: [part 1, the shared crate](wire/src/lib.rs), then [part 2, the connection](src/l07_protocol.rs) | 01 | crc32fast, tokio | 5 h |
| [ ] | [08 · Observability and operations](src/l08_operations.rs), with the [container lab](lab/README.md) | 02, 03, E04 | tracing, toml, axum | 4.5 h + 1.5 h lab |
| [ ] | [09 · Testing services](src/l09_testing.rs) | 01, 03, 08 | tower, sqlx | 3.5 h |
| [ ] | [C01 · Code review: the field-notes service](src/c01_code_review.rs) | 01 to 08 | all of the above | 1.5 h |
| [ ] | [Capstone · beacon](capstone/README.md) | everything | all of the above | 10 to 16 h |

The times include reading the concepts section. They are estimates, not measurements of learners; the whole module is roughly 45 to 50 hours. Lessons 02 to 07 need only lesson 01 (lesson 04 also uses 02), so you can change their order. Do 08 and 09 after them, then C01, then the capstone.

```sh
M=mastery/m11_services/Cargo.toml

cargo test --manifest-path $M -p m11_services -- --list        # every lesson test
cargo test --manifest-path $M -p m11_services l01_             # one lesson
cargo test --manifest-path $M -p m11_services --release l01_   # the same, optimised
cargo test --manifest-path $M -p beacon_wire                   # lesson 07, part 1
cargo test --manifest-path $M -p m11_services l07_             # lesson 07, part 2
cargo test --manifest-path $M --test l07_two_programs          # lesson 07, two real processes
cargo test --manifest-path $M --test l08_tracing               # lesson 08, task D
cargo test --manifest-path $M -p m11_services c01_             # the code review
cargo test --manifest-path $M -p beacon                        # the capstone
cargo test --manifest-path $M --doc                            # every code block in CONCEPTS.md
cargo clippy --manifest-path $M --all-targets
cargo test --manifest-path $M --no-fail-fast                   # everything
```

Lesson filters (`l01_` to `l09_`, `c01_`) match the module path of the unit tests. A filtered run also starts the other test binaries, which then report 0 tests. That is expected. Without `--no-fail-fast`, cargo stops at the first test binary that has a failure.

To try the two programs of lesson 07 by hand, start `cargo run --manifest-path $M --bin l07_recv` in one terminal, copy the address it prints, and run `cargo run --manifest-path $M --bin l07_send -- ADDRESS 5 100` in another.

## Topic map

| Topic | Where you practise it |
| --- | --- |
| Router, handlers, extractors, shared state | 01 |
| Typed errors mapped to responses | 01 (one enum), 05 (one shape for the whole API, including framework errors) |
| Timeouts, request ids, body limits, compression | 02 |
| Layer order; `from_fn` middleware; a hand-written `tower::Service` | 02 |
| Schema and embedded migrations | 03 |
| Bind parameters, row mapping, SQL injection | 03, C01 |
| Transactions and what happens on error | 03, 09 (fault injection) |
| Connection pools and their limits | 03, 08 (readiness) |
| N+1 queries, with a test that counts statements | 03 |
| Password hashing with Argon2id; parameters | 04 |
| Signed tokens, constant-time comparison, an auth extractor | 04 |
| Uniform login failures | 04, capstone |
| Validation at the boundary | 05 |
| Pagination, idempotency keys, API versions | 05 |
| Data parallelism with rayon: `par_iter`, `scope`, a custom pool | 06 |
| Bridging CPU work to async; a blocked runtime, detected by a test | 06, C01 |
| Bounded queues, load shedding, backpressure | 02 (shed), 06 (bounded pool), 07 (backpressure) |
| Framing, versioning, checksums, size limits | 07 part 1 |
| Compact encoding compared with JSON, measured in bytes | 07 part 1 |
| Partial reads, acknowledgements, stop-and-wait | 07 part 2 |
| Request spans and structured logs | 08, with its own test binary |
| Metrics and the exposition format; label cardinality | 08 |
| Liveness and readiness | 08 |
| Configuration from file and environment; secrets | 08, capstone |
| Graceful shutdown that drains requests | 08 |
| Container image, multi-stage build, what to check | 08 lab (by hand) |
| Handler tests without a socket; full-stack tests over loopback | 09 |
| Database fixtures, fault injection, tests that are tested | 09 |
| Reviewing a service for production hazards | C01 |
| Child-process tests on ephemeral ports | 07 (`l07_two_programs`), capstone |
| gRPC and message queues | [CONCEPTS.md, "At a glance"](CONCEPTS.md#at-a-glance-grpc-and-message-queues): overview and pointers only |

Not covered: TLS termination, OAuth and OpenID Connect, rate limiting, WebSockets and server-sent events, GraphQL, PostgreSQL-specific features, distributed tracing export (OpenTelemetry), deployment to an orchestrator. The security lessons state their own limits; read those paragraphs.

## Mastery check

Answer from memory, in [NOTES.md](NOTES.md), after the capstone. Model answers are at the end of [HINTS.md](HINTS.md#mastery-check--model-answers).

1. A request arrives at an axum router. Name the steps until a response is written, and say which steps can answer with an error before your handler runs. Why must the body extractor be the last parameter?
2. You write `.layer(a).layer(b)` on a router, and `ServiceBuilder::new().layer(a).layer(b)` elsewhere. Which layer is outermost in each? Where does a timeout layer go if timed-out responses must carry a request id?
3. A request future is dropped at an `.await`. Which of your code still runs? What follows for cleanup such as releasing a slot or decrementing a gauge?
4. A function begins a transaction, runs one `UPDATE`, and returns early with `?` from a second statement. What is in the database, and why?
5. All connections of a pool are in use. What happens to the next query, what should the client see, and which design mistakes produce this situation without real load?
6. Why must a password hash be slow, salted and self-describing, while a token MAC may be fast?
7. What makes a login endpoint leak which user names exist, and what do you do about each leak? What does a decoy hash not protect against?
8. Compare offset and keyset pagination when a row is deleted between two page requests.
9. How does an idempotency key make a retried `POST` safe? What must be atomic, and what happens when the key is reused with another body?
10. A handler computes for 200 ms without an `.await`. Describe the effect on a runtime with four worker threads under load, and two ways to fix it.
11. You receive bytes from a TCP connection. In what order do you check magic, version, length, checksum and payload contents, and what are the three possible answers of a frame decoder?
12. What is backpressure? Describe a receiver that has it and one that does not, and what happens to each when its consumer stalls.
13. Define liveness and readiness. Which of them may depend on the database? What is the order of events in a graceful shutdown?
14. Which behaviour of a service can a handler test without a socket verify, and for which do you need a socket or a child process?
15. Why does tokio's paused test clock give wrong results when real threads or real I/O take part?

## Crates and versions

Every direct dependency is pinned with `=` in the workspace `Cargo.toml`, and `Cargo.lock` fixes the rest. On 2026-11-20 each pin was the newest stable release on crates.io, and each crate had a release within the previous twelve months.

| Crate | Version | Used for | Why this one |
| --- | --- | --- | --- |
| axum | 0.8.9 | HTTP routing, extractors, serving | maintained by the tokio project; a thin layer over hyper and tower |
| tower | 0.5.3 | the `Service` and `Layer` traits, `ServiceBuilder` | the middleware model that axum, tonic and hyper-util share |
| tower-http | 0.7.1 | timeout, request id, body limit, gzip layers | the standard HTTP middleware collection for tower |
| tokio | 1.54.1 | runtime, sockets, channels, timers, signals | the runtime that the crates above require |
| sqlx | 0.9.1 | SQL with a pool, migrations, row mapping | async, plain SQL, embedded migrations; SQLite support compiles SQLite in |
| argon2 | 0.6.0 | password hashing (Argon2id) | pure Rust, from the RustCrypto project |
| hmac, sha2 | 0.13.0, 0.11.1 | HMAC-SHA-256 for tokens | RustCrypto; constant-time verification built in |
| rayon | 1.12.0 | data parallelism | the standard work-stealing pool |
| crc32fast | 1.5.5 | frame checksum | small and widely used |
| toml | 1.2.1 | configuration file | the format Cargo itself uses |
| serde, serde_json | 1.0.229, 1.0.154 | JSON bodies | as in the ecosystem track |
| thiserror | 2.0.21 | error enums | as in the ecosystem track |
| tracing, tracing-subscriber | 0.1.44, 0.3.23 | spans and log output | as in the ecosystem track |
| hyper-util | 0.1.21 | HTTP client in the supplied test helpers | already a dependency of axum, so it adds no crate |
| futures-core | 0.3.34 | two type aliases in the supplied `CountingDb` | already a dependency of sqlx |

Choices that were considered and not taken: `reqwest` as the test client (convenient, but many more crates for something only the supplied helpers use); `rusqlite` (simpler and synchronous, but it would hide the pool and the async transaction behaviour this module is about); the `metrics` crates and `tokio-util` (the lesson builds a registry and a frame reader by hand, and names the crates you would use instead); `uuid` request ids (a counter makes the tests exact; tower-http's `request-id` feature brings `uuid` in transitively anyway).

All 209 packages in `Cargo.lock` carry permissive licences: MIT, Apache-2.0, BSD-3-Clause, Zlib, 0BSD, BSL-1.0, Unlicense or Unicode-3.0, alone or as a choice. SQLite itself is in the public domain. Nothing in the tree links OpenSSL or needs a library installed on your system: SQLite is compiled from the source inside the `libsqlite3-sys` crate.

To upgrade a crate in your own project: read its changelog, run `cargo update -p NAME --precise VERSION`, look at `cargo tree --duplicates`, and run the whole test suite. sqlx 0.9, for example, stopped accepting non-literal SQL strings without `AssertSqlSafe`; lesson 03 shows the compiler message.

## Where the solutions are

Model solutions for every lesson, C01 and the capstone are in the course repository under `solutions/mastery/m11_services/`, at the same relative paths as the files they solve. A solution file is the exercise file with only the bodies, the predictions and (for C01) the reviewed code changed; the tests are identical. To run them, overlay them on a scratch copy of the module:

```sh
cp -R mastery/m11_services /tmp/m11_solved
cp -R solutions/mastery/m11_services/. /tmp/m11_solved/
cargo test --manifest-path /tmp/m11_solved/Cargo.toml --no-fail-fast
```

Do not copy them over your own work. Use them after your version passes, to compare designs, or when both hints were not enough.

## Validation scope

- The starters compile in debug and release with `--locked`, without compiler warnings. In the starting state, every lesson test fails at `todo!()` or `guess()`, except one C01 test (the demo, which works) and the self-tests of the supplied helpers. A full run of the starting state takes a few seconds and does not hang: every test has an outer time limit.
- With the solutions overlaid, all 168 tests (unit tests, the two process-level test binaries, the capstone's acceptance tests and the documentation tests) passed three times in debug and three times in release, then ten more times in each profile, and six times on a machine whose processor cores were all kept busy by other work. `cargo clippy --all-targets -- -D warnings` and `rustfmt --check` are clean on the solved copy.
- Every test listens only on `127.0.0.1` with a port chosen by the operating system, uses an in-memory or temporary-file database, and has a bounded wait. No test needs Docker or a network. Tests that involve only async code use tokio's paused clock; tests that involve SQLite or real sockets use real time, with waits on events instead of sleeps.
- Thirty plausible wrong solutions (a sequential loop where a parallel one was asked for, an acknowledgement sent too early, a query per row, a lock released between check and write, and so on) were each checked to fail the test that is aimed at them. Two of those tests are statistical and say so in a comment: `c_adjustments_are_atomic` in lesson 01 and `d_concurrent_retries_create_one_ticket` in lesson 05 make a race very likely, not certain. One technique cannot be observed from outside at all: whether the token MAC in lesson 04 is compared in constant time. Its test reads the source text of the lesson file for the `hmac` crate's check, and says so.
- The code blocks in CONCEPTS.md are compiled and run as documentation tests, and the compiler messages shown there were copied from the pinned toolchain.
- The container lab is done by hand, on your host (your own computer), not inside the course's dev container, which does not include Docker. Its Dockerfile is not built by any automatic check of this course, and it was not built when this module was written. Both base image tags it names existed on 2026-11-20.
