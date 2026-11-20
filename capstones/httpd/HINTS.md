# Hints for the `httpd` capstone

These are design hints. They contain no code, and they do not tell you
what to type. Each milestone has a first hint (a direction), a second hint
(more specific), and questions to check your understanding. Try for a
while before you read a hint, and read the second one only if the first
did not help.

The hints assume that you have read the sections of
[CONCEPTS.md](CONCEPTS.md) for the milestone. CONCEPTS.md explains the
ideas with examples from other settings; the hints are about this server.

When a test fails, read the test first. Every assertion has a message, and
the message names the section of SPEC.md that it checks. The requests in
the tests are written out as byte strings; you can send the same bytes to
your server by hand.

---

## Before M1 · The shape of the whole thing

**Hint 1.** Decide early which parts of the program need a socket and
which do not. Most of the difficult rules in the SPEC are about bytes:
"is this a valid head", "which file does this path name", "is this JSON".
None of these questions needs a network to be answered.

**Hint 2.**

- Write the socket-free parts as functions from bytes to a value or an
  error. You can then test them with ordinary unit tests, in
  milliseconds, with tables of inputs.
- Keep everything that blocks (accept, read, write) in a thin layer that
  calls those functions.
- Do not design the thread pool now. Until M6 your accept loop can serve
  each connection to its end before it accepts the next. If the function
  that serves one connection takes the connection and a reference to the
  shared settings, and nothing else, moving it onto a worker thread later
  is a small change.

---

## M1 · A process that answers

**Hint 1.** Three things happen before the first request: the command
line becomes a value, the root is checked, the socket is bound. Treat
"the command line is wrong" and "the server cannot start" as two
different kinds of failure from the beginning, because they have
different exit statuses.

**Hint 2.**

- Parsing the command line is a function from a list of arguments to
  either a configuration, a request for help, or an error that can
  describe itself. It should not print and it should not exit. `main`
  does both.
- Port `0` asks the system for a free port. The listener can tell you
  afterwards which one it got.
- Standard output is buffered. A parent process that waits for your line
  will wait until you flush it.
- For the first response, write the bytes of the SPEC example by hand,
  with the right length. Then replace the parts one by one with code.
  A single function that writes every response will pay off in each
  later milestone.

**Check your explanation.**

- Why is it wrong to print `listening on` with the port from the command
  line?
- What happens to your server when the process that started it closes
  the pipe to its standard output, and you print to it?
- Which exit status does your program have when `main` returns `Err`?
  Is that the status the SPEC asks for?

---

## M2 · The request head

**Hint 1.** There are two separate problems. The first is "where does the
head end?", and it must be solved while bytes are still arriving. The
second is "what does the head say?", and it can wait until the head is
complete. Solve them in two functions.

**Hint 2.**

- TCP gives you a stream of bytes, not messages. One `read` can return
  one byte, half a head, or two requests and the start of a third. Your
  connection needs one buffer that lives as long as the connection, and
  the rule "read until the buffer holds a complete head".
- The search for the end of the head is asked again after every read.
  Think about what it costs if it starts from the beginning each time,
  and what it must remember to avoid that. Remember that the four bytes
  it looks for can be split between two reads.
- SPEC 5.1 wants one fault to be reported while reading. The function
  that looks for the end of the head already looks at every byte.
- For the second problem, work on bytes, not on `String`. Header values
  may contain bytes that are not UTF-8. Split the head into lines, the
  first line at spaces, the others at the first colon.
- "Exactly one space" is easy to get wrong with a splitting function
  that skips empty pieces. Pick one that does not, and let the empty
  pieces be the error.
- Give the parser an error type with one variant per status code it can
  cause. The mapping from parser error to response is then a single
  `match` in the caller.
- For `HEAD`, do not write a second code path. Build the same response
  and decide at the last moment whether the body is sent.
- The date: the SPEC gives you one fixed point (which day of the week
  1970-01-01 was). The time of day and the day of the week are two
  divisions. The calendar date is the real work. One approach: write the
  slow version first (subtract whole years, then whole months) and test
  it against dates you know. A closed formula exists; look for
  "days from civil" if you want to study one.

