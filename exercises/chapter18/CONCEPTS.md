# Chapter 18 concepts — Files, streams and processes

[Chapter guide](README.md) · [Hints](HINTS.md)

Read one section, then do its lesson. Every code block on this page is compiled by the course checks, and the complete programs are also run, so you can paste any of them into [the scratch example](../../examples/ch18_scratch.rs) and change it.

## What you will be able to do

Input and output (I/O) is how a program exchanges bytes with the world outside its own memory: files, the terminal, the network, other programs. Rust's standard library describes all of these with two small traits, `Read` and `Write`. Code written against the traits works with every source and destination, also with the in-memory ones that make I/O code easy to test.

After this chapter you will be able to:

- write read loops and write loops that stay correct when one call moves only part of the data;
- read text by lines and binary data by delimiters, and say who owns each buffer;
- decide where a buffer must be flushed and report the error when flushing fails;
- move around in a seekable stream and read and write fixed-size binary fields in a stated byte order;
- change paths and file names without losing bytes that are not Unicode;
- list directories and inspect metadata with a stated policy for order, symbolic links and errors;
- exchange length-prefixed messages over a real TCP connection on the loopback interface;
- start other programs, connect pipes to them, and read their exit status without deadlocks.

## Before you start

Ownership and borrowing (Chapters 4 and 5), `Result` and the `?` operator (Chapter 7), traits and generic functions (Chapter 11), closures (Chapter 14), iterators and `collect` (Chapter 15), and strings as UTF-8 bytes (Chapter 17). Lessons 08 and 09 start one extra thread. Chapter 19 teaches threads in full; this page explains the two calls that you need here.

## Time

These are estimates for reading the section, doing the lesson, and writing your notes.

| Lesson | Estimate |
| --- | --- |
| 01 Streaming bytes | 45 min |
| 02 Buffered text | 40 min |
| 03 Writers and flushing | 40 min |
| 04 Seeking and binary formats | 45 min |
| 05 Paths | 40 min |
| 06 Directories and metadata | 45 min |
| 08 Framed messages over TCP | 90 min |
| 09 Child processes and pipes | 90 min |
| 07 Capstone | 60 min |
| R01, C01, and the drills | 2 to 3 h |

## Lesson 01 · Streaming bytes

**The model.** A *reader* is any value whose type implements the trait `std::io::Read`. On stable Rust, the one method you must write is `read`:

```text
fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
```

You lend the reader a buffer, and the reader copies bytes into the front of it. The result has three cases, and every read loop must handle all three:

- `Ok(n)` with `n > 0`: the first `n` bytes of the buffer are new data. `n` can be smaller than the buffer, even when more data will come later. A network connection gives you what has arrived so far. A test reader may give one byte at a time.
- `Ok(0)`: the stream has ended (when your buffer was not empty). This is called *end of file*, or EOF, also for streams that are not files.
- `Err(e)`: the read failed. One kind is special. `ErrorKind::Interrupted` means that a signal arrived before any byte moved. Nothing was lost, so the correct reaction is to call `read` again. Every other error is real and goes to the caller.

```text
buf after read returned Ok(3):

  [ 68 | 65 | 6C | ?? | ?? | ?? | ?? | ?? ]
    '--- new ---'  '---- old or zero ----'
    use &buf[..3]      never look here
```

`read` takes `&mut self` because reading changes the reader: it moves forward. Files, sockets, standard input, `&[u8]` and `Cursor<Vec<u8>>` all implement `Read`. A function that asks for `R: Read` instead of `File` therefore works with all of them, and you can test it without touching the disk.

**Worked example.** A checksum adds up all bytes of a stream. The function never knows how long the stream is and never holds more than 16 bytes of it.

```rust
use std::io::{self, Read};

/// Adds up all bytes of a stream, wrapping around at 256.
fn checksum<R: Read>(mut input: R) -> io::Result<u8> {
    let mut buf = [0u8; 16];
    let mut sum = 0u8;
    loop {
        match input.read(&mut buf) {
            Ok(0) => return Ok(sum), // end of stream
            Ok(n) => {
                // Only the first n bytes are new.
                for byte in &buf[..n] {
                    sum = sum.wrapping_add(*byte);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {} // try again
            Err(e) => return Err(e),
        }
    }
}

fn main() -> io::Result<()> {
    let data = [200u8, 100, 7];
    assert_eq!(checksum(&data[..])?, 51); // 307 - 256
    assert_eq!(checksum(io::empty())?, 0);
    // Two readers, one after the other, limited to 5 bytes in total.
    let joined = (&b"\x01\x02\x03"[..]).chain(&b"\x04\x05\x06"[..]).take(5);
    assert_eq!(checksum(joined)?, 15);
    Ok(())
}
```

The last lines use two *adapters*. `a.chain(b)` is a reader that gives all of `a` and then all of `b`. `r.take(n)` is a reader that ends after at most `n` bytes. Adapters take the reader by value. A `&mut R` is itself a reader, so you can lend a reader to an adapter or to a function and keep using it afterwards; `reader.by_ref()` is a readable way to write `&mut reader`.

**You rarely write the loop yourself.** The standard library has it in several forms:

| You want | Use |
| --- | --- |
| exactly `buf.len()` bytes, or an error | `read_exact(&mut buf)` |
| everything until the end, as bytes | `read_to_end(&mut vec)` |
| everything until the end, as checked UTF-8 text | `read_to_string(&mut string)` |
| to copy a whole reader into a writer | `io::copy(&mut reader, &mut writer)` |

Write the loop by hand when you process the data piece by piece, as `checksum` does, and when you practise. Lesson 01 asks for it so that you can do it once without help.

**Mistakes people make.**

Using the whole buffer instead of the part that `read` filled. The stale bytes of the previous round then appear in the output:

```rust
use std::io::Read;

let mut input: &[u8] = b"hello!!";
let mut buf = [0u8; 4];
let (mut wrong, mut right) = (Vec::new(), Vec::new());
loop {
    let n = input.read(&mut buf).unwrap();
    if n == 0 {
        break;
    }
    wrong.extend_from_slice(&buf); // all 4 bytes, every time
    right.extend_from_slice(&buf[..n]); // only the new bytes
}
assert_eq!(right, b"hello!!");
assert_eq!(wrong, b"hello!!l"); // the second read gave 3 bytes; the 4th is old
```

