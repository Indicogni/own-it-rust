# Coming to Rust from Python

[Course overview](../README.md) · [All guides](README.md)

Python lets you write a working program quickly and finds many mistakes only when that line runs. Rust moves most of those discoveries to compile time. The first weeks feel slower, because the compiler asks questions that Python never asked: what type is this, who owns it, can it be absent, can this fail. After that, a program that compiles tends to keep working. This page tells you what you can keep, what you must unlearn, and where to spend your time in this course.

## What transfers

- **Iteration as a way of thinking.** List comprehensions and generator expressions become iterator chains: `.iter().filter(...).map(...).collect()`. Like generators, Rust iterators are lazy.
- **Duck typing, made explicit.** "Anything with a `__len__`" becomes "anything that implements a trait". You name the trait, and the compiler checks it.
- **`with` blocks.** A value that owns a resource closes it when it goes out of scope. You do not need `with`; every scope is one.
- **Tests next to the code.** `#[test]` functions and `cargo test` are as easy as pytest.
- **Formatted strings.** `f"{name}: {count}"` becomes `format!("{name}: {count}")`.

## What does not transfer

- **Every value has a type, fixed at compile time.** Variables do not change type. A list holds one type of element.
- **Variables are not labels.** In Python, `b = a` makes two names for one object. In Rust it moves the value: `a` can no longer be used. To get a second, independent copy you call `.clone()`.
- **Integers have a size.** An `i32` goes up to about two billion. Overflow panics in a debug build and wraps in a release build (Cargo's default settings).
- **Strings are UTF-8 bytes.** You cannot index a string by character position.
- **Errors are return values.** There are no exceptions to catch. A function that can fail returns `Result`.

## Five mistakes Python programmers make

### 1. Indexing a string

```rust,compile_fail,E0277
fn main() {
    let word = String::from("café");
    let first = word[0]; // E0277: the type `str` cannot be indexed by `{integer}`
}
```

A Rust string is a buffer of UTF-8 bytes, and one character can take up to four of them. Ask for what you mean:

```rust
fn main() {
    let word = "café";
    assert_eq!(word.len(), 5);                  // bytes
    assert_eq!(word.chars().count(), 4);        // characters
    assert_eq!(word.chars().next(), Some('c')); // the first character
    assert_eq!(&word[..3], "caf");              // a byte range on character boundaries
}
```

### 2. Changing a list while looping over it

Python lets you append to a list inside `for x in items`, sometimes with surprising results. Rust refuses, because the loop holds a borrow of the vector:

```rust,compile_fail,E0502
fn main() {
    let mut items = vec![1, 2, 3];
    for x in &items {
        if *x > 1 {
            items.push(*x * 10); // E0502: cannot borrow `items` as mutable
        }
    }
}
```

Collect the new values first, then add them:

```rust
fn main() {
    let mut items = vec![1, 2, 3];
    let extra: Vec<i32> = items.iter().filter(|x| **x > 1).map(|x| x * 10).collect();
    items.extend(extra);
    assert_eq!(items, [1, 2, 3, 20, 30]);
}
```

### 3. Forgetting that integers overflow

```rust,should_panic
fn area(width: i32, height: i32) -> i32 {
    width * height
}

fn main() {
    // 100_000 * 100_000 does not fit in an i32: a debug build panics with
    // "attempt to multiply with overflow".
    let big = area(100_000, 100_000);
    println!("{big}");
}
```

Choose a wider type (`i64`, `u64`, `i128`) when values can be large, and use `checked_mul` and friends when overflow is possible and must be handled.

### 4. Cloning to make the compiler quiet

When the borrow checker complains, `.clone()` often makes the error go away. It also hides the design question the compiler was asking. Before you clone, ask whether the function needs to own the value or only to read it. A function that only reads text should take `&str`, not `String`:

```rust
fn shout(text: &str) -> String {
    text.to_uppercase()
}

fn main() {
    let name = String::from("ada");
    let loud = shout(&name); // borrow: `name` is still usable
    assert_eq!((name.as_str(), loud.as_str()), ("ada", "ADA"));
}
```

### 5. Using a dictionary as an object

A `dict` with keys `"name"` and `"age"` becomes a `struct` with two typed fields. A dictionary whose values have different types becomes a map to an `enum`. Both catch typos and wrong types at compile time:

```rust
struct Person {
    name: String,
    age: u8,
}

fn main() {
    let p = Person { name: "Lin".to_string(), age: 31 };
    assert_eq!(format!("{} is {}", p.name, p.age), "Lin is 31");
}
```

## Where to spend your time

Do not skip Chapters 1 and 2. They introduce the compiler, Cargo and tests at an easy pace, and they show the whole everyday language once.

| Priority | Unit | Why |
| --- | --- | --- |
| High | [Chapter 2, A first tour](../exercises/chapter02/README.md) | Types, structs, enums and `match` at first contact. |
| High | [Chapter 3, Numbers, text and arrays](../exercises/chapter03/README.md) | Sized integers, floats, strings as bytes. |
| High | [Chapter 4](../exercises/chapter04/README.md) and [Chapter 5](../exercises/chapter05/README.md) | Ownership and borrowing: the idea Python does not have. Expect them to take longer than the estimates. |
| High | [Chapter 7, Handling errors](../exercises/chapter07/README.md) | `Result` and `?` instead of `try`/`except`. |
| Medium | [Chapter 15, Iterators and adapters](../exercises/chapter15/README.md) | Comprehensions and generators, with ownership. |
| Medium | [Chapter 17, Working with text](../exercises/chapter17/README.md) | Bytes, characters, and formatting. |
| Later | [Chapter 23, Unsafe Rust](../exercises/chapter23/README.md) | New territory; take your time. |

## Idiom map

| Python | Rust |
| --- | --- |
| `list` | `Vec<T>` |
| `dict`, `set` | `HashMap<K, V>`, `HashSet<T>` (or `BTreeMap`, `BTreeSet` for sorted order) |
| `tuple` | tuple `(A, B)`, or a struct with named fields |
| `None` | `Option<T>` |
| `raise` / `try`/`except` | return `Err(...)` / `match` or `?` |
| `[f(x) for x in xs if p(x)]` | `xs.iter().filter(...).map(...).collect::<Vec<_>>()` |
| generator with `yield` | a type that implements `Iterator` |
| `with open(path) as f:` | `let f = File::open(path)?;` (closed when `f` goes out of scope) |
| `class` with methods | `struct` plus `impl` |
| abstract base class, protocol | trait |
| `len(s)` on a string | `s.chars().count()` (characters) or `s.len()` (bytes) |
| `f"{x}"` | `format!("{x}")` |
| `pip`, `venv`, `pyproject.toml` | Cargo, `Cargo.toml`, `Cargo.lock` |
| `pytest` | `cargo test` |

## Read more

- [std::string::String](https://doc.rust-lang.org/std/string/struct.String.html)
- [Integer overflow in the Rust Reference](https://doc.rust-lang.org/reference/expressions/operator-expr.html#overflow)
- [std::iter](https://doc.rust-lang.org/std/iter/index.html)