**Check your explanation.**

- A client sends `GET / HTTP/1.1\n\n`. What does a server that only
  looks for `\r\n\r\n` do, and when?
- Why is `get` answered with 501 and `G@T` with 400?
- Why may a server not treat `Content-length` and `Content-Length` as
  two different headers?
- The head arrives in two reads, split between `\r\n` and `\r\n`. Walk
  through your code with these two reads.

---

## M3 · Files and path safety

**Hint 1.** The safety rule of SPEC 8.2 is a rule about text. You can
decide whether a request is allowed without asking the file system
anything, and you should: a decision that depends on what happens to be
on the disk is hard to test and easy to get wrong.

**Hint 2.**

- Write one function from the request path to either a list of safe
  names or an error with two variants (one per status). It needs no
  root directory and no disk. Its unit test is a table of attacks.
- The order "split, then decode" is the whole defence against encoded
  separators. Decode each piece by itself and look at the decoded piece.
- Decoding produces bytes. Only after all the bytes of a segment are
  there can you ask whether they are UTF-8: one character may be spread
  over several escapes.
- Ask the file system what a path is before you open it. Opening a
  directory behaves differently on different systems.
- The length you announce and the bytes you send should come from the
  same open file, not from two separate questions to the file system.
- Do not read a large file into memory to send it. The standard library
  has a function that moves bytes from a reader to a writer with a fixed
  buffer, and a way to stop a reader after a given number of bytes.
- Map file-system errors to statuses in one place. Most of them mean
  "there is no such file" to a client.

**Check your explanation.**

- `/..%2Fsecret.txt`: what are its segments under "split, then decode"?
  What would they be under "decode, then split"?
