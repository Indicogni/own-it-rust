# Mastery M1 concepts — The type system in depth

[Module guide](README.md) · [Hints](HINTS.md) · [Notes](NOTES.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every code block in this file was compiled and
run with the course toolchain (Rust 1.99.0, edition 2024), and every compiler
message was copied from that compiler. Long messages are shortened: a line with
`...` marks a cut in the middle, and the end of a message is often left out.

## What you will be able to do

After this module you can:

- say, for any generic type, whether a longer lifetime may be used where a
  shorter one is expected, and explain the compiler error when it may not;
- write and read bounds such as `T: 'a`, `'a: 'b` and
  `for<'a> Fn(&'a str) -> &'a str`;
- define a trait with a generic associated type, and build an iterator whose
  items borrow from the iterator itself;
- choose the right `PhantomData` marker for a type that holds raw pointers or
  an unused parameter, and argue why your `unsafe impl Send` is sound;
- explain what a trait object is at run time, make a trait usable as one,
  convert between trait objects, and find the lifetime the compiler filled in;
- use types without values, and types with one value, to make a wrong program
  fail to compile at no run-time cost;
- design an API in which each state of a value is a type, and no other crate
  can add a state;
- decide which trait impls you may write, which impls you take away from your
  users when you write a blanket impl, and why.

## Before you start

Chapters 3 to 16 of this course, and in particular:
references and lifetimes in signatures (Chapter 5), structs with lifetime
parameters (Chapter 9), traits, generics and trait objects (Chapters 11 and
13), closures and the `Fn` traits (Chapter 14) and iterators (Chapter 15).
Lesson 04 contains a small amount of `unsafe` code; Chapter 23 explains raw
pointers. You should be able to read a compiler error calmly and find its
first line, its code and its `note:` lines.

## Time

These are estimates for a learner who has finished the chapters
above. They include reading the section, the predictions, the code, and your
notes.

| Lesson | Estimate |
| --- | --- |
| 01 · Subtyping and variance | 90 minutes |
| 02 · Lifetime bounds and higher-ranked trait bounds | 100 minutes |
| 03 · Generic associated types and a lending iterator | 100 minutes |
| 04 · PhantomData | 120 minutes |
| 05 · Trait objects | 120 minutes |
| 06 · Zero-sized and uninhabited types | 75 minutes |
| 07 · Sealed traits, typestate and const generics | 120 minutes |
| 08 · Coherence and the orphan rule | 120 minutes |
| 09 · Capstone | 180 minutes |
| C01 code review, drills | 150 minutes |

## How the checks work in this module

Many rules here are about programs
that must *not* compile. A normal test cannot check that, because a test that
does not compile stops the whole test run. This module therefore has three
kinds of checks:

- unit tests inside each lesson file, as in the chapters;
- **cases**: small programs in `tests/cases/`. The test file
  `tests/contracts.rs` compiles each one against your library and compares the
  outcome with the first line of the case: `//@ expect: run`, or an error code
  such as `//@ expect: E0599`. A case is written the way a *user* of your code
  would write it, in another crate;
- the `compile_fail` code blocks in this file. `cargo test --doc` checks that
  each of them is rejected. On this toolchain it does not compare the error
  code that is written next to `compile_fail`; the codes here were checked
  separately when the text was written.

## Words used in this module

- **Outlives**: `'a: 'b` is read "`'a` outlives `'b`". It means that `'a`
  lasts at least as long as `'b`. Every lifetime outlives itself, and
  `'static` outlives every lifetime.
- **Subtype**: type `S` is a subtype of type `T` when a value of type `S` may
  be used wherever a value of type `T` is expected, with no conversion.
- **Marker type**: a type that is used only for what it says to the compiler.
  It holds no data.
- **Zero-sized type (ZST)**: a type whose values need no memory.
- **Uninhabited type**: a type that has no values at all.
- **Trait object**: a value of a type `dyn Trait`. Its concrete type is known
  only at run time.
- **Vtable**: a table of function pointers that the compiler builds for one
  concrete type and one trait. A trait object carries a pointer to it.
- **Coherence**: the rule that a trait is implemented at most once for a type,
  in the whole program.

## Lesson 01 · Subtyping and variance

### The idea

This section assumes that you can read lifetimes in function signatures and
in struct definitions (Chapters 5 and 9).

A reference that is valid for a long time can be used where a reference that
is valid for a short time is expected. You have relied on this since Chapter
5, every time you passed a string literal to a function that takes `&str`:

```rust
fn longer<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

let fixed: &'static str = "fixed text";
let local = String::from("local");
// `fixed` is a `&'static str`. Here it is used as a `&'a str`, where 'a is
// no longer than the life of `local`.
assert_eq!(longer(fixed, &local), "fixed text");
```

The precise name for this is **subtyping**. If `'long: 'short`, then
`&'long T` is a subtype of `&'short T`. Rust has subtyping only through
lifetimes. There is no subtyping between `u8` and `u16`, and none between a
struct and another struct. (Conversions such as `&String` to `&str`, or
`&Circle` to `&dyn Shape`, are *coercions*: the compiler inserts a step
there. Subtyping inserts nothing.) The conversion costs nothing at run time: the
pointer is the same, and lifetimes do not exist in the compiled program.

Now take a generic type such as `Vec<T>`, `Cell<T>` or `&'a mut T`, and put a
reference type inside it. Is a `Vec<&'static str>` a subtype of a
`Vec<&'a str>`? The answer depends on the outer type. The property that
decides it is called **variance**. A type `F<T>` is:

- **covariant** in `T` if a subtype inside gives a subtype outside:
  `F<&'static str>` may be used as `F<&'a str>`;
- **contravariant** in `T` if the direction turns around: `F<&'a str>` may be
  used as `F<&'static str>`;
- **invariant** in `T` if neither is allowed: the inner type must match
  exactly.

The same three words apply to a lifetime parameter: `F<'a>` is covariant in
`'a` if `F<'long>` may be used as `F<'short>`.

| Type | In `'a` | In `T` |
| --- | --- | --- |
| `&'a T` | covariant | covariant |
| `&'a mut T` | covariant | **invariant** |
| `Box<T>`, `Vec<T>`, `Option<T>`, `[T]`, `(T, U)` | | covariant |
| `Cell<T>`, `RefCell<T>`, `Mutex<T>`, `UnsafeCell<T>` | | **invariant** |
| `*const T` | | covariant |
| `*mut T` | | **invariant** |
| `fn(T) -> U` | | **contravariant** in `T`, covariant in `U` |
| `dyn Trait<T> + 'a` | covariant | **invariant** |
| a field of type `<T as Trait>::Assoc`, such as `I::Item` | | **invariant** |

The rule behind the table is short. A place from which you can only *read* a
`T` may be covariant. A place into which you can *write* a `T` must not be:
if it is also readable it is invariant, and if it is only writable (the
argument of a function) it is contravariant.
The last two rows have another reason: the compiler cannot see what a
trait does with its parameter, or which type an associated type turns out
to be, so it allows no change at all.

### Why writing forces invariance

Here is the program that would be accepted if `&mut T` were covariant in `T`.

```rust,compile_fail,E0597
fn assign<'a>(slot: &mut &'a str, value: &'a str) {
    *slot = value;
}

fn main() {
    let mut motto: &'static str = "never dangle";
    {
        let local = String::from("short-lived");
        assign(&mut motto, &local);
    }
    println!("{motto}");
}
```

`assign` is a correct function: it stores a `&'a str` in a slot for
`&'a str`. The call is the problem. `&mut motto` is a `&mut &'static str`. To
make the call fit, `'a` would have to be the life of `local`, and the
`&mut &'static str` would have to become a `&mut &'a str`. That is covariance
in `T`, and this is what it would do:

```text
before the call      motto ───────────────► "never dangle"   (lives forever)

inside `assign`      slot ──► motto         the slot is typed `&'a str` here,
                                            so a short-lived value fits

after the block      motto ───────────────► the buffer of `local`, now freed
                     (still typed &'static str in `main`)
```

`main` would print freed memory. Because `&mut T` is invariant in `T`, the
compiler has only one choice left: `'a` must be `'static`, and then `local`
does not live long enough.

```text
error[E0597]: `local` does not live long enough
   |
 6 |     let mut motto: &'static str = "never dangle";
   |                    ------------ type annotation requires that `local` is borrowed for `'static`
 7 |     {
 8 |         let local = String::from("short-lived");
   |             ----- binding `local` declared here
 9 |         assign(&mut motto, &local);
   |                            ^^^^^^ borrowed value does not live long enough
10 |     }
   |     - `local` dropped here while still borrowed
```

Read this error as: "something invariant fixed a lifetime, and the fixed
lifetime is too long for this borrow". The error does not say "variance".
You have to see it. `Cell<T>` and `RefCell<T>` are invariant for the same
reason: `set` writes through a shared reference.

Note the difference between the container and a `&mut` to it. A
`Vec<&'static str>` that you *own* may be moved into a `Vec<&'a str>`
variable: after the move, nobody still believes that the elements are
`'static`. A `&mut Vec<&'static str>` may not be used as a
`&mut Vec<&'a str>`: the owner still believes it.

### Functions turn the direction around

A function pointer is the one common type that is contravariant.

```rust
fn count(text: &str) -> usize {
    text.len()
}

// For ONE lifetime 'a: a function that accepts `&'a str` is used as a
// function that accepts only `&'static str`. The lifetime in the argument
// grows from 'a to 'static. That is contravariance.
fn for_static_text<'a>(f: fn(&'a str) -> usize) -> fn(&'static str) -> usize {
    f
}

let for_literals = for_static_text(count);
assert_eq!(for_literals("abc"), 3);
```

A function that accepts *more* inputs can stand in for one that must accept
*fewer*: every `&'static str` is also a `&'a str`, so `f` can take whatever
the new type lets a caller pass. The other direction is refused. A function
that needs `&'static str` cannot stand in for one that is given short-lived
text. For the return type the direction is the usual one: a function that
returns a `&'static str` can stand in for one that returns a `&'a str`.

One more conversion looks the same and has a different reason.
`let f: fn(&'static str) -> usize = count;` also compiles. `count` was
declared with an elided lifetime, so it accepts *every* lifetime, and the
`let` picks one of them. Lesson 02 gives that kind of type its name:
`for<'a> fn(&'a str) -> usize`. Besides lifetimes inside a type, this
"every lifetime, so also this one" step is the only other subtyping in Rust.

### The variance of your own types

You never write variance down. The compiler computes it from the fields.
A parameter is covariant if every use of it is in a covariant position, and
contravariant if every use is contravariant. One invariant use, or a mix of
the two directions, makes it invariant.

```rust
use std::cell::Cell;

struct Page<'a> {
    title: &'a str,          // covariant use of 'a
    words: Vec<&'a str>,     // covariant use of 'a
}

struct Tracker<'a> {
    title: &'a str,          // covariant use of 'a ...
    last_seen: Cell<&'a str> // ... and an invariant one: Tracker is invariant
}

fn shorten<'long: 'short, 'short>(page: Page<'long>) -> Page<'short> {
    page // accepted: Page is covariant
}
```

The same function for `Tracker` is rejected, and the message names the field
type that is responsible:

```text
error: lifetime may not live long enough
  |
8 | fn shorten<'long: 'short, 'short>(tracker: Tracker<'long>) -> Tracker<'short> {
  |            -----          ------ lifetime `'short` defined here
  |            |
  |            lifetime `'long` defined here
9 |     tracker
  |     ^^^^^^^ function was supposed to return data with lifetime `'long` but it is returning data with lifetime `'short`
  |
  = help: consider adding the following bound: `'short: 'long`
  = note: requirement occurs because of the type `Tracker<'_>`, which makes the generic argument `'_` invariant
  = note: the struct `Tracker<'a>` is invariant over the parameter `'a`
```

This error has no error code. Its first sentence is confusing, because it
talks about what the function "was supposed to return". The two `note:`
lines are the useful part: they name the type and the parameter that are
invariant. To find out *why* `Tracker` is invariant, look through its fields
for a `Cell`, a `&mut`, or another invariant type.

### Mistakes people really make

**One lifetime for a `&mut` and for what is behind it.** This is the most
common variance bug in real code.

```rust,compile_fail,E0502
struct Editor<'a> {
    lines: &'a mut Vec<&'a str>,
}

