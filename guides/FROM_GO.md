# Coming to Rust from Go

[Course overview](../README.md) · [All guides](README.md)

Go and Rust were designed in the same years for the same kind of work: servers, command-line tools, infrastructure. They made opposite choices about memory. Go has a garbage collector and a runtime that schedules goroutines for you. Rust has neither; ownership rules decide when memory is freed, and you choose how concurrent code runs. This page tells you what you can keep, what you must unlearn, and where to spend your time in this course.

## What transfers

- **Errors are values.** A Go function returns `(T, error)`; a Rust function returns `Result<T, E>`. You already think about failure at every call.
- **Structs with methods, composition over inheritance.** Rust has no classes either. Methods live in `impl` blocks, much like Go's receiver functions.
- **Interfaces.** A Rust trait plays the role of a Go interface, and `dyn Trait` is the equivalent of an interface value.
- **Tooling culture.** `cargo fmt`, `cargo test`, `cargo doc` and `Cargo.lock` feel like `gofmt`, `go test`, `go doc` and `go.sum`.
- **Channels.** `std::sync::mpsc` gives you channels between threads, and "share memory by communicating" works in Rust too.

## What does not transfer

- **No garbage collector.** Every value has one owner. When the owner goes out of scope, the value is freed. Passing a value to a function moves it there unless you pass a reference.
- **No zero values.** A Rust variable must be initialised before use, and a struct literal must name every field. `Default` gives you zero-like values only when a type opts in.
- **No nil.** In safe code, a reference always points at a live value. "Maybe absent" is written `Option<T>`.
- **Traits are implemented on purpose.** A Go type satisfies an interface by having the right methods. A Rust type implements a trait only with an explicit `impl Trait for Type`.
- **No built-in scheduler.** Threads are operating-system threads. `async` code needs an executor, which the standard library does not include.

## Five mistakes Go programmers make

### 1. Writing `if err != nil` by hand

A `match` on every `Result` works, but it buries the logic. The `?` operator returns the error to the caller, converting it with `From` if needed:

```rust
use std::num::ParseIntError;

fn port(text: &str) -> Result<u16, ParseIntError> {
    let value = text.trim().parse::<u16>()?; // early return on error
    Ok(value)
}

fn main() {
    assert_eq!(port(" 8080 "), Ok(8080));
    assert!(port("eighty").is_err());
}
```

Ignoring a `Result` entirely earns a compiler warning (`unused_must_use`). Do not silence it with `let _ =` unless you have decided that the error does not matter.

### 2. Expecting zero values

```rust,compile_fail,E0063
struct Config {
    host: String,
    port: u16,
    verbose: bool,
}

fn main() {
    let c = Config { host: String::from("localhost"), port: 8080 }; // E0063: missing field `verbose`
}
```

Derive `Default` and fill in the rest with struct update syntax when zero-like values really are correct:

```rust
#[derive(Default)]
struct Config {
    host: String,
    port: u16,
    verbose: bool,
}

fn main() {
    let c = Config { port: 8080, ..Default::default() };
    assert!(!c.verbose && c.host.is_empty());
}
```

### 3. Starting a thread that borrows local data

A goroutine can capture anything, because the garbage collector keeps it alive. A Rust thread started with `thread::spawn` may outlive the function that started it, so it may not borrow that function's locals:

```rust,compile_fail,E0373
use std::thread;

fn main() {
    let lines = vec!["a", "b"];
    let handle = thread::spawn(|| lines.len()); // E0373: closure may outlive the current function
    println!("{}", handle.join().unwrap());
}
```

Either move the data into the thread (`move ||`), share it with `Arc`, or use a scoped thread, which is guaranteed to finish before the scope ends and may therefore borrow:

```rust
use std::thread;

fn main() {
    let lines = vec!["a", "b", "c"];
    let total = thread::scope(|s| {
        let left = s.spawn(|| lines[..1].len());
        let right = s.spawn(|| lines[1..].len());
        left.join().unwrap() + right.join().unwrap()
    });
    assert_eq!(total, 3);
}
```

