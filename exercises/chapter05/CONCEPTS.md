# Chapter 5 concepts — Borrowing with references

[Chapter guide](README.md) · [Hints](HINTS.md) · [Notes workbook](NOTES.md)

Read one section, then do the lesson with the same number. Each section gives you the idea, one worked example that is *not* the exercise, the mistakes people really make with the compiler's own words, and links to the official documentation.

## What you will be able to do

- Lend a value to a function instead of giving it away, for reading or for changing.
- Say exactly what a reference is at run time: one address, or an address and a length.
- Read a function signature with lifetimes and tell which input the result points into.
- Write a struct that borrows its data, and say how long it may be used.
- Read the borrow checker's most common errors (E0499, E0502, E0505, E0506, E0515, E0597, E0106) and fix them without cloning everything.
- Hand out two exclusive references into one slice safely.

## Before you start

- Chapter 3: integers, `String` and `&str`, `Vec<T>`, arrays and slices.
- Chapter 4: every value has exactly one owner, a move transfers ownership, and a value is dropped when its owner goes away.

## Time

These are estimates for a careful first pass: reading the section, predicting, implementing, and writing your explanation.

| Part | Estimate |
| --- | --- |
| Lesson 01 · Borrowing and dereferencing | 40 minutes |
| Lesson 02 · Reference bindings and identity | 45 minutes |
| Lesson 03 · Optional references and fat pointers | 45 minutes |
| Lesson 04 · Lifetimes through functions | 50 minutes |
| Lesson 05 · Structs that borrow | 50 minutes |
| Lesson 06 · Independent lifetimes and elision | 45 minutes |
| Lesson 07 · Sharing versus mutation | 50 minutes |
| Lesson 08 · Disjoint mutable borrows | 60 minutes |
| Lesson 09 · Capstone | 45 minutes |
| Review R01 and code review C01 | 90 minutes together |
| 13 compiler drills | 75 minutes |

## Words used in this chapter

| Word | Meaning |
| --- | --- |
| reference | A value that holds the address of another value. Written `&T` or `&mut T`. |
| borrow | To create a reference. The owner keeps ownership. |
| referent | The value that a reference points at. |
| dereference | To follow a reference to its referent. Written `*r`. |
| shared reference | `&T`. You may read through it. Many may exist at once. |
| exclusive reference | `&mut T`. You may read and write through it. While it is in use, nothing else may touch the referent. Often called a "mutable reference". |
| lifetime | The part of the program in which a reference is used. |
| dangling | Pointing at memory that has been freed or reused. Safe Rust never lets a reference dangle. |
| machine word | The size of `usize`: 8 bytes on a 64-bit computer. |

## Lesson 01 · Borrowing and dereferencing

**Assumed:** chapter 4. Passing a `String` to a function by value moves it, and the caller cannot use it afterwards.

### The idea

Often a function only needs to *look at* a value, or to *change it in place*. It does not need to own it. For this, Rust has references.

A reference is a small value that holds the address of another value. Creating a reference is called *borrowing*. The value at that address is the *referent*. Borrowing does not change who owns the referent, and when the reference goes away, nothing is freed.

There are two kinds:

- `&T` is a **shared** reference. You may read the referent. Any number of shared references to one value may exist.
- `&mut T` is an **exclusive** reference. You may read and write the referent. While it is in use, it is the only way to reach the referent.

You create them with `&value` and `&mut value`. You follow a reference to its referent with `*reference`. This is called *dereferencing*.

### Worked example

```rust
fn describe(score: &i32) -> String {
    format!("score {score}")
}

fn penalize(score: &mut i32, points: i32) {
    *score -= points;
}

fn main() {
    let mut score = 90;
    let text = describe(&score); // lend for reading
    penalize(&mut score, 15); // lend for writing
    assert_eq!(text, "score 90");
    assert_eq!(score, 75); // `main` still owns `score`
}
```

During the call to `penalize`, memory looks like this:

```text
stack frame of main            stack frame of penalize
+-------------+                +---------------------+
| score: 90   | <------------- | score: &mut i32     |
+-------------+    address     | points: 15          |
                               +---------------------+
```

`penalize` has only the address. The line `*score -= points` follows the address and changes the `90` that lives in `main`.

### Where you must write `*`

Rust adds the star for you in some places, and not in others.

- **Method calls and field access** with `.` dereference automatically, as many times as needed. If `r` is a `&String` or even a `&&String`, `r.len()` works.
- **Printing and formatting** show the referent, not the address.
- **Arithmetic and comparison** on numbers also accept references: `&a + &b` is fine for two integers.
- **Assignment** never adds a star. To write to the referent you must write `*r = value` or `*r += value`. Without the star, you would be assigning to the reference variable itself.

### Mistakes people really make

**Writing through a shared reference.** Change the parameter of `penalize` in the worked example to `score: &i32`, and the compiler says:

```text
error[E0594]: cannot assign to `*score`, which is behind a `&` reference
 --> src/main.rs:6:5
  |
6 |     *score -= points;
  |     ^^^^^^^^^^^^^^^^ `score` is a `&` reference, so it cannot be written to
  |
help: consider changing this to be a mutable reference
  |
5 | fn penalize(score: &mut i32, points: i32) {
  |                     +++
```

Fix: ask for `&mut i32`. Do this only when the function really has to change the value.

**Forgetting the star in an assignment.**

```rust,compile_fail,E0308
fn reset(score: &mut i32) {
    score = 0;
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:2:13
  |
1 | fn reset(score: &mut i32) {
  |                 -------- expected due to this parameter type
2 |     score = 0;
  |             ^ expected `&mut i32`, found integer
  |
help: consider dereferencing here to assign to the mutably borrowed value
  |
2 |     *score = 0;
  |     +
```

Fix: `*score = 0;`.

**Forgetting `mut` on the owner.** You can only take `&mut` of a variable that was declared with `let mut`. Write `let score = 90;` in the worked example, and you get:

```text
error[E0596]: cannot borrow `score` as mutable, as it is not declared as mutable
  --> src/main.rs:12:14
   |
12 |     penalize(&mut score, 15); // lend for writing
   |              ^^^^^^^^^^ cannot borrow as mutable
   |
help: consider changing this to be mutable
   |
10 |     let mut score = 90;
   |         +++
```

