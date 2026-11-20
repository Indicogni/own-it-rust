# Coming to Rust from JavaScript and TypeScript

[Course overview](../README.md) · [All guides](README.md)

If you write TypeScript, you already like a compiler that finds mistakes before your users do. Rust goes further: its types are checked everywhere, there is no `any` to escape to, and the compiler also checks how memory is used and shared. There is no garbage collector and no event loop built into the language. This page tells you what you can keep, what you must unlearn, and where to spend your time in this course.

## What transfers

- **Closures and higher-order functions.** Arrow functions become closures: `|x| x * 2`. Passing functions to `map`, `filter` and `sort_by` works the way you expect.
- **`async` and `await`.** The syntax is almost the same, and the purpose is the same: wait for input and output without blocking a thread.
- **Discriminated unions.** A TypeScript union such as `{ kind: "circle"; r: number } | { kind: "square"; side: number }` is a Rust `enum`, and `match` checks that you handled every case, like an exhaustive `switch` with `never`.
- **Optional values.** `x?.y ?? fallback` has a close relative in `x.map(...).unwrap_or(fallback)` and the `?` operator.
- **Package management.** `npm`, `package.json` and `package-lock.json` become Cargo, `Cargo.toml` and `Cargo.lock`.

## What does not transfer

- **Types are nominal.** Two structs with the same fields are different types. TypeScript's structural typing does not apply.
- **Numbers have many types.** There is no single `number`. You choose `i32`, `u64`, `f64` and so on, and they do not mix without a conversion.
- **Strings are UTF-8.** JavaScript strings are UTF-16, so lengths and positions differ for anything outside ASCII.
- **Futures are lazy.** A `Promise` starts running when you create it. A Rust future does nothing until something polls it, usually an `.await` inside a runtime.
- **No garbage collector.** Every value has one owner. A closure that outlives its function must own what it uses.

## Five mistakes JavaScript and TypeScript programmers make

### 1. Expecting an async call to start by itself

This program builds a future and polls it by hand, with the standard library only, to show when the body runs:

```rust
use std::cell::Cell;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

async fn record(log: &Cell<u32>) {
    log.set(log.get() + 1);
}

fn main() {
    let log = Cell::new(0);
    let future = record(&log);
    assert_eq!(log.get(), 0, "creating the future runs none of its body");

    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(future.as_mut().poll(&mut cx), Poll::Ready(()));
    assert_eq!(log.get(), 1, "the body ran when the future was polled");
}
```

In real programs a runtime such as tokio does the polling. Chapter 21 builds that machinery from scratch.

### 2. Returning a closure that borrows a local variable

In JavaScript, a closure keeps its captured variables alive. In Rust, a closure that only borrows `count` cannot outlive the function that owns `count`:

```rust,compile_fail,E0373
fn counter() -> impl FnMut() -> u32 {
    let mut count = 0;
    || {
        count += 1; // E0373: closure may outlive the current function
        count
    }
}
```

Add `move`, so that the closure owns its own copy of the counter:

```rust
fn counter() -> impl FnMut() -> u32 {
    let mut count = 0;
    move || {
        count += 1;
        count
    }
}

fn main() {
    let mut next = counter();
    assert_eq!((next(), next(), next()), (1, 2, 3));
}
```

### 3. Measuring strings in the wrong unit

```rust
fn main() {
    let word = "naïve";
    assert_eq!(word.len(), 6);           // UTF-8 bytes; JavaScript's .length is 5
    assert_eq!(word.chars().count(), 5); // Unicode scalar values
    assert_eq!(word.find('v'), Some(4)); // positions are byte offsets
}
```

Use `.chars()` to walk characters and byte offsets from `find` to slice. Never cut a string at a computed byte index without checking `is_char_boundary`.

### 4. Mixing integers and floats

```rust,compile_fail,E0277
fn main() {
    let total = 1 + 2.5; // E0277: cannot add a float to an integer
}
```

Write the literal as a float (`1.0 + 2.5`) or convert explicitly (`f64::from(count) + 2.5`). The conversion is a decision about precision, and Rust makes you write it down.

### 5. Relying on structural typing

```rust,compile_fail,E0308
struct User { name: String }
struct Admin { name: String }

fn greet(user: &User) -> String { format!("hi {}", user.name) }

fn main() {
    let admin = Admin { name: "root".to_string() };
    greet(&admin); // E0308: mismatched types, expected `&User`, found `&Admin`
}
```

If several types should be accepted, say so with a trait:

```rust
trait Named { fn name(&self) -> &str; }

struct User { name: String }
struct Admin { name: String }

impl Named for User { fn name(&self) -> &str { &self.name } }
impl Named for Admin { fn name(&self) -> &str { &self.name } }

fn greet(who: &impl Named) -> String { format!("hi {}", who.name()) }

fn main() {
    assert_eq!(greet(&Admin { name: "root".to_string() }), "hi root");
    assert_eq!(greet(&User { name: "kim".to_string() }), "hi kim");
}
```

## Where to spend your time

| Priority | Unit | Why |
| --- | --- | --- |
| High | [Chapter 2, A first tour](../exercises/chapter02/README.md) and [Chapter 3](../exercises/chapter03/README.md) | Sized numbers, strings as bytes, structs and enums. |
| High | [Chapter 4](../exercises/chapter04/README.md) and [Chapter 5](../exercises/chapter05/README.md) | Ownership and borrowing, which JavaScript does not have. |
| High | [Chapter 14, Closures and captures](../exercises/chapter14/README.md) | Capture by borrow or by move, and the `Fn`, `FnMut`, `FnOnce` traits. |
| High | [Chapter 20](../exercises/chapter20/README.md) and [Chapter 21](../exercises/chapter21/README.md) | Async without an event loop in the language: what a future is and who runs it. |
| Medium | [Chapter 10, Enums and pattern matching](../exercises/chapter10/README.md) | Your discriminated unions, with exhaustive checking. |
| Medium | [Chapter 17, Working with text](../exercises/chapter17/README.md) | UTF-8 versus UTF-16. |
| Then | [Ecosystem track](../ecosystem/README.md) | serde for JSON, tokio, an HTTP client and server. |

## Idiom map

| JavaScript / TypeScript | Rust |
| --- | --- |
| `number` | `i32`, `i64`, `u64`, `f64`, ... |
| `string` | `String` (owned) and `&str` (borrowed) |
| `Array<T>` | `Vec<T>`; `&[T]` for a borrowed view |
| `Map`, `Set` | `HashMap`, `HashSet` (or `BTreeMap`, `BTreeSet`) |
| `undefined`, `null`, `T \| undefined` | `Option<T>` |
| `throw` / `try`/`catch` | `Result<T, E>` and `?` |
| discriminated union | `enum` with data |
| `interface` (structural) | `trait` (implemented explicitly) |
| `(x) => x * 2` | `\|x\| x * 2` |
| `Promise<T>` | `impl Future<Output = T>` (lazy) |
| `arr.map(f).filter(g)` | `v.iter().map(f).filter(g).collect()` |
| `JSON.parse` / `JSON.stringify` | serde with serde_json (ecosystem track) |
| `const` binding | `let` (immutable by default) |
| `let` binding | `let mut` |
| npm, `package.json` | Cargo, `Cargo.toml` |
| Jest, Vitest | `#[test]` and `cargo test` |

## Read more

- [Closure expressions in the Rust Reference](https://doc.rust-lang.org/reference/expressions/closure-expr.html)
- [std::future::Future](https://doc.rust-lang.org/std/future/trait.Future.html)
- [`str` and UTF-8](https://doc.rust-lang.org/std/primitive.str.html)
