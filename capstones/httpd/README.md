# Capstone: `httpd`, a web server with its own thread pool

[All chapters](../../README.md) · [Specification](SPEC.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Rubric](RUBRIC.md)

You write an HTTP/1.1 server from an empty `main`. It serves the files of a
directory, and it has one small API endpoint that checks a JSON document
and sends it back in compact form. You use the standard library and
nothing else: no HTTP crate, no JSON crate, no thread-pool crate, no async
runtime.

The two features are small on purpose. The project is everything around
them:

- a request parser that reads bytes from a stranger and has a limit for
  every one of them;
- connections that stay open, requests that arrive one byte at a time and
  requests that arrive five at once;
- protection against the oldest attack on file servers, `GET /../../secret`,
  in all its disguises;
- time limits, so that a client that says nothing, or never reads, cannot
  keep a thread;
- a fixed pool of worker threads with a bounded queue, and an immediate
  `503` when both are full;
- a shutdown that finishes the requests in progress and then exits with
  status 0.

When you are done, this works:

```text
$ cargo run --release -- --root site --addr 127.0.0.1:8080
listening on 127.0.0.1:8080
```

```text
$ curl -i http://127.0.0.1:8080/hello.txt
HTTP/1.1 200 OK
Date: Fri, 20 Nov 2026 10:42:28 GMT
Content-Type: text/plain; charset=utf-8
Content-Length: 6
Connection: keep-alive

hello

$ curl -H 'Content-Type: application/json' -d '{ "a" : [1, 2] }' http://127.0.0.1:8080/echo
{"a":[1,2]}

$ curl -s -o /dev/null -w '%{http_code}\n' --path-as-is http://127.0.0.1:8080/../secret
403
```

This is a **blank-file project**. There are no lesson files, no types and
no function signatures. `src/main.rs` prints "not implemented" and exits.
You decide the modules, the types, the error handling and the threads.

| file | what it is |
|---|---|
| [SPEC.md](SPEC.md) | the complete specification: read all of it first |
| [CONCEPTS.md](CONCEPTS.md) | the ideas behind the project, with small compiled examples that are not the server |
| [HINTS.md](HINTS.md) | design hints per milestone; no code |
| [RUBRIC.md](RUBRIC.md) | how to judge your own result, including what tests cannot see |
| `src/main.rs` | the starter: a `main` that does nothing useful |
| `tests/` | 86 acceptance tests in six files, one per milestone (one of the 86 exists on Unix only) |
| `tests/common/mod.rs` | the test tools: a temporary site, the child process, a small HTTP client |

The model solution, with a `DESIGN.md` that explains its decisions, is in
`solutions/capstones/httpd/` of the course repository. Look at it after
you have finished, or when one problem has kept you stuck for a long time.

## Prerequisites

| from | you use it for |
|---|---|
| [Chapter 7, Handling errors](../../exercises/chapter07/) | error types for the parser and the start-up; `main` and exit codes |
| [Chapter 8, Modules, crates and Cargo](../../exercises/chapter08/) | splitting a binary into modules |
| [Chapters 9 and 10, Structs and methods; Enums and pattern matching](../../exercises/chapter10/) | requests, responses and statuses as types; slice patterns in the parser |
| [Chapter 13, Everyday traits](../../exercises/chapter13/) | `Drop` for clean-up that must not be forgotten |
| [Chapter 14, Closures and captures](../../exercises/chapter14/) | handing work to a pool: `Fn`, `Send`, `'static` |
| [Chapters 15 and 16, Iterators and adapters; Standard collections](../../exercises/chapter16/) | splitting bytes, `Vec<u8>` as a buffer, `VecDeque` as a queue |
| [Chapter 17, Working with text](../../exercises/chapter17/) | bytes versus text, UTF-8 validation, ASCII rules |
| [Chapter 18, Files, streams and processes](../../exercises/chapter18/) | `Read` and `Write`, files, `TcpListener` and `TcpStream`, partial reads |
| [Chapter 19, Threads and shared state](../../exercises/chapter19/) | `thread::spawn`, `Arc`, `Mutex`, `Condvar`, channels, atomics |
| Mastery module [`m3_concurrency`](../../mastery/m3_concurrency/README.md) | what a queue between threads must guarantee; lost wake-ups; choosing a memory ordering |
| Mastery module [`m7_perf`](../../mastery/m7_perf/README.md) | measuring before optimising, for the performance part of the rubric |

Helpful, not required: mastery module [`m8_api`](../../mastery/m8_api/README.md) for the boundaries between
your modules, and [`m4_async`](../../mastery/m4_async/README.md) afterwards, which builds a server of the
other kind (one thread, many connections).

You do **not** need to know HTTP. SPEC.md defines every byte this server
reads and writes, and [CONCEPTS.md](CONCEPTS.md) explains the ideas from
networking that the chapters do not cover: framing, deadlines, how a
connection ends, back-pressure, and how blocked threads are stopped. You also do not need Chapters 20 and 21: this server
uses blocking I/O and threads, and that is a decision, not a shortcut.
Seeing exactly where a thread per connection becomes expensive is one of
the things the project teaches.