fn main() {
    let mut lines = vec!["one"];
    let editor = Editor { lines: &mut lines };
    editor.lines.push("two");
    println!("{}", lines.len());
}
```

```text
error[E0502]: cannot borrow `lines` as immutable because it is also borrowed as mutable
  |
7 |     let editor = Editor { lines: &mut lines };
  |                                  ---------- mutable borrow occurs here
8 |     editor.lines.push("two");
9 |     println!("{}", lines.len());
  |                    ^^^^^
  |                    |
  |                    immutable borrow occurs here
  |                    mutable borrow later used here
```

The editor is not used after line 8, so why is the borrow "later used"? The
type `&'a mut Vec<&'a str>` uses one lifetime twice. `&mut T` is invariant in
`T`, so the `'a` inside `Vec<&'a str>` cannot shrink. The borrow `&'a mut`
must then last as long as the vector's own element lifetime, which is as long
as the vector is used at all. The vector is locked for the rest of its life.
The fix is two lifetimes, so that the borrow can be short:

```rust
struct Editor<'borrow, 'text> {
    lines: &'borrow mut Vec<&'text str>,
}

let mut lines = vec!["one"];
let editor = Editor { lines: &mut lines };
editor.lines.push("two");
assert_eq!(lines.len(), 2);
```

**Annotating a container as `'static` and then lending it mutably.** The
program at the start of this section is the small version. In real code the
`'static` often comes from a `const`, a `static` or a type alias. The fix is
to give the container the shorter element lifetime from the start, or to
store owned values.

**Expecting `Cell<&'long T>` to shrink.** It cannot. Build a new cell from
the value inside when you need the shorter type.

### When to use which

- Give a struct **one** lifetime parameter when all its references are
  shared (`&'a T`). Covariance lets the compiler pick a common lifetime.
- Give it **two** when one of them is under a `&mut` or inside a `Cell`,
  `RefCell` or `Mutex`: `&'borrow mut Parser<'text>`.
- In a method, write `&mut self`, not `&'a mut self`, unless you mean "this
  call borrows the value for the rest of its life".

### Read more