Assuming that one `read` fills the buffer. The code works with a file on your machine and fails with a socket, a pipe, or a slow disk. Loop, or call `read_exact`.

Giving a reader away and then using it again:

```rust,compile_fail,E0382
use std::io::{self, Read};

fn count_bytes<R: Read>(mut input: R) -> io::Result<usize> {
    let mut all = Vec::new();
    input.read_to_end(&mut all)
}

fn main() -> io::Result<()> {
    let source = io::Cursor::new(vec![1u8, 2, 3]);
    let first = count_bytes(source)?;
    let second = count_bytes(source)?;
    println!("{first} {second}");
    Ok(())
}
```

```text
error[E0382]: use of moved value: `source`
   |
 9 |     let source = io::Cursor::new(vec![1u8, 2, 3]);
   |         ------ move occurs because `source` has type `std::io::Cursor<Vec<u8>>`, which does not implement the `Copy` trait
10 |     let first = count_bytes(source)?;
   |                             ------ value moved here
11 |     let second = count_bytes(source)?;
   |                              ^^^^^^ value used here after move
   |
help: consider mutably borrowing `source`
   |
10 |     let first = count_bytes(&mut source)?;
   |                             ++++
```

The help is right: pass `&mut source` (and declare `let mut source`). The second call then continues where the first one stopped, which here is at the end.

