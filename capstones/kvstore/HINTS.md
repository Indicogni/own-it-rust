# Hints for the kvstore capstone

These are design hints. They contain no code, and they do not tell you
what to type. Each milestone has a first hint (a direction), a second hint
(more specific), and questions to check your understanding. Try for a
while before you read a hint, and read the second one only if the first
did not help.

When a test fails, read the test first. Every assertion has a message that
names the SPEC section it checks, and a failed test keeps its data
directory so that you can look at the log file.

---

## Before M1 · The shape of the program

**Hint 1.** Draw the path of one `SET` from the socket to the disk and
back: bytes arrive, a line is cut out, the line becomes a command, the
command changes the store, the store appends to the log, a response goes
out. Each arrow is a possible module boundary. Decide which of these
steps need to know about sockets, which about files, and which about
neither. The steps that need neither are the ones you can unit-test in
milliseconds.

**Hint 2.**

- Give the store one small interface (get, set, delete, increment, list,
  statistics, compact) and let the network side know nothing else about it.
  In M1 the store is a map in memory. In M2 it gains a log. The network
  side should not change when that happens.
- Put code in the library crate and keep `main.rs` to three steps: read the
  arguments, run, turn the result into an exit code. The exit codes of
  SPEC 3.4 map well onto the variants of one error type.
- Decide early how a thread that serves a connection reaches the store. It
  can share a reference, own a handle, or send messages to one thread that
  owns everything. Each works. Write down which one you chose and why.

**Check your explanation.**

- Which parts of your design would stay the same if the protocol were
  replaced by a different one? If the log format were?
- Where is the one place that decides the exit code?

---

## M1 · Protocol

**Hint 1.** The protocol is made of bytes, not text. Look at which reading
methods of the standard library insist on UTF-8 and which do not, and
choose before you write the first loop. Then treat SPEC 8.2 as an
algorithm: six numbered steps, in order. Translate it step by step and
resist the urge to reorganise it.

**Hint 2.**

- "Split at the first space" appears twice in SPEC 8.2, and the difference
  between "no rest" and "an empty rest" matters both times. A small helper
  that returns the part before the space and an *optional* part after it
  makes every row of the table in 8.2 fall out naturally.
- The line length limit is easiest if the code that collects a line knows
  three states: collecting, skipping a line that is already too long, and
  just finished. Write it so that it can be fed arbitrary pieces of input,
  and test it by feeding the same input one byte at a time and all at once.
- A buffered writer makes pipelining fast and creates one trap: a response
  that sits in the buffer while you wait for the next request. Find the
  one moment in your read loop when you are about to wait, and flush
  exactly there.
- For the arguments, collect each option into an `Option` and reject a
  second occurrence. Parse everything before you touch the file system:
  the test checks that a usage error leaves no directory behind.
- `SHUTDOWN` in M1 only has to work when nobody else is connected. Do the
  simplest thing that makes the test pass, and write a note that M6 will
  replace it.

**Check your explanation.**

- Why is the request `GET a b` a bad key and not a syntax error?
- A client sends `SET k v` and the line feed in two separate packets, one
  second apart. What does your server do in that second?
- What does your server do with a line of one gigabyte? How much memory
  does it use while that line arrives?

---

## M2 · The log

**Hint 1.** Build the record format as functions on bytes first, away from
any file: one that appends the bytes of a record to a buffer, one that
takes bytes apart again. Test them against the example in SPEC 5.5 until
your bytes match exactly. Only then open a file.

**Hint 2.**

- For the checksum, start with the loop in SPEC 5.3 exactly as written and
  check the three given values. Make it faster afterwards, with the slow
  version as the test for the fast one.
- Build each record completely in memory and give it to the file in one
  write call. Think about what the file looks like if the process dies
  between two write calls of the same record, and what it looks like if
  two threads each write a record in two calls.
- A buffered writer between you and the log file is a mistake here, even
  though it was right for the socket. Work out why the two cases differ:
  who is waiting, and for what promise?
- Recovery in M2 is "read records until the file ends". Give the reading
  code the same record functions that the writing code uses, in reverse.
  Read through a buffered reader: a system call per field is slow.
- For the lock, look in the documentation of `std::fs::File` for methods
  with "lock" in the name. Lock a separate file, not the log: on some
  systems a locked file cannot be read by other programs, and the tests
  read the log.
- `log_bytes` and `live_bytes` are running totals. Find the three places
  where each changes and update them there; do not recompute them.

**Check your explanation.**

- After `OK` for a `SET`, where exactly are the bytes of the record if
  `--fsync never` is set? What can still lose them?
- Why does every `SET` append a record even if the value is unchanged?
  What would a server that skips it have to get right?
- Why is the checksum over `kind`, the lengths, the key and the value, and
  not only over the value?

---

## M3 · Recovery

**Hint 1.** SPEC 5.4 is a list of seven conditions, and SPEC 6 says: stop
at the first offset where they do not all hold. Write one function that
answers "is there a valid record here, and if so how long is it?" and make
every reason for "no" look the same to its caller. Recovery is then a
short loop around it.

**Hint 2.**

- Check in the order of the list. The conditions on `kind` and the lengths
  need only the 11 header bytes. Until they have passed, the lengths are
  numbers from an untrusted file and must not be used to size a buffer or
  a read.
- "The file ends before the record does" is not an error. It is the most
  common reason to stop. Make sure your reading code can tell "the input
  ended" from "reading failed", and treats only the second as a failure
  to start.
- After the loop you know the valid length. Three things must be true
  before you accept connections: the file has that length, your writing
  position is at that length, and (for a careful solution) the shortening
  has been synced.
