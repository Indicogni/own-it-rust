# kvstore: design of the model solution

This is one solution, not the solution. Read it after you have finished
your own, and look for the places where you decided differently. For each
decision this document names the alternatives and says why they were not
taken. Several of them are good, and for a different set of limits they
would be better.

About 1 750 lines of code, comments included, and 850 lines of unit tests.
Standard library only, `#![forbid(unsafe_code)]`.

## 1. The layers

```text
main.rs      arguments in, exit code out
   |
lib.rs       run(): open the store, bind, print LISTENING, serve
   |
   +-- config.rs     the command line                       (no I/O)
   |
   +-- server.rs     sockets, threads, the connection limit, shutdown
   |      |
   |      +-- protocol.rs   bytes -> lines -> commands      (no I/O)
   |      |
   |      +-- store.rs      index + log behind one lock
   |             |
   |             +-- log.rs      the file: recover, append, read, compact
   |                    |
   |                    +-- record.rs   Key, Value, record bytes   (no I/O)
   |                           |
   |                           +-- crc32.rs                        (no I/O)
```

Dependencies point downward only. `log.rs` does not know which keys are
live. `store.rs` does not know what a request line is. `server.rs` does
not know what a record is. Four of the eight modules do no I/O at all, and
those four hold most of the rules of the SPEC; their unit tests run in
milliseconds.

## 2. The invariant

Everything is built around one sentence:

> The index in memory is exactly what replaying the log file gives, at
> every instant at which no write holds the lock.

`Store` keeps it by doing "append the record, then update the index" under
one exclusive lock, in that order (`Inner::set`, `Store::delete`). Log
first, because an index entry must never point at bytes that are not in
the file; and both under one lock, because otherwise two writers can pass
through the two steps in different orders (CONCEPTS section 8).

Recovery uses the same `Index::apply` that the running server's writes go
through, so there is one piece of code that defines what a record does to
the index and to `live_bytes`.

## 3. Decisions

### 3.1 Positions in memory, values on disk

The index maps each key to a `Slot`: the offset and length of its value in
`kv.log`. A `GET` looks up the slot and reads the value from the file.

*Alternative: keep the values in memory too.* Then `GET` never touches the
disk, there is no second file handle, compaction writes from the map, and
the program is about 80 lines shorter. With values of at most 4096 bytes
that is a perfectly reasonable design, and it is the one to choose if the
data set is known to fit in memory.

Positions were chosen because they are what makes this design scale past
memory: the index costs a few dozen bytes per key whatever the size of the
values, and start-up does not have to keep any value it reads. It also
makes the solution exercise the harder parts of the SPEC (every position
changes in a compaction; a reader must never see a slot whose bytes are
not yet in the file).

What it costs: a `GET` is a seek and a read, and an I/O error can now
happen on the read path. Such an error is treated like any other log
failure (section 3.10).

### 3.2 `BTreeMap` for the index

`KEYS` and compaction need the keys in byte order, and `KEYS <prefix>` is
a range in that order: start at the first key not smaller than the prefix,
stop at the first key that does not start with it (`Store::keys`).

*Alternative: `HashMap`, and sort when needed.* `GET` and `SET` become
O(1) instead of O(log n), and `KEYS` and compaction pay O(n log n) for a
sort each time; `KEYS <prefix>` has to look at every key. For a store
that is asked `KEYS` rarely, that trade is fine. The SPEC makes ordered
output part of two operations, so the ordered map does the work once, on
insertion.

Keys are stored as `Box<[u8]>`: they never change after insertion, so the
spare capacity and the third word of a `Vec` would be waste, multiplied by
the number of keys.

### 3.3 One `RwLock` around index and log

`Store` holds `RwLock<Inner>`. Reads (`GET`, `KEYS`, `STATS`) share it;
writes and compaction take it exclusively.

*Alternative: a `Mutex`.* Simpler, and correct. The `RwLock` lets
`KEYS` on a large store and many `GET`s proceed together.

