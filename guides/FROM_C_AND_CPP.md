# Coming to Rust from C and C++

[Course overview](../README.md) · [All guides](README.md)

You already think about memory: who allocates, who frees, how long a pointer stays valid. Rust asks the same questions. The difference is that the compiler checks your answers. This page tells you what you can keep, what you must unlearn, and where to spend your time in this course.

## What transfers

- **Value semantics and the stack.** A Rust `struct` is a plain value with no hidden object header. With `#[repr(C)]` it has C layout. Without it, the layout is unspecified and the compiler may reorder fields: on Rust 1.99, `struct S { a: u8, b: u32, c: u8 }` takes 8 bytes with `b` first, where the C struct takes 12. Local values live on the stack; `Box<T>` puts one on the heap.
- **RAII.** A C++ destructor is a Rust `Drop` implementation. Values are destroyed at the end of their scope, in reverse order of declaration; a value that was moved away is destroyed by its new owner instead. Files, locks and sockets close themselves the same way.
- **Zero-cost abstractions.** Generics are compiled once per concrete type, like templates. In an optimised build, iterator chains usually compile to the same code as the loop you would write by hand.
- **Thinking in bytes.** `u8`, `i32`, `usize`, `[T; N]`, `size_of`, alignment, two's complement: all of this works the way you expect.

## What does not transfer

