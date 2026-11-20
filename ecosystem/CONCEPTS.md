# Production crates: concepts

[Track guide](README.md) · [Hints](HINTS.md) · [Tooling workday](TOOLING.md) · [All chapters](../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson: what the crate is for, how to think about it, a
worked example, the mistakes people really make, and how to choose between
alternatives.

Every Rust code block in this file is compiled against the pinned crate
versions, and run where it can run, by:

```sh
cargo test --manifest-path ecosystem/Cargo.toml --doc
```

Every compiler message was copied from the pinned compiler (Rust 1.99.0,
edition 2024) and shortened: a line with `...` marks a cut, and the end of a
message is often left out. The worked examples never solve the lesson's tasks.
They use the same tools on a different problem.

## What this track gives you

After this track you can:

- design the error types of a library, and report errors well in an application;
- give a type a stable JSON form with serde, and keep the type's rules when
  data comes in from outside;
- build a command-line interface with help text, validation, and the right
  exit codes;
- add structured logs that carry context, and check them in tests;
- find bugs with generated inputs, and read a shrunk counterexample;
- write async code on tokio that limits concurrency, applies backpressure, and
  loses nothing when a future is cancelled;
- build, test, and cleanly stop a small HTTP service, and write a client for it;
- use Cargo's own tools to document, lint, measure, and profile your code.

**What you need first.** Each section names the chapters it builds on. For the
whole track you need error handling with `Result` and `?` (Chapter 7), traits
and generics (Chapter 11), and closures (Chapter 14). E06 and E09 also need
async functions and futures (Chapters 20 and 21).

**How a crate lesson differs from a chapter lesson.** A crate is someone
else's code with its own words. For each crate, first learn its three or four
central nouns (for serde: *data model*, *format*, *attribute*). After that,
most of the API is detail you look up. Keep the crate's documentation open
while you work; that is how the job is done in practice. The "Read more" links
point to the documentation of the exact versions pinned here.

**Time.** These are estimates for someone who has finished the chapters named
in each section. They include reading this file, the lesson, and writing your
explanation.

| Lesson | Estimate |
| --- | --- |
| E01 · Library and application errors | 2–3 hours |
| E02 · serde | 3–4 hours |
| E03 · Command-line interfaces | 2–3 hours |
| E04 · Structured logging | 2–3 hours |
| E05 · Property-based testing | 4–5 hours |
| E06 · Async services | 5–6 hours |
| E07 · Capstone: `jobq` | 6–8 hours |
| E08 · Tooling workday | 4–6 hours |
| E09 · An HTTP JSON service and its client | 5–6 hours |

## Lesson E01 · Library and application errors

**Builds on:** Chapter 7 (`Result`, `?`, the `std::error::Error` trait, `From`
conversions).

### The mental model

Error-handling code has two kinds of reader.

A **library** is called by other code. Its caller wants to *decide* something:
try again, use a default, tell the user which line is wrong. So a library
returns an error type that the caller can `match` on. Usually this is an enum
with one variant for each decision a caller might take.

An **application** talks to a person. Near the top of the program it mostly
wants to *report*: what failed, and what the program was doing at the time. So
an application wants one error type that can hold any error, and a way to add
a sentence of context at each level.

Two crates serve these two needs:

- **thiserror** is a derive macro. You write the enum. It writes the
  `Display`, `std::error::Error`, and `From` impls that you would otherwise
  write by hand. Nothing from thiserror appears in your public API.
- **anyhow** gives you `anyhow::Error`, a box that can hold any error type, and
  the alias `anyhow::Result<T>`. Its `context` method wraps an error in a new
  layer with a message of your own.

Both build on the *source chain* from Chapter 7: an error can point to the
error that caused it, through `Error::source()`. Picture the error from the
worked example below:

```text
anyhow::Error
└─ "reading 2 of sensor roof"            context, added by the application
   └─ ReadingError::BadNumber            the library's own type
      "invalid number in `2o.5C`"
      └─ ParseFloatError                 from the standard library
         "invalid float literal"
```

Each layer describes only itself. The code that prints the error decides how
much of the chain to show. With anyhow, `{}` prints the top layer and `{:#}`
prints every layer, joined with `: `.

### Worked example: temperature readings

```rust
use anyhow::Context as _;
use std::error::Error as _;
use std::num::ParseFloatError;

/// The library layer: every way a reading can be wrong has a name.
#[derive(Debug, thiserror::Error)]
enum ReadingError {
    #[error("empty reading")]
    Empty,
    #[error("invalid number in `{text}`")]
    BadNumber {
        text: String,
        #[source]
        cause: ParseFloatError,
    },
    #[error("unknown unit `{0}` (use C or F)")]
    UnknownUnit(char),
}

/// Parses `21.5C` or `70F` into degrees Celsius.
fn parse_reading(text: &str) -> Result<f64, ReadingError> {
    let unit = text.chars().last().ok_or(ReadingError::Empty)?;
    let digits = &text[..text.len() - unit.len_utf8()];
    let value: f64 = digits.parse().map_err(|cause| ReadingError::BadNumber {
        text: text.to_owned(),
        cause,
    })?;
    match unit {
        'C' => Ok(value),
        'F' => Ok((value - 32.0) / 1.8),
        other => Err(ReadingError::UnknownUnit(other)),
    }
}

/// The application layer: no new error type, only context for the reader.
fn average(sensor: &str, readings: &[&str]) -> anyhow::Result<f64> {
    let mut sum = 0.0;
    for (index, text) in readings.iter().enumerate() {
        sum += parse_reading(text)
            .with_context(|| format!("reading {} of sensor {sensor}", index + 1))?;
    }
    Ok(sum / readings.len() as f64)
}

fn main() {
    assert_eq!(average("roof", &["20C", "68F"]).unwrap(), 20.0);

    // A caller of the library can match on the variant and decide what to do.
    assert!(matches!(parse_reading("20K"), Err(ReadingError::UnknownUnit('K'))));

    // The application reports. `{}` shows the outermost layer, `{:#}` the chain.
    let err = average("roof", &["20C", "2o.5C"]).unwrap_err();
    assert_eq!(err.to_string(), "reading 2 of sensor roof");
    assert_eq!(
        format!("{err:#}"),
        "reading 2 of sensor roof: invalid number in `2o.5C`: invalid float literal"
    );

    // The typed error is still inside, so code can still make decisions.
    let inner = err.downcast_ref::<ReadingError>().expect("a ReadingError");
    assert!(inner.source().is_some());
}
```

Three attributes do the work inside the enum:

- `#[error("...")]` is the Display text. It can name fields: `{text}` for a
  named field, `{0}` for the first field of a tuple variant.
- `#[source]` marks the field that `Error::source()` returns.
- `#[from]` (not used above) also marks the source, and in addition generates
  `impl From<TheSourceType> for YourError`, which is what `?` needs.

### Mistakes people make

**1. Using `?` when no `From` impl exists.** The `?` operator converts the
error with `From`. If you have not told the compiler how, it says so:

```rust,compile_fail,E0277
use std::num::ParseFloatError;

#[derive(Debug, thiserror::Error)]
enum ReadingError {
    #[error("empty reading")]
    Empty,
    #[error("invalid number")]
    BadNumber(#[source] ParseFloatError),
}

fn parse(text: &str) -> Result<f64, ReadingError> {
    if text.is_empty() {
        return Err(ReadingError::Empty);
    }
    let value = text.parse::<f64>()?;
    Ok(value)
}

fn main() {
    println!("{:?}", parse("21.5"));
}
```

```text
error[E0277]: `?` couldn't convert the error to `ReadingError`
  --> src/main.rs:15:36
   |
11 | fn parse(text: &str) -> Result<f64, ReadingError> {
   |                         ------------------------- expected `ReadingError` because of this
...
15 |     let value = text.parse::<f64>()?;
   |                      --------------^ the trait `From<ParseFloatError>` is not implemented for `ReadingError`
   |                      |
   |                      this can't be annotated with `?` because it has type `Result<_, ParseFloatError>`
```

There are two fixes. Write `BadNumber(#[from] ParseFloatError)` when *every*
`ParseFloatError` in this module should become that variant. Use
`.map_err(...)` when the variant needs more data than the source (as `text`
does in the worked example), or when the same source type can mean two
different variants.

**2. Repeating the source inside the message.** It looks helpful to put the
cause into the Display text. Then every reporter that prints the chain prints
the cause twice:

```rust
use std::num::ParseIntError;

#[derive(Debug, thiserror::Error)]
enum Repeats {
    #[error("invalid age: {0}")] // the message includes the source...
    BadAge(#[source] ParseIntError), // ...and the chain includes it again
}

#[derive(Debug, thiserror::Error)]
enum Clean {
    #[error("invalid age")]
    BadAge(#[source] ParseIntError),
}

let cause = "x".parse::<u8>().unwrap_err();
let repeats = anyhow::Error::new(Repeats::BadAge(cause.clone())).context("reading form");
let clean = anyhow::Error::new(Clean::BadAge(cause)).context("reading form");
assert_eq!(
    format!("{repeats:#}"),
    "reading form: invalid age: invalid digit found in string: invalid digit found in string"
);
assert_eq!(
    format!("{clean:#}"),
    "reading form: invalid age: invalid digit found in string"
);
```

The rule: a message describes its own layer and nothing else.

**3. Calling `context` without importing the trait.** `context` and
`with_context` are methods of the trait `anyhow::Context`. A trait's methods
can only be called when the trait is in scope:

```rust,compile_fail,E0599
fn read_port(text: &str) -> anyhow::Result<u16> {
    let port = text.trim().parse::<u16>().context("reading the port")?;
    Ok(port)
}

fn main() {
    println!("{:?}", read_port("80"));
}
```

```text
error[E0599]: no method named `context` found for enum `Result<T, E>` in the current scope
   --> src/main.rs:2:43
    |
  2 |     let port = text.trim().parse::<u16>().context("reading the port")?;
    |                                           ^^^^^^^
...
    = help: items from traits can only be used if the trait is in scope
help: trait `Context` which provides `context` is implemented but not in scope; perhaps you want to import it
    |
  1 + use anyhow::Context;
```

Add `use anyhow::Context as _;`. The `as _` form brings the methods into scope
without adding the name `Context` to your module.

### When to use which

| Situation | Choose |
| --- | --- |
| A public function of a library | An error enum (thiserror, or impls written by hand) |
| A binary, a test, glue code between libraries | `anyhow::Result` with `context` at each level |
| An enum with two or three variants, and you want no dependency | Hand-written `Display` and `Error` impls |
| A variant that no caller would ever match on | Do not add it; fold it into a more general variant |

Never put `anyhow::Error` in a public library signature. Callers cannot
`match` on it; they can only try to downcast, and your signature does not tell
them to what.

Use `context("...")` for a fixed message. Use `with_context(|| format!(...))`
when building the message costs something: the closure runs only on the error
path.

### Read more

- [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html), especially `source`
- [The `?` operator](https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator) in the Rust Reference
- [thiserror 2.0.21](https://docs.rs/thiserror/2.0.21/thiserror/) and [anyhow 1.0.104](https://docs.rs/anyhow/1.0.104/anyhow/)

## Lesson E02 · serde

**Builds on:** Chapter 11 (traits and derive), Chapter 13 (`From`, `TryFrom`,
`Cow`), and lifetimes from Chapter 5.

### The mental model

To *serialize* a value is to turn it into bytes or text that can leave your
program. To *deserialize* is the way back. serde splits this job in two, with
a small **data model** in the middle:

```text
your types                     serde data model                formats
──────────                     ────────────────                ───────
struct Track      Serialize    bool, integers, floats,         serde_json
enum Source     ───────────►   strings, bytes, option,   ───►  toml
Percent         ◄───────────   seq, map, struct,         ◄───  postcard
                 Deserialize   the enum variant kinds          ...
```

- Your type implements `Serialize` and `Deserialize`. These impls say how the
  type maps to the data model: "I am a struct with these three fields."
  `#[derive(Serialize, Deserialize)]` writes them for you.
- A **format** crate, such as serde_json, maps the data model to bytes.
- **Attributes** (`#[serde(...)]`) adjust the generated impls: other field
  names, defaults, a different shape for an enum.

So the types know nothing about JSON, and serde_json knows nothing about your
types. That is why one derive works with every format.

The text or bytes that you produce are read by other programs and by later
versions of your own program. Treat that shape as a public API. The
attributes are where you design it.

### Worked example: a playlist entry

```rust
use serde::{Deserialize, Serialize};

/// A volume level from 0 to 100. The field is private, so `try_from` is the only way in.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
struct Percent(u8);

impl TryFrom<u8> for Percent {
    type Error = String;
    fn try_from(value: u8) -> Result<Percent, String> {
        if value <= 100 {
            Ok(Percent(value))
        } else {
            Err(format!("{value} is more than 100 percent"))
        }
    }
}

impl From<Percent> for u8 {
    fn from(percent: Percent) -> u8 {
        percent.0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Track {
    title: String,
    // Old files have no volume: fall back to `default_volume()`.
    #[serde(default = "default_volume")]
    volume: Percent,
    // `None` is left out of the output instead of being written as `null`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cover_art: Option<String>,
    source: Source,
}

fn default_volume() -> Percent {
    Percent(80)
}

/// Adjacently tagged: the variant name and its data sit in two named fields.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "at", rename_all = "lowercase")]
enum Source {
    File(String),
    Stream { url: String, bitrate: u32 },
}

fn main() {
    let text = r#"{"title":"Dawn","source":{"kind":"file","at":"dawn.flac"}}"#;
    let track: Track = serde_json::from_str(text).unwrap();
    assert_eq!(track.volume, Percent(80));
    assert_eq!(track.cover_art, None);
    assert_eq!(track.source, Source::File("dawn.flac".into()));

    // Writing it back adds the defaulted field and still leaves out the `None`.
    assert_eq!(
        serde_json::to_string(&track).unwrap(),
        r#"{"title":"Dawn","volume":80,"source":{"kind":"file","at":"dawn.flac"}}"#
    );

    // The derived code went through `try_from`, so the invariant holds for input too.
    let loud = r#"{"title":"Noise","volume":140,"source":{"kind":"file","at":"n.flac"}}"#;
    let err = serde_json::from_str::<Track>(loud).unwrap_err();
    assert_eq!(err.to_string(), "140 is more than 100 percent at line 1 column 29");
}
```

Things to notice:

- A field of type `Option<T>` may be missing in the input. Every other field is
  required unless it has `#[serde(default)]`.
- `#[serde(try_from = "u8")]` tells the derive: first deserialize a `u8`, then
  call `Percent::try_from`. A plain derive on `Percent` would fill the field
  directly and accept 140. **A derived `Deserialize` never calls your
  constructor.** Any type with a rule ("at most 100", "start before end") needs
  `try_from` or a hand-written impl.
- The enum attribute chooses one of four shapes. For a variant
  `Stream { url: "u", bitrate: 1 }` with no renaming:

| Representation | Attribute | JSON |
| --- | --- | --- |
| Externally tagged (the default) | none | `{"Stream":{"url":"u","bitrate":1}}` |
| Internally tagged | `tag = "kind"` | `{"kind":"Stream","url":"u","bitrate":1}` |
| Adjacently tagged | `tag = "kind", content = "at"` | `{"kind":"Stream","at":{"url":"u","bitrate":1}}` |
| Untagged | `untagged` | `{"url":"u","bitrate":1}`; when reading, serde tries each variant in order |

  An internally tagged enum puts the tag *among the variant's own fields*, so
  the variant's data must itself be a set of named fields: a struct variant, a
  newtype variant that wraps a struct or a map, or a unit variant, which has no
  data at all. `File(String)` holds a bare string, with no place for the tag.
  With `tag = "kind"` alone, serde_json refuses to write `Source::File`:
  "cannot serialize tagged newtype variant Source::File containing a string".
  That is why the worked example uses the adjacent form.

### Mistakes people make

**1. Deserializing into `&str`.** A `&str` field does not copy: it points into
the input. That only works when the text is present in the input exactly as it
should appear in the string. A JSON escape such as `\n` breaks this, because
the decoded text exists nowhere in the input:

```rust
let plain: &str = serde_json::from_str(r#""tea""#).unwrap();
assert_eq!(plain, "tea");

let escaped: Result<&str, _> = serde_json::from_str(r#""tea\ntime""#);
assert_eq!(
    escaped.unwrap_err().to_string(),
    r#"invalid type: string "tea\ntime", expected a borrowed string at line 1 column 11"#
);

let owned: String = serde_json::from_str(r#""tea\ntime""#).unwrap();
assert_eq!(owned, "tea\ntime");
```

Use `String` unless you have measured that the copies matter. When they do,
use `Cow<'a, str>` with `#[serde(borrow)]`: it borrows when it can and makes
an owned copy when it must.

**2. Asking for `Deserialize<'a>` when you mean "owns its data".**
`Deserialize<'de>` has a lifetime because the result may borrow from the
input, as in mistake 1. A function that reads a file into a local `String`
cannot return something that borrows from that `String`:

```rust,compile_fail,E0597
use serde::Deserialize;

fn load<'a, T: Deserialize<'a>>(path: &str) -> T {
    let text = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn main() {
    let limit: u32 = load("limit.json");
    println!("{limit}");
}
```

```text
error[E0597]: `text` does not live long enough
    --> src/main.rs:5:26
     |
   3 | fn load<'a, T: Deserialize<'a>>(path: &str) -> T {
     |         -- lifetime `'a` defined here
   4 |     let text = std::fs::read_to_string(path).unwrap();
     |         ---- binding `text` declared here
   5 |     serde_json::from_str(&text).unwrap()
     |     ---------------------^^^^^-
     |     |                    |
     |     |                    borrowed value does not live long enough
     |     argument requires that `text` is borrowed for `'a`
   6 | }
     | - `text` dropped here while still borrowed
```

The bound you want is `T: serde::de::DeserializeOwned`. It means "can be
deserialized from input of any lifetime", so the result cannot borrow from the
input.

**3. Expecting a missing field to become a default.** Only `Option` fields are
optional by themselves:

```rust
#[derive(Debug, serde::Deserialize)]
struct Reading {
    value: f64,
    unit: String,
    note: Option<String>,
}

let ok: Reading = serde_json::from_str(r#"{"value":1.5,"unit":"C"}"#).unwrap();
assert_eq!((ok.value, ok.unit.as_str(), ok.note), (1.5, "C", None));

let err = serde_json::from_str::<Reading>(r#"{"value":1.5}"#).unwrap_err();
assert_eq!(err.to_string(), "missing field `unit` at line 1 column 13");
```

Add `#[serde(default)]` (the type's `Default`) or
`#[serde(default = "function_name")]` to a field that old inputs may lack.

### When to use which

| Question | Guidance |
| --- | --- |
| Unknown fields in the input | Reject them (`deny_unknown_fields`) in config files written by people: a typo should fail loudly. Accept them in messages between programs: an old reader must survive a newer writer. |
| A type with a rule | `try_from = "..."` when the rule is "read a simpler type, then check". A hand-written `Deserialize` when you accept several input shapes or need your own error texts. |
| `String` or borrowed `&str` / `Cow` | `String` by default. Borrow on hot paths that you have measured. |
| Which enum shape | Internally tagged for "objects with a `type` field", the most common shape in JSON APIs. Adjacently tagged when variants hold values that are not objects. Avoid untagged when you can: its error messages cannot say which variant was meant. |

### Read more

- [The serde guide](https://serde.rs/), especially [container, variant, and field attributes](https://serde.rs/attributes.html), [enum representations](https://serde.rs/enum-representations.html), and [deserializer lifetimes](https://serde.rs/lifetimes.html)
- [serde_json 1.0.154](https://docs.rs/serde_json/1.0.154/serde_json/)
- [`std::borrow::Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html) and [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)

## Lesson E03 · Command-line interfaces

**Builds on:** Chapter 18 (arguments, standard streams, exit codes) and derive
macros from Chapter 11.

### The mental model

A command line is a small language: a program name, then *flags* (`--overwrite`),
*options* that take a value (`--quality 85`), *positional* arguments (`a.jpg`),
and sometimes *subcommands* (`git commit`). clap turns that language into a
value of a type that you define.

With clap's derive API, the type *is* the definition:

```text
argv: ["shrink", "--size", "800x600", "-q", "70", "a.jpg"]
                      │
                      ▼   Cli::try_parse_from(argv)
        ┌─────────────┴──────────────┐
        ▼                            ▼
   Ok(Cli { size: (800, 600),   Err(clap::Error)
            quality: 70,          kind       what went wrong, or "show help"
            inputs: ["a.jpg"],    message    ready to print
            .. })                 exit_code  0 for help/version, 2 for misuse
```

- Each field is one argument. Its type decides its behaviour (see the table
  under "When to use which").
- A doc comment on a field becomes its help text.
- A **value parser** turns one argument string into the field's type and can
  refuse it. Refusing here is better than checking later: the error names the
  flag, and the rest of the program never sees an invalid `Cli`.

There are two ways to run the parser. `Cli::parse()` is for `main`: on a
problem it prints the message and ends the process. `Cli::try_parse_from(args)`
returns a `Result` and touches nothing global, so it is the one you test.

### Worked example: an image resizer

```rust
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, ValueEnum};
use std::path::PathBuf;

/// Resize images.
#[derive(Debug, Parser)]
#[command(name = "shrink", version = "0.3.0")]
struct Cli {
    /// Target size, as WIDTHxHEIGHT
    #[arg(long, value_parser = parse_size)]
    size: (u32, u32),
    /// JPEG quality
    #[arg(short, long, default_value_t = 85, value_parser = clap::value_parser!(u8).range(1..=100))]
    quality: u8,
    /// How to scale
    #[arg(long, value_enum, default_value_t = Filter::Smooth)]
    filter: Filter,
    /// Replace existing files
    #[arg(long)]
    overwrite: bool,
    /// Images to resize
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Filter {
    Nearest,
    Smooth,
}

/// A value parser is a plain function from `&str` to `Result`.
fn parse_size(text: &str) -> Result<(u32, u32), String> {
    let (width, height) = text
        .split_once('x')
        .ok_or_else(|| format!("`{text}` is not WIDTHxHEIGHT"))?;
    match (width.parse(), height.parse()) {
        (Ok(width), Ok(height)) => Ok((width, height)),
        _ => Err(format!("`{text}` is not WIDTHxHEIGHT")),
    }
}

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    // The first item is the program name, as in a real argv.
    Cli::try_parse_from(std::iter::once("shrink").chain(args.iter().copied()))
}

fn main() {
    // Checks the definition itself: duplicate flags, bad defaults, and so on.
    Cli::command().debug_assert();

    let cli = parse(&["--size", "800x600", "a.jpg", "b.jpg"]).unwrap();
    assert_eq!((cli.size, cli.quality, cli.filter), ((800, 600), 85, Filter::Smooth));
    assert_eq!(cli.inputs.len(), 2);
    assert!(!cli.overwrite);

    // Every problem is a `clap::Error` with a kind, a message, and an exit code.
    let err = parse(&["--size", "800x600", "-q", "0", "a.jpg"]).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::ValueValidation);
    assert_eq!(err.exit_code(), 2);
    let err = parse(&["--size", "big", "a.jpg"]).unwrap_err();
    assert!(err.to_string().contains("`big` is not WIDTHxHEIGHT"));
    let err = parse(&["--size", "1x1"]).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);

    // `--help` is also an "error" for `try_parse_from`, with exit code 0.
    let help = parse(&["--help"]).unwrap_err();
    assert_eq!((help.kind(), help.exit_code()), (ErrorKind::DisplayHelp, 0));
    println!("{help}");
    println!("-----");
    println!("{}", parse(&["--size", "800x600", "-q", "0", "a.jpg"]).unwrap_err());
}
```

The two `println!` calls at the end print this:

```text
Resize images

Usage: shrink [OPTIONS] --size <SIZE> <INPUTS>...

Arguments:
  <INPUTS>...  Images to resize

Options:
      --size <SIZE>        Target size, as WIDTHxHEIGHT
  -q, --quality <QUALITY>  JPEG quality [default: 85]
      --filter <FILTER>    How to scale [default: smooth] [possible values: nearest, smooth]
      --overwrite          Replace existing files
  -h, --help               Print help
  -V, --version            Print version

-----
error: invalid value '0' for '--quality <QUALITY>': 0 is not in 1..=100

For more information, try '--help'.
```

You wrote none of that text by hand. It comes from the field names, the doc
comments, the defaults, and the value parsers.

### Mistakes people make

**1. A field type that clap cannot parse.** clap knows how to parse a type
when it implements `FromStr` or `ValueEnum` (and a few more). For your own
type, the message is long and hard to read:

```rust,compile_fail,E0599
use clap::Parser;

#[derive(Debug, Clone)]
struct Size {
    width: u32,
    height: u32,
}

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long)]
    size: Size,
}

