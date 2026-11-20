# Capstone · beacon, a small telemetry system

[Module guide](../README.md) · [Concepts](../CONCEPTS.md#capstone--beacon) · [Hints](../HINTS.md#capstone--beacon) · [Acceptance tests](tests/acceptance.rs)

You build two programs that work together:

- **`beacon-agent`** runs next to something you want to observe. It takes a sample at a fixed interval and ships the samples to the server over the binary protocol from lesson 07. The network between them is unreliable, so the agent buffers, reconnects and resends.
- **`beacon-server`** accepts samples on a TCP port, stores them in SQLite, and answers queries over an HTTP API that requires a login.

```text
 beacon-agent                         beacon-server
┌───────────────────────┐            ┌──────────────────────────────────────┐
│ sampler ──► bounded   │  beacon    │ ingest ──► SQLite ◄── HTTP API       │
│             buffer ──►│──wire/TCP─►│ (port 1)             (port 2, login) │
│   shipper ◄───────────│◄── Ack ────│                                      │
└───────────────────────┘            └──────────────────────────────────────┘
```

This is a design exercise. The lessons gave you signatures; the capstone gives you a contract and nine acceptance tests that start your programs as separate processes. There is no starter code beyond two `main` functions that say `todo!()`. Which modules exist, which types they use and how errors travel is your decision.

Estimated time: 10 to 16 hours.

## Rules

- Work in `capstone/src/`. Add files and modules as you like. You may add `capstone/migrations/` and unit tests.
- Do not edit `tests/acceptance.rs` or `tests/support/`. Add your own test files next to them.
- Use only the crates that `capstone/Cargo.toml` already lists. That list is enough.
- The protocol is your `beacon_wire` crate from lesson 07, unchanged. The acceptance tests speak it too.
- Run the tests with:

  ```sh
  cargo test --manifest-path mastery/m11_services/Cargo.toml -p beacon
  cargo test --manifest-path mastery/m11_services/Cargo.toml -p beacon --test acceptance s1_
  cargo test --manifest-path mastery/m11_services/Cargo.toml -p beacon --release
  ```

## The contract

Everything below is checked from the outside. "Print" means standard output, one line, flushed at once. Diagnostics and logs go to standard error.

### Both programs

- Configuration comes from environment variables that start with `BEACON_`.
- A missing or unusable value ends the program with **exit code 78** and a message on standard error that names the variable. The message never contains the value of a secret.

### `beacon-server`

| Variable | Default | Meaning |
| --- | --- | --- |
| `BEACON_INGEST_ADDR` | `127.0.0.1:7400` | address of the ingest (protocol) listener |
| `BEACON_HTTP_ADDR` | `127.0.0.1:7401` | address of the HTTP listener |
| `BEACON_DB` | `sqlite://beacon.db` | sqlx database URL; the file is created if missing |
| `BEACON_ADMIN_USER` | none, required | the one user who may log in |
| `BEACON_ADMIN_PASSWORD` | none, required | that user's password |
| `BEACON_TOKEN_KEY` | none, required | at least 32 bytes; secret key material for login tokens or sessions |

**Start-up.** When the schema is ready and both listeners are bound, the server prints two lines, in this order: `ingest=ADDR` and `http=ADDR`, with the addresses it really listens on (port 0 in the configuration means "let the system choose").

**Ingest port.** One connection carries `Hello { agent }` first and then any number of `Batch` messages.

- For each batch the server stores the samples durably and then answers `Ack { seq }` with the batch's `seq`. "Durably" means: if the server is killed right after the `Ack` left, the samples are there when it starts again with the same database.
- A sample is identified by agent, metric and `ts_ms`. A sample that arrives again is not stored twice, and its batch is acknowledged like any other.
- A batch that cannot be stored gets no `Ack`, and the server closes that connection. This covers a database error, and also a `ts_ms` that your schema cannot hold: the protocol carries unsigned 64-bit times, and an SQLite integer is signed.
- Bytes that are not a valid frame, a first message that is not `Hello`, or any message other than `Batch` afterwards: the server closes that connection without an `Ack`. Other connections and the HTTP API are not affected.

**HTTP API.** All error bodies are JSON `{"error": "<code>"}`.

| Request | Answer |
| --- | --- |
| `GET /healthz` | 200 |
| `POST /v1/login` with `{"user", "password"}` | 200 `{"token": "..."}`; with wrong credentials 401 `invalid_credentials`. An unknown user and a wrong password get exactly the same response. A body that is not JSON of this shape is a client error: 400, 415 or 422 with a code of your choice. |
| `GET /v1/agents/{agent}/metrics/{metric}/samples` | needs `Authorization: Bearer <token>`; without a valid token 401 `unauthorized` |
| any other path | 404 `not_found` |

The samples query answers 200 with

```json
{"samples": [{"ts_ms": 1800000000000, "value": 5}], "next_after": 1800000000000}
```

- Samples are ordered by `ts_ms`, oldest first. A series without data is an empty list, not an error.
- `?limit=N` sets the page size: default 100, allowed 1 to 1000.
- `?after=T` returns only samples with `ts_ms` greater than `T`.
- `next_after` is the `ts_ms` of the last sample on the page when more samples exist, and `null` on the last page. The client sends it back as `after`.
- A `limit` or `after` that is not acceptable: 400 `invalid_query`.

**Shutdown.** On SIGTERM or Ctrl-C the server stops accepting, finishes requests in progress within a bounded time, and exits with code 0. (No acceptance test sends a signal, because that cannot be done portably. Check it by hand, and again in the container lab.)

### `beacon-agent`

| Variable | Default | Meaning |
| --- | --- | --- |
| `BEACON_SERVER` | none, required | address of the server's ingest listener |
| `BEACON_AGENT_ID` | none, required | the name sent in `Hello` |
| `BEACON_METRIC` | `counter` | the metric name sent in every batch |
| `BEACON_SOURCE` | `counter` | what to sample; `counter` gives the values 0, 1, 2, ... |
| `BEACON_INTERVAL_MS` | `1000` | time between two samples |
| `BEACON_COUNT` | none: run until stopped | take this many samples, deliver them, then exit |
| `BEACON_BUFFER` | `1000` | how many unacknowledged samples the agent keeps at most |
| `BEACON_BATCH` | `50` | how many samples one batch carries at most |

- **Samples.** `ts_ms` is the time of the sample in milliseconds since the Unix epoch. Within one run it increases strictly from sample to sample, also when two samples fall into the same millisecond.
- **Sampling never waits for the network.** Samples are taken at the configured interval whether the server is reachable, slow, or silent.
- **The buffer is bounded.** A sample stays in the buffer until the server has acknowledged a batch that contains it. Samples that were sent but not yet acknowledged count against `BEACON_BUFFER`. When a new sample arrives and the buffer is full, the oldest sample is discarded and counted as dropped.
- **Delivery.** Each connection starts with `Hello`. The agent keeps one connection open and uses it for every batch for as long as it works; it does not open a connection per batch. Samples are sent in batches, oldest first, at most `BEACON_BATCH` per batch. When a connection fails or no `Ack` arrives, the agent connects again and sends what is in the buffer; nothing that is still buffered may be lost. How long the agent waits for an `Ack` before it gives up on a connection is your decision: choose a limit of a few seconds. The first retry comes within 200 ms; later retries may wait longer, up to 2 seconds.
- **Finishing.** With `BEACON_COUNT=N`: when the N-th sample has been taken, the agent prints `sampled=N`. When the buffer is empty after that, it prints `shipped=X dropped=Y` and exits with code 0. X counts samples removed from the buffer because they were acknowledged, Y counts samples discarded from a full buffer, and X + Y = N.

## A path through the work

The acceptance tests are named so that you can go one at a time. Each milestone gives less help than the one before.

**Milestone 1 · A server that starts (`s1`, `s2`).** Suggested steps: a function from "environment as pairs" to a validated configuration struct (lesson 08, task A); the listeners; the two printed lines; the router with `/healthz`, the 404 fallback and the JSON error type (lessons 01 and 05); the login and the extractor (lesson 04). Decide early where the password hash is computed and on which thread.

**Milestone 2 · Storing (`s3`, `s4`).** A schema in a migration, one function that stores a batch and one that reads a page. Questions to answer before you write code: What is the primary key? Where does the transaction begin and end, relative to the `Ack`? What happens to a connection task when the database returns an error?

**Milestone 3 · An agent on a good day (`a1`, `a2`).** Sampler, buffer, shipper. Draw the two tasks and the data between them first.

**Milestone 4 · An agent on a bad day (`a3`, `a4`).** No steps are given. The two tests describe the two failures.

**Milestone 5 · Together (`e1`).** If milestones 1 to 4 are right, this passes without new code.

## Before you call it done

The tests cannot see everything. Review your own work as you reviewed the pull request in C01:

- [ ] `cargo clippy --manifest-path mastery/m11_services/Cargo.toml -p beacon --all-targets` is clean.
- [ ] The tests pass three times in a row, in debug and with `--release`.
- [ ] No `unwrap()` or `expect()` can be reached by bytes from the network or by a database error.
- [ ] Every wait on the network has a timeout, and every queue or buffer has a bound. List them. Writes count: a peer that stops reading makes your `write` wait.
- [ ] Every message the agent can build fits in one frame (64 KiB of payload), whatever `BEACON_BATCH`, `BEACON_AGENT_ID` and `BEACON_METRIC` say. A batch that cannot be encoded can never be delivered, and the agent would retry it for ever.
- [ ] No lock is held across an `.await`.
- [ ] Password hashing does not run on an async worker thread, and only a few hashes can run at the same time.
- [ ] Token MACs are compared with the `hmac` crate's constant-time check, not with `==`.
- [ ] Nothing secret is printed: not in errors, not in logs, not in `Debug` output.
- [ ] The number of ingest connections is limited.
- [ ] Ctrl-C stops the server cleanly while an agent is connected.
- [ ] The `main` functions are short, and the logic has unit tests of its own.

Then answer in your notes: What does an `Ack` promise in your implementation? Under which failure does your agent lose a sample, and how would an operator notice? What would you have to change to run two servers behind a load balancer?

## Going further

- Add a second source, for example the agent's own uptime, and let one agent ship two metrics.
- Add `/metrics` and `/readyz` to the server with the pieces from lesson 08.
- Split the server into two programs that share the database file. What does SQLite need for that, and which test becomes harder?
- Let the agent send several batches before the first `Ack` arrives (a window). What must the server guarantee about the order of its `Ack`s?
- Do the [container lab](../lab/README.md) with your server.

The reference solution is in `solutions/mastery/m11_services/capstone/` in the course repository. Read it after your own version passes; it is one design among several.
