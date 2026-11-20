# Chapter 7 concepts — Handling errors

[Chapter guide](README.md) · [Hints](HINTS.md) · [Optional labs](LABS.md)

Read one section, then do the lesson with the same number. Every code block
in this file is compiled and run by `rustdoc --edition 2024 --test`, and every
diagnostic was copied from the pinned compiler (Rust 1.99.0) and shortened.
File names such as `a1.rs` in a diagnostic are the scratch files in which the
snippet was compiled.

## What you will be able to do

After this chapter you will be able to:

- decide whether a failure is a bug (panic) or an expected event (`Result`);
- handle a `Result` with `match`, with `?`, and with its methods;
- return several kinds of error through one function, with a boxed error or
  with your own error type;
- write an error type that prints a clear message and keeps its cause;
- say honestly when an error cannot happen, and when you choose to ignore one;
- send a boxed error to another thread, and decide what `main` returns.

## Before you start

Chapters 3 to 6: integer types and `parse`,
ownership and borrowing, `match`, loops, and `Option`. You do not need to
know traits in depth. This chapter uses a few traits from the standard
library (`Display`, `Debug`, `Error`, `From`) and explains each one when it
appears. Chapter 11 covers traits in general.

## Time

These are estimates for a first pass, including the reading:

| Part | Estimate |
| --- | --- |
| Lesson 01 · Panics and invariants | 35 minutes |
| Lesson 02 · Catching errors | 30 minutes |
| Lesson 03 · The `?` operator | 35 minutes |
| Lesson 04 · Result methods and aliases | 40 minutes |
| Lesson 05 · Boxed errors | 45 minutes |
| Lesson 06 · Custom error types | 50 minutes |
| Lesson 07 · Impossible and ignored errors | 35 minutes |
| Lesson 09 · Thread-safe boxed errors, and `main` | 60 minutes |
| Lesson 08 · Capstone | 60 minutes |
| Review, code review and drills | 2 hours |

Lesson 09 was added after the capstone was numbered. Do it before lesson 08:
the capstone asks you to explain how `main` reports a failure, and lesson 09
teaches that.

## Lesson 01 · Panics and invariants

**Assumed:** functions, `if`, and the idea from chapter 4 that a value's
destructor (`Drop`) runs when its owner goes out of scope.

Rust separates two kinds of failure.

- An **expected failure** is something the outside world can cause: a user
  types a letter where a number belongs, or a file is missing. The program is
  correct; the input is not. Rust reports these as an ordinary return value
  of type `Result`.
- A **bug** is a broken *invariant*. An invariant is a fact that the code
  relies on and that the programmer promised to keep true, such as "this
  slice is never empty here". When an invariant is broken the program cannot
  trust its own state, so it stops what it is doing. That stop is a
  **panic**.

`panic!("message")` starts a panic. So do `assert!`, `assert_eq!`,
`unreachable!`, indexing past the end of a slice, and `unwrap` on an `Err`.

When a thread panics, three things happen by default:

1. The *panic hook* prints the message and the source location to standard
   error.
2. The thread **unwinds**: it leaves each function it was inside, one after
   another, and runs the destructor of every local value on the way, exactly
   as it would at the end of each scope.
3. The thread ends. If it was the main thread, the process exits with
   status 101.

Here is a function with an invariant, next to a function whose failure is
expected:

```rust,should_panic
struct Lease(&'static str);

impl Drop for Lease {
    fn drop(&mut self) {
        println!("released {}", self.0);
    }
}

/// Mean of a non-empty slice. Calling it with no samples is a caller bug.
fn mean(samples: &[f64]) -> f64 {
    assert!(!samples.is_empty(), "mean of zero samples");
    samples.iter().sum::<f64>() / samples.len() as f64
}

/// A share of a whole. A zero whole is normal data, so it is an `Err`.
fn share(part: u32, whole: u32) -> Result<f64, &'static str> {
    if whole == 0 {
        Err("the whole is zero")
    } else {
        Ok(f64::from(part) / f64::from(whole))
    }
}

fn main() {
    assert_eq!(share(1, 4), Ok(0.25));
    assert_eq!(share(1, 0), Err("the whole is zero"));

    let _lease = Lease("printer");
    let nothing: [f64; 0] = [];
    println!("{}", mean(&nothing));
    println!("this line is never reached");
}
```

Running it prints this (the number after `'main'` is a thread id and changes
on every run):

```text
thread 'main' (122041812) panicked at u1.rs:11:5:
mean of zero samples
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
released printer
```

The line `released printer` comes *after* the panic message. The destructor
of `_lease` ran during unwinding. That is why a panic does not leak files,
locks or memory.

A program can also be built so that a panic **aborts**: the process is
stopped at once and no destructor runs. You choose this in `Cargo.toml` with
`panic = "abort"` in a profile. The same program built that way printed the
panic message but not `released printer`, and the shell on the test machine
(macOS) reported status 134 instead of 101.

`std::panic::catch_unwind` runs a closure and returns `Err` if the closure
panicked. The `Err` holds the *payload*, the value the panic carried:

```rust
use std::panic::catch_unwind;

let outcome = catch_unwind(|| {
    let empty: Vec<u8> = Vec::new();
    empty[0]
});
assert!(outcome.is_err());
```

The tests in this chapter use `catch_unwind` to look at panics. It is not a
general tool for handling errors: it exists for boundaries, such as a thread
pool that must survive a panicking job. It also cannot catch an abort.

**Mistakes people make**

*Panicking on input.* `unwrap` is short, so it ends up on values that come
from a user:

```rust,should_panic
let width: u32 = "wide".parse().unwrap();
```

```text
thread 'main' (122014877) panicked at a1.rs:2:37:
called `Result::unwrap()` on an `Err` value: ParseIntError { kind: InvalidDigit }
```

The user sees a message written for programmers, and the caller cannot
recover. Return the `Result` instead (lessons 02 and 03).

*Using `catch_unwind` as "try/catch".* The compiler resists this. A closure
that changes outside state through `&mut` is rejected:

```rust,compile_fail,E0277
use std::panic::catch_unwind;

let mut log = Vec::new();
let outcome = catch_unwind(|| {
    log.push(1);
    panic!("stop");
});
```

