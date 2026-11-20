# Checkpoint 4 reference solution: commentary

The solution is `src/lib.rs` in this directory. Read it after your own
sitting, next to your code. This note explains the decisions that are not
obvious from the code, and what a weaker solution usually does instead.

## `digest`: one buffer and three cases

The loop has exactly the cases that the contract of `Read::read` has:

- `Ok(0)`: the end. Nothing else means the end. A read that returns 5
  bytes into a buffer of 8192 is normal for a pipe, a socket or a slow
  disk.
- `Ok(count)`: use `buffer[..count]`, never the whole buffer.
- `Err` of the kind `Interrupted`: nothing was read. Go round again.
  Every other error is returned.

The buffer is an array of 8 KiB on the stack, created once. The memory in
use is the same for 10 bytes and for 10 GiB. `read_to_end` would give the
same digest and needs as much memory as the file is long.
`reader.bytes()` on a reader without a buffer asks the operating system
for one byte at a time. Wrapping the reader in a `BufReader` first is a
correct alternative to the loop.

The multiplication is `wrapping_mul`, because "keep the lowest 64 bits" is
the specified behaviour. Plain `*` panics in a debug build.

## `Display`: write the pieces, build nothing

`fmt` writes the digest with `{:016x}` and then walks through the name:
`find` returns the next character that needs an escape, the text in front
of it goes to the formatter as one slice, then the two characters of the
escape, and so on. No escaped copy of the name exists at any time, so the
function does not allocate. Because `write_manifest` uses `writeln!` with
this `Display`, it does not allocate either.

The common alternative is
`self.name.replace('\\', "\\\\").replace('\n', "\\n")...`. It is short and
correct **in this direction**, and it allocates one `String` per `replace`.

## `FromStr`: three traps

1. **`&line[..16]` can panic.** A byte offset may fall inside a character.
   `split_at_checked` returns `None` in that case, and also when the line
   is too short. One call handles both.
2. **`u64::from_str_radix` accepts a leading `+`.** The solution checks
   `is_ascii_hexdigit` for all 16 bytes first. After that the conversion
   cannot fail. The code still maps its error instead of calling `unwrap`,
   so that the function has no path that can panic.
3. **Unescaping cannot be a chain of `replace`.** The line text `\\n`
   means a backslash and the letter `n`. Whichever replacement runs first,
   a chain gets some input wrong, because the second replacement looks
   again at text that the first one produced. An escape must be read from
   the left, once. The loop over `char_indices` does that, and the index it
   gives is a byte offset, which is what `Escape` reports.

The name is built in a `String::with_capacity(escaped.len())`. A name is
never longer than its escaped form, so the capacity is enough and the
function allocates once. The three checks that need no name come first, so
they cost nothing.

## `read_manifest`: stop where the problem is

The loop uses `read_line` with one `String` that is cleared and filled
again. `BufRead::lines` would also be correct. It creates a new `String`
for every line.

The important property is that both read **one line at a time**. A bad
line 3 is reported before the reader is asked for line 4. A solution that
calls `read_to_string` first reports a later I/O error instead of the bad
line, and holds the whole file in memory.

The line ending is removed with `strip_suffix('\n')` and then
`strip_suffix('\r')`. `trim_end()` would also remove spaces at the end of
a name, and those belong to the name. `?` on `read_line` works because
`ManifestError` implements `From<io::Error>`.

## `check_all`: scoped threads and one counter

**What crosses a thread boundary.** The workers need `entries`, `open`
and a way to find the next entry. All three are borrowed. `thread::scope`
allows that, because the scope does not end before its threads have
ended. `F: Sync` is exactly what makes `&open` usable from several threads.
A reader never crosses a boundary: `check_one` opens, reads and drops it
on one thread. That is why the signature can accept readers that are not
`Send`, and why the compiler rejects designs that open on one thread and
read on another.

**Who takes which entry.** `next` is an `AtomicUsize`. A worker calls
`fetch_add(1)`, and the number it gets back is its entry. When the number
is past the end, the worker stops. No entry belongs to a worker in
advance, so a worker that is stuck on one slow file delays only that file.
Splitting the slice into `workers` parts with `chunks` looks simpler and
fails that requirement.

