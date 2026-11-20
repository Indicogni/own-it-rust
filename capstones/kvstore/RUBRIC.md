# Rubric for the kvstore capstone

The tests tell you whether the observable behaviour is right. This rubric
is for what a test cannot see: whether a write is really on the disk, how
much memory a hostile client can make you use, what happens when the disk
fails, and whether another person could maintain the code. Go through it
when all tests pass, and be honest: the point is to find what to improve,
not to collect a grade.

Each row has three levels. "Working" is a finished capstone. "Strong" is
what a careful professional would hand in. Most first attempts are a mix.
Every level is written so that you can check it: by reading your code, by
running a command, or by doing a small experiment.

## 1. Gate: the things that are simply true or false

All of these must hold before the rest matters.

- [ ] `cargo test` passes in debug and with `--release`.
- [ ] `cargo test --test m5_concurrency` and `cargo test --test m6_shutdown`
      pass ten times in a row, in both profiles.
- [ ] `cargo clippy --all-targets` prints no warnings, and you added no
      `#[allow(...)]` to get there.
- [ ] `grep -rn "unsafe" src/` finds at most a `#![forbid(unsafe_code)]`
      line.
- [ ] `Cargo.toml` has no dependencies.
- [ ] After a full test run, no `kvstore` process is still running.
- [ ] You started the server by hand, stopped it with Ctrl-C in the middle
      of a session, started it again, and your data was there.

## 2. Design

| | not yet | working | strong |
|---|---|---|---|
| Parts | one file, or modules that all know about sockets, files and the map at once | separate modules for the record format, the log file, the store, the protocol and the server; `main.rs` only reads arguments and maps the result to an exit code | a reader can name the layers from the module list alone; each module starts with a comment that says what it knows and what it does not; dependencies point one way (the log does not know the protocol) |
| The protocol without a socket | parsing reads from `TcpStream` directly | the parser is a function from bytes to a command; it has unit tests | cutting the stream into lines is also free of I/O (a state machine over byte slices, or generic over `BufRead`) and is tested with every way of splitting the input |
| Validated data | keys and values travel as `&[u8]` or `String`, checked wherever someone remembered | each kind of input (request line, log file) is checked in exactly one place | a key that has been checked has its own type, so a function that needs a valid key cannot be called with unchecked bytes; the same check serves the wire and the disk |
| The index | a structure chosen without a reason | you can say why you chose `HashMap` or `BTreeMap`, and what `KEYS` and compaction cost with it | the choice follows from the operations: you can give the cost of `GET`, `KEYS <prefix>` and compaction in terms of the number of keys, and the code has no sort that the structure could have done |
| Values: memory or disk | you did not notice there was a choice | you chose one of "values in memory" and "positions in memory, values read from the log" and can name one advantage of each | your `DESIGN` notes say what would have to change to switch, and which SPEC limits made your choice reasonable |

## 3. Correctness that the tests cannot see

| | not yet | working | strong |
|---|---|---|---|
| Written before acknowledged | records go through a `BufWriter` that is flushed "soon" | each record reaches the operating system in a write call before the response is produced | a record is written with **one** write call (built in memory first), so that two records can never interleave and a crash can only cut the last one; you can point to the line |
| Synced when asked | `--fsync` is parsed and ignored | with `always`, a sync call follows every append, before the response | you also sync: the new log before the rename in a compaction, the log at shutdown, the log after recovery shortened it, and (on Unix) the directory after creating or renaming a file. You can say for each what would be lost without it |
| Lengths from the file | the reader allocates `vlen` bytes and then reads | lengths are checked against the SPEC limits before they are used | recovery reads with a buffer of fixed maximum size; a test of your own feeds it a header with `vlen = 0xFFFFFFFF` |
| Recovery cost | reads the whole log into one `Vec`, or does a system call per field | one pass through a buffered reader; memory use does not grow with the log beyond the index | you measured it: a log of a million records recovers in about a second or less in a release build, and you know the number |
| Long lines | the line is collected in memory until the line feed arrives | at most a bounded number of bytes is kept per connection; the rest of a long line is dropped while reading | a unit test sends many megabytes without a line feed to your line reader and asserts that its buffer did not grow |
| The compaction switch | the new log is written into `kv.log` itself | temporary file, sync, rename | every handle on the old file is closed before the rename (Windows may refuse to replace an open file), and you can list the states the directory can be in if the process dies at each step |
| Disk errors | `unwrap()` on file operations | an I/O error on the log stops the server with exit code 1 and a message | after a failed append nothing further is appended or acknowledged, enforced by the structure of the code (not by hoping the process exits fast enough) |
| Lock scope | a lock is held while writing to a client socket | no lock on the store is held during socket I/O | you can state, for each lock, what it protects and what is never done while it is held; `KEYS` on a large store cannot be stalled by its own slow client while holding the lock |
| Shutdown | `std::process::exit` from the thread that read `SHUTDOWN`, while other threads may be in the middle of a write | the server stops accepting, closes the connections, waits for its threads, syncs, and returns from `main` | no thread is detached; you can say what wakes the accepting thread and each connection thread, and nothing in the shutdown path polls or sleeps |