### When to use which

| The function needs to | Parameter type |
| --- | --- |
| read the value | `&T` |
| change the value in place | `&mut T` |
| keep the value, or use it up | `T` (a move) |

Start with `&T`. Move to `&mut T` or `T` only when the function needs more.

### Read more

- [The `reference` primitive type](https://doc.rust-lang.org/std/primitive.reference.html)
- [Reference: borrow operators](https://doc.rust-lang.org/reference/expressions/operator-expr.html#borrow-operators) and [the dereference operator](https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-dereference-operator)
- [Reference: method call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html) (how `.` finds a method through references)

## Lesson 02 · Reference bindings and identity

**Assumed:** lesson 01.

### The idea

A reference is an ordinary value. You can store it in a variable, copy it (`&T` is `Copy`), and assign a different reference to the same variable. So there are two separate questions, and two separate places for the word `mut`:

```text
let r = &x;           r always points at x;        x cannot be changed through r
let mut r = &x;       r may be pointed elsewhere;  x cannot be changed through r
let r = &mut x;       r always points at x;        x can be changed through r
let mut r = &mut x;   r may be pointed elsewhere;  x can be changed through r
```

`let mut r` is about the *variable* `r`. `&mut x` is about the *referent* `x`.

Because a reference is a value, you can also borrow *it*. A `&&T` is a reference to a reference. A `&mut &T` is exclusive access to somebody's reference variable: through it you can make that variable point at something else, without changing any `T`.

### Worked example: a cursor that moves forward

```rust
/// Makes the caller's `&str` start `n` bytes later.
fn advance(cursor: &mut &str, n: usize) {
    *cursor = &cursor[n..];
}

fn main() {
    let text = String::from("key=value");
    let mut cursor: &str = &text;
    advance(&mut cursor, 4);
    assert_eq!(cursor, "value");
    assert_eq!(text, "key=value"); // the text itself did not change
}
```

```text
text (String) ----> heap:  k e y = v a l u e
                           ^       ^
cursor before -------------+       |          (address,     length 9)
cursor after  ---------------------+          (address + 4, length 5)
```

`advance` received the address of the variable `cursor`. The assignment `*cursor = ...` stored a new `&str` in that variable. No byte of the text was written.

### Equal values or the same object?

`==` on references compares the *referents*. It looks through the references, so both sides need the same number of `&`.

`std::ptr::eq(a, b)` compares the *addresses*. It answers a different question: do `a` and `b` point at the very same object?

```rust
let a = vec!['o', 'k'];
let b = vec!['o', 'k'];
assert!(&a == &b); // equal contents
assert!(!std::ptr::eq(&a, &b)); // but two separate vectors
let again = &a;
assert!(std::ptr::eq(again, &a)); // the same vector
```

In everyday code you nearly always want `==`. Identity matters when you must know that no copy was made. The tests in this course use `ptr::eq` for exactly that: to check that your function returned a reference to the original element and not to a copy of it.

### Mistakes people really make

**Comparing a reference with a plain value.**

```rust,compile_fail,E0277
fn main() {
    let limit = 5;
    let r = &limit;
    if r == 5 {
        println!("at the limit");
    }
}
```

```text
error[E0277]: can't compare `&{integer}` with `{integer}`
 --> src/main.rs:4:10
  |
4 |     if r == 5 {
  |          ^^ no implementation for `&{integer} == {integer}`
  |
help: consider dereferencing here
  |
4 |     if *r == 5 {
  |        +
```

Fix: `*r == 5`, or `r == &5`.

**Thinking `let mut r = &x` allows writing to `x`.** It only allows pointing `r` somewhere else.

```rust,compile_fail,E0594
fn main() {
    let low = 3;
    let mut current = &low;
    *current = 4;
}
```

```text
error[E0594]: cannot assign to `*current`, which is behind a `&` reference
 --> src/main.rs:4:5
  |
4 |     *current = 4;
  |     ^^^^^^^^^^^^ `current` is a `&` reference, so it cannot be written to
```

Fix: decide what you want. To change the integer, borrow it with `&mut` (and declare the integer with `let mut`). To choose a different integer, assign a new reference: `current = &other;`.

**Testing identity through a buffer address.** `String::as_ptr()` returns the address of the heap buffer, not of the `String` itself. Two different empty strings own no buffer, and `as_ptr()` returns the same placeholder address for both. If the question is "is this the same `String` object?", compare the references themselves with `ptr::eq`.

### Read more

- [`std::ptr::eq`](https://doc.rust-lang.org/std/ptr/fn.eq.html)
- [`PartialEq`](https://doc.rust-lang.org/std/cmp/trait.PartialEq.html), the trait behind `==`
- [Reference: pointer types, shared and mutable references](https://doc.rust-lang.org/reference/types/pointer.html#references--and-mut)

## Lesson 03 · Optional references and fat pointers

**Assumed:** lessons 01 and 02; `Option` and slices from chapter 3.

### The idea

**A reference always points at a valid value.** There is no null reference. When "nothing" is a possible answer, the type says so: `Option<&T>`. The caller must then handle `None` before using the value.

This costs no memory. A reference can never be the all-zero address, so `Option<&T>` uses that unused bit pattern for `None`. It is the same size as `&T`.

**Some references carry a length.** A `&u8` is one machine word: an address. A slice reference `&[T]` and a string slice `&str` are two machine words: an address and a length. They are called *fat pointers*. A subslice is simply a new (address, length) pair that points into the same storage. Nothing is copied.

```text
data: [10, 20, 30, 40, 50]          five elements, starting at address A

&data[..]      = (A,      length 5)
&data[1..4]    = (A + one element,  length 3)      -> 20, 30, 40
```

### Worked example

```rust
/// The most recent entry, or None when the log is empty.
fn newest(log: &[String]) -> Option<&String> {
    log.last()
}

/// The slice without its first and last element.
fn without_ends(bytes: &[u8]) -> &[u8] {
    if bytes.len() < 2 {
        &[]
    } else {
        &bytes[1..bytes.len() - 1]
    }
}

fn main() {
    let log = vec![String::from("boot"), String::from("ready")];
    assert_eq!(newest(&log).map(String::as_str), Some("ready"));
    assert_eq!(newest(&[]), None);

    let data = [10, 20, 30, 40, 50];
    let inner = without_ends(&data);
    assert_eq!(inner, [20, 30, 40]);
    assert_eq!(inner.as_ptr(), data[1..].as_ptr()); // same storage, one element in

    assert_eq!(size_of::<&u8>(), size_of::<usize>());
    assert_eq!(size_of::<Option<&u8>>(), size_of::<&u8>());
    assert_eq!(size_of::<&[u8]>(), 2 * size_of::<usize>());
}
```

Both functions return references into data that the caller owns. Neither allocates.

### Borrowing a value that has no name

A *temporary* is a value created in the middle of an expression, without a variable of its own. Normally it is dropped at the end of the statement. There is one useful exception: if you borrow a temporary directly in a `let`, it lives as long as the variable.

```rust
let kept = &String::from("extended"); // the String lives until `kept` goes out of scope
assert_eq!(kept.len(), 8);
```

This is called *temporary lifetime extension*. It applies to `let r = &temporary;`, not to the result of a method call on a temporary, as the second mistake below shows.

### Mistakes people really make

**Indexing when the element may be missing.** `log[5]` compiles. It fails when the program runs:

```text
thread 'main' (121923941) panicked at src/main.rs:3:21:
index out of bounds: the len is 2 but the index is 5
```

(The number in parentheses identifies the thread. It is different on every run.)

Use indexing when a wrong index would be a bug in your program. Use a method that returns `Option` when a missing element is a normal case.

**Returning a reference to a temporary.**

```rust,compile_fail,E0515
fn greeting(name: &str) -> &str {
    &format!("hello, {name}")
}
```

```text
error[E0515]: cannot return reference to temporary value
 --> src/main.rs:2:5
  |
2 |     &format!("hello, {name}")
  |     ^------------------------
  |     ||
  |     |temporary value created here
  |     returns a reference to data owned by the current function
```

The new `String` is dropped when the function returns. Fix: return the `String` itself.

**Borrowing from a temporary that ends with the statement.**

```rust,compile_fail,E0716
fn main() {
    let shout: &str = String::from("quiet").to_uppercase().as_str();
    println!("{shout}");
}
```

```text
error[E0716]: temporary value dropped while borrowed
 --> src/main.rs:2:23
  |
2 |     let shout: &str = String::from("quiet").to_uppercase().as_str();
  |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^         - temporary value is freed at the end of this statement
  |                       |
  |                       creates a temporary value which is freed while still in use
3 |     println!("{shout}");
  |                ----- borrow later used here
```

Fix: give the owner a name first.

```rust
let upper = String::from("quiet").to_uppercase();
let shout: &str = upper.as_str();
assert_eq!(shout, "QUIET");
```

### When to use which

- Parameter types: prefer `&[T]` to `&Vec<T>` and `&str` to `&String`. The slice forms accept more callers, and a `&Vec<T>` or `&String` argument converts to them automatically.
- Return `Option<&T>` when the element may not exist. Return `&T` when it always exists.

### Read more

- [`std::option`: representation](https://doc.rust-lang.org/std/option/index.html#representation) (why `Option<&T>` is the size of `&T`)
- [The `slice` primitive type](https://doc.rust-lang.org/std/primitive.slice.html)
- [Reference: dynamically sized types](https://doc.rust-lang.org/reference/dynamically-sized-types.html) (why `&[T]` and `&str` carry a length)
- [Reference: temporary lifetime extension](https://doc.rust-lang.org/reference/destructors.html#temporary-lifetime-extension)

## Lesson 04 · Lifetimes through functions

**Assumed:** lessons 01 to 03.

### The idea

Every reference is used during some part of the program: from the point where it is created to the point where it is used for the last time. That part is its *lifetime*. The compiler works it out. One rule must hold: **the lifetime of a reference must end before its referent is dropped.** Otherwise the reference would dangle.

Inside one function, the compiler can see everything. Across a function call it cannot, because it checks each function separately and reads only the *signature* of the functions you call. So when a function returns a reference, its signature must say where that reference points.

This is what a *lifetime parameter* is for. It is a name, written with an apostrophe, such as `'a`:

```rust
fn first_sentence<'a>(text: &'a str) -> &'a str {
    match text.find('.') {
        Some(end) => &text[..=end],
        None => text,
    }
}

fn main() {
    let article = String::from("Rust has references. They are checked.");
    let lead = first_sentence(&article);
    assert_eq!(lead, "Rust has references.");
    assert_eq!(lead.as_ptr(), article.as_ptr()); // `lead` points into `article`
}
```

Read the signature like this: "Give me a `str` that is borrowed for some stretch of the program, called `'a`. I return a `str` that may be used during that same stretch." Two things follow.

- Inside `first_sentence`, the compiler checks that the returned reference really comes from `text`.
- In `main`, the compiler knows that `lead` borrows from `article`. While `lead` is in use, `article` may not be dropped, moved or changed.

```text
article (String: address, length 38, capacity) ---> heap: Rust has references. They are checked.
                                                          ^
lead (&str: address, length 20) --------------------------+
```

**A lifetime annotation describes; it does not command.** It never makes a value live longer, and it produces no code. If the description is false, the program is rejected.

When a function has only one reference parameter, you may leave the names out, and the compiler fills them in: `fn first_sentence(text: &str) -> &str` means the same. Lesson 06 gives the exact rules.

### Mistakes people really make

**Returning a reference to a local variable.**

```rust,compile_fail,E0515
fn lowercase_start(text: &str) -> &str {
    let copy = text.to_lowercase();
    &copy[..3]
}
```

```text
error[E0515]: cannot return value referencing local variable `copy`
 --> src/main.rs:3:5
  |
3 |     &copy[..3]
  |     ^----^^^^^
  |     ||
  |     |`copy` is borrowed here
  |     returns a value referencing data owned by the current function
```

`copy` is dropped when the function returns. No annotation can fix this, because the data really is gone. Fix: return an owned `String`.

**Using the result after the owner is gone.**

```rust,compile_fail,E0597
fn first_sentence(text: &str) -> &str {
    match text.find('.') {
        Some(end) => &text[..=end],
        None => text,
    }
}

fn main() {
    let lead;
    {
        let article = String::from("Rust has references. They are checked.");
        lead = first_sentence(&article);
    }
    println!("{lead}");
}
```

```text
error[E0597]: `article` does not live long enough
  --> src/main.rs:12:31
   |
11 |         let article = String::from("Rust has references. They are checked.");
   |             ------- binding `article` declared here
12 |         lead = first_sentence(&article);
   |                               ^^^^^^^^ borrowed value does not live long enough
13 |     }
   |     - `article` dropped here while still borrowed
14 |     println!("{lead}");
   |                ---- borrow later used here
```

Fix: move the owner to the scope where the result is used.

**Reaching for `'static`.** `'static` means "valid for the whole run of the program", as string literals are. If you write `fn first_sentence(text: &'static str) -> &'static str` in the worked example, the function still works for literals, but it now refuses ordinary borrowed data:

```text
error[E0597]: `article` does not live long enough
  --> src/main.rs:10:31
   |
 9 |     let article = String::from("Rust has references. They are checked.");
   |         ------- binding `article` declared here
10 |     let lead = first_sentence(&article);
   |                ---------------^^^^^^^^-
   |                |              |
   |                |              borrowed value does not live long enough
   |                argument requires that `article` is borrowed for `'static`
...
13 | }
   | - `article` dropped here while still borrowed
```

Fix: use an ordinary lifetime parameter. Ask for `'static` only when the data must truly live for the whole program.

### When to use which

- The result is **a part of an input** (a subslice, an element, a field): return a reference tied to that input. It is free.
- The result is **new data** (uppercased text, a sum, a joined string): return an owned value.

### Read more

- [Rustonomicon: lifetimes](https://doc.rust-lang.org/nomicon/lifetimes.html)
- [Error code E0515](https://doc.rust-lang.org/error_codes/E0515.html) and [E0597](https://doc.rust-lang.org/error_codes/E0597.html)
- [Reference: lifetime elision in functions](https://doc.rust-lang.org/reference/lifetime-elision.html#lifetime-elision-in-functions)

## Lesson 05 · Structs that borrow

**Assumed:** lesson 04. You have seen a `struct` with named fields in chapter 3; chapter 9 covers structs in depth.

### The idea

A struct may store references instead of owned data. Such a struct is a *view*: a small value that points into data owned by someone else. It is cheap to build and to move, because it copies only addresses and lengths.

The compiler must be able to tell how long a view may be used. So a struct with a reference field needs a lifetime parameter:

```rust
struct Words<'a> {
    rest: &'a str,
}
```

Read `Words<'a>` as "a `Words` that borrows text which stays valid for `'a`". A value of this type cannot be used after that text is gone. Moving or dropping the view does nothing to the text.

### Worked example: a word reader

```rust
struct Words<'a> {
    rest: &'a str,
}

impl<'a> Words<'a> {
    fn new(text: &'a str) -> Self {
        Words { rest: text.trim_start() }
    }

    /// Returns the next word. The result borrows the TEXT (`'a`), not `self`.
    fn next_word(&mut self) -> Option<&'a str> {
        if self.rest.is_empty() {
            return None;
        }
        let end = self.rest.find(' ').unwrap_or(self.rest.len());
        let word = &self.rest[..end];
        self.rest = self.rest[end..].trim_start();
        Some(word)
    }
}

fn main() {
    let line = String::from("borrow  check pass");
    let mut words = Words::new(&line);
    let first = words.next_word();
    let second = words.next_word();
    assert_eq!((first, second), (Some("borrow"), Some("check")));
    assert_eq!(size_of::<Words>(), size_of::<&str>()); // the view is one fat pointer
}
```

```text
line (String) ------> heap:  b o r r o w _ _ c h e c k _ p a s s
                             ^               ^
first = Some(&str) ----------+ (length 6)    |
words.rest (&str) ---------------------------+   after the first call
```

Look at the return type of `next_word`. It is `Option<&'a str>`, with the lifetime of the *text*. The word points into `line`, so it stays valid as long as `line` does. It does not depend on how long `words` is borrowed. This is why `first` can still be used after the second call to `words.next_word()`.

`impl<'a> Words<'a>` declares the name `'a` for the whole `impl` block, in the same way that `fn f<'a>` declares it for one function.

### Mistakes people really make

**Leaving out the lifetime parameter.**

```rust,compile_fail,E0106
struct Words {
    rest: &str,
}
```

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:2:11
  |
2 |     rest: &str,
  |           ^ expected named lifetime parameter
  |
help: consider introducing a named lifetime parameter
  |
1 ~ struct Words<'a> {
2 ~     rest: &'a str,
  |
```

**Tying the result to the wrapper instead of the source.** If you write the method as `fn next_word(&mut self) -> Option<&str>`, the compiler fills in the lifetime of `&mut self` for the result (lesson 06 explains this rule). Now each word keeps `words` exclusively borrowed, and the two calls in `main` collide:

```text
error[E0499]: cannot borrow `words` as mutable more than once at a time
  --> src/main.rs:26:18
   |
25 |     let first = words.next_word();
   |                 ----- first mutable borrow occurs here
26 |     let second = words.next_word();
   |                  ^^^^^ second mutable borrow occurs here
27 |     assert_eq!((first, second), (Some("borrow"), Some("check")));
   |                 ----- first borrow later used here
```

Fix: return `Option<&'a str>`. When a view hands out a piece of its source, give the piece the source's lifetime.

**Dropping or moving the source while a view is in use.**

```rust,compile_fail,E0505
struct Words<'a> {
    rest: &'a str,
}

fn main() {
    let line = String::from("borrow check");
    let words = Words { rest: &line };
    drop(line);
    println!("{}", words.rest);
}
```

```text
error[E0505]: cannot move out of `line` because it is borrowed
 --> src/main.rs:8:10
  |
6 |     let line = String::from("borrow check");
  |         ---- binding `line` declared here
7 |     let words = Words { rest: &line };
  |                               ----- borrow of `line` occurs here
8 |     drop(line);
  |          ^^^^ move out of `line` occurs here
9 |     println!("{}", words.rest);
  |                    ---------- borrow later used here
```

Fix: finish using the view first, or keep the owner alive longer.

### When to use which

| Use a borrowing struct (`&'a str` fields) when | Use an owning struct (`String` fields) when |
| --- | --- |
| it is used for a short time, near the data | it must be stored for a long time |
| the source stays alive and unchanged meanwhile | it is returned from the function that created the data |
| you want to avoid copying, for example while parsing | the source will be changed or dropped |

If you find yourself fighting lifetime errors in a struct that lives a long time, an owning struct is usually the right design.

### Read more

- [Reference: structs](https://doc.rust-lang.org/reference/items/structs.html)
- [Reference: generic parameters](https://doc.rust-lang.org/reference/items/generics.html) (lifetime parameters on items)
- [Error code E0106](https://doc.rust-lang.org/error_codes/E0106.html)

## Lesson 06 · Independent lifetimes and elision

**Assumed:** lessons 04 and 05.

### The idea, part 1: what the compiler fills in

You have already left lifetimes out of many signatures. This is called *elision*. The compiler follows three fixed rules for function signatures. It does not guess.

1. Each reference parameter without a lifetime gets its own, new lifetime.
2. If there is exactly one lifetime among the parameters, every reference in the result gets that lifetime.
3. If the function is a method with `&self` or `&mut self`, every reference in the result gets the lifetime of `self`.

If neither rule 2 nor rule 3 applies and the result contains a reference, you must write the lifetimes yourself.

```text
fn first(values: &[u8]) -> &u8                 rule 2: the result borrows `values`
fn name(&self, hint: &str) -> &str             rule 3: the result borrows `self`
fn pick(a: &str, b: &str) -> &str              no rule applies: error E0106
```

`'_` is a placeholder. It means "there is a lifetime here; use the rules". You will see it in types such as `Record<'_>`.

### The idea, part 2: one name or two?

A lifetime name used twice says "these are tied together". If two references share one name, the compiler must find a single stretch of the program that fits *both*, and that is the shorter one. Everything that carries the name is then limited to the shorter stretch.

When two references come from unrelated owners, give them different names. Then each is tracked on its own.

### Worked example

```rust
struct Lookup<'d, 'q> {
    dictionary: &'d [String],
    query: &'q str,
}

impl<'d, 'q> Lookup<'d, 'q> {
    /// The result points into the dictionary, so it carries `'d` only.
    fn find(&self) -> Option<&'d String> {
        self.dictionary.iter().find(|word| word.starts_with(self.query))
    }
}

fn main() {
    let dictionary = vec![String::from("borrow"), String::from("box")];
    let hit;
    {
        let query = String::from("bo");
        let lookup = Lookup { dictionary: &dictionary, query: &query };
        hit = lookup.find();
    } // `query` and `lookup` are gone here
    assert_eq!(hit.map(String::as_str), Some("borrow"));
}
```

`hit` is used after `query` and `lookup` are dropped. That is correct, because the hit points into `dictionary`, which is still alive. The type `Option<&'d String>` tells the compiler so.

### Mistakes people really make

**One lifetime for unrelated borrows.** Change the struct to `Lookup<'a>` with `dictionary: &'a [String]` and `query: &'a str`, and let `find` return `Option<&'a String>`. The same `main` is now rejected:

```text
error[E0597]: `query` does not live long enough
  --> src/main.rs:18:63
   |
17 |         let query = String::from("bo");
   |             ----- binding `query` declared here
18 |         let lookup = Lookup { dictionary: &dictionary, query: &query };
   |                                                               ^^^^^^ borrowed value does not live long enough
19 |         hit = lookup.find();
20 |     } // `query` and `lookup` are gone here
   |     - `query` dropped here while still borrowed
21 |     assert_eq!(hit.map(String::as_str), Some("borrow"));
   |                --- borrow later used here
```

The message names `query`, although the hit never points at it. The single `'a` tied the result to both fields. Fix: two lifetime parameters.

**Annotating one input, then returning the other.**

```rust,compile_fail,E0621
fn pick<'a>(preferred: &'a str, fallback: &str) -> &'a str {
    if preferred.is_empty() { fallback } else { preferred }
}
```

```text
error[E0621]: explicit lifetime required in the type of `fallback`
 --> src/main.rs:2:31
  |
2 |     if preferred.is_empty() { fallback } else { preferred }
  |                               ^^^^^^^^ lifetime `'a` required
  |
help: add explicit lifetime `'a` to the type of `fallback`
  |
1 | fn pick<'a>(preferred: &'a str, fallback: &'a str) -> &'a str {
  |                                            ++
```

The signature promised that the result comes from `preferred`, but the body can return `fallback`. Here the result really can be either input, so sharing one name is correct: `fn pick<'a>(preferred: &'a str, fallback: &'a str) -> &'a str`.

**Leaving both inputs without a name.** With two reference parameters and no `self`, no elision rule applies, and you get E0106 with a suggestion. Drill d07 lets you meet this one yourself.

### When to use which

- The result may come from **either** input: one shared lifetime.
- The result comes from **one** input: name that input's lifetime, and leave the other input alone.
- A struct's fields borrow from **unrelated owners**, and you hand one of them back out: one lifetime parameter per owner.
- Do not add lifetime names that connect nothing. Clippy's `needless_lifetimes` warning points them out.

Lifetimes exist only while the compiler checks your program. They are erased afterwards and cost nothing at run time.

This chapter stops here. Rules that relate lifetimes to each other (`'a: 'b`, `T: 'a`), variance, and bounds of the form `for<'a>` belong to the mastery track, in the module [`mastery/m1_types`](../../mastery/m1_types/README.md).

### Read more

- [Reference: lifetime elision](https://doc.rust-lang.org/reference/lifetime-elision.html)
- [Rustonomicon: lifetime elision](https://doc.rust-lang.org/nomicon/lifetime-elision.html)
- [Error code E0106](https://doc.rust-lang.org/error_codes/E0106.html)

## Lesson 07 · Sharing versus mutation

**Assumed:** lessons 01 to 06.

### The idea

This is the central rule of the chapter:

> At any point in the program, a value may be reached through any number of shared references, **or** through exactly one exclusive reference. Never both.

Two more facts make the rule practical.

- A borrow lasts from the place where the reference is created to the place where it is **used for the last time**. It does not last to the closing brace. After the last use, the value is free again.
- The rule is checked for every possible run of the program. The compiler does not reason about values that are only known at run time, such as the current capacity of a vector.

### Why the rule exists

Changing a value can destroy what other references point at. A vector is the classic case. When it is full, `push` allocates a larger buffer, moves the elements, and frees the old buffer:

```text
before push (capacity 2, full)          after push (new buffer, capacity 4)

words --> [ "a" | "bb" ]                words --> [ "a" | "bb" | "bb" |      ]
                  ^                               old buffer: freed
                  word                              ^
                                                    word would still point here
```

A reference into the old buffer would dangle. Rust refuses the program at compile time instead.

### Worked example

```rust
fn append_total(values: &mut Vec<u32>) {
    let total: u32 = values.iter().sum(); // shared borrow: starts and ends on this line
    values.push(total); // exclusive borrow: nothing else is in use
}

fn add_one(n: &mut i32) {
    *n += 1;
}

fn main() {
    let mut values = vec![2, 3];
    append_total(&mut values);
    assert_eq!(values, [2, 3, 5]);

    let mut n = 0;
    let r = &mut n;
    add_one(r); // lends `r` onward for the length of this call
    add_one(r); // so `r` can be used again
    *r += 10;
    assert_eq!(n, 12);
}
```

In `append_total`, the read is finished before the write begins. `total` is a plain number: it borrows nothing.

The second half shows a *reborrow*. A `&mut` reference is not `Copy`, yet `r` can be passed twice. Each call receives `&mut *r`, a new exclusive reference made from `r`. While the reborrow is in use, `r` is paused. When the call returns, `r` works again.

### Mistakes people really make

**Changing a collection while looping over it.**

```rust,compile_fail,E0502
fn main() {
    let mut words = vec![String::from("a"), String::from("bb")];
    for word in &words {
        if word.len() > 1 {
            words.push(word.clone());
        }
    }
}
```

```text
error[E0502]: cannot borrow `words` as mutable because it is also borrowed as immutable
 --> src/main.rs:5:13
  |
3 |       for word in &words {
  |       -           ------
  |       |           |
  |       |           immutable borrow occurs here
  |  _____|           immutable borrow later used here
  | |
4 | |         if word.len() > 1 {
5 | |             words.push(word.clone());
  | |             ^^^^^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs here
6 | |         }
7 | |     }
  | |_____- this for loop borrows `words` immutably, preventing mutation within its body
  |
  = help: consider using an index-based loop instead, or collecting modifications into a separate collection
```

The loop holds a shared borrow of `words` until it ends. Fix: first collect what to add, then add it.

```rust
let mut words = vec![String::from("a"), String::from("bb")];
let long: Vec<String> = words.iter().filter(|w| w.len() > 1).cloned().collect();
words.extend(long);
assert_eq!(words, ["a", "bb", "bb"]);
```

**Two exclusive references in use at once.**

```rust,compile_fail,E0499
fn main() {
    let mut name = String::from("Ada");
    let first = &mut name;
    let second = &mut name;
    first.push('!');
    second.push('?');
}
```

```text
error[E0499]: cannot borrow `name` as mutable more than once at a time
 --> src/main.rs:4:18
  |
3 |     let first = &mut name;
  |                 --------- first mutable borrow occurs here
4 |     let second = &mut name;
  |                  ^^^^^^^^^ second mutable borrow occurs here
5 |     first.push('!');
  |     ----- first borrow later used here
```

Fix: finish with `first` before you create `second`. If you move `first.push('!')` up one line, the program compiles, because the first borrow has ended.

**Cloning until the error goes away.** A clone of a whole collection does silence the borrow checker, but it costs an allocation and a copy, and the code then works on a snapshot that may be out of date. The tests in this lesson count allocations to catch this. Look for the smaller fix first.

### How to resolve a conflict

Try these in order.

1. **Reorder.** Finish reading before you start writing.
2. **Take out only what you need.** Copy a number, or clone one element, so that no reference stays behind.
3. **Remember a position, not a reference.** A length or an index is a plain `usize`. It borrows nothing.
4. **Collect, then apply.** Gather the changes in a new collection and apply them after the loop.

One surprise is worth knowing now: a call such as `scores.truncate(scores.len() / 2)` compiles, although the vector is read inside a call that changes it. The section *Drill topics* at the end of this file explains why.

### Read more

- [Rustonomicon: references](https://doc.rust-lang.org/nomicon/references.html) and [aliasing](https://doc.rust-lang.org/nomicon/aliasing.html)
- [`Vec`: guarantees](https://doc.rust-lang.org/std/vec/struct.Vec.html#guarantees) (when a vector reallocates)
- [Error code E0502](https://doc.rust-lang.org/error_codes/E0502.html) and [E0499](https://doc.rust-lang.org/error_codes/E0499.html)

## Lesson 08 · Disjoint mutable borrows

**Assumed:** lesson 07.

### The idea

Two exclusive references are harmless when they point at *different* memory. The question is whether the compiler can see that they do.

The borrow checker follows *places*: expressions that name a location, such as a variable `game`, a field `game.score`, or an element `values[i]`. It treats them differently.

- **Different fields of one struct or tuple are different places.** You may borrow `game.score` and `game.log` mutably at the same time.
- **Elements are not tracked one by one.** The checker never compares index values, not even the constants `0` and `1`. For an array or slice, every `values[i]` counts as the same place. For a `Vec`, `v[i]` is a method call that borrows the whole vector.

So for elements you need help from the standard library. It offers methods that take one `&mut` slice and return several `&mut` references which never overlap. The library guarantees this itself: it checks the positions at run time and uses a small amount of unsafe code inside. The types alone do not prove it. Because the guarantee always holds, the method is safe to call, and you write no `unsafe`.

### Worked example

```rust
fn add_first_to_rest(values: &mut [i32]) {
    if let Some((first, rest)) = values.split_first_mut() {
        for value in rest {
            *value += *first;
        }
    }
}

struct Game {
    score: u32,
    log: Vec<String>,
}

fn main() {
    let mut values = [5, 1, 2, 3];
    add_first_to_rest(&mut values);
    assert_eq!(values, [5, 6, 7, 8]);

    let mut game = Game { score: 0, log: Vec::new() };
    let score = &mut game.score; // borrows only the field `score`
    let log = &mut game.log; // borrows only the field `log`
    *score += 10;
    log.push(format!("score is now {score}"));
    assert_eq!(game.log, ["score is now 10"]);
}
```

```text
values:  [ 5 | 1 | 2 | 3 ]
           ^   ^---------^
         first     rest            two exclusive references, no element in both
```

`split_first_mut` returns `None` for an empty slice, so there is no panic to worry about.

### Mistakes people really make

**Two indexed borrows.**

```rust,compile_fail,E0499
fn bump_both(values: &mut [i32]) {
    let a = &mut values[0];
    let b = &mut values[1];
    *a += 1;
    *b += 1;
}
```

```text
error[E0499]: cannot borrow `values[_]` as mutable more than once at a time
 --> src/main.rs:3:13
  |
2 |     let a = &mut values[0];
  |             -------------- first mutable borrow occurs here
3 |     let b = &mut values[1];
  |             ^^^^^^^^^^^^^^ second mutable borrow occurs here
4 |     *a += 1;
  |     ------- first borrow later used here
  |
  = help: use `.split_at_mut(position)` to obtain two mutable non-overlapping sub-slices
```

Note the `values[_]` in the message: the index is not part of the place. Fix: finish with `a` before creating `b`, or use one of the library methods in the table below.

**Calling a method on the whole struct while one field is borrowed.** A method with `&self` borrows *all* of `self`, even if its body reads a single field.

```rust,compile_fail,E0502
struct Game {
    score: u32,
    log: Vec<String>,
}

impl Game {
    fn bonus(&self) -> u32 {
        self.score / 10
    }
}

fn main() {
    let mut game = Game { score: 50, log: Vec::new() };
    let log = &mut game.log;
    let bonus = game.bonus();
    log.push(format!("bonus {bonus}"));
}
```

```text
error[E0502]: cannot borrow `game` as immutable because it is also borrowed as mutable
  --> src/main.rs:15:17
   |
14 |     let log = &mut game.log;
   |               ------------- mutable borrow occurs here
15 |     let bonus = game.bonus();
   |                 ^^^^ immutable borrow occurs here
16 |     log.push(format!("bonus {bonus}"));
   |     --- mutable borrow later used here
```

Fix: call `game.bonus()` before you borrow the field.

**A split point past the end.** `split_at_mut(mid)` needs `mid <= len`. This is checked when the program runs, and a wrong value panics with the message `mid > len`. Check the position first, or use a method that returns `Option` or `Result`.

### When to use which

| You need | Use |
| --- | --- |
| to exchange two elements | [`swap(i, j)`](https://doc.rust-lang.org/std/primitive.slice.html#method.swap) |
| everything before a position, and everything from it on | [`split_at_mut(mid)`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_at_mut) |
| the first element and all the others | [`split_first_mut()`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_first_mut) |
| a few elements at known positions | [`get_disjoint_mut([i, j])`](https://doc.rust-lang.org/std/primitive.slice.html#method.get_disjoint_mut) |
| groups of a fixed size | [`chunks_mut(n)`](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks_mut) |
| every element, one after the other | `iter_mut()`, or `for value in &mut values` |
| two fields of a struct | borrow the fields directly |

In the exercise you build a two-element borrow from `split_at_mut` by hand, to see how it works. In your own code, `get_disjoint_mut` does that job in one call. It checks its indices at run time and returns a `Result`.

### Read more

- [Rustonomicon: splitting borrows](https://doc.rust-lang.org/nomicon/borrow-splitting.html)
- [Reference: place expressions](https://doc.rust-lang.org/reference/expressions.html#place-expressions-and-value-expressions)
- [`IndexMut`](https://doc.rust-lang.org/std/ops/trait.IndexMut.html), the trait behind `v[i] = ...` on a `Vec`

## Lesson 09 · Capstone: borrowed roster queries

**Assumed:** the whole chapter.

The capstone has two small functions, but the real subject is a design question that returns in every Rust program: **how do you refer to one element of a collection that may change later?** There are three basic answers.

| You hold | Cost | What can go wrong |
| --- | --- | --- |
| a reference, `&T` | nothing | The collection stays borrowed. The compiler stops every change while you hold it. |
| an index, `usize` | nothing | Nothing is borrowed, so changes are allowed. After a removal, insertion or sort, the index may point at a different element, or past the end. Nobody warns you. |
| a clone, `T` | one copy | You have your own data. It does not see later changes. |

A reference is checked by the compiler. An index is checked by nobody:

```rust
let mut tracks = vec!["intro", "verse", "outro"];
let current = 2; // an index is a plain number; it borrows nothing
assert_eq!(tracks[current], "outro");

tracks.remove(0); // allowed, because nothing borrows `tracks`
assert_eq!(tracks.get(current), None); // the index is stale now
assert_eq!(tracks.iter().position(|t| *t == "outro"), Some(1));
```

With `let current = &tracks[2];` the same `remove` is a compile error (E0502) for as long as `current` is used afterwards. That is safer, but it also means that a selection stored as a reference would block every edit of the list.

### How to approach the capstone

- Task A returns an **index**. It reads the labels through a shared borrow and returns a plain number, so the caller is free to change the roster afterwards. Think about how to keep the *first* of several equally long labels, and remember from chapter 3 that the number of bytes in a string is not the number of Unicode scalar values.
- Task B returns a **borrow**. Its result must point into the roster's own `String`. The tests compare addresses and count allocations, so a copy of the label does not pass. Lessons 03 and 04 have the tools you need.
- After the tests pass, do the lab in the lesson header. Storing a reference to an element next to the vector that owns it does not work in safe Rust. Find out which error you get, then replace the reference with an index, and write down which operations make that index wrong.

A common fourth answer is a **stable identifier**: each entry stores its own ID, and you look entries up by ID. It survives removals and sorting. It costs a search, or a map from ID to position.

### Read more

- [`slice::get`](https://doc.rust-lang.org/std/primitive.slice.html#method.get)
- [Error code E0502](https://doc.rust-lang.org/error_codes/E0502.html)

## Drill topics · Three cases the borrow checker treats specially

**Assumed:** lessons 07 and 08. Read this before drills d11, d12 and d13.

### 1. Two-phase borrows: reading a value inside a call that changes it

By the rule of lesson 07 the call below looks wrong. `truncate` needs `&mut scores`, and the argument `scores.len() / 2` needs `&scores` at the same time.

```rust
let mut scores = vec![9, 7, 5, 3];
scores.truncate(scores.len() / 2);
assert_eq!(scores, [9, 7]);
```

It compiles because the exclusive borrow that a method call takes for its receiver starts in two steps. First it is only *reserved*: other code may still read the value. It becomes fully exclusive when the method actually starts to run, and by then the arguments are already computed. This is called a *two-phase borrow*.

It applies to the borrow that the compiler creates for you in `receiver.method(args)`. It does not apply to an exclusive reference you made yourself and still use:

```rust,compile_fail,E0502
fn main() {
    let mut scores = vec![9, 7, 5, 3];
    let handle = &mut scores;
    handle.truncate(scores.len() / 2);
}
```

```text
error[E0502]: cannot borrow `scores` as immutable because it is also borrowed as mutable
 --> src/main.rs:4:21
  |
3 |     let handle = &mut scores;
  |                  ----------- mutable borrow occurs here
4 |     handle.truncate(scores.len() / 2);
  |            -------- ^^^^^^ immutable borrow occurs here
  |            |
  |            mutable borrow later used by call
```

Fix: read through the same reference, `handle.truncate(handle.len() / 2)`, or compute the length on an earlier line.

### 2. Passing `&mut`: reborrow or move?

`&mut T` is not `Copy`. If it were, you could make two exclusive references. So using a `&mut` by value *moves* it, like a `String`.

Yet you can pass the same `&mut` to a function many times (lesson 07). The reason: when the compiler already knows that the target type is a `&mut` reference, it does not move your reference. It inserts a reborrow, `&mut *r`. When the target type is not known yet, because the compiler must infer it, the reference is moved.

```rust
let mut n = 1;
let r = &mut n;
let again: &mut i32 = r; // the type is written out, so this is a reborrow
*again += 1;
*r += 1; // `r` works again after the last use of `again`
assert_eq!(n, 3);
```

Remove the type annotation, and the same line becomes a move:

```rust,compile_fail,E0382
fn main() {
    let mut n = 1;
    let r = &mut n;
    let again = r;
    *again += 1;
    *r += 1;
}
```

```text
error[E0382]: use of moved value: `r`
 --> src/main.rs:6:5
  |
3 |     let r = &mut n;
  |         - move occurs because `r` has type `&mut i32`, which does not implement the `Copy` trait
4 |     let again = r;
  |                 - value moved here
5 |     *again += 1;
6 |     *r += 1;
  |     ^^^^^^^ value used here after move
```

The same happens when you pass a `&mut` to a function whose parameter is a generic `T`. Fix: write the reborrow yourself, `&mut *r`, or `&*r` when reading is enough.

### 3. A borrow that is returned on one path only

This function looks up a counter, and creates it when it is missing:

```rust,compile_fail,E0499
use std::collections::HashMap;

fn count_for<'a>(counts: &'a mut HashMap<String, u32>, word: &str) -> &'a mut u32 {
    if let Some(count) = counts.get_mut(word) {
        return count;
    }
    counts.insert(word.to_string(), 0);
    counts.get_mut(word).unwrap()
}
```

```text
error[E0499]: cannot borrow `*counts` as mutable more than once at a time
 --> src/main.rs:7:5
  |
3 | fn count_for<'a>(counts: &'a mut HashMap<String, u32>, word: &str) -> &'a mut u32 {
  |              -- lifetime `'a` defined here
4 |     if let Some(count) = counts.get_mut(word) {
  |                          ------ first mutable borrow occurs here
5 |         return count;
  |                ----- returning this value requires that `*counts` is borrowed for `'a`
6 |     }
7 |     counts.insert(word.to_string(), 0);
  |     ^^^^^^ second mutable borrow occurs here
```

No reference can really dangle here. When the search finds nothing, its borrow is not used again. But the search result *may* be returned, and a returned reference must be valid for all of `'a`, which reaches past the end of the function. The borrow checker in the pinned compiler then treats `counts` as borrowed on *every* path after the search, including the path where nothing was found. This is a known limit of the current analysis. A newer design for the borrow checker, called Polonius, accepts this function; on a nightly toolchain you can try it with the flag `-Zpolonius=next`.

Until then, restructure so that no borrow crosses the decision.

- **For a map, use the entry API.** It does the search and the insertion inside one borrow:

  ```rust
  use std::collections::HashMap;

  fn count_for<'a>(counts: &'a mut HashMap<String, u32>, word: &str) -> &'a mut u32 {
      counts.entry(word.to_string()).or_insert(0)
  }

  let mut counts = HashMap::new();
  *count_for(&mut counts, "borrow") += 1;
  *count_for(&mut counts, "borrow") += 1;
  assert_eq!(counts["borrow"], 2);
  ```

- **For a vector or slice, search for an index first.** An index borrows nothing (lesson 07), so the search is over before you decide. Drill d13 asks you to do this.

### Read more

- [Reference: type coercions](https://doc.rust-lang.org/reference/type-coercions.html#coercion-sites) (the places where the expected type is known)
- [`HashMap::entry`](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.entry)
- [Error code E0382](https://doc.rust-lang.org/error_codes/E0382.html) and [E0499](https://doc.rust-lang.org/error_codes/E0499.html)
