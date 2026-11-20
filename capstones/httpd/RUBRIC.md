# Rubric for the `httpd` capstone

The acceptance tests tell you whether the server behaves as SPEC.md says.
They cannot tell you whether the program is well built. This rubric is for
that part. Use it on your own code, with the code open, after all tests
pass.

Each row has three levels. "Working" is a good result. "Strong" is what
an experienced reviewer would be glad to see. Every level is described by
something you can observe: a command, a line you can point at, or a
question you can answer. Do not grade a feeling.

---

## 1. Correctness

| | not yet | working | strong |
|---|---|---|---|
| Acceptance tests | some fail, or pass only sometimes | all of them pass in debug and in release, three runs in a row each | you ran them twenty times in a loop while the machine was busy with something else, and no run failed |
| Hostile input | you can name a byte sequence that makes a worker panic or the process exit | no `unwrap`, `expect`, slice index or arithmetic in the request path can fail on client input; you checked each one | parsing functions return `Result` all the way; a search for `unwrap()` outside tests finds nothing you need to explain |
| Order of checks | statuses for requests with two faults differ from SPEC section 4 | the code follows the six steps in order | the six steps are visible in one function, in order, so that a reviewer can hold SPEC section 4 next to it and tick them off |
| Framing | a request with a body on `GET`, or two requests in one write, confuses the server | bytes after one request are kept for the next; a body is always consumed | there is exactly one place that knows where a request ends, and one buffer per connection that outlives the request |
| Time | a limit is implemented as "timeout per read" | every blocking read and write has a deadline; the request deadline is computed once per request | you can list every call in your server that can block (accept, read, write, lock, wait, join) and say what ends each one |

---

## 2. Design

| | not yet | working | strong |
|---|---|---|---|
| Module boundaries | one file, or modules that all know about `TcpStream` | parsing, path rules and JSON are separate from socket code | the head parser, the path resolver and the JSON scanner are functions from bytes to a value or an error: they take no socket, no clock and no file system, so they can be tested with plain values |
| Types | status codes are integers passed around; methods are strings | `enum`s for method, version and status; an error type per layer | an impossible state cannot be written down: a status without a reason phrase, a response without a `Connection` decision, a file body without its length |
| Limits | the numbers 8192, 64, 2048 appear inside the logic | every limit of the SPEC is a named constant | each limit is enforced where the bytes arrive, before memory is spent on them; you can show the line that stops a head at 8192 bytes and the line that refuses a body before reading it |
| The pool | the pool knows about HTTP or about sockets | the pool takes work items and runs a handler; it is tested without a network | the pool's whole contract is "at most `workers + queue` items inside", it has unit tests for exactly that, and the server decides what "full" means |
| Ownership of a connection | several threads hold handles to the same socket "just in case" | exactly one thread owns a connection at any time: the acceptor, then the queue, then one worker. (A second handle that exists for one stated purpose, such as waking a blocked read at shutdown, is fine if one place creates it and one place removes it.) | you can say, for each moment in the life of a connection, which value owns the `TcpStream` and what its drop does |
| Shutdown | `std::process::exit` from a helper thread | one flag, one ordered sequence in one place (stop accepting, announce, drop the queue, join the workers) | no thread is left to be killed by the exit of `main` except ones that are blocked in a call that cannot be interrupted, and you can name them |
| Response writing | status lines are built with `format!` in many places | one function writes every response; the four required headers cannot be forgotten | the same function serves `HEAD`, and the error responses are built from the status alone |

---

## 3. Idiom

| | not yet | working | strong |
|---|---|---|---|
| Bytes and text | the head is converted to `String` first (and bytes above 127 crash or are replaced) | the parser works on `&[u8]`; text is produced only where the grammar guarantees ASCII | slice patterns, `split`, `strip_prefix`, `trim_ascii` and `eq_ignore_ascii_case` do the work; there is no manual index arithmetic that could be off by one |
| Errors | `Box<dyn Error>` everywhere, or strings | small `enum`s; `?` carries them up; the mapping from error to status is one `match` | error types implement `Display` and `Error`; start-up errors say what was tried and why it failed |
| I/O traits | functions take `&mut TcpStream` | the response writer takes `impl Write`, so it is tested against a `Vec<u8>` | reading is behind a small type of your own that owns the buffer and the deadline logic |
| Locks | a `MutexGuard` is held across a read, a write, or the handler | every critical section is a few lines without I/O | you decided what a poisoned lock means in your pool and wrote the decision down in a comment |
| Clippy | warnings | `cargo clippy --all-targets -- -D warnings` is clean | clean without a single `#[allow]`, or each `#[allow]` has a reason next to it |
| Comments | none, or comments that repeat the code | every module says what it is for | comments explain decisions and name the SPEC section: why this order, why this limit is checked here, what would break |

---

## 4. Tests of your own

