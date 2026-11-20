# Design of the reference `httpd`

This is one way to build the server that `capstones/httpd/SPEC.md`
describes. It is not the only good one. This document says what was
decided, what the alternatives were, and what each decision costs. Section
numbers such as "SPEC 7.4" refer to SPEC.md.

About 2000 lines of Rust without tests, standard library only, no `unsafe`.

## 1. The map

```text
main.rs       exit statuses; nothing else
config.rs     arguments -> Config                     pure
request.rs    bytes -> where the head ends; -> Head   pure
json.rs       bytes -> compact bytes or an error      pure
files.rs      path -> names (pure), names -> file     file system
date.rs       seconds since 1970 -> HTTP date         pure
response.rs   Status, Response, writing to any Write  generic over Write
pool.rs       a bounded worker pool for any item      threads, no sockets
conn.rs       one connection: the six steps, timeouts sockets
server.rs     listener, admission, shutdown           sockets, threads
```

The rule behind the map: **everything that is difficult to get right is a
function that can be called from a unit test without a socket.** The head
grammar, the path rules, the JSON grammar, the calendar, the body-length
rules, the routing table and the pool's capacity are all tested with plain
values, in about 10 milliseconds for 45 tests. (A few of them create
small files in the temporary directory; none opens a socket or sleeps.) The
acceptance tests then only have to show that the parts are connected
correctly.

`conn.rs` is the one place where the order of the SPEC's processing steps
lives. `Connection::exchange` is written so that it can be read next to
SPEC section 4: step 1, step 2, and so on, top to bottom, each ending in
"continue" or in a `return` with a status.

## 2. Threads, and who owns a connection

```text
main thread      accept loop      owns the TcpListener
worker-0..N-1    Pool workers     each owns the one TcpStream it serves
control          reads stdin      owns nothing; sets one flag
```

A `TcpStream` has exactly one owner at every moment: the accept loop
receives it from `accept`, moves it into the pool's queue, a worker takes
it out and moves it into `conn::serve`, and when `serve` returns the
stream is dropped, which closes it. There is no registry of open
connections, no cloned handle, no `Arc<Mutex<TcpStream>>`. Every way a
connection can end (finished, timed out, client gone, write failed, panic)
ends in the same drop.

The price of this simplicity is paid in section 5: nobody can reach into a
worker's connection from outside, so a worker must find out by itself that
the server is shutting down.

Shared state is one `Arc<Context>`: the configuration (read-only) and one
`AtomicBool`.

## 3. The pool

`Pool<T>` knows nothing about HTTP. It owns a `Mutex<State<T>>` with a
`VecDeque<T>`, a count and a `closed` flag, plus one `Condvar`. Items go
in at the back and come out at the front, so connections that wait are
served in the order in which they were accepted (SPEC 10.2).

**What it counts.** SPEC 10.2 bounds the connections the server *holds*:
waiting plus being served. So the pool counts `admitted` = items in the
queue + items a handler is running on, and refuses when
`admitted == workers + queue`. The count goes up in `try_submit`, under
the lock, together with the push. It goes down in the worker, after the
handler has returned.

The obvious alternative is `std::sync::mpsc::sync_channel(queue)` with
`try_send`. It is less code and it is a fine answer for this capstone. It
bounds something slightly different: the items in the channel. A worker
that has finished a connection but has not yet called `recv` again is
neither "busy" nor "waiting" for a moment, and with `--queue 0` (a
rendezvous channel) `try_send` only succeeds while a worker is actually
parked in `recv`. Counting explicitly makes "full" mean exactly what the
SPEC says at every instant, and it makes `--queue 0` work. It also gives
`close()` something to return: the items that no worker started, which
the server needs for SPEC 11.2 (3).

**Panics.** A handler runs code that reads bytes from strangers. The
worker wraps the call in `catch_unwind`, so a panic costs one connection
(its `TcpStream` is dropped during unwinding, the client sees a close) and
neither the worker nor the slot. The handler runs outside the lock, so it
cannot poison the mutex; `lock()` still recovers from poisoning instead of
propagating it, because a pool that stops for ever after one bad moment is
worse than one that continues with a consistent `State`. The unit test
`a_panicking_handler_costs_neither_a_worker_nor_a_slot` pins both
properties.

I know of no input that makes the request path panic: there is no
`unwrap` on client data, and slice indexes are derived from lengths that
were checked. `catch_unwind` is the second line of defence, for the bug
I have not found.