*Alternative: separate locks for the index and the log.* This is the
design that breaks the invariant unless the two are always taken together,
at which point they are one lock with extra steps.

*Alternative: one thread owns the store and the others send it messages.*
No locks at all, a natural place for group commit, and `GET`s are
serialised behind writes. A good design for the async stretch goal.

**Known cost.** With `--fsync always` the sync happens while the write
lock is held, so readers wait for the disk. Measured here: about 300
writes per second, whatever the number of clients. The fix is group commit
(SPEC 16.1): release the lock after the append, and let one sync cover
every record appended before it. That needs a second notion of progress
("appended up to" and "synced up to") and a condition variable, and is
left as the stretch goal it is.

### 3.4 Two file handles and a small mutex

`Log` has a `writer` and a `reader` on the same file. The writer is used
only through `&mut self`, so the store's write lock already protects it.
The reader serves `GET`s through `&self`, from many threads; a read is a
seek followed by a read, and a `Mutex<File>` keeps those two calls of
different threads from interleaving.

*Alternative: positioned reads.* `std::os::unix::fs::FileExt::read_exact_at`
and `std::os::windows::fs::FileExt::seek_read` read at an offset in one
call, without a shared position and so without the mutex. That is the
better answer for throughput, and it needs a `#[cfg]` pair of functions.
This solution was developed and run on one platform only (section 6), and
a model answer should not contain code its author could not run; the
mutex version is the same on every system. The critical section is two
system calls.

*Why not one handle?* A handle has one position. Sharing it between the
appender and the readers would make every read move the place where the
next record goes.

*Why not `append(true)`?* Recovery has to shorten the file with `set_len`,
which Windows does not allow through a handle opened for appending only.
The writer is opened for writing, positioned once at the end of the valid
prefix, and from then on every write moves the position to the new end.

### 3.5 One `write` per record, no buffer

`Log::append` encodes the record into a reused `Vec` and passes it to
`write_all` in one piece. There is no `BufWriter` in front of the log.

- "Written before acknowledged" (SPEC 9.1) then needs no flush that
  someone could forget: when `append` returns, the kernel has the bytes.
- A record is never split between two system calls, so a crash cuts at
  most the last record, and cuts it at one place.

With `--fsync always`, `sync_data` follows. `sync_data` rather than
`sync_all` because the modification time is of no interest; the size,
which is needed to read the data back, is included by both.

### 3.6 Recovery

`Log::open` reads the file once through a 64 KiB `BufReader`, record by
record, into one reused buffer whose size is bounded by the format
(`record::body_len` returns at most 128 + 4096). Memory use beyond the
index does not depend on the size of the log.

Validation is split in two steps on purpose (`record::body_len`, then
`record::decode`). The first needs only the 11 header bytes and rejects
any kind and any length the format does not allow, so a damaged length is
never used to size a read. The second checks the checksum and then the
bytes of key and value. Every reason for "not a record" becomes the same
`None`, and the scan loop has one way to stop.

After the scan the file is shortened to the valid length, the write
position is set there, and, if anything changed, the file and its
directory are synced. Without the sync, a power failure after the next
acknowledged write could bring back the tail that was cut.

The header cases of SPEC 6 are one `if` chain in `Log::open`. The case
that must never go wrong is a file that is not ours: it returns
`OpenError::NotALog` before any call that could change the file.

### 3.7 Compaction

`Store::compact` takes the write lock, so nothing else runs. `Log::compact`
then:

1. writes `kv.compact`: the header, then, for each live key in index
   order, the key's **existing record copied byte for byte** from the old
   log, checksum included;
2. syncs it;
3. closes its own three handles (temporary file, writer, reader);
4. renames `kv.compact` to `kv.log`, and syncs the directory on Unix;
5. opens a new writer and reader, and returns the new position of every
   value.

The store swaps the positions into the index only after all of that has
succeeded.