| | not yet | working | strong |
|---|---|---|---|
| Unit tests | none: only the acceptance tests | the parser, the path rules, the JSON scanner and the date have unit tests | the tables in your unit tests include the cases that cost you time, with a name that says what went wrong |
| The pool | tested only through the server | a unit test fills the pool exactly and checks that the next item is refused | the tests hold workers busy with a signal they control, not with `sleep`; a test shows that a panicking handler costs neither a worker nor a slot |
| Edges | only typical values | each limit is tested at the limit and one above it | the incremental cases are tested without a socket: the end of the head split at every position, a multi-byte character split across two reads |
| Speed | the unit tests need seconds | `cargo test --bin httpd` runs in well under a second | no unit test opens a socket, sleeps, or touches the file system outside a temporary directory |

---

## 5. Performance

Measure before you change anything. A loop of 10 000 requests on one
connection and a stopwatch is enough to start.

| | not yet | working | strong |
|---|---|---|---|
| Latency of a small response | about 40 ms per request on Linux (two small writes and Nagle's algorithm) | a small response leaves in one write, or `TCP_NODELAY` is set and you know why | you measured requests per second on one keep-alive connection in a release build and can explain the number |
| Memory per connection | a body or file is read into memory completely before anything is checked | files are streamed; the body buffer grows as bytes arrive, never from the announced length alone | you can state the largest amount of memory one connection can make the server hold, as a formula of the limits, and the worst case for all workers together |
| Allocation per request | many small `String`s per header, per request | a handful of allocations per request | you counted them (the counting allocator from the course's support crate is a model) and removed the ones in the hot path, or you decided with numbers that it does not matter |
| Files | `fs::read` and then `write_all` | `io::copy` or a fixed buffer | you know which system call your platform uses for `io::copy` from a file to a socket, and you checked that a write timeout still applies to it |
| Idle cost | a busy loop, or a sleep, while waiting for bytes | idle connections block in the kernel | you know how many times per second an idle connection wakes a thread in your design, and what it would take to make that zero |
| Under overload | the server slows down for everybody, or memory grows | the 503 path does no allocation that depends on the client, never blocks the acceptor, and the queue never grows past its bound | you ran more clients than `workers + queue`, watched refusals and latency, and found the load at which the queue starts to fill |

---

## 6. Unsafe hygiene

The expected amount of `unsafe` in this project is zero.

| | not yet | working | strong |
|---|---|---|---|
| The base project | `unsafe` blocks, or `unsafe_code = "forbid"` was removed from `Cargo.toml` to make something compile | no `unsafe` anywhere; the lint is still `forbid` | you can say which standard-library types do the unsafe work for you here (`Mutex`, `Arc`, `Vec`) and what each one guarantees |
| `Send` and `Sync` | `unsafe impl Send` to get a value into a thread | every type that crosses a thread is `Send` because its fields are | you can explain why your handler closure is `Send + Sync + 'static` by listing what it captures |
| Shared flags | a `static mut` | an atomic or a `Mutex` behind an `Arc` | you chose each memory ordering on purpose and wrote one sentence about why a weaker one is (or is not) enough |
| Stretch goals that need FFI (signals) | `unsafe` spread over the server | all `unsafe` is in one small module behind a safe function | each `unsafe` block has a `SAFETY` comment that names the rule it relies on; the handler does only what is allowed in a signal handler; the lint is `deny` with one `#[allow]` on that module |

---

## 7. Robustness you can demonstrate

Do each of these by hand once. Each takes a minute.

| experiment | what a good server does |
|---|---|
| Connect with a raw socket tool, type `GET / HTTP/1.1` and press Enter twice | answers 400 at once (your terminal sends `LF`), not 408 after five seconds |
| Connect and send nothing | the connection is closed after the idle timeout, without a response |
| Start with `--workers 1 --queue 0`, open one connection, then a second | the second gets 503 immediately |
| Request a file of a few megabytes with `curl --limit-rate 500k` and type `shutdown` while it downloads | the download finishes and the file is complete; then the process exits with status 0 |
| `curl -T bigfile http://127.0.0.1:8080/echo` with a file larger than `--max-body` | `curl` prints the 413, not "connection reset" |
| Start the server with `< /dev/null` (or `< NUL` on Windows) | it prints both lines and exits with status 0 |
| Press Ctrl-C | the process dies at once; you can explain why, and what would be needed to change it |

---

## 8. Reflection

Write a short `DESIGN.md` for your version (the model solution has one).
Answer at least these:

1. How does a worker that is blocked in `read` learn that the server is
   shutting down? What does your answer cost while nothing is shutting
   down?
2. What exactly does your server count to decide that it is full, and
   where is that number increased and decreased? Can the two ever get out
   of step?
3. Which bug cost you the most time? What was the real cause, and which
   test (yours or the course's) would have found it in a minute?
4. Where does your server copy bytes that it would not have to copy?
   Would removing the copy make the code better?
5. If the server had to serve 10 000 idle connections, what would break
   first? What would you change, and what would stay?

If you can answer all five in a few sentences each, you understood the
project, whatever the tables above say.
