# Chapter 17 concepts — Working with text

[Chapter guide](README.md) · [Hints](HINTS.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the teaching text for that lesson. Every code block in this file was compiled and run with the course toolchain (Rust 1.99.0, edition 2024), and every compiler message and panic message was copied from that toolchain. Long messages are shortened: a line with `...` marks a cut, and the end of a message is often left out. In a panic message, the number in parentheses after the thread name is a thread id; yours will be different. You can paste any block into [the scratch example](../../examples/ch17_scratch.rs) and change it.

## What you will be able to do

After this chapter you can:

- say how many bytes and how many Unicode scalar values a piece of text has, and why neither number is "the number of characters on the screen";
- edit a `String` in place without ever cutting a character in half;
- search, split and trim text, and use the byte positions that the search methods return;
- turn text into your own types with `FromStr`, and your own types into text with `Display`;
- read and write format strings: width, alignment, precision, number bases, named arguments;
- write formatted output into any destination without building a temporary `String`;
- convert between bytes and text, and delay an allocation with `Cow` until it is really needed.

## Before you start

`String`, `&str`, `char` and slices (Chapter 3). Ownership and borrowing (Chapters 4 and 5). `Result`, `Option` and the `?` operator (Chapter 7). Implementing a trait for your own type (Chapter 11). Closures (Chapter 14) and iterator chains with `map`, `filter` and `collect` (Chapter 15). The review lesson also uses enums (Chapter 10) and `BTreeMap` (Chapter 16).

## Time

These are estimates for a learner who has done the earlier chapters. They include reading the section, the predictions, the code, and your notes.

| Lesson | Estimate |
| --- | --- |
| 01 · Unicode scalar operations and encoding | 45 minutes |
| 02 · Constructing and editing owned text | 45 minutes |
| 03 · Search patterns, byte positions, and replacement | 45 minutes |
| 04 · Text iteration, trimming, and casing | 40 minutes |
| 05 · FromStr and textual conversion | 50 minutes |
| 06 · The formatting language | 50 minutes |
| 07 · Display, fmt::Write, and format_args! | 60 minutes |
| 08 · UTF-8, borrowed text views, and delayed allocation | 50 minutes |
| 09 · Capstone | 60 minutes |
| R01 review, C01 code review, drills | 2 hours |
| Regex lab (separate package) | 60 minutes |

## Words used in this chapter

- **Byte**: eight bits. The length of a `String` or `&str` is always counted in bytes.
- **Code point**: the number that Unicode gives to a character. It is written in hexadecimal with the prefix `U+`, for example `U+00E9` for é.
- **Unicode scalar value** (short: **scalar**): any code point except the range `U+D800` to `U+DFFF`. That range is reserved for another encoding, UTF-16, and never stands for a character. A Rust `char` is exactly one scalar.
- **UTF-8**: the encoding that Rust uses for all text. It stores each scalar in one to four bytes.
- **Char boundary**: a byte position where a scalar starts, or the end of the text. Text may be cut only at a char boundary.
- **Grapheme cluster**: what a reader sees as one character. It can be made of several scalars.

## Lesson 01 · Unicode scalar operations and encoding

### The idea

Text in Rust has two levels. A `char` is one Unicode scalar. It always takes four bytes of memory, so that every scalar fits. A `String` or `&str` is a sequence of scalars stored as UTF-8, where a scalar takes between one and four bytes:

```text
scalar range             bytes in UTF-8   example
U+0000  to U+007F        1                'A'   41
U+0080  to U+07FF        2                '£'   C2 A3
U+0800  to U+FFFF        3                '✓'   E2 9C 93
U+10000 to U+10FFFF      4                '🎉'  F0 9F 8E 89
```

ASCII text uses one byte per scalar, so for ASCII all counts are equal. For all other text you must know which count you want.

### Worked example: one label, three ways to count

```rust
let label = "£5✓🎉";

assert_eq!(label.len(), 10); // bytes
assert_eq!(label.chars().count(), 4); // scalars

// `char_indices` gives each scalar together with the byte offset where it starts.
let starts: Vec<(usize, char)> = label.char_indices().collect();
assert_eq!(starts, [(0, '£'), (2, '5'), (3, '✓'), (6, '🎉')]);

assert_eq!(label.as_bytes()[..3], [0xC2, 0xA3, b'5']);
assert_eq!(size_of::<char>(), 4);
```

```text
byte offset    0    1    2    3    4    5    6    7    8    9
byte          C2   A3   35   E2   9C   93   F0   9F   8E   89
scalar        └── £ ──┘  5   └───── ✓ ────┘ └─────── 🎉 ──────┘
starts at      0         2    3              6
```

`len()` reads a number from the header and costs nothing. `chars().count()` must walk over all the bytes.

### Working with one char

```rust
assert!('ñ'.is_alphabetic());
assert!('\u{A0}'.is_whitespace()); // NO-BREAK SPACE
assert!('٣'.is_numeric()); // ARABIC-INDIC DIGIT THREE
assert!(!'٣'.is_ascii_digit());

// `to_digit(radix)` knows only 0-9 and the ASCII letters, small or capital.
assert_eq!('c'.to_digit(16), Some(12));
assert_eq!('c'.to_digit(10), None);
assert_eq!('٣'.to_digit(10), None);
assert_eq!(char::from_digit(7, 10), Some('7'));

// Between char and number.
assert_eq!('A' as u32, 0x41);
assert_eq!(char::from(b'A'), 'A');
assert_eq!(char::from_u32(0x2713), Some('✓'));
assert_eq!(char::from_u32(0xDC00), None); // inside the reserved range
assert_eq!('✓'.len_utf8(), 3);
```

Changing case is not always one scalar in, one scalar out. Some letters have no single-scalar partner in the other case. For that reason `to_uppercase` and `to_lowercase` on a `char` return an iterator of chars:

```rust
let upper: String = 'ǰ'.to_uppercase().collect();
assert_eq!(upper.chars().count(), 2); // 'J' followed by a combining mark
assert_eq!('q'.to_uppercase().collect::<String>(), "Q");
assert_eq!('q'.to_ascii_uppercase(), 'Q'); // ASCII only, returns a char
```

### One visible character can be several scalars

```rust
let composed = "\u{F6}"; // ö as one scalar
let combined = "o\u{308}"; // o followed by COMBINING DIAERESIS
assert_ne!(composed, combined); // different scalars, so not equal
assert_eq!(composed.chars().count(), 1);
assert_eq!(combined.chars().count(), 2);

let flag = "🇸🇪"; // two "regional indicator" scalars
assert_eq!((flag.chars().count(), flag.len()), (2, 8));
```

Both spellings of ö look the same on the screen. The standard library compares scalars and does not convert one spelling into the other (that conversion is called *normalization*). It also does not group scalars into grapheme clusters, does not know how many terminal columns a character takes, and does not reorder right-to-left text for display. Text is stored in reading order; the display order is decided when the text is drawn. For these jobs, programs use crates from outside the standard library.

### Mistakes people make

**Casting a number to `char` with `as`.** Only a `u8` can be cast, because every `u8` is a valid scalar. Other numbers may not be.

```rust,compile_fail,E0604
let code: u32 = 0x1F389;
let party = code as char;
```

```text
error[E0604]: only `u8` can be cast as `char`, not `u32`
 --> src/main.rs:3:17
  |
3 |     let party = code as char;
  |                 ^^^^^^^^^^^^ invalid cast
  |
help: consider using `char::from_u32` instead
```

The fix is `char::from_u32(code)`. It returns an `Option<char>`, so you must decide what to do with a number that is not a scalar.

**Expecting `to_uppercase` to return a `char`.**

```rust,compile_fail,E0308
let small = 'ß';
let big: char = small.to_uppercase();
```

```text
error[E0308]: mismatched types
 --> src/main.rs:3:21
  |
3 |     let big: char = small.to_uppercase();
  |              ----   ^^^^^^^^^^^^^^^^^^^^ expected `char`, found `ToUppercase`
  |              |
  |              expected due to this
```

Collect the iterator into a `String`, or push its chars into the text you are building with `extend`.

**Using `chars().count()` as "the number of characters the user sees".** It counts scalars. The flag above is one picture and two scalars.

### When to use which

| You need | Use |
| --- | --- |
| Memory size, a limit in a file format or a network protocol | bytes: `len()` |
| Logic about single letters or digits | scalars: `chars()`, `char_indices()` |
| A cursor position, or "at most 20 characters" as a person understands it | grapheme clusters, from an external crate |

### Read more

- [`char`](https://doc.rust-lang.org/std/primitive.char.html) in the standard library
- [`str::char_indices`](https://doc.rust-lang.org/std/primitive.str.html#method.char_indices)
- [Textual types](https://doc.rust-lang.org/reference/types/textual.html) in the Rust Reference

## Lesson 02 · Constructing and editing owned text

### The idea

A `String` is a `Vec<u8>` with one promise: the bytes are always valid UTF-8. The header holds a pointer, a capacity and a length, as you saw in Chapter 4. Both numbers count **bytes**.

Every editing method keeps the promise. This has one consequence that you must remember: a method that takes a position takes a **byte offset**, and it panics if that offset is not a char boundary. The methods never leave half a character in the buffer.

| Purpose | Methods |
| --- | --- |
| Create | `String::new()`, `String::with_capacity(bytes)`, `String::from(text)`, `text.to_owned()` |
| Add at the end | `push(char)`, `push_str(&str)`, `extend(iterator)` |
| Add in the middle | `insert(at, char)`, `insert_str(at, &str)` |
| Remove | `pop()`, `remove(at)`, `truncate(new_len)`, `clear()`, `drain(range)`, `retain(test)` |
| Replace a part | `replace_range(range, &str)` |

`pop` and `remove` return a whole `char`, never a single byte of one. All of these methods change the existing buffer. They allocate only when the text grows beyond the capacity.

### Worked example: hide the middle of a name

The task: keep the first and the last scalar of a name and replace everything between them with stars. The positions for `replace_range` must be byte offsets, so the code asks `char_indices` for them.

```rust
/// Replaces every scalar between the first and the last one with '*'.
fn mask(name: &mut String) {
    let count = name.chars().count();
    if count < 3 {
        return; // nothing between the first and the last scalar
    }
    // Byte offsets where the second and the last scalar start.
    let second = name.char_indices().nth(1).map(|(at, _)| at).unwrap();
    let last = name.char_indices().next_back().map(|(at, _)| at).unwrap();
    name.replace_range(second..last, &"*".repeat(count - 2));
}

let mut name = String::from("Åsa-Zoë");
mask(&mut name);
assert_eq!(name, "Å*****ë");
```

```text
byte offset   0  1   2   3   4   5   6   7  8
scalar        └ Å ┘  s   a   -   Z   o   └ ë ┘
                     ^ second = 2        ^ last = 7
replace_range(2..7, "*****")
```

If the code had used `1..count - 1`, the range would have started at byte 1, in the middle of `Å`, and the program would have panicked.

### Capacity

Capacity is the number of bytes the buffer can hold before it must grow. Growing means a new, larger allocation and a copy of the old bytes. When you know the final size, ask for it once:

```rust
let mut line = String::with_capacity(16);
let buffer = line.as_ptr();
line.push_str("temp=");
line.push_str("21.5");
line.push('C');
assert_eq!(line, "temp=21.5C");
assert_eq!(line.as_ptr(), buffer); // still the first buffer: 10 bytes fit in 16
assert!(line.capacity() >= 16);

line.clear();
assert_eq!(line.len(), 0);
assert!(line.capacity() >= 16); // `clear` keeps the buffer for reuse
```

### Mistakes people make

**A byte offset inside a character.** The compiler cannot see this mistake, because offsets are values that exist only when the program runs. The program panics.

```rust,should_panic
let mut name = String::from("Åsa");
name.truncate(1); // 'Å' occupies bytes 0 and 1
```

```text
thread 'main' (4242) panicked at src/main.rs:3:10:
assertion failed: self.is_char_boundary(new_len)
```

Slicing with a bad offset gives a more detailed message:

```rust,should_panic
let name = String::from("Åsa");
let rest = &name[1..];
```

```text
thread 'main' (4242) panicked at src/main.rs:3:21:
start byte index 1 is not a char boundary; it is inside 'Å' (bytes 0..2 of string)
```

Take offsets from `char_indices`, from a search method (lesson 03), or test a number with `is_char_boundary`:

```rust
let name = "Åsa";
assert!(!name.is_char_boundary(1));
assert!(name.is_char_boundary(2));
assert!(name.is_char_boundary(name.len())); // the end is a boundary
assert!(!name.is_char_boundary(99)); // beyond the end: false, no panic
```

**Reading capacity as a number of characters.** `String::with_capacity(5)` has room for five bytes. That is five ASCII letters, but only one `🎉` with one byte left over.

**Building text with `+` in a loop.** `a + &b` moves `a`, appends to its buffer and returns it. It works, but `push_str` says the same thing without the move, and `format!` is clearer when several pieces are combined.

### When to use which

- `push` and `push_str` in a loop, with `with_capacity` if you know the size.
- `format!` to build one text from a few values. It always creates a new `String`.
- `insert`, `remove` and `drain` at the front of a long text move all later bytes. That is fine for short text; for long text, build a new `String` in one pass.
- `truncate`, `pop` and `clear` never allocate.

### Read more

- [`String`](https://doc.rust-lang.org/std/string/struct.String.html), especially the section on UTF-8
- [`str::is_char_boundary`](https://doc.rust-lang.org/std/primitive.str.html#method.is_char_boundary)

## Lesson 03 · Search patterns, byte positions, and replacement

### The idea

Most search methods on `str` take a **pattern**: a description of what to look for. Four kinds of value can be a pattern.

| Pattern | Matches | Example |
| --- | --- | --- |
| `&str` | that exact text | `"::"` |
| `char` | that one scalar | `'/'` |
| array or slice of `char` | any one of the scalars | `['.', '/']` |
| function or closure from `char` to `bool` | any one scalar for which it returns `true` | `char::is_whitespace` |

The methods answer in three ways. Some return a `bool` (`contains`, `starts_with`, `ends_with`). Some return a position (`find`, `rfind`), and the position is a **byte offset**. Some return parts of the text (`split_once`, `strip_prefix`, `matches`): these parts are `&str` slices that borrow from the text you searched, so nothing is allocated. Only the replacing methods build a new `String`.

### Worked example: taking a path apart

```rust
let path = "reports/2024/summary.final.csv";

assert!(path.starts_with("reports/"));
assert_eq!(path.find('/'), Some(7));
assert_eq!(path.rfind('/'), Some(12)); // search from the end

// `rsplit_once` cuts at the last match and leaves the separator out.
let (dir, file) = path.rsplit_once('/').unwrap();
assert_eq!((dir, file), ("reports/2024", "summary.final.csv"));
let (stem, extension) = file.rsplit_once('.').unwrap();
assert_eq!((stem, extension), ("summary.final", "csv"));

// `strip_suffix` removes a known ending once, or reports that it is absent.
assert_eq!(file.strip_suffix(".csv"), Some("summary.final"));
assert_eq!(file.strip_suffix(".txt"), None);

// All matches, with or without their positions.
assert_eq!(path.matches('/').count(), 2);
let dots: Vec<usize> = file.match_indices('.').map(|(at, _)| at).collect();
assert_eq!(dots, [7, 13]);

// A closure and an array as patterns.
assert_eq!(path.find(|c: char| c.is_ascii_digit()), Some(8));
assert_eq!(path.find(['.', '/']), Some(7));

// Replacing builds a new String. `replacen` stops after the given number.
assert_eq!(path.replace('/', "::"), "reports::2024::summary.final.csv");
assert_eq!(path.replacen('/', "::", 1), "reports::2024/summary.final.csv");
```

### Positions are byte offsets

```rust
let route = "Zürich→Genève";
let at = route.find('→').unwrap();
assert_eq!(at, 7); // not 6: 'ü' takes two bytes

assert_eq!(&route[..at], "Zürich");
assert_eq!(&route[at + '→'.len_utf8()..], "Genève");

// The scalar position is a different number. You rarely need it.
assert_eq!(route.chars().position(|c| c == '→'), Some(6));
```

A byte offset is exactly what slicing needs, and getting it costs nothing extra. A match always starts and ends at a char boundary, so an offset from `find` is always safe for slicing.

Two more rules:

```rust
// Matches do not overlap. After a match, the search continues behind it.
assert_eq!("xoxoxox".matches("xox").count(), 2);

// The empty text matches at every char boundary, including both ends.
assert_eq!("ab".matches("").count(), 3);
```

### Mistakes people make

**Passing a `String` as a pattern.** An owned `String` is not one of the pattern types. A reference to it is, because `&String` can be read as `&str`.

```rust,compile_fail,E0277
let text = "one, two";
let needle = String::from(", ");
println!("{}", text.contains(needle));
```

```text
error[E0277]: the trait bound `String: Pattern` is not satisfied
 --> src/main.rs:4:34
  |
4 |     println!("{}", text.contains(needle));
  |                         -------- ^^^^^^ the trait `Pattern` is not implemented for `String`
  |                         |
  |                         required by a bound introduced by this call
...
help: consider borrowing here
  |
4 |     println!("{}", text.contains(&needle));
  |                                  +
```

Write `&needle`, as the compiler suggests.

**Stepping over a separator with `+ 1`.** This works only while the separator is one byte long.

```rust,should_panic
let route = "Zürich→Genève";
let at = route.find('→').unwrap();
let destination = &route[at + 1..];
```

```text
thread 'main' (4242) panicked at src/main.rs:4:29:
start byte index 8 is not a char boundary; it is inside '→' (bytes 7..10 of string)
```

Use `split_once`, which does the arithmetic for you, or add `separator.len_utf8()` (for a `char`) or `separator.len()` (for a `&str`).

**Using the result of `find` as a scalar position**, for example in `text.chars().nth(position)`. The two numbers agree only for ASCII text, so the bug stays hidden until the first accented letter arrives.

### When to use which

| Question | Method |
| --- | --- |
| Is it there? | `contains`, `starts_with`, `ends_with` |
| Where is it? | `find`, `rfind` |
| Give me both sides of the first or last separator | `split_once`, `rsplit_once` |
| Remove a known prefix or suffix once | `strip_prefix`, `strip_suffix` |
| Every match, or every match with its offset | `matches`, `match_indices` |
| Change every match, or only the first few | `replace`, `replacen` |

When the thing to find has a structure ("some letters, then `=`, then digits"), a regular expression is the usual tool. The standard library has none; the chapter's [regex lab](regex-lab/README.md) uses the `regex` crate.

### Read more

- [Methods of `str`](https://doc.rust-lang.org/std/primitive.str.html#method.find), from `find` downwards
- [`std::str::pattern`](https://doc.rust-lang.org/std/str/pattern/index.html): the list of types that can be patterns

## Lesson 04 · Text iteration, trimming, and casing

### The idea

A `&str` can be walked in several ways. Each way is an iterator (Chapter 15) over the same bytes. Nothing is copied: the items are numbers, chars, or `&str` slices that borrow from the original text.

| Iterator | Items |
| --- | --- |
| `bytes()` | each byte as `u8` |
| `chars()`, `char_indices()` | each scalar, alone or with its byte offset |
| `lines()` | each line without its line ending (`\n` or `\r\n`) |
| `split(pattern)` | the parts between matches |
| `split_terminator(pattern)` | like `split`, but without an empty part after a final match |
| `split_whitespace()` | the words between runs of whitespace |

The iterators that split text differ in what they do with empty parts and with the end of the text. This is a common source of small bugs, so look at one input under all of them:

```rust
let text = "alpha  beta\r\ngamma\n";

assert_eq!(text.lines().collect::<Vec<_>>(), ["alpha  beta", "gamma"]);
assert_eq!(
    text.split('\n').collect::<Vec<_>>(),
    ["alpha  beta\r", "gamma", ""]
);
assert_eq!(
    text.split_terminator('\n').collect::<Vec<_>>(),
    ["alpha  beta\r", "gamma"]
);
assert_eq!(
    "alpha  beta".split(' ').collect::<Vec<_>>(),
    ["alpha", "", "beta"]
);
assert_eq!(
    text.split_whitespace().collect::<Vec<_>>(),
    ["alpha", "beta", "gamma"]
);
```

`split` treats the pattern as a separator: two separators next to each other have an empty part between them, and a separator at the end is followed by an empty part. `lines` treats the line ending as a terminator and also removes a `\r` in front of the `\n`.

### Trimming

The `trim` family removes text from the ends and returns a slice of what remains. No method of this family allocates.

```rust
let raw = "  ## Title ##  ";
let title = raw.trim().trim_matches('#').trim();
assert_eq!(title, "Title");
// `title` is a view into `raw`: it starts at byte 5 of the same buffer.
assert_eq!(title.as_ptr(), raw[5..].as_ptr());

assert_eq!("xxabxx".trim_start_matches('x'), "abxx"); // removes every leading match
assert_eq!("xxabxx".strip_prefix('x'), Some("xabxx")); // removes exactly one
assert_eq!("  ab ".trim_end(), "  ab");
```

`trim`, `trim_start` and `trim_end` remove Unicode whitespace. `trim_matches`, `trim_start_matches` and `trim_end_matches` take a pattern (lesson 03).

### Worked example: the longest word of each line

```rust
let poem = "the quick red\nfox\n\njumps over\n";

let longest: Vec<Option<&str>> = poem
    .lines()
    .map(|line| line.split_whitespace().max_by_key(|word| word.chars().count()))
    .collect();

assert_eq!(longest, [Some("quick"), Some("fox"), None, Some("jumps")]);
```

`lines` keeps the empty line in the middle (it is a line), and for that line `split_whitespace` yields nothing, so the result is `None`. Every `&str` in the result points into `poem`.

### Changing case

`to_uppercase` and `to_lowercase` on a `str` return a **new `String`**, because the result can have a different length:

```rust
assert_eq!("weiß".to_uppercase(), "WEISS");
assert_eq!("weiß".len(), 5);
assert_eq!("WEISS".len(), 5); // same byte length here, but 4 scalars became 5
```

The whole-string methods can look at the neighbours of a letter. Greek has two lowercase forms of sigma: `σ` inside a word and `ς` at the end of a word. `str::to_lowercase` chooses by position. `char::to_lowercase` sees one char at a time and cannot.

```rust
assert_eq!("ΧΑΟΣ".to_lowercase(), "χαος");
assert_eq!('Σ'.to_lowercase().collect::<String>(), "σ");
```

For text that is known to be ASCII (keywords, protocol headers, hexadecimal digits) there are cheaper tools that never allocate:

```rust
assert!("Content-Type".eq_ignore_ascii_case("content-type"));
let mut code = String::from("ab12");
code.make_ascii_uppercase(); // in place
assert_eq!(code, "AB12");
```

### Mistakes people make

**Keeping slices of a temporary `String`.** The slices borrow from the text. If the text is a temporary value, it is freed at the end of the statement.

```rust,compile_fail,E0716
fn read_config() -> String {
    String::from("a=1\nb=2\n")
}

let lines: Vec<&str> = read_config().lines().collect();
println!("{}", lines.len());
```

```text
error[E0716]: temporary value dropped while borrowed
 --> src/main.rs:5:28
  |
5 |     let lines: Vec<&str> = read_config().lines().collect();
  |                            ^^^^^^^^^^^^^                  - temporary value is freed at the end of this statement
  |                            |
  |                            creates a temporary value which is freed while still in use
6 |     println!("{}", lines.len());
  |                    ----- borrow later used here
  |
help: consider using a `let` binding to create a longer lived value
```

Store the `String` in a variable first, then split it.

**`split(' ')` where `split_whitespace()` was meant.** With two spaces between words, `split(' ')` gives an empty word. It also does not split at tabs or line ends.

**Comparing without regard to case by lowercasing both sides.** It allocates two Strings for each comparison, and it is not correct for every language: `"ß"` and `"SS"` do not become equal, and Turkish has a dotted and a dotless i. For ASCII keywords use `eq_ignore_ascii_case`. For names and other human text, case-insensitive matching needs Unicode *case folding* from an external crate.

### Read more

- [`str::lines`](https://doc.rust-lang.org/std/primitive.str.html#method.lines), [`str::split`](https://doc.rust-lang.org/std/primitive.str.html#method.split) and [`str::split_whitespace`](https://doc.rust-lang.org/std/primitive.str.html#method.split_whitespace)
- [`str::to_lowercase`](https://doc.rust-lang.org/std/primitive.str.html#method.to_lowercase)

## Lesson 05 · FromStr and textual conversion

### The idea

Two traits connect text with all other types.

- **`FromStr`** turns text into a value. Its one method is `from_str(text: &str) -> Result<Self, Self::Err>`. You rarely call it by name: `text.parse::<T>()` calls `T::from_str(text)`.
- **`Display`** turns a value into text for people. Every type with `Display` also gets `to_string()`. Lesson 07 shows how to implement it.

Parsing in the standard library is strict. Nothing is skipped and nothing is guessed:

```rust
assert_eq!("17".parse::<u8>(), Ok(17));
assert!("17 ".parse::<u8>().is_err()); // no whitespace
assert!("1_000".parse::<u32>().is_err()); // no digit separators
assert!("0x1f".parse::<u32>().is_err()); // no prefixes
assert_eq!("-17".parse::<i8>(), Ok(-17));
assert_eq!("2.50".parse::<f64>(), Ok(2.5));
assert_eq!("true".parse::<bool>(), Ok(true));
assert_eq!("x".parse::<char>(), Ok('x'));

// Other bases have their own function.
assert_eq!(u8::from_str_radix("ff", 16), Ok(255));
assert_eq!(i64::from_str_radix("-101", 2), Ok(-5));
```

A leading `+` is accepted by every integer type. A leading `-` is accepted only by the signed types.

A failed integer parse returns a `ParseIntError`. Its `kind()` says what went wrong:

```rust
use std::num::IntErrorKind;

let err = "12a".parse::<i32>().unwrap_err();
assert_eq!(err.kind(), &IntErrorKind::InvalidDigit);
assert_eq!(err.to_string(), "invalid digit found in string");
assert_eq!(
    "-200".parse::<i8>().unwrap_err().kind(),
    &IntErrorKind::NegOverflow
);
```

### Worked example: a colour written as `#rrggbb`

```rust
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
struct Rgb {
    red: u8,
    green: u8,
    blue: u8,
}

#[derive(Debug, PartialEq)]
enum RgbError {
    /// The text is not '#' followed by exactly six ASCII characters.
    Shape,
    /// Two of the characters are not a hexadecimal number.
    Digit(ParseIntError),
}

// Lets `?` turn a ParseIntError into an RgbError (Chapter 7).
impl From<ParseIntError> for RgbError {
    fn from(err: ParseIntError) -> Self {
        RgbError::Digit(err)
    }
}

impl FromStr for Rgb {
    type Err = RgbError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let hex = text.strip_prefix('#').ok_or(RgbError::Shape)?;
        // In ASCII text every byte is one scalar, so the ranges below are
        // always char boundaries.
        if hex.len() != 6 || !hex.is_ascii() {
            return Err(RgbError::Shape);
        }
        let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16);
        Ok(Rgb {
            red: channel(0)?,
            green: channel(2)?,
            blue: channel(4)?,
        })
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }
}

let teal: Rgb = "#008080".parse().unwrap();
assert_eq!(teal, Rgb { red: 0, green: 128, blue: 128 });
assert_eq!(teal.to_string(), "#008080");

assert_eq!("008080".parse::<Rgb>(), Err(RgbError::Shape));
assert_eq!("#00808".parse::<Rgb>(), Err(RgbError::Shape));
assert!(matches!("#00zz80".parse::<Rgb>(), Err(RgbError::Digit(_))));

// Parsing accepts capital letters; printing always uses small ones.
assert_eq!("#FFA500".parse::<Rgb>().unwrap().to_string(), "#ffa500");
```

Three points of design are visible here.

1. **Check the shape first, then the values.** The parser first proves that there are six ASCII characters. Only then does it cut the text at fixed byte offsets.
2. **The error type says what went wrong.** A caller can tell "wrong shape" from "bad digit". A real library would also implement `Display` and `std::error::Error` for `RgbError` (Chapter 7).
3. **Printing and parsing are a pair only if you make them one.** `to_string` followed by `parse` gives the same `Rgb` back. The other direction does not give the same text back (`#FFA500` becomes `#ffa500`). Say in the documentation which direction is promised, and test it.

The parser is still more generous than it looks. `from_str_radix` accepts a leading `+`, so one strange input passes:

```rust
assert_eq!(u8::from_str_radix("+f", 16), Ok(15));
```

With the code above, `"#+f+f+f"` is a valid colour. If the format must be exact, test the characters yourself, for example with `hex.bytes().all(|b| b.is_ascii_hexdigit())`. A parser accepts exactly what its code accepts, so write tests for input that should be rejected.

### Mistakes people make

**Parsing text that still has whitespace around it.** A line read from a file or from the keyboard usually ends with `\n`. `"42\n".parse::<u32>()` is an error. Call `trim()` first, if your format allows spaces.

**`unwrap` on input from outside the program.**

```rust,should_panic
let n: u8 = "300".parse().unwrap();
```

```text
thread 'main' (4242) panicked at src/main.rs:2:31:
called `Result::unwrap()` on an `Err` value: ParseIntError { kind: PosOverflow }
```

Return the error to the caller, or map it to an error of your own.

**Using `?` without a conversion between the error types.**

```rust,compile_fail,E0277
#[derive(Debug)]
enum ColorError {
    Shape,
}

fn channel(text: &str) -> Result<u8, ColorError> {
    let value = u8::from_str_radix(text, 16)?;
    Ok(value)
}
```

```text
error[E0277]: `?` couldn't convert the error to `ColorError`
 --> src/main.rs:6:45
  |
5 | fn channel(text: &str) -> Result<u8, ColorError> {
  |                           ---------------------- expected `ColorError` because of this
6 |     let value = u8::from_str_radix(text, 16)?;
  |                 ----------------------------^ the trait `From<ParseIntError>` is not implemented for `ColorError`
  |                 |
  |                 this can't be annotated with `?` because it has type `Result<_, ParseIntError>`
```

Either implement `From<ParseIntError>` for your error type, as the worked example does, or convert at the call: `.map_err(|_| ColorError::Shape)?`.

### A limit of FromStr

`from_str` receives a `&str` and returns `Self`, and the signature has no lifetime that connects the two. So the result cannot borrow from the input: a type that implements `FromStr` must own its data. When you want a parsed view that borrows (for example a struct with `&str` fields), write an ordinary function such as `fn parse(text: &str) -> Result<View<'_>, Error>`.

### Read more

- [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) and [`str::parse`](https://doc.rust-lang.org/std/primitive.str.html#method.parse)
- [`IntErrorKind`](https://doc.rust-lang.org/std/num/enum.IntErrorKind.html)
- [`u32::from_str_radix`](https://doc.rust-lang.org/std/primitive.u32.html#method.from_str_radix)

## Lesson 06 · The formatting language

### The idea

`format!`, `println!`, `write!` and their relatives take a **format string**. The compiler reads it at compile time, so a wrong placeholder is a compile error and not a surprise at run time. Text outside braces is copied. Each `{...}` is a placeholder with two optional parts, separated by a colon: which argument to print, and how.

```text
{ argument : fill align sign # 0 width .precision type }

argument     nothing (the next argument), a number (a position) or a name
align        <  left      ^  centre      >  right       (one fill char may stand before it)
sign         +  print a sign for positive numbers too
#            alternate form: prefix 0x, 0b or 0o; Debug output over several lines
0            fill numbers with zeros, after the sign and the prefix
width        minimum number of scalars; longer output is never cut
.precision   floats: digits after the point; text: maximum number of scalars
type         nothing  Display        ?  Debug          p  pointer
             x X  hexadecimal        b  binary         o  octal        e E  scientific
```

Every part is optional, and the order is fixed.

```rust
// Width and alignment. Text goes left by default, numbers go right.
assert_eq!(format!("{:6}|{:6}|", "ab", 42), "ab    |    42|");
assert_eq!(format!("{:>6}|", "ab"), "    ab|");
assert_eq!(format!("{:^6}|", "ab"), "  ab  |");
assert_eq!(format!("{:*^7}", "ab"), "**ab***");

// Precision.
assert_eq!(format!("{:.3}", 2.0_f64 / 3.0), "0.667");
assert_eq!(format!("{:8.2}|", 3.14159), "    3.14|");
assert_eq!(format!("{:.3}", "kilometre"), "kil");

// Numbers.
assert_eq!(format!("{:+}", 12), "+12");
assert_eq!(format!("{:04}", 42), "0042");
assert_eq!(format!("{:x} {:X} {:o} {:b}", 255, 255, 8, 6), "ff FF 10 110");
assert_eq!(format!("{:#x} {:#010b}", 255, 6), "0xff 0b00000110");
assert_eq!(format!("{:e}", 0.00042), "4.2e-4");

// Debug shows text the way you would write it in source code.
assert_eq!(format!("{:?} {:?}", "hi\n", 'x'), "\"hi\\n\" 'x'");
assert_eq!(format!("{{}}"), "{}"); // doubled braces print one brace
```

In `{:#010b}` the width 10 counts the whole output, including the two characters of the prefix.

### Choosing the argument

```rust
let (city, temp) = ("Oslo", -3.456);

// A variable in scope can be named inside the braces.
assert_eq!(format!("{city}: {temp:.1}"), "Oslo: -3.5");
// Positions and names given in the call.
assert_eq!(format!("{1} {0} {1}", "a", "b"), "b a b");
assert_eq!(format!("{key}={value}", key = "x", value = 1), "x=1");

// Width and precision can come from arguments: a name or position, then `$`.
let (width, digits) = (9, 2);
assert_eq!(format!("{temp:>width$.digits$}|"), "    -3.46|");
assert_eq!(format!("{:>1$}|", "ab", 5), "   ab|");
```

### Worked example: a price list

The column of names must be as wide as the longest name. That is known only at run time, so the width is an argument.

```rust
let items = [("tea", 4.5), ("crème brûlée", 12.0), ("jam", 3.25)];
let width = items
    .iter()
    .map(|(name, _)| name.chars().count())
    .max()
    .unwrap_or(0);

let mut table = String::new();
for (name, price) in items {
    table += &format!("{name:<width$} {price:>6.2}\n");
}

assert_eq!(
    table,
    "tea            4.50\n\
     crème brûlée  12.00\n\
     jam            3.25\n"
);
```

The width is computed with `chars().count()`, because the formatter counts width in scalars, not in bytes. (Lesson 07 shows how to write the rows without a temporary `String` for each one.)

### Three things that surprise people

**Width is a minimum.** A value that is longer than the width is printed in full. Only precision can shorten text.

**Width is not screen width.** The formatter counts scalars. Many East Asian characters and most emoji take two columns in a terminal, and a combining mark takes none:

```rust
assert_eq!(format!("{:>4}|", "日本"), "  日本|"); // 2 + 2 scalars, but 6 columns wide
```

**Rounding uses the value that is really stored.** Most decimal fractions cannot be stored exactly in an `f64`. The formatter rounds the stored binary value, and when that value lies exactly in the middle between two results, it chooses the one whose last digit is even.

```rust
assert_eq!(format!("{:.2}", 2.675), "2.67"); // stored slightly below 2.675
assert_eq!(format!("{:.2}", 0.125), "0.12"); // stored exactly: a tie, 2 is even
assert_eq!(format!("{:.2}", 0.375), "0.38"); // stored exactly: a tie, 8 is even
```

### Mistakes people make

**`{}` for a type without `Display`.**

```rust,compile_fail,E0277
let scores = vec![3, 5];
println!("{}", scores);
```

```text
error[E0277]: `Vec<{integer}>` doesn't implement `std::fmt::Display`
 --> src/main.rs:3:20
  |
3 |     println!("{}", scores);
  |               --   ^^^^^^ `Vec<{integer}>` cannot be formatted with the default formatter
  |               |
  |               required by this formatting parameter
  |
  = help: the trait `std::fmt::Display` is not implemented for `Vec<{integer}>`
  = note: in format strings you may be able to use `{:?}` (or `{:#?}` for pretty-print) instead
```

Collections have only `Debug`, because there is no single correct way to show a list to a user. Use `{:?}` for a developer's view, or format the items yourself.

**An expression inside the braces.** Only a plain variable name can be captured.

```rust,compile_fail
let name = String::from("Ada");
println!("{name.len()}");
```

```text
error: invalid format string: expected `}`, found `.`
 --> src/main.rs:3:20
  |
3 |     println!("{name.len()}");
  |               -    ^ expected `}` in format string
  |               |
  |               because of this opening brace
```

Pass the expression as an argument: `println!("{}", name.len())`.

**Forgetting the `$` after a width name.** Without it, the formatter reads the name as a type letter.

```rust,compile_fail
let width = 8;
println!("[{:>width}]", "ab");
```

```text
error: unknown format trait `width`
 --> src/main.rs:3:19
  |
3 |     println!("[{:>width}]", "ab");
  |                   ^^^^^
```

### Read more

- [`std::fmt`](https://doc.rust-lang.org/std/fmt/index.html): the complete syntax, with examples for every part

## Lesson 07 · Display, fmt::Write, and format_args!

### The idea

When the formatter meets `{}`, it calls the `fmt` method of the `Display` trait on the argument:

```text
fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
```

The `Formatter` is two things in one. It is the **destination** of the output: a `String` that `format!` is filling, the terminal, a file. And it carries the **options** that the caller wrote after the colon, such as width and precision. Your `fmt` writes pieces of text into the formatter, and they go to the destination at once. It does not build a `String` and return it.

`fmt::Result` is `Result<(), fmt::Error>`. `fmt::Error` holds no information. It means one thing: the destination refused the output.

### Worked example: two Display implementations

```rust
use std::fmt;

enum Level {
    Info,
    Warning,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `pad` writes the text and applies the caller's width, fill and alignment
        // (and a precision, which cuts text).
        f.pad(match self {
            Level::Info => "info",
            Level::Warning => "warning",
        })
    }
}

struct Point {
    x: f64,
    y: f64,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Hand the caller's precision on to both numbers.
        match f.precision() {
            Some(digits) => write!(f, "({:.digits$}, {:.digits$})", self.x, self.y),
            None => write!(f, "({}, {})", self.x, self.y),
        }
    }
}

assert_eq!(format!("[{:>8}]", Level::Info), "[    info]");
assert_eq!(format!("[{:<8}]", Level::Warning), "[warning ]");

let p = Point { x: 1.0, y: 2.25 };
assert_eq!(p.to_string(), "(1, 2.25)");
assert_eq!(format!("{p:.1}"), "(1.0, 2.2)");
// `write!` does not look at the caller's width. Only `pad` does.
assert_eq!(format!("[{p:>12}]"), "[(1, 2.25)]");
```

`write!(f, ...)` starts a fresh format string, with its own options. The options of the caller reach your output in three ways only: you read them (`f.width()`, `f.precision()`, `f.alternate()` for `#`, `f.sign_plus()`), you call `f.pad`, or you hand the same formatter to the `fmt` method of an inner value (`fmt::Display::fmt(&self.x, f)`), which then sees the caller's options. Decide which options your type supports, and write that in its documentation.

### Two traits named Write

```text
trait             you implement                    it accepts   implemented by
std::fmt::Write   write_str(&mut self, &str)       text         String, Formatter, your own text sinks
std::io::Write    write(&mut self, &[u8]), flush   bytes        File, TcpStream, Vec<u8>, Stdout
```

`write!(destination, "...", args)` works with both. The macro expands to `destination.write_fmt(format_args!("...", args))`, and both traits provide a `write_fmt` method. The trait must be in scope where you use the macro.

`format_args!` is the piece under all formatting macros. It checks the format string and produces a value of type `fmt::Arguments<'_>`. That value borrows the arguments and knows how to print them, but it has not printed anything yet. A function that takes `fmt::Arguments` lets its caller use the whole formatting language, and the function decides where the text goes.

### Worked example: measure output without storing it

```rust
use std::fmt::{self, Write};

/// A destination that stores nothing. It only counts scalars.
struct Counter(usize);

impl Write for Counter {
    fn write_str(&mut self, piece: &str) -> fmt::Result {
        self.0 += piece.chars().count();
        Ok(())
    }
}

/// How many scalars `args` would print. No String is built.
fn printed_width(args: fmt::Arguments<'_>) -> usize {
    let mut counter = Counter(0);
    counter.write_fmt(args).expect("Counter never refuses output");
    counter.0
}

assert_eq!(printed_width(format_args!("{}°C", 21.5)), 6);
assert_eq!(printed_width(format_args!("{:>8}", "ab")), 8);

// The same macro appends to a String, because String implements fmt::Write.
let mut log = String::new();
write!(log, "{}-{:02}", "run", 7).unwrap();
writeln!(log, " done").unwrap();
assert_eq!(log, "run-07 done\n");
```

The formatting code calls `write_str` many times with small pieces: the parts of the format string between the placeholders, and the text of each argument (sometimes in more than one piece, for example padding first). An implementation of `write_str` may return `Err(fmt::Error)` to refuse output, and whoever is writing must then stop and hand the error to its caller. That is why every write in a `fmt` method ends with `?` or is the final expression.

### Mistakes people make

**`write!` into a `String` without the trait in scope.**

```rust,compile_fail,E0599
let mut out = String::new();
write!(out, "{}-{}", 1, 2).unwrap();
```

```text
error[E0599]: cannot write into `String`
 --> src/main.rs:3:12
  |
3 |     write!(out, "{}-{}", 1, 2).unwrap();
  |            ^^^
...
help: trait `Write` which provides `write_fmt` is implemented but not in scope; perhaps you want to import it
  |
1 + use std::fmt::Write;
```

**Ignoring the result of a write inside `fmt`.** This compiles with a warning, and your type then reports success after a failed write:

```text
warning: unused `Result` that must be used
 --> src/main.rs:5:9
  |
5 |         write!(f, "<{}>", self.0);
  |         ^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: this `Result` may be an `Err` variant, which should be handled
```

Write `write!(f, "<{}>", self.0)?;` or make the `write!` the last expression.

**Returning `fmt::Error` because the data is bad.** The callers of `fmt` expect an error only when the destination failed. `to_string` writes into a `String`, which never fails, so it treats an error as a bug in your type and panics:

```rust,should_panic
use std::fmt;

struct Percent(u8);

impl fmt::Display for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 > 100 {
            return Err(fmt::Error); // wrong: the destination did not fail
        }
        write!(f, "{}%", self.0)
    }
}

let text = Percent(140).to_string();
```

```text
a Display implementation returned an error unexpectedly: Error
```

Check the data where the value is created (in a constructor or in `FromStr`), so that every existing value can be printed.

**Returning `fmt::Arguments` that borrow a local value.**

```rust,compile_fail,E0515
use std::fmt;

fn label<'a>(id: u32) -> fmt::Arguments<'a> {
    let name = format!("item-{id}");
    format_args!("<{name}>")
}
```

```text
error[E0515]: cannot return reference to temporary value
 --> src/main.rs:4:5
  |
4 |     format_args!("<{name}>")
  |     ^^^^^^^^^^^^^^^^^^^^^^^^ returns a reference to data owned by the current function

error[E0515]: cannot return value referencing local variable `name`
 --> src/main.rs:4:5
  |
4 |     format_args!("<{name}>")
  |     ^^^^^^^^^^^^^^^^----^^^^
  |     |               |
  |     |               `name` is borrowed here
  |     returns a value referencing data owned by the current function
```

The compiler reports two things: the macro creates temporary values inside the function, and it borrows `name`. Both end when the function returns. `Arguments` only borrows. Create it in the same expression that uses it, or pass the destination into the function and write there.

### When to use which

- Implement `Display` for text that a user reads. Derive `Debug` for developers.
- A function that produces text can take `out: &mut impl fmt::Write` and write into it. The caller chooses a `String`, a formatter, or a counting sink, and no temporary `String` is needed.
- A function that receives a ready message can take `fmt::Arguments<'_>`.
- Use `format!` when you really need a `String` as the result.

### Read more

- [`fmt::Display`](https://doc.rust-lang.org/std/fmt/trait.Display.html) and [`fmt::Formatter`](https://doc.rust-lang.org/std/fmt/struct.Formatter.html)
- [`fmt::Write`](https://doc.rust-lang.org/std/fmt/trait.Write.html) and [`fmt::Arguments`](https://doc.rust-lang.org/std/fmt/struct.Arguments.html)
- [`format_args!`](https://doc.rust-lang.org/std/macro.format_args.html)

## Lesson 08 · UTF-8, borrowed text views, and delayed allocation

### The idea

Text and bytes have the same shape in memory. A `String` has the layout of a `Vec<u8>`, and a `&str` has the layout of a `&[u8]`. The only difference is the promise of valid UTF-8. So a conversion from text to bytes is free, and a conversion from bytes to text needs one check and can fail.

```text
from       to          call                              cost
&str       &[u8]       text.as_bytes()                   free
String     Vec<u8>     text.into_bytes()                 free; the same buffer
&[u8]      &str        str::from_utf8(bytes)             a check; Err(Utf8Error)
Vec<u8>    String      String::from_utf8(vec)            a check; the same buffer; the error returns the Vec
&[u8]      Cow<str>    String::from_utf8_lossy(bytes)    a check; allocates only to repair
```

```rust
let bytes = vec![0x52, 0x75, 0xFF, 0x74]; // 'R', 'u', an invalid byte, 't'

let err = String::from_utf8(bytes).unwrap_err();
assert_eq!(err.utf8_error().valid_up_to(), 2); // the first two bytes are good text

// The error owns the rejected Vec and gives it back. Nothing was copied or lost.
let bytes = err.into_bytes();
assert_eq!(bytes, [0x52, 0x75, 0xFF, 0x74]);

assert_eq!(str::from_utf8(&bytes[..2]), Ok("Ru"));

// The lossy conversion puts U+FFFD, the replacement character, where bytes are bad.
assert_eq!(String::from_utf8_lossy(&bytes), "Ru\u{FFFD}t");
```

Choose the strict conversion when bad bytes must be reported (a configuration file, a protocol message). Choose the lossy conversion only for display, such as a log line, and only when it is acceptable that the original bytes cannot be recovered from the result.

### Cow: borrow now, own only if needed

`from_utf8_lossy` has a problem to solve. For valid input, the best result is a `&str` that borrows the input. For invalid input, it must build a new `String`. One return type must cover both. That type is `Cow<'a, str>`, an enum from `std::borrow`:

```text
enum Cow<'a, str> {
    Borrowed(&'a str),   // points at text that someone else owns
    Owned(String),       // owns its text
}
```

You can use a `Cow<str>` like a `&str`: it has all the methods of `str`, and it can be compared and printed. The name stands for "clone on write".

### Worked example: a function that usually changes nothing

```rust
use std::borrow::Cow;

/// Replaces each tab with four spaces. Text without a tab is returned as it is.
fn expand_tabs(line: &str) -> Cow<'_, str> {
    if line.contains('\t') {
        Cow::Owned(line.replace('\t', "    "))
    } else {
        Cow::Borrowed(line)
    }
}

let plain = expand_tabs("no tabs here");
assert!(matches!(plain, Cow::Borrowed(_))); // no allocation

let changed = expand_tabs("a\tb");
assert!(matches!(changed, Cow::Owned(_)));
assert_eq!(changed, "a    b");
assert_eq!(changed.len(), 6); // str methods work on a Cow

// `into_owned` gives a String. It copies only if the Cow was borrowed.
let owned: String = changed.into_owned();
assert_eq!(owned, "a    b");
```

If most lines have no tab, most calls allocate nothing. A caller that needs to edit the result calls `to_mut()`, which gives a `&mut String` under the same rule as `into_owned`: the copy happens only if it has not happened before.

### Building one text from many pieces

```rust
let from_chars: String = ['o', 'k'].iter().collect();
let from_slices: String = ["ab", "cd"].into_iter().collect();
assert_eq!((from_chars.as_str(), from_slices.as_str()), ("ok", "abcd"));

let mut path = String::from("usr");
path.extend(["/", "lib"]);
assert_eq!(path, "usr/lib");

assert_eq!(["a", "b"].concat(), "ab");
assert_eq!(["a", "b", "c"].join("-"), "a-b-c");
```

All of these read borrowed pieces and allocate only the result. A common waste is to turn every piece into its own `String` first and then join the Strings.

### Text-like types that are not UTF-8

The operating system does not promise UTF-8. A file name on Linux is almost any sequence of bytes, and on Windows it is a sequence of 16-bit units that need not be valid Unicode. Rust has separate types for such data: `OsStr` and `OsString` for text from the operating system, `Path` and `PathBuf` for file paths (Chapter 18), `CStr` and `CString` for text shared with C code (Chapter 24). Converting them to `&str` can fail, so the conversion does not return a plain `&str`: `to_str()` gives an `Option<&str>` for `OsStr` and `Path`, and a `Result` for `CStr`.

```rust
use std::path::Path;

let path = Path::new("notes/día.txt");
assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("txt"));
assert_eq!(path.to_string_lossy(), "notes/día.txt"); // a Cow<str>, for display
```

Keep a path as a `Path` while you work with it. Convert it to text only to show it to a person, with `display()` or `to_string_lossy()`.

### Mistakes people make

**`unwrap` on bytes that come from outside.**

```rust,should_panic
let bytes = vec![0x52, 0x75, 0xFF, 0x74];
let text = String::from_utf8(bytes).unwrap();
```

```text
thread 'main' (4242) panicked at src/main.rs:3:41:
called `Result::unwrap()` on an `Err` value: FromUtf8Error { bytes: [82, 117, 255, 116], error: Utf8Error { valid_up_to: 2, error_len: Some(1) } }
```

**Returning a plain `&str` or `String` where a `Cow` is expected.**

```rust,compile_fail,E0308
use std::borrow::Cow;

fn tidy(input: &str) -> Cow<'_, str> {
    if input.contains('\t') {
        input.replace('\t', " ")
    } else {
        input
    }
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:4:9
  |
2 | fn tidy(input: &str) -> Cow<'_, str> {
  |                         ------------ expected `Cow<'_, str>` because of return type
3 |     if input.contains('\t') {
4 |         input.replace('\t', " ")
  |         ^^^^^^^^^^^^^^^^^^^^^^^^ expected `Cow<'_, str>`, found `String`
...
help: call `Into::into` on this expression to convert `String` into `Cow<'_, str>`
```

The compiler prints a second error of the same kind for the `input` branch (expected `Cow<'_, str>`, found `&str`). Wrap each branch: `Cow::Owned(...)` and `Cow::Borrowed(...)`, or call `.into()` on both.

**Calling `.to_string()` on a `Cow` at once.** That always allocates, so the `Cow` gave no benefit. Use the `Cow` as a `&str` for as long as you only read it.

**`path.to_str().unwrap()`.** It works on your machine and panics on the first file name that is not valid Unicode.

### Read more

- [`String::from_utf8`](https://doc.rust-lang.org/std/string/struct.String.html#method.from_utf8) and [`String::from_utf8_lossy`](https://doc.rust-lang.org/std/string/struct.String.html#method.from_utf8_lossy)
- [`Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html)
- [`OsStr`](https://doc.rust-lang.org/std/ffi/struct.OsStr.html) and [`Path`](https://doc.rust-lang.org/std/path/struct.Path.html)

## Lesson 09 · Capstone: a Unicode-aware report parser

The capstone has no new idea. It asks you to combine lessons 03, 04, 05 and 07 into one small program with three functions: parse one line, read a whole text, print a report. This section describes the approach. It does not show the solution.

### Think in stages

```text
whole text
   │  split into lines, remember each line's number
   ▼
one line ──► skip it?  (blank, or a comment)
   │
   ▼
parse one record ──► Ok(record)   → list of records
                 └─► Err(reason)  → list of rejected line numbers
   ▼
records ──► one output String
```

Write and test the stages one at a time, in the order of the tasks. The function for one line knows nothing about line numbers or comments. The function for the whole text knows nothing about `=`. When each function has one job, each test failure points at one place.

### Number first, filter second

A line number in a report must be the number that the user sees in an editor. So the lines must be numbered before any of them is left out. The order of two iterator adapters decides this. The example below is a different job from the capstone: it lists the lines of a text that contain the word `TODO`.

```rust
let source = "fn main() {\n    // TODO: read input\n    run();\n    // TODO: print result\n}\n";

let todos: Vec<(usize, &str)> = source
    .lines()
    .enumerate()
    .map(|(index, line)| (index + 1, line.trim())) // people count lines from 1
    .filter(|(_, line)| line.contains("TODO"))
    .collect();

assert_eq!(
    todos,
    [(2, "// TODO: read input"), (4, "// TODO: print result")]
);
```

With `filter` before `enumerate`, the pairs would be numbered 1 and 2: the position among the kept lines, which helps nobody. A `for` loop over `text.lines().enumerate()` with `continue` for the lines to skip follows the same rule and is just as good.

### Decisions to make before you write code

- **Where is whitespace allowed?** The contract says where. Find each point in your code where `trim` belongs: on the whole line, on each part, or both.
- **What does "exactly one separator" mean for your splitting method?** Find out what the method returns for no separator, for one, and for two, and which of these cases a later step already rejects.
- **Who owns the result?** The records are returned to the caller and may live longer than the input text. Decide what that means for the type of the name field.
- **Which count does the report need?** The contract says scalars. A name with a combining mark has more scalars than visible characters. That difference is accepted here.
- **How do you build the output?** One `String`, written row by row. Lesson 07 showed how to write formatted text into a `String` without a temporary `String` for each row.

### Why byte offsets are safe here

The separators of this format (`=`, `#`, the line ending) are ASCII. In UTF-8, a byte below `0x80` is always a complete ASCII scalar: the bytes of longer scalars are all `0x80` or above. So an offset found by searching for an ASCII separator can never be inside another character. This property is one reason why UTF-8 is easy to parse.

### What is left out

The parser does not normalize names (two spellings of ö stay different names), does not count grapheme clusters, and does not align columns for a terminal. A program for real users would need crates from outside the standard library for these. It would also report *why* a line was rejected, not only where; the review lesson R01 practises an error type that does this.

### Read more

- [`Iterator::enumerate`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.enumerate)
- [`str::split_once`](https://doc.rust-lang.org/std/primitive.str.html#method.split_once)
- [`std::fmt::Write`](https://doc.rust-lang.org/std/fmt/trait.Write.html)