### 4. Treating async functions like goroutines

`go save(x)` starts work at once. Calling an `async fn` in Rust only builds a future; nothing runs until something polls it. The compiler warns you:

```text
warning: unused implementer of `Future` that must be used
 --> fut.rs:4:5
  |
4 |     save("draft");
  |     ^^^^^^^^^^^^^
  |
  = note: futures do nothing unless you `.await` or poll them
```

Chapters 20 and 21 teach what a future is. The ecosystem track's tokio lesson shows the runtime most services use.

### 5. Reaching for `interface{}`

Go code often uses an empty interface and a type switch. In Rust, a closed set of alternatives is an `enum`, and `match` checks that you handled every case:

```rust
enum Value {
    Int(i64),
    Text(String),
}

fn describe(v: &Value) -> String {
    match v {
        Value::Int(n) => format!("int {n}"),
        Value::Text(s) => format!("text {s:?}"),
    }
}

fn main() {
    assert_eq!(describe(&Value::Int(3)), "int 3");
    assert_eq!(describe(&Value::Text("hi".into())), "text \"hi\"");
}
```

Use a generic function when the set is open and known at compile time, and `dyn Trait` when it is open and decided at run time. `std::any::Any` exists, but you will rarely need it.

## Where to spend your time

| Priority | Unit | Why |
| --- | --- | --- |
| High | [Chapter 4](../exercises/chapter04/README.md) and [Chapter 5](../exercises/chapter05/README.md) | Ownership and borrowing replace the garbage collector. Everything else depends on them. |
| High | [Chapter 10, Enums and pattern matching](../exercises/chapter10/README.md) | Enums with data replace type switches and many uses of interfaces. |
| High | [Chapter 11, Generic code with traits](../exercises/chapter11/README.md) | Explicit trait implementations, generics, and when to use `dyn`. |
| High | [Chapter 19](../exercises/chapter19/README.md), [20](../exercises/chapter20/README.md) and [21](../exercises/chapter21/README.md) | Threads, `Send` and `Sync`, and async without a built-in scheduler. |
| Medium | [Chapter 7, Handling errors](../exercises/chapter07/README.md) | Familiar idea; learn `?`, error enums and `From`. |
| Then | [Ecosystem track](../ecosystem/README.md) and [M11](../mastery/m11_services/README.md) | The crates and the service stack you will use instead of `net/http`. |

## Idiom map

| Go | Rust |
| --- | --- |
| `func f() (T, error)` | `fn f() -> Result<T, E>` |
| `if err != nil { return err }` | `?` |
| `nil` pointer, `ok` flag | `Option<T>` |
| `[]T` (owned) / sub-slice | `Vec<T>` / `&[T]` |
| `map[K]V` | `HashMap<K, V>`; `BTreeMap<K, V>` for sorted keys |
| `interface` | trait; `dyn Trait` for an interface value |
| type switch | `enum` and `match` |
| `go f()` | `thread::spawn`, `thread::scope`, or a task on an async runtime |
| `chan T` | `std::sync::mpsc::channel` (several senders, one receiver) |
| `sync.Mutex` beside the data | `Mutex<T>`, which owns the data |
| `sync.WaitGroup` | `thread::scope` |
| `defer` | a value whose `Drop` does the clean-up |
| `panic` / `recover` | `panic!` for bugs; `Result` for expected failures |
| `go test`, `gofmt`, `go vet` | `cargo test`, `cargo fmt`, `cargo clippy` |

## Read more

- [std::result](https://doc.rust-lang.org/std/result/index.html) and the [`?` operator](https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator)
- [std::thread::scope](https://doc.rust-lang.org/std/thread/fn.scope.html)
- [The Rust Reference: async blocks](https://doc.rust-lang.org/reference/expressions/block-expr.html#async-blocks)
