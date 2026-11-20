# Checkpoint 4 specification: `seal`

[All checkpoints](../README.md) · [Rubric](RUBRIC.md)

`seal` is a library that tells you whether files have changed. It computes
a 64-bit number, a *digest*, from the bytes of a file. A text file, the
*manifest*, lists the digest of every file at the time it was written.
Later the library reads the files again, in parallel, and compares.

```text
af63dc4c8601ec8c  notes/a.txt
85944171f73967e8  photos/Zoë in Århus.jpg
cbf29ce484222325  empty file
```

The library has four parts: the digest and the manifest text (sections 2
to 4), the parallel check with threads (section 5), a small executor for
futures (section 6), and a future that waits for work on another thread
(sections 7 and 8).

This document is the contract. Where it says "must", a test checks it. The
public names and signatures are already in `src/lib.rs`, so that the tests
compile. Everything behind them is yours to design.

## 1. Rules that apply everywhere

1. **No function may panic** for any input, in a debug or in a release
   build, unless this document says so. No function may print.
2. **Text is UTF-8.** A *byte offset* counts bytes from the start of a
   string, starting at 0. Names may contain any characters.
3. **I/O errors are returned, not hidden.** One kind is special: an
   operation that fails with `io::ErrorKind::Interrupted` has done nothing
   and must be tried again. No function returns an `Interrupted` error.
4. **Nothing may wait in a loop that only checks again.** A thread that
   has nothing to do must sleep until another thread wakes it. Loops with
   `thread::sleep`, `thread::yield_now` or `std::hint::spin_loop` are not
   allowed.
5. Only the standard library may be used. `unsafe` is not allowed.

## 2. `digest`

```text
pub fn digest<R: Read>(reader: R) -> io::Result<u64>
```

Reads `reader` to its end and returns the digest of all its bytes. The
digest is the 64-bit FNV-1a hash:

```text
hash = 0xcbf29ce484222325
for every byte, in order:
    hash = hash XOR byte
    hash = hash × 0x100000001b3, keeping the lowest 64 bits
```

| bytes | digest |
|---|---|
| none | `0xcbf29ce484222325` |
| `a` | `0xaf63dc4c8601ec8c` |
| `foobar` | `0x85944171f73967e8` |

- A `read` call may return fewer bytes than the buffer has room for. Only
  `Ok(0)` means the end.
- The first error that is not `Interrupted` ends the reading and is
  returned as it is.
- `digest` works for input of any length with a fixed amount of memory
  (section 9).

## 3. `Entry`: one line of the manifest

```text
pub struct Entry { pub digest: u64, pub name: String }
impl Display for Entry
impl FromStr for Entry { type Err = EntryError; }
pub enum EntryError { Digest, Separator, Name, Escape(usize) }
```

### 3.1 The line format

```text
<digest> <space> <space> <escaped name>
```