## 4. Idiom

| | not yet | working | strong |
|---|---|---|---|
| Errors | `Box<dyn Error>` everywhere, or strings | your own error enums implement `Display` and `Error`; `?` carries them | the types separate what must be separated: an error that ends one connection, one that ends the server, one that prevents the start. The exit code is computed in one place |
| `unwrap` and `expect` | on anything that depends on input or the system | only where failure is impossible, each with a message that says why | none in the request path; poisoned locks are handled deliberately (you decided what a panic in one connection thread should mean for the others) |
| Bytes | values pass through `String` or `from_utf8_lossy` | `&[u8]` and `Vec<u8>` for keys and values throughout | no conversion to text anywhere except for the number in `INCR`; byte-string patterns (`b"GET"`) in the parser |
| Integers | `as` casts between lengths and file fields without thought | conversions between `usize`, `u16`, `u32`, `u64` are written so that they cannot truncate (checked, or justified by a limit) | the justification is next to each cast, and the encoder uses `to_le_bytes`, never shifts by hand |
| Allocation | a new `Vec` and `String` per request for every step | buffers for the line and the record are reused per connection or per log | you counted: a `GET` or `SET` in steady state allocates at most a small fixed number of times |
| Threads | `Arc<Mutex<..>>` around everything | shared state is one clearly defined value; threads get what they need and no more | scoped threads or an equivalent make the lifetimes obvious; no `Arc` that exists only because a thread "might" outlive something it cannot outlive |

## 5. Tests of your own

| | not yet | working | strong |
|---|---|---|---|
| Unit tests | none | the checksum (with the three values from the SPEC), the record encoder and decoder, and the request parser have tests | every rule of SPEC 5.4 and every row of the table in SPEC 8.2 has a test of yours; the example of SPEC 5.5 is one of them |
| Recovery | only through the acceptance tests | a test opens your log type on a file cut at every length | a test with many random operations and random damage compares your recovery with a simple model, with a fixed seed |
| After a bug | fixed and forgotten | each bug that cost you more than half an hour has a test with a name that says what went wrong | you can show one test that fails if you remove a specific line you once forgot |

## 6. Performance

Measure with a release build and `--fsync never`, unless a row says
otherwise. The numbers are modest on purpose: they separate a sound
implementation from an accidentally quadratic one, not a tuned one from a
plain one. For orientation, the model solution on a laptop reaches about
50 000 requests per second with one client that waits for each answer,
and recovers a million records in about a quarter of a second.

| | not yet | working | strong |
|---|---|---|---|
| Single client | under 2 000 `SET` per second | 10 000 or more | you know where the time goes (system calls per request) and reduced them: one write per record, one write per response |
| Pipelining | each response is its own write call | responses to pipelined requests are batched and flushed only when the input is drained | 200 000 or more pipelined `SET` per second, and still an immediate answer to a single request |
| Start-up | time grows faster than the log | linear in the log size | a million records in about a second or less |
| Compaction | time grows faster than the number of live keys | linear | you measured how long writers are blocked during a compaction of a million keys, and wrote the number down |
| `--fsync always` | (the speed is set by the disk) | you measured it and can explain why it is 100 or more times slower than `never` | you implemented or at least designed group commit (SPEC 16) and can say what throughput with 64 clients it should give |

## 7. Unsafe hygiene

This project needs no `unsafe`, and that is the criterion.

| | not yet | working | strong |
|---|---|---|---|
| `unsafe` | present, for speed or to get around the borrow checker | none | `#![forbid(unsafe_code)]` at the crate root, so that it stays that way |
| Platform code | `#[cfg(unix)]` blocks spread through the logic | platform differences, if any, are in one or two small functions | each such function has a comment that says what the other platforms do and why that is acceptable |

## 8. What to write down

Write a short `DESIGN.md` of your own, one page. A reviewer should find in
it:

- the layers and what each knows;
- the one invariant that connects the log and the index, and where the code
  keeps it;
- what is locked by what;
- how shutdown wakes every blocked thread;
- two decisions you would make differently next time.

Then read the model solution's `DESIGN.md` and compare. The comparison is
the last exercise of this capstone.