Copying records instead of re-encoding them has two benefits. The record
of a live key is already the canonical SET record for it, so there is
nothing to compute. And each copied record is verified with the same
`record::decode` as in recovery, so a record that has gone bad on disk
since it was written stops the compaction with an error instead of being
carried into the new log with a fresh, valid checksum around bad data.

Handles are closed before the rename because Windows may refuse to
replace a file that is open. On Unix the old handles would simply keep
referring to the old content, which is the other reason they must be
reopened.

`Log::compact` takes `self` by value. If any step fails, the log object is
gone, the store's `Option<Log>` stays `None`, and the store is in its
failed state (section 3.10). On disk, `kv.log` is still either the
complete old log or the complete new one.

*Alternative: compact in the background.* Readers could continue against
the old file while the new one is written; writers either wait for the
switch or have their records copied over before it. That removes the
pause, which here lasts 0.6 seconds for a million keys, and it adds a
second generation of positions and a hand-over protocol. SPEC 16.2.

**Automatic compaction** runs inside the write that crosses the threshold,
before that write is acknowledged (`Store::after_write`). The rule of SPEC
10.3 then holds trivially, with no timer and no extra thread. The cost is
that one unlucky write waits for a whole compaction. A background thread
woken by a condition variable would move that cost away from the client;
the SPEC allows both.

### 3.8 `Key` and `Value` are types

`record::Key` and `record::Value` are borrowed byte slices that can only
be made through constructors that check the rules of SPEC 7. Everything
below the parser and the record decoder takes these types. There are two
ways into the system for outside bytes, the socket and the file, and each
has exactly one place where the check happens: `protocol::parse` and
`record::decode`. `Store::set` cannot be called with an unchecked key,
so it cannot write a record that the next recovery would reject.

The one `expect` in non-test code is in `Store::incr`, where a decimal
`i64` is turned into a `Value`; it cannot fail, and the message says why.

### 3.9 The protocol without sockets

`protocol::LineBuffer` is a state machine that is fed byte slices and
reports "need more", "a line is ready" or "a line that was too long has
just ended". It keeps at most 8192 bytes, whatever arrives. Because it
does no I/O it is tested by feeding the same input in pieces of every
size.

`protocol::parse` is SPEC 8.2 as a function from a line to
`Result<Command, ParseError>`, with the checks in the order of the SPEC.
`Command` borrows the key and the value from the line, so parsing
allocates nothing.

In `server::handle` the socket is wrapped in a `BufReader` and a
`BufWriter` (`&TcpStream` implements both `Read` and `Write`, so no clone
is needed). The writer is flushed at exactly one point: when the reader's
buffer is empty, which is the moment the next read would wait. A client
that pipelines 2000 requests gets its responses in a few large writes; a
client that sends one request gets its response immediately. Measured:
about 50 000 requests per second with one client waiting for each answer,
about 500 000 pipelined.

### 3.10 Errors, and what stops the server

Three kinds of failure are kept apart by type:

| type | meaning | effect |
|---|---|---|
| `config::UsageError` | bad command line | exit code 2 |
| `store::OpenError`, in `RunError::Start` | the server cannot start | exit code 3 |
| `store::StoreError`, in `RunError::Store` | the log failed while running | exit code 1 |

Inside a connection, `ClientError` separates a socket error (this client
is gone; end the thread) from a store error (stop the server). The exit
code is computed in one place, `RunError::exit_code`.

**Fail-stop, by construction.** When an append fails, `Inner::append`
sets the log to `None` before it returns the error. Every later operation
finds `None` and returns `StoreError::Failed`. So no write can be appended
behind a possibly torn record and then acknowledged, even if other threads
are already waiting for the lock. The thread that saw the error reports it
through `Shared::fail`, which starts the normal shutdown; `run` returns
the error and `main` exits with code 1.

*Alternative: try to repair in place* (truncate back to the last good
length and continue). If the disk has just failed a write, more writes to
it are not what the situation calls for.

A poisoned lock (a thread panicked while it held the store lock) is mapped
to `StoreError::Poisoned` and handled the same way: the state may be
half-changed, so nothing more is promised.

