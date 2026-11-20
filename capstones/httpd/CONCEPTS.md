# Concepts for the `httpd` capstone

Read this after [SPEC.md](SPEC.md) and before you design. Come back to a
section when you reach the milestone that needs it.

**What you will be able to do afterwards.** Cut a stream of bytes into
messages without losing or inventing a byte; parse bytes from a stranger
without treating them as text; put every limit at the place where it still
saves something; turn a name from a request into a file name without
letting it leave a directory; check nested input without a tree and without
recursion; give a whole operation one time limit; end a connection so that
your last message arrives; hand work to a fixed number of threads and say
"no" when there are too many; and stop threads that are blocked in the
operating system.

**What is assumed.** Chapters 03 to 19 of the course, in particular
Chapter 17 (bytes and text), Chapter 18 (`Read`, `Write`, files and
sockets) and Chapter 19 (threads, `Mutex`, `Condvar`, channels, atomics).
No knowledge of HTTP or of networking beyond Chapter 18 is assumed.

**Time (estimates).** About 15 to 25 minutes per section, and about three
hours for the whole file if you type in the examples and change them. You
should: every example is small enough to break on purpose.

| section | read it before |
|---|---|
| 1. A stream has no messages | M2 |
| 2. Bytes from a stranger are not text | M2 |
| 3. A calendar from a count of seconds | M2 |
| 4. Names from a stranger | M3 |
| 5. Checking nested input without a tree | M4 |
| 6. A limit must come before the cost | M4, M5 |
| 7. Deadlines, not timeouts | M5 |
| 8. Ending a connection | M5 |
| 9. A bounded hand-off between threads | M6 |
| 10. Threads that are blocked | M6 |

The examples are deliberately not the things you build. They show an idea
in a smaller or neighbouring setting: a different message format, a
different grammar, a different kind of worker. Carrying the idea over to
the server is your work. Every example in this file is compiled and run by
`rustdoc --test`.

---

## 1. A stream has no messages

TCP gives two programs a pair of byte streams, one in each direction. It
delivers the bytes in order and without gaps. It does not remember how the
sender grouped them. If the sender calls `write` three times, the receiver
may get everything in one `read`, or in ten. A single `read` may end in the
middle of one message, or return the end of one message together with the
start of the next.

So a protocol must say where a message ends, and the receiver must find
that place itself. This is called **framing**. There are two common ways:

- **A delimiter.** The message ends with a byte sequence that cannot occur
  inside it. A line that ends with a line feed is framed like this.
- **A length.** The message starts with a number, and exactly that many
  bytes follow.

Many protocols use both: a first part that ends with a delimiter and
contains the length of the second part. The request format in SPEC
sections 4 and 5 is of this kind.

The receiver needs one thing to do this correctly: **a buffer that lives
as long as the connection**. Bytes that arrive are appended at the end.
When the buffer starts with a complete message, exactly that message is
removed from the front. What is left is the start of the next message and
stays where it is.

```text
buffer after read 1:   5:hel
                       ^^^^^ not a complete message yet: read again

buffer after read 2:   5:hello3:ab
                       ^^^^^^^ a message; remove exactly these 7 bytes

buffer now:            3:ab
                       ^^^^ the start of the next message; keep it
```

The example uses a format that is not HTTP: a decimal length, a colon, and
then that many bytes, for example `5:hello`. The type has no socket in it.
It is given bytes and asked for messages, so it can be tested with plain
values.

```rust
/// Cuts a stream into messages of the form `<length>:<payload>`.
struct Inbox {
    buffer: Vec<u8>,
}

#[derive(Debug, PartialEq)]
enum Step {
    /// One complete message. Ask again: there may be another.
    Message(Vec<u8>),
    /// The buffer holds only the start of a message. Read more bytes.
    NeedMore,
    /// These bytes can never become a message. Give up on the stream.
    Broken(&'static str),
}

impl Inbox {
    const MAX_DIGITS: usize = 3;

    fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    fn next(&mut self) -> Step {
        // The first part ends with a delimiter. Look for it only as far
        // as it is allowed to be.
        let start = &self.buffer[..self.buffer.len().min(Self::MAX_DIGITS + 1)];
        let Some(colon) = start.iter().position(|&b| b == b':') else {
            return if start.len() > Self::MAX_DIGITS {
                Step::Broken("the length has too many digits")
            } else {
                Step::NeedMore
            };
        };
        let digits = &self.buffer[..colon];
        if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
            return Step::Broken("the length is not a number");
        }
        // At most three digits: this cannot overflow.
        let length = digits
            .iter()
            .fold(0usize, |n, digit| n * 10 + usize::from(digit - b'0'));

        // The second part has a known length. Is all of it here?
        let end = colon + 1 + length;
        if self.buffer.len() < end {
            return Step::NeedMore;
        }
        let payload = self.buffer[colon + 1..end].to_vec();
        // Remove this message and nothing else.
        self.buffer.drain(..end);
        Step::Message(payload)
    }
}

fn main() {
    let mut inbox = Inbox { buffer: Vec::new() };
    let mut messages = Vec::new();
    // Three messages, cut into pieces at inconvenient places.
    for piece in [&b"5:hel"[..], b"lo3:ab", b"c0", b":", b"2:o"] {
        inbox.push(piece);
        while let Step::Message(payload) = inbox.next() {
            messages.push(payload);
        }
    }
    assert_eq!(messages, [&b"hello"[..], b"abc", b""]);
    // The last piece started a fourth message that is not complete.
    assert_eq!(inbox.next(), Step::NeedMore);
    assert_eq!(inbox.buffer, b"2:o");

    let mut inbox = Inbox { buffer: b"12345".to_vec() };
    assert_eq!(inbox.next(), Step::Broken("the length has too many digits"));
}
```

Three details are worth a second look.

`next` is called in a loop after every `push`. One piece of input can
complete two messages (`lo3:ab` completes one and starts another), so "one
read, one message" is wrong in both directions.

`NeedMore` and `Broken` are different answers. The first means "wait"; the
second means "waiting will not help". The inbox can tell them apart only
because the format has limits: after four bytes without a colon, no fifth
byte can repair the message.

The search for the colon starts at the beginning on every call. Here that
costs at most four comparisons. If the delimiter may be thousands of bytes
away and the bytes arrive one at a time, starting again each time adds up
to millions of comparisons for a single message. A scanner for such a
format remembers how far it has looked. When the delimiter is longer than
one byte, it must also remember that the delimiter can be cut in two by
the end of a read.