- [Subtyping and variance](https://doc.rust-lang.org/reference/subtyping.html) in the Rust Reference
- [Subtyping and variance](https://doc.rust-lang.org/nomicon/subtyping.html) in the Rustonomicon
- [Error code E0597](https://doc.rust-lang.org/error_codes/E0597.html)

## Lesson 02 · Lifetime bounds and higher-ranked trait bounds

### The idea

This section assumes Lesson 01, and closures and the `Fn` traits from
Chapter 14.

A **lifetime bound** is a `where` condition about lifetimes. There are two
forms.

- `'a: 'b` relates two lifetimes: `'a` outlives `'b`.
- `T: 'a` relates a type and a lifetime: every reference *inside* a value of
  type `T` is valid for at least `'a`. It says nothing about how long the
  value itself lives.

`T: 'static` is the form you meet most. It does **not** mean "lives until the
program ends". It means "contains no reference that could expire". `String`,
`u8` and `Vec<String>` are all `'static` types: they own everything they
hold. `&'a str` is `'static` only when `'a` is. A `String` that is created
and dropped inside one function still has a `'static` type.

You rarely write `T: 'a` on a struct, because the compiler infers it from
the fields. You must write it when you turn a generic value into a trait
object, because a `dyn Trait` hides the type and keeps only a lifetime:

```rust
use std::fmt::Debug;

// The box may hold borrows that last 'a, so T must not hold shorter ones.
fn boxed<'a, T: Debug + 'a>(value: T) -> Box<dyn Debug + 'a> {
    Box::new(value)
}

let name = String::from("temporary");
let by_ref = boxed(&name);   // T = &String: fine, the box cannot outlive `name`
let owned = boxed(7_u8);     // T = u8
assert_eq!(format!("{by_ref:?} {owned:?}"), "\"temporary\" 7");
```

### Who chooses a lifetime

A lifetime parameter of a function is chosen by the **caller**, once for each
call, and it always lasts longer than the whole body of the function. This
matters as soon as a function receives a callback.

```rust,compile_fail,E0597
fn each_twice<'a, F>(items: &'a [String], f: F) -> usize
where
    F: Fn(&'a str) -> usize,
{
    let joined = items.join(" ");
    f(&joined) + items.iter().map(|item| f(item)).sum::<usize>()
}
```

```text
error[E0597]: `joined` does not live long enough
  |
1 | fn each_twice<'a, F>(items: &'a [String], f: F) -> usize
  |               -- lifetime `'a` defined here
...
5 |     let joined = items.join(" ");
  |         ------ binding `joined` declared here
6 |     f(&joined) + items.iter().map(|item| f(item)).sum::<usize>()
  |     --^^^^^^^-
  |     | |
  |     | borrowed value does not live long enough
  |     argument requires that `joined` is borrowed for `'a`
7 | }
  | - `joined` dropped here while still borrowed
```

The bound `F: Fn(&'a str) -> usize` says: "`f` can be called with a string
that lives for `'a`", where `'a` is the caller's choice. `joined` is a local
variable, so it lives for less than `'a`, and `f` cannot take it.

What the function needs is a callback that works for *every* lifetime,
including lifetimes that exist only inside the function. That is a
**higher-ranked trait bound** (HRTB):

```rust
fn each_twice<F>(items: &[String], f: F) -> usize
where
    F: for<'x> Fn(&'x str) -> usize,
{
    let joined = items.join(" ");
    f(&joined) + items.iter().map(|item| f(item)).sum::<usize>()
}

let items = vec![String::from("ab"), String::from("cde")];
assert_eq!(each_twice(&items, |text| text.len()), 6 + 2 + 3);
```

`for<'x>` is read "for every lifetime `'x`". Now the lifetime is chosen by
the code that *calls `f`*, separately for each call. You have used this form
many times without seeing it: `F: Fn(&str) -> usize` is the short spelling
of exactly the bound above. The elision rules for `Fn` traits insert the
`for<'x>`. The same holds for a return type: `F: Fn(&str) -> &str` means
`F: for<'x> Fn(&'x str) -> &'x str`, a function whose result borrows from its
argument.

### Higher-ranked bounds on other traits

`for<'a>` can stand before any bound. The usual reason is a bound on a
*reference* to a generic type, when the reference is created inside the
function:

```rust
use std::collections::BTreeSet;

// Collects the numbers into a `C`, then walks that collection twice by
// reference. "For every lifetime 'a, a `&'a C` can be iterated and yields
// `&'a u32`."
fn spread<C>(numbers: &[u32]) -> u32
where
    C: FromIterator<u32>,
    for<'a> &'a C: IntoIterator<Item = &'a u32>,
{
    let collection: C = numbers.iter().copied().collect();
    let largest = (&collection).into_iter().max().copied().unwrap_or(0);
    let smallest = (&collection).into_iter().min().copied().unwrap_or(0);
    largest - smallest
}

assert_eq!(spread::<Vec<u32>>(&[3, 9, 4]), 6);
assert_eq!(spread::<BTreeSet<u32>>(&[5, 5]), 0);
```

`collection` is a local variable. A borrow of it has a lifetime that exists
only inside `spread`, so no lifetime parameter of `spread` could describe it.
The bound must hold for *every* lifetime, and then it holds for this one.
You meet the same shape with operators: a function that adds two values by
reference needs a bound on `&T`, not on `T`.

### Closures that return a borrow

A closure has no written signature, so the compiler infers one. When the
closure is passed directly to a function, the function's bound provides the
signature, and everything works. When the closure is first stored in a
variable, nothing provides it:

```rust,compile_fail
let trim = |text: &str| text.trim();
assert_eq!(trim("  a "), "a");
```

```text
error: lifetime may not live long enough
  |
1 | let trim = |text: &str| text.trim();
  |                   -   - ^^^^^^^^^^^ returning this value requires that `'1` must outlive `'2`
  |                   |   |
  |                   |   return type of closure is &'2 str
  |                   let's call the lifetime of this reference `'1`
```

The compiler made the argument lifetime higher-ranked (`'1`, any lifetime),
and gave the result a *different* lifetime `'2`. It does not apply the
elision rule of `fn` items, "one input reference, so the output borrows from
it", to closures. Writing `-> &str` on the closure does not help: the result
is the same error. There are three repairs.

```rust
// 1. Pass the closure directly to the function that needs it.
fn apply<F: Fn(&str) -> &str>(text: &str, f: F) -> &str {
    f(text)
}
assert_eq!(apply("  a ", |text| text.trim()), "a");

// 2. Send the closure through an identity function that states the bound.
fn constrain<F>(f: F) -> F
where
    F: for<'a> Fn(&'a str) -> &'a str,
{
    f
}
let trim = constrain(|text| text.trim());
assert_eq!(trim("  a "), "a");

// 3. Use a function item. Its signature follows the elision rules.
fn trim_fn(text: &str) -> &str {
    text.trim()
}
assert_eq!(trim_fn("  a "), "a");
```

Returning a closure from a function works for the same reason as repair 2:
a return type such as `impl Fn(&str) -> &str` states the bound. Such a
closure may also keep a reference that the function received:

```rust
// The closure captures `suffix`. In edition 2024 an `impl Trait` in return
// position may use every lifetime that is in scope, so the borrow of
// `suffix` needs no extra annotation.
fn without(suffix: &str) -> impl Fn(&str) -> &str {
    move |text| text.strip_suffix(suffix).unwrap_or(text)
}

let ending = String::from(".rs");
let stem = without(&ending);
assert_eq!(stem("main.rs"), "main");
assert_eq!(stem("notes"), "notes");
```

(In editions before 2024 the same function is rejected with E0700, and the
compiler asks for `+ use<'_>` at the end of the return type.)

### Tie the result to the data, not to the borrow

When the lifetimes of a method are elided, the result borrows from `self`.
For a type that only *views* data, that is often too strict:

```rust
struct Row<'a> {
    cells: Vec<&'a str>,
}

impl<'a> Row<'a> {
    // Elided: the result is tied to the borrow of `self`.
    fn first_strict(&self) -> &str {
        self.cells[0]
    }
    // Explicit: the result lives as long as the text, not as long as the row.
    fn first(&self) -> &'a str {
        self.cells[0]
    }
}

let text = String::from("id,name");
let first = {
    let row = Row { cells: text.split(',').collect() };
    row.first() // `row.first_strict()` would not compile here
};
assert_eq!(first, "id");
```

Both bodies are identical. Only the signature says how long the caller may
keep the result. With `&mut self` the difference is larger: a result that is
tied to the `&mut` borrow keeps the whole value locked.

### Mistakes people really make

**A missing `T: 'a` or `T: 'static`.**

```rust,compile_fail,E0310
use std::fmt::Debug;

fn boxed<T: Debug>(value: T) -> Box<dyn Debug> {
    Box::new(value)
}
```

```text
error[E0310]: the parameter type `T` may not live long enough
  |
3 | fn boxed<T: Debug>(value: T) -> Box<dyn Debug> {
  |                                     --------- this `dyn Trait` has an implicit `'static` lifetime bound
4 |     Box::new(value)
  |     ^^^^^^^^^^^^^^^
  |     |
  |     the parameter type `T` must be valid for the static lifetime...
  |     ...so that the type `T` will meet its required lifetime bounds
  |
help: consider adding an explicit lifetime bound
  |
3 | fn boxed<T: Debug + 'static>(value: T) -> Box<dyn Debug> {
  |                   +++++++++
```

The suggestion compiles, and it may be more than you want: it forbids every
borrowed `T`. The other fix is the one at the start of this section, a
lifetime on the trait object. Lesson 05 explains where the "implicit
`'static`" comes from.

**A named lifetime where "every lifetime" was meant.** The `each_twice`
error above. If a callback is called with data that the function itself
creates, its bound must be higher-ranked.

**Storing a borrowing closure in a `let`.** Use one of the three repairs.

### When to use which

- `F: Fn(&str) -> &str` (higher-ranked, elided) is the default for callbacks.
  Write `for<'a>` yourself only when elision cannot express the bound, as in
  `for<'a> &'a C: IntoIterator<Item = &'a u32>`.
- Use a named lifetime in a callback bound only when the callback's result
  must be tied to *one specific* borrow that the caller provides.
- Add `T: 'static` when a value must be able to outlive the current stack
  frame (a spawned thread, a global registry). Otherwise prefer `T: 'a`,
  which also accepts borrowed data.

### Read more

- [Higher-ranked trait bounds](https://doc.rust-lang.org/reference/trait-bounds.html#higher-ranked-trait-bounds) and [lifetime bounds](https://doc.rust-lang.org/reference/trait-bounds.html#lifetime-bounds) in the Rust Reference
- [Higher-rank trait bounds](https://doc.rust-lang.org/nomicon/hrtb.html) in the Rustonomicon
- [Lifetime elision](https://doc.rust-lang.org/reference/lifetime-elision.html) in the Rust Reference
- [Closure types](https://doc.rust-lang.org/reference/types/closure.html) in the Rust Reference

## Lesson 03 · Generic associated types and a lending iterator

### The idea

This section assumes Lessons 01 and 02, associated types (Chapter 11) and
the `Iterator` trait (Chapter 15).

Look at the heart of `Iterator`:

```rust
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
```

`Item` is **one** type, fixed for the whole iterator. The call `next(&mut self)`
borrows the iterator for some lifetime, and that lifetime is different for
every call. `Item` has no way to mention it. So an item can never borrow from
the iterator itself. It can borrow from something *else* that outlives the
iterator, as `std::slice::Iter<'a, T>` yields `&'a T`. That is all.

This is what lets you write `let v: Vec<_> = iter.collect();`: all items are
alive at the same time, and none of them depends on the iterator's state.
It is also why some useful iterators cannot exist:

- one that reads each record into the *same* buffer and yields a reference to
  that buffer;
- one that yields *overlapping* mutable parts of a slice. Two overlapping
  `&mut` must never be alive together, and `Iterator` cannot stop a caller
  from keeping two items.

The compiler has a message of its own for the first attempt:

```rust,compile_fail
struct LineBuffer {
    source: Vec<&'static str>,
    buffer: String,
}

impl Iterator for LineBuffer {
    type Item = &str;
    fn next(&mut self) -> Option<&str> {
        self.buffer = self.source.pop()?.to_uppercase();
        Some(&self.buffer)
    }
}
```

```text
error: associated type `Iterator::Item` is declared without lifetime parameters, so using a borrowed type for them requires that lifetime to come from the implemented type
  |
7 |     type Item = &str;
  |                 ^
  |
note: you can't create an `Iterator` that borrows each `Item` from itself, but you can instead create a new type that borrows your existing type and implement `Iterator` for that new type
```

What is missing is an associated type that takes a lifetime parameter. That
is a **generic associated type** (GAT):

```rust
trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;
}
```

`Item<'a>` is not one type. It is a family of types, one for each lifetime.
`next` returns `Self::Item<'_>`, the member of the family for the lifetime of
*this* `&mut self` borrow. An item therefore keeps the iterator mutably
borrowed for as long as the item is alive. The borrow checker then enforces
the rule "one item at a time" by itself. Such an iterator *lends* its items;
the caller gives each one back before asking for the next.

### A worked example

Each row of Pascal's triangle can be computed from the row before, in place.
A lending iterator can keep one vector and lend each row from it.

```rust
trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;
}

struct Pascal {
    row: Vec<u64>,
}

impl LendingIterator for Pascal {
    // "For the lifetime 'a of one `next` call, the item is a slice that is
    // borrowed for 'a."
    type Item<'a> = &'a [u64];

    fn next(&mut self) -> Option<&[u64]> {
        self.row.push(1);
        // Update from right to left, so that each sum reads old values.
        for i in (1..self.row.len() - 1).rev() {
            self.row[i] += self.row[i - 1];
        }
        Some(&self.row)
    }
}

let mut triangle = Pascal { row: Vec::new() };
let mut sums = Vec::new();
while let Some(row) = triangle.next() {
    sums.push(row.iter().sum::<u64>());
    if row.len() == 5 {
        break;
    }
}
assert_eq!(sums, [1, 2, 4, 8, 16]);
```

```text
triangle ──► row: Vec<u64> ──► [1, 3, 3, 1]      one heap buffer, reused

             `row` (the item) points into the same buffer.
             While it is alive, `triangle` is mutably borrowed:
             `triangle.next()` cannot be called, so the buffer cannot change
             under the item.
```

Three details of the syntax:

- In the trait, `where Self: 'a` is required (see the mistakes below). In an
  impl whose `Self` type holds no borrows, as here, you may leave it out.
- `Option<Self::Item<'_>>` and `Option<&[u64]>` are the same type in this
  impl. The impl may write either.
- There is no `for` loop for a lending iterator. `for` works with `Iterator`
  only. The loop is `while let Some(item) = lender.next()`.

### What a lending iterator costs

You give up everything that needs two items at once: `collect`, `max`,
`peekable`, `zip` of an iterator with itself, sorting. An adapter such as
"take the first n" still works, because it forwards one item at a time and
its own `Item<'a>` can simply be the inner `Item<'a>`.

There is also a real limitation in today's compiler. A method that takes a
closure over the items needs a higher-ranked bound (Lesson 02):

```rust
trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;

    fn for_each<F>(mut self, mut f: F)
    where
        Self: Sized,
        F: FnMut(Self::Item<'_>),
    {
        while let Some(item) = self.next() {
            f(item);
        }
    }
}
```

This compiles, and it works for a lender that holds no borrows, such as
`Pascal`. For a lender that borrows something, a call fails:

```text
error[E0597]: `text` does not live long enough
   |
60 |         let text = String::from("abc");
   |             ---- binding `text` declared here
61 |         Prefixes { text: &text, end: 0 }.for_each(|p| println!("{p}"));
   |         -----------------^^^^^----------------------------------------
   |         |                |
   |         |                borrowed value does not live long enough
   |         argument requires that `text` is borrowed for `'static`
...
note: due to a current limitation of the type system, this implies a `'static` lifetime
   |
10 |         F: FnMut(Self::Item<'_>),
   |            ^^^^^^^^^^^^^^^^^^^^^
```

The bound means `for<'x> FnMut(Self::Item<'x>)`. For `Item<'x>` to exist,
`Self: 'x` must hold, and "for every `'x`" includes `'static`. The compiler
cannot yet restrict the `for<'x>` to the lifetimes for which `Self: 'x` is
true, so it demands `Self: 'static`. Until that changes, write the
`while let` loop at the call site, or keep such methods for lenders that own
their data. This limitation is one reason why the standard library has no
lending iterator trait yet.

### Mistakes people really make

**Keeping an item while asking for the next one.**

```text
error[E0499]: cannot borrow `w` as mutable more than once at a time
   |
17 |     let a = w.next().unwrap();
   |             - first mutable borrow occurs here
18 |     let b = w.next().unwrap();
   |             ^ second mutable borrow occurs here
19 |     a[1] = b[0];
   |     ---- first borrow later used here
```

This is the trait doing its work. If you need two items at once, copy what
you need out of the first item (`to_owned()`), or use an ordinary iterator
over data that does not overlap.

**Leaving out `where Self: 'a`.**

```rust,compile_fail
trait LendingIterator {
    type Item<'a>;
    fn next(&mut self) -> Option<Self::Item<'_>>;
}
```

```text
error: missing required bound on `Item`
  |
2 |     type Item<'a>;
  |     ^^^^^^^^^^^^^-
  |                  |
  |                  help: add the required where clause: `where Self: 'a`
  |
  = note: this bound is currently required to ensure that impls have maximum flexibility
```

`next` returns `Self::Item<'a>` for the lifetime `'a` of a borrow of `Self`.
A borrow `&'a mut Self` can only exist when `Self: 'a`. The compiler makes
you write that on the associated type, so that every impl may use it: with
the bound, `type Item<'a> = &'a mut [T]` is allowed even when `Self`
contains a `&'s mut [T]`.

**Trying to get around the borrow with `unsafe`.** An implementation that
turns the borrowed window into a `&'s mut [T]` with the slice's own lifetime
compiles, and it lets a caller hold two overlapping `&mut` slices. That is
undefined behavior. The lifetime in `Item<'a>` is the safety argument;
do not remove it.

### When to use which

- Use `Iterator` whenever the items can be owned, or can borrow from data
  outside the iterator. It has `for`, `collect` and all the adapters.
  Non-overlapping mutable parts are fine: `chunks_mut` and `iter_mut` are
  ordinary iterators.
- Use a lending iterator when an item must borrow from the iterator's own
  state: a reused buffer, a cursor that decodes in place, overlapping
  mutable windows.
- GATs are not only for lifetimes. `type Pointer<T>: Deref<Target = T>` lets
  a trait be generic over `Rc` and `Arc`. The same rule applies: use one when
  an associated type must depend on a parameter that is chosen later than
  the impl.

### Read more

- [Associated types](https://doc.rust-lang.org/reference/items/associated-items.html#associated-types) in the Rust Reference, which covers generic associated types and the required bounds
- [`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html) and [`slice::windows`](https://doc.rust-lang.org/std/primitive.slice.html#method.windows) in the standard library
- [Error code E0499](https://doc.rust-lang.org/error_codes/E0499.html)

## Lesson 04 · PhantomData: ownership, variance, auto traits and drop check

### The idea

This section assumes Lesson 01, the auto traits `Send` and `Sync` (Chapter
19), and raw pointers (Chapter 23).

The compiler learns three things about a generic type from its fields:

1. its **variance** in each parameter (Lesson 01);
2. whether it is **`Send` and `Sync`** (a type is, when all its fields are);
3. what happens when a value is **dropped**: which destructors run, and so
   which borrowed data must still be alive at that moment.

That works as long as the fields say what the type really does. Sometimes
they do not:

- A parameter is used only as a label, and no field has that type.
- A field is a raw pointer. `*const T` has no lifetime, it is neither `Send`
  nor `Sync`, and dropping it frees nothing. The type around it may behave
  like a `&'a T`, a `&'a mut T` or a `Box<T>`, and the compiler cannot know
  which.

A parameter that no field uses is an error, because the compiler would have
nothing to compute from:

```rust,compile_fail,E0392
struct Id<T> {
    raw: u32,
}
```

```text
error[E0392]: type parameter `T` is never used
  |
1 | struct Id<T> {
  |           ^ unused type parameter
  |
  = help: consider removing `T`, referring to it in a field, or using a marker such as `PhantomData`
```

`std::marker::PhantomData<X>` is the tool for both situations. It is a
zero-sized type with one value, also written `PhantomData`. For the three
questions above, the compiler treats a `PhantomData<X>` field as if the
struct stored an `X`. At run time there is nothing: no bytes, no destructor.

So the question when you add a marker is always: **which type `X` behaves the
way my type behaves?**

| Marker | Variance | `Send` / `Sync` | Says |
| --- | --- | --- | --- |
| `PhantomData<T>` | covariant in `T` | as `T` | "I own a `T`" |
| `PhantomData<&'a T>` | covariant in `'a` and `T` | both need `T: Sync` | "I borrow a `T` for `'a`" |
| `PhantomData<&'a mut T>` | covariant in `'a`, invariant in `T` | `Send` needs `T: Send`, `Sync` needs `T: Sync` | "I borrow a `T` exclusively" |
| `PhantomData<fn() -> T>` | covariant in `T` | always both | "I can produce a `T`; I hold none" |
| `PhantomData<fn(T)>` | contravariant in `T` | always both | "I can consume a `T`" |
| `PhantomData<fn(T) -> T>` | invariant in `T` | always both | "`T` must match exactly" |
| `PhantomData<*const T>` | covariant in `T` | never | "like a raw pointer" |
| `PhantomData<*mut T>` | invariant in `T` | never | "like a mutable raw pointer" |

Some rows, checked by the compiler:

```rust
use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;

fn is_send<T: Send>() {}
fn is_sync<T: Sync>() {}

// A function pointer type mentions T and holds no T.
is_send::<PhantomData<fn() -> Rc<u8>>>();
is_sync::<PhantomData<fn(Cell<u8>)>>();
// PhantomData<T> passes on what T is.
is_send::<PhantomData<String>>();
// PhantomData<&T> is Send when T is Sync, as &T is.
is_send::<PhantomData<&u8>>();

fn covariant<'long: 'short, 'short>(
    marker: PhantomData<fn() -> &'long str>,
) -> PhantomData<fn() -> &'short str> {
    marker
}

fn contravariant<'long: 'short, 'short>(
    marker: PhantomData<fn(&'short str)>,
) -> PhantomData<fn(&'long str)> {
    marker
}

assert_eq!(size_of::<PhantomData<[u8; 1000]>>(), 0);
```

```rust,compile_fail,E0277
use std::marker::PhantomData;
use std::rc::Rc;

fn is_send<T: Send>() {}
is_send::<PhantomData<Rc<u8>>>(); // "owns an Rc", so it is not Send
```

### A worked example: an owning pointer

`Owned<T>` is a small `Box<T>`. It keeps a raw pointer, so every one of the
three questions needs an answer from you.

```rust
use std::marker::PhantomData;
use std::ptr::NonNull;

/// Owns one heap-allocated `T`.
///
/// Invariant: `ptr` came from `Box::into_raw` and was not freed, and this
/// `Owned` is the only thing that will ever free it or reach the `T`.
struct Owned<T> {
    ptr: NonNull<T>,
    // "I own a T": the documented marker for an owning raw pointer.
    _owns: PhantomData<T>,
}

impl<T> Owned<T> {
    fn new(value: T) -> Self {
        let raw = Box::into_raw(Box::new(value));
        // SAFETY: `Box::into_raw` never returns a null pointer.
        let ptr = unsafe { NonNull::new_unchecked(raw) };
        Owned { ptr, _owns: PhantomData }
    }

    fn get(&self) -> &T {
        // SAFETY: by the invariant, `ptr` points to a live, initialized `T`
        // that only this `Owned` can reach. `self` stays borrowed for as
        // long as the result, so `get_mut` cannot make a `&mut T` meanwhile.
        unsafe { self.ptr.as_ref() }
    }

    fn get_mut(&mut self) -> &mut T {
        // SAFETY: as in `get`; `&mut self` is exclusive for as long as the
        // result is used, so the `&mut T` has no alias.
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Drop for Owned<T> {
    fn drop(&mut self) {
        // SAFETY: by the invariant, `ptr` came from `Box::into_raw` and was
        // not freed. `drop` runs at most once, and nothing can use `ptr`
        // afterwards.
        drop(unsafe { Box::from_raw(self.ptr.as_ptr()) });
    }
}

// SAFETY: an `Owned<T>` is the only owner of its `T`. Sending it to another
// thread moves the `T` there, which is sound exactly when `T: Send`.
unsafe impl<T: Send> Send for Owned<T> {}
// SAFETY: a `&Owned<T>` gives out only `&T` (through `get`), so sharing it
// between threads is sound exactly when `T: Sync`.
unsafe impl<T: Sync> Sync for Owned<T> {}

let mut boxed = Owned::new(String::from("on the heap"));
boxed.get_mut().push('!');
assert_eq!(boxed.get(), "on the heap!");
```

Go through the three questions:

- **Variance.** `NonNull<T>` is covariant in `T`, like `*const T`. An owner
  of a `T` may be covariant, as `Box<T>` is, so nothing more is needed. If
  the field were `*mut T`, the type would be invariant; that is one reason
  to prefer `NonNull<T>` in a container.
- **Auto traits.** `NonNull<T>` is neither `Send` nor `Sync`, and so the
  struct is neither, whatever the marker says. One field that is not `Send`
  is enough. The two `unsafe impl` lines add the traits back, with the same
  bounds that `Box<T>` has. A wrong bound here compiles without a warning.
  It is one of the easiest ways to write unsound code, so each line has a
  safety argument.
- **Drop.** The `Drop` impl frees the `T`. See the next part for what the
  drop checker makes of that.

### Drop check

When a variable goes out of scope, its destructor may run code. That code
must not read data that is already dead. Variables are dropped in the
reverse order of their declaration, so this can go wrong:

```rust,compile_fail,E0597
struct Inspector<'a>(&'a u8);

impl Drop for Inspector<'_> {
    fn drop(&mut self) {
        println!("last look: {}", self.0);
    }
}

fn main() {
    let inspector;
    let value = 7_u8;
    inspector = Inspector(&value);
}
```

```text
error[E0597]: `value` does not live long enough
   |
11 |     let value = 7_u8;
   |         ----- binding `value` declared here
12 |     inspector = Inspector(&value);
   |                           ^^^^^^ borrowed value does not live long enough
13 | }
   | -
   | |
   | `value` dropped here while still borrowed
   | borrow might be used here, when `inspector` is dropped and runs the `Drop` code for type `Inspector`
   |
   = note: values in a scope are dropped in the opposite order they are defined
```

`value` is dropped first, and then `Drop for Inspector` would read it. The
part of the borrow checker that finds this is called the **drop checker**.
Its rule on stable Rust has three cases. They depend on what happens when a
value of the type is dropped.

1. **Nothing runs.** The type has no `Drop` impl, and no field (at any
   depth) has one. A reference, a raw pointer, a number and a bare
   `PhantomData` are like this. Dropping such a value is no code at all, so
   the drop checker asks for nothing.
2. **The type has a `Drop` impl.** Then, at the moment a value is dropped,
   **every lifetime and every type parameter of the type must still be
   valid**. The compiler does not look inside `drop`; it assumes that the
   destructor may use everything the type mentions.
3. **The type has no `Drop` impl of its own, and some field has code to
   run** (a `String`, a `Box`, a field whose type has a `Drop` impl). Then
   the compiler goes through the fields and asks the same question of each
   one. In this walk, a field of type `PhantomData<X>` is treated like a
   field of type `X`.

Remove the `Drop` impl from `Inspector` and the program above compiles. That
is case 1: a dead reference that nobody reads is harmless.

```rust
struct Inspector<'a>(&'a u8);

let inspector;
let value = 7_u8;
inspector = Inspector(&value);
assert_eq!(*inspector.0, 7);
```

Where does `PhantomData` come in? Only in case 3. There the marker
"I own an `X`" is taken at its word:

```rust,compile_fail,E0597
use std::marker::PhantomData;

struct Inspector<'a>(&'a u8);
impl Drop for Inspector<'_> {
    fn drop(&mut self) {}
}

// No Drop impl of its own. The String must be freed, so dropping a Labelled
// runs code (case 3). The marker says "and I own an Inspector<'a>".
struct Labelled<'a> {
    label: String,
    _owns: PhantomData<Inspector<'a>>,
}

fn label<'a>(_borrowed: &'a u8) -> Labelled<'a> {
    Labelled { label: String::from("seven"), _owns: PhantomData }
}

fn main() {
    let labelled;
    let value = 7_u8;
    labelled = label(&value);
}
```

```text
error[E0597]: `value` does not live long enough
   |
21 |     let value = 7_u8;
   |         ----- binding `value` declared here
22 |     labelled = label(&value);
   |                      ^^^^^^ borrowed value does not live long enough
23 | }
   | -
   | |
   | `value` dropped here while still borrowed
   | borrow might be used here, when `labelled` is dropped and runs the destructor for type `Labelled<'_>`
```

No `Inspector` is ever created here. The compiler believes the marker: a
type that owns an `Inspector<'a>` runs `Drop for Inspector` when it is
dropped, and that destructor may read through `'a`. Three small changes each
make the program compile, and each one shows a part of the rule:

- Remove the `label` field. Nothing is left that has code to run, so the
  type falls under case 1 and is not checked at all. A struct that holds
  *only* markers never fails the drop check.
- Change the marker to `PhantomData<&'a u8>`. Dropping a reference runs no
  code.
- Change the marker to `PhantomData<fn() -> Inspector<'a>>`. A function
  pointer owns nothing. This is one more reason to use `fn() -> T` for a
  parameter that is only a label.

Now look at a type that owns its data through a raw pointer, such as
`Owned<T>` above. It needs a `Drop` impl to free the data, so it falls under
case 2, and case 2 already demands that everything is valid. For such a
type the marker `PhantomData<T>` does **not** change what the drop checker
accepts on stable Rust. Write it all the same: it is true, and readers
expect it.

The standard library has one more reason, and it needs an unstable feature.
A `Vec<&T>` may be declared *before* the data its elements point to,
although `Vec` has a `Drop` impl:

```rust
let list;
let value = 7_u8;
list = vec![&value]; // accepted for Vec; rejected for a Vec-like type of your own
assert_eq!(list.len(), 1);
```

The standard library marks the type parameter of `Vec`'s destructor with the
unstable attribute `#[may_dangle]`. With it, the library promises that the
destructor does not read a `T`, except to drop it. That switches case 2 off
for `T`. The drop checker then walks the fields, as in case 3, and finds a
`PhantomData<T>` deep inside `Vec`. That marker tells it that dropping a
`Vec<T>` can drop a `T`, so a `T` with a destructor of its own is still
checked. This is the historical "drop check" use of the marker.

### Mistakes people really make

**Deriving traits on a type with a marker.** `#[derive(Clone)]` adds the
bound `T: Clone` for every type parameter, used or not.

```text
error[E0382]: borrow of moved value: `a`
   |
12 |     let a: Handle<Texture> = Handle { index: 3, _kind: PhantomData };
   |         - move occurs because `a` has type `Handle<Texture>`, which does not implement the `Copy` trait
13 |     let b = a;
   |             - value moved here
14 |     println!("{} {}", a.index, b.index);
   |                       ^^^^^^^ value borrowed here after move
   |
note: if all bounds were met, you could clone the value
   |
 3 | #[derive(Clone, Copy)]
   |          ----- derived `Clone` adds implicit bounds on type parameters
 4 | struct Handle<T> {
   | ^^^^^^^^^^^^^^-^
   |               |
   |               introduces an implicit `T: Clone` bound
...
   = help: consider manually implementing `Clone` to avoid undesired bounds
```

`Handle<Texture>` holds a `usize` and should be `Copy`, and it is not,
because `Texture` is not. Write the impls by hand: `impl<T> Clone for
Handle<T>` with no bound on `T`.

**`PhantomData<T>` on a handle.** It says "I own a `T`". A `Handle<Rc<u8>>`
is then not `Send`, although it holds only an index. For a label, use
`PhantomData<fn() -> T>`.

**The wrong bound on `unsafe impl Send`.** A type that hands out `&T` is like
a `&T`: sending it to another thread needs `T: Sync`, not `T: Send`. With
`T: Send`, a wrapper around `Cell<u8>` could be read on one thread while it
is written on another. No test will show this reliably. Decide which
standard type your type behaves like, and copy the bounds of that type.

**A marker with the wrong direction.** `PhantomData<fn(&'a T)>` mentions
`'a` and compiles, and it is contravariant: the lifetime of your type can
then *grow*. For a type that borrows, that is unsound. `PhantomData<&'a T>`
is the marker for "I borrow".

### When to use which

- A label, an id, a typed index: `PhantomData<fn() -> T>`. For a typestate
  (Lesson 07) the states are zero-sized types of your own, so the plain
  `PhantomData<State>` behaves the same and is the usual spelling.
- An owning raw pointer: `PhantomData<T>`, and a `Drop` impl.
- A raw pointer that borrows: `PhantomData<&'a T>` or `PhantomData<&'a mut T>`.
- A type that must not leave its thread: `PhantomData<*const ()>`.
- A lifetime that must match exactly (a "brand"): `PhantomData<fn(&'a ()) -> &'a ()>`.

### Read more

- [`PhantomData`](https://doc.rust-lang.org/std/marker/struct.PhantomData.html) in the standard library
- [PhantomData](https://doc.rust-lang.org/nomicon/phantom-data.html), [drop check](https://doc.rust-lang.org/nomicon/dropck.html) and [Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html) in the Rustonomicon
- [Drop check](https://doc.rust-lang.org/std/ops/trait.Drop.html#drop-check) in the documentation of `Drop`, for the exact rules
- [Error code E0392](https://doc.rust-lang.org/error_codes/E0392.html)

## Lesson 05 · Trait objects: vtables, dyn compatibility, upcasting, lifetimes

### The idea

This section assumes trait objects as introduced in Chapter 11 (`&dyn Trait`
and `Box<dyn Trait>`), and Lessons 02 and 04 of this module.

A value of type `dyn Shape` is "some value whose type implements `Shape`".
Its size is not known at compile time, so it always lives behind a pointer.
That pointer is **two machine words** wide:

```text
&dyn Shape
┌──────────────┬──────────────┐
│ data pointer │ vtable ptr   │
└──────┬───────┴──────┬───────┘
       │              │            vtable for (Circle as Shape), in read-only data
       ▼              ▼            ┌─────────────────────────────┐
   Circle { radius }               │ drop_in_place::<Circle>     │
                                   │ size of Circle              │
                                   │ alignment of Circle         │
                                   │ <Circle as Shape>::area     │
                                   │ <Circle as Shape>::name     │
                                   └─────────────────────────────┘
```

The compiler builds a **vtable** for each pair of a concrete type and a
trait that is used as a trait object. A call `shape.area()` loads the
function pointer from the vtable and calls it with the data pointer. The
vtable also holds what is needed to handle the value without knowing its
type: its size and alignment (to free a `Box<dyn Shape>`) and its destructor
(to drop it).

The exact layout of a vtable is not a promise of the language, and you must
not read it with pointer tricks. You can observe the facts above safely:

```rust
trait Shape {
    fn area(&self) -> f64;
}

struct Circle {
    radius: f64,
}
struct Tile {
    width: f64,
    height: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.0 * self.radius * self.radius
    }
}
impl Shape for Tile {
    fn area(&self) -> f64 {
        self.width * self.height
    }
}

let word = size_of::<usize>();
assert_eq!(size_of::<&Circle>(), word);
assert_eq!(size_of::<&dyn Shape>(), 2 * word);
assert_eq!(size_of::<Box<dyn Shape>>(), 2 * word);

let tile = Tile { width: 2.0, height: 4.0 };
let shape: &dyn Shape = &tile;
// The size comes from the vtable; the method call goes through it.
assert_eq!(size_of_val(shape), 16);
assert_eq!(shape.area(), 8.0);
// The data pointer is the address of the value itself.
assert!(std::ptr::addr_eq(shape, &tile));
```

### A worked example: a vtable written by hand

To see that there is no magic, here is the same mechanism in plain Rust: a
struct of function pointers, one static instance per concrete type, and a
two-word "trait object". It reuses `PhantomData` from Lesson 04 to keep the
borrow.

```rust
use std::marker::PhantomData;

struct Circle {
    radius: f64,
}
struct Square {
    side: f64,
}

struct ShapeVtable {
    size: usize,
    name: &'static str,
    area: unsafe fn(*const ()) -> f64,
}

/// # Safety
/// `data` must point to a live `Circle`.
unsafe fn circle_area(data: *const ()) -> f64 {
    // SAFETY: the caller promises that `data` points to a live `Circle`.
    let circle = unsafe { &*data.cast::<Circle>() };
    3.0 * circle.radius * circle.radius
}

/// # Safety
/// `data` must point to a live `Square`.
unsafe fn square_area(data: *const ()) -> f64 {
    // SAFETY: the caller promises that `data` points to a live `Square`.
    let square = unsafe { &*data.cast::<Square>() };
    square.side * square.side
}

static CIRCLE: ShapeVtable = ShapeVtable {
    size: size_of::<Circle>(),
    name: "circle",
    area: circle_area,
};
static SQUARE: ShapeVtable = ShapeVtable {
    size: size_of::<Square>(),
    name: "square",
    area: square_area,
};

/// Invariant: `data` points to a value of the type that `vtable` was built
/// for, and that value is borrowed (shared) for 'a.
struct DynShape<'a> {
    data: *const (),
    vtable: &'static ShapeVtable,
    _borrow: PhantomData<&'a ()>,
}

impl<'a> DynShape<'a> {
    fn from_circle(circle: &'a Circle) -> Self {
        DynShape {
            data: (circle as *const Circle).cast(),
            vtable: &CIRCLE,
            _borrow: PhantomData,
        }
    }

    fn from_square(square: &'a Square) -> Self {
        DynShape {
            data: (square as *const Square).cast(),
            vtable: &SQUARE,
            _borrow: PhantomData,
        }
    }

    fn area(&self) -> f64 {
        // SAFETY: the two constructors are the only way to build a
        // `DynShape`. Each pairs `data` with the vtable of its own type, and
        // the value is borrowed for 'a, so it is still alive here.
        unsafe { (self.vtable.area)(self.data) }
    }
}

let circle = Circle { radius: 1.0 };
let square = Square { side: 2.0 };
let shapes = [DynShape::from_circle(&circle), DynShape::from_square(&square)];
let areas: Vec<f64> = shapes.iter().map(DynShape::area).collect();
assert_eq!(areas, [3.0, 4.0]);
assert_eq!(shapes[1].vtable.name, "square");
assert_eq!(shapes[0].vtable.size, 8);
assert_eq!(size_of::<DynShape>(), 2 * size_of::<usize>());
```

`dyn Trait` does all of this for you, and it checks the pairing of data and
vtable that this code has to argue for in a comment.

### Dyn compatibility

A trait can be used as `dyn Trait` only if a vtable can be built for it and
every entry can be called without knowing the concrete type. Such a trait is
**dyn compatible** (older texts say "object safe"). The rules follow from
the picture:

- The trait must not require `Self: Sized` (so no `Clone` supertrait): a
  `dyn Trait` is not `Sized`.
- No associated constants, and no associated types with generic parameters.
- Every method must be callable through the vtable, **or** be excluded from
  it with `where Self: Sized`. A method is callable through the vtable when
  - it has a receiver: `&self`, `&mut self`, `self: Box<Self>`,
    `self: Rc<Self>`, `self: Arc<Self>` or `self: Pin<..>`;
  - it has no type parameters (lifetime parameters are fine): one vtable
    slot cannot hold a function for every `T`;
  - it does not use `Self` anywhere else in its signature, and does not
    return `impl Trait` (an `async fn` does): the caller would not know the
    size of the value.

```rust,compile_fail,E0038
trait Parser {
    fn parse<R: std::io::Read>(&self, input: R) -> usize;
    fn name(&self) -> String;
}

fn first_name(parsers: &[Box<dyn Parser>]) -> String {
    parsers[0].name()
}
```

```text
error[E0038]: the trait `Parser` is not dyn compatible
  |
6 | fn first_name(parsers: &[Box<dyn Parser>]) -> String {
  |                              ^^^^^^^^^^ `Parser` is not dyn compatible
  |
note: for a trait to be dyn compatible it needs to allow building a vtable
      for more information, visit <https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility>
  |
1 | trait Parser {
  |       ------ this trait is not dyn compatible...
2 |     fn parse<R: std::io::Read>(&self, input: R) -> usize;
  |        ^^^^^ ...because method `parse` has generic type parameters
  = help: consider moving `parse` to another trait
```

The error appears where `dyn Parser` is written, not at the trait. A trait
can be not dyn compatible for a long time before anybody notices. There are
two repairs, and they mean different things:

```rust
use std::io::Read;

trait Parser {
    // Repair 1: take a trait object. The method stays in the vtable.
    fn parse(&self, input: &mut dyn Read) -> usize;

    // Repair 2: keep the generic method and take it out of the vtable.
    // It can then be called on concrete types only.
    fn parse_all<I: IntoIterator<Item = u8>>(&self, input: I) -> usize
    where
        Self: Sized,
    {
        input.into_iter().count()
    }

    fn name(&self) -> String;
}

struct Counting;
impl Parser for Counting {
    fn parse(&self, input: &mut dyn Read) -> usize {
        let mut bytes = Vec::new();
        input.read_to_end(&mut bytes).unwrap_or(0)
    }
    fn name(&self) -> String {
        "counting".to_string()
    }
}

let parsers: Vec<Box<dyn Parser>> = vec![Box::new(Counting)];
let mut input: &[u8] = b"abc";
assert_eq!(parsers[0].parse(&mut input), 3);
assert_eq!(Counting.parse_all([1, 2]), 2);
```

### Upcasting

If `trait Animal: Debug`, every `dyn Animal` is also something that
implements `Debug`. A pointer to a `dyn Animal` can be converted to a pointer
to a `dyn Debug`. This is **upcasting**, and it is an ordinary coercion: it
happens wherever the target type is known.

```rust
use std::any::Any;
use std::fmt::Debug;

trait Animal: Debug + Any {
    fn name(&self) -> String;
}

#[derive(Debug)]
struct Cat;
impl Animal for Cat {
    fn name(&self) -> String {
        "cat".to_string()
    }
}

let pet: Box<dyn Animal> = Box::new(Cat);

// To a reference to the supertrait...
let shown: &dyn Debug = &*pet;
assert_eq!(format!("{shown:?}"), "Cat");

// ...and, by value, to `dyn Any`, which can recover the concrete type.
let any: Box<dyn Any> = pet;
assert!(any.downcast::<Cat>().is_ok());
```

The data pointer does not change. Only the vtable pointer may change: after
the upcast it leads to a vtable of the supertrait for the same concrete type. Before upcasting was
part of the language, traits needed a hand-written method
`fn as_any(&self) -> &dyn Any`; you will still see it in older code.

`Any` works only for `'static` types, because a type id cannot encode a
lifetime. A trait with `Any` as a supertrait therefore accepts only
implementors that hold no borrows.

### The lifetime of a trait object

A `dyn Trait` hides its concrete type, and with it every lifetime inside
that type. The compiler still has to know how long the hidden value is
valid, so the full type is always `dyn Trait + 'lifetime`. When you do not
write the lifetime, a default applies:

| You write, in a signature, a field or a type alias | It means |
| --- | --- |
| `Box<dyn Trait>`, `Rc<dyn Trait>`, `Vec<Box<dyn Trait>>` | `dyn Trait + 'static` |
| `&'a dyn Trait`, `&'a mut dyn Trait` | `dyn Trait + 'a` |

The first row is the one that surprises people.

```rust,compile_fail,E0597
trait Shape {
    fn area(&self) -> f64;
}

struct View<'a>(&'a f64);
impl Shape for View<'_> {
    fn area(&self) -> f64 {
        *self.0
    }
}

struct Registry {
    items: Vec<Box<dyn Shape>>,
}

fn main() {
    let side = 2.0;
    let mut registry = Registry { items: Vec::new() };
    registry.items.push(Box::new(View(&side)));
}
```

```text
error[E0597]: `side` does not live long enough
   |
17 |     let side = 2.0;
   |         ---- binding `side` declared here
18 |     let mut registry = Registry { items: Vec::new() };
19 |     registry.items.push(Box::new(View(&side)));
   |                         --------------^^^^^--
   |                         |             |
   |                         |             borrowed value does not live long enough
   |                         coercion requires that `side` is borrowed for `'static`
20 | }
   | - `side` dropped here while still borrowed
   |
   = note: due to object lifetime defaults, `Box<dyn Shape>` actually means `Box<(dyn Shape + 'static)>`
```

The last line says it. A registry that should accept borrowing values needs
a lifetime parameter: `struct Registry<'a> { items: Vec<Box<dyn Shape + 'a>> }`.
In a function signature the short form is `Box<dyn Shape + '_>`.

The default applies where a type is written in an *item*: a field, a
function signature, a type alias, a `static`. Inside a function body the
rule is different. In the type of a `let`, a missing lifetime is not
`'static`; the compiler infers it, as it infers every other lifetime in a
body. So the same words can be accepted in one place and rejected in the
other:

```rust
trait Shape {
    fn area(&self) -> f64;
}

struct View<'a>(&'a f64);
impl Shape for View<'_> {
    fn area(&self) -> f64 {
        *self.0
    }
}

let side = 2.0;
// A `let` in a body: the lifetime of the trait object is inferred. The
// same type in the struct field above demanded 'static.
let local: Box<dyn Shape> = Box::new(View(&side));
assert_eq!(local.area(), 2.0);
```

This is why the error often appears only when you move a working `let` into
a struct or a return type.

Auto traits are written the same way: `Box<dyn Shape + Send>` is a different
type from `Box<dyn Shape>`. The second one is not `Send`, even when the
value inside is.

### Mistakes people really make

**Asking the box for its type.** `Box<dyn Any>` is a `'static` type, so it
implements `Any` itself. `boxed.type_id()` finds that impl first and returns
the type id of the *box*. `(*boxed).type_id()` asks the value. The same trap
exists for `&boxed` used as a `&dyn Any`: the trait object then wraps the
box. Dereference first: `&*boxed`.

**Generic code that does not accept the box.** `Box<dyn Shape>` does not
implement `Shape` by itself, so it cannot be passed to
`fn total<S: Shape>(shapes: &[S])`:

```text
error[E0277]: the trait bound `Box<dyn Shape>: Shape` is not satisfied
  |
7 |     println!("{}", total(&v));
  |                    ----- ^^ the trait `Shape` is not implemented for `Box<dyn Shape>`
  |                    |
  |                    required by a bound introduced by this call
```

The author of the trait can add a *forwarding impl* for a pointer type. This
one makes every `&S` a shape, including `&dyn Shape`:

```rust
trait Shape {
    fn area(&self) -> f64;
}

struct Square(f64);
impl Shape for Square {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
}

// `?Sized` allows `S = dyn Shape`.
impl<S: Shape + ?Sized> Shape for &S {
    fn area(&self) -> f64 {
        (**self).area()
    }
}

fn total<S: Shape>(shapes: &[S]) -> f64 {
    shapes.iter().map(Shape::area).sum()
}

let (a, b) = (Square(1.0), Square(2.0));
let list: [&dyn Shape; 2] = [&a, &b];
assert_eq!(total(&list), 5.0);
```

**Comparing trait object pointers with `==`.** A comparison of two
`*const dyn Trait` compares the vtable pointers too, and the same type can
have more than one copy of a vtable in a program. Use `std::ptr::addr_eq`
to ask "is this the same value?".

### When to use which

- **Generics** (`fn f<S: Shape>(s: &S)`): one copy of the code per type,
  calls can be inlined, and the type is known. Best in hot code and when all
  values have the same type.
- **Trait objects**: one copy of the code, a pointer indirection per call,
  and values of different types in one collection. Best for plug-in points,
  for long-lived collections of mixed values, and to keep compile times and
  binary size down.
- **An enum**: when the set of types is closed and known to you. It needs no
  allocation, and `match` checks that you handled every case.

### Read more

- [Trait object types](https://doc.rust-lang.org/reference/types/trait-object.html) and [dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility) in the Rust Reference
- [Default trait object lifetimes](https://doc.rust-lang.org/reference/lifetime-elision.html#default-trait-object-lifetimes) in the Rust Reference
- [`std::any`](https://doc.rust-lang.org/std/any/index.html) and [`std::ptr::addr_eq`](https://doc.rust-lang.org/std/ptr/fn.addr_eq.html) in the standard library
- [Error code E0038](https://doc.rust-lang.org/error_codes/E0038.html)

## Lesson 06 · Zero-sized types and uninhabited types

### The idea

This section assumes enums and pattern matching (Chapter 10), modules and
privacy (Chapter 8), and Lesson 04.

Count the values of a type. `bool` has two, `u8` has 256. The number of
values decides how much memory a value needs: enough bits to tell the values
apart. Two special cases follow.

A type with **exactly one value** needs no bits. It is a **zero-sized type**
(ZST). `()` is one. So are a struct without fields, an array of length zero,
`PhantomData<T>`, and the type of every function item and of every closure
that captures nothing.

A type with **no values at all** is **uninhabited**. An enum without
variants is one: `enum Never {}`. The standard library has
`std::convert::Infallible`, and the language has the never type `!`. No
expression can produce a value of such a type, so code that would receive
one can never run.

Both kinds cost nothing at run time, and both carry information for the
compiler. That combination is why they appear all over well-designed APIs.

### Zero-sized types

```rust
use std::marker::PhantomData;

struct Meters; // a unit struct: one value, written `Meters`

assert_eq!(size_of::<()>(), 0);
assert_eq!(size_of::<Meters>(), 0);
assert_eq!(size_of::<[String; 0]>(), 0);
assert_eq!(size_of::<PhantomData<String>>(), 0);

// A function item has a type of its own, and that type is zero-sized.
let trim = str::trim;
assert_eq!(size_of_val(&trim), 0);
// A function POINTER is one word: it can point to any function.
let pointer: fn(&str) -> &str = trim;
assert_eq!(size_of_val(&pointer), size_of::<usize>());

// A zero-sized field adds nothing.
struct Tagged {
    value: u32,
    unit: Meters,
}
assert_eq!(size_of::<Tagged>(), 4);
```

Collections handle ZSTs without touching the heap. A `Vec<()>` is a counter:

```rust
let mut ticks = Vec::new();
for _ in 0..1_000 {
    ticks.push(());
}
assert_eq!(ticks.len(), 1_000);
assert_eq!(ticks.capacity(), usize::MAX); // nothing was ever allocated
```

This is why `HashSet<T>` can be a `HashMap<T, ()>` inside: the `()` values
take no space.

One detail: zero-sized does not mean alignment 1. `[u64; 0]` has size 0 and
the alignment of `u64`, and a struct that contains it is aligned
accordingly.

The most useful ZST is one that only your module can create. A value of the
type is then a **proof**: it shows that some code of yours ran.

```rust
mod sensor {
    pub struct Sensor {
        offset: f64,
    }

    /// Proof that `calibrate` ran. The private field means that code
    /// outside this module cannot write `Calibrated(())`.
    pub struct Calibrated(());

    impl Sensor {
        pub fn new() -> Self {
            Sensor { offset: 0.0 }
        }

        pub fn calibrate(&mut self, reference: f64) -> Calibrated {
            self.offset = -reference;
            Calibrated(())
        }

        /// No check here: the caller has shown the proof.
        pub fn read(&self, raw: f64, _proof: &Calibrated) -> f64 {
            raw + self.offset
        }
    }
}

use sensor::Sensor;

let mut sensor = Sensor::new();
let proof = sensor.calibrate(0.5);
assert_eq!(sensor.read(2.0, &proof), 1.5);
assert_eq!(size_of_val(&proof), 0);
```

A caller cannot reach `read` without having called `calibrate`, and the
check costs no instruction. (This simple version does not tie the proof to
one particular sensor. That needs a lifetime or a type parameter as a
"brand", which is beyond this lesson.)

### Uninhabited types

`Result<T, E>` promises "a `T` or an `E`". If `E` has no values, only the
`Ok` side is left, and the compiler knows it:

```rust
use std::convert::Infallible;

// Every u8 fits in a u16, so this conversion cannot fail. The standard
// library says so with the error type `Infallible`.
let widened: Result<u16, Infallible> = u16::try_from(200_u8);

// No `else`, no `unwrap`: the pattern covers every value that can exist.
let Ok(value) = widened;
assert_eq!(value, 200);

// A match needs no arm for the impossible variant.
let again = match u16::try_from(7_u8) {
    Ok(value) => value,
};
assert_eq!(again, 7);

// The layout knows it too.
assert_eq!(size_of::<Result<u16, Infallible>>(), size_of::<u16>());
assert_eq!(size_of::<Option<Infallible>>(), 0);
```

The same works for your own types. A generic enum loses a variant when the
variant's payload is uninhabited:

```rust
use std::convert::Infallible;

enum Step<E> {
    Value(u32),
    Failed(E),
}

// A source that cannot fail: `Failed` cannot be constructed.
fn total(steps: Vec<Step<Infallible>>) -> u32 {
    steps
        .into_iter()
        .map(|step| match step {
            Step::Value(value) => value,
        })
        .sum()
}

assert_eq!(total(vec![Step::Value(2), Step::Value(3)]), 5);
```

This is the reason to give a trait an associated `Error` type, and not a
fixed one. An implementor that cannot fail chooses `Infallible`, and its
callers lose the error path at compile time.

An uninhabited type of your own is an empty enum: `enum Never {}`. When the
type needs a type parameter, an empty enum is not enough: `enum Tag<T> {}`
is rejected with E0392, because nothing uses `T` (Lesson 04). Use a struct
that has one field of an uninhabited type. No value can be written for
that field, so the struct has no values either:

```rust
use std::convert::Infallible;
use std::marker::PhantomData;

// Exists only as a type argument. It has no values, for every `T`.
struct Tag<T> {
    _never: Infallible,
    _marker: PhantomData<fn() -> T>,
}

assert_eq!(size_of::<Option<Tag<String>>>(), 0);
assert_eq!(size_of::<Result<u32, Tag<u8>>>(), size_of::<u32>());
```

The never type `!` is the built-in uninhabited type. It is the type of
`panic!()`, of `return`, of `loop {}` without `break`, and it converts to
every other type, which is why `let x: u32 = match o { Some(v) => v, None =>
return };` type-checks. On this toolchain you may write `!` as the return
type of a function, and not yet as the type of a variable:

```rust
fn fail(message: &str) -> ! {
    panic!("{message}")
}

let value: u32 = if true { 7 } else { fail("unreachable in this example") };
assert_eq!(value, 7);
```

```rust,compile_fail,E0658
let never: ! = panic!("not allowed as a variable type yet");
```

```text
error[E0658]: the `!` type is experimental
  |
1 | let never: ! = panic!("not allowed as a variable type yet");
  |            ^
```

For a type that you need to name, such as an error type, use `Infallible` or
an empty enum of your own.

> **Note: newer Rust versions.** Rust 1.100 (November 2026) stabilised the
> never type `!` and made `std::convert::Infallible` an alias of it
> (`pub type Infallible = !;`). This course is pinned to 1.99, where `!` may
> be written only as the return type of a function (anywhere else you get
> E0658, as above) and `Infallible` is still an empty enum,
> `pub enum Infallible {}`. On 1.100 and later the `compile_fail` example above
> compiles, and `Infallible` and `!` are the same type. What this lesson says
> about uninhabited types in general does not change. The release notes list these
> changes under
> [Rust 1.100.0](https://doc.rust-lang.org/stable/releases.html#version-11000-2026-11-12):
> "Stabilize never type", "`Infallible` is now a type alias to `!`" and "The
> never type fallback was changed to `!` on all editions".

### Mistakes people really make

**A proof that anyone can write.** `pub struct Token;` is a unit struct, and
every crate can create one with the expression `Token`. The proof needs a
private field: `pub struct Token(());` or `pub struct Token { _private: () }`.

**A proof that can be copied or defaulted.** `#[derive(Clone, Default)]` on
a proof type gives every caller a second way to obtain one. Derive nothing
that creates values, unless the proof is meant to be shared freely.

**Expecting an irrefutable pattern with an error type that has values.**

```rust,compile_fail,E0005
let parsed: Result<u8, String> = Ok(3);
let Ok(value) = parsed;
```

```text
error[E0005]: refutable pattern in local binding
  |
2 | let Ok(value) = parsed;
  |     ^^^^^^^^^ pattern `Err(_)` not covered
  |
  = note: `let` bindings require an "irrefutable pattern", like a `struct` or an `enum` with only one variant
...
  = note: the matched value is of type `Result<u8, String>`
```

`String` has values, so `Err` can happen. `Result<u8, ()>` has the same
problem: `()` is zero-sized, and it is *inhabited*. "Carries no information"
and "cannot exist" are different things.

**Pointer arithmetic that assumes a non-zero size.** Code that walks a slice
with a start and an end pointer never moves for a ZST. Count elements
instead. Lesson 04 does this.

### When to use which

- A ZST marker: when a *type* must differ and no data is needed (units,
  states, strategies, proofs).
- `Infallible` or an empty enum: when a generic slot must be filled and
  the case can never happen.
- `()`: when a slot must be filled and there is simply nothing to say.
- `Option<T>` or an enum with data: when the difference exists at run time.

### Read more

- [Exotically sized types](https://doc.rust-lang.org/nomicon/exotic-sizes.html) in the Rustonomicon
- [`Infallible`](https://doc.rust-lang.org/1.99.0/std/convert/enum.Infallible.html) in the standard library
- [The never type](https://doc.rust-lang.org/reference/types/never.html) in the Rust Reference
- [Error code E0005](https://doc.rust-lang.org/error_codes/E0005.html)

## Lesson 07 · Sealed traits, typestate and const generics

### The idea

This section assumes Lessons 04 and 06, and modules and privacy (Chapter 8).

Many values go through states: a connection is closed, then open; a builder
is incomplete, then complete; a file is open, then closed. Some operations
make sense in one state only. The usual way to handle this is a run-time
check: store the state in a field, test it in every method, and return an
error or panic when it is wrong. Every caller then has to deal with an error
that a correct program never produces.

**Typestate** moves the state into the type. A `Connection<Connected>` and a
`Connection<Disconnected>` are different types. A method that needs an open
connection is defined only for `Connection<Connected>`. Calling it in the
wrong state is then not an error to handle. It is a program that does not
compile.

Three ingredients, all from earlier lessons:

1. **State types** are zero-sized markers (Lesson 06).
2. The main type has a **type parameter** for the state and a `PhantomData`
   field for it (Lesson 04). The state costs no memory.
3. A **transition** is a method that takes `self` *by value* and returns the
   value in the new state. The move makes the old state unusable.

### A worked example

```rust
use std::marker::PhantomData;

struct Disconnected;
struct Connected;

struct Connection<State> {
    address: String,
    queries: u32,
    _state: PhantomData<State>,
}

// Available only before connecting.
impl Connection<Disconnected> {
    fn new(address: &str) -> Self {
        Connection {
            address: address.to_string(),
            queries: 0,
            _state: PhantomData,
        }
    }

    fn connect(self) -> Connection<Connected> {
        Connection {
            address: self.address,
            queries: self.queries,
            _state: PhantomData,
        }
    }
}

// Available only while connected.
impl Connection<Connected> {
    fn query(&mut self, text: &str) -> String {
        self.queries += 1;
        format!("{} <- {text}", self.address)
    }

    fn close(self) -> Connection<Disconnected> {
        Connection {
            address: self.address,
            queries: self.queries,
            _state: PhantomData,
        }
    }
}

// Available in every state.
impl<State> Connection<State> {
    fn queries(&self) -> u32 {
        self.queries
    }
}

let idle = Connection::new("db:5432");
let mut live = idle.connect();
assert_eq!(live.query("select 1"), "db:5432 <- select 1");
let closed = live.close();
assert_eq!(closed.queries(), 1);
// The state is not stored: both types are as large as their two fields.
assert_eq!(size_of::<Connection<Connected>>(), size_of::<(String, u32)>());
```

A transition moves every field into a new struct. This looks like work, and
it costs almost nothing: both types hold the same fields, a move of a
`String` copies three words, and an optimized build usually removes even
that. Nothing is allocated and nothing is dropped.

The two mistakes that the pattern prevents, with the real messages:

```text
error[E0599]: no method named `query` found for struct `Connection<Disconnected>` in the current scope
   |
 6 | struct Connection<State> {
   | ------------------------ method `query` not found for this struct
...
56 |     idle.query("select 1");
   |          ^^^^^ method not found in `Connection<Disconnected>`
   |
   = note: the method was found for `Connection<Connected>`
```

```text
error[E0382]: borrow of moved value: `live`
   |
56 |     let mut live = idle.connect();
   |         -------- move occurs because `live` has type `Connection<Connected>`, which does not implement the `Copy` trait
57 |     let closed = live.close();
   |                       ------- `live` moved due to this method call
58 |     live.query("select 1");
   |     ^^^^ value borrowed here after move
   |
note: `Connection::<Connected>::close` takes ownership of the receiver `self`, which moves `live`
   |
38 |     fn close(self) -> Connection<Disconnected> {
   |              ^^^^
```

The second one depends on `close(self)` taking the value. A transition that
takes `&self` and returns a new value leaves the old state usable, and the
pattern protects nothing.

### Sealed traits

In the example, `State` can be any type: `Connection<u8>` is a valid, useless
type. To say "the state is one of *these* types", give the states a trait
and bound the parameter by it. Then the next question is: who may implement
that trait? If a user of your library can add a state of their own, they can
call every method that is generic over states, and you can never add a
required item to the trait without breaking them.

A **sealed trait** is a public trait that only its own crate can implement.
Rust has no keyword for it. The pattern uses privacy:

```rust
mod sealed {
    // A public trait in a private module: this crate can name it,
    // other crates cannot.
    pub trait Sealed {}
}

/// A storage mode. Sealed: it cannot be implemented outside this crate.
pub trait Mode: sealed::Sealed {
    const READ_ONLY: bool;
}

pub struct ReadOnly;
pub struct ReadWrite;

impl sealed::Sealed for ReadOnly {}
impl sealed::Sealed for ReadWrite {}

impl Mode for ReadOnly {
    const READ_ONLY: bool = true;
}
impl Mode for ReadWrite {
    const READ_ONLY: bool = false;
}

fn describe<M: Mode>() -> &'static str {
    if M::READ_ONLY { "read-only" } else { "read-write" }
}

assert_eq!(describe::<ReadOnly>(), "read-only");
```

Another crate can *use* `Mode` as a bound and call its items. To implement
`Mode` for a type, it would also have to implement the supertrait, and it
cannot name `sealed::Sealed`. The compiler recognises the pattern and says
so:

```text
error[E0277]: the trait bound `Anything: Sealed` is not satisfied
   |
30 | impl storage::Mode for Anything {
   |                        ^^^^^^^^ unsatisfied trait bound
...
note: required by a bound in `Mode`
   |
 9 | pub trait Mode: sealed::Sealed {
   |                 ^^^^^^^^^^^^^^ required by this bound in `Mode`
   = note: `Mode` is a "sealed trait", because to implement it you also need to implement `storage::sealed::Sealed`, which is not accessible; this is usually done to force you to use one of the provided types that already implement it
   = help: the following types implement the trait:
             storage::ReadOnly
             storage::ReadWrite
```

(For this message the example was placed in a module named `storage`, and
the impl for `Anything` was written outside it.)

Seal a trait when its set of implementors is part of your design (states,
units, modes), or when you want to stay free to add methods to it. Do not
seal a trait that exists so that users can implement it.

A second trait can pick out a subset of the states. With
`trait Writable: Mode {}` implemented only for `ReadWrite`, an
`impl<M: Writable> Store<M>` block holds the methods for every writable
mode, written once.

### Const generics: values as type parameters

A type can also be generic over a *value*. You know one such type already:
`[T; N]`. With **const generics** you can write your own.

```rust
#[derive(Debug, PartialEq, Clone, Copy)]
struct Vector<const N: usize>([f64; N]);

impl<const N: usize> Vector<N> {
    // Both operands have the same N, so the lengths cannot differ.
    fn dot(&self, other: &Vector<N>) -> f64 {
        self.0.iter().zip(&other.0).map(|(a, b)| a * b).sum()
    }
}

// N is inferred from the array literal.
let a = Vector([1.0, 2.0, 3.0]);
let b = Vector([4.0, 5.0, 6.0]);
assert_eq!(a.dot(&b), 32.0);
assert_eq!(size_of::<Vector<3>>(), 24); // N is not stored
```

`Vector<3>` and `Vector<2>` are different types, and mixing them is a type
error that shows the two values:

```text
error[E0308]: mismatched types
   |
14 |     println!("{}", a.dot(&b));
   |                      --- ^^ expected `3`, found `2`
   |                      |
   |                      arguments to this method are incorrect
   |
   = note: expected reference `&Vector<3>`
              found reference `&Vector<2>`
```

This is typestate with numbers. An impl can also be written for some values
only, as `impl<const N: usize> Matrix<N, N>` holds the methods of square
matrices.

Know the limits on stable Rust. A const parameter can be an integer, a
`bool` or a `char`. And you cannot compute with it inside a type:

```rust,compile_fail
struct Stack<const N: usize> {
    items: [u8; N],
}

impl<const N: usize> Stack<N> {
    fn grow(self) -> Stack<{ N + 1 }> {
        todo!()
    }
}
```

```text
error: generic parameters may not be used in const operations
  |
6 |     fn grow(self) -> Stack<{ N + 1 }> {
  |                              ^ cannot perform const operation using `N`
  |
  = help: const parameters may only be used as standalone arguments here, i.e. `N`
```

So "append one element and get a `Stack<N + 1>`" cannot be expressed. What
can be expressed is every relation that only *repeats* parameters: same
length, swapped dimensions, a shared inner dimension.

### Mistakes people really make

**A transition that borrows.** `fn connect(&self) -> Connection<Connected>`
compiles and leaves the old value usable. Transitions take `self`.

**A constructor in the wrong `impl` block.** `impl<State> Connection<State>
{ fn new(..) -> Self }` lets a caller write `Connection::<Connected>::new(..)`
and skip the handshake. Put `new` in the `impl` of the first state.

**Public marker traits that are not sealed.** If `Mode` is not sealed, a
user's `struct Anything; impl Mode for Anything` is accepted, and the list of
states is no longer yours.

**Typestate for a state that is only known at run time.** If the state
depends on input (a config file, a message from the network), some code
must hold "one of the states" in one variable. That needs an enum:
`enum AnyConnection { Idle(Connection<Disconnected>), Live(Connection<Connected>) }`.
Typestate helps on the paths where the state is known when you write the
code.

### When to use which

- **Typestate**: a small, fixed set of states, a protocol that is known at
  compile time, and misuse that would be a bug in the calling code. Builders
  with required fields are the standard case.
- **An enum with a `match`**: states that change in response to run-time
  events. The compiler still checks that every state is handled.
- **A run-time check and a `Result`**: when the failure is a normal event
  that the caller should handle, such as bad user input.
- **Const generics**: sizes and dimensions that are fixed at compile time.
  For sizes that come from input, use a slice or a `Vec` and check at run
  time.

### Read more

- [Const generics](https://doc.rust-lang.org/reference/items/generics.html#const-generics) in the Rust Reference
- [Visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html) in the Rust Reference, for why a public trait in a private module cannot be named from outside
- [Supertraits](https://doc.rust-lang.org/reference/items/traits.html#supertraits) in the Rust Reference
- [Error code E0599](https://doc.rust-lang.org/error_codes/E0599.html)

## Lesson 08 · Coherence, the orphan rule and blanket impls

### The idea

This section assumes traits and generic impls (Chapters 11 and 13) and
crates and dependencies (Chapter 8).

When you call `value.to_string()`, the compiler looks for *the*
implementation of `Display` for the type of `value`. There must never be
two. If two crates could each implement `Display` for `Vec<u8>`, then a
program that uses both crates would have two answers, and the meaning of
your code would depend on which crates happen to be linked in.

**Coherence** is the property that every pair of a trait and a type has at
most one impl in the whole program. Rust enforces it at compile time with
two rules, and neither rule ever needs to look at more than one crate and
its dependencies:

- the **overlap rule**: inside one crate (together with the crates it
  depends on), no two impls of a trait may apply to the same type;
- the **orphan rule**: a crate may write an impl only if it "owns" enough of
  it, so that two crates that do not know each other can never write the
  same one.

### The orphan rule, precisely

Call a trait or a type **local** if it is defined in the current crate, and
**foreign** otherwise. Look at an impl in this general shape:

```text
impl<P1, .., Pn> Trait<T1, .., Tn> for T0
```

The impl is allowed if **the trait is local**, or if both of these hold:

1. at least one of the types `T0, T1, .., Tn` is a local type, and
2. no type parameter `P` appears *uncovered* in any of the types that come
   **before** the first local one, in the order `T0, T1, .., Tn`.

A parameter is **covered** when it appears as an argument of another type:
`T` is covered in `Vec<T>` and uncovered when it stands alone. Four wrapper
types are special ("fundamental"): `&T`, `&mut T`, `Box<T>` and `Pin<T>`
count as whatever is inside them. `Box<Local>` counts as local,
and `Box<T>` leaves `T` uncovered.

Here is the rule applied to one local type.

```rust
use std::fmt;
use std::str::FromStr;

struct Rgb(u8, u8, u8);

// Foreign trait, local type as T0.
impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

// Foreign trait, FOREIGN T0 (`u32`). Allowed: the local type is T1, and no
// type parameter stands before it.
impl From<Rgb> for u32 {
    fn from(color: Rgb) -> u32 {
        u32::from_be_bytes([0, color.0, color.1, color.2])
    }
}

// The same reason makes `(1, 2, 3) == Rgb(1, 2, 3)` possible.
impl PartialEq<Rgb> for (u8, u8, u8) {
    fn eq(&self, color: &Rgb) -> bool {
        *self == (color.0, color.1, color.2)
    }
}

// `Box` is fundamental, so `Box<Rgb>` counts as local.
impl FromStr for Box<Rgb> {
    type Err = ();
    fn from_str(text: &str) -> Result<Self, ()> {
        if text == "black" { Ok(Box::new(Rgb(0, 0, 0))) } else { Err(()) }
    }
}

assert_eq!(Rgb(255, 128, 0).to_string(), "#ff8000");
assert_eq!(u32::from(Rgb(1, 2, 3)), 0x010203);
assert!((1, 2, 3) == Rgb(1, 2, 3));
assert!("black".parse::<Box<Rgb>>().is_ok());
```

And two impls that the rule refuses:

```rust,compile_fail,E0117
use std::fmt;

struct Rgb(u8, u8, u8);

// `Option` is foreign and not fundamental. The local type inside it
// does not count.
impl fmt::Display for Option<Rgb> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("?")
    }
}
```

```text
error[E0117]: only traits defined in the current crate can be implemented for types defined outside of the crate
  |
7 | impl fmt::Display for Option<Rgb> {
  | ^^^^^^^^^^^^^^^^^^^^^^-----------
  |                       |
  |                       `Option` is not defined in the current crate
  |
  = note: impl doesn't have any local type before any uncovered type parameters
  = note: for more information see https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules
  = note: define and implement a trait or new type instead
```

```rust,compile_fail,E0210
use std::ops::Add;

struct Rgb(u8, u8, u8);

// T0 is a bare type parameter, and it stands before the local type.
impl<T> Add<Rgb> for T {
    type Output = Rgb;
    fn add(self, rhs: Rgb) -> Rgb {
        rhs
    }
}
```

```text
error[E0210]: type parameter `T` must be covered by another type when it appears before the first local type (`Rgb`)
  |
6 | impl<T> Add<Rgb> for T {
  |      ^ uncovered type parameter
  |
  = note: implementing a foreign trait is only possible if at least one of the types for which it is implemented is local,
          and no uncovered type parameters appear before that first local type
  = note: in this case, 'before' refers to the following order: `impl<..> ForeignTrait<T1, ..., Tn> for T0`,
          where `T0` is the first and `Tn` is the last
```

Why the second condition? `T` could be a type of some other crate, and that
crate is allowed to write `impl Add<Rgb> for TheirType` (their type is `T0`,
local to them). The two impls would collide in a program that uses both.

The way out of E0117 is the hint in the last line of its message: a
**newtype**. `struct Maybe(Option<Rgb>);` is a local type, and you can
implement any trait for it.

### The overlap rule and blanket impls

A **blanket impl** implements a trait for every type that meets a bound:
`impl<T: Display> Summary for T`. It is powerful, and it takes a great deal
of room. Two impls overlap when some type could match both, and the compiler
must decide that without knowing every type that will ever exist.

```rust,compile_fail,E0119
use std::fmt::Display;

trait Summary {
    fn summary(&self) -> String;
}

impl<T: Display> Summary for T {
    fn summary(&self) -> String {
        self.to_string()
    }
}

impl Summary for Vec<u8> {
    fn summary(&self) -> String {
        format!("{} bytes", self.len())
    }
}
```

```text
error[E0119]: conflicting implementations of trait `Summary` for type `Vec<u8>`
   |
 7 | impl<T: Display> Summary for T {
   | ------------------------------ first implementation here
...
13 | impl Summary for Vec<u8> {
   | ^^^^^^^^^^^^^^^^^^^^^^^^ conflicting implementation for `Vec<u8>`
   |
   = note: upstream crates may add a new impl of trait `std::fmt::Display` for type `std::vec::Vec<u8>` in future versions
```

`Vec<u8>` does not implement `Display` today. The `note:` explains why that
does not help: the standard library is allowed to add the impl in a later
version, and your crate must keep compiling when it does. The compiler may
conclude "this type does not implement that trait" only where *you* control
the answer. For a local type, you do:

```rust
use std::fmt::Display;

trait Summary {
    fn summary(&self) -> String;
}

impl<T: Display> Summary for T {
    fn summary(&self) -> String {
        self.to_string()
    }
}

struct Packet(Vec<u8>); // local, and it does not implement Display

impl Summary for Packet {
    fn summary(&self) -> String {
        format!("{} bytes", self.0.len())
    }
}

assert_eq!(5.summary(), "5");
assert_eq!(Packet(vec![1, 2]).summary(), "2 bytes");
```

This compiles, and it has a price: `Packet` can now never implement
`Display`. The day somebody adds that impl, the two `Summary` impls overlap.
The same is true for every user of your crate and every type of theirs.

Two more facts about overlap:

- There is no "more specific impl wins" on stable Rust. Specialization is
  not stable.
- Different associated types do not make two impls disjoint. The compiler
  rejects `impl<I: Iterator<Item = u8>> Tr for I` together with
  `impl<I: Iterator<Item = u16>> Tr for I` with E0119, although no type can
  match both.

### What this means for the design of a library

Coherence turns some choices into promises that you cannot take back.

- **Implement the standard traits for your types yourself**, generously:
  `Debug`, `Clone`, `PartialEq`, `Hash`, `Default`, `Display`, `From`. Your
  users cannot add them later; the orphan rule stops them.
- **Write the forwarding impls for `&T` and `Box<T>` yourself**, with
  `T: ?Sized`, if your trait should work through pointers. For your users,
  the trait is foreign and so are `&` and `Box` around a foreign type.
- **Think before you publish a blanket impl.** It forbids every other impl
  that might overlap, in your crate and in every crate that uses yours.
  Adding one later is a breaking change. A blanket impl is right when the
  trait exists *only* through it, as an extension trait does:
  `impl<I: Iterator> MyIteratorExt for I`.
- **Offer a wrapper type when you want "use that other trait"**:
  `struct Shown<T>(T)` with `impl<T: Display> Summary for Shown<T>` is
  coherent with everything, and each use is visible in the code.
- **Use a newtype** when you need a foreign trait on a foreign type.

### Which impl does a method call find?

Coherence guarantees one impl per trait and type. A method call still has to
pick a *type*, because of auto-referencing. For `value.method()` the
compiler tries a list of receiver types in order: the type of `value`, then
`&` of it, then `&mut` of it; then the same three for the type that `value`
dereferences to, and so on. (At the very end, an array is also tried as a
slice.) At each step, inherent methods are tried before trait methods. The
first match wins.

So if a trait is implemented for both `u8` and `&u8`, and `r` is a `&u8`,
then `r.method()` (with `&self`) finds the impl for `u8`: at the first step
the receiver type is `&u8`, and the `u8` impl's method takes exactly a
`&u8`. Inside a generic function the type parameter is fixed first, and the
answer can differ. The predictions of this lesson make you work through
such cases.

### Mistakes people really make

**A convenient blanket impl in a public crate.** It works for a long time,
and then a user cannot implement your trait for their type because it also
implements `Display`, or you cannot add an impl for `Vec<T>`.

**A missing `?Sized` on a forwarding impl.** `impl<T: Tr> Tr for Box<T>`
covers `Box<String>` and does not cover `Box<dyn Tr>` or `Box<str>`.

**A generic conversion.** `impl<T: Into<f64>> From<T> for Celsius` looks
harmless. The standard library has `impl<T> From<T> for T`. The two overlap
as soon as `Celsius` itself can be converted into `f64`.

### When to use which

- A plain impl per type: the default. It never surprises anybody.
- A blanket impl: for extension traits, and for traits whose whole meaning
  is "everything that has X also has Y".
- A wrapper type (`Shown<T>`, `Reverse<T>`, `Wrapping<T>`): to give existing
  types a second behavior for a trait, one value at a time.
- A newtype around a foreign type: to get past the orphan rule.

### Read more

- [Orphan rules](https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules) and [trait implementation coherence](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence) in the Rust Reference
- [Method call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html) in the Rust Reference
- Error codes [E0117](https://doc.rust-lang.org/error_codes/E0117.html), [E0119](https://doc.rust-lang.org/error_codes/E0119.html) and [E0210](https://doc.rust-lang.org/error_codes/E0210.html)

## Lesson 09 · Capstone: units of measure

### The idea

This section assumes Lessons 01 to 08.

The capstone asks you to build a small library in which a number carries its
unit in its type. Adding metres to seconds must not compile. Converting
kilometres to metres must work. Converting kilometres to seconds must not
compile. No unit may cost a byte at run time. You design every type
yourself. This section gives you one new technique and a way to work.

### An associated type as a fact that the compiler can compare

A trait bound says "this type has this ability". A bound with an associated
type *equality* says more: "and this associated type is exactly that one".

```rust
// Both arguments can be iterated, and their items have the SAME type.
fn merge<A, B>(first: A, second: B) -> Vec<A::Item>
where
    A: IntoIterator,
    B: IntoIterator<Item = A::Item>,
{
    first.into_iter().chain(second).collect()
}

assert_eq!(merge(vec![1, 2], [3]), [1, 2, 3]);
```

`A` and `B` may be different collection types. The bound
`Item = A::Item` relates them through one associated type. When the relation
does not hold, the error names both sides:

```text
error[E0271]: type mismatch resolving `<[&str; 1] as IntoIterator>::Item == {integer}`
   |
11 |     merge(vec![1, 2], ["three"]);
   |     -----             ^^^^^^^^^ expected integer, found `&str`
   |     |
   |     required by a bound introduced by this call
   |
note: required by a bound in `merge`
   |
 5 |     B: IntoIterator<Item = A::Item>,
   |                     ^^^^^^^^^^^^^^ required by this bound in `merge`
```

Think of an associated type as a function from types to types that the
compiler evaluates. `IntoIterator::Item` maps a collection to its item type.
In the capstone you need a function from a *unit* to its *dimension*, and a
bound that says "these two units have the same dimension". A type can also
be built from other types: if `Pair<A, B>` is a type for every `A` and `B`,
an impl for `Pair<A, B>` can compute its associated type from those of `A`
and `B`.

### A way to work

1. Read every program in `tests/cases/l09/` first. The ones that must fail
   tell you what the design has to make impossible.
2. Write down the types before any method: the value type, the marker types,
   the trait, and the marker field. For each parameter, answer the three
   questions of Lesson 04.
3. Make the simplest case pass (`new`, `value`, `+`), then conversions, then
   derived units.
4. When an impl is rejected, find out which rule rejected it: the orphan
   rule (E0117, E0210) or the overlap rule (E0119). Lesson 08 has the fix
   for each.
5. Check the cost at the end: sizes, auto traits, and `Copy`.

### Mistakes people really make

**Deriving on the value type.** `#[derive(Clone, Copy, PartialEq)]` adds
bounds on the unit parameter (Lesson 04), and marker types do not meet them.

**Putting the factor in a field.** A conversion factor that is stored in
each value doubles the size and makes the unit a run-time property. It
belongs in an associated constant of the unit's trait.

**Making addition "helpful".** An `Add` impl that converts the right-hand
side silently hides exactly the mistakes that this library exists to catch.

### Read more

- [Associated types](https://doc.rust-lang.org/reference/items/associated-items.html#associated-types) and [associated constants](https://doc.rust-lang.org/reference/items/associated-items.html#associated-constants) in the Rust Reference
- [Operator traits in `std::ops`](https://doc.rust-lang.org/std/ops/index.html)
- [Error code E0271](https://doc.rust-lang.org/error_codes/E0271.html)
