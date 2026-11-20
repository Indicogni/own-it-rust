# kvstore: specification

This document says exactly what your program must do. It does not say how.
There are no types, traits or function signatures here, because choosing
them is the work of this capstone. What is fixed is everything another
program can observe: the command line, the bytes in the log file, the bytes
on the socket, the exit codes, and what survives a crash.

Words used with a fixed meaning:

- **must**: required. Where a test checks it, section 15 names the test file.
- **should**: expected of a good solution, but no test can see it. The
  rubric asks about these.
- **may**: your decision.

A *byte* is written in hexadecimal with the prefix `0x`. `LF` is the line
feed, byte `0x0A`. `SP` is the space, byte `0x20`. `CR` is the carriage
return, byte `0x0D`. All multi-byte numbers in files are **little-endian**:
the least significant byte comes first.

Contents:

1. [What you build](#1-what-you-build)
2. [Build rules](#2-build-rules)
3. [The command line](#3-the-command-line)
4. [The data directory](#4-the-data-directory)
5. [The log format](#5-the-log-format)
6. [Recovery](#6-recovery)
7. [Keys and values](#7-keys-and-values)
8. [The protocol](#8-the-protocol)
9. [Durability](#9-durability)
10. [Compaction](#10-compaction)
11. [Concurrency](#11-concurrency)
12. [Graceful shutdown](#12-graceful-shutdown)
13. [Limits](#13-limits)
14. [Non-goals](#14-non-goals)
15. [Milestones and their tests](#15-milestones-and-their-tests)
16. [Stretch goals](#16-stretch-goals)

---

## 1. What you build

`kvstore` is a server that stores pairs of a *key* and a *value* and keeps
them when it is stopped, killed, or cut off in the middle of a write.

- Clients connect over TCP on the loopback interface and send one-line
  requests such as `SET fruit apple` and `GET fruit`.
- Every change is appended to one file, the **log**, as a *record* with a
  checksum. The log is never changed in the middle; it only grows.
- The server keeps an **index** in memory that lets it answer a `GET`
  without reading the whole log.
- When the server starts, it reads the log from the beginning to rebuild the
  index. This is **recovery**. A log that ends in half a record, or in
  damaged bytes, is repaired by cutting off the bad end.
- A log that only grows fills the disk. **Compaction** writes a new, short
  log that holds only the newest value of every key, and replaces the old
  log with it in one step that a crash cannot split.
- Many clients are served at the same time.
- A `SHUTDOWN` request stops the server cleanly.

The design is the one used, with many refinements, by log-structured
storage engines. You build the plain version, completely.

## 2. Build rules

- The package builds a binary named `kvstore`. The acceptance tests start
  that binary; they call nothing else.
- Use the standard library only. Section 16 says when a dependency becomes
  reasonable.
- You do not need `unsafe`. A solution should carry
  `#![forbid(unsafe_code)]`.
- The program must build without warnings, in the debug and in the release
  profile, and behave the same in both.
- The program must work on Linux, macOS and Windows. Do not start other
  programs, and do not depend on signals.
- How you divide the code into modules, which types you define, whether you
  use one thread per connection or something else: all of that is yours to
  decide. Section 11 says what the result must guarantee.

## 3. The command line

### 3.1 Arguments

```text
kvstore --dir <DIR> [--port <PORT>] [--fsync <MODE>] [--auto-compact <BYTES>] [--max-clients <N>]
```

| option | value | default | meaning |
|---|---|---|---|
| `--dir <DIR>` | a path | required | the data directory (section 4) |
| `--port <PORT>` | 0 to 65535 | 7878 | the TCP port; 0 lets the operating system choose a free one |
| `--fsync <MODE>` | `always` or `never` | `always` | whether every write is synced to the disk (section 9) |
| `--auto-compact <BYTES>` | 0 to 2^64 - 1 | 1048576 | the garbage threshold for automatic compaction; 0 switches it off (section 10.3) |
| `--max-clients <N>` | 1 or more | 256 | how many connections are served at once (section 11.3) |

Every option is followed by exactly one value, which is the next argument.
Options may come in any order. Numbers are written in decimal digits only:
no sign, no spaces, no other characters. So `+80`, `-1`, `1e6` and an empty
value are not numbers. Leading zeros are allowed: `007` is 7.

The server always listens on the address `127.0.0.1`. It is not reachable
from other machines.

### 3.2 Usage errors

Each of these is a **usage error**:

- `--dir` is missing;
- an argument in the place of an option is not one of the five option names
  (this includes `--help` and stray words);
- an option is the last argument and has no value;
- a value is not allowed for its option (not a number, out of range, not
  `always` or `never`);
- an option appears more than once.

On a usage error the program must print a message to standard error, print
nothing to standard output, and exit with code 2. It must do so **before**
it creates the data directory or touches any file.

### 3.3 Start-up and the LISTENING line

A server that was started correctly does these things, in this order:

1. It prepares the data directory and recovers the log (sections 4 and 6).
2. It binds a listening socket to `127.0.0.1` and the given port.
3. It writes one line to standard output and flushes it:

   ```text
   LISTENING 127.0.0.1:<port>
   ```

   `<port>` is the real port number in decimal, also when `--port 0` was
   given. The line ends with `LF`.
4. It serves clients until it shuts down (section 12).

The program that started the server reads this line to learn the port, and
takes it as the promise that recovery is finished and connections are
accepted. So the line must not be printed earlier, and it must not stay in
a buffer.

This line is the only thing the server ever writes to standard output.
Messages for people (what recovery found, why a start failed) go to
standard error; their wording is yours.

### 3.4 Exit codes

| code | when |
|---|---|
| 0 | after a graceful shutdown (section 12) |
| 1 | the server stopped because of a failure while running (section 9.3) |
| 2 | usage error (section 3.2) |
| 3 | start-up failure |

A **start-up failure** is anything that stops the server between reading
its arguments and printing the LISTENING line:

- the data directory cannot be created or opened;
- another server is using the data directory (section 4);
- `kv.log` exists but is not a log (section 6);
- reading or repairing the log fails with an I/O error;
- the port cannot be bound, for example because it is in use.

On a start-up failure the program must print a message to standard error,
print nothing to standard output, and exit with code 3.

## 4. The data directory

The server keeps everything in the directory given with `--dir`. If the
directory does not exist, the server creates it, with any missing parent
directories.

| file | what it is |
|---|---|
| `kv.log` | the log (section 5). It exists, with at least its header, from the first successful start on. |
| `kv.compact` | the new log while a compaction writes it (section 10.2). It exists only during a compaction. |
| `kv.lock` | a file used to lock the directory. Its content does not matter. |

**One server per directory.** Two servers that append to the same log would
destroy it. So a server must take an exclusive lock on the directory when it
starts, and hold it until the process ends. If the lock is already held, the
start fails (exit code 3), and the failed server must not change any file in
the directory.

The lock must be one that the operating system takes away when the process
ends **in any way**, a kill included. A lock that consists of "the file
`kv.lock` exists" is wrong: after a crash it would still exist, and the
server could never start again without manual help.

## 5. The log format

The log is a header followed by zero or more records, with nothing between
them and nothing after them.

```text
+----------+----------+----------+-- ... --+----------+
|  header  | record 1 | record 2 |         | record n |
| 8 bytes  |          |          |         |          |
+----------+----------+----------+-- ... --+----------+
```

### 5.1 The header

The first 8 bytes of the file are the ASCII text `KVSTORE1`:

```text
4B 56 53 54 4F 52 45 31
```

These are the **magic bytes**. They say "this file is a kvstore log,
format version 1". A file that starts differently is not a log, and the
server must not touch it (section 6).

### 5.2 A record

| offset | size | field | content |
|---|---|---|---|
| 0 | 4 | `crc` | CRC-32 (section 5.3) of all bytes of this record after this field, as a 32-bit number |
| 4 | 1 | `kind` | 1 for a SET record, 2 for a DEL record |
| 5 | 2 | `klen` | the length of the key in bytes, as a 16-bit number |
| 7 | 4 | `vlen` | the length of the value in bytes, as a 32-bit number; 0 in a DEL record |
| 11 | `klen` | `key` | the key |
| 11 + `klen` | `vlen` | `value` | the value; absent in a DEL record |

A record is `11 + klen + vlen` bytes long. The first 11 bytes are its
**record header**. The checksum covers `kind`, `klen`, `vlen`, `key` and
`value`: `7 + klen + vlen` bytes.

- A **SET record** says: from here on, `key` has this `value`.
- A **DEL record** says: from here on, `key` has no value.

### 5.3 The checksum

The checksum is the CRC-32 that zip archives, gzip and PNG images use. Its
full name in catalogues of checksums is CRC-32/ISO-HDLC. In words:

```text
crc = 0xFFFFFFFF
for each byte b of the data, in order:
    crc = crc XOR b                      (b changes the lowest 8 bits)
    repeat 8 times:
        if the lowest bit of crc is 1:
            crc = (crc shifted right by 1) XOR 0xEDB88320
        else:
            crc = crc shifted right by 1
result = crc XOR 0xFFFFFFFF
```

All values are unsigned 32-bit numbers. Values to check your
implementation with:

| data (ASCII) | CRC-32 |
|---|---|
| (no bytes) | `0x00000000` |
| `a` | `0xE8B7BE43` |
| `123456789` | `0xCBF43926` |

You must implement the checksum yourself. The loop above is correct and
slow; a table of 256 precomputed values makes it about eight times faster,
and is the usual form.

### 5.4 Valid records

A sequence of bytes that starts at some offset of the file is a **valid
record** only if all of these hold:

1. At least 11 bytes follow the offset (a whole record header).
2. `kind` is 1 or 2.
3. `klen` is from 1 to 128.
4. `vlen` is from 0 to 4096 if `kind` is 1, and exactly 0 if `kind` is 2.
5. At least `11 + klen + vlen` bytes follow the offset (the whole record).
6. `crc` equals the CRC-32 of the `7 + klen + vlen` bytes after it.
7. Every byte of `key` is from `0x21` to `0x7E`, and no byte of `value` is
   `0x0A` (the rules of section 7).

The order matters to a reader. Conditions 2 to 4 can be checked with only
the record header in hand, and they must be checked before the lengths are
used for anything: a damaged `vlen` can claim four gigabytes.

A server only ever writes valid records.

### 5.5 Example

A new store receives `SET fruit apple`, then `SET n ` (an empty value),
then `DEL fruit`. Its log is now these 57 bytes:

```text
offset  bytes                                                     meaning
     0  4B 56 53 54 4F 52 45 31                                   header "KVSTORE1"

     8  B9 E8 3E B7                                               crc   0xB73EE8B9
    12  01                                                        kind  SET
    13  05 00                                                     klen  5
    15  05 00 00 00                                               vlen  5
    19  66 72 75 69 74                                            key   "fruit"
    24  61 70 70 6C 65                                            value "apple"

    29  1C 98 F5 A5                                               crc   0xA5F5981C
    33  01                                                        kind  SET
    34  01 00                                                     klen  1
    36  00 00 00 00                                               vlen  0
    40  6E                                                        key   "n"

    41  38 D1 90 80                                               crc   0x8090D138
    45  02                                                        kind  DEL
    46  05 00                                                     klen  5
    48  00 00 00 00                                               vlen  0
    52  66 72 75 69 74                                            key   "fruit"
    57  (end of file)
```

The store now holds one key, `n`, with an empty value.

## 6. Recovery

Recovery is what the server does with the data directory between taking the
lock and printing the LISTENING line. It must do the following.

1. **Remove an unfinished compaction.** If `kv.compact` exists, delete it
   (section 10.2 explains why that is always right).
2. **Find out what `kv.log` is.**
   - It does not exist, or it is empty: this is a new store. Create the
     file with the 8 magic bytes.
   - It is shorter than 8 bytes and its bytes are the beginning of the
     magic bytes (`K`, `KV`, ... `KVSTORE`): the server was interrupted
     while it created the file. Treat it as a new store: make the file
     exactly the 8 magic bytes.
   - It is 8 bytes or longer and starts with the magic bytes: it is a log.
     Continue with step 3.
   - Anything else: the file is **not a log**. This is a start-up failure
     (exit code 3), and the file must be left exactly as it is. It may be
     somebody's data that ended up under this name.
3. **Replay the valid prefix.** Start at offset 8 with an empty store. As
   long as a valid record (section 5.4) starts at the current offset, apply
   it and move the offset to the end of that record:
   - a SET record sets its key to its value, replacing an older value;
   - a DEL record removes its key; if the key has no value, nothing happens.

   Stop at the first offset where no valid record starts. That is the end
   of the file in a log that was closed properly. Call this offset the
   **valid length**.
4. **Cut off the rest.** If the file is longer than the valid length,
   shorten it to exactly the valid length. New records are appended there.

When the LISTENING line is printed, `kv.log` must already be the valid
prefix and nothing more, and `kv.compact` must not exist.

Recovery must read the log in one pass, in time proportional to its size,
and must not need memory proportional to its size beyond the index.

**Why "stop at the first bad record"?** A crash in the middle of an append
leaves a record that is cut off, or whose bytes only partly reached the
disk. That record is always the last one, and it was never acknowledged
(section 9.1), so dropping it loses nothing that anyone was promised. This
is the case the rule is made for.

Damage in the middle of the log is different: it means the disk or another
program changed bytes that had been written correctly. The rule treats it
in the same way, and therefore also drops the valid records behind the
damage. That loses data, and it is still the simplest behaviour that is
safe: without the length of the damaged record there is no reliable way to
find where the next record starts, and the state after recovery is always
"exactly what the first *k* writes produced", never a mixture. Section 16
has a stretch goal for a stricter policy.

**Why cut?** If the bad bytes stayed in the file and new records were
appended behind them, the next recovery would stop at the same place and
lose the new records. If new records were written over the bad bytes
without shortening the file, old records further back could line up behind
a new one and come back to life. The tests check for both.

The shortening should itself be made durable (synced) before the server
goes on.

## 7. Keys and values

- A **key** is 1 to 128 bytes. Every byte is from `0x21` to `0x7E`: the
  printable ASCII characters without the space. Keys are compared byte by
  byte; `Key` and `key` are different keys.
- A **value** is 0 to 4096 bytes. Any byte is allowed except `LF`
  (`0x0A`). A value is not text: it may contain `0x00`, `CR`, bytes that
  are not valid UTF-8, and spaces at either end. The server stores and
  returns it unchanged.
- The **store** is a set of keys, each with one value. A key that has a
  value is **live**.
- Where an order of keys is needed, it is **ascending byte order**: compare
  the first bytes; if they are equal, the second; a key that is a beginning
  of another comes first. In this order `0` < `B` < `a` < `a1` < `aa` <
  `b` < `~`.

## 8. The protocol

### 8.1 Lines

A client sends **requests** and the server answers each with a
**response**. A request is one line. A response is one line, except the
response to `KEYS`, which is one line followed by one line per key.

- A line ends with `LF`. The `LF` is not part of the line.
- `CR` is an ordinary byte. The protocol knows nothing about `CR LF` line
  endings: a client that sends `PING`, `CR`, `LF` has sent the line
  `PING\r`, which is not a command.
- A request line has at most **8192 bytes**, not counting the `LF`.
- The server reads a connection as a stream of bytes. One line may arrive
  in many small pieces, and many lines may arrive in one piece.
- A client may send several requests without waiting for responses
  (**pipelining**). The server answers them in the order they were sent.
- The response to a request must be sent without waiting for another
  request. If the server collects responses in a buffer, it must flush the
  buffer before it waits for more input.

**A line that is too long.** If more than 8192 bytes arrive before an
`LF`, the server must throw the line away up to and including its `LF`,
answer with the single response `ERR line too long`, and continue with the
next line. The server should not keep the bytes of such a line in memory:
a client that sends a gigabyte without an `LF` must not make the server
use a gigabyte.

The server may send this response as soon as it knows that the line is too
long, or when the `LF` of the line arrives; the order of the responses is
the same in both cases. If the connection ends before the `LF`, the line is
an unfinished line (see below), and the server may or may not have answered
it.

**The end of a connection.** When the client closes the connection, or
closes its sending half, the server answers every complete line it has
received, ignores an unfinished last line (bytes without an `LF` after
them), and closes the connection. An unfinished line is never executed.

### 8.2 From a line to a command

The server examines each line in these steps and stops at the first that
gives an error.

1. If the line is longer than 8192 bytes: `ERR line too long` (section
   8.1).
2. Split the line at its **first space**. The bytes before it are the
   *name*. The bytes after it are the *rest*. If the line has no space, the
   name is the whole line and there is **no rest**. A rest can be empty:
   the line `PING` followed by one space has the name `PING` and an empty
   rest, and that is different from having no rest.
3. If the name is not one of the ten command names below, written exactly
   in upper case: `ERR unknown command`.
4. Check the shape:

   | command | required shape | otherwise |
   |---|---|---|
   | `PING`, `STATS`, `COMPACT`, `QUIT`, `SHUTDOWN` | no rest | `ERR syntax` |
   | `GET`, `DEL`, `INCR` | a rest; the whole rest is the key | `ERR syntax` |
   | `KEYS` | no rest, or a rest; the whole rest is the prefix | (no error at this step) |
   | `SET` | a rest that contains a space; the bytes before its first space are the key, all bytes after that space are the value | `ERR syntax` |

5. If the key or the prefix is not a valid key (section 7: 1 to 128 bytes
   from `0x21` to `0x7E`): `ERR bad key`.
6. If the value is longer than 4096 bytes: `ERR value too large`.

Some consequences, all of which the tests check:

| line | result | because |
|---|---|---|
| (empty line) | `ERR unknown command` | the name is empty |
| `ping` | `ERR unknown command` | names are upper case |
| ` PING` | `ERR unknown command` | the name, before the first space, is empty |
| `PING` + `SP` | `ERR syntax` | there is a rest |
| `GET` | `ERR syntax` | there is no rest |
| `GET` + `SP` | `ERR bad key` | the key is empty |
| `GET a b` | `ERR bad key` | the key is `a b`, which contains a space |
| `SET k` | `ERR syntax` | the rest `k` contains no space |
| `SET k` + `SP` | sets `k` to the empty value | the value is everything after the second space |
| `SET k  x ` | sets `k` to ` x ` (space, `x`, space) | spaces in the value are kept |
| `SET` + `SP` + `SP` + `v` | `ERR bad key` | the key is empty |
| `GET a` + `CR` | `ERR bad key` | `CR` is not allowed in a key |

A request that is answered with any `ERR` response changes nothing in the
store and appends nothing to the log. After an `ERR` response the
connection continues normally.

### 8.3 PING

```text
PING
```

Response: `PONG`.

### 8.4 GET

```text
GET <key>
```

Response: `VALUE <value>` if the key is live: the word `VALUE`, one space,
then the bytes of the value. For an empty value the response is `VALUE`
followed by one space. If the key is not live, the response is
`NOT_FOUND`.

### 8.5 SET

```text
SET <key> <value>
```

Sets the key to the value, replacing an older value. Appends one SET
record to the log, also when the new value equals the old one. Response:
`OK`.

### 8.6 DEL

```text
DEL <key>
```

If the key is live: removes it, appends one DEL record, and responds `OK`.
If the key is not live: appends nothing and responds `NOT_FOUND`.

### 8.7 INCR

```text
INCR <key>
```

Adds 1 to a number stored as decimal text, as one atomic step (section
11.1).

- The *old number* is 0 if the key is not live. Otherwise the value must be
  an **integer**: an optional `+` or `-`, then one or more ASCII digits,
  and nothing else, with a numeric value from -9223372036854775808 to
  9223372036854775807 (a signed 64-bit number). Leading zeros are allowed.
  If the value is anything else, a number outside that range included, the
  response is `ERR not an integer`.
- If the old number is 9223372036854775807, the response is `ERR overflow`.
- Otherwise the key is set to the old number plus 1, written in decimal
  with a `-` if negative and without a `+` or leading zeros. One SET record
  is appended. The response is `VALUE <new number>`.

Examples: a value of `007` becomes `8`; `+5` becomes `6`; `-1` becomes `0`.
The values `1.5`, `0x10`, `5` + `SP`, `-` and the empty value are not
integers.

### 8.8 KEYS

```text
KEYS
KEYS <prefix>
```

Lists the live keys, or the live keys that begin with the prefix (a key
equal to the prefix included). The response is the line `KEYS <count>`,
followed by `<count>` lines with one key each, in ascending byte order
(section 7). With no matching key the response is the single line
`KEYS 0`.

The list is a snapshot: it shows the store as it was at one instant
(section 11.1), also while other clients write. `<count>` is always the
number of key lines that follow it.

### 8.9 STATS

```text
STATS
```

Response, on one line:

```text
STATS keys=<n> log_bytes=<n> live_bytes=<n> compactions=<n>
```

| field | meaning |
|---|---|
| `keys` | the number of live keys |
| `log_bytes` | the current size of `kv.log` in bytes |
| `live_bytes` | the size `kv.log` would have directly after a compaction: 8, plus `11 + klen + vlen` for every live key |
| `compactions` | the number of compactions completed since this process started, manual and automatic |

The numbers are decimal without leading zeros. The four fields must come
first and in this order; a server may add fields of the same `name=value`
form after them.

A new store answers `STATS keys=0 log_bytes=8 live_bytes=8 compactions=0`.

### 8.10 COMPACT

```text
COMPACT
```

Runs a compaction (section 10) and responds `OK` when the new log is in
place. Every `COMPACT` runs a compaction and is counted, also when the log
holds no garbage.

### 8.11 QUIT

```text
QUIT
```

Response: `BYE`. The server then closes this connection. Requests that were
sent after `QUIT` on the same connection are not executed. Other
connections and the server are not affected.

### 8.12 SHUTDOWN

```text
SHUTDOWN
```

Response: `OK`. The server then shuts down as section 12 describes.

### 8.13 All responses

| response | sent for |
|---|---|
| `PONG` | `PING` |
| `VALUE <bytes>` | `GET` of a live key; `INCR` |
| `NOT_FOUND` | `GET` or `DEL` of a key that is not live |
| `OK` | `SET`, `DEL` of a live key, `COMPACT`, `SHUTDOWN` |
| `KEYS <count>` and `<count>` key lines | `KEYS` |
| `STATS ...` | `STATS` |
| `BYE` | `QUIT` |
| `ERR line too long` | a line of more than 8192 bytes |
| `ERR unknown command` | section 8.2, step 3 |
| `ERR syntax` | section 8.2, step 4 |
| `ERR bad key` | section 8.2, step 5 |
| `ERR value too large` | section 8.2, step 6 |
| `ERR not an integer`, `ERR overflow` | `INCR` |
| `ERR too many connections` | a connection over the limit (section 11.3) |

The server sends nothing that is not in this table.

### 8.14 Example

Lines that start with `>` are sent by the client, lines with `<` by the
server. The marks are not part of the protocol.

```text
> PING
< PONG
> SET fruit apple
< OK
> SET motto to be, or not
< OK
> GET motto
< VALUE to be, or not
> GET nothing
< NOT_FOUND
> INCR visits
< VALUE 1
> INCR visits
< VALUE 2
> INCR fruit
< ERR not an integer
> KEYS
< KEYS 3
< fruit
< motto
< visits
> KEYS m
< KEYS 1
< motto
> DEL fruit
< OK
> DEL fruit
< NOT_FOUND
> get motto
< ERR unknown command
> STATS
< STATS keys=2 log_bytes=110 live_bytes=55 compactions=0
> COMPACT
< OK
> STATS
< STATS keys=2 log_bytes=55 live_bytes=55 compactions=1
> QUIT
< BYE
```

## 9. Durability

### 9.1 Written before acknowledged

`SET`, `DEL` and `INCR` are the **writes**. A write is **acknowledged**
when the server sends its success response (`OK` or `VALUE`).

Before a write is acknowledged, its complete record must have been handed
to the operating system as part of `kv.log`. Bytes that still sit in a
buffer inside the server process do not count: they are lost when the
process is killed.

It follows that **every acknowledged write survives a kill of the server
process**, at any moment. The tests kill the server without warning and
check exactly this. A write that was not acknowledged may or may not
survive.

### 9.2 Synced, if asked

Handing bytes to the operating system does not put them on the disk. The
system keeps them in memory and writes them out later, so a power failure
or a crash of the whole machine can still lose them.

- With `--fsync always` (the default) the server must, in addition, ask the
  operating system to put the record on the storage device, and wait for
  that, before it acknowledges the write. Then an acknowledged write also
  survives a power failure.
- With `--fsync never` the server does not sync after each write. This is
  much faster and gives up exactly that last guarantee.

In both modes the server must sync:

- the log, at a graceful shutdown (section 12);
- the new log of a compaction, before it replaces the old one (section
  10.2). In `never` mode a power failure may lose the last writes; it must
  never lose the whole store.

No test can cut the power, so the tests cannot see syncing. The rubric
asks you to show it in the code.

### 9.3 When the disk fails

If appending a record, syncing, or a compaction fails with an I/O error,
the server cannot keep its promises any more. It must not acknowledge the
write that failed, and it must not acknowledge any later write: the log may
now end in half a record, and whatever is appended behind that will be cut
off by the next recovery. The server should report the error on standard
error and exit with code 1. The next start repairs the log.

The tests do not cover this case.

## 10. Compaction

Every overwrite and every deletion leaves a record in the log that no
longer matters. The bytes of the log that a compaction would remove are
the **garbage**:

```text
garbage = log_bytes - live_bytes
```

### 10.1 What a compaction produces

After a compaction `kv.log` must consist of:

- the 8 magic bytes;
- then one SET record for every live key, with its current value, in
  ascending byte order of the keys;
- and nothing else.

So `log_bytes` equals `live_bytes` directly after a compaction, and two
servers with the same content produce the same file, byte for byte. The
content of the store does not change. Writes after the compaction are
appended to the new log as usual.

Other commands may have to wait while a compaction runs.

### 10.2 A crash during a compaction

At every instant, a crash must leave a `kv.log` that is either the complete
old log or the complete new log. The new log must therefore never be
written into `kv.log` itself. The procedure is:

1. Write the new log to the file `kv.compact` in the data directory.
2. Sync `kv.compact`.
3. Rename `kv.compact` to `kv.log`. A rename that replaces a file is a
   single step for the file system: afterwards the name `kv.log` refers to
   the new file, and there is no instant at which it refers to half of one.

When `COMPACT` is answered with `OK`, the rename has happened and
`kv.compact` does not exist.

If the server dies before step 3, the old `kv.log` is untouched, and a
`kv.compact` in an unknown state is left behind. It is useless: the old
log holds everything. That is why recovery deletes it (section 6, step 1).
If the server dies after step 3, `kv.compact` is gone and the new log is
in place.

An acknowledged write must not be lost by a compaction, whenever the server
is killed.

### 10.3 Automatic compaction

`--auto-compact <BYTES>` sets a threshold *N*.

- If *N* is 0, the server compacts only when it receives `COMPACT`.
- If *N* is greater than 0: whenever a write leaves `garbage >= N`, the
  server must compact without being asked. It may do so before it
  acknowledges that write, or shortly afterwards in the background. In
  either case the garbage must be below *N* again at most 5 seconds after
  the write was acknowledged, without any further request.
- While `garbage < N`, the server must not compact by itself.
- The rule speaks about writes. If the log that a server finds at start-up
  already holds *N* bytes of garbage or more, the server may compact at
  once or wait for the first write.

An automatic compaction is a compaction like any other: sections 10.1 and
10.2 apply, and `compactions` counts it.

## 11. Concurrency

### 11.1 Commands are atomic

Each command takes effect at one instant between the moment the server
received it and the moment it sent the response. The store behaves as if
the commands of all clients had been executed one after the other, in the
order of those instants. In particular:

- **No lost updates.** If 8 clients send 250 `INCR counter` each, the
  counter ends at 2000, and each number from 1 to 2000 was returned exactly
  once.
- **No torn values.** A `GET` returns a complete value that one `SET`
  wrote, never a mixture of two, and never bytes of another record, also
  while a compaction runs.
- **A client sees its own writes.** After a write was acknowledged, every
  later command on any connection sees it.
- **The log has the same order.** The records in `kv.log` appear in the
  order in which the writes took effect, and the bytes of two records are
  never interleaved. Replaying the log therefore gives exactly the store
  that the server was serving.

### 11.2 Many clients

The server must serve at least 64 connections at the same time.

A connection that is idle, that has sent only part of a line, or that does
not read its responses must not delay the service to other connections.
(Waiting for a compaction, or for a sync of another client's write, is not
a delay in this sense.)

### 11.3 The connection limit

At most `--max-clients` connections are served at once. When a new
connection arrives while that many are being served, the server must send
it the line `ERR too many connections` at once, without waiting for a
request, and close it. Such a connection does not count against the limit.

When a served connection ends, its place must become free without any
other event, within 5 seconds.

## 12. Graceful shutdown

When the server receives `SHUTDOWN`, it must:

1. answer `OK` to the client that asked;
2. stop accepting new connections;
3. let commands that are being executed finish. Every response that is
   sent, before or during the shutdown, stays true: a write that was
   acknowledged is in the log;
4. close every client connection, including connections that are idle, in
   the middle of a line, or not reading. A client sees its connection end;
   it gets no farewell line;
5. sync the log (in both fsync modes). The log then ends at a record
   boundary;
6. exit with code 0.

All of this must be finished **within 5 seconds** after the `OK`, whatever
the clients are doing. After the exit a new server can be started on the
same directory immediately.

A command that arrives while the server shuts down is either executed and
answered normally, or not executed at all. What may not happen is an
acknowledgement for a write that is not in the log.

There is no other way to stop the server cleanly. Ending the process from
outside (Ctrl-C, a kill) is a crash, and a crash is safe: that is what the
log and recovery are for.

## 13. Limits

| what | limit |
|---|---|
| key | 1 to 128 bytes, each from `0x21` to `0x7E` |
| value | 0 to 4096 bytes, no `0x0A` |
| request line | at most 8192 bytes, without its `LF` |
| record | at most 11 + 128 + 4096 = 4235 bytes |
| connections served at once | `--max-clients`, default 256; at least 64 must work |
| number of keys, size of the log | bounded only by memory and disk |
| start-up | proportional to the size of the log; one pass |
| shutdown | at most 5 seconds |
| freeing the place of a closed connection | at most 5 seconds |
| automatic compaction after the threshold is reached | at most 5 seconds |

## 14. Non-goals

You do not need any of these, and the tests do not look for them:

- access from other machines, authentication, encryption;
- transactions over several keys, or any command that touches more than
  one key atomically;
- values larger than 4096 bytes, or a store larger than what fits one log
  file and one index in memory;
- expiry of keys, publish/subscribe, replication;
- protection against a second program that deliberately writes into the
  data directory while the server runs;
- repair of damage in the middle of the log beyond the rule of section 6;
- time limits for idle connections;
- signal handling, running as a service, log rotation for the messages on
  standard error;
- high throughput. A simple, correct server is fast enough for every test.
  The rubric has a few numbers to measure yourself against.

## 15. Milestones and their tests

Work in this order. Each milestone has one test file in `tests/`; run it
with `cargo test --test <name>`. A later milestone never breaks an earlier
test.

| milestone | you build | test file | tests |
|---|---|---|---|
| M1 | arguments, the LISTENING line, the protocol (sections 3, 7, 8) with the store in memory only, one connection at a time, a simple `SHUTDOWN` | `m1_protocol` | 23 |
| M2 | the data directory and its lock, the log format with CRC-32, appending before acknowledging, reading an intact log at start-up (sections 4, 5, 9.1, and 6 for intact logs) | `m2_log` | 14 |
| M3 | recovery from every kind of damaged log (sections 5.4 and 6) | `m3_recovery` | 9 |
| M4 | `COMPACT`, crash-safe replacement of the log, automatic compaction (section 10) | `m4_compaction` | 10 |
| M5 | many clients at once, atomic commands, the connection limit (section 11) | `m5_concurrency` | 10 |
| M6 | shutdown with clients in every state (section 12) | `m6_shutdown` | 6 |

The tests of M1 to M4 use one connection at a time: none of them needs a
second connection to be served before the first one has ended. A server
that serves one client after the other passes all of them, so you can
leave threads alone until M5.

What turns green when:

- **M1**: every test in `m1_protocol`. `STATS` only needs a correct `keys`
  here; print 8, 8 and 0 for the other fields until M2.
- **M2**: every test in `m2_log`. `m2_the_example_of_spec_5_5` compares
  your file with the bytes of section 5.5 and is the first to make green.
- **M3**: every test in `m3_recovery`. Start with
  `m3_a_record_cut_off_at_any_byte_is_dropped`; the test
  `m3_random_crashes_match_a_model` is the summary of the milestone.
- **M4**: every test in `m4_compaction`.
  `m4_compact_writes_one_record_per_live_key_in_key_order` first.
  `m4_a_kill_in_the_middle_of_a_compaction_loses_nothing` watches the data
  directory and kills the server while `kv.compact` is being written, and
  at the moment `kv.log` is replaced.
- **M5**: every test in `m5_concurrency`. A server with one thread already
  passes some of them; `m5_a_client_that_stops_in_the_middle_of_a_line_blocks_nobody`
  is the one that needs real concurrency. These tests always pass for a
  correct server. A server with a race fails them often, but not on every
  run, so run the file many times before you believe a green result.
- **M6**: every test in `m6_shutdown`.

Each test starts its own server on its own port and its own directory, and
stops it again. No test waits longer than 10 seconds for anything; a server
that hangs makes a test fail, not the test run.

## 16. Stretch goals

None of these is tested. Each is a real technique from real storage
engines. Do the ones that interest you, and measure before and after.

1. **Group commit.** With `--fsync always`, each write waits for its own
   sync. Let concurrent writers share one: the first writer to reach the
   disk syncs for all records appended so far, and the others wait for that
   sync, not for their own. Measure writes per second with 1, 8 and 64
   clients.
2. **Compaction in the background.** Compact without stopping reads, and
   then without stopping writes: writes that arrive during the compaction
   go to the old log and have to reach the new one before the switch.
3. **Strict recovery.** Add an option that makes damage in the middle of
   the log (a bad record that is followed by more bytes than a torn write
   could explain) a start-up failure, with a separate repair command.
4. **Checksums on read.** Verify the record again when a `GET` reads it
   from the file, and decide what the server should do when it is wrong.
5. **Several log files.** Close the log at a size limit and start a new
   one. Compaction then merges old files while writes go to the newest.
   Add a small file per closed log that lists its keys and offsets, so
   that start-up does not have to read the values.
6. **A sync interval.** Add `--fsync every:<milliseconds>`: a middle way
   between `always` and `never`, with a clear statement of what can be
   lost.
7. **Conditional writes.** Add `CAS <key> <old> <new>` in a form that
   works with values that contain spaces, and use it to build a counter
   without `INCR`.
8. **A load generator.** Write `kvbench`, a second binary that opens *n*
   connections and reports throughput and latency percentiles. Use it for
   every other item of this list.
9. **An async rewrite.** Rebuild the network side on an async runtime
   (the mastery module [`m4_async`](../../mastery/m4_async/README.md) builds one by hand; here `tokio` is the
   reasonable dependency). The log and the index stay as they are. The
   interesting questions: where does the blocking file I/O go, and what
   replaces "close all the sockets" at shutdown?
10. **A follower.** Start a second server that connects to the first,
    receives its log from a given offset on, and applies it. This is the
    beginning of replication.