Time: as a rough estimate, plan 25 to 40 hours. M1: 3 to 4, M2: 5 to 7,
M3: 3 to 5, M4: 5 to 8, M5: 4 to 6, M6: 6 to 10. These are estimates, not
targets.

## Setting up

The package stands alone. Work inside `capstones/httpd/`; it has its own
`Cargo.toml`, `Cargo.lock` and `rust-toolchain.toml` (Rust 1.99.0, the same
as the rest of the course). It has no dependencies, so nothing is
downloaded.

```text
cd capstones/httpd
cargo test --no-run        # compiles the starter and the tests
cargo test --test m1_startup
```

In the starting state every test fails, and all but one with the same
message:

```text
the server exited (exit status: 2) without printing `listening on 127.0.0.1:PORT` (SPEC 3.3).
  command: httpd --root <site>/public --addr 127.0.0.1:0
  standard error of the server (last lines):
    httpd: not implemented yet. Read SPEC.md and start with milestone M1.
```

That is correct. It is where you start. (The exception is the test for
`--help`, which does not start a server; it reports the exit status 2.)

The course commands `cargo drill` and `cargo progress` belong to the root
package and do not work inside this directory.

## How to work

1. Read [SPEC.md](SPEC.md) completely, once, before you write code. It is
   long because it is exact. Section 13 lists the milestones and which
   tests belong to each.
2. Take the milestones in order: `cargo test --test m1_startup`, then
   `m2_parser`, and so on. Until M6 a server that handles one connection
   at a time is enough. Build the protocol first and the threads last.
   Before each milestone, read the sections of [CONCEPTS.md](CONCEPTS.md)
   that its table names for that milestone.
3. Read a failing test before you guess. Every assertion has a message
   that names the section of the SPEC it checks.
4. Try the server by hand as early as M1. It is a real server:

   ```text
   mkdir site
   echo hello > site/hello.txt
   cargo run -- --root site
   ```

   Then, in a second terminal, use `curl -i http://127.0.0.1:8080/hello.txt`
   or a browser. To send exactly the bytes you want, including wrong ones,
   use a tool that writes raw bytes to a socket (`nc`, `ncat`, or ten
   lines of Rust with `TcpStream`). Type `shutdown` and Enter in the first
   terminal to stop the server.
5. Write unit tests of your own for the parts that need no network: the
   head parser, the path rules, the JSON scanner, the date, the pool. The
   acceptance tests tell you that something is wrong; your unit tests
   tell you where.

When a test fails, the message shows the last lines your server wrote to
standard error. Log there freely while you debug.

### What the tests do, and what they never do

The tests start your binary as a child process with
`--addr 127.0.0.1:0`, read the address from its first output line, and
talk to it over TCP on the loopback interface. Each test has its own
server on its own port and its own temporary directory, so the tests run
in parallel.

Every read, write and wait in the tests has a time limit. A server that
never answers makes tests fail after a few seconds; it does not make them
hang. When a test ends, also by failing, it kills its server process and
removes its files. If you interrupt `cargo test` itself, a correct server
still stops, because its standard input ends (SPEC 11.1); before you have
built M6, check for leftover `httpd` processes after an interrupted run.
An interrupted run can also leave directories named `httpd-test-...` in
the temporary directory of your system. One of them may hold a file of
32 MiB. They are safe to delete.

The tests need no network access beyond `127.0.0.1`, no fixed port, no
shell and no tool except the binary they test. They run on Linux, macOS
and Windows.

## How to assess your work

1. `cargo test` passes in debug and with `--release`, three times in a
   row. Tests about threads and time that pass once prove little.
2. `cargo clippy --all-targets -- -D warnings` prints nothing.
3. `grep -rn "unsafe" src/` finds nothing, and `Cargo.toml` still forbids
   `unsafe_code`.
4. `grep -rnE "unwrap\(\)|expect\(|panic!|\[[a-z_]+\]" src/`: look at
   every hit outside your tests and ask: can bytes from a client reach
   this line and make it fail? A server must have no such line.
5. You have used the server with a real browser and with `curl`, and you
   have sent it garbage by hand.
6. Go through [RUBRIC.md](RUBRIC.md). It covers what the tests cannot
   see: the structure of the code, where the limits live, what each
   thread owns, and what happens to memory under load.
7. Compare with the model solution and read its `DESIGN.md`. Different is
   fine. Look for decisions that you made without noticing that you were
   making one.

## A word on safety

This server is a learning project. Its path rules are strict and tested,
but it has no TLS, no access control and no audit behind it. Keep it on
`127.0.0.1`, which is the default, and do not point `--root` at a
directory with private files.

## Licence

The code, the specification and the tests in this package are original
work for this course and are covered by the MIT licence of the course
repository.
