# Chapter 21 optional labs — executors and real I/O

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md)

These four projects take the lesson models to real threads, sockets, and runtimes. They are optional. No test in the course repository checks them, and there is no model solution: the acceptance criteria at the end of each lab are your check. Do them after the lessons they build on.

Each lab lives in its own Cargo package outside the course repository (`cargo new`), so the course itself never gains a dependency. Labs 2 and 3 need crates from crates.io, so they need a network connection the first time you build them. The versions below are exact (`=`), and the steps were checked with these versions on the course's pinned Rust 1.99.0. A newer 1.x release of either crate will probably work as well, but then the details may differ from what is written here.

Work through the steps in order; each checkpoint tells you what you should see before moving on.

| Lab | Builds on | Needs |
| --- | --- | --- |
| 1 · Blocking work adapter | Lessons 03 and 04, R01 task B | standard library |
| 2 · A small reactor using real readiness | Lesson 05 | `mio` 1.2.5 |
| 3 · Runtime roles and fairness | Lesson 06 | `tokio` 1.54.1 |
| 4 · Pinning and future lifecycle audit | Lesson 07 | standard library |

## Lab 1 · Blocking work adapter

**Setup.** Standard library only (`cargo new --lib blocking-lab`). Copy your l03 completion cell and l04 `block_on` into it.

```toml
[dependencies]
# none: std::thread, std::sync, std::panic::catch_unwind
```

**Steps**

1. Adapt the completion cell into `spawn_blocking<T>(job: impl FnOnce() -> T + Send + 'static) -> impl Future<Output = Result<T, JobError>>`, where `JobError` distinguishes "the job panicked" from "the worker disappeared".
2. Move the closure into a new OS worker thread and run it under `catch_unwind`. Store the success or the failure in the shared state under the lock, take the registered waker, and wake *after* unlocking.
   - *Checkpoint:* you should now see `block_on(spawn_blocking(|| 6 * 7))` return `Ok(42)`.
3. Add explicit producer-disconnected behavior: if the worker side is dropped without storing a result, the future resolves to `Err(JobError::Disconnected)` instead of staying pending forever (a `Drop` impl on the producer half is one way).
4. Write tests for each case, ordering them with std channels, never sleeps:
   1. completion before the first poll;
   2. polling before completion;
   3. a replacement waker (poll with waker A, then B; only B is woken);
   4. a worker panic;
   5. the receiver dropped early.
   - *Checkpoint:* you should now see all five tests pass repeatedly in both debug and `--release`.
5. Choose and document a join/reaper policy (for example, keep each `JoinHandle` and join it when the result is taken, or hand it to a reaper thread) so no worker is accidentally detached.
6. Run everything through your l04 `block_on`. Then explain how this differs from Tokio's pooled `spawn_blocking`, including why dropping the future cannot cancel a job that is already running.

**Acceptance criteria**

- [ ] Completion before polling, polling before completion, replacement waker, worker panic, and receiver dropped early are all covered by tests.
- [ ] A disconnected producer resolves the future with an error rather than leaving it pending.
- [ ] Wakes happen after the lock is released.
- [ ] Ordering is driven by std channels; no sleeps.
- [ ] A documented join/reaper policy; no worker is detached by accident.
- [ ] A written comparison with Tokio's pooled `spawn_blocking`, including its cancellation limitations.

## Lab 2 · A small reactor using real readiness

**Setup.** Create a separate package (`cargo new reactor-lab`) and add Mio with its `os-poll` and `net` features:

```sh
cargo add mio@=1.2.5 --features os-poll,net
```

```toml
[dependencies]
mio = { version = "=1.2.5", features = ["os-poll", "net"] }
```

**Steps**

1. Bind a loopback listener on port 0, connect a client, accept, and set the accepted `TcpStream` to nonblocking (Mio streams are nonblocking already).
2. Build a registration table, `Mutex<HashMap<Token, Waker>>`, and a reactor thread that owns the `mio::Poll`. When an event arrives for a token, the reactor takes that token's waker and wakes it.
3. Write a read future. Each poll first attempts a nonblocking read. On `WouldBlock`, it registers (or updates) its waker under its token and arranges an interest update and a wakeup for the reactor.
4. Close the check/register race explicitly: after registering, retry the read once before returning `Pending`, so a byte that arrived in between is not missed.
   - *Checkpoint:* you should now see a single byte written by the client complete a pending read.
5. Remember that readiness can be stale or spurious: when woken, the future retries the I/O and may return `Pending` again. Distinguish EOF (`Ok(0)`) from `WouldBlock`.
6. Drive a two-part payload with acknowledgement channels: write part one, wait until the reader reports it, then write part two. Use no external network and no sleeps.
   - *Checkpoint:* you should now see both parts read in order, with the reader pending in between.
