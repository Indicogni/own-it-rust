# Chapter 18 — Files, streams and processes

[All chapters](../../README.md) · [Previous chapter](../chapter17/README.md) · [Next chapter](../chapter19/README.md)

Every useful program reads bytes from somewhere and writes bytes to somewhere: files, the terminal, the network, other programs. Rust describes all of them with two small traits, `Read` and `Write`, and this chapter teaches you to write code against those traits that stays correct when the world is less tidy than a test on your own machine. You start with the loops behind every read and write, where one call may move only part of the data. Then you read lines and delimited records, decide where a buffer must be flushed, seek inside binary data with an explicit byte order, and handle paths and directories without losing file names that are not Unicode. Lessons 08 and 09 leave the process: you exchange length-prefixed messages over a real TCP connection on your own machine, and you start child processes and connect pipes to them without deadlocks. Everything you need is taught in this directory; nothing depends on outside reading.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read the lesson's section in [CONCEPTS.md](CONCEPTS.md).** It explains the idea, works through an example that is different from the exercise, and shows the errors people usually meet.
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter18 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests use readers and writers that return one byte at a time, fail on purpose, or record how they were called, so a solution that assumes one `read` fills the buffer fails. Others measure the largest allocation, or send two mebibytes through a pipe. Then run `cargo clippy --test chapter18` and fix warnings in your code.
5. **Drill the compiler.** `cargo drill ch18` shows small programs; predict whether each compiles (and which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge case of your own, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

Do the lessons in the order of the table: lessons 08 and 09 come before the capstone (the capstone keeps its file name, `l07_capstone.rs`). After the capstone, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: fix a teammate's working-but-wrong code). Run `cargo progress` to see where you are and which chapters are due for spaced review.

Open [HINTS.md](HINTS.md) one hint at a time, and only after a real attempt. Passing tests alone does not demonstrate mastery.

| Done | Lesson | Coding tasks |
| --- | --- | --- |
| [ ] | [l01 · Streaming bytes](l01_readers.rs) | 2 |
| [ ] | [l02 · Buffered text](l02_buffered.rs) | 2 |
| [ ] | [l03 · Writers and flushing](l03_writers.rs) | 2 |
| [ ] | [l04 · Seeking and binary formats](l04_seek_binary.rs) | 2 |
| [ ] | [l05 · Paths preserve OS data](l05_paths.rs) | 2 |
| [ ] | [l06 · Directory metadata](l06_directories.rs) | 2 |
| [ ] | [l08 · Framed messages over TCP](l08_tcp_frames.rs) | 4 |
| [ ] | [l09 · Child processes and pipes](l09_processes.rs) | 3 |
| [ ] | [l07 · Capstone: a bounded record service](l07_capstone.rs) | 2 |
| [ ] | [r01 · Review: Chapters 3, 7, 11, 15–17 from memory](r01_review.rs) | 3 |
| [ ] | [c01 · Code review: a teammate's pull request](c01_code_review.rs) | review + fix |
| [ ] | [Compiler drills](../../drills/ch18/) — `cargo drill ch18` | 9 drills |

```sh
cargo test --test chapter18 l01_
cargo test --test chapter18 -- --list
# After completing all lessons:
cargo test --test chapter18
cargo test --release --test chapter18
cargo clippy --test chapter18
cargo drill ch18
cargo run --example ch18_scratch
```

The starter compiles; unfinished TODOs intentionally fail when reached. Keep the supplied tests. Each lesson is independent, including the capstone, so later unfinished tasks do not block an earlier lesson. Every task uses only the standard library and needs no internet connection.

Three things about the test run are worth knowing:

- Lesson 06 creates and removes small directories under your system's temporary directory.
- Lesson 08 opens TCP connections on the loopback address `127.0.0.1`, on ports that the operating system picks. Nothing leaves your machine.
- Lesson 09 starts the test program itself a second time as a child process, so it needs no other program. One always-passing test named `child_mode_helper` is that child's job, not a check of your code.

Tests that could wait forever under a wrong solution (a read loop that misses the end of input, two sockets or two processes that wait for each other) stop after a time limit and fail with a message that says what to look for.

Each lesson header also describes an optional lab: free-form experiments, some of which must not compile. Do those in [the scratch example](../../examples/ch18_scratch.rs), not in the lesson files, and restore it to compiling code afterward.

**Time.** Plan for about 10 to 12 hours in total: roughly 40 to 45 minutes for each of lessons 01 to 06 including the reading, about 90 minutes each for lessons 08 and 09, an hour for the capstone, and 2 to 3 hours for the review lesson, the code review and the drills. These are estimates; [CONCEPTS.md](CONCEPTS.md) lists one per lesson.

## Topic map

| Topic | Where you practise it |
| --- | --- |
| The `Read` contract: short reads, end of stream, `Interrupted` | l01 task A and predictions; l08 task B; C01; drills d03, d06, d09 |
| Reader adapters: `take`, `chain`, `by_ref` | l01 task B and predictions |
| Trait methods need the trait in scope | drill d01 |
| `BufRead`: `lines`, `read_line`, `read_until`, `split`; text versus bytes | l02 tasks and predictions; R01 tasks A and B; drills d02, d04 |
| The `Write` contract: short writes, `write_all`, `write!` | l03 task A; l07; C01 |
| Buffering and flushing: `BufWriter`, `LineWriter`, who flushes | l03 tasks and predictions; l07 task B; drill d07 |
| `Seek`, `SeekFrom` and `Cursor` as an in-memory file | l04 task A and predictions |
| Binary fields: fixed widths, little and big endian, checked offsets and lengths | l03 task B; l04 tasks and predictions; l08 tasks A and B |
| `Path`, `PathBuf`, `OsStr`: changing names without Unicode conversion | l05 task A and predictions; C01; drills d05, d08 |
| Path components and the limits of lexical checks | l05 task B and predictions |
| Listing a directory: entry errors, order, symbolic links | l06 task A |
| File-system error kinds; checking that something exists | l06 task B and predictions |
| Length-prefixed framing; validating a length before allocating | l04 task B; l07 task A; l08 tasks A and B |
| TCP on loopback: listener, accept, connect, half-close, clean and cut-off ends | l08 tasks C and D and predictions |
| Child processes: `Command`, exit status, captured output | l09 task A and predictions |
| Pipes between processes: feeding a child, a two-stage pipeline, avoiding deadlock | l09 tasks B and C |
| Error types that wrap `io::Error` | R01 task C; l09 task A |
| Files opened with `File` and `OpenOptions`, buffered file I/O, adapters such as `io::repeat` | optional lab 1 in [LABS.md](LABS.md) (not checked by tests) |
| Compression and serialization with external crates | optional lab 2 in [LABS.md](LABS.md) (not checked by tests) |
| Platform metadata, permissions, `canonicalize` | optional lab 3 in [LABS.md](LABS.md) (not checked by tests) |
| Socket timeouts, several clients, UDP | optional lab 4 in [LABS.md](LABS.md) (not checked by tests) |

Not in this chapter: asynchronous I/O is in Chapters 20 and 21 and in the mastery track ([`mastery/m4_async`](../../mastery/m4_async/README.md), which builds a reactor over sockets and a real service); threads beyond the one helper thread used here are in Chapter 19.

## Solutions

Model answers for every lesson, the review, the code review and the drills are in [`solutions/exercises/chapter18/`](../../solutions/exercises/chapter18/) and [`solutions/drills/ch18/`](../../solutions/drills/ch18/). Open a solution only after an honest attempt: first your own version, then the hints, then the solution, and then explain to yourself every difference between it and your code.

## Optional labs

[LABS.md](LABS.md) describes four longer projects that you build as separate small Cargo projects. They have step-by-step checks that you verify yourself; no test in this repository checks them.

## Mastery check

Answer from memory, in writing. Model answers are at the end of [HINTS.md](HINTS.md).

1. What may a successful Read::read or Write::write return before the buffer is exhausted?
2. When does read_line append, and how do lines() and split() differ?
3. How do Cursor, BufReader, Take, Chain, and Seek compose?
4. Why must paths and file names not be forced through String?
5. Which error kinds should retry, propagate, or end a stream?
6. How do you frame messages on TCP, and why does a single read not correspond to a whole message?
7. Why is a failed child process not an io::Error, and how can a parent and a child connected by pipes block each other forever?

Redo the capstone from its contract in a later session. Explain one design tradeoff and create a case the supplied tests do not exercise.

## Validation scope

Use the repository's pinned Rust 1.99.0 toolchain and run commands from the repository root (the [course guide](../../README.md) has the setup instructions). The starter target can be checked without running TODOs using `cargo test --offline --test chapter18 --no-run`. The optional labs are manual practice and are outside the automated checks.