- `<digest>` is the digest as exactly 16 hexadecimal digits.
- Then come exactly two space characters (U+0020).
- `<escaped name>` is the name, in which three characters are replaced by
  two characters each:

  | character in the name | in the line |
  |---|---|
  | backslash `\` | `\\` |
  | line feed (U+000A) | `\n` |
  | carriage return (U+000D) | `\r` |

  Every other character stands for itself. A line never contains a line
  feed or a carriage return.

### 3.2 `Display`

`Display` writes the line for an entry. The digest is written with the
digits `0` to `9` and `a` to `f`, with leading zeros. The entry
`Entry { digest: 255, name: "a\\b".to_string() }`, whose name has the three
characters `a`, `\`, `b`, is written as

```text
00000000000000ff  a\\b
```

### 3.3 `FromStr`

`line.parse::<Entry>()` reads a line. The checks run in this order, and
the first one that fails decides the error.

1. The line must start with 16 hexadecimal digits: `0` to `9`, `a` to `f`
   or `A` to `F`. If the line is shorter, or one of its first 16 characters
   is anything else: `EntryError::Digest`.
2. Two spaces must follow. Otherwise: `EntryError::Separator`.
3. At least one more character must follow. Otherwise: `EntryError::Name`.
4. The rest is the escaped name. Reading from the left, a backslash and
   the character after it stand for one character of the name, as in the
   table of 3.1. A backslash that is followed by any other character, or
   by nothing, is an error: `EntryError::Escape(offset)`, where `offset`
   is the byte offset of that backslash **in the line**.

Every other character of the rest is taken as it is, also a third space,
a tab, or a character that `Display` would never write.

For every entry with a name that is not empty,
`entry.to_string().parse::<Entry>()` gives back an equal entry.

### 3.4 `EntryError`

`EntryError` implements `std::error::Error` and has no source. `Display`
prints exactly:

| value | text |
|---|---|
| `EntryError::Digest` | `expected 16 hexadecimal digits` |
| `EntryError::Separator` | `expected two spaces after the digest` |
| `EntryError::Name` | `the name is empty` |
| `EntryError::Escape(21)` | `unknown escape at byte 21` |

## 4. Manifest files

```text
pub enum ManifestError { Io(io::Error), Line { number: usize, error: EntryError } }
pub fn read_manifest<R: BufRead>(reader: R) -> Result<Vec<Entry>, ManifestError>
pub fn write_manifest<W: Write>(out: W, entries: &[Entry]) -> io::Result<()>
```

### 4.1 `read_manifest`

- The input is split into lines at each line feed. A carriage return
  directly before a line feed belongs to the line ending, not to the line.
  The last line may end without a line feed.
- Lines are numbered from 1. Every line counts.
- A line with no characters is skipped.
- Every other line is parsed as in section 3.3. The entries are returned
  in the order of the lines.
- The first failure ends the reading:
  - the reader returns an error: `ManifestError::Io` with that error;
  - a line is not UTF-8: `ManifestError::Io` with an error of the kind
    `io::ErrorKind::InvalidData`. The line-reading methods of `BufRead`
    report it in exactly this way;
  - a line is not an entry: `ManifestError::Line` with the number of the
    line and the `EntryError`.
- **The reading stops there.** `read_manifest` asks the reader for more
  input only when it has dealt with all lines it already has. A problem
  that lies further on in the input is never seen.

### 4.2 `write_manifest`

Writes one line per entry, in order, each followed by one line feed.

- Everything is written, also when the writer accepts only a part of the
  bytes it is offered in one `write` call.
- The first error that is not `Interrupted` ends the writing and is
  returned as it is.
- A writer that returns `Ok(0)` for bytes it was offered can never take
  the rest. The writing ends with an error of the kind
  `io::ErrorKind::WriteZero`, as `Write::write_all` reports it.
- `write_manifest` does not flush and does not add buffering. The caller
  decides about both.

### 4.3 `ManifestError`

`ManifestError` implements `std::error::Error`, and `From<io::Error>`.

| value | `Display` text | `source()` |
|---|---|---|
| `ManifestError::Io(e)` | `cannot read the manifest` | `e` |
| `ManifestError::Line { number: 3, error }` | `line 3 of the manifest is not valid` | `error` |

## 5. Checking files in parallel

```text
pub enum Status { Match, Changed(u64), Unreadable(io::ErrorKind) }

pub fn check_all<F, R>(entries: &[Entry], workers: usize, open: F) -> Vec<Status>
where F: Fn(&str) -> io::Result<R> + Sync, R: Read;