**Read more.** [`std::io::Read`](https://doc.rust-lang.org/std/io/trait.Read.html) · [`ErrorKind`](https://doc.rust-lang.org/std/io/enum.ErrorKind.html) · [`io::copy`](https://doc.rust-lang.org/std/io/fn.copy.html) · [the `std::io` module overview](https://doc.rust-lang.org/std/io/index.html)

## Lesson 02 · Buffered text

**The model.** Lesson 01 read bytes in pieces of unknown size. Text needs something more: "give me everything up to the next newline". A plain reader cannot do that well. It would have to ask for one byte at a time, and for a file or a socket each request is a call into the operating system.

A *buffered reader* solves this. `BufReader::new(inner)` owns a block of memory, 8 KiB by default. It fills the block with one large `read` from `inner` and then answers many small requests from memory. The trait for readers that own such a buffer is `BufRead`. It adds the methods that look for a delimiter:

| Method | Gives you | Line ending | End of input |
| --- | --- | --- | --- |
| `read_line(&mut string)` | appends one line to your `String` | kept | returns `Ok(0)` |
| `lines()` | iterator of `io::Result<String>` | removed (`\n` or `\r\n`) | iterator ends |
| `read_until(byte, &mut vec)` | appends bytes up to and including `byte` | kept | returns `Ok(0)` |
| `split(byte)` | iterator of `io::Result<Vec<u8>>` | removed | iterator ends |

The two text methods check that the line is valid UTF-8, because they produce a `String`. The two byte methods never look at the encoding. `&[u8]` and `Cursor` already are `BufRead` (their data is in memory), so tests need no `BufReader`. `File` and `TcpStream` are not; you wrap them.

**Worked example.** Each line holds one number. The first function reuses one `String` for every line, so it allocates once however long the input is. The second is shorter and allocates one `String` per line.

```rust
use std::io::{self, BufRead};

fn bad_number(line: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("not a number: {line:?}"))
}

/// One buffer for all lines.
fn sum_reusing<R: BufRead>(mut input: R) -> io::Result<i64> {
    let mut line = String::new();
    let mut sum = 0;
    loop {
        line.clear(); // read_line appends, so empty the buffer first
        if input.read_line(&mut line)? == 0 {
            return Ok(sum); // 0 bytes: end of input
        }
        let text = line.trim(); // the line ending is still there
        if !text.is_empty() {
            sum += text.parse::<i64>().map_err(|_| bad_number(text))?;
        }
    }
}

/// One new String per line.
fn sum_by_lines<R: BufRead>(input: R) -> io::Result<i64> {
    let mut sum = 0;
    for line in input.lines() {
        let line = line?; // each item is an io::Result<String>
        let text = line.trim();
        if !text.is_empty() {
            sum += text.parse::<i64>().map_err(|_| bad_number(text))?;
        }
    }
    Ok(sum)
}

fn main() -> io::Result<()> {
    let text = "10\r\n-3\n\n35"; // CRLF, LF, an empty line, no final newline
    assert_eq!(sum_reusing(text.as_bytes())?, 42);
    assert_eq!(sum_by_lines(text.as_bytes())?, 42);
    assert_eq!(
        sum_by_lines(&b"7\nseven\n"[..]).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    Ok(())
}
```

Both functions report a last line that has no newline. Both stop at the first error and return it.

**Mistakes people make.**

Forgetting that `read_line` appends. Without `clear`, the second call adds the second line to the first:

```rust
use std::io::BufRead;

let mut input: &[u8] = b"red\ngreen\n";
let mut line = String::new();
input.read_line(&mut line).unwrap();
input.read_line(&mut line).unwrap(); // no clear() in between
assert_eq!(line, "red\ngreen\n");
```

Asking a plain reader for lines:

```rust,compile_fail,E0599
use std::fs::File;
use std::io::BufRead;

fn main() -> std::io::Result<()> {
    let file = File::open("notes.txt")?;
    for line in file.lines() {
        println!("{}", line?);
    }
    Ok(())
}
```

```text
error[E0599]: the method `lines` exists for struct `File`, but its trait bounds were not satisfied
  |
6 |     for line in file.lines() {
  |                      ^^^^^ method cannot be called on `File` due to unsatisfied trait bounds
  |
  = note: the following trait bounds were not satisfied:
          `File: BufRead`
          which is required by `&mut File: BufRead`
```

The fix is `BufReader::new(file).lines()`.

Throwing errors away with `lines().flatten()` or `lines().map_while(Result::ok)`. A line that is not UTF-8, or a failing disk, then looks like a missing line or like the end of the input. Keep the `Result`: use `?` inside a loop, or collect into `io::Result<Vec<_>>` as Chapter 15 showed.

Making a new `BufReader` for each line of the same source. The first one reads far ahead and takes those bytes with it when it is dropped. Create one buffered reader and keep it for as long as you read from the source.

**When to use which.** Use `lines()` when you want each line as its own `String`. Use `read_line` with one reused `String` in a loop that handles very many lines. Use `read_until` and `split` when the data is not text, or when the delimiter is not a newline. If you need the exact bytes of a text file, line endings included, `lines()` is the wrong tool, because you cannot tell afterwards whether a line ended with `\n` or `\r\n`.

**Read more.** [`std::io::BufRead`](https://doc.rust-lang.org/std/io/trait.BufRead.html) · [`BufReader`](https://doc.rust-lang.org/std/io/struct.BufReader.html) · [`Cursor`](https://doc.rust-lang.org/std/io/struct.Cursor.html)

## Lesson 03 · Writers and flushing

**The model.** A *writer* implements `std::io::Write`. The trait mirrors `Read`:

```text
fn write(&mut self, buf: &[u8]) -> io::Result<usize>
fn flush(&mut self) -> io::Result<()>
```

`write` may accept only the first part of your bytes and still return `Ok`. The number tells you how many it took. This is a *short write*. You almost never call `write` directly. `write_all(buf)` contains the loop: it calls `write` again with the rest until everything is accepted or an error happens. The macros `write!` and `writeln!` format text straight into a writer and use `write_all` inside.

Many writers keep bytes in memory before they pass them on. `BufWriter::new(inner)` collects small writes in an 8 KiB block and hands them to `inner` in large pieces.

```text
your code --write_all--> [ BufWriter's block ] --when full, on flush, on drop--> file / socket / Vec
```

Two consequences follow. First, a successful `write_all` on a buffered writer says only "the bytes are in the block". An error from the disk or the network can appear much later, at the `flush`. Second, when a `BufWriter` is dropped it tries to flush, but a destructor cannot return a `Result`, so an error at that moment is lost without a trace. A function that promises complete output therefore calls `flush()` itself and returns the result.

**Worked example.** A hex dump writes two characters per byte, eight bytes per line. The function adds its own buffer, so it stays fast when the caller passes a file.

```rust
use std::io::{self, BufWriter, Write};

/// Writes `data` as hex, eight bytes per line.
fn hex_dump<W: Write>(out: W, data: &[u8]) -> io::Result<()> {
    let mut out = BufWriter::new(out);
    for (index, byte) in data.iter().enumerate() {
        let end = if index % 8 == 7 { '\n' } else { ' ' };
        write!(out, "{byte:02x}{end}")?; // formats into the writer
    }
    if data.len() % 8 != 0 {
        writeln!(out)?;
    }
    out.flush() // the last chance to see an error from the destination
}

fn main() -> io::Result<()> {
    let mut page = Vec::new();
    hex_dump(&mut page, b"Own It: Hands-On Mastery of Rust")?;
    assert_eq!(
        String::from_utf8(page).unwrap(),
        "4f 77 6e 20 49 74 3a 20\n48 61 6e 64 73 2d 4f 6e\n\
         20 4d 61 73 74 65 72 79\n20 6f 66 20 52 75 73 74\n"
    );
    Ok(())
}
```

`hex_dump(&mut page, ...)` passes `&mut Vec<u8>`. As with readers, a `&mut W` is itself a writer, so the caller keeps the vector.

The next program shows why the flush result matters. The destination fails, and the only call that reports it is `flush`:

```rust
use std::io::{self, BufWriter, Write};

/// A destination that is full.
struct Full;
impl Write for Full {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::StorageFull.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

let mut out = BufWriter::new(Full);
assert!(out.write_all(b"important").is_ok()); // only reached the block
assert_eq!(out.flush().unwrap_err().kind(), io::ErrorKind::StorageFull);
```

**Mistakes people make.**

Calling `write` and ignoring the count. The compiler accepts it. Clippy rejects it with an error, not a warning:

```text
error: written amount is not handled
  |
4 |     out.write(data)?;
  |     ^^^^^^^^^^^^^^^^
  |
  = help: use `Write::write_all` instead, or handle partial writes
  = note: `#[deny(clippy::unused_io_amount)]` on by default
```

Using `write!` without the trait in scope:

```rust,compile_fail,E0599
fn main() -> std::io::Result<()> {
    let mut out = Vec::new();
    write!(out, "{} items", 3)?;
    Ok(())
}
```

```text
error[E0599]: cannot write into `Vec<_>`
  |
3 |     write!(out, "{} items", 3)?;
  |            ^^^
  |
note: must implement `io::Write`, `fmt::Write`, or have a `write_fmt` method
  = help: items from traits can only be used if the trait is in scope
help: trait `Write` which provides `write_fmt` is implemented but not in scope; perhaps you want to import it
  |
1 + use std::io::Write;
  |
```

Flushing after every small write. That is correct, but it removes the benefit of the buffer: each flush is a call into the operating system. Flush where a reader on the other side waits for the data, and at the end.

**When to use which.** Use `write_all`, `write!` and `writeln!` for all normal output. Wrap a file or socket in `BufWriter` when you make many small writes. Wrapping a `Vec<u8>` gains nothing: it is already memory. `LineWriter` flushes at each newline, which suits output that a person reads while it appears; standard output uses that strategy. Who flushes? The code that knows the output is complete. A helper that writes one record should not flush, because its caller may write a thousand records and flush once.

**Read more.** [`std::io::Write`](https://doc.rust-lang.org/std/io/trait.Write.html) · [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html) · [`LineWriter`](https://doc.rust-lang.org/std/io/struct.LineWriter.html) · [`write!`](https://doc.rust-lang.org/std/macro.write.html)

## Lesson 04 · Seeking and binary formats

**The model.** A file is a numbered row of bytes. A reader or writer on it has a *position*: the number of the byte that the next `read` or `write` uses. Reading and writing move the position forward. A stream that implements `std::io::Seek` also lets you set it:

| Call | New position |
| --- | --- |
| `seek(SeekFrom::Start(n))` | `n` bytes after the beginning |
| `seek(SeekFrom::Current(d))` | the old position plus `d` (may be negative) |
| `seek(SeekFrom::End(d))` | the length plus `d` (use a negative `d` to reach data) |
| `rewind()` | 0 |
| `stream_position()` | unchanged; tells you where you are |

`seek` returns the new position, counted from the start. Files can seek. Sockets and pipes cannot. `Cursor<Vec<u8>>` behaves like a small file in memory, so you can test seeking code without a disk.

A *binary format* stores numbers as raw bytes, not as digits. The format must then fix two things that text never needed. The first is the **width** of each field: a `u32` is always 4 bytes. The second is the **byte order**: which end of the number comes first.

```text
the u32 value 0x0A0B0C0D

little endian   to_le_bytes()   0D 0C 0B 0A    lowest byte first
big endian      to_be_bytes()   0A 0B 0C 0D    highest byte first
```

Rust never guesses. `to_le_bytes` and `to_be_bytes` turn an integer into an array, and `from_le_bytes` and `from_be_bytes` turn an array back. A file written with one order and read with the other gives wrong numbers and no error, so the order belongs to the description of the format, next to the field widths.

**Worked example.** A small log format: a 4-byte counter in big-endian order at the start, then one `u16` in little-endian order per event. The program updates the counter in place and reads the last event from the end.

```rust
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};

/// Adds one to the big-endian u32 at the start of the stream.
fn bump_counter<F: Read + Write + Seek>(file: &mut F) -> io::Result<u32> {
    let mut field = [0u8; 4];
    file.rewind()?;
    file.read_exact(&mut field)?; // the position is now 4
    let count = u32::from_be_bytes(field)
        .checked_add(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "counter is full"))?;
    file.seek(SeekFrom::Current(-4))?; // back to the start of the field
    file.write_all(&count.to_be_bytes())?;
    Ok(count)
}

