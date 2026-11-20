# Capstone: kvstore, a key-value store that survives crashes

[All chapters](../../README.md) · [Specification](SPEC.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Rubric](RUBRIC.md)

You write a server that stores keys and values and does not lose them: not
when it is stopped, not when it is killed, and not when the kill comes in
the middle of a write. Clients talk to it over TCP with one-line commands.

Underneath is the design that many real storage engines start from: an
append-only log of checksummed records, an index in memory, recovery that
replays the log and repairs its end, and compaction that replaces the log
without ever leaving half of one behind.

This is a **blank-file project**. There are no lesson files, no types and
no function signatures. You get a precise specification and 72 acceptance
tests that start your binary, talk to it, kill it, damage its files, and
start it again. Every design decision inside the program is yours.

| file | what it is |
|---|---|
| [SPEC.md](SPEC.md) | the complete specification: read all of it first |
| [CONCEPTS.md](CONCEPTS.md) | the ideas you need, with small compiled examples |
| [HINTS.md](HINTS.md) | design hints per milestone; no code |
| [RUBRIC.md](RUBRIC.md) | how to judge your own result |
| `src/main.rs` | a stub for the binary; it prints one line and exits |
| `src/lib.rs` | an empty library, as a place for your modules |
| `tests/` | the acceptance tests, one file per milestone |

The model solution, with its own `DESIGN.md`, is in
`solutions/capstones/kvstore/` of the course repository. Look at it after
you have finished, or when one problem has kept you stuck for a long time.

## Prerequisites

| from | you use it for |
|---|---|
| [Chapter 03](../../exercises/chapter03/README.md), Numbers, text and arrays | fixed-width integers, `to_le_bytes` and `from_le_bytes`, byte slices, shifts and XOR for the checksum |
| Chapters [04](../../exercises/chapter04/README.md) and [05](../../exercises/chapter05/README.md), Ownership and moves; Borrowing with references | who owns the log, the index and each connection; what a request borrows from its line |
| [Chapter 07](../../exercises/chapter07/README.md), Handling errors | your own error types, `?`, turning errors into exit codes, deciding which errors end a connection and which end the server |
| [Chapter 08](../../exercises/chapter08/README.md), Modules, crates and Cargo | a library plus a binary; module boundaries you draw yourself |
| Chapters [09](../../exercises/chapter09/README.md) and [10](../../exercises/chapter10/README.md), Structs and methods; Enums and pattern matching | commands, responses, records, parser states |
| [Chapter 11](../../exercises/chapter11/README.md), Generic code with traits | writing against `Read`, `Write` and `BufRead` so that the parser can be tested without a socket |
| [Chapter 16](../../exercises/chapter16/README.md), Standard collections | `HashMap` or `BTreeMap` for the index, and why the choice matters here |
| [Chapter 17](../../exercises/chapter17/README.md), Working with text | bytes that are not text: `&[u8]`, byte-string literals, why `String` is the wrong type for a value |
| [Chapter 18](../../exercises/chapter18/README.md), Files, streams and processes | `File`, `OpenOptions`, seeking, `sync_data`, buffered and unbuffered I/O, `TcpListener` and `TcpStream`, `std::process::Command` in the tests |
| [Chapter 19](../../exercises/chapter19/README.md), Threads and shared state | threads, `Mutex` and `RwLock`, atomics, scoped threads, what a data race on a file looks like |
| Mastery [`m3_concurrency`](../../mastery/m3_concurrency/README.md) | reasoning about orderings and lock scope; why "append, then update the index" must be one critical section |
| Mastery [`m8_api`](../../mastery/m8_api/README.md) | small, hard-to-misuse internal APIs: types that make an unvalidated key impossible to pass around |

Helpful but not required: mastery [`m7_perf`](../../mastery/m7_perf/README.md) for measuring throughput, and
mastery [`m4_async`](../../mastery/m4_async/README.md) if you want to try the async stretch goal.

You do **not** need any knowledge of databases or storage. CONCEPTS.md
explains logs, checksums, torn writes, syncing, atomic renames and
compaction from the beginning.

Time: as a rough estimate, plan 25 to 45 hours: M1 5 to 8, M2 4 to 7, M3 4
to 7, M4 4 to 7, M5 4 to 8, M6 3 to 6. These are estimates, not targets.
The design work between the milestones is part of the time and is the
point of the exercise.

## Setting up

The package stands alone. Work inside `capstones/kvstore/`; it has its own
`Cargo.toml`, `Cargo.lock` and `rust-toolchain.toml` (Rust 1.99.0, the same
as the rest of the course). There are no dependencies and nothing to
install.

```text
cargo build
cargo test --test m1_protocol
```

Every test fails at first, with a message that shows the line the starter
prints. That is the starting line.

## How to work