```text
error[E0277]: the type `&mut Vec<i32>` may not be safely transferred across an unwind boundary
  = help: within `{closure@a2.rs:5:32: 5:34}`, the trait `UnwindSafe` is not implemented for `&mut Vec<i32>`
  = note: `UnwindSafe` is implemented for `&Vec<i32>`, but not for `&mut Vec<i32>`
```

The reason: after a caught panic, `log` could be half-updated. If the
failure is one you plan to handle, it is not a panic; it is a `Result`.

*Guarding safety with `debug_assert!`.* `debug_assert!` is checked only in
builds with debug assertions (the default for `cargo build` and `cargo test`,
but not for `--release`). Use `assert!` for an invariant that must hold in
every build.

**When to use which**

| Situation | Use |
| --- | --- |
| Input from a user, a file or the network is wrong | `Result` |
| The caller broke a documented rule of your function | panic (`assert!`) |
| A case that your own logic makes impossible | `unreachable!` or `expect` with the reason |
| A test checks a value | `assert!`, `assert_eq!` |

**Read more**

- [`panic!`](https://doc.rust-lang.org/std/macro.panic.html) and
  [`catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)
  in the standard library documentation.
- [Panic](https://doc.rust-lang.org/reference/panic.html) in the Rust
  Reference, including [unwinding](https://doc.rust-lang.org/reference/panic.html#unwinding).
- [The `panic` profile setting](https://doc.rust-lang.org/cargo/reference/profiles.html#panic)
  in the Cargo book.

## Lesson 02 · Catching Result errors and printing context

**Assumed:** enums and `match` from chapter 6, and lesson 01.

`Result<T, E>` is an ordinary enum with two variants: `Ok(T)` holds the
value, `Err(E)` holds a description of the failure. A function that can fail
returns one. Nothing is thrown and nothing unwinds. The caller receives a
value and must look inside it before it can use the `T`.

The basic tool is `match`. Each arm states a decision:

```rust
use std::net::Ipv4Addr;

/// The address to listen on. A bad setting is reported and replaced.
fn bind_address(text: &str) -> Ipv4Addr {
    match text.parse::<Ipv4Addr>() {
        Ok(address) => address,
        Err(error) => {
            eprintln!("warning: {text:?} is not usable ({error}); using 127.0.0.1");
            Ipv4Addr::LOCALHOST
        }
    }
}

assert_eq!(bind_address("10.0.0.7"), Ipv4Addr::new(10, 0, 0, 7));
assert_eq!(bind_address("10.0.0.300"), Ipv4Addr::LOCALHOST);
```

The second call prints this on standard error:

```text
warning: "10.0.0.300" is not usable (invalid IPv4 address syntax); using 127.0.0.1
```

Three things in this example are worth naming.

**The recovery policy is a decision.** Here the policy is "warn and use a
default". Other policies are: stop and return the error to the caller, ask
again, or skip this item and count it. Replacing an error with a default is
correct only if the caller really does not need to know. A default that is
also a valid value (zero is the usual one) hides the failure completely:
afterwards nobody can tell "the user typed 0" from "the user typed
nonsense".

**An error value has two text forms.** `{error}` uses the `Display` trait
and is meant for people who use the program. `{error:?}` uses `Debug` and is
meant for programmers:

```rust
let error = "12x".parse::<u8>().unwrap_err();
assert_eq!(format!("{error}"), "invalid digit found in string");
assert_eq!(format!("{error:?}"), "ParseIntError { kind: InvalidDigit }");
```

**The message adds context.** The parse error knows what was wrong with the
text. It does not know which setting the text came from. The caller knows
that, so the caller adds it. The standard library does not promise to keep
the exact wording of its `Display` texts between versions. If other programs
or tests read your message, write the whole message yourself and look at the
error's *kind* instead of its text:

```rust
use std::num::IntErrorKind;

let error = "".parse::<u8>().unwrap_err();
assert_eq!(error.kind(), &IntErrorKind::Empty);
```

Diagnostics go to standard error (`eprintln!`), not standard output
(`println!`). Then a user can send the program's real output to a file or to
another program, and the warnings still appear on the screen.

**Mistakes people make**

*Using the `Result` as if it were the value.*

```rust,compile_fail,E0369
let next = "41".parse::<u32>() + 1;
```

```text
error[E0369]: cannot add `{integer}` to `Result<u32, ParseIntError>`
 --> b1.rs:2:36
  |
2 |     let next = "41".parse::<u32>() + 1;
  |                ------------------- ^ - {integer}
  |                |
  |                Result<u32, ParseIntError>
```

The number is inside the `Ok` variant. Take it out with `match`, with `?`
(lesson 03) or with a method (lesson 04).

*Handling only the success case.* A `match` must cover every variant:

```rust,compile_fail,E0004
let parsed: Result<u8, std::num::ParseIntError> = "12".parse();
let value = match parsed {
    Ok(n) => n,
};
```

```text
error[E0004]: non-exhaustive patterns: `Err(_)` not covered
 --> b2.rs:3:23
  |
3 |     let value = match parsed {
  |                       ^^^^^^ pattern `Err(_)` not covered
```

The compiler suggests `Err(_) => todo!()`. Treat that as a reminder to
decide the policy, not as the fix.

*Printing the error and continuing as if nothing happened.* Printing is not
handling. After the `eprintln!`, the code still needs a value to continue
with, or it must return.

**Read more**

- [The `std::result` module](https://doc.rust-lang.org/std/result/index.html):
  an overview of `Result` and of the ways to handle one.
- [`IntErrorKind`](https://doc.rust-lang.org/std/num/enum.IntErrorKind.html):
  the reasons an integer parse can fail.

## Lesson 03 · Propagating failures with question mark

**Assumed:** lesson 02, and `Option` from chapter 6.

Most functions cannot do anything useful with an error. The right action is
to stop and give the error to the caller. Written with `match`, that is:

```rust
use std::num::ParseIntError;

fn doubled(text: &str) -> Result<i64, ParseIntError> {
    let n = match text.parse::<i64>() {
        Ok(value) => value,
        Err(error) => return Err(error),
    };
    Ok(n * 2)
}

assert_eq!(doubled("21"), Ok(42));
assert!(doubled("x").is_err());
```

The `?` operator is a short form of that `match`. `expression?` means:

- if the result is `Ok(value)`, the whole expression has the value `value`,
  and the function continues;
- if it is `Err(error)`, the function returns `Err(From::from(error))`
  immediately.

So the body above can be `let n: i64 = text.parse()?;`. `From::from` is a
conversion. When the error already has the type that the function returns,
the conversion changes nothing. Lessons 05 and 06 use it to change the error
type.

`?` also works on `Option`: `None` makes the function return `None`. A
function may use `?` on `Result` values or on `Option` values, but not on
both, because it has only one return type. `.ok()` turns a `Result` into an
`Option`, and `.ok_or(error)` turns an `Option` into a `Result`:

```rust
/// "07:45" -> 465. `None` for anything that is not a time of day.
fn minutes_since_midnight(text: &str) -> Option<u32> {
    let (hours, minutes) = text.split_once(':')?; // None if there is no ':'
    let hours: u32 = hours.parse().ok()?; // Result -> Option, then ?
    let minutes: u32 = minutes.parse().ok()?;
    if hours < 24 && minutes < 60 {
        Some(hours * 60 + minutes)
    } else {
        None
    }
}

assert_eq!(minutes_since_midnight("07:45"), Some(465));
assert_eq!(minutes_since_midnight("0745"), None);
assert_eq!(minutes_since_midnight("07:xx"), None);
assert_eq!(minutes_since_midnight("24:00"), None);
```

Because `?` returns at the first failure, nothing after it runs. In a loop,
the items after the failing one are never looked at. The same is true when
you `collect()` an iterator of `Result` values into a
`Result<Vec<T>, E>`: collecting stops at the first `Err`.

```rust
let good: Result<Vec<u8>, _> = ["1", "2"].iter().map(|s| s.parse::<u8>()).collect();
assert_eq!(good, Ok(vec![1, 2]));

let bad: Result<Vec<u8>, _> = ["1", "x", "3"].iter().map(|s| s.parse::<u8>()).collect();
assert!(bad.is_err());
```

**Mistakes people make**

*Using `?` in a function that does not return a `Result`.*

```rust,compile_fail,E0277
fn half(text: &str) -> i32 {
    let n: i32 = text.parse()?;
    n / 2
}
```

```text
error[E0277]: the `?` operator can only be used in a function that returns `Result` or `Option` (or another type that implements `FromResidual`)
 --> c1.rs:2:30
  |
1 | fn half(text: &str) -> i32 {
  | -------------------------- this function should return `Result` or `Option` to accept `?`
2 |     let n: i32 = text.parse()?;
  |                              ^ cannot use the `?` operator in a function that returns `i32`
```

`?` needs somewhere to send the error. Change the return type to
`Result<i32, ParseIntError>`, or handle the error here with `match`.

*Forgetting to wrap the final value.* After you change the return type, the
last line must build a `Result` too:

```rust,compile_fail,E0308
use std::num::ParseIntError;

fn doubled(text: &str) -> Result<i64, ParseIntError> {
    let n: i64 = text.parse()?;
    n * 2
}
```

```text
error[E0308]: mismatched types
 --> c3.rs:4:5
  |
4 |     n * 2
  |     ^^^^^ expected `Result<i64, ParseIntError>`, found `i64`
help: try wrapping the expression in `Ok`
  |
4 |     Ok(n * 2)
  |     +++     +
```

*Mixing `Option` and `Result`.*

```rust,compile_fail,E0277
use std::num::ParseIntError;

fn first_number(text: &str) -> Result<i32, ParseIntError> {
    let word = text.split_whitespace().next()?;
    word.parse()
}
```

```text
error[E0277]: the `?` operator can only be used on `Result`s, not `Option`s, in a function that returns `Result`
 --> c2.rs:3:46
  |
3 |     let word = text.split_whitespace().next()?;
  |                                              ^ use `.ok_or(...)?` to provide an error compatible with `Result<i32, ParseIntError>`
```

`None` carries no error value, so you must supply one with `ok_or`. Here
that does not work well with `ParseIntError`: the type has no public
constructor, and "there was no word" is not a parse failure anyway. That is
a sign that this function needs its own error type (lesson 06).

**Read more**

- [The try propagation expression](https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-try-propagation-expression)
  in the Rust Reference. That is the formal name of `?`.
- [The question mark operator, `?`](https://doc.rust-lang.org/std/result/index.html#the-question-mark-operator-)
  in the `std::result` documentation.

## Lesson 04 · Result methods and aliases

**Assumed:** lessons 02 and 03, and closures used in the simple form
`|x| expression`.

Many `match` expressions on a `Result` have the same shape: do something
with one variant and pass the other one through unchanged. `Result` has a
method for each common shape. These methods are often called *combinators*.

| Method | On `Ok(v)` | On `Err(e)` |
| --- | --- | --- |
| `map(f)` | `Ok(f(v))` | unchanged |
| `map_err(f)` | unchanged | `Err(f(e))` |
| `and_then(f)` | `f(v)`, which is itself a `Result` | unchanged |
| `or_else(f)` | unchanged | `f(e)`, which is itself a `Result` |
| `unwrap_or(d)` | `v` | `d` |
| `unwrap_or_else(f)` | `v` | `f(e)` |
| `ok()` | `Some(v)` | `None` |

The important pair is `map` and `and_then`. Use `map` when the next step
cannot fail. Use `and_then` when the next step can fail too, so that both
failures end up in one flat `Result`.

```rust
type Lookup<T> = Result<T, String>;

fn user(id: u32) -> Lookup<&'static str> {
    match id {
        1 => Ok("ada"),
        2 => Ok(""),
        _ => Err(format!("no user {id}")),
    }
}

/// map: the next step cannot fail.
fn shout(id: u32) -> Lookup<String> {
    user(id).map(str::to_uppercase)
}

/// and_then: the next step can fail too.
fn initial(id: u32) -> Lookup<char> {
    user(id).and_then(|name| name.chars().next().ok_or(format!("user {id} has no name")))
}

/// or_else: try something else after a failure.
fn user_or_admin(id: u32) -> Lookup<&'static str> {
    user(id).or_else(|_| user(1))
}

assert_eq!(shout(1), Ok(String::from("ADA")));
assert_eq!(initial(1), Ok('a'));
assert_eq!(initial(2), Err(String::from("user 2 has no name")));
assert_eq!(initial(9), Err(String::from("no user 9")));
assert_eq!(user_or_admin(9), Ok("ada"));
assert_eq!(user(9).unwrap_or("guest"), "guest");
```

**Eager and lazy fallbacks.** The argument of `unwrap_or(d)` is an ordinary
function argument, so `d` is computed before the call, also when the result
is `Ok`. `unwrap_or_else(f)` receives a closure and calls it only for an
`Err`. Use the `_else` form when the fallback is costly or has an effect,
such as writing to a log.

**`transpose`.** A value that is optional *and* can be invalid has two
layers, and there are two ways to nest them. `Option<Result<T, E>>` means
"maybe there is a result". `Result<Option<T>, E>` means "it worked, and
maybe there is a value". `transpose` converts between the two. The second
form is the one that `?` can use:

```rust
let absent: Option<Result<u8, String>> = None;
assert_eq!(absent.transpose(), Ok(None));

let present: Option<Result<u8, String>> = Some(Ok(5));
assert_eq!(present.transpose(), Ok(Some(5)));
```

**Type aliases.** `type Lookup<T> = Result<T, String>;` gives a second name
to an existing type. It does not create a new type. A value of type
`Lookup<u8>` *is* a `Result<u8, String>`, with the same methods, and the two
names can be mixed freely. The standard library does this itself:
`std::io::Result<T>` is `Result<T, std::io::Error>`.

**Mistakes people make**

*`map` with a step that can fail.* The results nest:

```rust,compile_fail,E0308
fn checked_half(n: u32) -> Result<u32, String> {
    if n % 2 == 0 { Ok(n / 2) } else { Err(format!("{n} is odd")) }
}

let parsed: Result<u32, String> = Ok(10);
let halved: Result<u32, String> = parsed.map(checked_half);
```

```text
error[E0308]: mismatched types
 --> d1.rs:6:39
  |
6 |     let halved: Result<u32, String> = parsed.map(checked_half);
  |                 -------------------   ^^^^^^^^^^^^^^^^^^^^^^^^ expected `Result<u32, String>`, found `Result<Result<u32, String>, String>`
```

Do not follow the compiler's suggestion to add `expect` here. Replace `map`
with `and_then`.

*An alias named `Result` that hides the standard one.* This is a common and
useful pattern, but inside that module `Result` now takes one argument:

```rust,compile_fail,E0107
type Result<T> = std::result::Result<T, String>;

fn parse(text: &str) -> Result<u8, std::num::ParseIntError> {
    todo!()
}
```

```text
error[E0107]: type alias takes 1 generic argument but 2 generic arguments were supplied
 --> d2.rs:3:25
  |
3 | fn parse(text: &str) -> Result<u8, std::num::ParseIntError> {
  |                         ^^^^^^   ------------------------- help: remove the unnecessary generic argument
  |                         |
  |                         expected 1 generic argument
```

Write `std::result::Result<u8, ParseIntError>` for the exception, or give
the alias a different name, as this chapter does.

*Long chains.* A chain of five combinators with closures is harder to read
than a few lines with `?`. Use `?` for "stop on failure" and a combinator
for one small change to one side.

**Read more**

- [`Result`](https://doc.rust-lang.org/std/result/enum.Result.html): all
  methods, each with an example.
- [`Option::transpose`](https://doc.rust-lang.org/std/option/enum.Option.html#method.transpose).
- [Type aliases](https://doc.rust-lang.org/reference/items/type-aliases.html)
  in the Rust Reference.

## Lesson 05 · Multiple error types at a boundary

**Assumed:** lesson 03 (`?` calls `From::from` on the error), and `Box<T>`
from chapter 4 as "an owning pointer to a value on the heap".

A function that reads text and then parses it can fail in two ways, and the
two failures have different types. A `Result<T, E>` has room for one `E`. So
what is `E`?

One answer is a **trait object**. The standard library has a trait named
`Error` that error types implement. `dyn Error` means "some value of some
type that implements `Error`; which type is decided at run time". Such a
value has no fixed size, so it lives behind a pointer: `Box<dyn Error>`.

```text
 Box<dyn Error> (two machine words)       heap
+------------------+                    +--------------------------------+
| data pointer     | -----------------> | the error value, for example   |
| vtable pointer   | ---.               | a ParseFloatError              |
+------------------+    |               +--------------------------------+
                        |               read-only data in the program
                        '-------------> | drop, size, alignment,         |
                                        | Display::fmt, Debug::fmt,      |
                                        | source, type id                |
```

The *vtable* is a table of function pointers for the concrete type. Through
it the program can print the error, drop it, and ask which type it really
is.

The standard library contains conversions that make `?` do the boxing: one
`From` impl that covers every type that implements `Error`, and separate
ones for text (`&str` and `String`).

```rust
use std::error::Error;
use std::str::Utf8Error;

/// Mean of comma-separated numbers in raw bytes.
fn mean(bytes: &[u8]) -> Result<f64, Box<dyn Error>> {
    let text = std::str::from_utf8(bytes)?; // Utf8Error -> Box<dyn Error>
    if text.trim().is_empty() {
        return Err("no numbers".into()); // &str -> Box<dyn Error>
    }
    let mut sum = 0.0;
    let mut count = 0;
    for field in text.split(',') {
        sum += field.trim().parse::<f64>()?; // ParseFloatError -> Box<dyn Error>
        count += 1;
    }
    Ok(sum / f64::from(count))
}

assert_eq!(mean(b"1, 2, 6").unwrap(), 3.0);

// The caller can still ask what really went wrong.
let error = mean(&[b'1', 0xff]).unwrap_err();
match error.downcast_ref::<Utf8Error>() {
    Some(cause) => assert_eq!(cause.valid_up_to(), 1),
    None => panic!("expected a UTF-8 error"),
}

let error = mean(b"1, two").unwrap_err();
assert!(error.is::<std::num::ParseFloatError>());
assert_eq!(mean(b"").unwrap_err().to_string(), "no numbers");
```

**Downcasting.** `error.is::<T>()` asks "is the boxed value a `T`?".
`error.downcast_ref::<T>()` returns `Some(&T)` if it is. This is how a
caller gets the concrete error back. Notice the cost for the reader: nothing
in the signature of `mean` says which types to try. You have to read the
body or the documentation.

**Cost.** A `Box<dyn Error>` is created only when an error happens.
Creating it allocates memory: one allocation holds the error value, and an
error made from a `&str` needs one more for its text. The success path pays
nothing extra.

**Reading text from a reader.** The lesson gives you its input as
`impl Read`. `std::io::Read` is the trait for anything that delivers bytes:
a file, a network connection, or a plain `&[u8]` in a test. Its method
`read_to_string` appends everything the reader delivers to a `String` that
you supply. It returns an `io::Result` (the `Ok` value is the number of
bytes, which you rarely need). The reader must be `mut`, because reading
changes its position.

```rust
use std::io::Read;

/// Counts the words in everything that a reader delivers.
fn word_count(mut reader: impl Read) -> std::io::Result<usize> {
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    Ok(text.split_whitespace().count())
}

assert_eq!(word_count("to be or not".as_bytes()).unwrap(), 4);

// A String must be valid UTF-8, so other bytes are an error.
let not_text: &[u8] = &[0xff];
assert!(word_count(not_text).is_err());
```

**Mistakes people make**

*Returning a bare string where a boxed error is expected.* `?` converts
automatically; a plain `Err(...)` does not:

```rust,compile_fail,E0308
use std::error::Error;

fn first_word(text: &str) -> Result<&str, Box<dyn Error>> {
    match text.split_whitespace().next() {
        Some(word) => Ok(word),
        None => Err("empty input"),
    }
}
```

```text
error[E0308]: mismatched types
 --> e1.rs:5:21
  |
5 |         None => Err("empty input"),
  |                 --- ^^^^^^^^^^^^^ expected `Box<dyn Error>`, found `&str`
help: call `Into::into` on this expression to convert `&'static str` into `Box<dyn std::error::Error>`
  |
5 |         None => Err("empty input".into()),
  |                                  +++++++
```

*Turning every error into a `String`.* `map_err(|e| e.to_string())` also
gives one error type, but the value is now only text. The caller cannot
downcast it, cannot read its kind, and cannot follow its cause. Box the
error itself.

*Downcasting to the wrong type.* `downcast_ref` returns `None` for a wrong
guess; nothing warns you. An integer parse error and a float parse error
are different types. Always keep a final "other" case.

**When to use which**

| You are writing | Good choice |
| --- | --- |
| A library, or code whose callers react differently to different failures | your own error enum (lesson 06) |
| The top of a program, which mostly prints the error and stops | `Box<dyn Error>` |
| Code whose errors may cross threads | the thread-safe box of lesson 09 |

Many programs use the `anyhow` crate at the top level instead of a boxed
error, and the `thiserror` crate to generate the impls of lesson 06. Neither
is part of the standard library, so this chapter leaves them to an
[optional lab](LABS.md) and to lesson E01 of the course's
[`ecosystem` package](../../ecosystem/README.md), which you can do after
this chapter.

**Read more**

- [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html),
  including `is` and `downcast_ref`.
- [Trait objects](https://doc.rust-lang.org/reference/types/trait-object.html)
  in the Rust Reference.
- [`Read::read_to_string`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_string)
  and [`io::ErrorKind`](https://doc.rust-lang.org/std/io/enum.ErrorKind.html),
  the list of reasons an I/O operation can fail.
- The mastery track ([`mastery/m1_types`](../../mastery/m1_types/README.md)) looks inside trait objects in
  detail.

## Lesson 06 · Custom errors with a source chain

**Assumed:** lesson 05, and enums with data from chapter 6.

A boxed error hides the list of possible failures. When callers need that
list, write your own error type. Any type can be an error type if it
implements three traits:

- `Debug`, the programmer's view. You normally derive it.
- `Display`, the message for people. You write it.
- `Error`, which marks the type as an error and has one optional method,
  `source`. It returns the lower-level error that caused this one, if there
  is one.

An enum is the usual choice: one variant per failure that a caller might
treat differently.

```rust
use std::error::Error;
use std::fmt;
use std::num::ParseFloatError;

#[derive(Debug)]
enum ReadingError {
    /// The field is not a number. Keeps the cause and names the field.
    BadNumber { field: &'static str, source: ParseFloatError },
    /// A number, but not one that the sensor can report.
    OutOfRange(f64),
}

impl fmt::Display for ReadingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadingError::BadNumber { field, .. } => {
                write!(f, "field `{field}` is not a number")
            }
            ReadingError::OutOfRange(value) => write!(f, "{value} °C is outside -90..=60"),
        }
    }
}

impl Error for ReadingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ReadingError::BadNumber { source, .. } => Some(source),
            ReadingError::OutOfRange(_) => None,
        }
    }
}