- Why does the SPEC forbid `:` and `\` on Linux, where both are ordinary
  characters in file names?
- Your server answers 403 for `/docs/../hello.txt`. A file system would
  have found the file. What do you gain by refusing?
- Between your question "is this a file?" and your `open`, another
  program replaces the file with a directory. What does your server
  send?

---

## M4 · Bodies, echo and persistent connections

**Hint 1.** For the connection, the important question is "where does
this request end?". Answer it once, in one place, before you look at
what the request wants. For the echo endpoint, notice what you do not
need: you never have to know what the JSON *means*.

**Hint 2.**

- After a request is answered, remove exactly its bytes from the
  buffer. What is left is the start of the next request. A fresh buffer
  per request loses it.
- The body is read for every method. If you skip that for `GET`, the
  body is parsed as the next request.
- Decide the length of the body from the head alone, as a function that
  returns a number or a status. Unit test it with a table.
- Do not reserve memory for a body because a header says how long it
  will be. Let the buffer grow as bytes arrive.
- For JSON, you do not need a tree of values. A scanner that walks the
  bytes once, knows what may come next, and copies tokens to the output
  as it goes is enough. "What may come next" is a small set of states.
- Depth is chosen by the client. If your scanner calls itself for every
  `[`, the client chooses how deep your call stack goes. Either count
  and stop, or keep the open brackets in a collection of your own. Then
  ask which of the two still works if someone raises the limit to a
  million.
- A number such as `01` needs no special case if the number rule stops
  after the `0` and the caller then finds a `1` where a comma or the end
  should be.
- Validate UTF-8 once for the whole body. Afterwards the scanner only
  has to care about ASCII bytes; everything else can only occur inside
  strings.
- Whether the connection stays open is decided from three facts (SPEC
  7.1). Compute it once, before the response is written, because the
  response must say it.

**Check your explanation.**

- Why does a 400 for invalid JSON keep the connection, while a 400 for
  a malformed head closes it?
- A client sends `Content-Length: 5` and then 9 bytes. What does your
  server do with the last 4?
- Why does the SPEC refuse two `Content-Length` headers even when they
  agree?
- Your scanner sees `tru`. In which state is it, and which rule fails?

---

## M5 · Limits and time

**Hint 1.** A limit is only a limit if it is applied before the cost is
paid: before the bytes are buffered, before the body is read, before the
thread has waited. Go through SPEC 5.4 and 7.3 and find, for each limit,
the earliest line of your code where it can be known that the limit is
exceeded.

**Hint 2.**

- A socket can be given a timeout for reads. That is a timeout per call.
  The SPEC wants a limit for the whole request. Compute the moment when
  the request must be complete once, and before each read ask how much
  time is left.
- The idle wait and the request wait differ in two ways: the length of
  the limit, and what happens when it ends. Make them two code paths
  that share the read.
- A timed-out read reports a different error kind on Unix and on
  Windows. Handle both.
- A write can block as well. Find the socket option for it.
- For SPEC 7.4, read the documentation of the method that closes one
  direction of a `TcpStream`. The reading that follows needs its own
  limits, or a client can keep the worker by sending slowly for ever.
- Setting an option on a socket can fail when the other side has
  already reset the connection. Decide what that means for each place
  where you set one. "Try again" is the wrong answer in a loop.

**Check your explanation.**

- A client sends one byte every 100 ms. How long does a server with a
  5 s timeout per read listen to a head of 8000 bytes?
- Why is there no response when the idle timeout ends, but a 408 when
  the request timeout ends?
- A client sends a 2 GB upload. Your server answers 413 after the head
  and closes. Describe what the client sees without SPEC 7.4, and why.
- Why is the lingering read limited in time *and* in bytes?

---

## M6 · The pool, back-pressure and shutdown

**Hint 1.** Build the pool as a small library of its own, with no
knowledge of HTTP or sockets: it holds a bounded number of work items
and runs a function on each. Then the three hard questions become
separate: what does "full" mean (the pool), what do you tell a client
when it is full (the server), and how does everything stop (the order of
a few calls).

**Hint 2.**

- The pool needs a queue that several threads take from and one thread
  puts into, and a way for an idle worker to sleep until there is work.
  Chapter 19 has two tools for this. Whichever you take, the admission
  decision ("is there room?") and the insertion must happen under the
  same lock, or two threads can both see the last free place.
- Decide what the pool counts. "Items in the queue" and "items in the
  queue plus items being worked on" give different answers at the edges;
  SPEC 10.2 says which one is meant. Think about the moment when a
  worker has finished an item and has not yet come back for the next.
- When the pool is full, the item must come back to the caller, because
  the caller still has something to do with it.
- A worker runs code that handles input from strangers. If that code
  panics, what happens to the worker thread, to the count, and to a lock
  that was held? Make each of the three answers a decision.
- Do not hold the pool's lock while a handler runs.
- The accept loop blocks in `accept`, and the standard library has no
  way to interrupt it. Two classic answers: make something arrive that
  `accept` will return, or do not block for ever. Each has a cost; find
  it.
- A worker that waits for a request blocks in `read`. Think about how it
  can notice a flag that another thread sets, and how quickly it must.
  Then compute what your answer costs per idle connection per second.
- The shutdown flag is a single fact that is set once and read by many
  threads. Ask what the weakest tool is that does this job correctly,
  and whether anything else must be visible at the same moment.
- Shutdown is an order of steps (SPEC 11.2). Write them in one function,
  in order, and make each step finish before the next begins. The last
  step is to wait for the workers. A program that exits while a worker
  still writes a response cuts that response off.
- Reading standard input blocks too, in a thread that can never be told
  to stop. Check what happens to such a thread when `main` returns.

**Check your explanation.**

- With `--workers 2 --queue 1`, three connections are open and idle. A
  fourth arrives. Who answers it, with what, and on which thread?
- Why must the thread that sends 503 not wait for the client to read it?
- A worker panics in the middle of a request. List everything that must
  still be true afterwards.
- At shutdown, one connection is waiting in the queue with a complete
  request in its socket buffer. The SPEC says it gets no response. Why
  is that acceptable for the client, and why is the alternative (serve
  everything that was accepted) harder to bound in time?
- Your process prints `shutting down` and a client connects one
  millisecond later. What does the client see, and which line of your
  code made that true?
