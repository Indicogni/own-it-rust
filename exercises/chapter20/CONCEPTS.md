# Chapter 20 concepts — Writing async code

[Chapter guide](README.md) · [Hints](HINTS.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every code block in this file was compiled and
run with the course toolchain (Rust 1.99.0, edition 2024), and every compiler
message was copied from that compiler. Long messages are shortened: a line with
`...` marks a cut in the middle, and the end of a message is often left out.

## What you will be able to do

After this chapter you can:

- say what happens, and what does not happen, when you call an `async fn`;
- decide whether a future should borrow its inputs or own them, and read the
  errors that appear when a future must be `Send` or `'static`;
- put async methods in a trait and call them through a generic bound;
- accept an async closure as a callback with `AsyncFn`, `AsyncFnMut` or
  `AsyncFnOnce`;
- predict what is dropped, and what is left half done, when a future is
  cancelled;
- write a small combinator by hand that polls two futures.

## Before you start

Ownership and borrowing with lifetimes (Chapters 4
and 5), traits and generics (Chapter 11), `Drop` (Chapter 13), closures and the
`Fn` traits (Chapter 14), and threads with `Send` (Chapter 19). You do not need
any async experience.

## Time

These are estimates for a learner who has finished the earlier
chapters. They include reading the section, the predictions, the code, and your
notes.

| Lesson | Estimate |
| --- | --- |
| 01 · Lazy futures and await | 45 minutes |
| 02 · Borrowed and owned futures | 45 minutes |
| 03 · Async behavior in traits | 40 minutes |
| 04 · Lending async callbacks | 50 minutes |
| 05 · Cancellation by dropping | 50 minutes |
| 06 · Polling a race | 60 minutes |
| 07 · Capstone | 45 minutes |
| R01 review, C01 code review, drills | 2 hours |

## What this chapter does not cover

The core lessons use only the standard
library. The standard library defines what a future is, but it does not contain
a *runtime*: the program part that runs many futures, waits for sockets and
timers, and offers `spawn`. So the lessons teach the language rules, and tiny
test helpers drive the futures. Three other parts of the course go further:

- [LABS.md](LABS.md): optional labs with the Tokio runtime. No test checks them.
- The `ecosystem` package, unit `e06_tokio`: tested exercises with Tokio.
- Chapter 21, where you build wakers and an executor yourself, and the mastery
  track [`mastery/m4_async`](../../mastery/m4_async/README.md), which covers pin projection, streams and
  cancellation in a real service.

## Words used in this chapter

- **Future**: a value that describes a computation which may not be finished
  yet. It does nothing until something polls it.
- **Poll**: to ask a future to make progress. The answer is `Ready(output)` or
  `Pending` ("not yet, ask again later").
- **Await**: inside async code, `future.await` means "run this future to the
  end and give me its output; if it is not ready, pause me here".
- **Executor**: the code that polls futures until they are ready. A
  **runtime** is an executor together with timers, sockets and `spawn`.
- **Task**: a future that was handed to a runtime to be run by itself.
- **Waker**: a handle that a pending future uses to say "poll me again now".
- **Cancel**: to drop a future before it is ready.

## The `run` helper

Because the standard library has no executor, most examples in this file start
with the same small function. It polls one future in a loop until it is ready:

```rust
use std::{future::Future, pin::pin, task::{Context, Poll, Waker}};

fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

fn main() {
    assert_eq!(run(async { 6 * 7 }), 42);
}
```

You do not need to understand its body before Lesson 06. Until then, read
`run(f)` as "drive the future `f` to its end and return its output". A real
executor does not spin in a loop like this; it sleeps until a waker tells it
that a future can continue. Some examples also use a helper named `pause`: a
future that is not ready the first time it is polled and ready the second time.

## Lesson 01 · Lazy futures and await

**You need:** functions and closures; nothing about async yet.

### The idea

A normal function runs when you call it. An `async fn` does not. Calling it
only *creates a future*: a value that holds the arguments and remembers that
no work has been done yet. The body runs later, when something polls the
future. People say that futures are **lazy**.

```rust
use std::{future::Future, pin::pin, task::{Context, Poll, Waker}};

fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

async fn load_page(id: u32) -> String {
    println!("  loading page {id}");
    format!("<page {id}>")
}

fn main() {
    println!("calling load_page");
    let future = load_page(7);
    println!("got a future; nothing was loaded yet");
    let page = run(future);
    println!("result: {page}");
}
```

```text
calling load_page
got a future; nothing was loaded yet
  loading page 7
result: <page 7>
```

The type of `load_page(7)` is not `String`. It is an anonymous type that
implements `Future<Output = String>`. An `async { ... }` block is the same
thing without a function around it: an expression whose value is a future.
Every async fn and every async block has a future type of its own, which you
cannot write down by name.

### Await

Inside an `async fn` or an `async` block you can write `other.await`. It means:
run `other` until it is ready, then continue with its output. If `other` is not
ready, the whole surrounding future stops at this point and reports `Pending`
to whoever polled it. On the next poll it continues from the same point.

Two awaits written one after the other run one after the other. The second
future is not polled until the first one is finished.

```rust
use std::{future::{Future, poll_fn}, pin::pin, task::{Context, Poll, Waker}};

fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

/// Not ready on the first poll, ready on the second.
async fn pause() {
    let mut first = true;
    poll_fn(|cx| {
        if first {
            first = false;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await
}

async fn step(name: &str) -> usize {
    println!("{name}: start");
    pause().await;
    println!("{name}: end");
    name.len()
}

async fn install() -> usize {
    let a = step("download").await;
    let b = step("unpack").await;
    a + b
}

fn main() {
    assert_eq!(run(install()), 14);
}
```

```text
download: start
download: end
unpack: start
unpack: end
```

The compiler turns `install` into a **state machine**: a value that records
where the body stopped and keeps the local variables that are still needed.

```text
the future returned by install()

  state      where the body is             what the future stores
  ---------  ----------------------------  ------------------------------
  start      before the first line         nothing
  waiting 1  at step("download").await     the future of step("download")
  waiting 2  at step("unpack").await       a, the future of step("unpack")
  done       after the last line           nothing (the output was returned)

  one poll = run from the stored state to the next await that is not ready
```

Sequential awaits never overlap. To make two futures progress during the same
period you need a combinator that polls both (you write one in Lesson 06) or a
runtime that runs them as separate tasks.

### Mistakes people make

**Forgetting `.await`.** The call gives you a future, not the value.

```rust,compile_fail,E0308
async fn price() -> u32 {
    40
}

async fn total() -> u32 {
    let base: u32 = price();
    base + 2
}

fn main() {
    drop(total());
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:6:21
  |
6 |     let base: u32 = price();
  |               ---   ^^^^^^^ expected `u32`, found future
  |               |
  |               expected due to this
  |
note: calling an async function returns a future
...
help: consider `await`ing on the `Future`
  |
6 |     let base: u32 = price().await;
  |                            ++++++
```

When the value is not used at all, the program compiles and the work is
silently skipped. The only sign is a warning, so read warnings in async code:

```text
warning: unused implementer of `Future` that must be used
 --> src/main.rs:6:5
  |
6 |     save("first");
  |     ^^^^^^^^^^^^^
  |
  = note: futures do nothing unless you `.await` or poll them
```

**Using `.await` in a function that is not async.** Only async code is
compiled into a state machine that can pause.

```rust,compile_fail,E0728
async fn price() -> u32 {
    40
}

fn total() -> u32 {
    price().await + 2
}

fn main() {
    println!("{}", total());
}
```

```text
error[E0728]: `await` is only allowed inside `async` functions and blocks
 --> src/main.rs:6:13
  |
5 | fn total() -> u32 {
  | ----------------- this is not `async`
6 |     price().await + 2
  |             ^^^^^ only allowed inside `async` functions and blocks
```

The fix is to make `total` an `async fn`. Then its caller must await it too.
At the very top, a runtime (or a helper such as `run`) drives the outermost
future from normal code.

**Thinking that two awaits run "at the same time".** They do not. `a.await;
b.await;` finishes `a` before `b` starts.

### Read more

- [`std::future::Future`](https://doc.rust-lang.org/std/future/trait.Future.html)
- [The `async` keyword](https://doc.rust-lang.org/std/keyword.async.html) and
  [the `await` keyword](https://doc.rust-lang.org/std/keyword.await.html)
- The Rust Reference: [async blocks](https://doc.rust-lang.org/reference/expressions/block-expr.html#async-blocks)
  and [await expressions](https://doc.rust-lang.org/reference/expressions/await-expr.html)

## Lesson 02 · Borrowed and owned futures

**You need:** Lesson 01; lifetimes in function signatures (Chapter 5); `Send`
and `'static` as bounds for threads (Chapter 19).

### The idea

A future stores everything its body will need: the arguments of the async fn,
or the variables that an async block captured. If one of these is a reference,
the future *contains a borrow*, and it must not live longer than the value it
borrows from. If the future contains only owned values, it depends on nothing
outside itself.

You can see this in the type. An `async fn` hides the type of its future, but
you can write the same function as a normal `fn` that returns
`impl Future`. Then the lifetime is visible:

```rust
use std::{future::Future, pin::pin, task::{Context, Poll, Waker}};

fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

/// The future borrows `text`. `+ 'a` says so in the return type.
fn count_words<'a>(text: &'a str) -> impl Future<Output = usize> + 'a {
    async move { text.split_whitespace().count() }
}

/// The future owns `bytes`. It borrows nothing, so it can be `'static`.
fn checksum(bytes: Vec<u8>) -> impl Future<Output = u32> + Send + 'static {
    async move { bytes.iter().map(|&b| u32::from(b)).sum() }
}

fn main() {
    let text = String::from("one two three");
    let borrowed = count_words(&text);
    let owned = checksum(vec![1, 2, 3]);
    assert_eq!(run(borrowed), 3); // `text` must still be alive here
    drop(text);
    assert_eq!(run(owned), 6); // needs nothing from its caller
}
```

```text
main's variables                          heap

  text      String header ─────────────►  "one two three"
  borrowed  future { text: &str ───────►  (the same bytes; not owned) }
  owned     future { bytes: Vec header ►  [1, 2, 3] }   (owned by the future)
```

`async move` moves each captured variable into the future. Look at what the
variable *is*. In `count_words` the variable `text` is a `&str`, so the
reference is moved and the text stays where it was. In `checksum` the variable
`bytes` is the `Vec` itself, so the vector's ownership moves into the future. A
move does not copy the heap buffer in either case.

### Why Send and 'static matter

A runtime's `spawn` hands a future to the runtime as a task. The task may run
long after the function that created it has returned, so the future must be
`'static`: it must not borrow from any local variable. On a runtime with
several threads the task may also be moved from one thread to another, so the
future must be `Send`. A future is `Send` only if everything it stores is
`Send`: its arguments or captures, and every local variable that is alive
across an await. The examples below use a function `spawn_like` that has the
same bounds as a real `spawn` and does nothing else.

### Mistakes people make

**Returning a future that borrows a local variable.**

```rust,compile_fail,E0373
use std::future::Future;

fn shout(word: &str) -> impl Future<Output = String> {
    let upper = word.to_uppercase();
    async { format!("{upper}!") }
}

fn main() {
    drop(shout("hi"));
}
```

```text
error[E0373]: async block may outlive the current function, but it borrows `upper`, which is owned by the current function
 --> src/main.rs:5:5
  |
5 |     async { format!("{upper}!") }
  |     ^^^^^             ----- `upper` is borrowed here
  |     |
  |     may outlive borrowed value `upper`
...
help: to force the async block to take ownership of `upper` (and any other referenced variables), use the `move` keyword
  |
5 |     async move { format!("{upper}!") }
  |           ++++
```

A plain `async` block captures a variable by reference when its body only
needs a reference, as `format!` does here. `async move` makes the future the
owner of `upper`, which is what a returned future needs.

**Expecting `async move` to make a borrow `'static`.** Moving a reference
into a future does not move the value behind the reference.

```rust,compile_fail,E0597
use std::future::Future;

fn spawn_like<F: Future + Send + 'static>(future: F) {
    drop(future);
}

fn main() {
    let text = String::from("three short words");
    let view: &str = &text;
    spawn_like(async move { view.len() });
}
```

```text
error[E0597]: `text` does not live long enough
  --> src/main.rs:9:22
   |
 8 |     let text = String::from("three short words");
   |         ---- binding `text` declared here
 9 |     let view: &str = &text;
   |                      ^^^^^ borrowed value does not live long enough
10 |     spawn_like(async move { view.len() });
   |     ------------------------------------- argument requires that `text` is borrowed for `'static`
11 | }
   | - `text` dropped here while still borrowed
```

The fix is to move `text` itself into the block, or to share it through an
`Arc<String>` when several tasks need it.

**Keeping a value that is not `Send` alive across an await.**

```rust,compile_fail
use std::future::Future;
use std::rc::Rc;

fn spawn_like<F: Future + Send + 'static>(future: F) {
    drop(future);
}

async fn pause() {}

fn main() {
    spawn_like(async {
        let label = Rc::new(String::from("job-7"));
        pause().await;
        println!("{label}");
    });
}
```

```text
error: future cannot be sent between threads safely
...
   = help: within `{async block@src/main.rs:11:16: 11:21}`, the trait `Send` is not implemented for `Rc<String>`
note: future is not `Send` as this value is used across an await
  --> src/main.rs:13:17
   |
12 |         let label = Rc::new(String::from("job-7"));
   |             ----- has type `Rc<String>` which is not `Send`
13 |         pause().await;
   |                 ^^^^^ await occurs here, with `label` maybe used later
```

This error has no error code. Use `Arc` instead of `Rc`, or end the life of
the value before the await by putting it in an inner block `{ ... }`. Calling
`drop(label)` before the await is not enough for this check: the compiler
looks at scopes, not at calls to `drop` (drill 11 shows this).

### When to use which

| Situation | Choose |
| --- | --- |
| The caller awaits the future at once, in the same function | Borrow: `async fn f(text: &str)`. No copy, no allocation. |
| The future is stored, returned far away, or spawned as a task | Own: take `String`, `Vec<T>` or another owned value. |
| Several tasks need the same data | Share: give each task a clone of an `Arc<T>`. |

### Read more

- The Rust Reference: [async functions](https://doc.rust-lang.org/reference/items/functions.html#async-functions)
  and [capture modes of async blocks](https://doc.rust-lang.org/reference/expressions/block-expr.html#capture-modes)
- [`Send`](https://doc.rust-lang.org/std/marker/trait.Send.html) and
  [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)

## Lesson 03 · Async behavior in traits

**You need:** Lessons 01 and 02; traits, generic bounds and `dyn Trait`
(Chapter 11).

### The idea

A trait can contain an `async fn`. It is short for a method that returns some
future:

```text
async fn greet(&self, name: &str) -> String;
        means
fn greet(&self, name: &str) -> impl Future<Output = String>;
```

Each implementation has its own body, so each implementation returns its own
future type. Code that uses the trait through a generic bound (`G: Greeter`)
is compiled once for each concrete type. The compiler knows the exact future
type there, stores it inline, and calls it directly. This is **static
dispatch**: no `Box`, no heap allocation, no pointer to a method table.

```rust
use std::{future::Future, pin::pin, task::{Context, Poll, Waker}};

fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

trait Greeter {
    async fn greet(&self, name: &str) -> String;
}

struct Formal;
struct Casual;

impl Greeter for Formal {
    async fn greet(&self, name: &str) -> String {
        format!("Good day, {name}.")
    }
}

impl Greeter for Casual {
    async fn greet(&self, name: &str) -> String {
        format!("Hi {name}!")
    }
}

/// Works for every Greeter. Each call is awaited in place.
async fn welcome<G: Greeter>(greeter: &G, names: &[&str]) -> Vec<String> {
    let mut lines = Vec::new();
    for name in names {
        lines.push(greeter.greet(name).await);
    }
    lines
}

fn main() {
    assert_eq!(run(welcome(&Formal, &["Ada"])), ["Good day, Ada."]);
    assert_eq!(run(welcome(&Casual, &["Ada", "Lin"])), ["Hi Ada!", "Hi Lin!"]);
}
```

### Mistakes people make

**Using such a trait as `dyn Trait`.** A trait object needs one return type
that fits every implementation. "Some future, different for each type" is not
one type.

```rust,compile_fail,E0038
trait Sensor {
    async fn read(&self) -> i32;
}

struct Fixed(i32);

impl Sensor for Fixed {
    async fn read(&self) -> i32 {
        self.0
    }
}

fn main() {
    let sensors: Vec<Box<dyn Sensor>> = vec![Box::new(Fixed(3))];
    println!("{}", sensors.len());
}
```

```text
error[E0038]: the trait `Sensor` is not dyn compatible
  --> src/main.rs:14:30
   |
14 |     let sensors: Vec<Box<dyn Sensor>> = vec![Box::new(Fixed(3))];
   |                              ^^^^^^ `Sensor` is not dyn compatible
   |
note: for a trait to be dyn compatible it needs to allow building a vtable
...
 1 | trait Sensor {
   |       ------ this trait is not dyn compatible...
 2 |     async fn read(&self) -> i32;
   |              ^^^^ ...because method `read` is `async`
```

When you really need trait objects, give the method one fixed return type: a
boxed future. `Pin<Box<dyn Future<Output = T> + 'a>>` is "a heap allocation
that holds some future with output `T`, which may borrow for `'a`".

```rust
use std::{future::Future, pin::{Pin, pin}, task::{Context, Poll, Waker}};

fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

trait DynGreeter {
    fn greet<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = String> + 'a>>;
}

struct Formal;
struct Casual;

impl DynGreeter for Formal {
    fn greet<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = String> + 'a>> {
        Box::pin(async move { format!("Good day, {name}.") })
    }
}

impl DynGreeter for Casual {
    fn greet<'a>(&'a self, name: &'a str) -> Pin<Box<dyn Future<Output = String> + 'a>> {
        Box::pin(async move { format!("Hi {name}!") })
    }
}

fn main() {
    let greeters: Vec<Box<dyn DynGreeter>> = vec![Box::new(Formal), Box::new(Casual)];
    let mut lines = Vec::new();
    for greeter in &greeters {
        lines.push(run(greeter.greet("Ada")));
    }
    assert_eq!(lines, ["Good day, Ada.", "Hi Ada!"]);
}
```

The named lifetime `'a` on both `self` and `name` is needed. With `+ '_` the
future would be allowed to borrow only from `self`, and the compiler rejects a
body that also uses `name`:

```text
error: lifetime may not live long enough
...
  |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ method was supposed to return data with lifetime `'2` but it is returning data with lifetime `'1`
```

**Expecting the future of a trait method to be `Send`.** An `async fn` in a
trait promises nothing about `Send`, so generic code cannot hand the future to
something that needs it.

```rust,compile_fail,E0277
use std::future::Future;

trait Sensor {
    async fn read(&self) -> i32;
}

fn spawn_like<F: Future + Send>(future: F) {
    drop(future);
}

fn watch<S: Sensor + Sync>(sensor: &S) {
    spawn_like(sensor.read());
}

fn main() {}
```

```text
error[E0277]: `impl Future<Output = i32>` cannot be sent between threads safely
...
help: `Send` can be made part of the associated future's guarantees for all implementations of `Sensor::read`
   |
 4 -     async fn read(&self) -> i32;
 4 +     fn read(&self) -> impl std::future::Future<Output = i32> + Send;
```

The help text is the fix. Declare the method in the long form with `+ Send`.
Implementations may still write `async fn read`; the compiler then checks that
their futures really are `Send`.

### When to use which

| You want | Write | Cost |
| --- | --- | --- |
| Speed, and the concrete type is known where the code is compiled | `async fn` in the trait, used through `T: Trait` or `impl Trait` | none |
| A list of different implementations, or a choice made at run time | a method that returns `Pin<Box<dyn Future<...> + 'a>>`, used through `dyn Trait` | one allocation and one indirect call for each call |
| Futures that can be spawned on a multi-threaded runtime | add `+ Send` to the returned future | implementations must not keep `!Send` values across an await |

### Read more

- The Rust Reference: [`impl Trait` in the return position of trait methods](https://doc.rust-lang.org/reference/types/impl-trait.html#return-position-impl-trait-in-traits-and-trait-implementations)
  and [dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)
- [`Pin`](https://doc.rust-lang.org/std/pin/struct.Pin.html) and
  [`Box::pin`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.pin)

## Lesson 04 · Lending async callbacks

**You need:** Lessons 01 and 02; closures and the difference between `Fn`,
`FnMut` and `FnOnce` (Chapter 14).

### The idea

An **async closure** is written `async |args| body`. Calling it returns a
future, as calling an `async fn` does. The important difference from an
ordinary closure that returns an async block is *what the future may borrow*.
The future of an async closure may borrow the closure's own captured
variables. The closure **lends** its captures to the future for as long as
that future lives.

Three traits describe callers' needs, in parallel to the three `Fn` traits:

| Bound | A call lends | Typical use |
| --- | --- | --- |
| `AsyncFn` | shared access to the captures | may be called many times, even with overlapping futures |
| `AsyncFnMut` | exclusive (`&mut`) access to the captures | may be called many times, one future at a time |
| `AsyncFnOnce` | nothing; the call consumes the closure | called once, may move captures out |

All three are in the prelude, so you can name them without a `use` line.

```rust
use std::{future::{Future, poll_fn}, pin::pin, task::{Context, Poll, Waker}};

fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

/// Not ready on the first poll, ready on the second.
async fn pause() {
    let mut first = true;
    poll_fn(|cx| {
        if first {
            first = false;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await
}

/// Calls `check` for each line, one call at a time, and counts the `true`s.
async fn count_accepted<F: AsyncFnMut(&str) -> bool>(lines: &[&str], mut check: F) -> usize {
    let mut accepted = 0;
    for line in lines {
        if check(line).await {
            accepted += 1;
        }
    }
    accepted
}

fn main() {
    let mut lengths = Vec::new();
    let count = run(count_accepted(&["ok", "", "fine"], async |line| {
        pause().await;
        lengths.push(line.len()); // uses the captured Vec after an await
        !line.is_empty()
    }));
    assert_eq!(count, 2);
    assert_eq!(lengths, [2, 0, 4]);
}
```

Follow one loop round. `check(line)` borrows `check` mutably and returns a
future. That future holds the borrow, and through it the `&mut lengths`. The
`.await` runs the future to its end and drops it. Only now is `check` free for
the next call. A `for` loop with one `.await` in it gives exactly this
one-at-a-time pattern.

### Mistakes people make

**Using `FnMut() -> Fut` for a callback that changes captured state.**
With this older pattern the future type `Fut` is one type, fixed before any
call. It cannot contain a borrow that begins with each call.

```rust,compile_fail
use std::future::Future;

async fn call_twice<F, Fut>(mut callback: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = ()>,
{
    callback().await;
    callback().await;
}

fn main() {
    let mut seen = Vec::new();
    drop(call_twice(|| async {
        seen.push("tick");
    }));
}
```

```text
error: captured variable cannot escape `FnMut` closure body
...
14 |       drop(call_twice(|| async {
   |  ______________________-_^
   | |                      |
   | |                      inferred to be a `FnMut` closure
15 | |         seen.push("tick");
   | |         ---- variable captured here
16 | |     }));
   | |_____^ returns an `async` block that contains a reference to a captured variable, which then escapes the closure body
```

The fix has two parts: the function asks for `F: AsyncFnMut()`, and the caller
passes `async || { seen.push("tick"); }`.

**Creating a second future while the first is alive.** With `AsyncFnMut`
each call needs the closure exclusively.

```rust,compile_fail,E0499
async fn call_twice<F: AsyncFnMut()>(mut callback: F) {
    let first = callback();
    let second = callback();
    first.await;
    second.await;
}

fn main() {
    drop(call_twice(async || {}));
}
```

```text
error[E0499]: cannot borrow `callback` as mutable more than once at a time
 --> src/main.rs:3:18
  |
2 |     let first = callback();
  |                 -------- first mutable borrow occurs here
3 |     let second = callback();
  |                  ^^^^^^^^ second mutable borrow occurs here
4 |     first.await;
  |     ----- first borrow later used here
```

Await each call before you make the next one. If the futures really must
overlap, the bound has to be `AsyncFn`, and then the closure cannot change its
captures without a `Cell`, `RefCell` or lock.

**Asking for `Send` on the future of an `AsyncFn` bound.** You cannot yet
write a bound on that future's type, so a generic function cannot promise that
it is `Send`:

```rust,compile_fail,E0277
use std::future::Future;

fn spawn_like<F: Future + Send + 'static>(future: F) {
    drop(future);
}

fn start<F: AsyncFn(u32) -> u32>(handler: F) {
    spawn_like(handler(1));
}

fn main() {
    start(async |n| n + 1);
}
```

```text
error[E0277]: `<F as AsyncFnMut<(u32,)>>::CallRefFuture<'_>` cannot be sent between threads safely
```

### When to use which

Use the `AsyncFn` family when your function awaits the callback's future
itself, in place. That covers most callbacks, and it is the only form that
lets the callback borrow its own state.

Use the older two-parameter form, `F: Fn(A) -> Fut` with
`Fut: Future + Send + 'static`, when your function hands the future to
something else, such as a runtime's `spawn`. In that form the future has a
name (`Fut`), so you can put bounds on it. The price is that such a future
cannot borrow from the closure. This is why APIs that spawn their callbacks
still use it.

### Read more

- [`AsyncFn`](https://doc.rust-lang.org/std/ops/trait.AsyncFn.html),
  [`AsyncFnMut`](https://doc.rust-lang.org/std/ops/trait.AsyncFnMut.html),
  [`AsyncFnOnce`](https://doc.rust-lang.org/std/ops/trait.AsyncFnOnce.html)
- The Rust Reference: [async closures](https://doc.rust-lang.org/reference/expressions/closure-expr.html#async-closures)

## Lesson 05 · Cancellation by dropping

**You need:** Lessons 01 and 02; `Drop` and drop order (Chapter 13).

### The idea

Nothing forces a future to finish. Whoever owns a future can stop polling it
and drop it. That is how async code is **cancelled** in Rust: there is no
special signal and no exception. The body simply never continues after the
await at which it was waiting.

What is dropped with the future? Exactly what the future stores at that
moment:

- A future that was **never polled** stores its arguments or captures. No line
  of the body has run, so no local variable of the body exists.
- A future that is **suspended at an await** stores its arguments or captures,
  and the local variables that were alive at that await. All of them are
  dropped, and their `Drop` code runs. The lines after the await never run.

```rust
use std::{future::Future, task::{Context, Waker}};

struct Lease(&'static str);

impl Drop for Lease {
    fn drop(&mut self) {
        println!("  returned: {}", self.0);
    }
}

async fn print_job(_printer: Lease, wait: impl Future<Output = ()>) {
    println!("  page 1");
    let _tray = Lease("paper tray");
    wait.await;
    println!("  page 2");
}

fn main() {
    let mut cx = Context::from_waker(Waker::noop());

    println!("suspended, then dropped:");
    let mut job = Box::pin(print_job(Lease("printer"), std::future::pending()));
    assert!(job.as_mut().poll(&mut cx).is_pending());
    drop(job);

    println!("run to the end:");
    let mut job = Box::pin(print_job(Lease("printer"), std::future::ready(())));
    assert!(job.as_mut().poll(&mut cx).is_ready());
}
```

```text
suspended, then dropped:
  page 1
  returned: paper tray
  returned: printer
run to the end:
  page 1
  page 2
  returned: paper tray
  returned: printer
```

`std::future::pending()` is a future that is never ready, and
`std::future::ready(v)` is one that is ready at once. In the first half,
"page 2" is never printed, but both leases are returned. This is the good
news about cancellation: values that clean up in `Drop` still clean up.

### The danger: work that is half done

The bad news is that an await can be the last line that ever runs. If the code
changed something before the await and planned to finish the change after it,
a cancel leaves the change half done.

```rust
use std::{future::Future, task::{Context, Waker}};

async fn publish(log: &mut Vec<&'static str>, review: impl Future<Output = ()>) {
    log.push("BEGIN");
    review.await;
    log.push("END");
}

fn main() {
    let mut cx = Context::from_waker(Waker::noop());
    let mut log = Vec::new();
    {
        let mut job = Box::pin(publish(&mut log, std::future::pending()));
        assert!(job.as_mut().poll(&mut cx).is_pending());
    } // job is dropped here, while it waits for the review
    assert_eq!(log, ["BEGIN"]); // no "END": the log is left half written
}
```

There are two repairs.

1. **Wait first, change afterwards.** Do every await before the first change,
   and put no await between the changes. A cancel can then happen only before
   anything was changed.
2. **Use a guard.** When the change must begin before the await, create a
   value whose `Drop` puts things right. The future owns the guard while it is
   suspended, so the guard is dropped on cancel as well as at the normal end.

```rust
use std::{cell::Cell, future::Future, task::{Context, Waker}};

/// Clears the flag when it is dropped: at the normal end, and on cancel.
struct Busy<'a>(&'a Cell<bool>);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

async fn work(flag: &Cell<bool>, wait: impl Future<Output = ()>) {
    flag.set(true);
    let _busy = Busy(flag);
    wait.await;
}

fn main() {
    let mut cx = Context::from_waker(Waker::noop());
    let flag = Cell::new(false);
    let mut job = Box::pin(work(&flag, std::future::pending()));
    assert!(job.as_mut().poll(&mut cx).is_pending());
    assert!(flag.get()); // busy while suspended
    drop(job);
    assert!(!flag.get()); // the guard cleared it
}
```

A guard that should undo work only on cancel, and keep it at the normal end,
needs to know which of the two happened. A common way is a `bool` field that
the code sets on its last line.

### Mistakes people make

**Putting clean-up code after an await.** Lines after an await are not
guaranteed to run. Clean-up belongs in `Drop`.

**Believing that the lines before the first await run at creation.** They run
at the first poll. A future that is dropped without a poll has done nothing.

**Forgetting that a future holds its borrows even when it is not running.**
A future that took `&mut total` keeps that exclusive borrow for as long as the
future is still used somewhere (here, until the `drop`), even if it is never
polled.

```rust,compile_fail,E0502
async fn add_later(total: &mut u32) {
    *total += 1;
}

fn main() {
    let mut total = 0;
    let pending = add_later(&mut total);
    println!("{total}");
    drop(pending);
}
```

```text
error[E0502]: cannot borrow `total` as immutable because it is also borrowed as mutable
 --> src/main.rs:8:16
  |
7 |     let pending = add_later(&mut total);
  |                             ---------- mutable borrow occurs here
8 |     println!("{total}");
  |                ^^^^^ immutable borrow occurs here
9 |     drop(pending);
  |          ------- mutable borrow later used here
```

Finish or drop the future first, then look at the value.

### Read more

- [`Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html)
- [`std::future::pending`](https://doc.rust-lang.org/std/future/fn.pending.html)
  and [`std::future::ready`](https://doc.rust-lang.org/std/future/fn.ready.html)
- The Rust Reference: [destructors](https://doc.rust-lang.org/reference/destructors.html)

## Lesson 06 · Polling a race

**You need:** Lessons 01 and 05. This lesson opens the box: you call `poll`
yourself. Chapter 21 continues from here.

### The idea

Everything so far rests on one trait. Shortened, it looks like this:

```text
trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

enum Poll<T> { Ready(T), Pending }
```

`poll` asks the future to run as far as it can. The rules that both sides must
follow are:

1. `Ready(output)` means the future is finished. Do not poll it again.
   A future made by `async` panics if you do.
2. `Pending` means "not yet". Before a future returns `Pending`, it must make
   sure that the waker in `cx` will be called when progress is possible. An
   executor sleeps until then. A future that returns `Pending` and arranges no
   wake-up is never polled again on a real executor.
3. `self` is `Pin<&mut Self>`: the caller promises not to move the future in
   memory between polls. The state machine of an async block may contain
   references into itself, and a move would break them.

For rule 3 you need two tools. `std::pin::pin!(future)` pins a future in the
current function (or async body) and gives you a `Pin<&mut F>`. Calling
`.as_mut()` on that lends the pinned reference for one `poll` call, so you can
poll again later.

`std::future::poll_fn(closure)` builds a future out of a closure: each poll of
the future calls the closure with the `Context`. With `pin!` and `poll_fn` you
can write a **combinator**, a future that drives other futures, in a few
lines and without `unsafe`.

### A worked example: wait for both

`both` polls two futures on every poll and finishes when each has given its
output. The two futures make progress in turns, inside one task and on one
thread. This is what "concurrent" means for futures.

```rust
use std::{cell::RefCell, future::{Future, poll_fn}, pin::pin, task::{Context, Poll, Waker}};

fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

/// Not ready on the first poll, ready on the second.
async fn pause() {
    let mut first = true;
    poll_fn(|cx| {
        if first {
            first = false;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await
}

async fn both<A: Future, B: Future>(a: A, b: B) -> (A::Output, B::Output) {
    let mut a = pin!(a);
    let mut b = pin!(b);
    let mut a_out = None;
    let mut b_out = None;
    poll_fn(|cx| {
        // Rule 1: a future that has given its output is not polled again.
        if a_out.is_none() {
            if let Poll::Ready(value) = a.as_mut().poll(cx) {
                a_out = Some(value);
            }
        }
        if b_out.is_none() {
            if let Poll::Ready(value) = b.as_mut().poll(cx) {
                b_out = Some(value);
            }
        }
        if a_out.is_some() && b_out.is_some() {
            Poll::Ready((a_out.take().unwrap(), b_out.take().unwrap()))
        } else {
            // Rule 2: each future that is still pending was polled with `cx`
            // in this round, so it has seen the waker.
            Poll::Pending
        }
    })
    .await
}

fn main() {
    let log = RefCell::new(Vec::new());
    let worker = async |name: &'static str| {
        log.borrow_mut().push(format!("{name} starts"));
        pause().await;
        log.borrow_mut().push(format!("{name} ends"));
        name.len()
    };
    assert_eq!(run(both(worker("a"), worker("bb"))), (1, 2));
    assert_eq!(*log.borrow(), ["a starts", "bb starts", "a ends", "bb ends"]);
}
```

Compare the log with Lesson 01: with two sequential awaits it would be
`a starts, a ends, bb starts, bb ends`.

A **race** is the other basic combinator: it finishes as soon as one of its
futures is ready, and the future that lost is dropped, which cancels it
(Lesson 05). A race must decide which future to poll first. If it always
polls the same one first, that one wins every tie; such a race is called
**biased**. The `select!` macro of the Tokio runtime is a race with many
branches; by default it picks the starting branch at random.

### Mistakes people make

**Calling `poll` on a future that is not pinned.**

```rust,compile_fail,E0599
use std::future::{Future, poll_fn};

async fn first_poll_only<F: Future>(future: F) -> bool {
    let mut future = future;
    poll_fn(|cx| future.poll(cx).map(|_| true)).await
}

fn main() {
    drop(first_poll_only(async {}));
}
```

```text
error[E0599]: no method named `poll` found for type parameter `F` in the current scope
...
5 |     poll_fn(|cx| future.poll(cx).map(|_| true)).await
  |                         ^^^^ method not found in `F`
  |
help: consider pinning the expression with `std::pin::pin!()` and assigning that to a new binding
```

`poll` exists only on `Pin<&mut F>`. Do not "fix" this by adding the bound
`F: Unpin`: no async block and no future of an `async fn` is `Unpin`, so your
function would reject nearly every future. This is true even for a block as
simple as `async { 1 }`; the compiler does not look inside to decide.

```rust,compile_fail,E0277
fn needs_unpin<T: Unpin>(_: &T) {}

fn main() {
    let plain = async { 1 };
    needs_unpin(&plain);
}
```

```text
error[E0277]: `{async block@src/main.rs:4:17: 4:22}` cannot be unpinned
...
  = note: consider using the `pin!` macro
          consider using `Box::pin` if you need to access the pinned value outside of the current scope
```

**Polling again after `Ready`.** This compiles and then panics:

```rust,should_panic
use std::{future::Future, pin::pin, task::{Context, Waker}};

fn main() {
    let mut cx = Context::from_waker(Waker::noop());
    let mut job = pin!(async { 1 });
    assert!(job.as_mut().poll(&mut cx).is_ready());
    let _ = job.as_mut().poll(&mut cx); // panics
}
```

```text
thread 'main' (4711) panicked at src/main.rs:5:24:
`async fn` resumed after completion
```

(The number in parentheses is a thread id; it is different in every run. The
message says `async fn` for an async block too.)

A combinator must remember which of its futures are finished, as `both` does
with its two `Option`s.

**Returning `Pending` without a wake-up.** There is no compiler message for
this. The program just stops making progress, because the executor waits for
a call that never comes. The `run` helper in this file hides the mistake,
because it polls in a busy loop; a real executor does not. When you write
`Poll::Pending` yourself, ask: who will call the waker? If your combinator
polled an inner future with the same `cx` and that future was pending, the
inner future has taken care of it.

### Read more

- [`Future::poll`](https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll)
  and [`Poll`](https://doc.rust-lang.org/std/task/enum.Poll.html)
- [`std::future::poll_fn`](https://doc.rust-lang.org/std/future/fn.poll_fn.html)
- [`std::pin::pin!`](https://doc.rust-lang.org/std/pin/macro.pin.html) and the
  [`std::pin` module](https://doc.rust-lang.org/std/pin/index.html), which
  explains why pinning exists
- [`Waker`](https://doc.rust-lang.org/std/task/struct.Waker.html)

## Lesson 07 · Capstone: asynchronous graph traversal

**You need:** Lessons 01 to 03; `VecDeque`, `BTreeSet` and `BTreeMap`
(Chapter 16); the `?` operator (Chapter 7).

The capstone joins the chapter's ideas in one small program: an async method
in a trait, a generic function that awaits it in a loop, and errors that
travel through `.await?`. This section explains the approach. The code is
yours to write.

### The problem in plain words

A **graph** is a set of named nodes; each node lists the names of its
neighbors. Asking for a node's neighbors is the slow step (think of a network
request), so it is an async method. Starting from one node, you must find
every node that can be reached, ask for each node's neighbors exactly once,
and stop at the first failed request.

**Breadth-first** order means: first the start node, then all its neighbors,
then their neighbors, and so on. The usual tool is a queue: take a name from
the front, and put newly found names at the back.

```text
graph:   hub ──► east ──► far          queue over time (front on the left)
          │                ▲
          └────► west ─────┘           [hub]
                                       [east, west]      after expanding hub
order of requests:                     [west, far]       after expanding east
hub, east, west, far                   [far, far]        after expanding west
                                       [far]             far is expanded once;
                                       []                the second copy is skipped
```

### Questions to answer before you write code

1. Graphs can contain cycles (`a` lists `b`, and `b` lists `a`). What must
   your loop remember so that it ends? Which collection gives you that memory
   and also the sorted result that the contract asks for?
2. The same name can be in the queue twice, as `far` is above. At which point
   do you notice this, so that no node is requested twice?
3. A request returns a `Result`. Where does the `?` go relative to `.await`,
   and what does the function return in that case?
4. The function takes `&G` with `G: Graph`. Is anything boxed or allocated
   for the call itself (Lesson 03)?

### What `async` adds here, and what it does not

Your loop awaits one request and continues only when it has the answer. So
only one request is in progress at any time. The traversal is **sequential**,
exactly like a version without `async`. What `async` adds is that the
function can pause at each request without blocking a thread, so an executor
can run other tasks during the wait.

A concurrent traversal would keep several requests in progress, for example
with a combinator such as `both` from Lesson 06 or with tasks on a runtime.
It would also need a limit on the number of requests in progress, and a
decision about what happens to the other requests when one fails. Lab 1 in
[LABS.md](LABS.md) is that version; it is optional.

### After the tests pass

Explain your design in NOTES.md. Then change one requirement in your head,
for example "return the nodes in the order they were first reached, not
sorted", and say which collection you would change and why.

### Read more

- [`VecDeque`](https://doc.rust-lang.org/std/collections/struct.VecDeque.html)
  and [`BTreeSet`](https://doc.rust-lang.org/std/collections/struct.BTreeSet.html)
- [The `?` operator](https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-try-propagation-expression)
