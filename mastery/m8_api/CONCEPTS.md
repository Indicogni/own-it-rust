# M8 concepts: library design, semver and stewardship

[Module guide](README.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Read one section, then do its lesson. Each section ends with links to the official documentation for the same topic.

## What you will be able to do

A program has one author and one reader: you. A library has users you will never meet. Their code compiles against yours today, and they expect it to compile next month, after you have released twice. This module is about that second fact. It teaches you to see a public interface as a set of promises, to know exactly which changes break a promise, and to design the first version so that the next ones are cheap.

After this module you can:

- say for a concrete change whether it needs a major, a minor or a patch release, and prove a "major" with a program that stops compiling;
- write a version 1.0.0 that leaves room for change: private fields, `#[non_exhaustive]`, sealed traits, newtypes;
- design an error type that callers can act on and that you can extend;
- design Cargo features that only add, and keep an optional dependency out of your interface;
- write documentation whose examples are tests, and that states errors and panics;
- deprecate instead of remove, keep a promised minimum Rust version, and choose an edition;
- review an interface against the Rust API Guidelines, and ship a release that does not break an existing user.

## Before you start

Chapters 1 to 24, in particular traits and generics (chapters 11 and 13), error handling (chapter 7), and crates and modules (chapter 8). You know what `Send` and `Sync` mean (chapter 19). Sealed traits also appear in mastery module [M1](../m1_types/); this module uses them from the point of view of a maintainer and explains what it needs.

## Time

These are estimates for a first pass, including reading.

| Part | Estimate |
| --- | --- |
| Lesson 01 · Version numbers as promises | 90 minutes |
| Lesson 02 · Traits, bounds and auto traits | 2 hours |
| Lesson 03 · Room to change | 90 minutes |
| Lesson 04 · An error type for a library | 2 hours |
| Lesson 05 · Features that only add | 2 hours |
| Lesson 06 · Documentation as interface | 2 hours |
| Lesson 07 · Deprecation, minimum Rust version, editions | 2 hours |
| Code review 01 · An interface against the API Guidelines | 2 hours |
| Lesson 08 · Capstone | 3 to 4 hours |
| Drills | 1 hour |

## Words used in this module

A *library* is a crate that other crates depend on. A *user* is a person who writes code against your library; a *downstream crate* is that code. The *public interface*, or API, is everything a downstream crate can name or observe: public items, their signatures, their trait implementations, their documented behaviour. A *breaking change* is a change after which some downstream crate that was correct before no longer compiles, or no longer behaves as documented.

## Lesson 01 · Version numbers as promises

**The mental model.** A version number `major.minor.patch` is a message from you to a program: Cargo. A user writes `shelf = "1.2.3"` in a manifest. That line does not mean "exactly 1.2.3". It is a *caret requirement*: "1.2.3 or any later version that is compatible with it". When you publish 1.3.0, `cargo update` gives it to that user without asking. So the number you choose decides who receives your release automatically. If the release breaks them, you have broken a build that nobody touched.

Cargo's rule for "compatible" is short: find the left-most number that is not zero. Two versions are compatible when they agree in that number and in everything to its left.

```text
requirement    Cargo may choose          the incompatible number
1.2.3          >= 1.2.3 and < 2.0.0      major
0.2.3          >= 0.2.3 and < 0.3.0      minor   (before 1.0.0)
0.0.3          exactly 0.0.3             patch   (every release)
```

From this rule the meaning of the three numbers follows. A release that can break users must change the incompatible number (a *major* release). A release that only adds must not change it (a *minor* release). A release that only repairs is a *patch* release. Before 1.0.0 there is one number less: `0.4.2` to `0.5.0` is the major step, and additions and repairs both go into the last number.

Version numbers are numbers. `1.10.0` is newer than `1.9.9`, although the text `"1.10.0"` sorts before `"1.9.9"`.

**Which changes break?** Rust has an official list: the *SemVer Compatibility* chapter of the Cargo book. It sorts changes into three classes.

- **Major**: the change requires a major release, because it can stop a correct program from compiling and the user has no reasonable defence. Examples: removing or renaming a public item; adding a public field to a struct whose fields are all public; adding a variant to an enum that is not `#[non_exhaustive]`; adding a parameter to a function.
- **Minor**: the change requires only a minor release. Examples: a new public function; a new private field in a struct that already has one; loosening a bound.
- **Possibly-breaking**: the change has the potential to break some programs, and many projects still accept it in a minor release. The best-known example is a new method. The alternative would be that a library can never add a method.

The list is about builds: almost every entry is a change after which `rustc` rejects something it accepted before. It is not complete. It says nothing about a type that loses a trait implementation, and for changes of run-time behaviour it leaves the judgement to the maintainers. For those cases use the definition from the overview: a change is breaking when a program that used the library as documented stops compiling or stops behaving as documented.

**A worked example.** Version 1 of a function returns `Option<u16>`. Version 2 returns a `Result`, so that the caller learns why the text was refused. The two modules below stand for the two versions of the crate. This program is correct for version 1:

```rust
mod v1 {
    pub fn parse_port(text: &str) -> Option<u16> {
        text.trim().parse().ok()
    }
}
use v1::parse_port;

fn main() {
    let port = parse_port("8080").unwrap_or(80);
    if let Some(port) = parse_port("443") {
        assert_eq!(port, 443);
    }
    assert_eq!(port, 8080);
}
```

Change only the library, and the same `main` no longer compiles:

```rust,compile_fail,E0308
mod v2 {
    pub fn parse_port(text: &str) -> Result<u16, std::num::ParseIntError> {
        text.trim().parse()
    }
}
use v2::parse_port;

fn main() {
    let port = parse_port("8080").unwrap_or(80);
    if let Some(port) = parse_port("443") {
        assert_eq!(port, 443);
    }
    assert_eq!(port, 8080);
}
```

```text
error[E0308]: mismatched types
  --> src/main.rs:10:12
   |
10 |     if let Some(port) = parse_port("443") {
   |            ^^^^^^^^^^   ----------------- this expression has type `Result<u16, ParseIntError>`
   |            |
   |            expected `Result<u16, ParseIntError>`, found `Option<_>`
```

Look at the first line of `main`: `parse_port("8080").unwrap_or(80)` compiles with both versions, because `Option` and `Result` both have `unwrap_or`. One user survives the change and another does not. That is the reason for a rule that looks at the change and not at one program: a change of a return type is *major*, whoever happens to survive it.

The compatible way to make this change is to add and not to replace: a new function `try_parse_port` that returns the `Result`, with `parse_port` left as it is. Lesson 07 shows how to retire the old one.

**Mistakes people really make.**

*"It compiles for me, so it is not breaking."* The class of a change is a statement about every program that was correct, not about the programs you have. In the lesson, one change breaks nothing in the test program and is major all the same.

*"The compiler found nothing, so it is not breaking."* Behaviour is part of the interface when it is documented. If `names()` is documented to return sorted names and a release stops sorting, every program still compiles, and the ones that relied on the documentation are now wrong. This is the most expensive kind of breaking change, because nothing announces it.

*A minor change that does break.* A new public function is minor. It can still collide with a user's names when the user imports with a glob (`*`) from two places:

```rust,compile_fail,E0659
mod paint {
    pub fn mix() -> &'static str { "paint" }
}
mod audio {
    // New in this release. Before it, only `paint` had a `mix`.
    pub fn mix() -> &'static str { "audio" }
}
use audio::*;
use paint::*;

fn main() {
    assert_eq!(mix(), "paint");
}
```

```text
error[E0659]: `mix` is ambiguous
  --> src/main.rs:12:16
   |
12 |     assert_eq!(mix(), "paint");
   |                ^^^ ambiguous name
   |
   = note: ambiguous because of multiple glob imports of a name in the same module
```

The rules call this acceptable for two reasons. The user can repair it with one line (`use paint::mix;`), and without this rule no library could ever add a function. As a user, the lesson is: glob imports from crates you do not control are a risk you take.

**When to use which bump.** Ask in this order. Does any item or documented behaviour that a user could rely on go away or change its meaning? Then major. Does the release add anything public? Then minor. Otherwise patch. When in doubt between minor and major, find the breaking program or convince yourself that none exists; lesson 02 trains exactly that. Tools help: `cargo semver-checks` (a separate tool, installed with `cargo install cargo-semver-checks`) compares your public interface with the last release and reports many of the changes on the official list. It cannot see behaviour, so it supports the review and does not replace it.

**Read more.**

- [SemVer Compatibility](https://doc.rust-lang.org/cargo/reference/semver.html) in the Cargo book: the official list, with an example for every entry.
- [Specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#default-requirements) in the Cargo book: caret requirements and the rule for `0.y.z`.
- [RFC 1105, API evolution](https://rust-lang.github.io/rfcs/1105-api-evolution.html): the reasoning behind the list.

## Lesson 02 · Traits, bounds and auto traits

**What is assumed.** Lesson 01. You know what a trait object (`dyn Trait`) is, and what `Send` and `Sync` mean.

**The mental model.** For a struct there is one kind of user: code that holds a value. A trait has two kinds, and they want opposite things.

- A *caller* uses the trait: calls its methods, writes `T: Trait`, stores a `Box<dyn Trait>`. A new method is a gift to a caller.
- An *implementor* writes `impl Trait for MyType`. A new method is a debt for an implementor: the `impl` block is incomplete until it is paid.

So for every change to a trait, ask who can be on the other side. If users can implement the trait, a new method without a body is a major change, because every implementation stops compiling. If the method has a body (a *provided* method), implementations stay valid, and the class is "possibly-breaking": the new name can still collide with a method of another trait that the user calls on the same type.

Generic bounds work like a contract with two sides. A bound on a parameter is what you *ask* of the caller. Asking for more (tightening, `T: Read` to `T: Read + Send`) can break a caller whose type does not have the extra trait: major. Asking for less (loosening) breaks nobody: minor.

**The part that no signature shows: auto traits.** `Send`, `Sync`, `Unpin`, `UnwindSafe` and `RefUnwindSafe` are *auto traits*. Nobody writes `impl Send for Session`. The compiler implements an auto trait for a struct exactly when every field has it. This is a rule about fields, including private ones. So the set of auto traits of your type is part of your public interface, and you can change it by editing a private field.

```rust,compile_fail,E0277
use std::rc::Rc;

pub struct Session {
    user: String,
    // Added in a patch release, to share the settings cheaply.
    settings: Rc<Vec<String>>,
}

fn assert_send<T: Send>() {}

fn main() {
    assert_send::<Session>();
}
```

```text
error[E0277]: `Rc<Vec<String>>` cannot be sent between threads safely
  --> src/main.rs:12:19
   |
12 |     assert_send::<Session>();
   |                   ^^^^^^^ `Rc<Vec<String>>` cannot be sent between threads safely
   |
   = help: within `Session`, the trait `Send` is not implemented for `Rc<Vec<String>>`
note: required because it appears within the type `Session`
```

A user who moved a `Session` into a thread compiled yesterday and does not compile today. No public signature changed. The official list has no entry with the words "auto trait"; the change is still breaking in the plain sense of lesson 01, because a trait implementation that users relied on is gone.

The same leak exists for `impl Trait` in return position. A signature `fn words(text: &str) -> impl Iterator<Item = &str>` promises one trait. The compiler still knows the hidden type, and it answers "is this `Send`?" from the hidden type. If you change the body so that the hidden type contains an `Rc` or a `Box<dyn Iterator>`, callers that send the iterator to a thread stop compiling. Only auto traits leak in this way. A caller cannot use `Clone` or `DoubleEndedIterator` of the hidden type unless the signature names them.

**A worked example: protect the promise with a test.** The fix for an invisible promise is to make it visible in your own test suite. This function compiles only when the bound holds, so a field that removes `Send` fails your build, not your users':

```rust
use std::sync::Arc;

pub struct Session {
    user: String,
    settings: Arc<Vec<String>>,
}

// A compile-time check. It costs nothing at run time and needs no test runner.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Session>();
};

fn main() {
    let session = Session { user: String::from("ada"), settings: Arc::new(Vec::new()) };
    let handle = std::thread::spawn(move || session.user.len() + session.settings.len());
    assert_eq!(handle.join().unwrap(), 3);
}
```

The API Guidelines ask for exactly this (C-SEND-SYNC): types are `Send` and `Sync` where possible, and a test says so.

**Trait objects.** A trait can be used as `dyn Trait` only when it is *dyn compatible* (older texts say "object safe"). Roughly: every method must be callable through a table of function pointers, by code that does not know the concrete type. So a method must take `self` in some form (`&self`, `&mut self`, `self`, `Box<Self>`), must not have type parameters, and must not mention `Self` anywhere else in its signature, for example as the return type. A method that breaks one of these rules may stay in the trait when it is marked `where Self: Sized`; it is then missing from `dyn Trait`. If your trait is dyn compatible today, users may hold it as `Box<dyn Trait>`. A new method with a type parameter takes that away from them, even when it has a body:

```rust
pub trait Encoder {
    fn encode(&self, value: u32) -> String;

    // `where Self: Sized` keeps the method out of `dyn Encoder`. Callers with
    // a concrete type can use it; the trait stays dyn compatible.
    fn encode_all<I: IntoIterator<Item = u32>>(&self, values: I) -> Vec<String>
    where
        Self: Sized,
    {
        values.into_iter().map(|value| self.encode(value)).collect()
    }
}

struct Hex;

impl Encoder for Hex {
    fn encode(&self, value: u32) -> String {
        format!("{value:x}")
    }
}

fn main() {
    let boxed: Box<dyn Encoder> = Box::new(Hex);
    assert_eq!(boxed.encode(255), "ff");
    assert_eq!(Hex.encode_all([10, 11]), ["a", "b"]);
}
```

Without the `where Self: Sized` clause, the first line of `main` is rejected:

```text
error[E0038]: the trait `Encoder` is not dyn compatible
  --> src/main.rs:18:24
   |
18 |     let boxed: Box<dyn Encoder> = Box::new(Hex);
   |                        ^^^^^^^ `Encoder` is not dyn compatible
   |
...
 4 |     fn encode_all<I: IntoIterator<Item = u32>>(&self, values: I) -> Vec<String> {
   |        ^^^^^^^^^^ ...because method `encode_all` has generic type parameters
```

**Mistakes people really make.**

*Adding a required method "because all our implementations have it".* Your implementations are not the only ones unless the trait is sealed (lesson 03). Suppose a trait `Shape` with one method `area` gains a second one, `perimeter`, without a body. A user's `impl` block that was complete yesterday fails with `E0046`:

```text
error[E0046]: not all trait items implemented, missing: `perimeter`
  --> src/main.rs:11:1
   |
 5 |         fn perimeter(&self) -> f64;
   |         --------------------------- `perimeter` from trait
...
11 | impl geometry::Shape for Square {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ missing `perimeter` in implementation
```

*Adding a bound "for later".* A bound that the body does not use still binds every caller. Add a bound on the day the body needs it, and treat that day as a major release.

*Believing that a private field is private in every respect.* Its type decides the auto traits, the size and the drop behaviour of your public type.

**When to use which.** New capability for a trait that users implement: a provided method, and say in the release notes that a name was added. New capability that every implementation must supply: a new trait (often a subtrait), or a major release. A trait that only you should implement: seal it on the first day. A new required method then has no implementor to break, and only the small risk of a provided method is left (a clash of names on the callers' side). A helper with type parameters on a trait that is used as `dyn`: `where Self: Sized`.

**Read more.**

- [SemVer Compatibility: trait changes](https://doc.rust-lang.org/cargo/reference/semver.html#trait-new-item-no-default) and [generic bounds](https://doc.rust-lang.org/cargo/reference/semver.html#generic-bounds-tighten) in the Cargo book.
- [Auto traits](https://doc.rust-lang.org/reference/special-types-and-traits.html#auto-traits) and [dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility) in the Rust Reference.
- [`impl Trait`](https://doc.rust-lang.org/reference/types/impl-trait.html) in the Rust Reference, including what an abstract return type hides and what it does not.
- [C-SEND-SYNC](https://rust-lang.github.io/api-guidelines/interoperability.html#c-send-sync) and [C-OBJECT](https://rust-lang.github.io/api-guidelines/flexibility.html#c-object) in the Rust API Guidelines.

## Lesson 03 · Room to change

**What is assumed.** Lessons 01 and 02: you know which changes are major.

**The mental model.** Every capability you give a user today is a change you cannot make tomorrow. If users can write `Config { retries: 3, base_ms: 100 }`, you cannot add a field. If users can match every variant, you cannot add a variant. If users can implement your trait, you cannot add a required method. Future-proofing means: before 1.0.0, take away the capabilities that users do not need, so that the matching changes stay minor. Rust gives you four tools. Each one removes exactly one capability.

| Tool | The user can no longer | You can later |
| --- | --- | --- |
| private fields | build the struct with a struct expression, read fields, match on all fields | add, remove and retype fields |
| `#[non_exhaustive]` | match an enum without a wildcard arm; build or fully destructure a struct or variant | add variants and fields |
| a sealed trait | implement the trait | add required items to the trait |
| a newtype | see the type inside | replace the type inside |

**Private fields** are the default tool for structs. A struct with at least one private field can only be built by your functions, so your constructor is the single place where the rules of the type (its *invariants*) are checked. This is also why "all fields private, plus getters" is the normal shape of a public struct in a library.

**`#[non_exhaustive]`** is for the types whose fields or variants you *want* to be public. The attribute has no effect inside the crate that defines the type. For other crates it means: this list may grow. The standard library uses it for `std::io::ErrorKind`, so every program that matches on an error kind already has a wildcard arm:

```rust,compile_fail,E0004
use std::io::ErrorKind;

fn advice(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::NotFound => "check the path",
        ErrorKind::PermissionDenied => "check your rights",
    }
}

fn main() {
    println!("{}", advice(ErrorKind::NotFound));
}
```

```text
error[E0004]: non-exhaustive patterns: `_` not covered
   --> src/main.rs:4:11
    |
  4 |     match kind {
    |           ^^^^ pattern `_` not covered
    |
    = note: the matched value is of type `ErrorKind`
    = note: `ErrorKind` is marked as non-exhaustive, so a wildcard `_` is necessary to match exhaustively
```

On a struct with public fields, the attribute forbids struct expressions and patterns without `..` in other crates. Users then need a constructor from you, for example `Options::default()` followed by assignments to the public fields.

**A sealed trait** is a public trait with a supertrait that users cannot name. The usual form puts a public trait into a private module:

```rust
mod export {
    mod sealed {
        pub trait Sealed {}
    }

    /// A file format. Sealed: only this module implements it.
    pub trait Format: sealed::Sealed {
        fn extension(&self) -> &'static str;
    }

    pub struct Csv;
    impl sealed::Sealed for Csv {}
    impl Format for Csv {
        fn extension(&self) -> &'static str {
            "csv"
        }
    }
}

use export::{Csv, Format};

// Users can name the trait, call it, and use it as a bound.
fn file_name<F: Format>(stem: &str, format: &F) -> String {
    format!("{stem}.{}", format.extension())
}

fn main() {
    assert_eq!(file_name("report", &Csv), "report.csv");
}
```

Why is this a guarantee and not only a hint? To implement `Format` for a type `Xml`, the compiler requires `Xml: Sealed`. An implementation of `Sealed` must name the trait, and the path `export::sealed::Sealed` crosses a private module. So no code outside `export` can write that implementation, and therefore no code outside can implement `Format`. The trait `Sealed` is declared `pub` so that the public trait `Format` may mention it without a warning (the lint `private_bounds`); it is the module that is private. The guarantee holds as long as no public path leads to `Sealed`. If you re-export it anywhere public, even with `#[doc(hidden)]`, the seal is open.

A user who writes `impl export::Format for Xml` gets a message that names the pattern (shortened here):

```text
error[E0277]: the trait bound `Xml: Sealed` is not satisfied
  --> src/main.rs:22:25
   |
22 | impl export::Format for Xml {
   |                         ^^^ unsatisfied trait bound
   |
   = note: `Format` is a "sealed trait", because to implement it you also need to implement `export::sealed::Sealed`, which is not accessible; this is usually done to force you to use one of the provided types that already implement it
```

And a user who tries to implement the supertrait first is stopped by privacy: `error[E0603]: module `sealed` is private`.

**A newtype** is a struct with one private field. It hides which type does the work. Two uses matter for a library. First, a value with a rule: `struct SensorId(NonZeroU32)` can only be made by your constructor, and its representation can change. Second, a return type: if a function returns `std::slice::Iter<'_, T>` or `std::vec::IntoIter<T>`, that standard type is your interface, with all its methods and traits, for ever. A newtype of your own that implements `Iterator` promises only what you implement. `impl Iterator<Item = T>` hides the type too, but users cannot name it in a struct field or a signature, and it leaks auto traits (lesson 02). The guideline C-NEWTYPE-HIDE prefers the newtype for a public interface.

A newtype costs nothing at run time. The compiler gives it the size of its field, and a wrapper around a type that has an impossible value, such as `NonZeroU32`, still lets `Option` use that value for `None`; the lesson measures both. Only the attribute `#[repr(transparent)]` turns "same layout as the field" into a guarantee of the language. You need that guarantee when the type crosses a boundary to C or is reinterpreted by unsafe code, not for an ordinary library type.

**Mistakes people really make.**

*Adding `#[non_exhaustive]` after the release.* The attribute takes capabilities away, so adding it later is a major change: the user's struct expression fails with `E0639`, the user's `match` with `E0004`. Decide before 1.0.0.

*Sealing with `#[doc(hidden)]`.* A hidden public trait can still be named and implemented. Hiding is a request, privacy is a rule.

*Using every tool everywhere.* Each tool costs your users something real. A `#[non_exhaustive]` enum forces a wildcard arm, so the compiler no longer tells the user about a variant they forgot. A struct with private fields cannot be built in a `const` unless you provide a `const fn`. A sealed trait cannot be mocked in the user's tests. A type that is complete by its nature (`Ordering`, a 2D point, the days of the week) should stay open.

**When to use which.** Private fields: always, unless the struct is plain data whose fields are its meaning. `#[non_exhaustive]`: on error-kind enums, option structs with public fields, and any enum that describes an open set. Sealed trait: when the trait exists so that *your* functions can be generic over *your* types. Newtype: for values with rules, and for every iterator or handle type that you return.

**Read more.**

- [The `non_exhaustive` attribute](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute) in the Rust Reference.
- [Visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html) in the Rust Reference.
- [Future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html) in the Rust API Guidelines: C-SEALED, C-STRUCT-PRIVATE, C-NEWTYPE-HIDE.
- [`std::num::NonZero`](https://doc.rust-lang.org/std/num/struct.NonZero.html): the layout guarantee for `Option`.

## Lesson 04 · An error type for a library

**What is assumed.** Chapter 7: `Result`, `?`, the `Error` trait and `Box<dyn Error>`. Lesson 03: private fields and `#[non_exhaustive]`.

**The mental model.** An error value has three readers, and each needs something different.

1. **The calling code** wants to *decide*: retry, use a default, show a form again. It needs something it can match on.
2. **A person** wants to *understand*: a short text, usually printed by a program you did not write, often in the middle of a longer line.
3. **A reporter** (a logging library, or `main` returning an error) wants to *trace*: it walks from the error to its cause, and to the cause of the cause, with `Error::source`.

An error type of a library serves all three, and it is public interface like any other type, so the rules of lessons 01 to 03 apply to it. That last point is what people forget: the day you add a failure, you change the error type.

**Two shapes.** The first shape is a public enum with one variant per failure. It is simple, and callers can match. Every variant and every field in it is a promise. Without `#[non_exhaustive]` you can never add a failure. And if a variant holds an error of another crate (`Json(serde_json::Error)`), that crate's type is now part of your interface: you cannot replace the JSON library without a major release.

The second shape is the one `std::io::Error` uses: an opaque struct with private fields, plus a small `kind()` enum.

```rust
use std::error::Error;
use std::fmt;
use std::num::ParseFloatError;

/// What went wrong, for code that wants to decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    Empty,
    NotANumber,
    BelowAbsoluteZero,
}

/// An error from `parse_celsius`.
#[derive(Debug)]
pub struct TemperatureError {
    kind: ErrorKind,
    // Private: callers reach it only as `&dyn Error`, through `source()`.
    source: Option<ParseFloatError>,
}

impl TemperatureError {
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for TemperatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Lower case, no full stop, and no text of the source.
        f.write_str(match self.kind {
            ErrorKind::Empty => "no temperature given",
            ErrorKind::NotANumber => "temperature is not a number",
            ErrorKind::BelowAbsoluteZero => "temperature is below absolute zero",
        })
    }
}

impl Error for TemperatureError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|source| source as &(dyn Error + 'static))
    }
}

pub fn parse_celsius(text: &str) -> Result<f64, TemperatureError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(TemperatureError { kind: ErrorKind::Empty, source: None });
    }
    let value: f64 = text
        .parse()
        .map_err(|source| TemperatureError { kind: ErrorKind::NotANumber, source: Some(source) })?;
    if value < -273.15 {
        return Err(TemperatureError { kind: ErrorKind::BelowAbsoluteZero, source: None });
    }
    Ok(value)
}

/// What a reporter does: print the error and every cause.
fn report(error: &dyn Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(error) = cause {
        text.push_str(": ");
        text.push_str(&error.to_string());
        cause = error.source();
    }
    text
}

fn main() {
    assert_eq!(parse_celsius(" 21.5 ").unwrap(), 21.5);

    let error = parse_celsius("warm").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotANumber);
    assert_eq!(report(&error), "temperature is not a number: invalid float literal");

    let cold = parse_celsius("-300").unwrap_err();
    assert_eq!(cold.kind(), ErrorKind::BelowAbsoluteZero);
    assert!(cold.source().is_none());
}
```

Read the example from the point of view of each reader. The code matches on `kind()`, with a wildcard arm because the enum is `#[non_exhaustive]`. The person reads the `Display` text. The reporter finds the `ParseFloatError` through `source()`. And you, the maintainer, can add a field (a line number), add a kind, or stop using the standard float parser, all in minor releases, because none of that is visible in a signature.

**The chain invariant.** Each error in a chain describes one level, in its own words, once. So a `Display` implementation must not include the text of its source. Reporters print the whole chain; if your text already contains the cause, the cause is printed twice ("temperature is not a number: invalid float literal: invalid float literal"). The other half of the invariant: if you put the cause into your text, do not also return it from `source()`. Choose one; for a library, `source()` is the better choice, because the cause stays a value that callers can inspect.

**`Send + Sync + 'static`.** Applications move errors between threads and put them into `Box<dyn Error + Send + Sync>`. The `?` operator performs that conversion only for errors that are `Send`, `Sync` and `'static`. `Send` and `Sync` are auto traits again (lesson 02), and `'static` means that the error holds no borrowed data. So one field decides:

```rust,compile_fail,E0277
use std::error::Error;
use std::fmt;
use std::rc::Rc;

#[derive(Debug)]
struct LookupError {
    name: Rc<str>,
}

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no entry for `{}`", self.name)
    }
}

impl Error for LookupError {}

fn find(name: &str) -> Result<u32, LookupError> {
    Err(LookupError { name: name.into() })
}

fn run() -> Result<u32, Box<dyn Error + Send + Sync>> {
    let id = find("carol")?;
    Ok(id)
}

fn main() {
    println!("{:?}", run());
}
```

```text
error[E0277]: `?` couldn't convert the error: `Rc<str>: Send` is not satisfied
  --> src/main.rs:23:27
   |
22 | fn run() -> Result<u32, Box<dyn Error + Send + Sync>> {
   |             ----------------------------------------- required `Rc<str>: Send` because of this
23 |     let id = find("carol")?;
   |              -------------^ `Rc<str>` cannot be sent between threads safely
   |
   = help: within `LookupError`, the trait `Send` is not implemented for `Rc<str>`
   = note: required for `Box<dyn std::error::Error + Send + Sync>` to implement `From<LookupError>`
```

The fix is a `String` or an `Arc<str>` in the field. Put the check from lesson 02 (`assert_send_sync::<MyError>()`) into the tests of every error type.

**Mistakes people really make.**

*`Result<T, String>`.* A text serves the person and nobody else. The caller cannot decide without parsing your sentence, and from then on your wording is interface.

*`Box<dyn Error>` or `anyhow::Error` in a public signature of a library.* That serves the reporter and takes the decision away from the caller. Opaque error types are right for applications; the ecosystem lesson E01 draws that line.

*One variant per function, or one enum for the whole crate.* Callers then match on variants that the function they called can never return. Group by what the caller would do differently.

*Messages with capital letters and full stops.* Your text is usually printed as part of a longer line, such as `error: could not read config: no temperature given`. The convention of the standard library (lower case, no trailing punctuation) makes such lines read well.

**When to use which.** A public enum (with `#[non_exhaustive]`) is fine when the failures are few, are plain data, and hold no types of other crates. Use the struct-with-kind shape when the error needs context that may grow, when it wraps errors of other crates, or when the crate is meant to live for years. Use an opaque boxed error only in binaries and tests.

**Read more.**

- [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html): `source`, and the conversions to `Box<dyn Error>`.
- [`std::io::Error`](https://doc.rust-lang.org/std/io/struct.Error.html) and [`std::io::ErrorKind`](https://doc.rust-lang.org/std/io/enum.ErrorKind.html): the model for the struct-with-kind shape.
- [C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err) in the Rust API Guidelines.

## Lesson 05 · Features that only add

**What is assumed.** Chapter 8: manifests, `[dependencies]`, `#[cfg(...)]`. The chapter 8 lab built one feature with an optional path dependency; this lesson is about designing a set of features that cannot break each other.

**The mental model.** A Cargo *feature* is a name in your manifest that a user can turn on. In your code, `#[cfg(feature = "json")]` includes an item only when the feature is on. That part is simple. The part that decides the design is how Cargo combines requests:

> In one build there is one copy of your crate (one per incompatible version, in the sense of lesson 01), and it is compiled with the *union* of the features that all crates in the build ask for.

This is called *feature unification*. Picture a workspace with two programs:

```text
reader ──── lamp (no features)  ─┐
                                 ├──▶ one build of `lamp`, features = { bright }
stage  ──── lamp + "bright"     ─┘
```

`reader` never asked for `bright`, and it gets it. Nobody can turn a feature *off* for a crate once any other crate in the build has turned it on. (The union is taken over what is really built together. A request that comes only from a build script, only from a dependency for another platform, or only from tests that are not being built does not count for your normal build. These exceptions make the union smaller; they never switch a feature off.) From this single fact the central rule follows:

> **A feature may only add.** Turning a feature on must never remove an item, change a signature, or change what existing code means.

If that holds, every crate in the build sees at least the interface it asked for, whatever its neighbours ask for. If it does not hold, two correct crates break each other when they meet in one build, and neither author did anything wrong. The Cargo book puts it this way: enabling a feature should be a SemVer-compatible change, in the sense of lesson 01.

**A worked example.** A library for exporting tables, `tablekit`, has an optional YAML output. The manifest:

```text
[dependencies]
yamlwriter = { version = "2", optional = true }

[features]
default = ["std"]
std = []
yaml = ["dep:yamlwriter"]
```

Three things are worth a comment.

`optional = true` means the dependency is not built unless a feature asks for it. Users who do not need YAML do not download or compile `yamlwriter`.

`dep:yamlwriter` is the way to say "this feature turns the dependency on". Without any `dep:` mention, Cargo creates a feature with the *name of the dependency*: users would write `features = ["yamlwriter"]`, and the name of a crate you happen to use today becomes part of your interface. With `dep:`, the public name is `yaml`, your word, and you can change the library behind it. Cargo explains the difference when a user asks for the hidden name:

```text
error: package `tablekit v1.2.0 (...)` does not have feature `yamlwriter`

help: an optional dependency with that name exists, but the `features` table includes it with the "dep:" syntax so it does not have an implicit feature with that name
Dependency `yamlwriter` would be enabled by these features:
	- `yaml`
```

`default = ["std"]` lists what a user gets without saying anything. A user opts out with `default-features = false`. Note the direction: the feature is `std` and is on by default. It is not `no-std` and off by default. A feature called `no-something` removes things, so it breaks the rule above.

In the code, the feature guards whole items:

```rust
pub struct Table {
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new() -> Table {
        Table { rows: Vec::new() }
    }

    pub fn push(&mut self, row: &[&str]) {
        self.rows.push(row.iter().map(|cell| cell.to_string()).collect());
    }

    /// Always there, with the same signature in every configuration.
    pub fn to_csv(&self) -> String {
        self.rows.iter().map(|row| row.join(",") + "\n").collect()
    }

    /// Only with the feature `yaml`. An extra method, not a changed one.
    #[cfg(feature = "yaml")]
    pub fn to_yaml(&self) -> String {
        yamlwriter::sequence(&self.rows)
    }
}

fn main() {
    let mut table = Table::new();
    table.push(&["id", "name"]);
    table.push(&["1", "ada"]);
    assert_eq!(table.to_csv(), "id,name\n1,ada\n");
}
```

(This block is compiled without the feature, so the YAML method is left out, and the crate `yamlwriter` is never looked for.)

**Mistakes people really make.**

*A feature that changes a type.* "With `wide`, `Channel` is `u16` instead of `u8`." A user without the feature writes `let byte: u8 = color.r;`. In a build where anybody turns `wide` on, that line is a type error in a crate that did not change. Add a second type or a second function instead.

*Features that exclude each other.* "Choose `backend-a` or `backend-b`", guarded by `compile_error!` when both are on. Unification will turn both on as soon as two dependencies of one program choose differently, and the program cannot be built at all. Let both be on at once and choose at run time, or give one a fixed priority, or split the backends into separate crates.

*Testing only the default.* Each combination is a different program. Code behind `#[cfg(feature = "a")]` that uses an item behind `#[cfg(feature = "b")]` compiles only when both are on, and you will not notice with `cargo test`. With `n` independent features there are `2^n` combinations; for small `n`, check them all:

```text
cargo check --no-default-features
cargo check --no-default-features --features json
cargo check --no-default-features --features json,wide
cargo check --all-features
```

*Removing a feature.* A user's manifest names it, so their build fails at once:

```text
error: failed to select a version for `swatch`.
...
package `exporter` depends on `swatch` with feature `json` but `swatch` does not have that feature.
```

Removing a feature, or removing an optional dependency whose implicit feature somebody may use, is on the list of lesson 01. Adding a feature is minor.

**When to use which.** A feature is right for something that many users do not need and that costs them compile time, a dependency, or a platform requirement. It is wrong for a choice of behaviour (use a parameter or a type) and wrong for something nearly everyone needs (just include it). Put into `default` what a newcomer expects to work; remember that removing something from `default` later breaks users who relied on it. When an optional part has its own large dependencies and its own release rhythm, a separate crate is cleaner than a feature.

**Read more.**

- [Features](https://doc.rust-lang.org/cargo/reference/features.html) in the Cargo book, in particular [feature unification](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification), [optional dependencies](https://doc.rust-lang.org/cargo/reference/features.html#optional-dependencies) and [mutually exclusive features](https://doc.rust-lang.org/cargo/reference/features.html#mutually-exclusive-features).
- [SemVer Compatibility: Cargo features](https://doc.rust-lang.org/cargo/reference/semver.html#cargo-feature-add) in the Cargo book.
- [Conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html) in the Rust Reference.
- [C-FEATURE](https://rust-lang.github.io/api-guidelines/naming.html#c-feature) in the Rust API Guidelines: names such as `std`, not `use-std` or `no-std`.

## Lesson 06 · Documentation as interface

**What is assumed.** You have written `///` comments and run `cargo doc`. Chapter 7 for `?` in examples.

**The mental model.** For a user, your library *is* its documentation page. They read signatures and sentences; they do not read your code. Two consequences. First, what the documentation says is a promise in the sense of lesson 01: if the text says "sorted", sorted is interface. Second, what the documentation does not say is not a promise, however stable it has been. So writing documentation is the act of deciding what you promise.

Rust adds something most languages do not have: the examples in documentation are compiled and run by `cargo test`. An example is therefore a test that users can read, and a promise that cannot silently become false.

**How rustdoc reads a comment.** `///` documents the item that follows; `//!` documents the item it is inside, and at the top of `lib.rs` that is the crate. The text is Markdown. The first paragraph is the summary that appears in lists, so make it one sentence that says what the item is or does. A code block is treated as Rust and becomes a test unless its label says otherwise:

| Label | Compiled | Run | Passes when |
| --- | --- | --- | --- |
| none, or `rust` | yes | yes | it does not panic |
| `no_run` | yes | no | it compiles |
| `should_panic` | yes | yes | it panics |
| `compile_fail` | yes | no | it does *not* compile |
| `ignore` | no | no | never run; listed as ignored |
| `text`, `sh`, `toml`, ... | no | no | it is not a test |

Rustdoc wraps each example in `fn main() { ... }` and makes your crate available, so examples use the library exactly as a user does: through its public paths. A line that starts with `# ` is compiled and hidden from the page. That is how an example can use `?` without showing a `main` function:

````rust
/// Parses a percentage such as `"45%"` into a fraction.
///
/// White space around the text is ignored. The value may be larger than
/// 100 %, because discounts and growth rates can be.
///
/// # Errors
///
/// Returns the error of the number parser if the text before `%` is not a
/// number, and also if the `%` sign is missing.
///
/// # Examples
///
/// ```
/// # use std::num::ParseFloatError;
/// # fn parse_percent(text: &str) -> Result<f64, ParseFloatError> {
/// #     Ok(text.trim().strip_suffix('%').unwrap_or("").parse::<f64>()? / 100.0)
/// # }
/// assert_eq!(parse_percent("45%")?, 0.45);
/// assert_eq!(parse_percent(" 150% ")?, 1.5);
/// assert!(parse_percent("half").is_err());
/// # Ok::<(), ParseFloatError>(())
/// ```
pub fn parse_percent(text: &str) -> Result<f64, std::num::ParseFloatError> {
    let number = text.trim().strip_suffix('%').unwrap_or("");
    Ok(number.parse::<f64>()? / 100.0)
}

fn main() {
    assert_eq!(parse_percent("45%").unwrap(), 0.45);
    assert!(parse_percent("45").is_err());
}
````

(In a real crate the hidden lines would be one line, `# use mycrate::parse_percent;`. Here the example carries a copy of the function, because this page is tested without a crate around it.)

The last hidden line, `# Ok::<(), ParseFloatError>(())`, gives the example a return type, so that `?` is allowed. Examples should use `?` and not `unwrap()` (guideline C-QUESTION-MARK): users copy examples.

**The three standard sections.** The API Guidelines ask for them (C-FAILURE), and readers look for them by name:

- `# Errors`: for a function that returns `Result`, which conditions give which error.
- `# Panics`: every condition under which the function panics. A panic that is not documented is a bug in the function or in the documentation.
- `# Safety`: for an `unsafe fn`, exactly what the caller must guarantee.

**Links.** Write the name of an item in square brackets and backticks and rustdoc resolves it like a path in code: ``[`Tag::parse`]``, ``[`std::io::Error`]``. These *intra-doc links* are checked: a name that does not resolve gives a warning from the lint `rustdoc::broken_intra_doc_links`. A written-out URL into your own documentation is never checked and breaks at the first rename.

**Examples as a specification.** An example can *show* a function or *specify* it. `assert_eq!(slug("Hello"), "hello")` shows that the function exists. It holds for a correct implementation and for many wrong ones. To find out what your examples specify, ask of each sentence in the text: is there an assertion that would fail if this sentence were false? The lesson does this mechanically: it builds the library with one deliberate bug at a time (a *mutant*) and runs your examples. An example that passes against a wrong library says nothing about that part of the behaviour. Good examples go to the edges that the text mentions: the empty input, the boundary, the value that is refused.

**Tools that keep documentation true.**

```text
cargo test --doc                                  run the examples
cargo doc --no-deps --open                        read the page as a user does
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps    broken links fail the build
```

In `lib.rs`, `#![warn(missing_docs)]` makes the compiler list every public item without a comment. Programs in the directory `examples/` are built by `cargo test` and run with `cargo run --example name`; they are the place for a complete use of the library that is too long for a comment. The page docs.rs builds for a published crate uses only the default features unless the manifest says otherwise under `[package.metadata.docs.rs]`, so items behind features can be missing from the page that users read.

**Mistakes people really make.**

*Repeating the signature in words.* "`fn len(&self) -> usize`: returns the length as a usize." Say what the signature cannot: the unit (bytes or characters?), the edge cases, the cost.

*A code block that is not meant to be Rust.* Shell commands and file listings in a plain block are compiled as Rust and fail. Label them `text` or `sh`. An indented block (four spaces) is also treated as Rust.

*`ignore` to silence a failing example.* The example then rots. Use `no_run` when the code must compile and cannot run (it needs a network), `compile_fail` when the point is that it is rejected, and repair it otherwise.

*Documenting the implementation.* "Uses a binary search" is a promise about the algorithm that you will regret. Promise the result and, when it matters, the cost ("logarithmic in the length").

**Read more.**

- [Documentation tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html) in the rustdoc book: all labels, hidden lines, `?` in examples.
- [Linking to items by name](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html) and [lints](https://doc.rust-lang.org/rustdoc/lints.html) in the rustdoc book.
- [Documentation](https://rust-lang.github.io/api-guidelines/documentation.html) in the Rust API Guidelines: C-CRATE-DOC, C-EXAMPLE, C-QUESTION-MARK, C-FAILURE, C-LINK.
- [Examples](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#examples) in the Cargo book.

## Lesson 07 · Deprecation, minimum Rust version, editions

**What is assumed.** Lessons 01 to 03.

**The mental model.** This lesson is about three slow changes: retiring a name, moving to a newer compiler, and moving to a newer edition. In each of them the question is the same as in lesson 01: who is forced to act, and when?

### Deprecation: a removal in two steps

Removing a public item is a major change. Deprecating it is not: the item stays and works, and the compiler tells its users to move.

```rust
pub struct Stack {
    items: Vec<u8>,
}

impl Stack {
    /// The number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// The number of items.
    #[deprecated(since = "0.8.0", note = "use `Stack::len` instead")]
    pub fn size(&self) -> usize {
        // The old name calls the new one. Two copies of the body would drift apart.
        self.len()
    }
}

fn main() {
    let stack = Stack { items: vec![1, 2, 3] };
    assert_eq!(stack.len(), 3);
    // A user who has not moved yet still compiles, with a warning.
    #[allow(deprecated)]
    let old = stack.size();
    assert_eq!(old, 3);
}
```

Without the `#[allow]`, the user sees your `note` word for word:

```text
warning: use of deprecated method `Stack::size`: use `Stack::len` instead
  --> src/main.rs:21:21
   |
21 |     let old = stack.size();
   |                     ^^^^
   |
   = note: `#[warn(deprecated)]` on by default
```

So write the note for that reader: name the replacement, in backticks, in one line. `since` is the version in which the deprecation started; it appears in the documentation. The whole path is: add the new item and deprecate the old one in a minor release; keep both for a long time; remove the old one in the next major release, which you are making for other reasons too.

A deprecation is still not free. A project that builds with `#![deny(warnings)]`, or with `RUSTFLAGS="-D warnings"` in CI, turns your warning into an error. This is the reason for a common recommendation: do not deny all warnings in the source of a library; deny them in CI, where a new warning stops a pull request and not a user. And inside your own crate, every use of the deprecated item warns too, so move your own code first.

### The minimum supported Rust version

Every library has an oldest compiler that can build it. If you do not know which, your users find out by accident. The field `rust-version` in the manifest states it:

```text
[package]
name = "oldtimer"
edition = "2021"
rust-version = "1.74"
```

Cargo uses the field in two ways. It refuses to build the package with an older compiler, with a clear message instead of a confusing compile error deep in your code:

```text
error: rustc 1.99.0 is not supported by the following package:
  fromthefuture@3.1.0 requires rustc 1.120
```

And the dependency resolver takes it into account: with resolver 3, which is the default from edition 2024 on, Cargo prefers versions of dependencies whose `rust-version` is not newer than the `rust-version` of the user's own package, or than the user's compiler when that package states none. So a correct `rust-version` lets a user on an older compiler keep getting your older releases instead of a broken build.

The field is a promise, and a promise needs a check. Three levels of checking exist:

1. Clippy's lint `clippy::incompatible_msrv` reads `rust-version` and reports every item of the standard library that is newer:

   ```text
   error: current MSRV (Minimum Supported Rust Version) is `1.74.0` but this item is stable since `1.80.0`
   ```

   It is fast and needs no second compiler. It sees library items only. It does not see new *syntax* or language rules, and it does not see what your dependencies need.
2. Building and testing with that compiler in CI: `cargo +1.74 test`. This is the only check that proves the promise.
3. Checking the dependencies: a dependency that raises its own minimum in a minor release raises yours.

Is raising `rust-version` a breaking change? The official list calls it possibly-breaking. Most projects treat it as a minor change and say so in their documentation, often with a policy such as "the last six stable releases". What matters is that you choose a policy, write it down, and keep it.

### Editions

An *edition* is a setting of one crate that selects a dialect of the language: which words are keywords, a few defaults, a few rules. Editions exist so that the language can make such changes without breaking old code. Two facts make them harmless for a library.

- **The edition is per crate.** A crate of edition 2021 and a crate of edition 2024 link together without any adapter. Your choice of edition is invisible to your users, and theirs to you. (The exception is the compiler version: edition 2024 needs Rust 1.85, so the edition sets a floor for `rust-version`.)
- **Old editions stay supported.** Nobody is forced to move.

Moving is mostly mechanical: `cargo fix --edition` rewrites what it can. Two changes of edition 2024 touch public interfaces. `gen` is a reserved word, so an item of that name must be written `r#gen` in 2024 code. And the rule for `impl Trait` in return position changed. In edition 2021, such a type could use only the lifetime parameters it named. In edition 2024, it may use *every* generic parameter in scope, lifetimes included. For a caller this means: the returned value is treated as borrowing from every reference argument, even when the body copies everything it needs.

```rust,compile_fail,E0597
fn initials(names: &[String]) -> impl Iterator<Item = char> {
    let letters: Vec<char> = names.iter().filter_map(|name| name.chars().next()).collect();
    letters.into_iter()
}

fn main() {
    let letters = {
        let names = vec![String::from("Ada"), String::from("Grace")];
        initials(&names)
    };
    assert_eq!(letters.collect::<String>(), "AG");
}
```

```text
error[E0597]: `names` does not live long enough
  --> src/main.rs:9:18
   |
 7 |     let letters = {
   |         ------- borrow later stored here
 8 |         let names = vec![String::from("Ada"), String::from("Grace")];
   |             ----- binding `names` declared here
 9 |         initials(&names)
   |                  ^^^^^^ borrowed value does not live long enough
10 |     };
   |     - `names` dropped here while still borrowed
   |
note: this call may capture more lifetimes than intended, because Rust 2024 has adjusted the `impl Trait` lifetime capture rules
help: use the precise capturing `use<...>` syntax to make the captures explicit
   |
 1 | fn initials(names: &[String]) -> impl Iterator<Item = char> + use<> {
   |                                                             +++++++
```

The bound `use<>` states which generic parameters the hidden type may use; an empty list means none. With it, the example compiles:

```rust
fn initials(names: &[String]) -> impl Iterator<Item = char> + use<> {
    let letters: Vec<char> = names.iter().filter_map(|name| name.chars().next()).collect();
    letters.into_iter()
}

fn main() {
    let letters = {
        let names = vec![String::from("Ada"), String::from("Grace")];
        initials(&names)
    };
    assert_eq!(letters.collect::<String>(), "AG");
}
```

The captured parameters are part of the signature in the sense of lesson 01. Going from `use<>` to capturing a lifetime takes a capability away from callers and is on the official list as a major change; the other direction is minor. So when a public function of a 2024 crate returns `impl Trait` and takes references, decide on purpose what the result borrows.

**Mistakes people really make.**

*Deprecating without a replacement.* The user gets a warning and no way to make it go away. Ship the new item in the same release.

*A `rust-version` that nobody tests.* It is true on the day it is written and false a month later, when somebody uses a newer method.

*Moving the edition and the public names in one release.* Edition moves are internal. Keep them separate from interface changes, so that each release has one kind of risk.

**Read more.**

- [The `deprecated` attribute](https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-deprecated-attribute) in the Rust Reference.
- [Rust version](https://doc.rust-lang.org/cargo/reference/rust-version.html) in the Cargo book, and [Rust version in the resolver](https://doc.rust-lang.org/cargo/reference/resolver.html#rust-version).
- [The Edition Guide](https://doc.rust-lang.org/edition-guide/): [RPIT lifetime capture rules](https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html) and the [`gen` keyword](https://doc.rust-lang.org/edition-guide/rust-2024/gen-keyword.html).
- [SemVer Compatibility: changing the minimum version of Rust](https://doc.rust-lang.org/cargo/reference/semver.html#env-new-rust) and [capturing more generic parameters in RPIT](https://doc.rust-lang.org/cargo/reference/semver.html#generic-rpit-capture).

## Code review 01 · An interface against the API Guidelines

**What is assumed.** Lessons 01 to 07. The review uses all of them.

**The mental model.** The Rust API Guidelines are a checklist written by the Rust library team. Each item has a short identifier, such as C-GETTER, and a page that explains it. The guidelines are not rules of the language; a crate that ignores them compiles. They record what users of Rust have come to *expect*. An interface that follows them can be used without reading much documentation, because names and traits mean what they mean everywhere else. That is the real argument for conventions: every deviation is something each user must learn and remember.

A review before 1.0.0 is the cheapest moment to apply the checklist, because after it every rename costs a deprecation cycle (lesson 07) and every changed type a major release (lesson 01).

**How to read an interface.** Do not start with the code. Generate the documentation (`cargo doc --open`) or list the public items, and go through them in this order. Each step has the guidelines that find the most.

1. **Names.** Do they follow the conventions? (C-CASE, C-CONV, C-GETTER, C-ITER, C-WORD-ORDER)
2. **Signatures.** What does each function demand of the caller, and what does it give back? (C-GENERIC, C-CALLER-CONTROL, C-CUSTOM-TYPE, C-NO-OUT, C-METHOD, C-CTOR)
3. **Traits.** Which standard traits does each type implement, and which are missing? (C-COMMON-TRAITS, C-CONV-TRAITS, C-COLLECT, C-SEND-SYNC, C-DEBUG)
4. **Failures.** How does each function report a problem? (C-GOOD-ERR, C-VALIDATE, C-FAILURE)
5. **The future.** Which changes has this interface made impossible? (C-STRUCT-PRIVATE, C-SEALED, C-NEWTYPE-HIDE)
6. **Documentation.** (C-CRATE-DOC, C-EXAMPLE)

**The naming conventions in one table.** These are the ones that reviews find most often.

| You see | Convention | Example in `std` |
| --- | --- | --- |
| `as_x(&self)` | free, borrowed view | `str::as_bytes` |
| `to_x(&self)` | expensive conversion, new value | `str::to_lowercase` |
| `into_x(self)` | consumes `self`, usually cheap | `String::into_bytes` |
| getter of a field `x` | `x(&self)`, no `get_` | `Path::extension` |
| iterators of a collection | `iter`, `iter_mut`, `into_iter` | `Vec::iter` |
| constructor | `new`; `with_x` or `from_x` for variants | `Vec::with_capacity` |
| a type with `len` | also `is_empty` | `Vec::is_empty` |

**A worked example.** Here is a small type as a hurried author might write it, with the findings as comments. It compiles; that is the problem with it.

```rust
pub struct Playlist {
    pub songs: Vec<String>,                      // C-STRUCT-PRIVATE: anyone can reorder or clear it
}

impl Playlist {
    pub fn make() -> Playlist {                  // C-CTOR: constructors are called `new`
        Playlist { songs: Vec::new() }
    }

    pub fn get_count(&self) -> usize {           // C-GETTER: no `get_`; a count is `len`
        self.songs.len()
    }

    pub fn add(&mut self, title: &String, at_front: bool) {
        // C-GENERIC: `&String` forces callers to own a String; `&str` accepts both
        // C-CUSTOM-TYPE: `playlist.add(&title, true)` says nothing at the call site
        if at_front {
            self.songs.insert(0, title.clone());
        } else {
            self.songs.push(title.clone());
        }
    }

    pub fn to_songs(self) -> Vec<String> {       // C-CONV: consumes `self`, so `into_songs`
        self.songs
    }
}
// C-COMMON-TRAITS, C-DEBUG: no `Debug`, `Clone`, `PartialEq`, `Default`

fn main() {
    let mut playlist = Playlist::make();
    playlist.add(&String::from("Blue"), true);
    assert_eq!(playlist.get_count(), 1);
    assert_eq!(playlist.to_songs(), ["Blue"]);
}
```

And the same type after the review. The call sites at the bottom show what the user gains.

```rust
/// Where a new song goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Front,
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Playlist {
    songs: Vec<String>,
}

impl Playlist {
    pub fn new() -> Playlist {
        Playlist::default()
    }

    pub fn len(&self) -> usize {
        self.songs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }

    pub fn add(&mut self, title: impl Into<String>, position: Position) {
        match position {
            Position::Front => self.songs.insert(0, title.into()),
            Position::Back => self.songs.push(title.into()),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.songs.iter().map(String::as_str)
    }

    pub fn into_songs(self) -> Vec<String> {
        self.songs
    }
}

fn main() {
    let mut playlist = Playlist::new();
    playlist.add("Blue", Position::Back);                   // a literal is enough
    playlist.add(String::from("Intro"), Position::Front);   // an owned String is moved, not copied
    assert_eq!(playlist.len(), 2);
    assert_eq!(playlist.iter().next(), Some("Intro"));
    assert_eq!(playlist.clone(), playlist);
    assert_eq!(playlist.into_songs(), ["Intro", "Blue"]);
}
```

`impl Into<String>` follows C-CALLER-CONTROL: the function needs to own the title, so it asks for ownership, and the caller decides whether that means a move or a fresh allocation. A function that only reads takes `&str`.

**How to write a finding.** A review comment that helps has three parts: what you saw, which expectation it breaks (the guideline, with its identifier), and what you suggest. "C-GETTER: `get_count` should be `len`, and then `is_empty` is expected too" can be acted on. "The naming is unusual" cannot.

**Mistakes people really make in reviews.**

*Reviewing the implementation and not the interface.* A slow loop can be repaired in any patch release. A wrong name is for ever.

*Treating the checklist as law.* Each guideline has a reason. When the reason does not apply, say so and move on. For example, C-COLLECT asks that a collection can be built with `collect()`; a collection whose items must be validated cannot offer that honestly, because `collect()` has no way to report a refused item. `FromStr` or a fallible constructor is the better answer.

*Stopping at names.* The findings that cost most later are in step 5: the public field, the open enum, the trait that anyone can implement.

**Read more.**

- [Rust API Guidelines checklist](https://rust-lang.github.io/api-guidelines/checklist.html): the whole rubric on one page; each item links to its explanation.
- [Naming](https://rust-lang.github.io/api-guidelines/naming.html), [Interoperability](https://rust-lang.github.io/api-guidelines/interoperability.html), [Predictability](https://rust-lang.github.io/api-guidelines/predictability.html), [Flexibility](https://rust-lang.github.io/api-guidelines/flexibility.html), [Type safety](https://rust-lang.github.io/api-guidelines/type-safety.html).
- [Clippy's lint list](https://doc.rust-lang.org/clippy/lints.html): `ptr_arg`, `len_without_is_empty`, `new_without_default`, `wrong_self_convention` and `should_implement_trait` find some of these automatically.

## Lesson 08 · Capstone: a release

The capstone has no new idea. It asks you to use the earlier ones together, in the order in which a maintainer works.

**Before you write code,** read the existing user. Everything that user does with the library is a constraint: its struct expressions, its `match` without a wildcard, its `impl` of your trait, its function pointers, its own traits on your types. Write the list down. Then read each request and sort it: can it be done by adding? If the obvious way changes something on your list, look for the additive way.

| The request seems to need | The compatible way |
| --- | --- |
| a new field in a struct that users build | a new type with private fields; a conversion from the old one |
| a new variant in an enum that users match | a new error type for the new functions |
| a new name for a function | the new name, and the old one deprecated (lesson 07) |
| a new method in a trait that users implement | a provided method (lesson 02) |

Each row is also a lesson about version 1.0.0: private fields, `#[non_exhaustive]` and a sealed trait would have made the direct way possible. When you design the new types, do for them what 1.0.0 did not do.

**A release checklist.** Real projects keep one like this in their repository.

1. The users you know of still build and pass, untouched.
2. The new interface has been reviewed like any interface (code review 01), because it is about to become permanent.
3. Every public item is documented; examples run; the library builds without warnings.
4. The version number follows from the changes (lesson 01), not from how large the release feels.
5. The changelog names every addition, deprecation and repair, in words a user understands, and says what to do about each deprecation.
6. `cargo publish --dry-run` builds the package exactly as it would be uploaded. `cargo package --list` shows which files are in it.

A published version cannot be changed or deleted, only *yanked*: `cargo yank` stops new projects from choosing it and leaves existing lock files working. So the checks come before the upload.

**Read more.**

- [Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html) in the Cargo book, including `cargo yank`.
- [C-RELNOTES](https://rust-lang.github.io/api-guidelines/documentation.html#c-relnotes) and [C-METADATA](https://rust-lang.github.io/api-guidelines/documentation.html#c-metadata) in the Rust API Guidelines.
- [SemVer Compatibility](https://doc.rust-lang.org/cargo/reference/semver.html) in the Cargo book, once more: read the whole list now that you have used most of it.