fn main() -> io::Result<()> {
    let mut log = Cursor::new(Vec::new());
    log.write_all(&0u32.to_be_bytes())?; // the counter
    for event in [300u16, 7, 513] {
        log.write_all(&event.to_le_bytes())?;
    }
    assert_eq!(log.get_ref().len(), 4 + 3 * 2);

    assert_eq!(bump_counter(&mut log)?, 1);
    assert_eq!(bump_counter(&mut log)?, 2);
    assert_eq!(&log.get_ref()[..4], [0, 0, 0, 2]);

    // The last event occupies the last two bytes.
    let position = log.seek(SeekFrom::End(-2))?;
    assert_eq!(position, 8);
    let mut field = [0u8; 2];
    log.read_exact(&mut field)?;
    assert_eq!(field, [0x01, 0x02]); // 513 = 0x0201, lowest byte first
    assert_eq!(u16::from_le_bytes(field), 513);
    Ok(())
}
```

`read_exact` is the right call for a fixed field: it loops until the array is full and reports an early end as an error. `checked_add` does the same job for arithmetic: a counter at its maximum becomes an error and not a silent wrap to zero.

**Mistakes people make.**

Passing a slice where the conversion wants an array of the exact size:

```rust,compile_fail,E0308
fn main() {
    let bytes = [0x2a, 0, 0, 0, 0xff];
    let value = u32::from_le_bytes(&bytes[..4]);
    println!("{value}");
}
```

```text
error[E0308]: mismatched types
  |
3 |     let value = u32::from_le_bytes(&bytes[..4]);
  |                 ------------------ ^^^^^^^^^^^ expected `[u8; 4]`, found `&[{integer}]`
  |                 |
  |                 arguments to this function are incorrect
```

Read into a `[u8; 4]` with `read_exact`, as above. If the bytes are already in a slice, convert with `try_into`, which fails when the length is wrong:

```rust
let bytes = [0x2a, 0, 0, 0, 0xff];
let field: [u8; 4] = bytes[..4].try_into().expect("exactly four bytes");
assert_eq!(u32::from_le_bytes(field), 42);
```

Computing an offset with plain arithmetic. `index * record_size` can overflow when `index` comes from a file or from a user. A debug build then panics; a release build wraps around and seeks to a wrong place. Use `checked_mul` and turn `None` into an error.

Writing a length with `as`. `payload.len() as u16` cuts off the high bits of a large length without any error, and the reader then finds a frame that lies about its size. Use `u16::try_from(len)` and handle the failure.

Copying the memory of a struct into a file. The layout of a struct (field order, padding, byte order) belongs to one compiler and one machine. Write each field with `to_le_bytes` or `to_be_bytes` in a stated order.

**When to use which.** Choose one byte order for the whole format and write it down. Little endian matches most current processors. Big endian is the tradition of network protocols, where it is called *network byte order*. For reading a file from start to end, do not seek at all. Seek when you need one record of many, or when you must return to a header after the body has been written.

**Read more.** [`std::io::Seek`](https://doc.rust-lang.org/std/io/trait.Seek.html) · [`SeekFrom`](https://doc.rust-lang.org/std/io/enum.SeekFrom.html) · [`u32::from_le_bytes`](https://doc.rust-lang.org/std/primitive.u32.html#method.from_le_bytes) · [`Read::read_exact`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact)

## Lesson 05 · Paths preserve OS data

**The model.** A path is not a string. On Unix a file name is a row of bytes that need not be UTF-8. On Windows it is a row of 16-bit units that need not be valid UTF-16. Rust's `String` and `str` are always valid UTF-8 (Chapter 17), so they cannot hold every file name.

The standard library has a second pair of types for text that comes from the operating system: `OsString` (owned) and `OsStr` (borrowed). `PathBuf` and `Path` are thin wrappers around them that add knowledge about separators and components.

```text
owned      borrowed    content
String     &str        always valid UTF-8
OsString   &OsStr      whatever the operating system uses
PathBuf    &Path       an OsString / OsStr, seen as path components
```

A `&str` can always become a `&Path`, for example with `Path::new("a/b")`. The other direction can fail. `path.to_str()` returns `Option<&str>`. `path.display()` and `to_string_lossy()` always work, but they replace what is not Unicode, so use them for messages and never for a name that you will open later.

The methods of `Path` are *lexical*: they look at the text of the path and never at the disk.

| Method | For `/data/site/index.html` |
| --- | --- |
| `parent()` | `Some("/data/site")` |
| `file_name()` | `Some("index.html")` |
| `file_stem()` | `Some("index")` |
| `extension()` | `Some("html")` |
| `with_extension("md")` | `/data/site/index.md` |
| `with_file_name("a.txt")` | `/data/site/a.txt` |
| `join("css")` | `/data/site/index.html/css` |
| `starts_with("/data")` | `true` |

`file_name`, `file_stem` and `extension` return `Option<&OsStr>`, not strings.

**Worked example.** A thumbnail gets the name of its picture with `-small` added before the extension, in another directory. The file name is never converted to `str`, so a name that is not Unicode survives.

```rust
use std::path::{Component, Path, PathBuf};