fn celsius(text: &str) -> Result<f64, ReadingError> {
    let value: f64 = text
        .trim()
        .parse()
        .map_err(|source| ReadingError::BadNumber { field: "celsius", source })?;
    if (-90.0..=60.0).contains(&value) {
        Ok(value)
    } else {
        Err(ReadingError::OutOfRange(value))
    }
}

/// Joins an error and all of its causes into one line.
fn chain(error: &dyn Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(inner) = cause {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        cause = inner.source();
    }
    text
}

assert_eq!(celsius(" 21.5 ").unwrap(), 21.5);
assert_eq!(chain(&celsius("75").unwrap_err()), "75 °C is outside -90..=60");
assert_eq!(
    chain(&celsius("warm").unwrap_err()),
    "field `celsius` is not a number: invalid float literal"
);

// A caller can react to one variant and pass the others on.
let clamped = match celsius("75") {
    Err(ReadingError::OutOfRange(value)) => value.clamp(-90.0, 60.0),
    other => other.unwrap_or(0.0),
};
assert_eq!(clamped, 60.0);
```

**The source chain.** `BadNumber` stores the `ParseFloatError` as a value
and returns it from `source()`. The `Display` text of `BadNumber` does not
repeat the cause's text. A reporter such as `chain` walks from error to
cause to cause and prints each message once. Keep to this rule: a cause is
either part of your `Display` text or returned from `source()`, not both.
Otherwise every report prints it twice.

Not every variant has a cause. `OutOfRange` is a rule of your own domain,
broken by a perfectly valid number, so its `source()` is `None`.

**`From` or `map_err`?** `?` converts the error with `From::from`. If you
write `impl From<ParseFloatError> for ReadingError`, then a bare `?`
converts a parse error into your type. That is convenient when the lower
error alone is enough to build your variant:

```rust
use std::num::ParseIntError;

