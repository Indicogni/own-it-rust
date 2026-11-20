# Specification: `httpd`

This document is the contract. The acceptance tests in `tests/` check what
is written here and nothing else. If a test seems to demand something that
this document does not say, treat it as a bug in the course and trust this
document.

Sections 3 to 11 are the specification. Section 12 lists what you do not
have to build. Section 13 turns the specification into six milestones.
Section 14 has ideas for afterwards.

Contents:

1. [What you build](#1-what-you-build)
2. [Words and notation](#2-words-and-notation)
3. [The process](#3-the-process)
4. [The life of a request](#4-the-life-of-a-request)
5. [The request head](#5-the-request-head)
6. [Responses](#6-responses)
7. [Connections](#7-connections)
8. [Static files](#8-static-files)
9. [`POST /echo`](#9-post-echo)
10. [The worker pool](#10-the-worker-pool)
11. [Shutdown](#11-shutdown)
12. [Non-goals](#12-non-goals)
13. [Milestones](#13-milestones)
14. [Stretch goals](#14-stretch-goals)

---

## 1. What you build

`httpd` is one program. It listens on a TCP port and speaks a subset of
HTTP/1.1. It can do two things:

- send the files of one directory (`GET` and `HEAD`);
- check a JSON document and send it back in compact form (`POST /echo`).

Around these two small features is the part that makes it a server:

- a parser that never trusts its input and has a limit for everything;
- persistent connections, with requests that arrive in pieces or several
  at once;
- time limits, so that a slow or silent client cannot hold a thread;
- a fixed number of worker threads, a bounded waiting queue, and a clear
  answer when both are full;
- a shutdown that finishes the work in progress.

Rules for the whole project:

- **Standard library only.** No crates.
- **Safe Rust.** The server needs no `unsafe`. `Cargo.toml` forbids it.
- **One binary named `httpd`.** How you arrange the code behind it
  (modules, a library next to the binary) is your decision.
- **Blocking I/O and threads.** No `async`, no readiness API. (The mastery
  module [`m4_async`](../../mastery/m4_async/README.md) builds the other kind of server.)
- The program must behave the same on Linux, macOS and Windows.

---

## 2. Words and notation

- A **byte** is 8 bits. HTTP messages are bytes, not text: do not assume
  UTF-8 anywhere unless this document says so.
- `CR` is byte 13, `LF` is byte 10, `SP` is byte 32 (space), `HTAB` is
  byte 9. `CRLF` is `CR` followed by `LF`.
- In examples, `\r\n` stands for `CRLF`. A line that starts with `>` shows
  bytes from the client, a line that starts with `<` shows bytes from the
  server. The mark and the space after it are not part of the bytes, and
  neither is the line break of the example.
- A **client** is the program at the other end of a connection.
- **MUST**, **MUST NOT** and **MAY** have their usual meaning: required,
  forbidden, allowed.
- Grammar is written like this: `a b` means `a` then `b`; `a / b` means
  `a` or `b`; `*a` means zero or more; `1*a` means one or more; `[a]`
  means optional; `%x41` is the byte with that hexadecimal value;
  `%x30-39` is a range of byte values.

---

## 3. The process

### 3.1 Command line

```text
httpd --root DIR [--addr IP:PORT] [--workers N] [--queue N]
      [--max-body BYTES] [--request-timeout-ms MS] [--idle-timeout-ms MS]
httpd --help
```

| option | meaning | allowed values | default |
|---|---|---|---|
| `--root DIR` | the directory whose files are served | an existing directory | required |
| `--addr IP:PORT` | the address to listen on | an IPv4 address and port, or `[IPv6]:PORT`; port `0` lets the system choose | `127.0.0.1:8080` |
| `--workers N` | number of worker threads | 1 to 256 | 4 |
| `--queue N` | connections that may wait for a worker | 0 to 4096 | 16 |
| `--max-body BYTES` | largest request body | 0 to 16777216 | 1048576 |
| `--request-timeout-ms MS` | time allowed for one request (7.3) | 1 to 600000 | 5000 |
| `--idle-timeout-ms MS` | time a connection may wait for a request (7.3) | 1 to 600000 | 5000 |

Rules:

- An option and its value are two arguments. `--workers=4` is an error.
- Each option may be given at most once. The order does not matter.
- A number is one or more decimal digits and nothing else: no sign, no
  spaces, no fraction.
- `--addr` takes a literal address. Host names such as `localhost` are an
  error.
- Any other argument is an error.
- If `--help` appears anywhere, the program prints a usage text that names
  the options to standard output and exits with status 0. Nothing else
  happens: no socket is opened, and the rest of the command line is not
  checked, so `httpd --workers 0 --help` also exits with status 0.

### 3.2 Start-up

The program checks the command line first. Then, in any order, it checks
that the root is an existing directory and it opens the listening socket.

The root is fixed at start-up: if `DIR` is a relative path, it is resolved
once, and a later change of the working directory has no effect.

### 3.3 Standard output and standard error

When the socket is listening, and not before, the program prints one line
to standard output and flushes it:

```text
listening on 127.0.0.1:54321
```

The address is the one the socket really has, in the form
`SocketAddr` prints it. With port `0` on the command line, the line shows
the port the system chose. A parent process that reads this line may
connect at once.

A server writes exactly two lines to standard output during its life:
this one, and `shutting down` (11.2). Everything else that it wants to
say (errors, logs) goes to standard error. The format there is free.

### 3.4 Exit status

| status | when |
|---|---|
| 0 | after `--help`; after a graceful shutdown (section 11) |
| 2 | the command line is wrong (3.1). A message is on standard error, nothing is on standard output. |
| 1 | the program cannot start: the root does not exist or is not a directory, or the address cannot be used (for example, it is in use). A message is on standard error, nothing is on standard output. |

In both error cases the program exits by itself, at once.

---

## 4. The life of a request

A worker (section 10) serves one connection at a time. On a connection it
handles one request after the other. For each request it does these steps
in this order. **The order is part of the specification**: it decides
which status a request with several faults gets.

**Step 1: wait.** Wait for the first byte of a request. If none arrives
within the idle timeout, or the client closes the connection, close the
connection without a response (7.3).

**Step 2: the head.** Read until the head is complete (5.1), then check
it against the grammar of section 5.

| fault | status |
|---|---|
| the head is not complete after 8192 bytes (5.4) | 431 |
| an `LF` without a `CR` before it (5.1) | 400 |
| the head is not complete when the request timeout ends (7.3) | 408 |
| the request line or a header line breaks the grammar (5.2, 5.3) | 400 |
| the version is well formed but not `HTTP/1.1` or `HTTP/1.0` (5.2) | 505 |
| the target is longer than 2048 bytes (5.4) | 414 |
| more than 64 header lines (5.4) | 431 |
| the rule for `Host` is broken (5.3) | 400 |

The first three faults are found while reading. The others need a
complete head; if a head has several of them, the server MAY report any
one of them.

**Step 3: the length of the body.** Decide how many body bytes follow
the head. Check in this order:

| fault | status |
|---|---|
| there is a `Transfer-Encoding` header, with any value | 501 |
| there is more than one `Content-Length` header line | 400 |
| the value of `Content-Length` is not `1*DIGIT` | 400 |
| the value of `Content-Length` is larger than `--max-body` | 413 |
| the method is `POST` and there is no `Content-Length` | 411 |

Without a fault, the body is `Content-Length` bytes long, or empty if the
header is absent. This is true for every method, also for `GET`.

A length is "larger than `--max-body`" also when it has so many digits
that it does not fit any integer type. The server MUST answer 413 from
the number alone: it MUST NOT wait for, or read, a body that it refuses.

**Step 4: the body.** Read exactly that many bytes. If they have not all
arrived when the request timeout ends: 408. The body is read for every
method. Where this document gives the body no meaning (everything except
`POST /echo`), the server throws it away.

**Step 5: the answer.** The request is now complete. Look at the method
first, then at the path. The **path** is the target up to, and not
including, the first `?`. What follows the `?` is ignored everywhere.

| method | path | answer |
|---|---|---|
| not `GET`, `HEAD` or `POST` | any | 501 |
| `POST` | exactly `/echo` | section 9 |
| `GET` or `HEAD` | exactly `/echo` | 405 with the header `Allow: POST` |
| `POST` | anything else | 405 with the header `Allow: GET, HEAD` |
| `GET` or `HEAD` | anything else | section 8 |

`/echo` is compared byte for byte with the path as sent. `/echo/`,
`/Echo` and `/%65cho` are not the endpoint; they are file paths.

**Step 6: send.** Send the response (section 6) and decide whether the
connection stays open (section 7).

If the client closes its side of the connection before a request is
complete (during step 2 or step 4), the server closes the connection
without a response.

A fault found in steps 2 to 4 means that the server cannot be sure where
this request ends. So after such a response the server always closes the
connection (7.2). A fault found in step 5 does not have this problem.

### Example

```text
> POST /echo HTTP/1.1\r\n
> Host: example\r\n
> Content-Type: application/json\r\n
> Content-Length: 30\r\n
> \r\n
> { "a" : [ 1, 2.50 ], "b":null}

< HTTP/1.1 200 OK\r\n
< Date: Fri, 20 Nov 2026 11:01:42 GMT\r\n
< Content-Type: application/json\r\n
< Content-Length: 23\r\n
< Connection: keep-alive\r\n
< \r\n
< {"a":[1,2.50],"b":null}
```

Step 1 ended with the byte `P`. Step 2 found a well-formed head of 90
bytes. Step 3 read `Content-Length: 30`. Step 4 read 30 more bytes. Step 5
found `POST` and the path `/echo`, checked the content type and the JSON
(section 9), and built the body. Step 6 sent the response; the request
was complete, has version 1.1 and no `Connection: close`, so the
connection stays open.

---

## 5. The request head

```text
head         = request-line *header-line CRLF
request-line = method SP target SP version CRLF
header-line  = name ":" OWS value OWS CRLF

method       = token
target       = "/" *%x21-7E
version      = "HTTP/" DIGIT "." DIGIT
name         = token
value        = *( HTAB / %x20-7E / %x80-FF )

token        = 1*tchar
tchar        = ALPHA / DIGIT / "!" / "#" / "$" / "%" / "&" / "'" / "*"
             / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
OWS          = *( SP / HTAB )
ALPHA        = %x41-5A / %x61-7A
DIGIT        = %x30-39
```

### 5.1 Lines and the end of the head

The head ends with the first `CRLF CRLF`. Its length includes these four
bytes.

Inside the head, every `CR` MUST be followed by `LF` and every `LF` MUST
have a `CR` before it. Anything else is 400.

An `LF` without a `CR` before it MUST be answered **when it arrives**:
as soon as the bytes received so far contain such an `LF`, the server
sends 400 and does not wait for more. The reason is practical. Many
tools, and many people typing into a terminal, send `LF` alone. Such a
client never sends `CRLF CRLF`, so a server that waits for it answers 408
much later, and the 408 says nothing useful. (A `CR` without an `LF`
after it MAY be found later, when the head is complete.)

Nothing may come before the request line. An empty line first is 400.

### 5.2 The request line

- Exactly one `SP` separates the three parts. Two spaces, a tab, or a
  space at the start or the end of the line are 400.
- The method is a `token`. Method names are case-sensitive. `GET`, `HEAD`
  and `POST` are the methods this server implements. Any other token,
  such as `PUT`, `BREW` or `get`, is a valid request line; it is answered
  with 501 in step 5. A method with a character that is not a `tchar` is
  400.
- The target starts with `/` and has only bytes from `%x21` to `%x7E`
  (visible ASCII). Other forms (`*`, `http://host/path`) are 400.
- The version has the form `HTTP/` digit `.` digit. `HTTP/1.1` and
  `HTTP/1.0` are supported. Another version of the right form, such as
  `HTTP/2.0`, is 505. Any other text, such as `http/1.1`, `HTTP/1` or
  `HTTP/1.10`, is 400.

### 5.3 Header lines

- The name is a `token`. Names are compared without regard to ASCII case:
  `Host`, `host` and `HOST` are the same header.
- The name ends at the first colon of the line. No space is allowed
  between the name and the colon. The value may contain more colons.
- Spaces and tabs around the value are not part of the value. The value
  may be empty.
- A value may contain bytes from `%x80` to `%xFF`. The server does not
  interpret them. Control characters other than `HTAB` (bytes `%x00-08`,
  `%x0A-1F`, `%x7F`) are 400.
- A line that starts with `SP` or `HTAB` is 400. (Old clients used such
  lines to continue the line before. The grammar above has no place for
  them.)
- The same name may occur on several lines.

The server gives a meaning to five headers and ignores all others:

| header | rule |
|---|---|
| `Host` | A request with version `HTTP/1.1` MUST have exactly one `Host` line. A request with version `HTTP/1.0` may have one or none. More than one is always 400. The value is not interpreted and may be empty. |
| `Content-Length` | See step 3. The value is one or more digits; leading zeros are allowed. |
| `Transfer-Encoding` | See step 3. |
| `Connection` | A list of tokens separated by commas, with optional spaces and tabs around each. Tokens are compared without regard to case. If the header occurs on several lines, the lists are joined. See 7.1. |
| `Content-Type` | See 9.1. If it occurs more than once, the first line counts. |

### 5.4 Limits

| what | limit | status when exceeded |
|---|---|---|
| the head, from its first byte to the end of `CRLF CRLF` | 8192 bytes | 431 |
| header lines in one head | 64 | 431 |
| the target | 2048 bytes | 414 |
| the body | `--max-body` bytes | 413 |

A head of exactly 8192 bytes, 64 header lines (counting `Host`), a target
of exactly 2048 bytes and a body of exactly `--max-body` bytes are
accepted.

The limit for the head MUST be applied while reading: as soon as 8192
bytes of a request have arrived and the end of the head is not among
them, the server answers 431. It does not wait for the head to end, and
it does not buffer more of it.

The limits are per request. Body bytes do not count towards the head
limit, and a connection may carry any number of requests.

---

## 6. Responses

### 6.1 Format

```text
response    = status-line *header-line CRLF body
status-line = "HTTP/1.1" SP 3DIGIT SP reason CRLF
```

Header lines have the form of 5.3. The status line always says
`HTTP/1.1`, also in the answer to an `HTTP/1.0` request.

### 6.2 Status codes

The server uses these codes with exactly these reason phrases.

| code | reason phrase | used when |
|---|---|---|
| 200 | `OK` | a file is sent; a JSON value is echoed |
| 400 | `Bad Request` | malformed head (step 2), bad `Content-Length` (step 3), bad path (8.1), invalid JSON (9.2) |
| 403 | `Forbidden` | a path segment the server never serves (8.2); the system denies access to the file |
| 404 | `Not Found` | no such file (8.3) |
| 405 | `Method Not Allowed` | the method does not fit the path (step 5) |
| 408 | `Request Timeout` | the request did not arrive in time (7.3) |
| 411 | `Length Required` | `POST` without `Content-Length` (step 3) |
| 413 | `Content Too Large` | the body would be larger than `--max-body` (step 3) |
| 414 | `URI Too Long` | target longer than 2048 bytes |
| 415 | `Unsupported Media Type` | `POST /echo` without the JSON content type (9.1) |
| 431 | `Request Header Fields Too Large` | head longer than 8192 bytes, or more than 64 header lines |
| 500 | `Internal Server Error` | the server failed for a reason of its own before it could send a response |
| 501 | `Not Implemented` | unknown method (step 5); `Transfer-Encoding` (step 3) |
| 503 | `Service Unavailable` | the server holds as many connections as it may (10.3) |
| 505 | `HTTP Version Not Supported` | see 5.2 |

### 6.3 Headers of every response

Every response has each of these four headers exactly once:

| header | value |
|---|---|
| `Date` | the time at which the response is sent, in UTC, in the fixed format below |
| `Content-Type` | the media type of the body (8.4, 9.3, 6.4) |
| `Content-Length` | the length of the body in bytes, in decimal |
| `Connection` | `keep-alive` or `close` (7.1) |

Two responses have one more header: `Allow` on 405 (step 5) and
`Retry-After: 1` on 503 (10.3).

The order of the header lines is free. The server MAY add other headers,
for example `Server`.

The date has always 29 characters:

```text
Sun, 06 Nov 1994 08:49:37 GMT
```

```text
date  = day "," SP 2DIGIT SP month SP 4DIGIT SP 2DIGIT ":" 2DIGIT ":" 2DIGIT SP "GMT"
day   = "Mon" / "Tue" / "Wed" / "Thu" / "Fri" / "Sat" / "Sun"
month = "Jan" / "Feb" / "Mar" / "Apr" / "May" / "Jun"
      / "Jul" / "Aug" / "Sep" / "Oct" / "Nov" / "Dec"
```

The fields are: day of the month, month, year, hour (00 to 23), minute,
second. The day name MUST be the right one for the date. The clock is read
for every response: a date that was formatted once, when the server
started, is wrong a second later. The standard
library gives you the seconds since 1970-01-01 00:00:00 UTC
(`SystemTime::now().duration_since(UNIX_EPOCH)`); 1970-01-01 was a
Thursday. Turning that number into a calendar date is part of the
project.

### 6.4 The body of a response without content of its own

Every response whose status is not 200 has the same kind of body: the
status code, one space, the reason phrase, and one `LF`. Its
`Content-Type` is `text/plain; charset=utf-8`.

```text
< HTTP/1.1 404 Not Found\r\n
< Date: Fri, 20 Nov 2026 11:01:42 GMT\r\n
< Content-Type: text/plain; charset=utf-8\r\n
< Content-Length: 14\r\n
< Connection: keep-alive\r\n
< \r\n
< 404 Not Found
```

(The last line ends with `LF`: `404 Not Found` has 13 bytes, the body 14.)

### 6.5 `HEAD`, and sending without delay

The response to a `HEAD` request has the status line and the headers that
the same request with `GET` would get, including the `Content-Length` of
the body that `GET` would send. It has no body. This holds for every
status that step 5 produces, not only for 200.

There is one exception. When a request is refused in steps 2 to 4, the
server often cannot know the method: the head may be incomplete or
malformed. So after such a fault the server MAY send the body of 6.4 even
if the method was `HEAD`. This is harmless, because the connection is
closed after such a response (7.2), and the client cannot take the body
for the start of another response.

A response MUST NOT be held back. In particular, a small response should
leave the server as one unit. If a server sends the head and a small body
with two separate writes, the operating system may delay the second one
for about 40 milliseconds while it waits for the client to acknowledge
the first (look for "Nagle's algorithm" and "delayed acknowledgement").
One test sends 200 small requests on one connection and fails if that
takes 4 seconds or more. A healthy server needs a few hundredths of a
second for it.

---

## 7. Connections

### 7.1 Persistent connections

After a response, the connection stays open for another request if all of
these are true:

1. The request was complete: the response comes from step 5, not from a
   fault in steps 2 to 4.
2. The client did not ask to close:
   - version `HTTP/1.1`: the `Connection` header does not contain the
     token `close`;
   - version `HTTP/1.0`: the `Connection` header contains the token
     `keep-alive` and does not contain `close`.
3. The server is not shutting down (11.2).

If the connection stays open, the response says `Connection: keep-alive`.
Otherwise it says `Connection: close`, and the server closes the
connection after the response, as described in 7.4.

The header of the response MUST tell the truth: a client decides from it
whether it may send another request.

### 7.2 Which responses close the connection

| response | connection afterwards |
|---|---|
| 400 for a malformed head or a bad `Content-Length`; 408; 411; 413; 414; 431; 501 for `Transfer-Encoding`; 505 | always closed |
| 503 (10.3) | always closed |
| 200; 400 for a bad path or invalid JSON; 403; 404; 405; 415; 501 for an unknown method | as 7.1 decides |

### 7.3 Time limits

**Idle timeout** (`--idle-timeout-ms`). It runs while a worker waits for
the first byte of a request: from the moment the worker starts to serve a
new connection, and again from the moment it has sent a response. When it
ends, the server closes the connection **without sending anything**. An
idle client has asked for nothing, so there is nothing to answer.

**Request timeout** (`--request-timeout-ms`). It starts with the first
byte of a request and ends with the last byte of its body. It is a limit
for the request as a whole. A client that sends one byte every 100
milliseconds makes every single read succeed quickly; it MUST still get
408 when the limit is reached. After the 408 the connection is closed.

The two limits are independent. A request that has started is not idle,
however long the idle timeout is, and a short request timeout does not
shorten the wait between requests.

**Writing.** A client may stop reading. If the server cannot write a
single byte of a response for as long as the request timeout, it gives
up and closes the connection. Sending a large file to a slow client may
take longer than the request timeout in total; that is fine as long as it
makes progress.

Time that a connection spends waiting for a worker (10.2) is not limited
by either timeout. Both limits are measured by the worker that serves the
connection. If a client sends a request while its connection waits, the
request timeout starts when the worker reads the first byte, not when the
client sent it.

### 7.4 Closing so that the response arrives

There is a trap in TCP. If a program closes a socket while received bytes
are still unread, most systems do not close the connection normally: they
reset it. A reset tells the other side to give up at once, and it can
make the other side throw away data that it has received but not yet
read. That data is your response.

This happens exactly when the server answers early. A client uploads a
large body; the server reads the head, sees `Content-Length`, answers
413 and closes. The client is still sending. The reset arrives, and the
client reports "connection reset" instead of "413".

So, after a response with `Connection: close`, the server MUST close the
connection in three steps:

1. Close the sending direction only (`TcpStream::shutdown` with
   `Shutdown::Write`). The client now sees the end of the response.
2. Keep reading and throw the bytes away, until the first of: the client
   closes its side; 1 second has passed; 65536 bytes have been thrown
   away.
3. Close the socket.

What the tests can see of this: after an error response that was sent
while up to 30000 bytes from the client were still unread, the client
receives the complete response and then a normal end of the stream, not
a reset. And a client that stays connected after such a response, without
sending and without closing, holds its worker for 1 second and no longer.

When the server closes a connection without having sent a response (idle
timeout, shutdown, a client that went away), it simply closes it. The 503
of section 10.3 is also sent without these three steps, because the
thread that sends it must not wait.

### 7.5 Several requests at once

A client may send the next request without waiting for the response to
the one before (this is called pipelining), and one `read` may return the
end of one request together with the start of the next. The server
answers the requests of a connection strictly in the order in which they
arrived. Bytes that follow the end of a request belong to the next
request; they MUST NOT be lost.

In the other direction, a request may arrive in any number of pieces.
One `read` may return a single byte, and the `CRLF CRLF` at the end of
the head may be split anywhere.

---

## 8. Static files

This section applies to `GET` and `HEAD` requests whose path is not
`/echo` (step 5).

### 8.1 From the path to names

1. Take the path (the target up to the first `?`).
2. Split it at every `/`. Throw away the empty pieces. The pieces that
   remain are the **segments**. (So `/a//b/`, `//a/b` and `/a/b` have the
   same segments, `a` and `b`. The path `/` has none.)
3. Decode each segment: every `%` followed by two hexadecimal digits
   (upper or lower case) becomes the byte with that value. A `%` that is
   not followed by two hexadecimal digits is **400**. The decoded bytes
   MUST be valid UTF-8; if they are not: **400**.

`+` is an ordinary character in a path. `#` has no special meaning.

The order of 2 and 3 matters: split first, decode afterwards. `%2F` is an
encoded `/`. It MUST NOT become a separator.

### 8.2 Names that are never served

A decoded segment is **forbidden** if

- it starts with `.` (this covers `.`, `..`, and hidden files and
  directories such as `.git` or `.env`), or
- it contains `/`, `\`, `:` or the character U+0000.

A request with a forbidden segment is answered with **403**.

The reasons: `..` leads out of a directory. `\` separates path components
on Windows, so `..\..\file` is a path there. `:` names a drive, or
another data stream of a file, on Windows. U+0000 ends a string in the C
interfaces of every system. The server forbids all of them on every
system, so that it behaves the same everywhere.

The check is about text. It is made before the file system is asked
anything, and `..` is never interpreted: `/docs/../hello.txt` is 403
although a file system would resolve it to a path inside the root.

Segments are handled from left to right, and the first segment with a
fault decides the status: `/../%zz` is 403, `/%zz/..` is 400.

With these rules, no request can name a file outside the root directory.
Every test site has a file next to the root, outside it; no response may
ever contain its content.

### 8.3 Finding the file

Join the root directory and the segments, in order, to a file-system
path. Then:

1. If the path names a directory, add `index.html` to it.
2. If the path now names a regular file, the response is **200** with the
   bytes of the file as body.
3. Otherwise the response is **404**. That includes a directory without
   `index.html`, a name that the file system rejects, and a path that
   goes "through" a file (`/hello.txt/more`).
4. If the system refuses access to the file (permission denied), the
   response is **403**.

Consequences of 8.1 and these rules: `/docs` and `/docs/` give the same
response, and so do `/hello.txt` and `/hello.txt/`.

The file is looked up and read when the request is handled. A file that
changes, or appears, while the server runs is served in its new form.
`Content-Length` is the size of the file at the moment it is opened.

The server MUST be able to send a file that is much larger than the
memory it wants to use: read and send it piece by piece.

### 8.4 Content types

The `Content-Type` of a file follows from the extension of the file that
is sent (for a directory, that is `index.html`). The extension is the
text after the last `.` of the file name and is compared without regard
to ASCII case.

| extension | `Content-Type` |
|---|---|
| `html`, `htm` | `text/html; charset=utf-8` |
| `css` | `text/css; charset=utf-8` |
| `js` | `text/javascript; charset=utf-8` |
| `txt` | `text/plain; charset=utf-8` |
| `json` | `application/json` |
| `svg` | `image/svg+xml` |
| `png` | `image/png` |
| `jpg`, `jpeg` | `image/jpeg` |
| `gif` | `image/gif` |
| `ico` | `image/x-icon` |
| `wasm` | `application/wasm` |
| `pdf` | `application/pdf` |
| anything else, or no extension | `application/octet-stream` |

---

## 9. `POST /echo`

### 9.1 The request

The request MUST have a `Content-Type` header whose media type is
`application/json`. The media type is the value up to the first `;`,
without spaces and tabs around it, compared without regard to ASCII case.
So `application/json; charset=utf-8` and `Application/JSON` are accepted;
`text/plain`, `application/jsonx` and a missing header are not. If the
type is wrong, the response is **415**, whatever the body contains.

### 9.2 The body

The body MUST be exactly one JSON value as defined by RFC 8259, encoded
in UTF-8, with optional whitespace before and after it. If it is not, the
response is **400**.

```text
document = ws value ws
value    = object / array / string / number / "true" / "false" / "null"
object   = "{" ws [ member *( ws "," ws member ) ws ] "}"
member   = string ws ":" ws value
array    = "[" ws [ value *( ws "," ws value ) ws ] "]"

string   = %x22 *char %x22
char     = unescaped / "\" escape
unescaped = any Unicode character except %x00-1F, %x22 (") and %x5C (\)
escape   = %x22 / "\" / "/" / "b" / "f" / "n" / "r" / "t" / "u" 4HEXDIG

number   = [ "-" ] int [ "." 1*DIGIT ] [ ( "e" / "E" ) [ "+" / "-" ] 1*DIGIT ]
int      = "0" / ( %x31-39 *DIGIT )

ws       = *( %x20 / %x09 / %x0A / %x0D )
HEXDIG   = DIGIT / %x41-46 / %x61-66
```

Notes:

- Whitespace is exactly these four characters: space, tab, line feed,
  carriage return.
- A body that is empty or has only whitespace is not a value: 400.
- A byte order mark at the start is not whitespace: 400.
- `01`, `1.`, `.5`, `+1`, `NaN`, `'text'`, comments and a comma before
  `]` or `}` are not JSON: 400.
- A top-level value may be of any kind: `42` and `"text"` are documents.
- The server checks the form and nothing more. Repeated member names are
  allowed. Numbers of any size are allowed. A `\u` escape needs four
  hexadecimal digits; whether two such escapes form a valid surrogate
  pair is not checked.

**Depth.** An array or object that is not inside another one is at depth
1; an array or object directly inside one at depth *n* is at depth
*n* + 1. A document with an array or object at depth 65 or more is
**400**. Strings, numbers and literals do not add depth, and neither do
values that are next to each other.

The limit exists because the depth is chosen by the client. A body of a
million `[` MUST produce a 400, like any other invalid document. It MUST
NOT crash the server or a worker.

### 9.3 The response

The response is **200** with `Content-Type: application/json`. The body
is the same JSON value in **compact form**: every token of the request
body is copied byte for byte, in the same order, and the whitespace
between tokens is left out.

The tokens are `{`, `}`, `[`, `]`, `:`, `,`, strings (with their
quotation marks), numbers, `true`, `false` and `null`.

"Byte for byte" means that nothing is converted. `2.50` stays `2.50`,
`1E+2` stays `1E+2`, `"\u00e9"` stays `"\u00e9"`, and whitespace inside a
string stays where it is.

| request body | response body |
|---|---|
| `{ "a" : [ 1, 2.50 ], "b":null}` | `{"a":[1,2.50],"b":null}` |
| ` [ ] ` | `[]` |
| `"  two  spaces  "` | `"  two  spaces  "` |
| `{ "b" : 1 , "a" : 2 , "b" : 3 }` | `{"b":1,"a":2,"b":3}` |

---

## 10. The worker pool

### 10.1 Workers

The server starts `--workers` worker threads at start-up and never starts
more. A worker takes one connection, serves it as described in sections 4
to 9 until the connection is closed, and then takes the next one.

A worker belongs to its connection for the whole life of the connection,
also while the connection is idle between two requests. With
`--workers 2`, two connections are served at the same time and a third
one waits, even if the first two are doing nothing.

Requests are handled by workers only. The thread that accepts connections
MUST NOT handle requests, and the number of threads MUST NOT grow with
the number of connections.

### 10.2 Waiting, and counting

When a connection is accepted and no worker is free, it waits. Waiting
connections are served in the order in which they were accepted. The
server reads nothing from a waiting connection.

The server **holds** a connection from the moment it has accepted it
until a worker is done with it, or until it is closed for another reason
(10.3, 11.2). The server holds at most `workers + queue` connections at
any time.

A worker MUST come back for the next connection whatever happened to the
last one: the client closed it in the middle of a request, a read or a
write failed, a time limit ended, or your own code panicked. A connection
that has ended no longer counts as held.

### 10.3 When the server is full

If a connection is accepted while the server already holds
`workers + queue` connections, the server:

1. sends, without waiting for a request and without looking at one, a
   **503** response with the body of 6.4, the headers of 6.3,
   `Retry-After: 1` and `Connection: close`;
2. closes the connection.

This happens at once, on the accepting thread. That thread MUST NOT wait
for this client: not for a request, and not for the client to read the
response. (It MAY throw away bytes that have already arrived, as long as
it does not wait for more.)

```text
< HTTP/1.1 503 Service Unavailable\r\n
< Date: Fri, 20 Nov 2026 11:01:46 GMT\r\n
< Content-Type: text/plain; charset=utf-8\r\n
< Content-Length: 24\r\n
< Retry-After: 1\r\n
< Connection: close\r\n
< \r\n
< 503 Service Unavailable
```

This is what back-pressure means here: the amount of work the server
accepts has a fixed upper bound, and a client beyond the bound learns it
immediately instead of waiting in a line of unknown length.

---

## 11. Shutdown

### 11.1 How it starts

The server reads its standard input, line by line, for as long as it
runs. Shutdown begins when

- a line is exactly `shutdown` (the line may end with `LF` or `CRLF`), or
- standard input ends.

Every other line is ignored, including ` shutdown`, `SHUTDOWN` and
`shutdown now`.

Two consequences are worth knowing. If you start the server with standard
input closed or redirected from an empty source, it shuts down at once.
And if the process that started the server dies, the server sees the end
of its input and stops too; it cannot be left behind as an orphan.

### 11.2 What happens

1. The server stops accepting: it closes the listening socket.
   Connections that have not been accepted yet are never served.
2. Then it prints the line `shutting down` to standard output and flushes
   it.
3. Connections that wait for a worker (10.2) are closed without a
   response.
4. Connections on which a worker waits for a request (step 1) are closed
   without a response. This MUST happen promptly, within about a second.
   It MUST NOT take until the idle timeout.
5. A request **in progress** is finished. A request is in progress when a
   worker has read at least its first byte. It is handled exactly as
   usual: every step, every limit, and 408 if it does not arrive in time.
   Its response says `Connection: close` (7.1), and the connection is
   closed as described in 7.4.

Shutdown may refuse work. It MUST NOT damage work: a client either gets a
complete, correct response or none at all.

### 11.3 How it ends

When no connection is left, the process exits with status 0. It prints
nothing to standard output after `shutting down`.

No request can delay the exit for longer than the time limits of 7.3 and
7.4 allow, because every wait in the server has a limit.

---

## 12. Non-goals

You do not have to build any of this. The tests do not check it.

- TLS, HTTP/2, HTTP/3.
- `Transfer-Encoding`, in requests or responses (a request that uses it
  gets 501).
- `Expect: 100-continue`. The header is ignored. (`curl` sends it with
  large bodies, waits one second for an answer, and then sends the body
  anyway.)
- Range requests, conditional requests (`If-Modified-Since`, `ETag`),
  compression, caching headers.
- Directory listings. A directory without `index.html` is 404.
- Redirects. A directory path without `/` at the end is served directly
  (8.3). A real server redirects, so that relative links in the page
  work.
- Symbolic links. The server follows them like any program does. Whoever
  controls the root directory is trusted. The checks of 8.2 are about
  what a *request* can name.
- Virtual hosts. The value of `Host` is not used.
- A log format. Write what you find useful to standard error.
- Signals. The standard library cannot handle them, so Ctrl-C ends the
  process at once, without a graceful shutdown. Use standard input.
- Configuration files, running as a background service, reloading.

This server is stricter than the HTTP standard (RFC 9110 and RFC 9112) in
a few places. Know them, so that you do not take them for general rules:
the standard lets a server skip an empty line before the request line,
lets it accept a bare `LF` as a line end, and lets it leave out
`Content-Length` in the answer to `HEAD`. Most servers also resolve `..`
in a target and refuse the request only if the result leaves the root.
Strict rules are easier to state exactly, to test and to make safe.

---

## 13. Milestones

Each milestone has one test file. Work through them in order. A test file
is "green" when `cargo test --test <name>` reports no failure.

Until M6, no test needs two connections to be served at the same moment.
A server that accepts a connection, serves it to the end, and only then
accepts the next one can pass M1 to M5. Build the protocol first and the
threads last.

### M1 · A process that answers (`tests/m1_startup.rs`)

Sections 3, 6.1, 6.2, 6.3 (without `Date`), 6.4, and the simplest part of
8: one file, one missing file, more than one request per connection.

| test | what it checks |
|---|---|
| `help_prints_usage_and_exits_with_0` | 3.1, 3.4 |
| `usage_errors_exit_with_2` | 3.1, 3.4 |
| `a_root_that_is_not_a_directory_exits_with_1` | 3.2, 3.4 |
| `an_address_in_use_exits_with_1` | 3.2, 3.4 |
| `announces_a_real_port_and_accepts_connections` | 3.3 |
| `the_connection_is_usable_right_after_the_announcement` | 3.3 |
| `serves_a_small_file` | 6.1 to 6.3, 8.3, 8.4 for `.txt` |
| `a_missing_file_is_404_with_the_standard_body` | 6.4 |
| `serves_one_connection_after_another` | `Connection: close` in the request |
| `several_requests_on_one_connection` | the connection stays open |

### M2 · The request head (`tests/m2_parser.rs`)

Section 5, the status codes of steps 2 and 5 that come from the head,
6.3 (`Date`), 6.5 (`HEAD`), 7.5 (requests in pieces).

| test | what it checks |
|---|---|
| `malformed_request_lines_are_400` | 5.2 |
| `a_well_formed_but_unsupported_version_is_505` | 5.2 |
| `unknown_methods_are_501_and_the_connection_stays_open` | 5.2, step 5, 7.2 |
| `the_wrong_method_for_a_resource_is_405_with_allow` | step 5 |
| `http_1_1_needs_exactly_one_host_header` | 5.3 |
| `malformed_header_lines_are_400` | 5.3 |
| `unusual_but_valid_header_lines_are_accepted` | 5.3 |
| `header_names_ignore_case_and_values_are_trimmed` | 5.3 |
| `a_bare_line_feed_is_refused_at_once` | 5.1 |
| `a_request_may_arrive_one_byte_at_a_time` | 7.5 |
| `a_request_may_be_split_anywhere` | 7.5 |
| `head_has_the_headers_of_get_and_no_body` | 6.5 |
| `every_response_has_a_current_date` | 6.3 |

### M3 · Files and path safety (`tests/m3_files.rs`)

Section 8.

| test | what it checks |
|---|---|
| `content_type_follows_the_extension` | 8.4 |
| `a_directory_is_answered_with_its_index_html` | 8.3 |
| `nested_paths_empty_segments_and_trailing_slashes` | 8.1, 8.3 |
| `the_query_string_is_not_part_of_the_path` | step 5, 8.1 |
| `percent_escapes_are_decoded` | 8.1 |
| `broken_escapes_and_invalid_utf8_are_400` | 8.1 |
| `dot_segments_and_hidden_names_are_403` | 8.2 |
| `an_escape_cannot_smuggle_a_separator_or_a_drive` | 8.1, 8.2 |
| `the_first_bad_segment_decides_between_400_and_403` | 8.2 |
| `nothing_outside_the_root_is_ever_served` | 8.2 |
| `files_arrive_byte_for_byte` | 8.3: binary data, a 3 MiB file, an empty file |
| `files_are_read_when_they_are_requested` | 8.3 |
| `a_file_the_system_refuses_to_open_is_403` | 8.3 (4); this test exists on Unix only |

### M4 · Bodies, echo and persistent connections (`tests/m4_bodies.rs`)

Steps 3 and 4, sections 7.1, 7.2, 7.5 and 9, and the second half of 6.5.

| test | what it checks |
|---|---|
| `echo_returns_the_value_without_insignificant_whitespace` | 9.3 |
| `invalid_json_is_400_and_the_connection_stays_open` | 9.2, 7.2 |
| `a_body_that_is_not_utf8_is_400` | 9.2 |
| `nesting_is_limited_to_64_levels` | 9.2 |
| `echo_needs_the_json_content_type` | 9.1, 5.3 (the first `Content-Type` line counts) |
| `the_query_string_does_not_change_the_echo_endpoint` | step 5 |
| `a_body_may_arrive_in_pieces` | step 4, 7.5 |
| `a_body_of_exactly_the_default_limit_is_echoed` | 5.4 with the default `--max-body` |
| `post_without_content_length_is_411` | step 3 |
| `content_length_must_be_one_plain_number` | step 3 |
| `transfer_encoding_is_501` | step 3, and its order |
| `many_requests_on_one_connection` | 7.1 |
| `connection_close_is_honoured` | 7.1 |
| `http_1_0_closes_unless_asked_to_keep_alive` | 7.1 |
| `pipelined_requests_are_answered_in_order` | 7.5 |
| `a_body_on_get_is_read_and_thrown_away` | step 4 |
| `errors_after_a_complete_request_keep_the_connection` | 7.2 |
| `two_hundred_requests_take_far_less_than_four_seconds` | 6.5 |

### M5 · Limits and time (`tests/m5_limits.rs`)

Sections 5.4, 7.3 and 7.4.

| test | what it checks |
|---|---|
| `a_head_may_be_8192_bytes_and_not_one_more` | 5.4 |
| `a_head_without_end_is_431_as_soon_as_it_is_too_long` | 5.4 |
| `a_request_may_have_64_header_lines` | 5.4 |
| `a_target_may_be_2048_bytes` | 5.4 |
| `a_body_may_be_max_body_bytes_and_not_one_more` | step 3, 5.4 |
| `max_body_zero_allows_only_empty_bodies` | step 3 |
| `an_error_response_can_be_read_although_more_bytes_were_sent` | 7.4 |
| `a_client_that_never_closes_cannot_keep_a_worker_after_an_error` | 7.4 (the limit of step 2) |
| `a_request_that_stalls_in_the_head_gets_408` | 7.3 |
| `a_request_that_stalls_in_the_body_gets_408` | 7.3 |
| `the_request_timeout_covers_the_whole_request` | 7.3 |
| `an_idle_connection_is_closed_without_a_response` | 7.3 |
| `the_idle_timeout_does_not_limit_a_request_in_progress` | 7.3 |
| `a_client_that_does_not_read_cannot_keep_a_worker` | 7.3 (writing) |
| `a_slow_but_steady_reader_gets_the_whole_file` | 7.3 (writing) |

These tests take a few seconds, because they wait for time limits to end.
Two of them create a file of 32 MiB in the temporary directory and remove
it afterwards.

### M6 · The pool, back-pressure and shutdown (`tests/m6_pool.rs`)

Sections 10 and 11. This is where the server becomes concurrent.

| test | what it checks |
|---|---|
| `workers_serve_connections_at_the_same_time` | 10.1 |
| `a_slow_client_does_not_delay_the_others` | 10.1 |
| `connections_beyond_the_workers_wait_in_the_queue` | 10.1, 10.2 |
| `waiting_connections_are_served_in_the_order_they_arrived` | 10.2 |
| `time_spent_waiting_for_a_worker_is_not_limited` | 7.3 (the last paragraph), 10.2 |
| `a_full_server_answers_503_at_once` | 10.3 |
| `workers_return_to_the_pool_when_clients_vanish` | 10.2 |
| `a_long_series_of_short_connections_is_served` | 10.2 |
| `parallel_clients_all_get_their_own_answers` | 10.1, 10.2 |
| `the_line_shutdown_stops_the_server` | 11.1, 11.3 |
| `end_of_input_stops_the_server` | 11.1 |
| `other_input_lines_are_ignored` | 11.1 |
| `shutdown_closes_idle_connections_at_once` | 11.2 (4) |
| `shutdown_lets_a_request_in_progress_finish` | 11.2 (1), (2), (5) |
| `shutdown_drops_waiting_connections_without_a_response` | 11.2 (3) |
| `a_stalled_request_cannot_delay_shutdown_beyond_its_timeout` | 11.2 (5), 11.3 |
| `shutdown_under_load_loses_no_accepted_request` | 11.2 |

How the tests know what your server is doing is explained at the top of
`tests/m6_pool.rs`. Read it before you start M6.

---

## 14. Stretch goals

None of these has tests. Each is a real feature of real servers. Write
your own tests first.

1. **`Expect: 100-continue`.** When a request with this header passes
   step 3, send `HTTP/1.1 100 Continue\r\n\r\n` before you read the body.
   `curl` then no longer waits a second before it uploads.
2. **Conditional requests.** Send `Last-Modified`, understand
   `If-Modified-Since`, answer 304 without a body. You have the date
   formatter already; now you need a date parser.
3. **Range requests** for one range: `Range: bytes=100-199`, status 206,
   `Content-Range`, and 416 for a range that cannot be satisfied.
4. **Redirect directories.** Answer `/docs` with 301 and
   `Location: /docs/`. Build the location from the checked segments, not
   from the raw target: a server that copies `//example.org` from the
   target into `Location` sends browsers to another site.
5. **A symbolic-link policy.** Refuse a file whose real location is
   outside the root, by comparing canonical paths. Think about what can
   change between the check and the `open`.
6. **An access log** on standard error, one line per request, written so
   that lines from different workers never mix.
7. **Chunked request bodies.** Then you can remove the 501 of step 3.
   Read about "request smuggling" before you start.
8. **A status endpoint**, for example `GET /-/status`, that reports the
   number of busy workers, waiting connections and refused connections
   as JSON.
9. **A drain limit.** Add `--shutdown-grace-ms`: when it ends, close the
   remaining connections and exit with a status that says "not clean".
10. **Signals.** Handle Ctrl-C like the line `shutdown`. This needs a
    foreign function interface and `unsafe`; the mastery module [`m6_ffi`](../../mastery/m6_ffi/README.md)
    shows how. Keep the unsafe code in one small module.
11. **Measure it.** Write a small load generator with threads, find the
    request rate at which the queue starts to fill, and explain the
    number.