7. Make a dropped read future unregister its token, and make tokens unique (for example, a generation counter) so a stale token cannot wake a replacement resource.
8. Add a shutdown signal (a Mio `Waker` or a closed channel) that makes the reactor thread exit, and join it.
9. Bound every integration test with a timeout (for example, run it on a thread and `recv_timeout` its result) so broken code fails with a diagnosis instead of hanging.
10. Explain edge-triggered draining (read until `WouldBlock` after each event) and why an event alone is not a complete message.

**Acceptance criteria**

- [ ] No wake is lost when a byte arrives around registration.
- [ ] EOF is distinguished from `WouldBlock`.
- [ ] Dropped futures unregister their token.
- [ ] Stale tokens cannot wake a replacement resource.
- [ ] A shutdown signal stops the reactor, and the reactor thread is joined.
- [ ] Integration tests are bounded by timeouts; ordering uses acknowledgement channels, not sleeps.
- [ ] A written explanation of edge-triggered draining and why an event is not a complete message.

## Lab 3 · Runtime roles and fairness

**Setup.** A separate package (`cargo new fairness-lab`) with Tokio:

```sh
cargo add tokio@=1.54.1 --features rt,macros
```

```toml
[dependencies]
tokio = { version = "=1.54.1", features = ["rt", "macros"] }
```

**Steps**

1. On a current-thread runtime (`#[tokio::main(flavor = "current_thread")]`), spawn two budgeted CPU futures (like l06's `BudgetSum`, or a loop that calls `tokio::task::yield_now().await` between chunks). Each appends `(task, chunk)` to a shared log.
   - *Checkpoint:* you should now see entries from both tasks interleaved in the log.
2. Replace one of them with a future that does all its work in a single poll.
   - *Checkpoint:* you should now see that task's entries appear as one uninterrupted run.
3. Assert only what is guaranteed: both tasks finish with the right results and each task's own chunks are in order. Do not assert an implementation-specific interleaving.
4. Read the official documentation for Tokio's task queues, cooperative budget, I/O driver, and timer driver. Note which statements are public guarantees and which are implementation details.
5. Draw a diagram separating the future, executor, reactor (I/O driver), timer, waker, and OS readiness, with arrows for who calls whom.

**Acceptance criteria**

- [ ] Both versions produce the same final results.
- [ ] All handles are joined.
- [ ] No blocking I/O happens inside `poll`.
- [ ] Tests do not assert an implementation-specific task order.
- [ ] Notes distinguish Tokio's public guarantees from implementation details.
- [ ] A diagram separates future, executor, reactor, timer, waker, and OS readiness.

## Lab 4 · Pinning and future lifecycle audit

**Setup.** Standard library only; this is mostly a compile-only project.

```toml
[dependencies]
# none: std::pin, std::marker::PhantomPinned
```

**Steps**

1. Define `Plain` (which is `Unpin`) and `Pinned` (which contains `std::marker::PhantomPinned`).
2. Build a compile-only matrix: for `T`, `Box<T>`, `Pin<&mut T>`, and `Pin<Box<T>>`, with each of `Plain` and `Pinned`, try to move the pointee out (`std::mem::swap`, `mem::replace`, `Pin::get_mut`, `Pin::into_inner`). Record each compiler diagnostic in a table.
   - *Checkpoint:* you should now see every attempt on `Plain` compile and every safe attempt to move a `Pinned` pointee out of a pin rejected.
3. Show that moving the pinned *pointer handle* (the `Pin<Box<Pinned>>` itself) is allowed, while moving the `!Unpin` pointee through a safe API is rejected. Do not bypass `Pin` with unchecked access to force a forbidden move.
4. Compare a newly constructed async future (no self-references yet, so moving it is harmless) with the same future after it has been polled to `Pending` while holding a borrow of its own local (now it must not move).
5. Write a safe pinned adapter, like l07's `WithTag`, that needs no `unsafe` projection.
6. Explain why `poll` takes `Pin<&mut Self>` even for futures that happen to be `Unpin`.

**Acceptance criteria**

- [ ] Compiler diagnostics for each rejected move are recorded in the matrix.
- [ ] Moving a pinned pointer handle is shown to be allowed; moving a `!Unpin` pointee through a safe API is shown to be rejected.
- [ ] No unchecked access is used to bypass `Pin`.
- [ ] The constructed-versus-polled comparison is written up.
- [ ] A safe pinned adapter works without `unsafe` projection.
- [ ] A written explanation of why polling takes `Pin<&mut Self>` even for `Unpin` futures.

## Where to go next

Lab 4 stops at the safe pinned box. Pin projection (a `Pin<&mut Field>` to a future that is stored directly in a struct), a task queue with `spawn`, timers, streams, and a reactor that you write yourself are the subject of the mastery track, in [`mastery/m4_async`](../../mastery/m4_async/).

References: [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html), [the `std::pin` module](https://doc.rust-lang.org/std/pin/index.html), [Mio 1.2.5](https://docs.rs/mio/1.2.5/mio/), and [the Tokio 1.54.1 runtime](https://docs.rs/tokio/1.54.1/tokio/runtime/index.html).