/// "shots/cat.png" with out_dir "thumbs" becomes "thumbs/cat-small.png".
/// Returns None when `picture` has no file name.
fn thumbnail_path(picture: &Path, out_dir: &Path) -> Option<PathBuf> {
    // to_os_string gives an owned OsString that can grow.
    let mut name = picture.file_stem()?.to_os_string();
    name.push("-small");
    if let Some(extension) = picture.extension() {
        name.push(".");
        name.push(extension);
    }
    Some(out_dir.join(name))
}

fn main() {
    let thumbs = Path::new("thumbs");
    assert_eq!(
        thumbnail_path(Path::new("shots/cat.png"), thumbs),
        Some(PathBuf::from("thumbs/cat-small.png"))
    );
    assert_eq!(
        thumbnail_path(Path::new("README"), thumbs),
        Some(PathBuf::from("thumbs/README-small"))
    );
    assert_eq!(thumbnail_path(Path::new("shots/.."), thumbs), None);

    // components() shows how Path reads a path.
    let parts: Vec<Component> = Path::new("/var/./log/../cache").components().collect();
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[0], Component::RootDir);
    assert_eq!(parts[1], Component::Normal("var".as_ref()));
    assert_eq!(parts[3], Component::ParentDir);
}
```

Look at the end. `components()` dropped the `.` in the middle, because "the same directory" changes nothing. It kept the `..`. A lexical method cannot remove `log/..`: if `log` is a symbolic link, then `log/..` is not `/var`. Only the file system knows, and `std::fs::canonicalize` asks it.

**Mistakes people make.**

Building paths with string operations:

```rust,compile_fail,E0369
use std::path::Path;

fn main() {
    let dir = Path::new("reports");
    let file = dir + "/june.csv";
    println!("{}", file.display());
}
```

```text
error[E0369]: cannot add `&str` to `&Path`
  |
5 |     let file = dir + "/june.csv";
  |                --- ^ ----------- &str
  |                |
  |                &Path
```

Use `dir.join("june.csv")`. It also puts in the separator of the platform. `join` has one surprise when its argument is an absolute path; the first prediction of the lesson asks you about it.

Calling `path.to_str().unwrap()`. This panics on the first file whose name is not Unicode. Such files are rare on a developer's machine and normal on servers that store what users upload. Stay with `OsStr` methods, as the example does. When you really need text, handle the `None`.

Comparing an `OsStr` with a string in the wrong form:

```rust,compile_fail,E0308
use std::path::Path;

fn main() {
    let path = Path::new("photo.JPG");
    if path.extension() == Some("JPG") {
        println!("a photo");
    }
}
```

```text
error[E0308]: mismatched types
  |