pub fn check_dir(root: &Path, entries: &[Entry], workers: usize) -> Vec<Status>
```

`check_all` checks every entry and returns one `Status` per entry, **in
the order of `entries`**. To check an entry:

1. Call `open(&entry.name)`. An error `e` gives
   `Status::Unreadable(e.kind())`.
2. Compute the digest of the reader as in section 2. An error `e` gives
   `Status::Unreadable(e.kind())`.
3. If the digest equals `entry.digest`, the status is `Status::Match`.
   Otherwise it is `Status::Changed(actual)` with the digest that was
   computed.

`open` is called exactly once for each entry.

**Threads.** `workers` is the number of threads that check entries at the
same time. The thread that called `check_all` may be one of them. A value
of 0 means 1.

- **Really parallel.** If there are at least `workers` entries, then
  `workers` readers are read at the same time. The test uses readers that
  wait for each other.
- **Never more.** At no moment are more than `workers` readers alive. A
  reader is alive from the moment `open` returns it until it is dropped.
- **No fixed shares.** An entry does not belong to a worker in advance.
  Whenever a worker is free and an entry has not been opened yet, that
  worker takes it. One slow file must not keep other entries waiting.
- A reader is opened, read and dropped on one thread. That is why `R`
  needs neither `Send` nor `Sync`.
- When `check_all` returns, all threads it started have ended.

`check_dir` does the same for files: the reader of an entry is the file
`root.join(&entry.name)`, opened for reading.

## 6. `block_on`

```text
pub fn block_on<F: Future>(future: F) -> F::Output
```

Runs a future to its end on the calling thread and returns its output.

- `block_on` polls the future. When a poll returns `Pending`, the thread
  sleeps until the future's waker is used, then polls again.
- The waker may be used at any moment and from any thread: during the
  poll itself, before the thread has started to sleep, or much later. No
  use of the waker may be lost.
- The waker may be cloned and used after `block_on` has returned. That
  must do no harm.
- `block_on` must not poll in a loop without sleeping (rule 4). The test:
  a future that is woken once, after 100 ms, may be polled at most 10
  times.

## 7. `in_background` and `Background<T>`

```text
pub struct Panicked;
pub struct Background<T> { /* your design */ }
impl<T> Future for Background<T> { type Output = Result<T, Panicked>; }

pub fn in_background<T, W>(work: W) -> Background<T>
where W: FnOnce() -> T + Send + 'static, T: Send + 'static;
```

`in_background(work)` starts a new thread that runs `work`, and returns at
once. The thread starts whether or not the returned future is ever polled.
The future is the way to get the result.

- When `work` has returned `value`, the future completes with
  `Ok(value)`.
- When `work` has panicked, the future completes with `Err(Panicked)`.
  The future must not stay pending for ever, and the panic must not reach
  the thread that polls.
- `poll` never waits. If the result is not there yet, `poll` keeps a copy
  of the waker it was given and returns `Pending`.
- When the work ends, the waker of the **most recent** poll is used. A
  future that was polled with one waker and later with another must wake
  the second.
- No wake-up may be lost: if the work ends at the same moment as a poll,
  either that poll returns the result, or the waker of that poll is used.
- Dropping the future does not stop the work. The work runs to its end,
  and its result is dropped.
- Polling the future again after it has completed may panic.

`Panicked` implements `std::error::Error`. Its `Display` text is
`background work panicked`.

## 8. `digest_all`

```text
pub async fn digest_all<R>(readers: Vec<R>) -> Vec<io::Result<u64>>
where R: Read + Send + 'static;
```

Computes the digest of every reader and returns the results in the order
of `readers`.

- Every reader is read on a background thread of its own, as in section
  7, and **all of them are read at the same time**. The test uses readers
  that wait for each other.
- If reading a reader panics, its result is an `io::Error` of the kind
  `io::ErrorKind::Other`.

## 9. Cost

1. **`digest` uses a fixed amount of memory.** For input of any length it
   may request at most 64 KiB of heap memory in total. It reads in large
   pieces: for a reader that holds 1 MiB and always gives as many bytes as
   it is asked for, `digest` may call `read` at most 300 times.
2. **Formatting does not allocate.** `Display` for `Entry`, and
   `write_manifest`, must not allocate memory.
3. **Parsing allocates once.** `str::parse::<Entry>()` may allocate at
   most once (the name) and at most as many bytes as the line is long,
   and not at all when the error is `Digest`, `Separator` or `Name`.

## 10. What is not specified

Anything this document does not mention is your decision. You may add
derives, trait implementations, private items and files. You may not
remove or change a public name, a public field, or a signature.