**Mistakes people make.**

- Calling `read` once and parsing what it returned as one message. This
  works on a quiet machine with a local client and fails in production.
- Emptying the buffer after a message (`clear`) and not removing exactly
  the bytes of that message. The start of the next message is lost, and
  the client waits for an answer that never comes.
- Reading the first part through a `BufReader` and the second part from
  the socket directly. The `BufReader` has already taken more bytes out of
  the socket than the first part needed; those bytes are in its buffer,
  and the direct read never sees them.
- Deciding "broken" too late, or never. Section 6 is about that.

**Read more.**
[`Read::read`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read)
(read the paragraph about how many bytes it may return),
[`Vec::drain`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.drain),
[`slice::windows`](https://doc.rust-lang.org/std/primitive.slice.html#method.windows).

---

## 2. Bytes from a stranger are not text

A `String` and a `&str` promise valid UTF-8. A network protocol makes no
such promise unless its specification says so, and old text protocols
usually allow any byte above 127 in some places. If you convert the input
to a `String` first, you have two bad choices: fail on input that the
protocol allows, or use `String::from_utf8_lossy`, which silently replaces
bytes and changes lengths.

```rust
let received = b"caf\xe9"; // "café" in Latin-1: four bytes
let text = String::from_utf8_lossy(received);
assert_eq!(text, "caf\u{fffd}");
assert_eq!(text.len(), 6, "one byte became a three-byte replacement character");
```

After that conversion, a position in the text is no longer a position in
the input, and a length that you computed from the text is wrong for the
bytes.

The better rule: **parse `&[u8]`, and make text only from the parts that
the grammar has already proved to be ASCII.** Slices of bytes have most of
the tools that `str` has.

| you want | on `&[u8]` |
|---|---|
| cut at a byte, keeping empty pieces | `split(\|&b\| b == b' ')` |
| cut once | `splitn(2, ...)`, or `iter().position(...)` and two slices |
| test or remove the start or the end | `starts_with`, `strip_prefix`, `strip_suffix` |
| look at the first or last bytes | slice patterns: `[b'#', rest @ ..]` |
| compare without regard to ASCII case | `eq_ignore_ascii_case` |
| classify one byte | `u8::is_ascii_digit`, `is_ascii_alphabetic`, `is_ascii_graphic` |
| make text of bytes you have checked | `std::str::from_utf8` (it cannot fail for ASCII) |

The example parses lines of an invented protocol for a robot: a verb in
capital letters, exactly one space, and an argument that may be any bytes.

```rust
#[derive(Debug, PartialEq)]
enum Command<'a> {
    Move(u32),
    Say(&'a [u8]),
}

fn parse(line: &[u8]) -> Result<Command<'_>, &'static str> {
    // Cut at the first space only. A second space stays at the start of
    // the argument, where the check for that argument finds it.
    let mut parts = line.splitn(2, |&b| b == b' ');
    let (Some(verb), Some(argument)) = (parts.next(), parts.next()) else {
        return Err("expected `VERB argument`");
    };
    match verb {
        b"MOVE" => {
            // Digits only. `str::parse` would also accept `+3`.
            if argument.is_empty() || !argument.iter().all(u8::is_ascii_digit) {
                return Err("MOVE needs a number");
            }
            // The digits come from outside: use arithmetic that cannot wrap.
            argument
                .iter()
                .try_fold(0u32, |n, digit| {
                    n.checked_mul(10)?.checked_add(u32::from(digit - b'0'))
                })
                .map(Command::Move)
                .ok_or("the number is too large")
        }
        b"SAY" => Ok(Command::Say(argument)),
        _ => Err("unknown verb"),
    }
}

fn main() {
    assert_eq!(parse(b"MOVE 30"), Ok(Command::Move(30)));
    assert_eq!(parse(b"SAY caf\xe9 !"), Ok(Command::Say(b"caf\xe9 !")));
    assert_eq!(parse(b"MOVE  30"), Err("MOVE needs a number"));
    assert_eq!(parse(b"MOVE +3"), Err("MOVE needs a number"));
    assert_eq!(parse(b"MOVE 99999999999"), Err("the number is too large"));
    assert_eq!(parse(b"move 3"), Err("unknown verb"));
    assert_eq!(parse(b"STOP"), Err("expected `VERB argument`"));
}
```

Some standard functions are more generous than a strict grammar. Know
what they accept before you use them on input from outside:

```rust
// Integer parsing accepts a plus sign and leading zeros.
assert_eq!("+7".parse::<u32>(), Ok(7));
assert_eq!("007".parse::<u32>(), Ok(7));

// `split_whitespace` hides repeated separators; `split` shows them.
let line = "MOVE  north";
assert_eq!(line.split_whitespace().count(), 2);
assert_eq!(line.split(' ').collect::<Vec<_>>(), ["MOVE", "", "north"]);

// "ASCII whitespace" is more than space and tab: it includes line feed,
// carriage return and form feed (byte 12).
assert_eq!(b" \x0c value\t".trim_ascii(), b"value");
```

None of these is a bug. Each is the wrong tool when a specification says
"digits only", "exactly one space", or "spaces and tabs".

**A mistake the compiler catches.** Byte slices and strings do not compare
with each other:

```rust,compile_fail,E0277
fn main() {
    let name: &[u8] = b"Host";
    if name == "Host" {
        println!("found it");
    }
}
```

```text
error[E0277]: can't compare `[u8]` with `str`
   --> src/main.rs:3:13
    |
  3 |     if name == "Host" {
    |             ^^ no implementation for `[u8] == str`
    |
    = help: the trait `PartialEq<str>` is not implemented for `[u8]`
```

Write the other side as bytes: `name == b"Host"`, or
`name.eq_ignore_ascii_case(b"host")` when case does not matter.

**Mistakes people make.**

- `from_utf8_lossy` "to be safe". It is the opposite of safe: the program
  now works on data that the client did not send.
- `to_lowercase` for a comparison that the specification defines for ASCII
  only. It allocates, and it changes characters outside ASCII.
- Indexing with a position that came out of a search on a different view
  of the data (a `String` made from the bytes, a trimmed copy).
- `parse::<u64>()` on a field that must be digits only.

**Read more.**
[slice methods](https://doc.rust-lang.org/std/primitive.slice.html)
(`split`, `splitn`, `strip_prefix`, `trim_ascii`, `eq_ignore_ascii_case`),
[slice patterns](https://doc.rust-lang.org/reference/patterns.html#slice-patterns),
[`str::from_utf8`](https://doc.rust-lang.org/std/str/fn.from_utf8.html),
[`u32::checked_mul`](https://doc.rust-lang.org/std/primitive.u32.html#method.checked_mul).

---

## 3. A calendar from a count of seconds

The standard library has clocks and no calendar. `SystemTime::now()`
minus `UNIX_EPOCH` gives the number of seconds since 1970-01-01 00:00:00
UTC. Everything else is arithmetic, and it comes apart into three
independent questions:

```text
seconds since 1970 ──┬── % 86 400 ──> time of day: hours, minutes, seconds
                     │
                     └── / 86 400 ──> days since 1970-01-01
                                        ├── % 7 ──> day of the week
                                        └── calendar ──> year, month, day
```

A day has 86 400 seconds. (UTC sometimes inserts a leap second; Unix time
pretends that it does not, so this division is always right.) Day 0 was a
Thursday, and the days of the week repeat every 7 days whatever the
calendar does. Only the third question needs the rules for leap years: a
year is a leap year if it can be divided by 4, except that a year that can
be divided by 100 is one only if it can also be divided by 400. So 2000
was a leap year, and 2100 will not be one.

The example answers the first two questions and half of the third: it
finds the year and the number of the day in that year. It does so in the
slow and obvious way, one year at a time.

```rust
const DAY_NAMES: [&str; 7] = [
    "Thursday", "Friday", "Saturday", "Sunday", "Monday", "Tuesday", "Wednesday",
];

fn is_leap(year: u64) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

/// The year, and the day in that year (0 for 1 January).
fn year_and_day(mut days: u64) -> (u64, u64) {
    let mut year = 1970;
    loop {
        let length = if is_leap(year) { 366 } else { 365 };
        if days < length {
            return (year, days);
        }
        days -= length;
        year += 1;
    }
}

fn main() {
    let seconds: u64 = 951_827_696; // 2000-02-29 12:34:56 UTC
    let (days, in_day) = (seconds / 86_400, seconds % 86_400);

    assert_eq!((in_day / 3_600, in_day / 60 % 60, in_day % 60), (12, 34, 56));
    assert_eq!(DAY_NAMES[(days % 7) as usize], "Tuesday");
    // January has 31 days, so 29 February is day 31 + 28 = 59, counted from 0.
    assert_eq!(year_and_day(days), (2000, 59));

    assert_eq!(DAY_NAMES[0], "Thursday", "1970-01-01");
    assert_eq!(year_and_day(0), (1970, 0));
    assert_eq!(year_and_day(365), (1971, 0));
    // 2100 is not a leap year: 1 March 2100 is day 59 as well, not day 60.
    assert_eq!(year_and_day(4_107_542_400 / 86_400), (2100, 59));
}
```

Going from the day in the year to a month and a day of the month is the
same idea once more, with the lengths of the months instead of the lengths
of the years.

**When to use which.** The loop takes one step per year. For a date in
this century that is about 60 steps, which you will not be able to
measure. A formula without a loop exists. If you want one, write the loop
first and keep it: a test that compares the two for every day of four
hundred years (146 097 days, a full cycle of the leap-year rules) proves
the formula in a few milliseconds.

**Mistakes people make.**

- "Every fourth year is a leap year." The program is then right until
  2100, which makes the mistake hard to find with dates from today.
- Taking the day of the week from the calendar date with a formula from
  memory. `days % 7` cannot be wrong.
- Testing only with the current date. Test the last second of a year, the
  29th of February, and the day after it.
- Using `SystemTime` to measure how long something took. The system clock
  can be set back. `Instant` is the clock for durations (section 7);
  `SystemTime` is the clock for "what time is it".

**Read more.**
[`SystemTime`](https://doc.rust-lang.org/std/time/struct.SystemTime.html),
[`UNIX_EPOCH`](https://doc.rust-lang.org/std/time/constant.UNIX_EPOCH.html),
[`Duration::as_secs`](https://doc.rust-lang.org/std/time/struct.Duration.html#method.as_secs).

---

## 4. Names from a stranger

A program that opens a file whose name came from outside has handed part
of its file system to whoever sent the name. The classic attack is a name
with `..` in it. There are many disguises, and all of them use the same
weakness: **two pieces of code disagree about what a name means.** A check
looks at one form of the name; the file system later sees another.

Two disagreements are worth knowing well.

### Escaped separators

Many formats let a character be written as an escape. In a URL, `%` and
two hexadecimal digits stand for one byte: `%41` is `A`, `%26` is `&`,
`%2F` is `/`. Escapes exist so that a character which normally separates
things can occur *inside* a thing. The rule that follows is simple and
easy to get backwards: **split at the separators first, decode each piece
afterwards.**

The example uses the fields of a query string, which are separated by `&`.
The decoder is a stand-in that knows exactly one escape; a real one is
part of your project.

```rust
/// A stand-in for a real decoder: it knows one escape.
fn decode(text: &str) -> String {
    text.replace("%26", "&")
}

let query = "title=Tom%26Jerry&lang=en";

// Right: split at the separators that are really there, then decode.
let right: Vec<String> = query.split('&').map(decode).collect();
assert_eq!(right, ["title=Tom&Jerry", "lang=en"]);

// Wrong: decode, then split. The escaped `&` has become a separator.
let decoded = decode(query);
let wrong: Vec<&str> = decoded.split('&').collect();
assert_eq!(wrong, ["title=Tom", "Jerry", "lang=en"]);
```

In a query string the wrong order produces a wrong field. In a path, the
separator is `/`, and the wrong order turns one harmless name into two
components, one of which can be `..`.

Two more rules belong to decoding. Decode exactly once: after one pass,
`%252F` is the text `%2F`, and a second pass would make it a `/`. And
decode to bytes first, text second: one character can be spread over
several escapes (`%C3%A9` is `é`), so only the complete sequence of bytes
can be checked for UTF-8. `String::from_utf8` does that check strictly; it
also refuses the "overlong" forms that old attacks used to hide a dot or a
slash.

### What `Path` does and does not do

`std::path` works with text. It does not ask the file system, and it does
not make a path safe.

```rust
use std::path::{Component, Path};

// `join` with an absolute path does not append. It replaces.
let joined = Path::new("/srv/site").join("/etc/passwd");
assert_eq!(joined, Path::new("/etc/passwd"));

// `join` keeps `..`. Nothing is resolved until the file system sees it.
let joined = Path::new("site").join("../secret.txt");
let parts: Vec<Component> = joined.components().collect();
assert_eq!(parts.len(), 3);
assert_eq!(parts[1], Component::ParentDir);

// Comparing paths as text is wrong. `Path::starts_with` compares whole
// components.
assert!("/srv/site-backup/x".starts_with("/srv/site"));
assert!(!Path::new("/srv/site-backup/x").starts_with("/srv/site"));

/// True if every component is an ordinary name: no root, no `..`, no `.`.
fn only_names(relative: &Path) -> bool {
    relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
}

assert!(only_names(Path::new("css/site.css")));
assert!(!only_names(Path::new("../secret.txt")));
assert!(!only_names(Path::new("/etc/passwd")));

// The same text is one name on one system and three components on another.
let parts = Path::new("docs\\..\\secret.txt").components().count();
if cfg!(windows) {
    assert_eq!(parts, 3);
} else {
    assert_eq!(parts, 1);
}
```

The last lines are the reason why SPEC 8.2 forbids `\` and `:` on every
system. A rule that is written in terms of "what the platform calls a
separator" gives different answers on different machines, and tests that
pass on one of them say nothing about the others.

**When to use which.** There are two families of defence.

| | a rule about the text | asking the file system |
|---|---|---|
| how | refuse every name that is not plainly an entry of a directory | join, `canonicalize`, check that the result is inside the canonical root |
| good | no system call; the same on every platform; easy to test with a table | also sees symbolic links that lead outside |
| bad | does not see symbolic links | needs the file to exist; the answer can be out of date when the file is opened; behaves differently per platform |

SPEC.md prescribes a rule of the first kind. Real servers often use both.

**Mistakes people make.**

- Removing `../` from the name and using what is left. `....//` becomes
  `../` when the inner `../` is taken out.
- Checking before decoding, or decoding twice.
- Checking that the joined path, as a string, starts with the root.
- Resolving `..` by hand ("go up one level, but not above the root"). It
  can be done correctly, and it is one more interpreter of names that can
  disagree with the file system. Refusing is simpler than resolving.

**Read more.**
[`Path::join`](https://doc.rust-lang.org/std/path/struct.Path.html#method.join),
[`Path::components`](https://doc.rust-lang.org/std/path/struct.Path.html#method.components),
[`Component`](https://doc.rust-lang.org/std/path/enum.Component.html),
[`fs::canonicalize`](https://doc.rust-lang.org/std/fs/fn.canonicalize.html),
[`String::from_utf8`](https://doc.rust-lang.org/std/string/struct.String.html#method.from_utf8).

---

## 5. Checking nested input without a tree

A parser usually builds a tree: an expression has two operands, each of
which may be an expression. A tree is the right result when the program
wants to *evaluate* or *change* the input. Sometimes the program only
wants to know "is this well formed?", or wants the same input back in a
tidier form. Then a tree is work for nothing. It is enough to walk over
the input once and to know, at each position, **what may come next**.

"What may come next" is a small set of situations, so it fits an `enum`.
The scanner is then one `match` on the pair (situation, next byte): each
allowed pair moves on, and everything else is an error.

Nesting needs one more piece of memory: what is currently open. For a
single kind of bracket, a counter is enough. As soon as there are two
kinds that close differently, the scanner must remember *which* ones are
open, in order. That is a stack, and a `Vec` is a stack.

The example checks arithmetic such as `( 1 + 20 ) * 3` and returns it
without the spaces. It has two situations and one kind of bracket.

```rust
#[derive(Clone, Copy, PartialEq)]
enum Expect {
    /// A number or `(`.
    Operand,
    /// `+ - * /`, or `)`, or the end.
    Operator,
}

fn compact(input: &str) -> Result<String, String> {
    const MAX_DEPTH: usize = 32;
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut expect = Expect::Operand;
    let mut depth = 0;
    let mut at = 0;

    while at < bytes.len() {
        let byte = bytes[at];
        match (expect, byte) {
            (_, b' ') => at += 1,
            (Expect::Operand, b'(') => {
                if depth == MAX_DEPTH {
                    return Err(format!("nested too deeply at byte {at}"));
                }
                depth += 1;
                out.push('(');
                at += 1;
            }
            (Expect::Operand, b'0'..=b'9') => {
                let start = at;
                while at < bytes.len() && bytes[at].is_ascii_digit() {
                    at += 1;
                }
                // Only ASCII digits were passed, so these are character
                // boundaries. The digits are copied, never converted.
                out.push_str(&input[start..at]);
                expect = Expect::Operator;
            }
            (Expect::Operator, b'+' | b'-' | b'*' | b'/') => {
                out.push(char::from(byte));
                at += 1;
                expect = Expect::Operand;
            }
            (Expect::Operator, b')') => {
                if depth == 0 {
                    return Err(format!("`)` without `(` at byte {at}"));
                }
                depth -= 1;
                out.push(')');
                at += 1;
            }
            _ => return Err(format!("unexpected byte {byte:#04x} at byte {at}")),
        }
    }
    if expect == Expect::Operand {
        return Err("the input ends where a number should be".to_string());
    }
    if depth != 0 {
        return Err(format!("{depth} `(` not closed"));
    }
    Ok(out)
}

fn main() {
    assert_eq!(compact("( 1 + 20 ) * 3").as_deref(), Ok("(1+20)*3"));
    assert_eq!(compact("007").as_deref(), Ok("007"), "copied, not converted");
    for bad in ["", "1 +", "1 2", "+ 1", "( 1", "1 )", "( )", "1 + x"] {
        assert!(compact(bad).is_err(), "{bad:?} should be refused");
    }
    // The input decides the depth. The answer must be an error, not a crash.
    assert!(compact(&"(".repeat(33)).is_err());
    assert!(compact(&"(".repeat(1_000_000)).is_err());
}
```

Notice how `1 2` is refused. No rule says "two numbers in a row are
forbidden". The number rule stops after `1`, the situation becomes
`Operator`, and `2` is not allowed in that situation. A scanner that is
written as situations gets many such cases for free.

### Why not recursion

The textbook way to handle nesting is a function that calls itself for
every opening bracket. The depth of the input then becomes the depth of
the call stack, and the input comes from a stranger. Each thread has a
call stack of fixed size (2 MiB by default for a thread you spawn). When
it is full, this happens:

```text
thread 'main' (127702583) has overflowed its stack
fatal runtime error: stack overflow, aborting
```

(The number is the id of the thread; it differs from run to run.) That is
not a panic. No destructor runs, `catch_unwind` does not catch it,
and it does not end one thread: it ends the process. One request can stop
a whole server this way.

**When to use which.**

| | recursion with a depth limit | explicit counter or stack |
|---|---|---|
| good | reads like the grammar | the depth uses memory that you control; a larger limit costs a larger `Vec`, nothing else |
| bad | safe only while limit × frame size fits the stack, and nobody checks that when the limit is raised | the grammar is spread over the arms of one `match` |

Both are acceptable with a small limit. Know which one you chose, and what
would have to be true for it to stay safe.

**Mistakes people make.**

- A depth limit that is checked after the recursive call, or only when the
  bracket closes.
- Counting brackets without checking their order or kind: `[}` and `)(`
  have balanced counts.
- Converting tokens on the way (parsing a number into `f64` and printing
  it again). `2.50` comes back as `2.5`, and a number with forty digits
  comes back as a different number.
- Collecting the whole input into a tree "because we might need it".

**Read more.**
[`match` on tuples and ranges](https://doc.rust-lang.org/reference/patterns.html),
[`std::thread::Builder::stack_size`](https://doc.rust-lang.org/std/thread/struct.Builder.html#method.stack_size),
[`std::panic::catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)
(see what it says about aborts).

---

## 6. A limit must come before the cost

A server spends three things on a client: memory, time and threads. A
limit protects one of them only if it is applied **before** the thing is
spent. A check after the work is a report, not a limit.

The most common form of the mistake is to believe a number. The client
says "500 000 000 bytes follow", and the server makes room for them:

```rust,should_panic
// A length that came from the network.
let announced = usize::MAX;
let buffer: Vec<u8> = Vec::with_capacity(announced);
# drop(buffer);
```

```text
thread 'main' (127702528) panicked at /rustc/b940084d7eb6a299eb4bfeb8e34901bc051e7ac4/library/alloc/src/raw_vec/mod.rs:28:5:
capacity overflow
```

With a smaller number the allocation may succeed, and then a few bytes
from one client have cost the server gigabytes. If it fails, the process
is aborted. Neither is acceptable for a number that anybody can send.

There are two separate rules here.

1. **Compare the number with the limit before anything else.** The number
   alone is enough to refuse. Nothing has to be read, and nothing has to
   be allocated.
2. **Even below the limit, let memory follow the bytes that really
   arrive.** A `Vec` that grows as data comes in costs a client one byte
   of sending for one byte of your memory. A `Vec` that is sized from the
   announcement costs the client nothing.

The example reads a block that is announced by a four-byte length. It
works on any `Read`, so the test needs no socket.

```rust
use std::io::{self, Read};

fn read_block(input: &mut impl Read, max: u32) -> io::Result<Vec<u8>> {
    let mut length = [0u8; 4];
    input.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length);
    // Rule 1: refuse on the number alone.
    if length > max {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "block too large"));
    }
    // Rule 2: `take` stops the reader after `length` bytes, and
    // `read_to_end` makes the vector grow as bytes arrive.
    let mut block = Vec::new();
    input.by_ref().take(u64::from(length)).read_to_end(&mut block)?;
    if block.len() as u64 != u64::from(length) {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    Ok(block)
}

fn main() {
    let mut input: &[u8] = b"\x00\x00\x00\x05helloREST";
    assert_eq!(read_block(&mut input, 100).unwrap(), b"hello");
    assert_eq!(input, b"REST", "not one byte more than the block was taken");

    // Four bytes that announce four gigabytes. Refused; nothing allocated.
    let mut input: &[u8] = b"\xff\xff\xff\xff";
    let error = read_block(&mut input, 100).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);

    // A block that ends early is an error, not a short block.
    let mut input: &[u8] = b"\x00\x00\x00\x05hel";
    let error = read_block(&mut input, 100).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
}
```

Input that ends with a delimiter has no announced number, so the limit
must be applied while reading: after every read, either the delimiter has
been found, or the buffer is still below the limit, or the input is
refused *now*. The `Inbox` of section 1 does this for its length field.
The standard helpers `read_until`, `read_line` and `read_to_end` do not:
they collect until the delimiter or the end arrives, however long that
takes. Used directly on a socket, each of them is an unlimited buffer.

A useful exercise before M5: for every limit in SPEC 5.4, find the first
moment at which your server *can* know that the limit is exceeded, and
check that this is where your code refuses.

**Mistakes people make.**

- `vec![0; announced]` followed by `read_exact`.
- Reading a whole body and then comparing its length with the limit.
- A limit on each line and none on the number of lines.
- Parsing the announced length into an integer type and treating "does
  not fit" as a different error from "too large". For the sender both
  mean the same thing.

**Read more.**
[`Read::take`](https://doc.rust-lang.org/std/io/trait.Read.html#method.take),
[`Read::by_ref`](https://doc.rust-lang.org/std/io/trait.Read.html#method.by_ref),
[`Read::read_to_end`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_end),
[`Vec::try_reserve`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.try_reserve).

---

## 7. Deadlines, not timeouts

A blocking `read` waits until at least one byte is there. On a socket you
can limit that wait with `set_read_timeout`. This is a limit **for one
call**. A sender who delivers one byte just before each timeout ends can
keep the receiver busy for as long as it likes, and every single read
succeeds.

What a server usually wants is a limit for a whole operation: "this
request must be complete within five seconds". The way to get it:

1. Compute the **deadline** once: the moment at which the operation must
   be finished (`Instant::now() + limit`).
2. Before every blocking call, compute how much time is left, and use that
   as the timeout of this one call.
3. When no time is left, stop.

```text
timeout per read:   |--2s--|--2s--|--2s--|--2s--| ...   each read is "in time"
deadline:           |-------- 5s --------|              the whole thing is not
```

In the example, one thread sends ten bytes with a pause of 30 ms before
each of the last nine. The receiver first uses a generous timeout per
read, then a deadline of 100 ms for all ten bytes.

```rust
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

/// Fills `buf` completely, or fails when `deadline` has passed.
fn read_exact_by(stream: &mut TcpStream, buf: &mut [u8], deadline: Instant) -> io::Result<()> {
    let mut filled = 0;
    while filled < buf.len() {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        stream.set_read_timeout(Some(left))?;
        match stream.read(&mut buf[filled..]) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            // A timed-out read is `WouldBlock` on Unix, `TimedOut` on Windows.
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {
                return Err(io::ErrorKind::TimedOut.into());
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    thread::scope(|scope| -> io::Result<()> {
        // The slow sender: two connections, ten bytes on each.
        scope.spawn(|| {
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else { return };
                for byte in b"0123456789" {
                    if stream.write_all(&[*byte]).is_err() {
                        break; // the receiver has given up
                    }
                    thread::sleep(Duration::from_millis(30));
                }
            }
        });

        // A timeout per read. No read waits anywhere near two seconds, so
        // the timeout never fires, although the message takes its time.
        let mut patient = TcpStream::connect(addr)?;
        patient.set_read_timeout(Some(Duration::from_secs(2)))?;
        let mut message = [0u8; 10];
        patient.read_exact(&mut message)?;
        assert_eq!(&message, b"0123456789");

        // A deadline. The last byte is sent 270 ms after the first, so the
        // message cannot be complete after 100 ms.
        let started = Instant::now();
        let mut strict = TcpStream::connect(addr)?;
        let outcome = read_exact_by(&mut strict, &mut message, started + Duration::from_millis(100));
        assert_eq!(outcome.unwrap_err().kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() >= Duration::from_millis(100));
        Ok(())
    })
}
```

Three facts about the socket options, all easy to trip over:

```rust
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

let listener = TcpListener::bind("127.0.0.1:0")?;
let stream = TcpStream::connect(listener.local_addr()?)?;

// `None` means "wait for ever". That is the default.
assert_eq!(stream.read_timeout()?, None);

// A zero duration is not "do not wait". It is an error.
let error = stream.set_read_timeout(Some(Duration::ZERO)).unwrap_err();
assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);

// Reading and writing have separate timeouts.
stream.set_write_timeout(Some(Duration::from_secs(1)))?;
assert_eq!(stream.read_timeout()?, None);
# Ok::<(), std::io::Error>(())
```

The second fact is why `read_exact_by` tests `left.is_zero()` before it
sets the timeout. The third is a reminder that a `write` blocks too: when
the other side stops reading, the buffers between the two programs fill
up, and the next `write` waits. A server that limits only its reads can
still lose a thread to a client that asks for something large and then
does nothing.

**When to use which.** A timeout per call is the right tool when "nothing
at all has happened for this long" is what you want to detect: an idle
connection, a write that makes no progress. A deadline is the right tool
when a piece of work must be finished by a certain moment. A server needs
both, for different waits, and SPEC 7.3 says which is which.

**Mistakes people make.**

- One `set_read_timeout` when the connection is accepted, and nothing
  else. That is a limit per read.
- Handling only `WouldBlock`, or only `TimedOut`. The program then works
  on one family of systems and treats a timeout as a broken connection on
  the other.
- Treating *every* error as a timeout and trying again. An error that
  comes back immediately turns the loop into one that never sleeps.
- No limit on writes.
- Measuring with `SystemTime`. Use `Instant`: it never goes backwards.

**Read more.**
[`TcpStream::set_read_timeout`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.set_read_timeout)
(the documentation names both error kinds),
[`TcpStream::set_write_timeout`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.set_write_timeout),
[`Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html),
[`Instant::saturating_duration_since`](https://doc.rust-lang.org/std/time/struct.Instant.html#method.saturating_duration_since).

---

## 8. Ending a connection

A TCP connection is two streams, and each can be ended separately.

- `stream.shutdown(Shutdown::Write)` ends *your* sending direction. The
  other side reads everything you sent and then gets `Ok(0)` from `read`,
  the end of the stream. You can still read. This is called a
  **half-close**.
- Dropping the `TcpStream` closes both directions and gives the socket
  back to the system.

A half-close is the polite way to say "that was all from me". In the
example, a client sends a text of unknown length, says that it is done,
and then reads the answer on the same connection.

```rust
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    thread::scope(|scope| -> io::Result<()> {
        // A service that answers with the text it received, in capitals.
        let service = scope.spawn(|| -> io::Result<()> {
            let (mut stream, _) = listener.accept()?;
            let mut text = Vec::new();
            // Returns when the client has ended its sending direction.
            stream.read_to_end(&mut text)?;
            text.make_ascii_uppercase();
            // The other direction is still open.
            stream.write_all(&text)
        });

        let mut client = TcpStream::connect(addr)?;
        client.write_all(b"hello, ")?;
        client.write_all(b"world")?;
        client.shutdown(Shutdown::Write)?;

        let mut answer = String::new();
        client.read_to_string(&mut answer)?;
        assert_eq!(answer, "HELLO, WORLD");
        service.join().expect("the service thread does not panic")
    })
}
```

Without the `shutdown` line this program waits for ever: the service
waits for the end of the text, and the client waits for the answer.

### A close that destroys what you sent

The system ends a connection in one of two ways. The normal way is an
orderly end: everything that was sent is delivered, then the other side
sees the end of the stream. The other way is a **reset**: "this
connection no longer exists, stop at once". A program that receives a
reset gets an error such as `ConnectionReset` from its next read or
write, and its system may throw away data that had already arrived and
was waiting to be read.

On most systems, dropping a socket **while received bytes are still
unread** produces a reset and not an orderly end. The system reasons that
data was lost, so the other side should know.

A server gets into exactly this situation when it answers early:

```text
client                                        server
  │ ── start of a large upload ─────────────────> │  reads the start only
  │ ── more of the upload ──────────────────────> │  decides: "too large"
  │ <───────────────────────── error response ─── │
  │ ── more of the upload ──────────────────────> │  drops the socket,
  │ <─────────────────────────────────── reset ── │  unread bytes inside
  ✗ reports "connection reset"; the response may be gone
```

The response was correct and was sent, and the client still cannot read
it. The cure has three steps, in this order: end the sending direction
(the response is now complete, and the client sees its end); keep reading
and throwing away what the client still sends, so that no unread bytes
are left; then drop the socket. SPEC 7.4 gives the limits that the middle
step needs. Without limits, a client that never stops sending holds the
thread.

**When to use which.**

| situation | how to end |
|---|---|
| you sent a last message and the other side may still be sending | half-close, read and discard within limits, drop |
| the other side has ended its direction and you have read everything | drop |
| nothing was sent, and there is nothing to protect | drop |

**Mistakes people make.**

- Thinking that `write_all` returning `Ok` means the other side has the
  data. It means the system has accepted it for sending.
- `Shutdown::Both` to "close properly". It also ends your reading
  direction. You can then no longer take the arriving bytes out of the
  socket, and what the system does with them differs from system to
  system.
- Reading and discarding without a limit on time *and* on bytes.
- Testing only on one system. Whether the response survives a reset
  depends on the systems at both ends and on timing; a test that passes
  on your machine does not show that the close is right.

**Read more.**
[`TcpStream::shutdown`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.shutdown),
[`Shutdown`](https://doc.rust-lang.org/std/net/enum.Shutdown.html),
[`io::ErrorKind::ConnectionReset`](https://doc.rust-lang.org/std/io/enum.ErrorKind.html#variant.ConnectionReset).

---

## 9. A bounded hand-off between threads

A thread per client is the simplest design for a server, and it has no
upper bound: a thousand clients are a thousand threads, each with its own
stack. A **pool** turns this around. The number of threads is fixed, and
work waits in a queue until a thread is free.

The queue needs a bound as well. An unbounded queue only moves the
problem: when work arrives faster than it is done, the queue grows until
memory runs out, and every item in it waits longer than the one before.
With a bound, a third answer becomes possible besides "now" and "later":
**"no"**. Refusing at once, while the client can still go elsewhere or
try again, is called **back-pressure**.

The standard library has a bounded queue: `mpsc::sync_channel(n)`. Its
`try_send` never waits. When the channel is full, it gives the item back
inside the error, so the caller still owns it and can do something with
it.

In the example, two clerks take jobs from a channel with room for one
waiting job. The clerks report each job they start and then hold it until
the main thread says "finish one", so the test controls exactly who is
busy. There is no `sleep` in it.

```rust
use std::sync::mpsc::{self, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let (jobs, taken) = mpsc::sync_channel::<u32>(1);
    // A `Receiver` cannot be shared directly; see the error below.
    let taken = Arc::new(Mutex::new(taken));
    let (started, has_started) = mpsc::channel::<u32>();
    let (finish, finished) = mpsc::channel::<()>();
    let finished = Arc::new(Mutex::new(finished));

    let clerks: Vec<_> = (0..2)
        .map(|_| {
            let taken = Arc::clone(&taken);
            let started = started.clone();
            let finished = Arc::clone(&finished);
            thread::spawn(move || {
                loop {
                    // The guard is a temporary of this statement. It is
                    // dropped at the semicolon, before the job is done.
                    let job = taken.lock().unwrap().recv();
                    let Ok(job) = job else {
                        break; // every sender is gone: no more work
                    };
                    started.send(job).unwrap();
                    // "Work" until the main thread allows one job to end.
                    let _ = finished.lock().unwrap().recv();
                }
            })
        })
        .collect();

    // Two jobs, and each is seen to start: both clerks are busy.
    jobs.try_send(1).unwrap();
    assert_eq!(has_started.recv(), Ok(1));
    jobs.try_send(2).unwrap();
    assert_eq!(has_started.recv(), Ok(2));

    // One more may wait. The next one is refused and comes back.
    jobs.try_send(3).unwrap();
    match jobs.try_send(4) {
        Err(TrySendError::Full(job)) => assert_eq!(job, 4),
        other => panic!("expected a full channel, got {other:?}"),
    }

    // One clerk finishes and takes the waiting job. Now there is room.
    finish.send(()).unwrap();
    assert_eq!(has_started.recv(), Ok(3));
    jobs.try_send(5).unwrap();

    // Closing: no more jobs, and nothing holds the clerks back.
    drop(jobs);
    drop(finish);
    for clerk in clerks {
        clerk.join().unwrap();
    }
    assert_eq!(has_started.try_iter().collect::<Vec<_>>(), [5]);
}
```

Look closely at what "full" meant here. The channel counts the jobs that
*wait*. It knows nothing about the clerks. The example only behaves so
neatly because the test waits until each job has been taken before it
sends the next. Send three jobs in a row without waiting, and the third
may be refused although both clerks are about to be free. With
`sync_channel(0)` the edge is sharper still: `try_send` succeeds only
while a receiver is already blocked in `recv`.

```rust
use std::sync::mpsc::{self, TrySendError};

let (tx, _rx) = mpsc::sync_channel::<u32>(0);
// A receiver exists, but it is not waiting at this moment.
assert!(matches!(tx.try_send(1), Err(TrySendError::Full(1))));
```

Whether that is good enough depends on what your limit is supposed to
count. Read SPEC 10.2 with this question in mind before you choose
between a channel and a queue of your own with a `Mutex` and a `Condvar`.

### Two mistakes with a shared receiver

A `Receiver` can be moved to another thread but not shared:

```rust,compile_fail,E0277
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

fn main() {
    let (jobs, taken) = mpsc::channel::<u32>();
    let taken = Arc::new(taken);
    let clerk = {
        let taken = Arc::clone(&taken);
        thread::spawn(move || taken.recv())
    };
    jobs.send(1).unwrap();
    let _ = clerk.join();
}
```

```text
error[E0277]: `std::sync::mpsc::Receiver<u32>` cannot be shared between threads safely
   --> src/main.rs:10:23
    |
 10 |         thread::spawn(move || taken.recv())
    |         ------------- ^^^^^^^^^^^^^^^^^^^^ `std::sync::mpsc::Receiver<u32>` cannot be shared between threads safely
    |         |
    |         required by a bound introduced by this call
    |
    = help: the trait `Sync` is not implemented for `std::sync::mpsc::Receiver<u32>`
    = note: required for `Arc<std::sync::mpsc::Receiver<u32>>` to implement `Send`
```

The fix is the `Arc<Mutex<Receiver<_>>>` of the example. And that fix has
a trap of its own. This loop looks like the natural way to write a
worker:

```rust
use std::sync::Mutex;
use std::sync::mpsc;

let (jobs, taken) = mpsc::channel();
jobs.send("job").unwrap();
let taken = Mutex::new(taken);

while let Ok(_job) = taken.lock().unwrap().try_recv() {
    // The guard that `lock()` returned is still alive here.
    assert!(taken.try_lock().is_err(), "the lock is held while the job runs");
}

// A `let` statement drops its temporaries at the semicolon.
let _job = taken.lock().unwrap().try_recv();
assert!(taken.try_lock().is_ok());
```

In a `while let`, the temporaries of the condition live until the end of
the loop body. The worker would hold the lock on the queue for as long as
its job takes, and all other workers would wait for the lock: a pool that
does one thing at a time. Take the job out in a statement of its own.

### A job that panics

A panic in a spawned thread ends that thread and nothing else. Nobody is
told unless somebody calls `join`. A pool that loses a thread on every
panic gets smaller without a sound, until it has no threads left and the
queue only fills.

```rust
use std::panic;
use std::thread;

let worker = thread::spawn(|| {
    let mut done = 0;
    for input in ["1", "x", "3"] {
        // A job that panics on bad input. The panic stops this job, not
        // the loop. (Its message still appears on standard error.)
        let job = panic::catch_unwind(|| input.parse::<u32>().unwrap());
        if job.is_ok() {
            done += 1;
        }
    }
    done
});
assert_eq!(worker.join().unwrap(), 2);
```

`catch_unwind` is not a replacement for error handling: code that reads
input from outside should return `Err`, not panic. It is the second line
of defence, for the bug that you have not found yet. Also think about
what a panic leaves behind: a count that was increased and never
decreased, or a `Mutex` that was locked at that moment. Such a mutex is
**poisoned**, and `lock()` returns `Err` from then on. What that should
mean for your pool is a decision, and `unwrap()` makes it without telling
you.

**Mistakes people make.**

- An unbounded queue "to be safe".
- Checking for room and inserting in two steps, with the lock released in
  between. Two threads can both see the last free place.
- Holding a lock while a job runs (the `while let` above is one way).
- Waiting on a `Condvar` with `if` and not with `while`. A wait can
  return without a notification, and another thread may have taken the
  item first.
- Counting something other than what the limit is about.

**Read more.**
[`mpsc::sync_channel`](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html),
[`SyncSender::try_send`](https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html#method.try_send),
[`Condvar`](https://doc.rust-lang.org/std/sync/struct.Condvar.html),
[`Mutex` and poisoning](https://doc.rust-lang.org/std/sync/struct.Mutex.html#poisoning),
[temporary scopes](https://doc.rust-lang.org/reference/destructors.html#temporary-scopes).
The mastery module [`m3_concurrency`](../../mastery/m3_concurrency/README.md) builds a channel and a mutex from
their parts.

---

## 10. Threads that are blocked

A thread that waits for the network is not running your code. It is
inside the operating system, in a call such as `accept` or `read`, and it
stays there until the thing it waits for happens. A flag that another
thread sets does nothing by itself: the blocked thread looks at the flag
only after its call has returned.

So stopping a program in good order has two parts. One **flag** says that
it is time to stop. And for each thread that may be blocked, either
something makes its call return, or the call never waits for long.

| the thread is blocked in | ways to make it look at the flag |
|---|---|
| `TcpListener::accept` | make a connection to the listener yourself; or use a non-blocking listener and sleep between attempts |
| `TcpStream::read` | give each read a short timeout and check the flag between reads; or keep a second handle (`try_clone`) and call `shutdown` on it from another thread |
| `Condvar::wait` | `notify_all`, after the flag was set under the same mutex |
| `Receiver::recv` | drop every `Sender` |
| `Stdin::read_line` | nothing portable. Leave the thread alone (see below) |

Every entry has a price. A wake-up connection is a real connection that
the accepting thread must recognise and throw away. Short timeouts mean
that every waiting thread wakes many times per second for nothing, and
that stopping takes up to one timeout. A second handle for every socket
means a list of handles that must be kept in step with the threads that
own the sockets. There is no free choice; there is the one whose price
you can state.

The example shows the first row. A doorman thread greets every visitor
and counts them. It stops when the flag is set *and* something wakes it.

```rust
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let closing = AtomicBool::new(false);

    thread::scope(|scope| -> io::Result<()> {
        let doorman = scope.spawn(|| {
            let mut greeted = 0;
            loop {
                let visitor = listener.accept();
                // Look at the flag before anything is done with the
                // visitor: the wake-up connection is not a visitor.
                if closing.load(Ordering::Relaxed) {
                    return greeted;
                }
                if let Ok((mut stream, _)) = visitor {
                    if stream.write_all(b"!").is_ok() {
                        greeted += 1;
                    }
                }
            }
        });

        for _ in 0..3 {
            let mut visitor = TcpStream::connect(addr)?;
            let mut greeting = [0u8; 1];
            // Wait for the greeting, so that this visitor has been counted.
            visitor.read_exact(&mut greeting)?;
        }

        // Setting the flag is not enough: the doorman is inside `accept`.
        closing.store(true, Ordering::Relaxed);
        // This connection is what makes `accept` return.
        let _wake = TcpStream::connect(addr)?;

        assert_eq!(doorman.join().expect("the doorman does not panic"), 3);
        Ok(())
    })
}
```

The order of the two lines at the end matters. If the connection were
made first and the flag set afterwards, the doorman could accept the
wake-up connection, find the flag still clear, greet it, and go back to
sleep in `accept` for ever.

The flag is an `AtomicBool` with `Ordering::Relaxed`. That is enough when
a flag guards no other data: no thread uses the flag to conclude that
some *other* memory is ready to be read. If your flag does guard
something, the mastery module [`m3_concurrency`](../../mastery/m3_concurrency/README.md) explains what to use.

### The thread that cannot be woken

The standard library offers no way to interrupt a thread that reads
standard input. You do not need one. When `main` returns, the process
ends, and every other thread ends with it wherever it is:

```rust
use std::thread;
use std::time::Duration;

fn main() {
    thread::spawn(|| {
        loop {
            thread::sleep(Duration::from_secs(3600));
        }
    });
    // `main` returns here. The sleeping thread does not keep the
    // process alive.
}
```

This is safe for a thread that owns nothing which must be finished: no
half-written file, no response on its way. It is wrong for every other
thread. A worker that is cut off in the middle of a `write` leaves a
client with half an answer, which is why the last step of an orderly stop
is to `join` the threads that do real work, and why `std::process::exit`
from a helper thread is a crash that happens to return 0.

**Mistakes people make.**

- Setting a flag and expecting blocked threads to notice.
- Waking a thread first and setting the flag second.
- A loop that checks the flag and calls `thread::yield_now` or a very
  short sleep. That is a busy loop with extra steps.
- Stopping the acceptor and forgetting the connections that already
  wait in a queue or sit idle with a worker.
- Calling `exit` when "the important work is done", while other threads
  are still writing.

**Read more.**
[`TcpListener::accept`](https://doc.rust-lang.org/std/net/struct.TcpListener.html#method.accept),
[`TcpListener::set_nonblocking`](https://doc.rust-lang.org/std/net/struct.TcpListener.html#method.set_nonblocking),
[`TcpStream::try_clone`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.try_clone),
[`thread::scope`](https://doc.rust-lang.org/std/thread/fn.scope.html),
[`AtomicBool`](https://doc.rust-lang.org/std/sync/atomic/type.AtomicBool.html),
[`std::process::exit`](https://doc.rust-lang.org/std/process/fn.exit.html)
(read what it says about destructors).
