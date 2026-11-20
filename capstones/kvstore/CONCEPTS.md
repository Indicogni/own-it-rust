# Concepts for the kvstore capstone

Read this after SPEC.md and before you design. Come back to a section when
you reach the milestone that needs it.

**What you will be able to do afterwards.** Explain why an append-only log
is the simplest way to make data survive a crash; define a binary record
format and read it back safely from a file you do not trust; say exactly
what "the data has been written" means at each of three levels; replace a
file so that a crash cannot leave half of it; read a network connection as
a stream of bytes; share one store between many threads without breaking
the order of its log; and stop threads that are blocked in the operating
system.

**What is assumed.** Chapters 03 to 11 and 16 to 19 of the course:
integers and byte slices, ownership and borrowing, error handling, structs
and enums, traits, collections, text, files and sockets, threads and locks.
No knowledge of databases or storage is assumed.

**Time (estimates).** About 15 to 25 minutes per section; three hours for
the whole file if you type in and change the examples, which you should.

| section | read it before |
|---|---|
| 1. A log is the data | M2 |
| 2. Records made of bytes | M2 |
| 3. Checksums | M2 |
| 4. What "written" means | M2 |
| 5. Reading a file you do not trust | M3 |
| 6. Replacing a file in one step | M4 |
| 7. A connection is a stream of bytes | M1 |
| 8. One store, many threads | M5 |
| 9. Stopping threads that are blocked | M6 |
| 10. One process per directory | M2 |

The examples here are deliberately different from the things you build.
They show the idea in a smaller or neighbouring setting; carrying it over
is your work. Every example in this file is compiled and run by
`rustdoc --test`.

---

## 1. A log is the data

There are two ways to keep a changing set of data on a disk.

The first is to keep the *current state* in a file and change the file in
place when the state changes. This is what you would do without thinking,
and it has a hard problem: a change usually touches several places in the
file, and a crash between two of them leaves a file that is neither the old
state nor the new one.

The second is to never change anything. Every change is written as a small
description of itself, a **record**, at the end of a file that only grows.
That file is a **log**. The current state is not stored anywhere; it is
what you get when you start with nothing and apply all records in order.

```text
log:    [ +5 apples ] [ +2 pears ] [ -3 apples ] [ pears: none ]
                                                                 ^ new records go here

state after replaying:   apples = 2
```

The log has three properties that make it worth the detour:

- **A crash can only damage the end.** Everything before the record that
  was being written is exactly as it was. Section 5 shows how to find the
  damaged end.
- **Writing is one operation in one place.** Appending is the cheapest
  thing a disk can do, and it is easy to make atomic enough.
- **The order of events is kept.** That is what makes replaying possible.

It also has two costs. Reading the state means replaying the log, so a
program keeps the state, or an **index** into the log, in memory and
replays only when it starts. And the log grows without limit, so now and
then it must be replaced by a shorter one that produces the same state.
That is **compaction**.