- **Assignment moves.** `let b = a;` moves `a` unless the type is `Copy` (plain numbers, `bool`, `char`, shared references, tuples or arrays of those, and types that derive `Copy`). There is no copy constructor that runs behind your back. Copying a `String` or a `Vec` needs an explicit `.clone()`.
- **References are checked.** In safe code, a `&T` (shared, read-only) or `&mut T` (exclusive) can never dangle and can never be null. At any moment a value has either many `&T` or one `&mut T`, never both.
- **No null, no exceptions.** `Option<T>` replaces a nullable pointer; `Result<T, E>` replaces error codes and exceptions.
- **No inheritance and no overloading.** Behaviour is shared through traits. A function name has one signature.
- **No undefined behaviour in safe code.** Out-of-bounds indexing panics. Integer overflow, signed or unsigned, panics in a debug build and wraps in a release build (Cargo's default settings); it is never undefined.

## Five mistakes C and C++ programmers make

### 1. Expecting assignment to copy

```rust,compile_fail,E0382
fn main() {
    let names = vec![String::from("ada"), String::from("lin")];
    let backup = names;          // a move: `names` may not be used any more (nothing is copied or emptied)
    println!("{}", names.len()); // E0382: borrow of moved value
}
```

The heap buffer has exactly one owner, so only one variable can free it. Clone when you really need two buffers, or borrow (`let backup = &names;`) when you only need to look.

### 2. Keeping a pointer into a vector while it grows

In C++ this compiles and sometimes crashes: `push_back` can reallocate and leave `first` dangling. Rust refuses it.

```rust,compile_fail,E0502
fn main() {
    let mut scores = vec![10, 20];
    let first = &scores[0];
    scores.push(30);       // E0502: cannot borrow as mutable
    println!("{first}");
}
```

The fix is to finish using the reference before the change, or to keep an index instead of a reference:

```rust
fn main() {
    let mut scores = vec![10, 20];
    let first = 0; // an index stays valid when the vector grows
    scores.push(30);
    assert_eq!(scores[first], 10);
}
```

### 3. Building linked structures from raw pointers

A doubly linked list or a graph with back pointers is the first thing many C++ programmers try, and the hardest thing to write in safe Rust. Start with a `Vec` and indices; reach for `Rc` and `Weak` (Chapter 9) when you need shared ownership. Write `unsafe` code only after Chapter 23, and then keep it small, behind a safe interface, with a `// SAFETY:` comment for every block.

### 4. Trusting casts

`as` never fails. It truncates, wraps and saturates without telling you:

```rust
fn main() {
    let big: i32 = 300;
    assert_eq!(big as u8, 44);         // 300 - 256: bits are dropped
    assert_eq!(-1_i32 as u32, u32::MAX);
    assert_eq!(f64::NAN as i32, 0);    // float-to-int casts saturate; NaN gives 0
    assert!(u8::try_from(big).is_err()); // the checked conversion
}
```

Use `From` for conversions that cannot lose information and `TryFrom` for the others. For arithmetic that can overflow, say which policy you want: `checked_add`, `wrapping_add` or `saturating_add`.

### 5. Writing C++ error handling in Rust

`unwrap()` is not `assert`, and it is not a substitute for exceptions. A library function that can fail returns `Result`, and the caller passes the error up with `?`:

```rust
use std::num::ParseIntError;

fn total(fields: &[&str]) -> Result<u64, ParseIntError> {
    let mut sum = 0;
    for field in fields {
        sum += field.trim().parse::<u64>()?; // returns the error to the caller
    }
    Ok(sum)
}

fn main() {
    assert_eq!(total(&["1", " 2"]), Ok(3));
    assert!(total(&["1", "x"]).is_err());
}
```

## Where to spend your time

Chapters 1 and 2 are short for you; do them anyway, because they teach the course's tools and the lesson loop. Then go slowly where your instincts are wrong:

| Priority | Unit | Why |
| --- | --- | --- |
| High | [Chapter 4, Ownership and moves](../exercises/chapter04/README.md) | Moves replace copy constructors and `std::move`. |
| High | [Chapter 5, Borrowing with references](../exercises/chapter05/README.md) | Lifetimes are the compiler's version of "this pointer must not outlive that buffer". |
| High | [Chapter 7, Handling errors](../exercises/chapter07/README.md) | `Result` and `?` replace exceptions and error codes. |
| High | [Chapters 11](../exercises/chapter11/README.md) and [13](../exercises/chapter13/README.md) | Traits replace virtual functions, templates with concepts, and operator conventions. |
| Medium | [Chapter 3, Numbers, text and arrays](../exercises/chapter03/README.md) | Overflow and cast rules differ from C. |
| Faster | Chapters 6, 12, 16 | Familiar ideas with new syntax. |
| Your strength | [Chapter 23](../exercises/chapter23/README.md), [Chapter 24](../exercises/chapter24/README.md), [M2](../mastery/m2_unsafe/README.md), [M6](../mastery/m6_ffi/README.md), [M7](../mastery/m7_perf/README.md) | Unsafe code, FFI and performance, where your C and C++ experience pays off. Do not skip ahead to them: their safety arguments need the ownership model. |

## Idiom map

| C or C++ | Rust |
| --- | --- |
| `std::unique_ptr<T>` | `Box<T>` |
| `std::shared_ptr<T>`, `std::weak_ptr<T>` | `Rc<T>` and `Weak<T>` in one thread; `Arc<T>` across threads |
| `std::vector<T>`, `std::string` | `Vec<T>`, `String` |
| `std::span<T>`, `std::string_view` | `&[T]`, `&str` |
| `const T&`, `T&` | `&T`, `&mut T` |
| `std::move(x)` | `x` (moving is the default) |
| copy constructor | `#[derive(Clone)]` and an explicit `.clone()` |
| destructor | `impl Drop` |
| `std::optional<T>`, nullable `T*` | `Option<T>`, `Option<&T>`, `Option<Box<T>>` |
| exceptions, `errno` | `Result<T, E>` and `?` |
| virtual functions | `dyn Trait` |
| templates and concepts | generics with trait bounds |
| `enum class` plus `union` | `enum` with data, taken apart with `match` |
| `std::mutex` beside the data | `Mutex<T>`, which owns the data it protects |
| header files, `#include` | modules and `use` |
| CMake or Make | Cargo |

## Read more

- [The Rust Reference: type layout](https://doc.rust-lang.org/reference/type-layout.html), including `repr(C)`
- [The Rust Reference: behaviour considered undefined](https://doc.rust-lang.org/reference/behavior-considered-undefined.html)
- [std::boxed](https://doc.rust-lang.org/std/boxed/index.html) and [std::rc](https://doc.rust-lang.org/std/rc/index.html)
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/), after Chapter 23