#[derive(Debug)]
enum ConfigError {
    BadNumber(ParseIntError),
}

impl From<ParseIntError> for ConfigError {
    fn from(error: ParseIntError) -> Self {
        ConfigError::BadNumber(error)
    }
}

fn retries(text: &str) -> Result<u8, ConfigError> {
    let n = text.trim().parse::<u8>()?; // From does the wrapping
    Ok(n)
}

assert_eq!(retries(" 3 ").ok(), Some(3));
assert!(matches!(retries("many"), Err(ConfigError::BadNumber(_))));
```

`ReadingError::BadNumber` needs more than the lower error: it needs the
field name, which only the call site knows. A `From` impl has no way to
receive it. That is when you use `map_err`, as `celsius` does.

**Mistakes people make**

*Implementing `Error` without `Display`.*

```rust,compile_fail,E0277
use std::error::Error;

#[derive(Debug)]
struct Timeout {
    seconds: u32,
}

impl Error for Timeout {}
```

```text
error[E0277]: `Timeout` doesn't implement `std::fmt::Display`
 --> f1.rs:8:16
  |
8 | impl Error for Timeout {}
  |                ^^^^^^^ unsatisfied trait bound
note: required by a bound in `std::error::Error`
  |
  | pub trait Error: Debug + Display {
  |                          ^^^^^^^ required by this bound in `Error`
```

`Display` cannot be derived. Write the message by hand.

*Using `?` when no conversion exists.* Remove the `From` impl from the
`ConfigError` example and the compiler says:

```text
error[E0277]: `?` couldn't convert the error to `ConfigError`
 --> f2.rs:9:38
  |
9 |     let n = text.trim().parse::<u8>()?;
  |                         -------------^ the trait `From<ParseIntError>` is not implemented for `ConfigError`
note: `ConfigError` needs to implement `From<ParseIntError>`
```

Add the `From` impl, or convert at the call site with `map_err`.

*Storing the cause as a `String`.* A variant such as `BadNumber(String)`
keeps the text and throws away the kind and the rest of the chain. Store
the error value.

**Read more**

- [`Error::source`](https://doc.rust-lang.org/std/error/trait.Error.html#method.source).
- [`From`](https://doc.rust-lang.org/std/convert/trait.From.html), which
  has a section on error handling.
- [`Display`](https://doc.rust-lang.org/std/fmt/trait.Display.html).
- The mastery track ([`mastery/m8_api`](../../mastery/m8_api/README.md)) covers the design of error types for
  published libraries.

## Lesson 07 · Impossible errors and deliberate recovery

**Assumed:** lessons 01 to 03.

Sometimes you hold a `Result` and you will not pass the error on. There are
three honest ways to do that. Each one leaves a decision that a reader can
see.

**1. "This cannot fail": `expect` with the reason.** Some operations return
a `Result` because they can fail in general, while your code guarantees
that this call cannot. `expect("...")` returns the `Ok` value and panics on
`Err`. This follows lesson 01: if the guarantee is ever broken, that is a
bug. The message should state the guarantee, not the symptom:

```rust
/// The last decimal digit of `n`, as a character.
fn last_digit(n: u32) -> char {
    char::from_digit(n % 10, 10).expect("n % 10 is always below 10")
}

assert_eq!(last_digit(2026), '6');
```

`from_digit` returns an `Option` here; `expect` works the same way on
`Option` and on `Result`. On a `Result`, `expect` adds the `Debug` text of
the error after your message. This is the real output of a program in which
the "guarantee" was wrong (the literal contains the letter O, not a zero):

```text
thread 'main' (122021572) panicked at g3.rs:2:34:
the grid size is a literal in this file: ParseIntError { kind: InvalidDigit }
```

Whoever reads that panic learns which promise was broken and where to look.

**2. "I do not care whether this worked": `let _ =`.** `Result` is marked
`#[must_use]`, so the compiler warns when you drop one without looking.
Best-effort work, such as removing a temporary file, is a fair reason to
ignore a failure. Say so in code:

```rust,no_run
use std::fs;

// Best effort: if the file is already gone, that is fine.
let _ = fs::remove_file("session.tmp");
```

`let _ =` silences the warning. It does not make the operation succeed. It
records that somebody decided the failure does not matter.

**3. "Carry on, but keep a record".** When one bad item should not stop a
batch, recover, and report what you discarded. The result of the batch is
then "what worked, and what did not":

```rust
let fields = ["8", "x", "13", ""];
let (good, bad): (Vec<_>, Vec<_>) = fields
    .iter()
    .map(|field| field.parse::<u8>())
    .partition(|parsed| parsed.is_ok());

assert_eq!(good.len(), 2);
assert_eq!(bad.len(), 2); // the caller can still see that two were rejected
```

Compare that with `.filter_map(|f| f.parse::<u8>().ok())`, which keeps the
good values and makes the bad ones disappear without a trace. That short
form is correct only when the missing record does not matter.

**`_` is not a variable.** `let _ = make_value();` binds nothing, so the
value that the call produced is dropped at the end of that statement.
`let _name = make_value();` is a real variable with an unusual name; its
value lives to the end of the scope. The
difference matters when the value is a guard, which does its work by staying
alive. For one kind of guard the mistake is so common that the compiler
rejects it. A `Mutex` is a lock (chapter 19); `lock()` returns a guard, and
the lock is held for as long as the guard lives:

```rust,compile_fail
use std::sync::Mutex;

let counter = Mutex::new(0);
let _ = counter.lock().unwrap();
```

```text
error: non-binding let on a synchronization lock
 --> g4.rs:4:9
  |
4 |     let _ = counter.lock().unwrap();
  |         ^ this lock is not assigned to a binding and is immediately dropped
  |
  = note: `#[deny(let_underscore_lock)]` (part of `#[deny(let_underscore)]`) on by default
help: consider binding to an unused variable to avoid immediately dropping the value
  |
4 |     let _unused = counter.lock().unwrap();
  |          ++++++
```

For a `Result` the immediate drop is exactly what you want: you are throwing
the value away. For a guard, write `let _guard = ...`.

**Mistakes people make**

*Calling a fallible function as a statement.*

```rust,no_run
use std::fs;

fs::remove_file("no-such-file.tmp");
```

This compiles, with a warning:

```text
warning: unused `Result` that must be used
 --> g1.rs:3:5
  |
3 |     fs::remove_file("no-such-file.tmp");
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: this `Result` may be an `Err` variant, which should be handled
help: use `let _ = ...` to ignore the resulting value
```

The program runs and the failure is lost. Do not add `let _ =` only to make
the warning go away. First decide which of the three cases above you are in.

*`expect("failed to parse")`.* A message that repeats the symptom tells the
reader nothing new. Write why the failure was thought to be impossible.

*`expect` on input.* If the text comes from a user, a file or the network,
no guarantee exists, and case 1 does not apply. Return the error.

**Read more**

- [`Result::expect`](https://doc.rust-lang.org/std/result/enum.Result.html#method.expect),
  with advice on how to word the message.
- [The `must_use` attribute](https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-must_use-attribute)
  in the Rust Reference.

## Lesson 09 · Thread-safe boxed errors and what main returns

**Assumed:** lessons 05 and 07. No knowledge of threads is needed beyond
this: `std::thread::spawn(closure)` runs the closure on a new thread, and
`join()` on the returned handle waits for it and gives back what the closure
returned. Chapter 19 teaches threads.

### Part 1: `Box<dyn Error + Send + Sync + 'static>`

Rust checks at compile time which values may cross from one thread to
another. It uses two marker traits. A *marker trait* has no methods; it only
states a fact about a type.

- `Send`: a value of this type may be **moved** to another thread.
- `Sync`: a value of this type may be **shared** by reference between
  threads.

The compiler works these out for each concrete type. `ParseIntError` and
`std::io::Error` are both. A trait object is different. `dyn Error` means
"some error type, unknown until run time", so the compiler knows only what
the type's name promises, and the name `dyn Error` promises nothing about
threads. `Box<dyn Error>` is therefore neither `Send` nor `Sync`, even when
the value inside happens to be.

You add the promises to the name with `+`:

```rust
use std::error::Error;
use std::thread;

type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// Adds the numbers in a line, wrapping around at 256.
fn checksum(line: &str) -> Result<u8, BoxError> {
    if line.trim().is_empty() {
        return Err("empty line".into()); // &str -> BoxError
    }
    let mut sum = 0u8;
    for word in line.split_whitespace() {
        sum = sum.wrapping_add(word.parse::<u8>()?); // ParseIntError -> BoxError
    }
    Ok(sum)
}

let worker = thread::spawn(|| checksum("250 10"));
let result = worker.join().expect("the worker does not panic");
assert_eq!(result.ok(), Some(4));

let worker = thread::spawn(|| checksum("4 x"));
let error = worker.join().expect("the worker does not panic").unwrap_err();
assert!(error.is::<std::num::ParseIntError>()); // downcasting still works
```

`'static` means that the error borrows nothing that could go away. For a
`Box<dyn Trait>` in a signature it is already the default, so
`Box<dyn Error + Send + Sync>` is the same type. The two extra traits cost
nothing at run time. Only the set of types that you may put in the box is
smaller, and almost every error type qualifies.

The same `From` conversions exist for this type as for the plain one, so
`?` and `.into()` work as in lesson 05. What does not exist is a `From`
conversion between the two boxed types:

| From | To | How |
| --- | --- | --- |
| a concrete error, `&str`, `String` | either box | `?` or `.into()` |
| `Box<dyn Error + Send + Sync>` | `Box<dyn Error>` | a coercion: `let plain: Box<dyn Error> = error;` or `error as Box<dyn Error>` |
| `Box<dyn Error>` | `Box<dyn Error + Send + Sync>` | not possible |

Forgetting a promise is always safe, so that direction is a free
*coercion*: the same pointer under a less specific type. The other direction
would invent a promise, so the compiler refuses it.

```rust
use std::error::Error;

let strict: Box<dyn Error + Send + Sync> = "disk full".into();
let plain: Box<dyn Error> = strict; // same allocation, less specific type
assert_eq!(plain.to_string(), "disk full");
```

**Mistakes people make (part 1)**

*Returning a plain boxed error from a thread.*

```rust,compile_fail,E0277
use std::error::Error;
use std::thread;

fn checksum(text: &str) -> Result<u8, Box<dyn Error>> {
    Ok(text.trim().parse()?)
}

let handle = thread::spawn(|| checksum("7"));
```

```text
error[E0277]: `dyn std::error::Error` cannot be sent between threads safely
 --> h1.rs:9:18
  |
9 |     let handle = thread::spawn(|| checksum("7"));
  |                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `dyn std::error::Error` cannot be sent between threads safely
  |
  = help: the trait `Send` is not implemented for `dyn std::error::Error`
note: required because it appears within the type `Box<dyn std::error::Error>`
note: required because it appears within the type `Result<u8, Box<dyn std::error::Error>>`
note: required by a bound in `spawn`
```

Read the notes from the bottom up: `spawn` requires a `Send` result; the
result contains a `Box<dyn Error>`; that is not `Send`. Change the return
type of `checksum`.

*Using `?` between the two boxed types.* In the impossible direction the
compiler reports three errors for one `?`. The first two are the real
reason; the third, about `Sized`, is noise:

```text
error[E0277]: `?` couldn't convert the error: `dyn std::error::Error: Send` is not satisfied
error[E0277]: `?` couldn't convert the error: `dyn std::error::Error: Sync` is not satisfied
error[E0277]: `?` couldn't convert the error: `dyn std::error::Error: Sized` is not satisfied
```

In the allowed direction `?` fails as well, because `?` only uses `From`
and never a coercion:

```rust,compile_fail,E0277
use std::error::Error;

fn load() -> Result<u32, Box<dyn Error + Send + Sync>> {
    Ok("7".parse::<u32>()?)
}

fn report() -> Result<u32, Box<dyn Error>> {
    let n = load()?;
    Ok(n)
}
```

```text
error[E0277]: `?` couldn't convert the error: `dyn std::error::Error + Send + Sync: Sized` is not satisfied
   = note: required for `Box<dyn std::error::Error>` to implement `From<Box<dyn std::error::Error + Send + Sync>>`
```

Convert the error yourself, with the coercion from the table, before `?`
sees it. Task C of the lesson asks you to work out where the coercion goes.

**When to use which.** If you choose a boxed error for an application, use
the `Send + Sync + 'static` form everywhere. It costs nothing, and it is
what threads, async tasks (chapter 20) and crates such as `anyhow` require.
Plain `Box<dyn Error>` is fine for short programs and examples that stay on
one thread.

### Part 2: what `main` returns

`main` may return any type that implements the trait
`std::process::Termination`. The runtime calls `report()` on the returned
value to get the **exit status**, a small number that the process hands to
whoever started it. Zero means success; anything else means failure. Scripts
test this number, so keep its meanings few and documented.

| `main` returns | Exit status | Printed to standard error |
| --- | --- | --- |
| `()` | 0 | nothing |
| `Result<(), E>`, value `Ok(())` | 0 | nothing |
| `Result<(), E>`, value `Err(e)` | 1 | `Error: ` and the **Debug** text of `e` |
| `ExitCode` | the code you built | nothing; you print |
| (a panic) | 101 | the panic message |

Returning a `Result` is the quickest way to use `?` in `main`:

```rust,no_run
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let text = std::fs::read_to_string("limit.txt")?;
    let limit: u32 = text.trim().parse()?;
    println!("limit = {limit}");
    Ok(())
}
```

Run without the file, and then with a file that contains `ten`, it printed
the following and exited with status 1 both times:

```text
Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }
Error: ParseIntError { kind: InvalidDigit }
```

That is `Debug` output: acceptable in a tool for yourself, poor in a tool
for other people. For a stable message and a status of your choice, return
`ExitCode` and print the error yourself:

```rust,no_run
use std::error::Error;
use std::process::ExitCode;

fn read_limit() -> Result<u32, Box<dyn Error>> {
    let text = std::fs::read_to_string("limit.txt")?;
    Ok(text.trim().parse()?)
}

fn main() -> ExitCode {
    match read_limit() {
        Ok(limit) => {
            println!("limit = {limit}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("limit.txt: {error}");
            ExitCode::from(2)
        }
    }
}
```

The same two runs now printed this, with status 2:

```text
limit.txt: No such file or directory (os error 2)
limit.txt: invalid digit found in string
```

`ExitCode::SUCCESS` and `ExitCode::FAILURE` are the portable values;
`ExitCode::from(n)` takes a `u8`.

**Writing to a stream you were given.** `eprintln!` always writes to the
real standard error, so a test cannot see what was printed. Code that should
be testable takes the stream as a parameter instead. `std::io::Write` is the
trait for anything that accepts bytes, and `&mut dyn Write` means "a mutable
borrow of some writer". The macro `writeln!(out, "...")` formats like
`println!`, writes to `out`, and returns an `io::Result<()>`, because
writing can fail. A `Vec<u8>` is a writer that keeps the bytes in memory:

```rust
use std::io::Write;

fn greet(out: &mut dyn Write, name: &str) -> std::io::Result<()> {
    writeln!(out, "hello, {name}")
}

let mut buffer = Vec::new();
greet(&mut buffer, "ada").unwrap();
assert_eq!(buffer, b"hello, ada\n");

// In a real program the same function gets the real stream.
greet(&mut std::io::stderr(), "ada").unwrap();
```

**Mistakes people make (part 2)**

*Expecting `main` to print the `Display` text.* It prints `Debug`. An error
made from a string shows this most clearly: a `main` that returns
`Err("disk full".into())` prints the message *with* quotation marks,
`Error: "disk full"`.

*Calling `std::process::exit` deep inside the program.* `exit` ends the
process immediately. Destructors of live values do not run. A file wrapped
in a `BufWriter`, for example, never receives the data that is still in the
buffer. Returning from `main`, with a `Result` or an `ExitCode`, drops
everything normally first. Let errors travel up to `main` and decide the
status there.

**Read more**

- [`Send`](https://doc.rust-lang.org/std/marker/trait.Send.html) and
  [`Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html).
- [`Termination`](https://doc.rust-lang.org/std/process/trait.Termination.html)
  and [`ExitCode`](https://doc.rust-lang.org/std/process/struct.ExitCode.html).
- [`Write`](https://doc.rust-lang.org/std/io/trait.Write.html) and
  [`writeln!`](https://doc.rust-lang.org/std/macro.writeln.html).
- [`std::process::exit`](https://doc.rust-lang.org/std/process/fn.exit.html),
  which explains why destructors are skipped.
- The mastery track ([`mastery/m3_concurrency`](../../mastery/m3_concurrency/README.md)) goes much further with
  threads.

## Lesson 08 · Capstone: fallible batch report

**Assumed:** every lesson above. This section describes the approach. It
does not contain the solution.

The capstone gives you an error enum and asks for two functions: one checks
a single row, the other processes a whole batch and returns either a
complete answer or the first failure. Plan it in this order.

1. **Read the error type first.** Each variant is one thing a caller could
   act on, and each carries the row number. Ask of every variant: where in
   my code is the first moment at which I know this has happened? Create
   the error at that moment. That is the only place where the row number is
   known without extra work.
2. **Write the single-row function as a classifier.** One piece of text
   goes in. One of several outcomes comes out. Decide which outcome each
   kind of text belongs to before you write code: text that is not a number,
   a number that the rules reject, a good number. A `match` on the parse
   result can express all the cases at once.
3. **Write the batch function as one pass.** Go through the rows in order
   with a running total. Each step can fail in more than one way: the row
   itself can be bad, or adding it can overflow. Chapter 3 introduced the
   `checked_` arithmetic methods, which return an `Option`; lesson 03 showed
   how to turn an `Option` into a `Result` so that `?` can propagate it.
4. **Never return a partial answer as success.** If row 3 fails, the caller
   must receive the failure for row 3, not the sum of rows 1 and 2.
5. **Check the technique test.** One test counts heap allocations and
   expects zero. Errors that are plain enum values need no `String`, and a
   running total needs no `Vec`.

When the tests pass, answer the last part of the lesson's EXPLAIN prompt
with lesson 09 in mind: how would a `main` turn each variant into a message
and an exit status? [Lab 5](LABS.md) lets you try it in a real program.

**Read more**

- [`u16::checked_add`](https://doc.rust-lang.org/std/primitive.u16.html#method.checked_add).
- [`Option::ok_or`](https://doc.rust-lang.org/std/option/enum.Option.html#method.ok_or).
- [`Iterator::enumerate`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.enumerate).