**How results come back.** Each worker returns a list of
`(index, status)` through `join`. The caller writes them into a `Vec` that
already has the right length. A `Mutex<Vec<Status>>` shared by all workers
would also work; returning values needs no lock at all.

**How many threads.** `workers - 1` threads are spawned, and the calling
thread is the last worker. With `workers == 1` no thread is created. The
count is limited to the number of entries first, so `workers = 1000` with
three entries starts two threads.

If `open` or a reader panics, `join` returns the payload, and
`resume_unwind` passes it on as the caller's panic. `thread::scope` would
panic by itself in that case, with a less useful message.

## `block_on`: a flag and `park`

The waker is an `Arc<Signal>` with `impl Wake`. `Waker::from` turns it
into a `Waker`. Using the waker sets a flag and unparks the thread.

After a poll that returned `Pending`, the loop is

```text
while !flag.swap(false) { park(); }
```

The flag is what makes this correct:

- **A wake-up before the sleep.** If the waker was used during the poll,
  the flag is already set. `swap` returns `true` and the thread does not
  park at all.
- **A wake-up that is not ours.** `park` may return without a reason, and
  a waker of an earlier `block_on` on the same thread may still be around.
  Without the flag, each of these would cause an extra poll. With the
  flag, the thread parks again.

The version from chapter 21, lesson 04, which parks without a flag and
polls after every return from `park`, also passes the tests. The token of
`park` remembers one early wake-up by itself. The price is an extra poll
for every return that was not meant for this future. A `Condvar` without a
flag is different: `notify_one` before `wait` wakes nobody, and the thread
then sleeps for ever. A loop that polls again and again without sleeping
gives the right results and uses a whole processor core while it waits.

Both orderings are `Relaxed`. The flag is the whole message: no other
memory is read on the strength of it, so a stronger ordering has nothing
to protect.

`pin!(future)` pins the future on the stack of `block_on`. No `Box` and no
`unsafe` are needed, and the future does not have to be `Send` or
`'static`, because it never leaves this thread.

## `Background`: one lock for two questions

The thread and the future share an `Arc<Mutex<Stage<T>>>` with three
states:

```text
Running(Option<Waker>)  --thread stores the result-->  Done(result)
Done(result)            --poll takes the result---->   Fetched
```

`poll` locks, and then either takes the result or stores the waker of this
poll. The thread locks, replaces the state by `Done`, and takes the waker
out in the same step. Because both happen under the same lock, there is no
moment in which `poll` has seen "not done" and the thread has already
looked for a waker and found none. That moment is the classic lost
wake-up, and it produces a program that hangs once in a few thousand
runs.

Other points:

- `poll` always stores the waker of the **current** poll. A future may be
  moved to another task between two polls, and then the old waker wakes
  the wrong task.
- The waker is used **after** the lock is released. A waker may run the
  future at once, and the future would then try to take the same lock.
- The work runs inside `catch_unwind`. Without it, a panic ends the thread
  before it can store anything, and the future stays `Pending` for ever. A
  guard value whose `Drop` stores `Err(Panicked)` is another good way to
  get the same effect.
- When the future is dropped, the thread still holds its `Arc`. It stores
  the result, finds no waker and ends; the last `Arc` goes away and drops
  the result. Nothing leaks and nothing special had to be written.
- `Background<T>` has no pinned data. `poll` only needs `&self.shared`, so
  there is no pin projection.

A design with a channel also works: the thread sends the result, and
`poll` calls `try_recv`. It still needs a shared slot for the waker, and
that slot and the channel are then two things that must be kept in step.
One `Mutex` around one enum is easier to get right.

## `digest_all`: start everything, then wait

`in_background` starts its thread when it is called. So the function first
turns every reader into a `Background` and collects them into a `Vec`.
Only then does it `await` them, one after the other. While the task waits
for the first result, the other threads keep reading.

Writing `in_background(..).await` inside one loop starts the second reader
only after the first has finished. It gives the same results, and runs one
reader at a time.

Note the `collect`. Iterator adapters are lazy: without it, `map` would
start each job only when the second loop asks for it, and the code would
be sequential again although it does not look like it.

## Tests in the solution

The `tests` module checks the parts that the public tests cannot reach or
that were easy to get wrong: the escape table in both directions, the byte
offset of an escape error, the line ending rule, the number of threads
that `check_all` really uses, and the stages of `Background`.