### 3.11 Threads

One thread per connection, started with `thread::Builder::spawn_scoped`
inside a `thread::scope` in `server::serve`.

- The threads borrow the store and the shared state. Nothing is in an
  `Arc` for the sake of a lifetime; the only `Arc` is around each socket,
  which really has two owners (section 3.12).
- The scope joins every thread before `serve` continues. "Wait for all
  connections to finish" is a closing brace.
- `Builder` is used instead of `Scope::spawn` because it returns an error
  when the system cannot create a thread, where `spawn` panics. A server
  should drop one connection in that case, not die.

*Alternative: a fixed pool of worker threads.* It bounds the number of
threads, and a connection that is idle still occupies a worker, so the
pool must be as large as the connection limit anyway. Thread per
connection with a limit is the same thing with less code.

*Alternative: non-blocking sockets and one thread, or an async runtime.*
The right answer for ten thousand connections, most of them idle. SPEC
16.9.

The connection limit is enforced in the accepting thread
(`Shared::admit`), before a thread exists. The refusal line is written to
a client from which nothing has been read; closing a socket with unread
input makes TCP reset the connection, and the client could lose the line.

### 3.12 Shutdown

At rest the server has one thread in `accept` and one thread per
connection in `read` (or, for a client that does not read, in `write`).
`SHUTDOWN` has to make each of those calls return.

1. The thread that parsed `SHUTDOWN` writes `OK`, flushes, and calls
   `Shared::stop`: it sets an `AtomicBool` and, if it was the first to set
   it, **connects to the server's own port**. Just before that it closes
   a spare handle on the listening socket that the server has held since
   it started (see "The reserved descriptor" below).
2. `accept` returns that connection. The accepting thread sees the flag
   and leaves its loop.
3. The accepting thread calls `shutdown(Both)` on every socket in the
   table of open connections. A blocked read returns 0, a blocked write
   fails, and each connection thread runs to its end. Only the accepting
   thread ever adds to that table, and it has stopped accepting, so no
   connection can be missed.
4. The scope ends when all threads have ended.
5. No thread is left that could send a response, so every acknowledged
   write is in the file. `Store::sync` makes it durable.
6. `serve` returns, `run` returns, `main` returns 0. Dropping the `Store`
   closes the lock file.

The table holds an `Arc<TcpStream>` for each connection, shared with the
thread that serves it. `try_clone` would also give a second handle, at
the price of a second file descriptor per connection, and descriptors are
the first limit a server with many clients runs into.

*Alternative: a non-blocking listener that polls the flag.* No connection
to itself, and either a busy loop or a delay on every accept.

*Alternative: read timeouts on every socket.* Each idle connection then
wakes up several times per second for as long as the server runs, to save
a table of sockets.

*Alternative: `std::process::exit(0)` after syncing.* It passes every
test. Other threads are cut off at arbitrary points, which is safe here
only because recovery tolerates anything; it is a crash with a friendly
exit code, and it would hide the day someone adds state that does need an
orderly end.

**The reserved descriptor.** The connection to itself needs a file
descriptor, and a server with many clients may have none left: the
default limit on macOS is 256 open files per process, the same number as
the default of `--max-clients`. Without a reserve the connection fails,
the accepting thread stays in `accept`, and the server never exits. This
was observed, not imagined: with the limit lowered to 20 (`ulimit -n 20`)
and 13 clients connected, an earlier version of this solution answered
`SHUTDOWN` with `OK` and then hung. So `serve` clones the listener once
(`TcpListener::try_clone`), never uses the clone, and `stop` drops it
right before it connects. If the accepting thread takes the freed
descriptor first, for a client that connected at that moment, that is
just as good: it then looks at the flag.

**Known limits.** A `SHUTDOWN` that arrives while a compaction of a very
large store is running waits for that compaction, which can take longer
than the 5 seconds of SPEC 12. At the limit of open files a compaction
can fail, because it opens the new log before it closes the old one; that
stops the server with exit code 1 and loses nothing (section 3.10).