5 |     if path.extension() == Some("JPG") {
  |                            ---- ^^^^^ expected `&OsStr`, found `&str`
  |                            |
  |                            arguments to this enum variant are incorrect
```

An `OsStr` can be compared with a `str` directly, so compare inside the `Option`:

```rust
use std::path::Path;

let path = Path::new("photo.JPG");
assert!(path.extension().is_some_and(|extension| extension == "JPG"));
```

Treating a lexical check as a security check. "This path has no `..` and is relative" does not mean "this path stays inside my directory". A symbolic link inside the directory can point anywhere, and the file system can change between your check and your use. Lexical checks catch honest mistakes. Keeping a hostile user inside a directory needs help from the operating system.

**Read more.** [`std::path::Path`](https://doc.rust-lang.org/std/path/struct.Path.html) · [`PathBuf`](https://doc.rust-lang.org/std/path/struct.PathBuf.html) · [`Component`](https://doc.rust-lang.org/std/path/enum.Component.html) · [`std::ffi::OsStr`](https://doc.rust-lang.org/std/ffi/struct.OsStr.html)

## Lesson 06 · Directory metadata

**The model.** The functions in `std::fs` ask the file system to do something now. Every one returns `io::Result`, because the answer depends on a world that your program does not control: another program can create, change or delete a file at any moment.

Some functions do a whole job in one call:

| Function | Does |
| --- | --- |
| `fs::read(path)`, `fs::read_to_string(path)` | reads a whole file |
| `fs::write(path, bytes)` | creates the file, or replaces its content |
| `fs::create_dir(path)`, `fs::create_dir_all(path)` | one directory; a directory with all missing parents |
| `fs::remove_file`, `fs::remove_dir`, `fs::remove_dir_all` | deletes |
| `fs::rename(from, to)`, `fs::copy(from, to)` | moves; copies |
| `fs::metadata(path)` | size, kind, times, permissions |

`fs::read_dir(path)` lists a directory. It is fallible twice. The call itself can fail (no such directory, no permission). Then it returns an iterator, and each *item* is an `io::Result<DirEntry>`, because reading the next entry can fail too. A `DirEntry` gives you `file_name()` (an `OsString`), `path()`, `file_type()` and `metadata()`.

Two rules cause most surprises:

- **The order of entries is not specified.** It differs between file systems and can change when files are added. A listing with a stable order sorts.
- **Symbolic links need a decision.** A symbolic link is a small file that holds the path of another file. `fs::metadata(path)` *follows* the link and describes the target. `fs::symlink_metadata(path)`, `DirEntry::file_type()` and `DirEntry::metadata()` describe the link itself.

**Worked example.** The program builds a small tree in the temporary directory, finds the largest regular file directly inside it, and removes the tree again.

```rust
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::Path;

/// Name and size of the largest regular file directly inside `dir`.
/// Of several files with that size, the one with the smallest name.
fn largest_file(dir: &Path) -> io::Result<Option<(OsString, u64)>> {
    let mut best: Option<(OsString, u64)> = None;
    for entry in fs::read_dir(dir)? {
        let entry = entry?; // each item can be an error
        let meta = entry.metadata()?; // describes the entry itself, not a link target
        if !meta.is_file() {
            continue;
        }
        let candidate = (entry.file_name(), meta.len());
        let better = match &best {
            None => true,
            // The second rule makes the answer independent of the listing order.
            Some((name, size)) => {
                candidate.1 > *size || (candidate.1 == *size && candidate.0 < *name)
            }
        };
        if better {
            best = Some(candidate);
        }
    }
    Ok(best)
}

fn main() -> io::Result<()> {
    let root = std::env::temp_dir().join(format!("concepts-ch18-{}", std::process::id()));
    fs::create_dir(&root)?;
    fs::write(root.join("b.log"), "12345")?;
    fs::write(root.join("a.log"), "54321")?;
    fs::write(root.join("tiny"), "1")?;
    fs::create_dir(root.join("archive"))?;
    fs::write(root.join("archive/huge"), "a much longer content")?;

    let found = largest_file(&root);
    let nothing = largest_file(&root.join("archive/huge"));
    fs::remove_dir_all(&root)?; // clean up before any assertion can fail

    assert_eq!(found?, Some((OsString::from("a.log"), 5)));
    assert!(nothing.is_err(), "a file is not a directory");
    Ok(())
}
```

The function does not descend into `archive`, because `read_dir` lists one level. It decides what to do with equal sizes; without that rule the result would depend on the order in which the file system lists the entries.

**Mistakes people make.**

Using an entry before checking it:

```rust,compile_fail,E0599
use std::fs;

fn main() -> std::io::Result<()> {
    for entry in fs::read_dir(".")? {
        println!("{}", entry.path().display());
    }
    Ok(())
}
```

```text
error[E0599]: no method named `path` found for enum `Result<T, E>` in the current scope
  |
5 |         println!("{}", entry.path().display());
  |                              ^^^^ method not found in `Result<DirEntry, std::io::Error>`
  |
help: use the `?` operator to extract the `DirEntry` value, propagating a `Result::Err` value to the caller
  |
5 |         println!("{}", entry?.path().display());
  |                             +
```

Checking first and acting second. `if !path.exists() { fs::write(path, data)? }` has a gap between the question and the action, and another program can create the file in that gap. This is called a *time-of-check to time-of-use* problem. Ask the operating system to do both steps as one, and handle its answer:

```rust
use std::fs::OpenOptions;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let path = std::env::temp_dir().join(format!("concepts-ch18-new-{}", std::process::id()));
    let create = || OpenOptions::new().write(true).create_new(true).open(&path);

    let mut file = create()?; // creates the file, or fails
    file.write_all(b"first")?;
    // A second attempt cannot overwrite it.
    let second = create().map(|_| ());
    std::fs::remove_file(&path)?;
    assert_eq!(second.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    Ok(())
}
```

Reading "false" as "no". `Path::exists` returns a plain `bool`, so it answers `false` for every problem: a missing file, but also a directory that you may not read, or a disk that has failed. When the difference matters, call a function that returns a `Result` and look at the `ErrorKind`.

Comparing a listing with a fixed list without sorting. The test passes on your machine and fails on another file system.

**When to use which.** Use `fs::read_to_string` and `fs::write` for small files that you handle as a whole. Open a `File` and wrap it in `BufReader` or `BufWriter` (Lessons 02 and 03) when the file is large or when you process it in pieces. Use `create_dir_all` when "it already exists" is fine, and `create_dir` when it would be a mistake.

**Read more.** [`std::fs`](https://doc.rust-lang.org/std/fs/index.html) · [`fs::read_dir`](https://doc.rust-lang.org/std/fs/fn.read_dir.html) · [`DirEntry`](https://doc.rust-lang.org/std/fs/struct.DirEntry.html) · [`OpenOptions`](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html) · [`Metadata`](https://doc.rust-lang.org/std/fs/struct.Metadata.html)

## Lesson 08 · Framed messages over TCP

Do this lesson after Lesson 06 and before the capstone.

**The model.** TCP connects two programs with two streams of bytes, one in each direction. It promises that the bytes arrive complete and in order. It does **not** keep your writes apart. What the sender wrote in two calls can arrive in one read, and what it wrote in one call can arrive in three.

```text
sender calls       write_all("HELLO")    write_all("BYE")
bytes that travel  H E L L O B Y E
receiver may read  "HE"   then "LLOBY"   then "E"        any split is possible
```

So the two sides must agree on where a message ends. This agreement is called *framing*. One way is a delimiter, such as a newline after each message. The other way is a *length prefix*: first a number of fixed width that says how many bytes follow, then the bytes.

```text
00 00 00 05  H E L L O   00 00 00 03  B Y E
'-- 5 ----'  '- body -'  '-- 3 ----'  'body'
```

The standard library types are these:

- `TcpListener::bind(addr)` opens a port and starts to listen on it. It returns at once. With `"127.0.0.1:0"` the address is the *loopback* interface, which only programs on the same machine can reach, and port 0 asks the operating system for any free port. `local_addr()` tells you which one you got.
- `listener.accept()` blocks until a client connects and returns a `TcpStream` for this one client.
- `TcpStream::connect(addr)` is the client's side.
- A `TcpStream` is a reader and a writer. So is a `&TcpStream`, which lets you read and write through shared references to one socket.

How does a reader learn that the peer has finished? In the same way as in Lesson 01: `read` returns `Ok(0)`. A peer can close only its sending direction with `shutdown(Shutdown::Write)` and keep receiving. This *half-close* says "I have sent everything, now I listen".

**Worked example.** This is a different protocol from the one in the lesson: a newline ends each message. The server answers each line with its length. The server runs on a second thread, so that both sides can be in one program. `thread::spawn` starts the closure on a new thread, and `join` waits for it and gives back its result.

```rust
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?; // port 0: any free port
    let addr = listener.local_addr()?;

    let server = thread::spawn(move || -> io::Result<usize> {
        let (stream, _peer) = listener.accept()?;
        let mut answers = &stream; // &TcpStream is a writer
        let mut lines = 0;
        for line in BufReader::new(&stream).lines() {
            let line = line?;
            writeln!(answers, "{}", line.len())?;
            lines += 1;
        }
        Ok(lines) // the loop ended: the client finished sending
    });

    let mut client = TcpStream::connect(addr)?;
    client.write_all(b"alpha\nbe")?; // the message borders are ours,
    client.write_all(b"ta\n")?; // not the borders of the writes
    client.shutdown(Shutdown::Write)?; // "I have sent everything"
    let mut answers = String::new();
    client.read_to_string(&mut answers)?; // until the server closes
    assert_eq!(answers, "5\n4\n");
    assert_eq!(server.join().expect("server thread panicked")?, 2);
    Ok(())
}
```

The client may send everything first and read afterwards only because the data is tiny. The operating system holds a limited number of unread bytes for each direction. With megabytes, the server would stop because nobody reads its answers, the client would stop because the server no longer reads, and both would wait forever. A protocol in which the sides take turns, one request and then one reply, does not have this problem. The lesson uses turns.

**Mistakes people make.**

Treating one `read` as one message. On the loopback interface, small messages usually do arrive whole, so the program passes its tests and fails later on a real network. Read a header with a loop or `read_exact`, then read exactly the announced number of bytes.

Calling `read_to_end` or `read_to_string` to receive one message. These return when the *stream* ends. If the peer keeps the connection open and waits for your reply, both sides wait forever.

Trusting the length. The number in a header was chosen by the other side. If you allocate a buffer of that size first and check later, one small packet can make your program ask for gigabytes. Compare the length with a limit, and only then create the buffer.

Giving the stream away:

```rust,compile_fail,E0382
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:7000")?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    stream.write_all(b"thanks\n")?;
    Ok(())
}
```

```text
error[E0382]: borrow of moved value: `stream`
  |