fn main() {
    println!("{:?}", Cli::parse());
}
```

```text
error[E0599]: the method `value_parser` exists for reference `&&&&&&clap::builder::_infer_ValueParser_for<Size>`, but its trait bounds were not satisfied
    --> src/main.rs:11:5
     |
   4 | struct Size {
     | ----------- doesn't satisfy 7 bounds
...
  11 |     #[arg(long)]
     |     ^ method cannot be called on `&&&&&&clap::builder::_infer_ValueParser_for<Size>` due to unsatisfied trait bounds
...
note: the traits `From`, `FromStr`, `ValueEnum`,  and `ValueParserFactory` must be implemented
```

Read the last line: any one of those traits is enough. Either implement
`FromStr` for `Size`, or name a parsing function on the field, as the worked
example does with `value_parser = parse_size`.

**2. Two arguments with the same short name.** clap adds `-h` for help. A
field named `height` with `#[arg(short)]` wants `-h` too. This compiles. It
panics when the command is built, and only in a debug build, because clap
checks a definition with debug assertions:

```rust,no_run
use clap::{CommandFactory, Parser};

#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long)]
    width: u32,
    #[arg(short, long)]
    height: u32,
}

fn main() {
    Cli::command().debug_assert();
}
```

```text
Command shop: Short option names must be unique for each argument, but '-h' is in use by both 'height' and 'help' (call `cmd.disable_help_flag(true)` to remove the auto-generated `--help`)
```

(`shop` is the name of the program that produced this message.) Give the field
another letter with `#[arg(short = 'H', long)]`. More important: put
`Cli::command().debug_assert()` in one test. It checks the whole definition,
so a mistake like this fails in `cargo test` and not in front of a user.

**3. Calling `Cli::parse()` in code that you test.** `parse` prints to the
real standard streams and calls `std::process::exit`. In a test that ends the
whole test process. Keep `parse()` in `main`, and give the rest of the program
a function that takes the arguments and the output streams as parameters.

### When to use which

| Field type | What the user writes |
| --- | --- |
| `bool` with `#[arg(long)]` | a flag: `--overwrite` or nothing |
| `T` with `#[arg(long)]` | a required option: `--size 800x600` |
| `T` with `default_value_t = ...` | an option with a default |
| `Option<T>` with `#[arg(long)]` | an option that may be absent |
| `Vec<T>` with `#[arg(long)]` | an option that may repeat: `--tag a --tag b` |
| `T` or `Vec<T>` without `short`/`long` | positional arguments |
| `u8` with `action = clap::ArgAction::Count` | a flag that is counted: `-vvv` |

Use `ValueEnum` for a fixed set of words: help and error messages then list
the possible values. Use a value-parser function when the value has a format.
Use subcommands when the program does several different things with different
arguments.

clap costs compile time. For a tool with two flags that only your team runs,
`std::env::args` or a small crate such as lexopt is a fair choice.

### Read more

- [clap's derive tutorial](https://docs.rs/clap/4.6.7/clap/_derive/_tutorial/index.html) and [derive reference](https://docs.rs/clap/4.6.7/clap/_derive/index.html)
- [`std::process::ExitCode`](https://doc.rust-lang.org/std/process/struct.ExitCode.html) and [`std::env::args_os`](https://doc.rust-lang.org/std/env/fn.args_os.html)

## Lesson E04 · Structured logging

**Builds on:** Chapter 18 (writing to streams), closures from Chapter 14, and
`Display`/`Debug` from Chapter 17.

### The mental model

A log line written with `println!` is text. To find all lines for one order,
you search for a substring and hope every line included it. *Structured*
logging records **fields** (`order=17`) next to the message, and it records
*where in the work* the program was.

tracing has two kinds of data:

- An **event** is a moment: "out of stock". `info!`, `warn!`, and the other
  level macros create events.
- A **span** is a period of time with a name and fields: "packing order 17".
  While a span is *entered*, every event belongs to it and shows its fields.

The code that creates events and spans does not know where they go. A
**subscriber** receives them and decides what to keep, how to format it, and
where to write it.

```text
your code                          the subscriber (chosen by the binary)
─────────                          ─────────────────────────────────────
info_span!("pack", order) ──────►  keep this level?  ── no ──► dropped,
warn!(item = %name, "...") ─────►         │ yes                arguments
                                          ▼                    not evaluated
                                   format (text, JSON, ...)
                                          ▼
                                   write (stderr, a file, a test buffer)
```

Two consequences shape how the crate is used:

- A **library** only creates events and spans. Only the **binary** installs a
  subscriber, once, in `main`. With no subscriber, the macros do almost
  nothing.
- A disabled event is skipped *before* its arguments are evaluated. A
  `debug!` with an expensive argument costs nothing when the level is INFO.

Levels, from most to least severe: `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`.

### Worked example: packing an order

```rust
use tracing::{Level, debug, info, info_span, warn};

/// Packs an order. Returns how many items were packed.
fn pack(order: u64, items: &[(&str, u32)]) -> u32 {
    // The span is the context. Every event inside it is tagged with `order=..`.
    let span = info_span!("pack", order);
    let _inside = span.enter();

    let mut packed = 0;
    for &(name, in_stock) in items {
        if in_stock == 0 {
            // `%name` records the field with Display; plain `name` also works for &str.
            warn!(item = %name, "out of stock");
        } else {
            debug!(item = %name, in_stock, "packed");
            packed += 1;
        }
    }
    info!(packed, "order done");
    packed
}

fn main() {
    // The subscriber decides what is kept and where it goes. Here: INFO and
    // above, to standard output, in the plainest format.
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_writer(std::io::stdout)
        .without_time()
        .with_ansi(false)
        .finish();

    // Active only on this thread and only inside the closure.
    let packed = tracing::subscriber::with_default(subscriber, || {
        pack(17, &[("lamp", 3), ("desk", 0), ("chair", 1)])
    });
    assert_eq!(packed, 2);

    // No subscriber here: the same call is silent, and almost free.
    assert_eq!(pack(18, &[("lamp", 1)]), 1);
}
```

The first call prints two lines. The `debug!` events are below the chosen
level and are dropped:

```text
 WARN pack{order=17}: shop: out of stock item=desk
 INFO pack{order=17}: shop: order done packed=2
```

Read a line from left to right: the level, the span with its fields, the
*target* (by default the module path; here the crate was called `shop`), the
message, and the event's own fields.

In a field, `%value` records with `Display` and `?value` records with `Debug`.
A bare name works for numbers, booleans, and `&str`.

### Mistakes people make

**1. Letting `#[instrument]` record every argument.** The attribute
`#[tracing::instrument]` wraps a function in a span named after the function.
By default it records *all* arguments with `Debug`. If one of them has no
`Debug` impl, the code does not compile:

```rust,compile_fail,E0277
struct Password(String);

#[tracing::instrument]
fn login(user: &str, password: &Password) -> bool {
    !user.is_empty() && password.0 == "hunter2"
}

fn main() {
    println!("{}", login("ann", &Password("guess".into())));
}
```

```text
error[E0277]: `Password` doesn't implement `Debug`
   --> src/main.rs:3:1
    |
  3 | #[tracing::instrument]
    | ^^^^^^^^^^^^^^^^^^^^^^ the trait `Debug` is not implemented for `Password`
```

Do not fix this with `#[derive(Debug)]`. Then it compiles, and the password is
written to your logs. Say exactly what to record:
`#[tracing::instrument(skip_all, fields(user = %user))]`.

**2. Recording a value of your own type without `%` or `?`.**

```rust,compile_fail,E0277
#[derive(Debug)]
struct Order {
    id: u64,
}

fn main() {
    let order = Order { id: 17 };
    tracing::info!(order, "packed");
    println!("{}", order.id);
}
```

```text
error[E0277]: the trait bound `Order: Value` is not satisfied
 --> src/main.rs:8:5
  |
8 |     tracing::info!(order, "packed");
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unsatisfied trait bound
```

Only a few basic types can be recorded directly. Write `?order` to record it
with `Debug`, or record the one field that matters: `order = order.id`.

**3. Installing a subscriber in a library, or twice.** A process has one
global subscriber. The second attempt to set it fails:

```rust
let first = tracing::subscriber::set_global_default(tracing_subscriber::fmt().finish());
let second = tracing::subscriber::set_global_default(tracing_subscriber::fmt().finish());
assert!(first.is_ok());
assert_eq!(
    second.unwrap_err().to_string(),
    "a global default trace dispatcher has already been set"
);
```

If a library did this, it would take the choice away from every application
that uses it. Tests are in the same position: many tests run in one process.
In a test, use `tracing::subscriber::with_default`, as the worked example does.

**4. Holding a span guard across `.await`.** `span.enter()` returns a guard,
and the span is entered until the guard is dropped. In an `async fn`, the task
can be suspended at an `.await` while the guard is alive. The thread then runs
other tasks, and their events are tagged with your span. Nothing warns you.
In async code, attach the span to the future with `.instrument(span)` (from
the trait `tracing::Instrument`) or use `#[instrument]`. Both enter the span
only while that future is being polled.

### When to use which

| You want to record | Use |
| --- | --- |
| Something that happened | an event at a level: `warn!(...)` |
| Context for everything that happens during a piece of work | a span |
| A value known only at the end of the work | declare the field as `tracing::field::Empty`, then `span.record(...)` |
| Text meant for the user of a CLI | `println!` / `eprintln!`; that is output, not logging |

Choosing a level: `ERROR` when the operation failed and someone should look.
`WARN` when it was unusual and handled. `INFO` for the few lines that tell the
story of a normal run. `DEBUG` and `TRACE` for detail you switch on while
investigating.

The older `log` crate has events but no spans. It is simpler, and still common
in libraries. Choose tracing when work is concurrent (threads or async),
because spans are what keep interleaved lines apart.

### Read more

- [tracing 0.1.44](https://docs.rs/tracing/0.1.44/tracing/): spans, events, and the [`#[instrument]` attribute](https://docs.rs/tracing/0.1.44/tracing/attr.instrument.html)
- [tracing-subscriber's `fmt` module](https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/fmt/index.html)
- [`std::fmt`](https://doc.rust-lang.org/std/fmt/index.html) for `Display` and `Debug`

## Lesson E05 · Property-based testing

**Builds on:** Chapter 15 (iterators), Chapter 16 (collections), and unit
tests as used in every chapter.

### The mental model

An example test says: "for *this* input, the answer is *that*." You choose the
inputs, so the test only covers the cases you thought of.

A **property** says: "for *every* input of this kind, this statement is true."
A property test has three parts:

```text
generator (Strategy)        property                    on failure
────────────────────        ────────                    ──────────
makes random inputs   ───►  a function that checks  ──► shrinking: try simpler
of a chosen shape           one input and returns       inputs that still fail,
                            Ok or a failure             report the simplest one
```

proptest runs the property 256 times by default, each time with a new input.
When a case fails, the input is usually large and random. **Shrinking** then
searches for a smaller input that still fails. You get a counterexample that
you can read.

You do not need to know the answer for each input. You need a statement that
is true for all of them. Four kinds cover most code:

- **Round trip:** `decode(encode(x)) == x`.
- **Invariant:** something that always holds for the output: "the result is
  sorted", "the length is unchanged".
- **Oracle** (or *model*): a slow, obviously correct way to get the same answer.
- **Idempotence:** doing it twice equals doing it once: `f(f(x)) == f(x)`.

### Worked example: merging two sorted lists

```rust
use proptest::prelude::*;

/// Merges two sorted slices into one sorted vector.
fn merge(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::with_capacity(a.len() + b.len());
    while i < a.len() && j < b.len() {
        if a[i] <= b[j] {
            out.push(a[i]);
            i += 1;
        } else {
            out.push(b[j]);
            j += 1;
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
    out
}

/// A generator of sorted vectors: generate any vector, then sort it.
fn sorted_vec() -> impl Strategy<Value = Vec<u32>> {
    // Small numbers on purpose: with `any::<u32>()`, two equal elements almost
    // never appear, and equal elements are where merge code goes wrong.
    proptest::collection::vec(0u32..20, 0..30).prop_map(|mut v| {
        v.sort_unstable();
        v
    })
}

fn main() {
    // The closure form of `proptest!` runs at once, with 256 generated cases.
    proptest!(|(a in sorted_vec(), b in sorted_vec())| {
        let merged = merge(&a, &b);

        // Invariant: the output is sorted.
        prop_assert!(merged.windows(2).all(|w| w[0] <= w[1]));

        // Oracle: a slower way that is obviously right gives the same answer.
        let mut expected = [a.clone(), b.clone()].concat();
        expected.sort_unstable();
        prop_assert_eq!(merged, expected);
    });
}
```

Inside `proptest!`, `a in sorted_vec()` reads "for every `a` that this
strategy can generate". `prop_assert!` and `prop_assert_eq!` work like
`assert!` and `assert_eq!`, but they report the failure to the runner by
returning an error instead of panicking.

In a real project the same thing is written as a test function inside the
macro: `proptest! { #[test] fn merge_is_sorted(a in sorted_vec(), ...) { ... } }`.

Now one property, about the length of the result, against a `merge` with a bug:

```rust
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestError, TestRng, TestRunner};

/// Wrong on purpose: when both heads are equal, it keeps only one of them.
fn merge_dropping(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < a.len() && j < b.len() {
        if a[i] < b[j] {
            out.push(a[i]);
            i += 1;
        } else if a[i] > b[j] {
            out.push(b[j]);
            j += 1;
        } else {
            out.push(a[i]);
            i += 1;
            j += 1;
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
    out
}

fn sorted_vec() -> impl Strategy<Value = Vec<u32>> {
    proptest::collection::vec(0u32..20, 0..30).prop_map(|mut v| {
        v.sort_unstable();
        v
    })
}

fn main() {
    // A runner with a fixed seed, so that this example gives the same result
    // on every machine. `proptest!` normally uses a fresh random seed.
    let config = Config {
        failure_persistence: None,
        ..Config::default()
    };
    let rng = TestRng::deterministic_rng(RngAlgorithm::ChaCha);
    let mut runner = TestRunner::new_with_rng(config, rng);

    let result = runner.run(&(sorted_vec(), sorted_vec()), |(a, b)| {
        prop_assert_eq!(merge_dropping(&a, &b).len(), a.len() + b.len());
        Ok(())
    });

    // The runner first finds some failing pair, usually two long vectors. What
    // it reports is the *shrunk* pair:
    let Err(TestError::Fail(_, (a, b))) = result else {
        panic!("the property should fail");
    };
    assert_eq!((a, b), (vec![8], vec![8]));
}
```

The report `([8], [8])` tells you almost everything: one element on each side,
and the two are equal. It also shows a limit of shrinking. `([0], [0])` would
be even simpler, but the shrinker changes one value at a time, and changing
only one 8 makes the test pass. It stops at an input where no single step
gives a simpler failure.

### Mistakes people make

**1. A generator that cannot reach the bug.** This is the most common reason
a property test finds nothing. The bug above needs two equal numbers. With
`any::<u32>()` as the element strategy, two random elements are equal about
once in four billion pairs, and the test would almost certainly pass. Ask of
every generator: which inputs are special for this code (empty, equal, at a
limit, very long), and can the generator produce them often? Use small
ranges, and use `prop_oneof!` to mix in special cases.

**2. Two strategies in an `if`.** Each strategy has its own type, like each
iterator adapter does:

```rust,compile_fail,E0308
use proptest::prelude::*;

fn numbers(small: bool) -> impl Strategy<Value = u32> {
    if small { 0u32..10 } else { any::<u32>() }
}

fn main() {
    let _ = numbers(true);
}
```

```text
error[E0308]: `if` and `else` have incompatible types
 --> src/main.rs:4:34
  |
4 |     if small { 0u32..10 } else { any::<u32>() }
  |                --------          ^^^^^^^^^^^^ expected `Range<u32>`, found `Any`
  |                |
  |                expected because of this
```

Call `.boxed()` on both and return `BoxedStrategy<u32>`. To choose between
strategies at random rather than by a flag, use
`prop_oneof![0u32..10, any::<u32>()]`.

**3. `prop_assert!` in a function that returns nothing.** The macro returns
early with an `Err`, so the function around it must return a `Result`:

```rust,compile_fail,E0308
use proptest::prelude::*;

fn check_sorted(v: &[u32]) {
    prop_assert!(v.windows(2).all(|w| w[0] <= w[1]));
}

fn main() {
    check_sorted(&[1, 2]);
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:4:5
  |
3 | fn check_sorted(v: &[u32]) {
  |                           - help: a return type might be missing here: `-> _`
4 |     prop_assert!(v.windows(2).all(|w| w[0] <= w[1]));
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `()`, found `Result<_, TestCaseError>`
```

Declare the helper as `fn check_sorted(v: &[u32]) -> Result<(), TestCaseError>`,
end it with `Ok(())`, and call it with `?`.

**4. Writing the implementation a second time in the test.** A property that
computes the expected value the same way as the code under test agrees with
it, bugs included. An oracle must be *simpler* than the code: sorting the
concatenation is simpler than merging.

### When to use which

| Tool | Good for | Limit |
| --- | --- | --- |
| Example tests | Documenting behaviour; cases from bug reports | Only the inputs you thought of |
| Property tests | Searching for inputs you did not think of; code with a clear rule | Needs a good generator and a true statement |
| Fuzzing (cargo-fuzz) | Parsers and decoders of untrusted input; runs for hours and learns from coverage | Runs outside `cargo test`; finds crashes more easily than wrong answers |

Keep both example tests and properties. When a property finds a bug, add the
shrunk input as an example test, so that the case stays covered forever.

### Read more

- [proptest 1.12.0](https://docs.rs/proptest/1.12.0/proptest/): [`Strategy`](https://docs.rs/proptest/1.12.0/proptest/strategy/trait.Strategy.html), [`collection`](https://docs.rs/proptest/1.12.0/proptest/collection/index.html), [`prop_oneof!`](https://docs.rs/proptest/1.12.0/proptest/macro.prop_oneof.html)
- [The proptest guide](https://proptest-rs.github.io/proptest/intro.html), which has a chapter on shrinking
- [Writing tests](https://doc.rust-lang.org/cargo/guide/tests.html) in the Cargo documentation

## Lesson E06 · Async services with tokio

**Builds on:** Chapters 20 and 21 (`async`/`.await`, futures, pinning) and
Chapter 19 (threads, channels, `Send`).

### The mental model

From Chapter 20 you know that a future does nothing until it is polled, and
that `.await` is a point where a task can pause. tokio is the **runtime** that
does the polling. It has three parts: a scheduler that runs tasks, a timer,
and (with the `net` feature) a driver that waits for sockets.

Five ideas carry this lesson:

- A **task** is a future given to the runtime with `tokio::spawn`. It runs
  independently, like a very cheap thread. `spawn` returns a `JoinHandle`;
  a `JoinSet` holds many handles and gives you results as tasks finish.
- **Cancellation is dropping.** To cancel a future, you drop it. It stops at
  the `.await` where it was paused, and its local variables are dropped.
  `tokio::time::timeout` and `tokio::select!` both work this way: when one
  side wins, the other futures are dropped.
- A future is **cancellation safe** if dropping it before it finishes loses
  nothing. A future that has taken ownership of your value, and is then
  dropped, takes the value with it.
- A bounded **channel** is a queue with a limit. When it is full, `send`
  waits. That waiting is **backpressure**: a fast producer is slowed to the
  speed of the consumer.
- One slow synchronous call blocks the thread, and with it every task on that
  thread. Long computations and blocking calls go to
  `tokio::task::spawn_blocking`.

```text
select! {                         one turn of the loop
    a = fut_a => ...,       ┌──────────────────────────────────┐
    b = fut_b => ...,       │ poll fut_a, fut_b, fut_c          │
    c = fut_c => ...,       │ fut_b is ready first              │
}                           │   → run b's branch                │
                            │   → DROP fut_a and fut_c          │
                            └──────────────────────────────────┘
State that must live longer than one turn stays in variables outside `select!`.
```

When several branches are ready at the same moment, `select!` picks one at
random, unless the block starts with `biased;`. Then it takes the first ready
branch in source order.

### Worked example: a meter that reports every 100 ms

```rust
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{self, Instant, MissedTickBehavior};

/// Counts the readings that arrive on `rx` and reports the count every `period`.
/// Stops when `stop` fires or when every sender is gone.
async fn meter(
    mut rx: mpsc::Receiver<f64>,
    period: Duration,
    mut stop: oneshot::Receiver<()>,
) -> Vec<(Duration, u32)> {
    let start = Instant::now();
    let mut reports = Vec::new();
    // State that must survive between loop turns lives OUTSIDE `select!`.
    let mut count = 0;
    let mut ticker = time::interval_at(start + period, period);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        // Each turn waits on three things at once. The first one that is ready
        // wins; the futures of the other two branches are dropped.
        tokio::select! {
            _ = &mut stop => break,
            reading = rx.recv() => match reading {
                Some(_) => count += 1,
                None => break,
            },
            _ = ticker.tick() => {
                reports.push((start.elapsed(), count));
                count = 0;
            }
        }
    }
    reports
}

// `start_paused` makes time virtual: sleeping costs nothing and is exact.
#[tokio::main(flavor = "current_thread", start_paused = true)]
async fn main() {
    let ms = Duration::from_millis;
    let (tx, rx) = mpsc::channel(8);
    let (stop_tx, stop_rx) = oneshot::channel();
    let task = tokio::spawn(meter(rx, ms(100), stop_rx));

    // Five readings, 30 ms apart: at 30, 60, 90, 120, and 150 ms.
    for _ in 0..5 {
        time::sleep(ms(30)).await;
        tx.send(21.5).await.unwrap();
    }
    // Stop at 260 ms: after the second report, before the third.
    time::sleep(ms(110)).await;
    stop_tx.send(()).unwrap();

    let reports = task.await.unwrap();
    assert_eq!(reports, [(ms(100), 3), (ms(200), 2)]);
}
```

`&mut stop` matters: the branch polls the receiver *through a reference*, so
the receiver itself survives each turn. `rx.recv()` and `ticker.tick()` create
a new small future every turn. Both are cancellation safe: dropping them
before they finish loses no reading and no tick.

The test uses **virtual time**. With `start_paused = true`, the clock only
moves when every task is waiting, and then it jumps straight to the next
timer. A test that "sleeps" for an hour finishes at once, and elapsed times
are exact. This needs tokio's `test-util` feature. It only works while all
waiting is on tokio timers and channels: with real sockets the runtime looks
idle while a packet is on its way, and the clock would jump (see E09).

### Mistakes people make

**1. Holding a `std::sync::MutexGuard` across `.await` in a spawned task.**
A spawned task may move between threads at each `.await`, so everything it
holds at that point must be `Send`. A std mutex guard is not:

```rust,compile_fail
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let total = Arc::new(Mutex::new(0));
    let shared = Arc::clone(&total);
    let task = tokio::spawn(async move {
        let mut guard = shared.lock().unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        *guard += 1;
    });
    task.await.unwrap();
    println!("{}", total.lock().unwrap());
}
```

```text
error: future cannot be sent between threads safely
   --> src/main.rs:8:16
    |
  8 |       let task = tokio::spawn(async move {
...
 12 | |     });
    | |______^ future created by async block is not `Send`
    |
    = help: within `{async block@src/main.rs:8:29: 8:39}`, the trait `Send` is not implemented for `std::sync::MutexGuard<'_, i32>`
note: future is not `Send` as this value is used across an await
   --> src/main.rs:10:55
    |
  9 |         let mut guard = shared.lock().unwrap();
    |             --------- has type `std::sync::MutexGuard<'_, i32>` which is not `Send`
 10 |         tokio::time::sleep(Duration::from_millis(10)).await;
    |                                                       ^^^^^ await occurs here, with `mut guard` maybe used later
```

The compiler is protecting you from a real problem: every other task that
wants the lock would wait for the whole sleep. Take the lock, do the update,
and release the lock, all without an `.await` in between:
`*shared.lock().unwrap() += 1;` after the sleep.

**2. Borrowing a local variable in a spawned task.** A spawned task can
outlive the function that started it, so it may not borrow from that function:

```rust,compile_fail,E0373
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let names = vec![String::from("ann"), String::from("bo")];
    let task = tokio::spawn(async { names.len() });
    println!("{} of {}", task.await.unwrap(), names.len());
}
```

```text
error[E0373]: async block may outlive the current function, but it borrows `names`, which is owned by the current function
 --> src/main.rs:4:29
  |
4 |     let task = tokio::spawn(async { names.len() });
  |                             ^^^^^   ----- `names` is borrowed here
  |                             |
  |                             may outlive borrowed value `names`
...
help: to force the async block to take ownership of `names` (and any other referenced variables), use the `move` keyword
```

`async move` gives the task ownership. If the caller still needs the data,
give the task its own copy: clone the data, or put it in an `Arc` and clone
the `Arc`.

**3. Forgetting `.await`.** Calling an async function only builds a future:

```rust,no_run
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tokio::time::sleep(Duration::from_secs(1));
    println!("done");
}
```

```text
warning: unused `Sleep` that must be used
 --> src/main.rs:5:5
  |
5 |     tokio::time::sleep(Duration::from_secs(1));
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: futures do nothing unless you `.await` or poll them
```

This is only a warning, and the program prints `done` at once. Read warnings.

**4. Blocking the runtime.** `std::thread::sleep`, a long loop, or a blocking
file or network call inside a task stops the thread, and every other task on
that thread stops with it. Nothing in the compiler tells you. Use
`tokio::time::sleep` for waiting and `spawn_blocking` for work.

### When to use which

| Need | Use |
| --- | --- |
| Wait for several futures, need all results | `tokio::join!`, or a `JoinSet` when the number is not fixed |
| Wait for whichever comes first | `tokio::select!`; check that the losing futures are cancellation safe |
| A time limit | `tokio::time::timeout(limit, future)` |
| Hand items to another task, with a limit | `mpsc::channel(capacity)` |
| One answer, once | `oneshot::channel()` |
| "The newest value", read by many | `watch::channel(initial)` |
| Protect data for a short moment, no `.await` inside | `std::sync::Mutex` |
| Hold a lock across an `.await` | `tokio::sync::Mutex` (and first ask whether a channel is simpler) |
| CPU-heavy or blocking work | `spawn_blocking`, or rayon for parallel computation |

Async is the right tool when a program *waits* on many things at once:
sockets, timers, queues. For a program that mostly computes, threads are
simpler and just as fast.

### Read more

- [tokio 1.54.1](https://docs.rs/tokio/1.54.1/tokio/): [`select!`](https://docs.rs/tokio/1.54.1/tokio/macro.select.html) (read the section on cancellation safety), [`JoinSet`](https://docs.rs/tokio/1.54.1/tokio/task/struct.JoinSet.html), [`mpsc`](https://docs.rs/tokio/1.54.1/tokio/sync/mpsc/index.html), [`time::pause`](https://docs.rs/tokio/1.54.1/tokio/time/fn.pause.html)
- [The tokio tutorial](https://tokio.rs/tokio/tutorial)
- [`std::future::Future`](https://doc.rust-lang.org/std/future/trait.Future.html) and [`Send`](https://doc.rust-lang.org/std/marker/trait.Send.html)

## Lesson E07 · Capstone: `jobq`

**Builds on:** E01 to E06. There is no new crate here. The new skill is
putting the pieces in the right places.

### The mental model

A well-built command-line program is a thin `main` around a library. Each
crate from this track owns one layer, and each layer hands the next one a
typed value:

```text
argv ──► clap ──► Cli                               usage error → exit 2
                   │
                   ▼
         load_config: file → serde → validate       ConfigError (thiserror)
                   │                                 Io / Json / Invalid
                   ▼
         run_jobs: bounded queue → worker tasks      a job that fails is not an
                   │   (tokio; a tracing span        error of the program: it is
                   │    around each job)             data, an `Outcome`
                   ▼
         Report ──► serde ──► stdout
                   │
                   ▼
         execute: anyhow::Result<i32>                adds "loading {path}"
                   │
                   ▼
         main: install subscriber, build runtime,    exit code from the typed
               call execute, map to ExitCode         error found in the chain
```

Three rules explain most of the design:

- **Everything testable lives in the library.** `main` is the only code that
  touches the real arguments, installs the global tracing subscriber, and
  ends the process. All of that is hard to test, so `main` should contain
  nothing else.
- **Typed errors below, context above.** The config layer returns an enum, so
  that the exit code can depend on *which* failure happened. The application
  layer only adds a sentence for the human reader.
- **An expected failure is a value.** A job that fails or times out does not
  stop the run. It becomes an `Outcome` in the report. Keep `Result::Err` for
  what prevents the function from doing its job at all.

### How to work through it

Do the tasks in the order of the file: A (config types), B (errors and
validation), C (the worker pool), D (the application function), then `main`.
The integration tests are named `a_` to `e_` in the same order, so you can run
them one group at a time. For C, write down first, on paper, who owns the
receiving end of the queue and how each worker learns that no more jobs will
come. The answer to the second question is the reason the sender must be
dropped.

Every technique was practised in its own lesson. When a part resists, go back
to that lesson's section here: E02 for the config attributes, E01 for the
error enum and the exit codes, E06 for the queue and the timeout, E04 for the
spans.

### Read more

- [`std::process::ExitCode`](https://doc.rust-lang.org/std/process/struct.ExitCode.html) and the [`Termination`](https://doc.rust-lang.org/std/process/trait.Termination.html) trait
- [`tracing::Instrument`](https://docs.rs/tracing/0.1.44/tracing/trait.Instrument.html), for attaching a span to a future

## Lesson E08 · Tooling workday

**Builds on:** E01 (the package must build), macros from Chapter 22, and
iterators from Chapter 15 for the last two tasks. The tasks themselves are in
[TOOLING.md](TOOLING.md).

### The mental model

Much of a working day with Rust is not writing features. It is asking
questions *about* code, and Cargo ships a tool for most of them:

| Question | Tool |
| --- | --- |
| How do I use this API? Are my examples still correct? | `cargo doc`, doc tests |
| Why is this crate in my build, and with which features? | `cargo tree` |
| Is this code idiomatic? Can we agree on rules? | `cargo clippy`, the `[lints]` table |
| What does this macro turn into? | `stringify!`, rust-analyzer, cargo-expand |
| How is the shipped binary built? | profiles in `Cargo.toml` |
| How long does this take? | `std::time::Instant`, then a benchmark harness |
| *Where* does the time go? | a sampling profiler and a flamegraph |

Two ideas need a model before you start.

**A profile is a set of compiler settings with a name.** `cargo build` uses
the `dev` profile: fast to compile, slow to run, with overflow checks.
`cargo build --release` uses `release`: the opposite. They are different
programs for the purpose of measuring. Never time a debug build.

**A sampling profiler takes many snapshots of the call stack.** It stops the
program many times per second and writes down which functions are on the
stack. Each distinct stack is counted. The *folded* form of the result is one
line per distinct stack:

```text
main;load;parse 60        60 samples saw main → load → parse, with parse on top
main;load 10              10 samples saw load itself running
main;render 30
```

From this you can answer two different questions about `load`:

- **Inclusive** samples: all stacks that contain `load` = 60 + 10 = 70. This
  is the cost of calling `load`, with everything it calls. In a flamegraph it
  is the *width* of the box.
- **Self** samples: stacks where `load` is the last frame = 10. This is the
  time in `load`'s own code.

A function with a large inclusive count and a small self count is not slow.
Something it calls is.

### Worked example: looking at a macro expansion

A macro bug is hard to see because you read the call, not the code that the
compiler sees. `stringify!` turns tokens into text, so a temporary arm that
stringifies the expansion shows you that code:

```rust
/// Squares its argument. There is a bug to find.
macro_rules! square {
    // A debugging arm, placed first: show the code instead of running it.
    (@show $x:expr) => {
        stringify!($x * $x)
    };
    ($x:expr) => {
        $x * $x
    };
}

fn main() {
    let mut calls = 0;
    let mut next = || {
        calls += 1;
        calls
    };
    // We expect 1 * 1 = 1. The macro pastes the expression twice, so `next()`
    // runs twice: 1 * 2.
    assert_eq!(square!(next()), 2);
    assert_eq!(square!(@show next()), "next() * next()");

    // The usual fix: evaluate once, into a local variable.
    macro_rules! square_once {
        ($x:expr) => {{
            let value = $x;
            value * value
        }};
    }
    assert_eq!(square_once!(next()), 9);
}
```

`@show` is not special syntax. It is a token that no expression starts with,
so the general arm cannot match it by accident. Put such a debugging arm first
all the same. Arms are tried from top to bottom, and a general arm that takes
*any* tokens, such as `$($t:tt)*`, would also take `@show`. The macro in
TOOLING.md task 4 has such an arm.

### Mistakes people make

**1. A link in a doc comment that points nowhere.** Rustdoc resolves
`` [`Name`] `` in doc comments to the item with that name. A typo gives a
warning, and the published page gets plain text and no link:

```text
warning: unresolved link to `Confg::new`
 --> src/lib.rs:3:46
  |
3 | /// Settings for the cache. Build one with [`Confg::new`].
  |                                              ^^^^^^^^^^ no item named `Confg` in scope
  |
  = note: `#[warn(rustdoc::broken_intra_doc_links)]` on by default
```

Warnings are easy to miss in a long build. Projects run
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` in CI, which turns every
documentation warning into an error.

**2. Switching on a lint group and one lint at the same priority.** In the
`[lints]` table, a group such as `pedantic` and a single lint can contradict
each other. Cargo does not go by the order of the lines, so clippy refuses the
table:

```text
error: lint group `pedantic` has the same priority (0) as a lint
 --> Cargo.toml:8:1
  |
8 | pedantic = "warn"
  | ^^^^^^^^   ------ has an implicit priority of 0
9 | must_use_candidate = "allow"
  | ------------------ has the same priority as this lint
  |
  = note: the order of the lints in the table is ignored by Cargo
...
help: to have lints override the group set `pedantic` to a lower priority
  |
8 - pedantic = "warn"
8 + pedantic = { level = "warn", priority = -1 }
```

**3. Measuring something the optimizer removed.** In a release build, a
computation whose result is never used can be deleted. The loop then measures
nothing, and you read "0 ns". Pass the result (and often the input) through
`std::hint::black_box`. It returns its argument unchanged, and the optimizer
must assume that the value is used.

**4. Trusting one measurement.** The first run pays for cold caches and lazy
initialization. Later runs are disturbed by other programs and by the CPU
changing speed. Run a warm-up first, take many samples, and report the
minimum and the median. One number from one run is an anecdote.

**5. Believing that `cargo test --all-targets` runs everything.** It does not
run doc tests. They are a separate step, `cargo test --doc`.

### When to use which

| Situation | Reach for |
| --- | --- |
| "Is version B faster than version A?" for a function | `Instant` with warm-up and many samples for a first look; criterion or divan when the answer matters |
| "Is the whole command faster?" | hyperfine, which times complete runs of a program |
| "Why is it slow?" | a profiler (samply, cargo-flamegraph); measure first, then read code |
| A lint that is right in general and wrong in one place | `#[expect(lint, reason = "...")]` on that item |
| A lint that the team rejects | `"allow"` in `[lints]`, with a comment |
| A macro you wrote behaves oddly | `stringify!` on the expansion, or cargo-expand |

### Read more

- [Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) and [the `[lints]` section](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section) in the Cargo reference
- [`cargo tree`](https://doc.rust-lang.org/cargo/commands/cargo-tree.html) and [features](https://doc.rust-lang.org/cargo/reference/features.html), including feature unification
- [Documentation tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html) and [linking to items by name](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html) in the rustdoc guide
- [`std::hint::black_box`](https://doc.rust-lang.org/std/hint/fn.black_box.html) and the [Clippy documentation](https://doc.rust-lang.org/clippy/)

## Lesson E09 · An HTTP JSON service and its client

**Builds on:** E01 (error types), E02 (serde), E06 (tasks, timeouts,
cancellation), and Chapter 18 for the idea of a socket.

### The mental model

HTTP is a request and a response. A request has a *method* (`GET`, `PUT`,
`POST`...), a *path* (`/notes/7`), headers, and maybe a body. A response has
a *status code* (200, 404...), headers, and maybe a body. A service is a
function from one to the other.

axum splits that function into small parts:

```text
              one task per connection
client ──TCP──► hyper reads the request
                    │
                    ▼
                 Router ── no route → 404, wrong method → 405
                    │
                    ▼  middleware layers (outermost first)
                 extractors: State, Path, Json ...
                    │   └─ a failed extractor answers by itself (400, 422 ...)
                    ▼
                 your handler: async fn(extractors) -> impl IntoResponse
                    │
                    ▼
client ◄──────── status + headers + body
```

- A **handler** is an async function. Each parameter is an **extractor**: a
  type that knows how to get itself out of the request. `Path<u32>` parses a
  path segment, `Json<T>` reads and deserializes the body, `State<S>` hands
  you the shared state.
- The return type implements **`IntoResponse`**. Many types do: a
  `StatusCode`, a `String`, `Json<T>`, a tuple `(StatusCode, Json<T>)`, and
  `Result<T, E>` when both `T` and `E` do. That last one is why your own error
  type should implement it: handlers can then use `?`.
- **State** is shared by all requests, and many requests run at the same
  time. So the state must be cheap to clone and safe to share: in practice an
  `Arc` around the data, with a lock or a channel inside.
- **Middleware** wraps every handler: a time limit, logging, authentication.
  axum uses the `Service` and `Layer` traits of the tower crate for this.

A client such as reqwest keeps a pool of open connections and gives you a
future for each response. One fact surprises almost everyone: for the client,
**a 404 or 500 is a successful exchange**. A response came back, so you get
`Ok(response)` and must look at the status yourself.

**Graceful shutdown** means: stop accepting new connections, let the requests
that have already started finish, then return. The usual signal inside a
program is a `CancellationToken` from tokio-util: one part of the program
calls `cancel()`, and every holder of a clone can wait for `cancelled()`.

### Worked example: a notes service, tested over loopback

```rust
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Note {
    id: usize,
    text: String,
}

#[derive(Deserialize)]
struct NewNote {
    text: String,
}

/// The state is a handle: cloning it copies the `Arc`, not the notes.
#[derive(Clone, Default)]
struct Notes(Arc<Mutex<Vec<String>>>);

/// One error type for all handlers, with one place that maps it to HTTP.
enum NoteError {
    Empty,
    Missing(usize),
}

impl IntoResponse for NoteError {
    fn into_response(self) -> Response {
        match self {
            NoteError::Empty => (StatusCode::UNPROCESSABLE_ENTITY, "a note needs text").into_response(),
            NoteError::Missing(id) => (StatusCode::NOT_FOUND, format!("no note {id}")).into_response(),
        }
    }
}

// Extractors, left to right: shared state first, the body last.
async fn create(
    State(notes): State<Notes>,
    Json(new): Json<NewNote>,
) -> Result<(StatusCode, Json<Note>), NoteError> {
    if new.text.trim().is_empty() {
        return Err(NoteError::Empty);
    }
    let mut all = notes.0.lock().unwrap();
    all.push(new.text.clone());
    Ok((StatusCode::CREATED, Json(Note { id: all.len(), text: new.text })))
}

async fn read(State(notes): State<Notes>, Path(id): Path<usize>) -> Result<Json<Note>, NoteError> {
    let all = notes.0.lock().unwrap();
    // Ids start at 1, so id 0 has no slot: `checked_sub` gives `None` for it.
    let text = id.checked_sub(1).and_then(|index| all.get(index));
    match text {
        Some(text) => Ok(Json(Note { id, text: text.clone() })),
        None => Err(NoteError::Missing(id)),
    }
}

fn app(notes: Notes) -> Router {
    Router::new()
        .route("/notes", post(create))
        .route("/notes/{id}", get(read))
        .with_state(notes)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Port 0 asks the system for any free port; `local_addr` says which one it chose.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());

    // The server stops gracefully when `stop` fires (or is dropped).
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app(Notes::default()))
            .with_graceful_shutdown(async move {
                let _ = stopped.await;
            })
            .await
    });

    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    let response = client
        .post(format!("{base}/notes"))
        .json(&serde_json::json!({"text": "water the plants"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let note: Note = response.json().await.unwrap();
    assert_eq!(note, Note { id: 1, text: "water the plants".into() });

    // A 404 is a normal `Ok(response)` for reqwest. You look at the status yourself.
    let response = client.get(format!("{base}/notes/7")).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.text().await.unwrap(), "no note 7");

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}
```

The address `127.0.0.1` is the *loopback* interface: the traffic never leaves
the machine. Port 0 means "any free port", so two tests can run at the same
time without fighting over a port number. Write `127.0.0.1` and not
`localhost`: on some machines `localhost` resolves to the IPv6 address `::1`
first, where nothing is listening.

`.no_proxy()` protects the test too. By default reqwest obeys proxy variables
such as `http_proxy`, and the request would go to that proxy.

### Mistakes people make

**1. A handler that axum does not accept.** The body can be read only once,
so the extractor that reads it (`Json`, `String`, `Bytes`) must be the *last*
parameter. With the wrong order, the function is simply "not a handler", and
the message does not say why:

```rust,compile_fail,E0277
use axum::extract::Path;
use axum::routing::put;
use axum::{Json, Router};

async fn rename(Json(name): Json<String>, Path(id): Path<u32>) -> String {
    format!("user {id} is now {name}")
}

fn main() {
    let _app: Router = Router::new().route("/users/{id}", put(rename));
}
```

```text
error[E0277]: the trait bound `fn(Json<String>, Path<u32>) -> _ {rename}: Handler<_, _>` is not satisfied
   --> src/main.rs:10:63
    |
 10 |     let _app: Router = Router::new().route("/users/{id}", put(rename));
    |                                                           --- ^^^^^^ unsatisfied trait bound
    |                                                           |
    |                                                           required by a bound introduced by this call
    |
    = help: the trait `Handler<_, _>` is not implemented for fn item `fn(Json<String>, axum::extract::Path<u32>) -> impl Future<Output = String> {rename}`
    = note: Consider using `#[axum::debug_handler]` to improve the error message
```

When you see this, check four things: every parameter is an extractor, the
body extractor is last, the return type implements `IntoResponse`, and the
function's future is `Send`. The attribute named in the note needs axum's
`macros` feature.

**2. Holding a std lock across `.await` in a handler.** This is mistake 1 of
E06 again. Here it gives the same unhelpful message as above, because a
function whose future is not `Send` is not a handler:

```rust,compile_fail,E0277
use axum::extract::State;
use axum::routing::get;
use axum::Router;
use std::sync::{Arc, Mutex};
use std::time::Duration;

type Hits = Arc<Mutex<u64>>;

async fn hit(State(hits): State<Hits>) -> String {
    let mut count = hits.lock().unwrap();
    tokio::time::sleep(Duration::from_millis(5)).await;
    *count += 1;
    count.to_string()
}

fn main() {
    let _app: Router = Router::new()
        .route("/hit", get(hit))
        .with_state(Hits::default());
}
```

```text
error[E0277]: the trait bound `fn(State<Arc<std::sync::Mutex<u64>>>) -> impl Future<Output = String> {hit}: Handler<_, _>` is not satisfied
   --> src/main.rs:18:28
    |
 18 |         .route("/hit", get(hit))
    |                        --- ^^^ the trait `Handler<_, _>` is not implemented for fn item `fn(State<Arc<std::sync::Mutex<u64>>>) -> impl Future<Output = String> {hit}`
```

Finish with the lock before the first `.await`. Put the locked part in a block
`{ ... }` or in a small synchronous function, so the guard is gone when the
`.await` comes.

**3. Reading the body of an error response as if it were the data.**

```rust
use axum::Router;

#[derive(Debug, serde::Deserialize)]
struct Note {
    text: String,
}

# #[tokio::main(flavor = "current_thread")]
# async fn main() {
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
let url = format!("http://{}/notes/7", listener.local_addr().unwrap());
// A server with no routes: every request gets 404 with an empty body.
tokio::spawn(async move { axum::serve(listener, Router::new()).await });
let client = reqwest::Client::builder().no_proxy().build().unwrap();

// `send` succeeds. The failure only appears when the empty body is parsed,
// and it says nothing about the real cause.
let response = client.get(&url).send().await.unwrap();
let note = response.json::<Note>().await;
assert!(note.unwrap_err().is_decode());

// Check the status first. `error_for_status` is the short way.
let response = client.get(&url).send().await.unwrap();
let err = response.error_for_status().unwrap_err();
assert_eq!(err.status(), Some(reqwest::StatusCode::NOT_FOUND));
assert_eq!(err.without_url().to_string(), "HTTP status client error (404 Not Found)");
# }
```

`error_for_status` throws the body away. If the server sends a useful error
body, read the status first and then the body.

**4. No time limit.** A reqwest client has no total timeout unless you set
one. A server that accepts the connection and then says nothing keeps your
task waiting forever. Set `timeout` on the client builder. On the server, put
a deadline around handlers, so that one stuck request cannot hold its
resources forever. (Test such limits with real time when a socket is
involved: paused time, from E06, jumps ahead while a request is in the
kernel.)

### When to use which

| Question | Guidance |
| --- | --- |
| How do I test a handler? | Send a request straight into the `Router` with `tower::ServiceExt::oneshot`: no socket, fast, and paused time works. Use a real listener on `127.0.0.1:0` for the few tests about connections, shutdown, and the client. |
| Which lock for shared state? | `std::sync::Mutex` (or `RwLock`) when the critical section is short and has no `.await`. `tokio::sync::Mutex` only when you must hold it across an `.await`. |
| Two steps that belong together (check, then change) | One critical section. With two, another request can run in between. |
| The item does not exist | 404 Not Found |
| The body is not valid for this endpoint | 400 Bad Request, or 422 Unprocessable Content for well-formed JSON that breaks a rule; choose one policy |
| The request is valid but conflicts with the current state | 409 Conflict |
| The server is overloaded or too slow right now | 503 Service Unavailable |

Give every error body one fixed JSON shape, with a stable code for programs
and a message for people. A client can then tell "no such item" from "wrong
URL", which both arrive as 404.

### Read more

- [axum 0.8.9](https://docs.rs/axum/0.8.9/axum/): [extractors](https://docs.rs/axum/0.8.9/axum/extract/index.html), [responses](https://docs.rs/axum/0.8.9/axum/response/index.html), [middleware](https://docs.rs/axum/0.8.9/axum/middleware/index.html), and [`serve`](https://docs.rs/axum/0.8.9/axum/fn.serve.html)
- [reqwest 0.13.5](https://docs.rs/reqwest/0.13.5/reqwest/), especially [`ClientBuilder`](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html) and [`Response`](https://docs.rs/reqwest/0.13.5/reqwest/struct.Response.html)
- [`CancellationToken`](https://docs.rs/tokio-util/0.7.20/tokio_util/sync/struct.CancellationToken.html) in tokio-util
- [`std::net::TcpListener`](https://doc.rust-lang.org/std/net/struct.TcpListener.html), for what binding and port 0 mean
- [RFC 9110, HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html), the definition of methods and status codes