Here is the idea in a setting other than key-value pairs: the stock of a
small shop, as a log of movements.

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
enum Movement {
    Received { item: &'static str, count: u32 },
    Sold { item: &'static str, count: u32 },
    Discontinued { item: &'static str },
}

/// The state is a function of the log, and of nothing else.
fn replay(log: &[Movement]) -> BTreeMap<&'static str, u32> {
    let mut stock = BTreeMap::new();
    for movement in log {
        match *movement {
            Movement::Received { item, count } => *stock.entry(item).or_insert(0) += count,
            Movement::Sold { item, count } => {
                if let Some(have) = stock.get_mut(item) {
                    *have = have.saturating_sub(count);
                }
            }
            Movement::Discontinued { item } => {
                stock.remove(item);
            }
        }
    }
    stock
}

/// A shorter log with the same state: one record per item that is left.
fn compact(log: &[Movement]) -> Vec<Movement> {
    replay(log)
        .into_iter()
        .map(|(item, count)| Movement::Received { item, count })
        .collect()
}

fn main() {
    let log = vec![
        Movement::Received { item: "apples", count: 5 },
        Movement::Received { item: "pears", count: 2 },
        Movement::Sold { item: "apples", count: 3 },
        Movement::Discontinued { item: "pears" },
    ];
    let stock = replay(&log);
    assert_eq!(stock, BTreeMap::from([("apples", 2)]));

    let short = compact(&log);
    assert_eq!(short.len(), 1);
    assert_eq!(replay(&short), stock, "compaction must not change the state");

    // Any beginning of the log is also a state the shop was really in.
    assert_eq!(replay(&log[..2]), BTreeMap::from([("apples", 5), ("pears", 2)]));
}
```

The last assertion is the important one for crash recovery. If a crash
cuts the log short, what is left is still a state the system really had at
some earlier moment. It is never a mixture.

**Mistakes people make.**

- Treating the map in memory as the truth and the log as a backup that is
  written "also". The log is the truth. A change that is in memory but not
  in the log has not happened yet.
- Updating memory first and the log second. If the append fails, memory
  already shows something that will be gone after a restart.

**Read more.**
[`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html),
[`Entry`](https://doc.rust-lang.org/std/collections/btree_map/enum.Entry.html).

---

## 2. Records made of bytes

A file is a sequence of bytes, and nothing in it says where one record ends
and the next begins. The format has to say it. The two common ways are a
**terminator** (a byte that cannot occur inside a record, like the line
feed at the end of a line) and a **length prefix** (the record starts with
a number that says how long it is). A format for arbitrary bytes needs the
length prefix.

A number in a file needs two decisions: how many bytes, and in which order.
**Little-endian** means the least significant byte comes first: the 32-bit
number `0x0000012C` (300) is the four bytes `2C 01 00 00`. Rust has methods
for both directions on every integer type, and they are the only correct
way to do it; do not write the shifts yourself.

The example is a log of temperature readings. A record is a fixed header
(8 bytes of time, 2 bytes of length) followed by the name of the sensor.

```text
offset  size  field
     0     8  seconds     u64, little-endian
     8     2  name_len    u16, little-endian
    10     n  name        n = name_len bytes of UTF-8
```

```rust
#[derive(Debug, PartialEq)]
struct Reading {
    seconds: u64,
    sensor: String,
}

const HEADER: usize = 10;
const MAX_NAME: usize = 64;

fn encode(reading: &Reading, out: &mut Vec<u8>) {
    let name = reading.sensor.as_bytes();
    assert!(name.len() <= MAX_NAME, "the writer enforces the limit");
    out.extend_from_slice(&reading.seconds.to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(name);
}

/// Returns the reading and the number of bytes it used, or `None` if no
/// complete, sensible record starts at the front of `bytes`.
fn decode(bytes: &[u8]) -> Option<(Reading, usize)> {
    let header: &[u8; HEADER] = bytes.get(..HEADER)?.try_into().ok()?;
    let seconds = u64::from_le_bytes(header[0..8].try_into().ok()?);
    let name_len = usize::from(u16::from_le_bytes(header[8..10].try_into().ok()?));
    // Check the length BEFORE using it.
    if name_len > MAX_NAME {
        return None;
    }
    let name = bytes.get(HEADER..HEADER + name_len)?;
    let sensor = String::from_utf8(name.to_vec()).ok()?;
    Some((Reading { seconds, sensor }, HEADER + name_len))
}

fn main() {
    let mut file = Vec::new();
    encode(&Reading { seconds: 300, sensor: "attic".into() }, &mut file);
    encode(&Reading { seconds: 301, sensor: "cellar".into() }, &mut file);
    assert_eq!(&file[..10], [0x2C, 0x01, 0, 0, 0, 0, 0, 0, 5, 0]);

    let (first, used) = decode(&file).unwrap();
    assert_eq!(first, Reading { seconds: 300, sensor: "attic".into() });
    let (second, _) = decode(&file[used..]).unwrap();
    assert_eq!(second.sensor, "cellar");

    // Half a record is not a record.
    assert_eq!(decode(&file[..12]), None);
    // A length that the format does not allow is not a record either.
    let mut bad = file.clone();
    bad[8..10].copy_from_slice(&u16::MAX.to_le_bytes());
    assert_eq!(decode(&bad), None);
}
```

Three things in `decode` are worth copying. It uses `get(..)`, which
returns `None` instead of panicking when the slice is too short. It turns a
slice into an array with `try_into` before calling `from_le_bytes`. And it
compares the length with the limit before the length is used for anything.

**Mistakes people make.**

- Passing a slice where an array is needed. `from_le_bytes` takes
  `[u8; 4]`, not `&[u8]`:

  ```rust,compile_fail,E0308
  fn main() {
      let header = [0x2A, 0, 0, 0, 7, 0];
      let length = u32::from_le_bytes(&header[0..4]);
      println!("{length}");
  }
  ```

  ```text
  error[E0308]: mismatched types
      --> src/main.rs:3:37
       |
     3 |     let length = u32::from_le_bytes(&header[0..4]);
       |                  ------------------ ^^^^^^^^^^^^^ expected `[u8; 4]`, found `&[{integer}]`
       |                  |
       |                  arguments to this function are incorrect
  ```

  The fix is `header[0..4].try_into()`, which gives a
  `Result<[u8; 4], _>`, or a pattern that takes the array apart:
  `let [a, b, c, d, ..] = header;`.

- Trusting a length from the file. A reader that does
  `vec![0; length]` with a damaged length of four thousand million tries
  to allocate four gigabytes. The writer's limit must be the reader's
  check.

- Using `usize` in a file format. Its size depends on the machine. A file
  format names exact widths: `u16`, `u32`, `u64`.

**Read more.**
[`u32::to_le_bytes`](https://doc.rust-lang.org/std/primitive.u32.html#method.to_le_bytes),
[`u32::from_le_bytes`](https://doc.rust-lang.org/std/primitive.u32.html#method.from_le_bytes),
[`slice::get`](https://doc.rust-lang.org/std/primitive.slice.html#method.get),
[arrays and `TryFrom`](https://doc.rust-lang.org/std/primitive.array.html).

---

## 3. Checksums

A length tells you where a record should end. It does not tell you whether
the bytes are the ones that were written. A crash can leave the first half
of a record followed by whatever was in that part of the disk before; a
failing disk can flip bits. A **checksum** is a small number computed from
the bytes and stored with them. The reader computes it again and compares.

A checksum cannot prove that data is intact; many different inputs have
the same checksum. What matters is which *changes* it is guaranteed to
notice. The simplest checksum, the sum of all bytes, notices any single
changed byte but not two bytes that swap places. A better one mixes in the
position of each byte.

```rust
/// The sum of all bytes, modulo 256.
fn sum8(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |sum, &b| sum.wrapping_add(b))
}

/// Fletcher-16: a second sum adds up the running values of the first, so
/// a byte counts more the earlier it comes.
fn fletcher16(bytes: &[u8]) -> u16 {
    let (mut low, mut high) = (0u16, 0u16);
    for &b in bytes {
        low = (low + u16::from(b)) % 255;
        high = (high + low) % 255;
    }
    (high << 8) | low
}

fn main() {
    let written = b"pay 12 to account 34";
    let swapped = b"pay 21 to account 34";
    assert_eq!(sum8(written), sum8(swapped), "a plain sum cannot see a swap");
    assert_ne!(fletcher16(written), fletcher16(swapped));
}
```

A **CRC** (cyclic redundancy check) goes further. It treats the data as one
long binary number and takes the remainder of a division by a fixed
constant, in an arithmetic where addition is XOR. The CRC-32 of this
project notices every change of one bit, every change of two bits, and
every run of damage up to 32 bits long, whatever the bytes around it are.
Larger damage slips through only by chance, about once in four thousand
million times. That covers what crashes and disks usually do. SPEC 5.3 gives the algorithm you need as a loop over
bits. You do not need the mathematics to implement it; you need the check
values.

The loop over bits does eight steps per byte. Those eight steps depend
only on eight bits of the state, so their combined effect can be worked
out once for each of the 256 possible values and stored in a table. Rust
can build such a table **at compile time**. The example does that for a
different function, the number of one-bits in a byte:

```rust
/// `ONES[b]` is the number of one-bits in `b`, computed by the compiler.
const ONES: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut value = 0;
    while value < 256 {
        let mut bits = value;
        while bits != 0 {
            table[value] += (bits & 1) as u8;
            bits >>= 1;
        }
        value += 1;
    }
    table
};

fn main() {
    assert_eq!(ONES[0b1011_0000], 3);
    for value in 0..=255u8 {
        assert_eq!(u32::from(ONES[usize::from(value)]), value.count_ones());
    }
}
```

Inside a `const` block only a subset of Rust is allowed: `while` loops but
not `for` loops (a `for` loop calls a trait method, which is not `const`).
The table is part of the program file; it costs nothing at run time and
needs no initialisation and no lock.

**Mistakes people make.**

- Putting the checksum over the payload only. If the length field is not
  covered, a damaged length sends the reader to the wrong place, where the
  "checksum" is four arbitrary bytes.
- Thinking a correct checksum means valid content. It means "these are the
  bytes that some program wrote". The reader still has to check that they
  make sense (SPEC 5.4, condition 7).
- Testing a checksum only against itself. Encode-then-decode succeeds with
  any function, even a wrong one. Use known values from the specification.

**When to use which.** A CRC is for accidents. It is fast and tiny, and
anyone can produce a matching one, so it gives no protection against a
person who changes the data on purpose. That needs a cryptographic hash or
a signature, which this project does not.

**Read more.**
[Constant evaluation](https://doc.rust-lang.org/reference/const_eval.html)
in the Rust Reference,
[`u32::count_ones`](https://doc.rust-lang.org/std/primitive.u32.html#method.count_ones),
[`u8::wrapping_add`](https://doc.rust-lang.org/std/primitive.u8.html#method.wrapping_add).

---

## 4. What "written" means

When your program has data that should be in a file, the data can be in
one of three places.

```text
 your process            the operating system           the storage device
+---------------+  write  +---------------------+  sync  +----------------+
| a buffer, for | ------> | the page cache: the | -----> | flash or disk  |
| example in a  |         | file as other       |        |                |
| BufWriter     |         | programs see it     |        |                |
+---------------+         +---------------------+        +----------------+
 lost when the             lost when the machine          survives a power
 process dies              loses power or crashes         failure
```

1. **In your process.** A `BufWriter` collects small writes in memory to
   save system calls. Until it is flushed, the operating system has never
   heard of the bytes. If the process is killed, they are gone.
2. **In the operating system.** After a `write` call returns, the bytes
   are part of the file as far as every program on the machine can tell,
   and they stay so if your process dies. But the system keeps them in
   memory and writes them to the device when it suits it, often seconds
   later.
3. **On the device.** `File::sync_data` and `File::sync_all` ask the
   system to send the file's data to the device now and return when the
   device has confirmed it. This is slow: a sync takes from a fraction of
   a millisecond to tens of milliseconds, where a small write takes a few
   microseconds.

A storage engine decides, for every promise it makes, which of the three
levels the promise needs. SPEC 9 does that for kvstore.

The example shows the border between level 1 and level 2, which you can
observe from inside one program.

```rust
use std::fs::{self, File};
use std::io::{BufWriter, Write};

fn main() -> std::io::Result<()> {
    let dir = std::env::temp_dir().join(format!("concepts-written-{}", std::process::id()));
    fs::create_dir_all(&dir)?;
    let path = dir.join("journal.txt");

    let mut journal = BufWriter::new(File::create(&path)?);
    journal.write_all(b"entry one\n")?;
    // The bytes are in the BufWriter. The file is still empty.
    assert_eq!(fs::metadata(&path)?.len(), 0);

    journal.flush()?;
    // Now the operating system has them.
    assert_eq!(fs::read(&path)?, b"entry one\n");

    // And now the device has them too. `get_ref` reaches the File inside.
    journal.get_ref().sync_data()?;

    drop(journal);
    fs::remove_dir_all(&dir)?;
    Ok(())
}
```

`flush` and `sync_data` sound alike and are different things. `flush` on a
`BufWriter` moves bytes from level 1 to level 2. `flush` on a `File` does
nothing at all, because a `File` has no buffer of its own. Only the `sync_`
methods reach level 3.

`sync_all` also syncs the file's metadata, such as its size and
modification time. `sync_data` may skip metadata that is not needed to read
the data back. For a file that grows, the new size is needed, and
`sync_data` includes it; `sync_data` is the right call after an append.

**Mistakes people make.**

- Relying on `drop` to flush a `BufWriter`. It tries, but it cannot report
  an error, and it never runs when the process is killed. Flush explicitly
  where it matters.
- Answering "saved" after `write`, when the promise was about power
  failures. Or the opposite: syncing after every small write in a program
  whose users only care about the process crashing, and wondering why it
  manages 300 operations per second.
- Writing one logical record with several `write` calls. If the process
  dies between them, part of a record is in the file. Build the record in
  memory and write it once.

**When to use which.** Use a `BufWriter` when many small writes go to the
same place and nobody is waiting for any single one of them, for example
when you produce a large file from start to end. Do not put one between
you and a file when each write is a promise to someone.

**Read more.**
[`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html),
[`File::sync_data`](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_data),
[`File::sync_all`](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all),
[`Write::flush`](https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.flush).

---

## 5. Reading a file you do not trust

When a program starts after a crash, it does not know what is in its own
file. The last write may have been cut off at any byte. The file system may
have made the file longer and filled the new part with zeros. So recovery
reads the file like input from a stranger: every field is checked before it
is believed, and "the data ends here" is an ordinary result, not an error.

The pattern is a loop that reads one record at a time and stops at the
first position where no good record starts. Everything before that
position is kept. `Read::read_exact` is the tool: it fills a buffer
completely or returns an error of the kind `UnexpectedEof`, which here
means "the record is cut off".

The example is a file of length-prefixed text messages: one byte of length
(1 to 100), then that many bytes.

```rust
use std::io::{self, Cursor, ErrorKind, Read};

/// Fills `buf`, or reports `false` if the input ends first. Real I/O
/// errors are passed on.
fn read_full(input: &mut impl Read, buf: &mut [u8]) -> io::Result<bool> {
    match input.read_exact(buf) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(e),
    }
}

/// Returns the messages of the good beginning of the file, and its length.
fn recover(input: &mut impl Read) -> io::Result<(Vec<String>, u64)> {
    let mut messages = Vec::new();
    let mut good_len = 0u64;
    let mut body = [0u8; 100];
    loop {
        let mut len = [0u8; 1];
        if !read_full(input, &mut len)? {
            break; // a clean end, or a file cut exactly between messages
        }
        let len = usize::from(len[0]);
        if !(1..=100).contains(&len) {
            break; // not a length this format allows
        }
        if !read_full(input, &mut body[..len])? {
            break; // the message is cut off
        }
        let Ok(text) = std::str::from_utf8(&body[..len]) else {
            break; // the bytes are not what a writer would have written
        };
        messages.push(text.to_string());
        good_len += 1 + len as u64;
    }
    Ok((messages, good_len))
}

fn main() -> io::Result<()> {
    let intact = b"\x05hello\x05world\x03end";
    let (messages, good_len) = recover(&mut Cursor::new(intact))?;
    assert_eq!(messages, ["hello", "world", "end"]);
    assert_eq!(good_len, intact.len() as u64);

    // Cut the file at every possible length: the result is always a
    // beginning of the message list, and never a panic.
    for cut in 0..intact.len() {
        let (messages, good_len) = recover(&mut Cursor::new(&intact[..cut]))?;
        let expected = match cut {
            0..=5 => 0,
            6..=11 => 1,
            _ => 2,
        };
        assert_eq!(messages.len(), expected, "cut at {cut}");
        assert!(good_len <= cut as u64);
    }

    // Zeros after the good part, as some file systems leave them.
    let (messages, good_len) = recover(&mut Cursor::new(b"\x05hello\0\0\0\0"))?;
    assert_eq!((messages.len(), good_len), (1, 6));
    Ok(())
}
```

`recover` reads through `impl Read`, so the test can hand it a `Cursor`
over bytes in memory, and the real program hands it a `BufReader` around a
`File`. Testing recovery by cutting a known-good input at **every** length
is cheap and finds most mistakes.

The function returns `good_len` because recovery is not finished when the
reading is. The bad end has to be removed, with `File::set_len`, and the
writing position has to be at the new end before the first new record is
appended. SPEC 6 explains what goes wrong otherwise.

**Mistakes people make.**

- Treating a short read as a failure and refusing to start. A cut-off last
  record is the normal result of a crash. The file is fine up to there.
- Using `read` where `read_exact` is meant. A single `read` may return
  fewer bytes than asked for without being at the end. `read_exact` loops
  for you.
- Reading the whole file into a `Vec` first. It works for small files and
  makes start-up memory proportional to the log. A `BufReader` and a
  buffer of the maximum record size do the same job in constant memory.
- Leaving the bad bytes in the file and moving on.

**Read more.**
[`Read::read_exact`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact),
[`ErrorKind::UnexpectedEof`](https://doc.rust-lang.org/std/io/enum.ErrorKind.html#variant.UnexpectedEof),
[`File::set_len`](https://doc.rust-lang.org/std/fs/struct.File.html#method.set_len),
[`Cursor`](https://doc.rust-lang.org/std/io/struct.Cursor.html).

---

## 6. Replacing a file in one step

Sometimes a file has to be replaced by a new version as a whole: a
settings file that is saved, or a log that is compacted. Writing the new
content over the old file is wrong, because for a moment the file holds
the beginning of the new version and the end of the old one, or nothing at
all, and a crash at that moment leaves it so.

The safe procedure uses the fact that a file's *name* and its *content*
are separate things. The name is an entry in a directory that points to
the content. `std::fs::rename` changes where a name points, and when the
new name already exists, it replaces that entry. On Linux, macOS and other
Unix systems this is guaranteed to be a single step: no other program, and
no crash, can observe a state in between. Windows makes no such formal
promise, but the same call is the closest thing it has, and it is what
careful programs use there too.

```text
before:     settings.txt  ->  [old content]
            settings.tmp  ->  [new content, complete and synced]

rename("settings.tmp", "settings.txt")

after:      settings.txt  ->  [new content]
            (the old content has no name any more and is freed)
```

```rust
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Replaces `path` with `content` so that a crash leaves either the old
/// file or the new one.
fn save(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file = File::create(&tmp)?;
    file.write_all(content)?;
    // The content must be on the device before the name points to it.
    file.sync_all()?;
    drop(file); // close before renaming; Windows may insist
    fs::rename(&tmp, path)
}

fn main() -> std::io::Result<()> {
    let dir = std::env::temp_dir().join(format!("concepts-replace-{}", std::process::id()));
    fs::create_dir_all(&dir)?;
    let path = dir.join("settings.txt");

    save(&path, b"volume = 3\n")?;
    save(&path, b"volume = 11\n")?;
    assert_eq!(fs::read(&path)?, b"volume = 11\n");
    assert!(!path.with_extension("tmp").exists(), "the rename consumed the temporary file");

    // What a crash before the rename leaves behind: the old file, intact,
    // and a temporary file that can simply be deleted.
    fs::write(path.with_extension("tmp"), b"volu")?;
    assert_eq!(fs::read(&path)?, b"volume = 11\n");

    fs::remove_dir_all(&dir)?;
    Ok(())
}
```

The order of the three steps is the whole technique: write, sync, rename.
If the sync is left out, the rename can reach the disk before the content
does, and after a power failure the name points to an empty or partial
file. That is worse than the problem the procedure was meant to solve,
because now the *old* content is gone too.

Two details for a careful program:

- The temporary file must be in the same directory as the target, or at
  least on the same file system. `rename` cannot move a file to another
  file system; it fails. (Tools that seem to do it copy the file, and a
  copy is not one step.)
- On Unix systems the rename itself is recorded in the directory, and the
  directory is synced like a file: open it with `File::open` and call
  `sync_all`. Without that, a power failure shortly after the rename may
  bring the old name back, which is safe but loses the new version.
  Windows gives the standard library no way to do this.

**Mistakes people make.**

- Deleting the old file first and then renaming the new one. Between the
  two calls there is no file with that name.
- Keeping a handle on the old file and expecting it to show the new
  content. An open handle refers to the content, not the name. After the
  rename it still reads the old content. Reopen by name.
- Writing the temporary file to the system's temporary directory, which is
  often a different file system. The rename then fails.

**Read more.**
[`std::fs::rename`](https://doc.rust-lang.org/std/fs/fn.rename.html),
[`File::sync_all`](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all),
[`Path::with_extension`](https://doc.rust-lang.org/std/path/struct.Path.html#method.with_extension).

---

## 7. A connection is a stream of bytes

TCP gives two programs a pair of byte streams, one in each direction. It
keeps the bytes in order and delivers each exactly once. It does **not**
keep the boundaries of the `write` calls that produced them. If one side
writes `PING\n` and then `PONG\n`, the other side may read `PING\nPONG\n`
in one piece, or `PI` and then `NG\nPONG\n`, or one byte at a time. A
program that assumes "one read is one message" works on a quiet loopback
connection and fails under load.

So the receiver has to cut the stream into messages itself. For a protocol
of lines that means collecting bytes until a line feed arrives, handing
over the line, and keeping whatever came after it for the next round.
`BufRead` is the trait for this. Its two basic methods are `fill_buf`,
which returns the bytes that are available without copying them, and
`consume`, which says how many of them you have used. `read_until` is
built on those two.

The example cuts a stream into records that end with a semicolon, and
feeds it input that is split in an awkward place.

```rust
use std::io::{self, BufRead, BufReader, Read};

/// A reader that hands out its data in the given pieces, the way a socket
/// might.
struct Pieces(Vec<&'static [u8]>);

impl Read for Pieces {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.0.is_empty() {
            return Ok(0); // end of stream
        }
        let piece = self.0.remove(0);
        buf[..piece.len()].copy_from_slice(piece);
        Ok(piece.len())
    }
}

fn main() -> io::Result<()> {
    let stream = Pieces(vec![b"red;gr", b"een;", b"blue;unfinis", b"hed"]);
    let mut input = BufReader::new(stream);
    let mut records = Vec::new();
    loop {
        let mut record = Vec::new();
        let n = input.read_until(b';', &mut record)?;
        if n == 0 {
            break; // end of stream, nothing left
        }
        if record.last() != Some(&b';') {
            // The stream ended in the middle of a record.
            assert_eq!(record, b"unfinished");
            break;
        }
        record.pop();
        records.push(record);
    }
    assert_eq!(records, [&b"red"[..], b"green", b"blue"]);
    Ok(())
}
```

`read_until` works on bytes. Its relatives `read_line` and `lines` produce
`String`s and therefore insist on UTF-8. Given other bytes they fail at
run time:

```rust
use std::io::{BufRead, Cursor, ErrorKind};

let input = Cursor::new(b"caf\xe9\n".to_vec()); // "café" in Latin-1
let first = input.lines().next().unwrap();
let error = first.unwrap_err();
assert_eq!(error.kind(), ErrorKind::InvalidData);
assert_eq!(error.to_string(), "stream did not contain valid UTF-8");
```

Whether that is right depends on the protocol. If the protocol says
"lines of text", an error is correct. If it says "any bytes", `String` is
the wrong type from the first line of the program.

`read_until` has one property that matters for a server: it keeps
appending to the `Vec` until the delimiter arrives, however long that
takes. A client that never sends the delimiter makes the server collect
bytes without limit. When input comes from strangers, the length must be
bounded, and that means working with `fill_buf` and `consume` directly.
The borrow checker has an opinion about how:

```rust,compile_fail,E0499
use std::io::{BufRead, Cursor};

fn main() -> std::io::Result<()> {
    let mut input = Cursor::new(b"abc".to_vec());
    let available = input.fill_buf()?;
    input.consume(1);
    println!("{}", available.len());
    Ok(())
}
```

```text
error[E0499]: cannot borrow `input` as mutable more than once at a time
 --> src/main.rs:6:5
  |
5 |     let available = input.fill_buf()?;
  |                     ----- first mutable borrow occurs here
6 |     input.consume(1);
  |     ^^^^^ second mutable borrow occurs here
7 |     println!("{}", available.len());
  |                    --------- first borrow later used here
```

The slice from `fill_buf` points into the reader's buffer, and `consume`
changes that buffer. Finish looking at the slice (copy what you need,
remember how much you used as a plain number), and call `consume` after
the last use.

The same holds in the other direction. A response written with several
small `write` calls may travel as several packets; a `BufWriter` around
the socket turns them into one. But a buffered response that is never
flushed never arrives. The rule that works: flush when you are about to
wait for more input.

**Mistakes people make.**

- One `read` call, then parsing the result as one request.
- `lines()` for a protocol that carries arbitrary bytes.
- Unbounded `read_until` or `read_line` on input from the network.
- A `BufWriter` around the socket and no flush before the next blocking
  read. Client and server then wait for each other for ever.

**Read more.**
[`BufRead`](https://doc.rust-lang.org/std/io/trait.BufRead.html),
[`BufRead::fill_buf`](https://doc.rust-lang.org/std/io/trait.BufRead.html#tymethod.fill_buf),
[`BufRead::read_until`](https://doc.rust-lang.org/std/io/trait.BufRead.html#method.read_until),
[`TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html).

---

## 8. One store, many threads

A server with one thread per connection has many threads that all want
the same data. Chapter 19 gave you the tools: `Mutex` for one thread at a
time, `RwLock` for many readers or one writer. This section is about two
questions that the tools do not answer: how the threads get at the data,
and how much one lock must cover.

**How threads reach shared data.** `thread::spawn` demands that everything
the new thread uses lives for the rest of the program, because the thread
may still be running after the function that started it has returned:

```rust,compile_fail,E0373
use std::sync::Mutex;
use std::thread;

fn main() {
    let total = Mutex::new(0);
    let worker = thread::spawn(|| {
        *total.lock().unwrap() += 1;
    });
    worker.join().unwrap();
}
```

```text
error[E0373]: closure may outlive the current function, but it borrows `total`, which is owned by the current function
 --> src/main.rs:6:32
  |
6 |     let worker = thread::spawn(|| {
  |                                ^^ may outlive borrowed value `total`
7 |         *total.lock().unwrap() += 1;
  |          ----- `total` is borrowed here
```

The usual answer is `Arc`: shared ownership, counted at run time. There is
a second answer when the threads provably end before the data does:
`thread::scope`. Threads started inside a scope are joined before the
scope returns, so they may borrow anything that outlives the scope. No
`Arc`, no `'static`, and no forgotten `join`.

**How much one lock must cover.** A lock protects an *invariant*: a
statement about the data that is true whenever no thread is in the middle
of a change. If the invariant connects two things, both must be changed
under the same lock. Two locks, one for each, protect each thing and not
the connection between them.

The example is a bank. It has balances and a journal of transfers, and the
invariant is: *the journal, replayed, gives the balances.* One lock covers
both.

```rust
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::thread;

#[derive(Default)]
struct Bank {
    balances: BTreeMap<&'static str, i64>,
    journal: Vec<(&'static str, &'static str, i64)>,
}

impl Bank {
    /// Journal entry and balances change together or not at all.
    fn transfer(&mut self, from: &'static str, to: &'static str, amount: i64) {
        self.journal.push((from, to, amount));
        *self.balances.entry(from).or_insert(0) -= amount;
        *self.balances.entry(to).or_insert(0) += amount;
    }

    fn replayed(&self) -> BTreeMap<&'static str, i64> {
        let mut balances = BTreeMap::new();
        for &(from, to, amount) in &self.journal {
            *balances.entry(from).or_insert(0) -= amount;
            *balances.entry(to).or_insert(0) += amount;
        }
        balances
    }
}

fn main() {
    let bank = Mutex::new(Bank::default());
    // Scoped threads borrow `bank`; no Arc is needed.
    thread::scope(|scope| {
        for teller in 0..8 {
            let bank = &bank;
            scope.spawn(move || {
                for i in 0..500 {
                    let (from, to) = if (teller + i) % 2 == 0 { ("ann", "bo") } else { ("bo", "ann") };
                    bank.lock().unwrap().transfer(from, to, 1 + i % 3);
                }
            });
        }
    }); // all eight threads have ended here

    let bank = bank.into_inner().unwrap();
    assert_eq!(bank.journal.len(), 4000);
    assert_eq!(bank.replayed(), bank.balances, "the journal explains the balances");
    assert_eq!(bank.balances["ann"] + bank.balances["bo"], 0, "no money was created");
}
```

Now imagine `journal` and `balances` behind two separate mutexes, and
`transfer` locking one after the other. Every access is still protected
and the program has no data race. But two tellers can interleave: A
appends to the journal, B appends, B updates the balances, A updates the
balances. For amounts that are simply added, the end result is the same.
For operations where order matters ("set the balance to", "the last writer
wins") the journal now tells a different story from the balances, and a
restart that replays the journal produces a state the bank was never in.
That is exactly the bug the test `m5_the_log_has_the_order_the_clients_saw`
looks for.

**Mistakes people make.**

- A lock per field, because "every field is protected".
- Reading under a lock, releasing it, computing, and locking again to
  write. Between the two lock calls, the value you read may have changed.
  A read-modify-write is one critical section.
- Holding a lock while doing something slow that the lock does not need
  to cover, above all writing to a network connection. One slow client
  then stops everyone.

**When to use which.** Start with one `Mutex` around everything that
belongs to one invariant. Change to `RwLock` when you have measured that
readers wait for each other. Split the lock only when you can state the
invariants on both sides of the split, and they are independent.

**Read more.**
[`thread::scope`](https://doc.rust-lang.org/std/thread/fn.scope.html),
[`Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html),
[`RwLock`](https://doc.rust-lang.org/std/sync/struct.RwLock.html).

---

## 9. Stopping threads that are blocked

A thread that waits for the network is not running your code. It is
inside the operating system, in a call such as `accept` or `read`, and it
stays there until the thing it waits for happens. You cannot stop it from
outside, and setting a flag does nothing by itself: the thread will look at
the flag only after the call returns.

So a clean shutdown has two parts. A **flag** says that it is time to
stop. And for every thread that may be blocked, something makes its
blocking call **return**, so that the thread gets to look at the flag.

| the thread is blocked in | what makes the call return |
|---|---|
| `TcpListener::accept` | a connection arrives; any connection will do, also one from the same process |
| `TcpStream::read` | `shutdown` is called on the same socket from another thread, through a second handle made with `try_clone`; the read then returns `Ok(0)` (on some systems an error; treat both as "this connection is over") |
| `TcpStream::write` to a peer that does not read | the same |
| `Receiver::recv` on a channel | every `Sender` is dropped |
| `Condvar::wait` | `notify_one` or `notify_all` |

The example is a small server that counts the bytes its clients send, and
stops when a client sends an exclamation mark. It has a thread blocked in
`accept` and threads blocked in `read`, and it ends without killing
anything.

```rust
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let stopping = AtomicBool::new(false);
    let counted = AtomicUsize::new(0);
    // A second handle on every accepted socket, for the shutdown.
    let handles: Mutex<Vec<TcpStream>> = Mutex::new(Vec::new());

    thread::scope(|scope| -> std::io::Result<()> {
        // The server side: an accepting thread and one thread per client.
        scope.spawn(|| {
            thread::scope(|clients| {
                for stream in listener.incoming() {
                    if stopping.load(Ordering::SeqCst) {
                        break; // woken by the connection made below
                    }
                    let Ok(mut stream) = stream else { continue };
                    let Ok(handle) = stream.try_clone() else { continue };
                    handles.lock().unwrap().push(handle);
                    let (stopping, counted) = (&stopping, &counted);
                    clients.spawn(move || {
                        let mut byte = [0u8; 1];
                        // Ok(0) is the end of the stream: the peer closed,
                        // or someone called shutdown on this socket.
                        while let Ok(1) = stream.read(&mut byte) {
                            if byte[0] == b'!' {
                                stopping.store(true, Ordering::SeqCst);
                                // Wake the accepting thread.
                                let _ = TcpStream::connect(addr);
                                break;
                            }
                            counted.fetch_add(1, Ordering::SeqCst);
                        }
                    });
                }
                // Wake every thread that is blocked in a read.
                for handle in handles.lock().unwrap().iter() {
                    let _ = handle.shutdown(Shutdown::Both);
                }
            }); // waits for the client threads
        });

        // The client side.
        let mut idle = TcpStream::connect(addr)?; // connects and says nothing
        let mut busy = TcpStream::connect(addr)?;
        busy.write_all(b"abc!")?;

        // The idle client was never answered; its connection simply ends.
        let mut rest = Vec::new();
        idle.read_to_end(&mut rest)?;
        assert!(rest.is_empty());
        Ok(())
    })?; // waits for the server threads: the program does not hang

    assert_eq!(counted.load(Ordering::SeqCst), 3);
    Ok(())
}
```

Follow the shutdown through. The client thread that reads `!` sets the
flag and makes a connection to its own server. `accept` returns that
connection, the accepting thread sees the flag and leaves its loop. It
then calls `shutdown` on its second handle of every client socket. The
thread serving the idle client was blocked in `read`; the call now returns
`Ok(0)`, the `while let` ends, the thread ends. The inner scope has
nothing left to wait for.

Two details carry the design. The list of handles is changed only by the
accepting thread, so when that thread has left its loop, no connection can
be added behind its back. And the scopes do the waiting: there is no list
of `JoinHandle`s to keep and no thread that can be forgotten.

**Mistakes people make.**

- Setting a flag and expecting blocked threads to notice.
- Giving every socket a short read timeout and checking the flag in a
  loop. It works, and it makes every idle connection wake up many times
  per second for nothing, and shutdown as slow as the timeout.
- Calling `std::process::exit` instead. Threads are cut off wherever they
  are, destructors do not run, and buffered output is lost. It is a crash
  that returns 0.
- Dropping the second handle and expecting the first to close. `try_clone`
  makes another handle on the same socket; the socket closes when all
  handles are gone. `shutdown` acts on the socket itself, whichever handle
  you call it on.

**Read more.**
[`TcpStream::shutdown`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.shutdown),
[`TcpStream::try_clone`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.try_clone),
[`TcpListener::incoming`](https://doc.rust-lang.org/std/net/struct.TcpListener.html#method.incoming),
[`AtomicBool`](https://doc.rust-lang.org/std/sync/atomic/type.AtomicBool.html).

---

## 10. One process per directory

Two copies of a program that both append to the same file will each track
their own idea of where the file ends, and write over each other. A program
that owns files should make sure it is the only one.

The traditional way is a **lock file**: a file whose existence means
"taken". The program creates it at start and deletes it at exit. This
fails in the case that matters most. After a crash the file is still
there, and the program refuses to start until a person deletes it.

Operating systems offer a better tool: a **lock on an open file**. The
lock belongs to the open handle. It is released when the handle is closed,
and the system closes all handles of a process that ends, however it ends.
`File::try_lock` asks for such a lock and returns at once.

```rust
use std::fs::{self, File, TryLockError};

fn main() -> std::io::Result<()> {
    let dir = std::env::temp_dir().join(format!("concepts-lock-{}", std::process::id()));
    fs::create_dir_all(&dir)?;
    let path = dir.join("workspace.lock");

    let first = File::create(&path)?;
    first.try_lock().expect("nobody else holds the lock");

    // A second handle, as another process would have. The file exists and
    // can be opened; the lock is a separate matter.
    let second = File::open(&path)?;
    assert!(matches!(second.try_lock(), Err(TryLockError::WouldBlock)));

    // Closing the first handle releases its lock. No clean-up code ran.
    drop(first);
    second.try_lock().expect("the lock is free again");

    drop(second);
    fs::remove_dir_all(&dir)?;
    Ok(())
}
```

Keep the locked `File` alive for as long as the program needs the lock.
Stored in a struct field that nothing else uses, it does its work by
existing; a leading underscore in the field's name tells the compiler and
the reader that this is intended.

These locks are **advisory** on Unix systems: they stop other programs
that also ask for the lock, and nobody else. A program that opens the file
without asking can still write to it. On Windows a locked file cannot be
read or written through other handles at all. For both reasons, lock a
small separate file and not the data file itself.

**Mistakes people make.**

- "The lock file exists" as the lock.
- Letting the locked `File` go out of scope at the end of the function
  that took the lock. The lock is gone before the program has started its
  real work.
- `lock` where `try_lock` is meant. `lock` waits until the lock is free.
  A second server that silently waits for the first one to end is rarely
  what anyone wants.

**Read more.**
[`File::try_lock`](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock),
[`File::lock`](https://doc.rust-lang.org/std/fs/struct.File.html#method.lock),
[`TryLockError`](https://doc.rust-lang.org/std/fs/enum.TryLockError.html).