- The header has its own small set of cases (SPEC 6, step 2). Write them
  as a table in a comment first. The dangerous case is a file that is not
  yours: make sure no path through your code can shorten or overwrite it.
- Read the test `m3_records_that_were_cut_off_never_come_back` and work
  out on paper what it would observe if you moved the writing position
  back but did not shorten the file.

**Check your explanation.**

- A crash can leave a half-written last record. Can it leave a damaged
  record in the middle of the log? What else can?
- Recovery drops valid records that come after a damaged one. Give one
  argument for this rule and one against it.
- Why must a record whose checksum is correct still be rejected if its key
  contains a space?

---

## M4 · Compaction

**Hint 1.** A compaction has two halves that are almost independent:
producing the new file, and switching to it. The first is ordinary work
that may take long. The second must be one step that either has happened
or has not. Find the operation in `std::fs` that replaces one file by
another under an existing name, and build the procedure of SPEC 10.2
around it.

**Hint 2.**

- What goes into the new file is exactly what `KEYS` would list, with the
  current values, in the same order. If your index is not ordered, this is
  the moment to ask whether it should be.
- List every open handle your process has on the old log. Each of them
  must end up on the new file, and on some systems the old file cannot be
  replaced while a handle on it is open. Decide the order: close, rename,
  open.
- If your index stores positions in the log, every position changes. Know
  the new position of each record at the moment you write it, and swap the
  whole index only when the switch has succeeded.
- For the crash cases, write down the content of the directory after each
  step of your procedure, and what recovery does with it. There should be
  no step after which recovery could pick the wrong file.
- Automatic compaction needs one comparison after each write. Where you
  run the compaction (in the write, or in a background thread) is a design
  decision with consequences for latency and for the complexity of M5. The
  simplest correct choice is fine.

**Check your explanation.**

- Why is it always safe to delete a `kv.compact` found at start-up?
- A power failure hits right after the rename, and the new file was not
  synced before it. What can `kv.log` contain when the machine comes back?
- During a compaction, where do new writes go in your design?

---

## M5 · Concurrency

**Hint 1.** State the invariant before you choose the locks: *the order of
records in the log is the order in which the index was updated.* Any
design in which a thread can be between "appended" and "index updated"
while another thread does either of those steps breaks it. The simplest
way to keep an invariant about two things is to protect both with the same
lock.

**Hint 2.**

- One thread per connection is a perfectly good design for the 64 clients
  of the SPEC. Think about how each thread reaches the shared store, and
  how you will later wait for all of them (M6). The standard library has a
  way to start threads that are guaranteed to end before a given block
  ends; it removes a whole class of lifetime problems.
- `INCR` is a read followed by a write. Under which lock does the read
  happen in your code, and is it still held at the write?
- A reader-writer lock lets `GET`s run in parallel. Check what a `GET`
  needs if values are read from the file: a shared file position is shared
  state too.
- Never hold a store lock while writing to a client. A client that stops
  reading would then stop the whole server. Copy what you need, release,
  then write.
- `KEYS` answers from one instant (SPEC 8.8). Collect the whole list while
  you hold the lock once. If you count first and list later, or list in
  pieces, another client can change the store in between, and the count
  no longer fits the lines.
- Ask what happens to a `SET` that arrives while a compaction runs. If it
  can be appended to the old log after the compaction has decided what to
  copy, it is lost at the switch. The simplest correct design makes
  writers wait until the switch is done.
- For the connection limit, count where you accept, before a thread is
  started, and make sure the place is given back on every way out of the
  connection code, including errors.
- A test that fails one run in ten is failing. Run the file in a loop.

**Check your explanation.**

- Two clients send `SET k a` and `SET k b` at the same time. List every
  final state that is allowed, for the store and for the log together.
- With `--fsync always`, where does a `GET` wait while another client's
  `SET` is being synced? Is that allowed by SPEC 11.2?
- What happens to the other connections if one connection thread panics
  while it holds your lock?

---

## M6 · Shutdown

**Hint 1.** At the moment `SHUTDOWN` arrives, make a list of every thread
in your server and the exact call it is blocked in: one in `accept`,
several in a read from a socket, perhaps one in a write to a client that
does not read. Shutting down means making each of these calls return. For
each, find what can be done *from another thread* to make that happen.

**Hint 2.**

- A blocked `accept` returns when a connection arrives. Who could connect?
- A blocked read or write on a socket returns when the socket is shut
  down. To shut a socket down from another thread, that thread needs its
  own handle on the same socket. Keep a registry of those handles, and
  think about which thread adds to it, which removes from it, and whether
  a connection can slip in after you have closed "all" of them.
- Order the steps so that the promise of SPEC 12 holds: after the last
  connection thread has ended, no further response can be sent, so that is
  the moment when "everything acknowledged is in the log" becomes a fact
  you can sync.
- Ending the process abruptly from the thread that received `SHUTDOWN`
  can pass the tests and is the weakest design: think about what the other
  threads may be in the middle of. Aim for a `main` that returns.
- Send `OK` before you start closing sockets, not after.

**Check your explanation.**

- After the `OK` for `SHUTDOWN`, another client's `SET` is being executed.
  Which outcomes does SPEC 12 allow for that client?
- Why does the log have to be synced at shutdown even with
  `--fsync always`? (Is it really necessary then? What does it cost?)
- Your server is killed during its shutdown. What is lost?

---

## After M6

Read your own code once from top to bottom as if someone else had written
it. Then go through RUBRIC.md, write your one-page `DESIGN.md`, and compare
with the model solution.