5 |     let mut stream = TcpStream::connect("127.0.0.1:7000")?;
  |         ---------- move occurs because `stream` has type `TcpStream`, which does not implement the `Copy` trait
6 |     let mut reader = BufReader::new(stream);
  |                                     ------ value moved here
...
9 |     stream.write_all(b"thanks\n")?;
  |     ^^^^^^ value borrowed here after move
```

Write `BufReader::new(&stream)`, as the worked example does, and write through another `&stream`.

Buffering replies and never flushing. With a `BufWriter` around a socket, a reply stays in memory until you call `flush`. The peer waits for it, you wait for the peer's next request, and nothing moves.

**When to use which.** A delimiter suits text that never contains the delimiter. A length prefix suits arbitrary bytes and lets the receiver check the size before it reads. Put a `BufReader` around a socket when you read many small pieces, and a `BufWriter` when you write header and body separately, so that they leave together; then flush after each message. In a program that runs for a long time, set `set_read_timeout`, so that a silent peer produces an error and not a thread that waits forever.

**Read more.** [`std::net::TcpListener`](https://doc.rust-lang.org/std/net/struct.TcpListener.html) · [`TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html) · [`TcpStream::shutdown`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.shutdown) · [`std::thread::spawn`](https://doc.rust-lang.org/std/thread/fn.spawn.html)

## Lesson 09 · Child processes and pipes

**The model.** A *process* is a running program. Your program can start another one, called a *child process*, with `std::process::Command`. A `Command` is a builder: you describe the program, its arguments, its environment variables and its three standard streams, and nothing happens until you call one of three methods.

| Method | Standard streams by default | Returns |
| --- | --- | --- |
| `status()` | all three shared with the parent | waits; the `ExitStatus` |
| `output()` | no input; stdout and stderr collected | waits; an `Output` with the status and both byte vectors |
| `spawn()` | all three shared with the parent | at once; a `Child` that is running |

For each stream you can choose `Stdio::inherit()` (share the parent's), `Stdio::null()` (connect to nothing) or `Stdio::piped()`. A *pipe* is a one-way byte stream between two processes, kept by the operating system. After `spawn`, the parent's ends are in `child.stdin`, `child.stdout` and `child.stderr`, each an `Option`.

```text
parent                                              child
child.stdin   --write-->  [ pipe: limited space ]  --read-->   standard input
child.stdout  <--read---  [ pipe: limited space ]  <--write--  standard output
```

Three facts about pipes explain almost every problem with child processes:

1. **A pipe has limited space.** When it is full, the writer waits until the reader takes something out.
2. **A reader sees the end only when every writing end is closed.** A handle is closed when it is dropped.
3. **Writing to a pipe that nobody can read fails** with `ErrorKind::BrokenPipe`. (A Rust program sees this error because its start-up code tells Unix not to end the process with the `SIGPIPE` signal. Programs in other languages may simply be ended at that write.)

The parent does not have to sit in the middle of every pipe. `Stdio::from(handle)` turns a pipe end that you hold, such as a `ChildStdout` taken from one child, into a `Stdio` that you can give to another `Command` as its standard input. The first child then writes straight to the second one, and no byte passes through your program.

An `ExitStatus` tells you how the child ended. `success()` is true for exit code 0. `code()` gives the number, or `None` when a signal ended the child (on Unix). A child that fails is **not** an `Err`. The `io::Result` of `status`, `output` and `spawn` only tells you whether the operating system could start the program and wait for it.

**Worked example.** A program needs another program to start. To work on every system, this one starts *itself* a second time and uses an environment variable to tell the copy that it is the child. The parent sends three lines, then collects standard output and standard error at the same time.

```rust
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::thread;

/// The child's job: count the lines on standard input.
fn child() -> ! {
    let mut text = String::new();
    let code = match io::stdin().read_to_string(&mut text) {
        Ok(_) => {
            println!("{}", text.lines().count());
            0
        }
        Err(e) => {
            eprintln!("bad input: {e}");
            2
        }
    };
    std::process::exit(code)
}

/// Runs the child with `input` and returns (exit code, stdout, stderr).
fn count_lines(input: &[u8]) -> io::Result<(Option<i32>, String, String)> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .env("CONCEPTS_CHILD", "1") // seen by the child only
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?; // the program starts here

    // `take` moves the handle out and leaves None, so `child` stays whole.
    let mut stdin = child.stdin.take().expect("stdin is piped");
    stdin.write_all(input)?; // a few bytes: they fit into the pipe
    drop(stdin); // closed: now the child can see the end of its input

    let mut stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let (mut out, mut err) = (String::new(), String::new());
    // One thread per pipe, so that neither pipe can fill up while we read
    // the other one. A scoped thread may borrow local variables; the scope
    // ends only when the thread has finished.
    thread::scope(|scope| -> io::Result<()> {
        let errors = scope.spawn(|| stderr.read_to_string(&mut err));
        stdout.read_to_string(&mut out)?;
        errors.join().expect("the reading thread panicked")?;
        Ok(())
    })?;
    let status = child.wait()?; // always collect the exit status
    Ok((status.code(), out, err))
}

fn main() -> io::Result<()> {
    if std::env::var_os("CONCEPTS_CHILD").is_some() {
        child();
    }
    let (code, out, err) = count_lines(b"one\ntwo\nthree\n")?;
    assert_eq!((code, out.trim(), err.as_str()), (Some(0), "3", ""));

    // Input that is not UTF-8: the child reports failure with its exit code.
    let (code, out, err) = count_lines(b"\xff")?;
    assert_eq!((code, out.as_str()), (Some(2), ""));
    assert!(err.starts_with("bad input"));
    Ok(())
}
```

`Command::output` does the two-pipe reading for you, and it is the right choice whenever the child needs no input. The long form above shows what happens inside, and you need it when you feed the child.

**Mistakes people make.**

Waiting before reading. `child.wait()` returns when the child exits. A child with much output cannot exit: it is stopped in a `write`, because the pipe is full (fact 1) and the parent is not reading. Read first, then wait; or call `wait_with_output` or `output`, which read while they wait.

Writing everything, then reading everything. This works while the data fits into the pipes and stops forever when it does not. The child stops writing its output because nobody reads it, so it stops reading its input, so the parent's write never finishes. The data must flow in both directions at the same time: write the input on one thread and read the output on another, as the example does for two outputs.

Keeping a writing end open. A child that reads until the end of its input never finishes while the parent still holds `child.stdin` (fact 2). Drop it when you have written everything. The same is true for a `Command` that was given a pipe end with `Stdio::from`: it holds that end until the `Command` itself is dropped.

Moving a handle out with `unwrap`:

```rust,compile_fail,E0382
use std::io::Write;
use std::process::{Command, Stdio};

fn main() -> std::io::Result<()> {
    let mut child = Command::new("sort").stdin(Stdio::piped()).spawn()?;
    let mut stdin = child.stdin.unwrap();
    stdin.write_all(b"b\na\n")?;
    drop(stdin);
    child.wait()?;
    Ok(())
}
```

```text
error[E0382]: borrow of partially moved value: `child`
  |
6 |     let mut stdin = child.stdin.unwrap();
  |                                 -------- `child.stdin` partially moved due to this method call
...
9 |     child.wait()?;
  |     ^^^^^ value borrowed here after partial move
  |
  = note: partial move occurs because `child.stdin` has type `Option<ChildStdin>`, which does not implement the `Copy` trait
```

Use `child.stdin.take()`. It moves the handle out and leaves `None` behind, so `child` is still complete.

Putting the arguments into the program name. `Command::new("rustc --version")` looks for a program whose name contains a space and fails to start. No shell splits the text for you. Write `Command::new("rustc").arg("--version")`.

Ignoring the exit status. `let output = command.output()?;` succeeds when the child ran, also when it printed an error message and exited with 1. Check `output.status.success()` before you use `output.stdout`.

**When to use which.** Use `status()` when the child should talk to the terminal directly and you only want to know whether it succeeded. Use `output()` when you want what it printed and it needs no input. Use `spawn()` when you send input, read while the child runs, or connect two children. Dropping a `Child` does not stop the process and does not wait for it, so call `wait` in every path.

**Read more.** [`std::process::Command`](https://doc.rust-lang.org/std/process/struct.Command.html) · [`Child`](https://doc.rust-lang.org/std/process/struct.Child.html) · [`Stdio`](https://doc.rust-lang.org/std/process/struct.Stdio.html) · [`ExitStatus`](https://doc.rust-lang.org/std/process/struct.ExitStatus.html) · [`std::thread::scope`](https://doc.rust-lang.org/std/thread/fn.scope.html)

## Lesson 07 · Capstone: a bounded record service

Do the capstone last, after Lessons 08 and 09. It has no new concept. It asks you to combine the earlier ones from the contract alone, without a section to imitate, and it uses in-memory readers and writers, so every case can be tested exactly.

**What it combines.** The service receives one request frame, changes the payload, and sends one reply frame. Framing comes from Lessons 03, 04 and 08. Here the header is 2 bytes in little-endian order, as in Lessons 03 and 04, and not the 4 big-endian bytes of Lesson 08. Read the contract and do not copy numbers from memory.

**How to approach it.**

1. Write `receive` first and make its tests pass before you start `handle_one`. The second function should call the first one.
2. In `receive`, put the steps in an order that you can defend: read the header, decode the length, compare it with `max`, and only then create the buffer and read the body. For each step, ask which error the caller sees if the input ends there. Lesson 01 tells you which read call reports an early end as an error.
3. In `handle_one`, think about the promise "no output if receiving fails". The simplest way to keep a promise of this kind is to finish every step that can reject the request before the first write.
4. The reply needs a length. Ask whether that conversion can fail here, and why or why not.
5. Decide where the flush belongs, and remember Lesson 03: who knows that the output is complete?

**Check your own work.** The tests give your functions readers that end early, a writer that accepts one byte per call, a header that announces far more than `max`, and a payload that is not UTF-8. Before you run them, write down for each of these what your code does. Then add one case of your own. For example: what should happen when two frames arrive and the second one is too large?

**A design question for your notes.** `receive` takes `max` as a parameter and does not use a constant. Who should choose that number in a real service, and what goes wrong when it is too small or too large?

**Read more.** [`Read::read_exact`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact) · [`Write::write_all`](https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all) · [`slice::reverse`](https://doc.rust-lang.org/std/primitive.slice.html#method.reverse)