**Full means refused, on the accept thread.** `try_submit` hands the
stream back, and `server::reject` writes the 503. It sets a 100 ms write
timeout first; the response is about 190 bytes and a fresh socket's send
buffer is far larger, so the write does not block in practice, and it
cannot block for long in theory. After writing, it reads without
blocking, at most 64 KiB, to take what the client has already sent out of
the socket, and drops the stream. (SPEC 10.3 allows this as long as
nothing waits. The bound matters: a loop that reads "until nothing is
left" can be kept busy by a client that sends as fast as the loop reads.) That is weaker than the lingering close of section 6
(a client that is still sending can still cause a reset), and it is the
SPEC's choice: the accept thread must never wait for one client. A
stronger design hands refused connections to one extra thread that lingers
on them, with its own small bound.

## 4. Reading: one buffer, one deadline

Each connection has one `Vec<u8>` that lives as long as the connection.
`read_more(deadline)` appends whatever one `read` returns. The head
scanner and the body loop only ever ask "is enough in the buffer yet?".
After a response, exactly the bytes of that request are drained; what
remains is the start of the next request. This one rule gives requests in
pieces, pipelining, and "a body on `GET` is skipped" without any special
case.

`request::find_head_end` is called after every read and resumes where it
stopped, so every byte is examined once. It reports four things:
complete, incomplete, too long, and the bare line feed that SPEC 5.1
wants reported at once. Since it looks at each byte anyway, the bare-LF
check costs nothing.

**Deadlines, not timeouts.** `set_read_timeout` is a limit per call.
SPEC 7.3 wants a limit per request. So `exchange` computes
`deadline = now + request_timeout` once, and `read_more` sets the socket
timeout to the time that is left before every read. A client that sends
one byte per 100 ms makes every read succeed and still runs into the
deadline.

**The head is parsed into owned values.** `Head` owns a `String` for the
target and a `Vec` of `(String, Vec<u8>)` pairs for the headers: about a
dozen small allocations per request. The alternative is a `Head<'a>` that
borrows from the connection buffer, with zero allocations. I did not take
it because the buffer must be borrowed mutably to read the body while the
head is still needed afterwards; the ways around that (ranges into the
buffer, or splitting the head off into its own allocation) are more code
than this server's two endpoints justify. If a profile showed the parser
in the hot path, ranges would be my first change.

**Memory per connection** is bounded by `8192 + max_body + 8192` bytes:
the head limit, the body limit and one read's worth of the next request.
The body buffer grows as bytes arrive, never from the announced
`Content-Length`, so a client cannot make the server allocate 16 MiB with
one header line. After each request the buffer is shrunk back, so idle
connections are cheap.

## 5. How a blocked worker learns about shutdown

This is the least elegant part, and the most instructive.

A worker that waits for the next request is blocked in `read`. The
standard library offers no way to wait for "bytes, or a signal from
another thread". The options:

1. **Poll.** Read with a short timeout, look at the flag, read again.
2. **Shut the socket from outside.** Keep a cloned handle of every idle
   connection in a registry; at shutdown call `shutdown()` on each, which
   makes the blocked `read` return.
3. **Non-blocking sockets and a readiness API** (`epoll`, `kqueue`, IOCP).
   That is a different architecture, and not in the standard library.

The reference uses option 1 with a 50 ms slice, in `wait_for_request`
only. A request that has started is read with the full remaining
deadline, because SPEC 11.2 says it is finished normally and nothing has
to interrupt it.

The cost: every idle connection wakes its worker 20 times per second for
nothing. With at most 256 workers that is at most about 5000 wake-ups per
second on an otherwise idle machine. That is measurable and acceptable
here; it would be unacceptable for 10 000 connections, but so would a
thread per connection.

Option 2 has no idle cost. I did not take it for two reasons. It breaks
the ownership story of section 2: a second handle to every connection
exists, and a registry must be kept exactly in step with the workers. And
its key property, that `shutdown()` from one thread wakes a `read` that is
already blocked in another, is not promised by the documentation of the
standard library. I did not want the reference to depend on it on three
operating systems when I could run only one.

The same problem exists once more, for the accept loop: it is blocked in
`accept`. There the answer is the classic one: the control thread makes a
connection to the server's own address. `accept` returns, the loop looks
at the flag before it does anything with what it got, and leaves. If that
connection attempt fails, the listen queue is full, which means `accept`
has something to return anyway.

The flag is an `AtomicBool` with `Ordering::Relaxed`. It guards no other
data: no reader uses the flag to decide that some *other* memory is ready
to be read. A reader that sees the old value for a moment longer does one
more 50 ms wait, or accepts one more connection that is then dropped.
Nothing needs a stronger ordering.

## 6. Closing

`Next` has three values, and the difference between two of them matters:

- `Close`: drop the stream. Used when nothing was sent (idle timeout,
  shutdown of an idle connection, the client went away) or when sending
  failed.
- `Linger`: used after every response with `Connection: close`. Close the
  sending side, then read and discard for at most 1 second or 64 KiB,
  then drop.

Why: on the system I developed on, dropping a socket that still had
unread bytes turned the close into a reset, and the test client then saw
"connection reset by peer" after the response instead of a normal end of
stream. I verified this by removing the `linger` call: the acceptance
test `an_error_response_can_be_read_although_more_bytes_were_sent` then
fails on every run. Whether the response itself survives a reset depends
on the client's system and on timing, which is exactly why a server must
not rely on it.

The two limits on the lingering read are both needed. Without the time
limit, a client that never closes holds a worker. Without the byte limit,
a client that keeps sending holds it too, one second at a time.

One more detail from the same family: setting a socket option on a
connection that the client has already reset fails (with "invalid
argument" on macOS). `read_more` treats that as "the client is gone". My
first version treated it as a timeout. In the idle wait, a timeout means
"look at the flag and wait again", so that version would loop without
ever blocking until the idle deadline. No acceptance test shows this; I
found it because the test client made the same call and failed on it.

## 7. Files

`files::resolve` is the security boundary, and it is a function from text
to text: the path becomes a list of decoded names, and a name that starts
with a dot or contains `/`, `\`, `:` or NUL is refused. It never touches
the disk, so its unit test is a table of attacks.

The alternative that many servers use is to ask the file system: join the
path, call `canonicalize`, and check that the result still starts with the
canonical root. That also defends against symbolic links that lead out of
the root, which the textual rule does not (the SPEC makes symbolic links a
non-goal). I prefer the textual rule as the first line because it does
not depend on the platform, cannot be raced, and fails closed: a name the
rule does not understand is refused before any system call sees it. A
production server would do both.

The root is canonicalized once at start-up. Requests are then independent
of the working directory. On Windows the canonical form is a "verbatim"
path (`\\?\C:\...`). The Windows documentation says that the system does
not rewrite the names in such a path, so `NUL`, `CON` or a name with a
trailing dot should mean what they say: a file that does not exist. (I
could not run this on Windows; see section 11.)

`open` asks `fs::metadata` whether the path is a directory (because
`File::open` on a directory succeeds on Unix and fails with "access
denied" on Windows), opens the file, and then takes the length from the
*open file's* metadata. One case is left over after the directory check: a
directory that is itself called `index.html`. On Unix the open succeeds
and the metadata says "not a regular file"; on Windows the open is denied,
and the code asks once more whether the path is a directory before it
calls that a 403. Both ways the answer is 404, as SPEC 8.3 says. The length in `Content-Length` and the bytes that
follow come from the same file object. If the file shrinks while it is
being sent, `write_to` notices that fewer bytes were copied than promised
and returns an error, and the connection is dropped: the client must not
be left waiting for bytes that will never come, and it must not read the
next response as the rest of this one.

Files above 16 KiB are sent with `io::copy` from `File::take(len)` to the
stream (smaller ones travel with the head, see section 9). It
uses a fixed buffer, and the standard library's documentation says that on
Linux it can hand a file-to-socket copy to the kernel. I ran the server
only on macOS, where the plain read/write loop is used; the test
`a_client_that_does_not_read_cannot_keep_a_worker` checks there that the
write timeout ends a stalled copy.

## 8. JSON without a tree and without recursion

`POST /echo` must answer one question (is this JSON?) and produce one
output (the same tokens, without the whitespace between them). Neither
needs a tree of values. `json::compact` walks the bytes once with a small
state (`Expect`), copies each token to the output as it is recognised,
and keeps the open brackets in a `Vec<Container>`.

A recursive-descent parser is the more familiar design and about the same
length. With a depth limit of 64 it would also be safe. I chose the
explicit stack because the property "the input cannot choose how deep the
call stack goes" is then true by construction, not by a constant: if
someone raises `MAX_DEPTH` to a million, the recursive version overflows
a worker's stack and aborts the whole process (a stack overflow is not a
panic and `catch_unwind` does not help), while this version allocates a
megabyte and carries on.

UTF-8 is validated once for the whole body with `str::from_utf8`. After
that the scanner only inspects ASCII bytes; a byte above 0x7F can only be
inside a string, where it is copied.

## 9. Responses

`Response::write_to` is the only code that writes to a client. It always
writes the four required headers, so no response can lack one. The two
things that depend on the connection and not on the response (`HEAD`, and
keep-alive or close) are parameters of the call, so `route` cannot get
them wrong.

The head and a body of up to 16 KiB are joined and written with one
`write`, whether the body comes from memory or from a file. So the common
small response is one system call and one TCP segment, as SPEC 6.5 asks.
Larger bodies and files follow the head in further writes, and
`TCP_NODELAY` is set on every connection so that these never wait for the
client's acknowledgement of the first. The two measures cover different
cases: joining removes the second write where it is cheap to remove, and
`TCP_NODELAY` makes the remaining ones harmless. The unit test
`a_small_file_leaves_in_one_write_with_its_head` counts the writes.

A small file is read completely before anything is sent. If it turns out
shorter than the length that was read when it was opened, no byte has
left the server yet; the connection is dropped all the same, which keeps
one rule for both sizes.

`Status` is an `enum`, so a code without a reason phrase cannot be
written. `Response::extra` is a single optional header, not a list,
because exactly three responses need one extra header each. A list would
be more general than the program.

## 10. Shutdown, in order

`server::run` ends with five lines that are SPEC 11.2:

```text
accept_loop(...)        returns when the flag is set
drop(listener)          new clients are refused by the system
announce("shutting down")
drop(pool.close())      waiting connections: closed, unanswered
pool.join()             workers finish what they have
```

Each step is finished before the next begins. `pool.join()` is what makes
the shutdown graceful: `main` does not return, and the process does not
exit, while a worker is still inside a request.

The control thread blocks in a read of standard input and cannot be told
to stop. It is the one thread that is still alive when `main` returns; the
exit of the process ends it. It owns nothing that needs cleaning up.

Shutdown is bounded without a grace timer because every wait in a worker
is bounded: at most one request timeout to read, one request timeout per
stalled write, one second to linger.

## 11. What was verified, and how

- 45 unit tests and the 86 acceptance tests pass in debug and release
  builds, three runs each, on macOS (aarch64). `cargo clippy --all-targets
  -- -D warnings` is clean. The unit tests also pass under Miri
  (`cargo +nightly-2026-11-20 miri test --bin httpd` with
  `MIRIFLAGS=-Zmiri-disable-isolation`); the acceptance tests cannot run
  there, because they start a process and use sockets.
- The acceptance tests were checked against 40 deliberately wrong
  variants of this server. Among them: single-threaded; a thread per
  connection; a timeout per read; decode before split; `..` resolved
  instead of refused; no depth limit; a body sent for `HEAD`; the buffer
  cleared between requests; `exit` on shutdown; standard input ignored; no
  write timeout; one deadline for a whole response; a leaked pool slot; a
  queue that hands out the newest connection first; no lingering close; a
  lingering close without its time limit; a 503 that waits for the refused
  client; a date that is formatted once; and a "first evening" server that
  handles one connection at a time and has no time limits and no shutdown
  at all. 39 of them fail at least one acceptance test. None makes the
  test run hang: the slowest run took 19 seconds.
- The single-threaded variant fails six tests, all of them in
  `m6_pool.rs`. That is the promise of SPEC section 13: a server that
  handles one connection at a time passes M1 to M5.
- The one variant that passes on macOS sends each response with two small
  writes and without `TCP_NODELAY`. The test for it
  (`two_hundred_requests_take_far_less_than_four_seconds`) depends on the
  delayed-acknowledgement behaviour of the system, which is the
  well-known 40 ms on Linux and not observable on the macOS loopback
  interface.
- I did not run the server on Linux or Windows. The code uses no
  platform-specific API, and the places where the systems are known to
  differ are handled explicitly: the error kind of a timed-out read
  (`WouldBlock` or `TimedOut`), opening a directory, path separators and
  reserved names, and option calls on a dead socket.

## 12. What I would do next

- Replace the 50 ms poll. The honest fix is the other architecture:
  non-blocking sockets and a readiness API, which is what the mastery
  module [`m4_async`](../../../mastery/m4_async/README.md) builds.
- Give refused connections a lingering close on a helper thread.
- Cache the formatted `Date` for the current second; today it is
  formatted for every response.
- Borrow the head from the buffer if a profile says the allocations
  matter.
- Add the symbolic-link check (canonical path inside canonical root) as a
  second line behind the textual rule.