### 3.13 The directory lock

`Store::open` opens `kv.lock` and calls `File::try_lock`. The `File` is
kept in the `Store` and does nothing else. The operating system drops the
lock when the handle is closed, which includes every way a process can
end. A separate file is locked, and not the log, because on Windows a
locked file cannot be read through other handles.

`kv.compact` is removed only after the lock is held. Before that, the file
might belong to a running server in the middle of a compaction.

## 4. What the unit tests cover

37 unit tests, next to the code:

- `crc32`: the three check values of SPEC 5.3; the table against the
  bitwise definition; feeding in pieces; every 1-bit and 2-bit change of a
  short record, and a sample of 4-byte damage, is detected.
- `record`: the rules for keys and values; the bytes of SPEC 5.5; a
  round trip at the limits; every single-byte change is rejected; records
  with a correct checksum and forbidden fields are rejected; absurd lengths
  are rejected from the header alone.
- `log`: a new file; append, read back, reopen; a tail cut at every
  length; every header case, including that a foreign file is not
  changed; appending after a cut; compaction.
- `store`: the STATS arithmetic; the lock; `KEYS` by prefix; `INCR`;
  compaction; the garbage bound of automatic compaction after every single
  write; a leftover `kv.compact`.
- `protocol`: every row of the table in SPEC 8.2; lines split at every
  possible place; the exact length limit; a line of four megabytes that
  must not make the buffer grow.
- `config`: defaults, any order, every usage error.

The acceptance tests were also run against 35 deliberately wrong versions
of this solution (a record acknowledged before it is written, recovery
that does not shorten the file, a checksum that is not checked, `INCR` in
two steps, the log and the index under different locks, a lock file that
merely exists, responses flushed too late, a new log that is put in place
before it is complete, a `KEYS` whose count and list come from two
instants, and so on). Each wrong version fails at least one acceptance
test.

The tests were also run against a second, independent implementation
with a different design: values in a `HashMap` in memory, one `Mutex`,
everything in the binary crate, automatic compaction from a background
thread that looks every 200 ms, and a shutdown that ends the process from
the thread that read `SHUTDOWN`. It passes all of them, and with its
threads switched off it passes M1 to M4, as SPEC 15 promises.

## 5. Measurements

Release build, Apple silicon laptop, internal SSD, one run each; a rough
guide, not a benchmark.

| what | result |
|---|---|
| `SET`, one client waiting for each response, `--fsync never` | about 54 000 per second |
| `GET`, same | about 40 000 per second |
| `SET`, one client pipelining a million requests, `--fsync never` | about 490 000 per second (the client was a Python script and is part of the time) |
| `SET`, one client, `--fsync always` | about 310 per second |
| recovery of a log with 1 050 000 records (34 MB) | 0.26 seconds until the LISTENING line |
| `COMPACT` with 1 000 000 live keys | 0.6 seconds |

`GET` is slower than `SET` here because it is two system calls (seek,
read) plus an allocation for the value, against one write call for a
`SET`.

## 6. What was not verified

- **Linux and Windows.** The solution and the tests were run on macOS
  only. The code avoids everything known to differ (no signals, no append
  mode, handles closed before a rename, a separate lock file, no assumption
  about what a read returns after `shutdown`), and the only `#[cfg]` is the
  directory sync in `log.rs`. That is care, not evidence.
- **Power failures.** The sync calls are where SPEC 9 wants them, and
  nothing here can test whether a device honours them.
- **I/O errors while running.** The fail-stop path (section 3.10) is
  exercised by reasoning and by the types, not by a test: `std` offers no
  portable way to make a write fail on demand.

## 7. What to do next

In order of value for the effort:

1. Group commit, which changes `--fsync always` from 300 writes per second
   to "as many as arrive during one sync".
2. Positioned reads behind `#[cfg]`, removing the reader mutex.
3. Compaction that does not block readers.
4. A `kvbench` binary, so that items 1 to 3 can be judged by numbers.