1. Read SPEC.md from start to end once. It is long because it is complete;
   you will come back to single sections many times. Then read
   CONCEPTS.md.
2. **Design before you type.** Take paper. Which parts does the program
   have? Who owns the file, who owns the map, which thread may touch what?
   What are the two or three types that everything else is built on? You
   will change your mind later, and that is fine; a first design gives you
   something to change.
3. Work through the milestones in order (SPEC section 15). Each has its own
   test file:

   ```text
   cargo test --test m1_protocol
   cargo test --test m2_log
   cargo test --test m3_recovery
   cargo test --test m4_compaction
   cargo test --test m5_concurrency
   cargo test --test m6_shutdown
   ```

4. Write your own unit tests as you go. The acceptance tests can only see
   the outside of the program. A checksum function, a record encoder, a
   line parser are all much easier to get right with ten small tests next
   to them than through a socket.
5. When all tests pass, go through RUBRIC.md, and only then read the model
   solution.

Things that help:

- **Read the failing test.** Each assertion has a message that names the
  SPEC section it checks. The helpers in `tests/common/mod.rs` are short
  and commented; the file also contains a second, independent
  implementation of the log format that you can compare yours against.
- **Look at the files.** When a test fails, it keeps its data directory
  and prints the path. A hex dump shows what your server really wrote:

  ```text
  xxd target/tmp/kvstore-m2_example-12345-0/kv.log          # Linux, macOS
  Format-Hex target\tmp\kvstore-m2_example-12345-0\kv.log   # Windows PowerShell
  ```

- **A test never hangs.** Every wait in the tests has a limit of 10
  seconds. If your server does not answer, the test fails with a message
  that says what it was waiting for. On a very slow machine, raise the
  limit: `KVSTORE_TEST_TIMEOUT_SECS=30 cargo test`.
- **No server is left behind.** The tests kill every server they started,
  also when a test fails. If you interrupt `cargo test` itself with Ctrl-C,
  check for leftover `kvstore` processes.
- **Run the concurrency tests many times.** A race does not show on every
  run. Before you call M5 or M6 done:

  ```text
  for i in 1 2 3 4 5 6 7 8 9 10; do cargo test --test m5_concurrency || break; done
  ```

  (In PowerShell: `1..10 | % { cargo test --test m5_concurrency; if ($LASTEXITCODE) { break } }`.)
- Keep `cargo clippy --all-targets` clean as you go.

## Running the server by hand

```text
cargo run -- --dir data --port 7878
```

The server prints `LISTENING 127.0.0.1:7878` and waits. Talk to it from a
second terminal with any tool that opens a TCP connection and sends lines
that end in a line feed:

```text
nc 127.0.0.1 7878            # Linux, macOS (netcat)
ncat 127.0.0.1 7878          # where Nmap is installed, also on Windows
```

```text
PING
PONG
SET fruit apple
OK
GET fruit
VALUE apple
```

(`PING`, `SET ...` and `GET ...` are what you type; the other lines are the
answers.)

Do not use `telnet`: it ends lines with a carriage return and a line feed,
and the protocol treats the carriage return as an ordinary byte (SPEC 8.1).
If you have no such tool, write a ten-line client with `TcpStream`; you
have done that in Chapter 18.

Try the things the project is about:

- Stop the server with Ctrl-C in the middle of a session and start it
  again. Your data is there. Ctrl-C is a crash, and a crash is safe.
- Stop it, cut a few bytes off the end of `data/kv.log`, and start it
  again. Look at what it prints on standard error.
- Start a second server on the same directory.
- Send `STATS`, overwrite one key a thousand times, send `STATS`, send
  `COMPACT`, send `STATS`.

The directory `data/` is listed in `.gitignore`.

## How to assess your work

1. `cargo test` passes in debug and with `--release`, ten times in a row.
2. `cargo clippy --all-targets` prints no warnings.
3. `grep -rn "unsafe" src/` finds at most a `#![forbid(unsafe_code)]` line.
4. `grep -rn "unwrap()\|expect(" src/` finds nothing outside tests that you
   cannot defend in one sentence.
5. You can answer these without looking at your code:
   - Which single line of your program makes a write survive a kill? Which
     one makes it survive a power failure?
   - At which exact moment does a compaction take effect?
   - What does a client see if the server is killed between writing a
     record and sending `OK`? Is that allowed?
   - Which thread is blocked where when the server is idle, and what wakes
     each of them at shutdown?
6. Go through [RUBRIC.md](RUBRIC.md). It covers what the tests cannot see:
   syncing, memory bounds, error handling, the structure of the code.
7. Compare with the model solution and read its `DESIGN.md`. Different is
   fine. Look for decisions you made without noticing that you were making
   one.

## Licence

The code, the specification, the file format and the protocol in this
package are original work for this course and are covered by the MIT
licence of the course repository.
