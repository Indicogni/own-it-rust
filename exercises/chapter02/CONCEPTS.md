# Chapter 2 concepts — A first tour

[Chapter guide](README.md) · [Hints](HINTS.md) · [Your notes](NOTES.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every Rust code block in this file was compiled
and run with the course toolchain (Rust 1.99.0, edition 2024), and every
compiler message was copied from that compiler. Long messages are shortened: a
line with `...` marks a cut, and the closing lines of a message are usually
left out. In panic messages, the number in parentheses after the thread name
is a thread id; yours will be different.

## What you will be able to do

After this chapter you can:

- declare variables, choose between `mut` and shadowing, and pick a number
  type for a job;
- write functions, and say what a function returns by looking at its last
  line;
- steer a program with `if`, `while`, `loop` and `for` over a range;
- build and format text, and tell `&str` from `String`;
- collect values in a vector and walk through it;
- define your own types with `struct` and `enum`, and take them apart with
  `match`;
- handle "no value" and "it failed" with `Option` and `Result`;
- write tests for your own code;
- write a small program of about 80 lines from a description, with your own
  design.

This chapter is a tour. It shows each part of the language at first contact,
with enough depth to use it and to read other people's code. Chapters 3 to 24
come back to every topic and go to the bottom of it.

## Before you start

Chapter 1: you can run one lesson with
`cargo test --test chapter02 l01_`, read the report of a failed test, read a
compiler message, and you know the loop of predict, implement and explain.
From other languages you know what a variable, a function, an `if` and a loop
are.

## Time

These are estimates for a first pass. They include reading the
section, the predictions, the code, and your notes.

| Lesson | Estimate |
| --- | --- |
| 01 · Variables, mutability and basic types | 40 minutes |
| 02 · Functions, expressions and statements | 40 minutes |
| 03 · if, loops and ranges | 45 minutes |
| 04 · Text: &str, String and formatting | 50 minutes |
| 05 · Vectors | 45 minutes |
| 06 · Structs and methods | 45 minutes |
| 07 · Enums and match | 45 minutes |
| 08 · Option and Result | 55 minutes |
| 09 · Writing your own tests | 45 minutes |
| Compiler drills | 40 minutes |
| 10 · Capstone: a text statistics tool | 90 minutes |
| C01 · Code review | 40 minutes |

That is about ten hours. Plan several sittings of 45 to 60 minutes.

## New in this chapter: clippy

`cargo clippy --test chapter02` runs a second
checker over your code. It knows several hundred patterns that compile but
can be written in a simpler or safer way, and it prints its advice in the
same form as compiler warnings. Run it after each lesson and repair the
warnings in code that you wrote. The warnings in `c01_code_review.rs` are
there on purpose; leave them until you do that lesson.

## Words used in this chapter

- **Value**: a piece of data, such as `42`, `true` or `"tea"`.
- **Literal**: a value that is written out in the source text itself: `42`,
  `2.5`, `'A'`, `"tea"`.
- **Type**: the kind of a value. The type decides which operations are
  allowed. Every value has exactly one type, and the compiler knows it before
  the program runs.
- **Variable**: a name for a value.
- **Expression**: a piece of code that produces a value, such as `2 + 3`.
- **Statement**: a piece of code that does something and produces no value,
  such as `let x = 5;`.
- **Block**: code between `{` and `}`.

## Lesson 01 · Variables, mutability and basic types

### The idea

`let` gives a name to a value:

```rust
fn main() {
    let width = 12;
    let height = 5;
    let area = width * height;
    assert_eq!(area, 60);
}
```

A variable made with `let` alone **cannot be changed**. Rust calls it
*immutable*. If a variable must change, you say so when you declare it, with
`mut` (short for "mutable"):

```rust
fn main() {
    let mut score = 10;
    score = score + 5;
    score += 1; // short for: score = score + 1
    assert_eq!(score, 16);
}
```

Why this way round? Most variables in a program never change after their
first value. When only the changing ones are marked, a reader sees at once
which variables need attention, and the compiler can catch an accidental
change.

### Shadowing

You may declare a new variable with a name that is already in use. The new
variable **shadows** the old one: from that line on, the name means the new
variable.

```rust
fn main() {
    let price = 1999; // cents
    let price = price / 100; // whole units: a new variable
    assert_eq!(price, 19);

    {
        let price = price + 1; // a third one, only inside this block
        assert_eq!(price, 20);
    }
    assert_eq!(price, 19); // the block's variable is gone
}
```

Shadowing is not the same as changing. Each `let` makes a new variable, so no
`mut` is needed, and the new variable may even have another type. A variable
that is declared inside a block exists only until the end of that block.
After the block, the name means the outer variable again.

### Types by use

Every value has a type. You rarely write it, because the compiler works it
out from the value. You can write it after a colon: `let width: u32 = 12;`.
These are the types you need first:

| Type | Holds | Example | Use it for |
| --- | --- | --- | --- |
| `i32` | whole numbers, negative or positive (about ±2 thousand million) | `-7` | whole numbers in general; a number without other clues is an `i32` |
| `i64` | the same, with a far larger range | `9_000_000_000` | large amounts |
| `u32` | whole numbers from 0 up | `42` | amounts that are never negative |
| `usize` | whole numbers from 0 up | `3` | sizes and positions in lists; `len()` returns it |
| `f64` | numbers with a fraction ("floats") | `2.5` | measurements, averages |
| `bool` | `true` or `false` | `true` | conditions |
| `char` | one character, in single quotes | `'A'` | single letters and signs |

The number in a type's name is its size in bits. An `i32` takes 32 bits,
which is 4 bytes; an `i64` and an `f64` take 8 bytes. The same pattern gives
more types that later chapters use: `u8`, `u16` and `u64` (never negative),
and `i8` and `i16` (with a sign). A `usize` is as wide as an address on your
computer, which is 8 bytes on today's desktop machines.

Because the size is fixed, every whole-number type has a smallest and a
largest value. They are written `i32::MIN` and `i32::MAX`, and the same for
the other types. A result outside that range is called an **overflow**. In a
normal build the program panics there, with a message such as `attempt to
add with overflow`. You met the same thing for subtraction in the capstone
of Chapter 1. Chapter 3 covers overflow in depth.

An underscore may be written inside a number to make it readable:
`1_000_000`. A type's name may be written directly after a number, to say
which type the number has: `7_u32`, `0.5_f64`.

Rust **never converts between number types by itself**. To calculate with an
`i32` and an `f64`, you convert one of them yourself. `f64::from(n)` turns an
`i32` or a `u32` into an `f64`.

Division shows why the type matters. Between whole numbers, `/` drops the
rest. Between floats, it keeps the fraction:

```rust
fn main() {
    assert_eq!(7 / 2, 3);
    assert_eq!(7.0 / 2.0, 3.5);

    let apples = 7;
    let people = 2;
    let each = f64::from(apples) / f64::from(people);
    assert_eq!(each, 3.5);
}
```

Characters can be compared: `'a' < 'b'` is true, because letters are ordered
as in the alphabet. `char` also has methods that answer common questions,
such as `c.is_ascii_digit()`.

### A worked example

A step counter reports a walk. Read it line by line and say for each variable
whether it changes, and which type it has.

```rust
fn main() {
    let steps = 8_000; // i32
    let stride_cm = 75; // i32
    let distance_cm = steps * stride_cm;

    // A new variable with the same name and another type: shadowing.
    let distance_km = f64::from(distance_cm) / 100_000.0;
    assert_eq!(distance_km, 6.0);

    // A running total changes, so it is `mut`.
    let mut week_km = 0.0;
    week_km += distance_km;
    week_km += 2.5;
    assert_eq!(week_km, 8.5);

    let goal_reached: bool = week_km >= 8.0;
    assert!(goal_reached);

    let grade: char = if goal_reached { 'A' } else { 'B' };
    assert_eq!(grade, 'A');
}
```

```text
 name           type   changes?
 steps          i32    no
 stride_cm      i32    no
 distance_cm    i32    no
 distance_km    f64    no
 week_km        f64    yes (mut)
 goal_reached   bool   no
 grade          char   no
```

### Mistakes people really make

**Changing a variable that is not `mut`.**

```rust,compile_fail,E0384
fn main() {
    let total = 1;
    total += 2;
    println!("{total}");
}
```

```text
error[E0384]: cannot assign twice to immutable variable `total`
 --> src/main.rs:3:5
  |
2 |     let total = 1;
  |         ----- first assignment to `total`
3 |     total += 2;
  |     ^^^^^^^^^^ cannot assign twice to immutable variable
  |
help: consider making this binding mutable
  |
2 |     let mut total = 1;
  |         +++
```

Fix: `let mut total`. ("Binding" is the compiler's word for a variable.)

**Mixing number types.**

```rust,compile_fail,E0277
fn main() {
    let items: i32 = 3;
    let price: f64 = 2.5;
    let cost = items * price;
    println!("{cost}");
}
```

```text
error[E0277]: cannot multiply `i32` by `f64`
 --> src/main.rs:4:22
  |
4 |     let cost = items * price;
  |                      ^ no implementation for `i32 * f64`
  ...
```

Fix: convert the whole number, `f64::from(items) * price`.

The real message is longer than this. After the lines shown here it lists
many other types that "implement trait `Mul`". A **trait** is a named
ability of a type, here the ability to be multiplied; Chapter 11 is about
traits. For now, skip that list: the headline and the label already say what
is wrong. If the source code of the standard library is installed on your
computer (rust-analyzer asks for it), such a list also shows lines from
that source, and the line numbers on the left take more room. That is the
same message.

When the numbers have no written type, as in `2.5 * 4`, the same message
names `{float}` and `{integer}`. That is how the compiler prints "some float
type" and "some whole-number type" while it has not yet decided which one.

**A value that the type cannot hold.**

```rust,compile_fail,E0600
fn main() {
    let floor: u32 = -1;
    println!("{floor}");
}
```

```text
error[E0600]: cannot apply unary operator `-` to type `u32`
 --> src/main.rs:2:22
  |
2 |     let floor: u32 = -1;
  |                      ^^ cannot apply unary operator `-`
  |
  = note: unsigned values cannot be negated
```

A `u32` is never negative ("unsigned" means "without a sign"). Use `i32`
when a value can go below zero.

### When to use which

- **`mut`**: one thing that changes over time, such as a counter or a running
  total.
- **Shadowing**: a value that you transform in steps, where the old form is
  not needed again, or where the type changes.
- **A new name**: when you need both the old and the new value.

### Read more

- [The Rust Reference: `let` statements](https://doc.rust-lang.org/reference/statements.html#let-statements)
- [The Rust Reference: Variables](https://doc.rust-lang.org/reference/variables.html)
- [Primitive types in the standard library](https://doc.rust-lang.org/std/index.html#primitives)

## Lesson 02 · Functions, expressions and statements

### The idea

A function has a name, parameters, a return type and a body:

```rust
fn area(width: u32, height: u32) -> u32 {
    width * height
}

fn main() {
    assert_eq!(area(3, 4), 12);
}
```

In a function's first line, the **signature**, every type is written out.
The compiler does not guess here. The signature is a promise to everyone who
calls the function, and it should not change because someone edits the body.
Inside the body, the compiler works out the types of local variables.

To understand what a function returns, you need the difference between two
kinds of code.

- An **expression** produces a value. `2 + 3` is an expression. So are
  `area(3, 4)`, `width * height`, and also `if` and blocks, as you will see.
- A **statement** does something and produces no value. `let x = 5;` is a
  statement. An expression with a semicolon after it is also a statement:
  the semicolon throws the value away.

A **block** is a pair of braces with statements inside, and at the end,
optionally, one expression without a semicolon. The value of that last
expression is the value of the whole block:

```rust
fn main() {
    let total = {
        let price = 250;
        let tax = price / 10;
        price + tax
    };
    assert_eq!(total, 275);
}
```

A function body is a block. So **a function returns the value of the last
expression in its body**, the one without a semicolon. That is all there is
to it. Rust also has the word `return`, which leaves a function at once; it
is used for leaving early, not on the last line.

### Functions that return nothing

A function without `->` returns "nothing". Rust has a type for that. It is
written `()` and called **unit**; it has exactly one value, also written
`()`.

```rust
fn announce(name: &str) {
    println!("Now playing: {name}");
}

fn main() {
    announce("Intro");
}
```

You will meet `()` mostly in compiler messages. "expected `u32`, found `()`"
means: a value of type `u32` was needed here, and this code produces nothing.
The usual cause is a semicolon after the last expression.

### Two results: tuples

A **tuple** groups a fixed number of values, possibly of different types, in
parentheses. Its type is written the same way: `(u32, u32)`. A function can
return a tuple to give back two results. The caller takes the tuple apart
with a pattern, or reads one part with `.0`, `.1`:

```rust
/// Split a number of seconds into minutes and the seconds left over.
fn minutes_and_seconds(total: u32) -> (u32, u32) {
    (total / 60, total % 60)
}

fn main() {
    let (minutes, seconds) = minutes_and_seconds(200);
    assert_eq!(minutes, 3);
    assert_eq!(seconds, 20);

    let pair = minutes_and_seconds(59);
    assert_eq!(pair.0, 0);
    assert_eq!(pair.1, 59);
}
```

### A worked example

Small functions that call each other. The inner call runs first, and its
value becomes the argument of the outer call.

```rust
fn minutes_and_seconds(total: u32) -> (u32, u32) {
    (total / 60, total % 60)
}

/// The total length of three songs, in seconds.
fn album_seconds(first: u32, second: u32, third: u32) -> u32 {
    first + second + third
}

/// True when an album is longer than ten minutes.
fn is_long(first: u32, second: u32, third: u32) -> bool {
    let (minutes, _seconds) = minutes_and_seconds(album_seconds(first, second, third));
    minutes >= 10
}

fn main() {
    assert_eq!(album_seconds(200, 180, 240), 620);
    assert!(is_long(200, 180, 240));
    assert!(!is_long(100, 100, 100));
}
```

```text
 is_long(200, 180, 240)
   └─ album_seconds(200, 180, 240)      runs first, gives 620
   └─ minutes_and_seconds(620)          gives (10, 20)
   └─ minutes >= 10                     last expression: the result, true
```

A name that starts with an underscore, such as `_seconds`, tells the compiler
that you do not use this variable on purpose. Without the underscore you get
a warning about an unused variable.

### Mistakes people really make

**Forgetting the return type.** Without `->`, the function promises to return
`()`, and the body produces a number:

```rust,compile_fail,E0308
fn area(width: u32, height: u32) {
    width * height
}

fn main() {
    area(3, 4);
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:2:5
  |
1 | fn area(width: u32, height: u32) {
  |                                 - help: try adding a return type: `-> u32`
2 |     width * height
  |     ^^^^^^^^^^^^^^ expected `()`, found `u32`
```

**A semicolon on the last line.** This is the opposite mistake: the function
promises a value and the body produces `()`. Chapter 1 showed the message; it
ends with "help: remove this semicolon to return this value".

**A parameter without a type.**

```rust,compile_fail
fn square(n) -> i64 {
    n * n
}

fn main() {
    println!("{}", square(3));
}
```

```text
error: expected one of `:`, `@`, or `|`, found `)`
 --> src/main.rs:1:12
  |
1 | fn square(n) -> i64 {
  |            ^ expected one of `:`, `@`, or `|`
  |
help: if this is a parameter name, give it a type
  |
1 | fn square(n: TypeName) -> i64 {
  |            ++++++++++
```

The headline is hard to read, and the help is clear: write `n: i64`.

**An argument of the wrong type.** `square(2.5)` for a function that takes an
`i64` gives `E0308` with the label "expected `i64`, found floating-point
number". The compiler does not round for you.

### Read more

- [The Rust Reference: Functions](https://doc.rust-lang.org/reference/items/functions.html)
- [The Rust Reference: Statements and expressions](https://doc.rust-lang.org/reference/statements-and-expressions.html)
- [The Rust Reference: Block expressions](https://doc.rust-lang.org/reference/expressions/block-expr.html)
- [Tuples in the standard library](https://doc.rust-lang.org/std/primitive.tuple.html)

## Lesson 03 · if, loops and ranges

### if is an expression

`if` chooses between blocks. The condition must be a `bool`. Rust does not
treat a number or a text as "true" or "false"; you write the comparison
yourself (`count > 0`, not `count`).

Because blocks have values, an `if` with an `else` has a value too: the value
of the branch that runs. You can bind it with `let` or use it as the last
expression of a function.

```rust
fn ticket(age: u32) -> &'static str {
    if age < 6 {
        "free"
    } else if age < 18 {
        "half price"
    } else {
        "full price"
    }
}

fn main() {
    assert_eq!(ticket(4), "free");
    assert_eq!(ticket(17), "half price");
    assert_eq!(ticket(18), "full price");

    let age = 30;
    let seats = if age >= 65 { 2 } else { 1 };
    assert_eq!(seats, 1);
}
```

Two rules follow. All branches must produce the same type. And when you use
the value, there must be an `else`; otherwise the `if` would have no value
when the condition is false.

### Three loops

**`for`** runs its body once for each value of a sequence. The simplest
sequence is a **range** of numbers:

- `a..b` runs from `a` up to, but **not including**, `b`.
- `a..=b` runs from `a` up to and **including** `b`.

```rust
fn main() {
    let mut seen = Vec::new();
    for i in 0..3 {
        seen.push(i);
    }
    assert_eq!(seen, [0, 1, 2]);

    let mut sum = 0;
    for i in 1..=3 {
        sum += i;
    }
    assert_eq!(sum, 6);

    // .rev() walks a range backwards.
    let mut countdown = Vec::new();
    for i in (1..=3).rev() {
        countdown.push(i);
    }
    assert_eq!(countdown, [3, 2, 1]);
}
```

(`Vec::new()` makes an empty list and `push` adds to it. Lesson 05 is about
lists; here they only make the loops visible. A fixed list in square
brackets, such as `[0, 1, 2]`, is called an **array**. Its type is written
with the element type and the length: `[i32; 3]`.) A range whose start is not
below its end, such as `5..5`, is empty, and the body does not run at all.
The half-open form `0..n` is the common one, because it gives exactly `n`
rounds and fits positions in a list, which start at 0.

**`while`** runs its body as long as a condition is true. The condition is
checked before every round:

```rust
/// How many digits does `n` have when written in decimal?
fn digits(n: u32) -> u32 {
    let mut rest = n;
    let mut count = 1;
    while rest >= 10 {
        rest /= 10;
        count += 1;
    }
    count
}

fn main() {
    assert_eq!(digits(7), 1);
    assert_eq!(digits(10), 2);
    assert_eq!(digits(40_321), 5);
}
```

A parameter such as `n` cannot be changed, so the function copies it into a
`mut` variable first.

**`loop`** runs its body again and again, without a condition, until a
`break` ends it. Use it when the natural place to stop is in the middle of
the body:

```rust
/// You save `monthly` every month. After how many months do you have at
/// least `goal`? `monthly` is at least 1.
fn months_to_save(goal: u32, monthly: u32) -> u32 {
    let mut saved = 0;
    let mut months = 0;
    loop {
        if saved >= goal {
            break;
        }
        saved += monthly;
        months += 1;
    }
    months
}

fn main() {
    assert_eq!(months_to_save(1000, 150), 7);
    assert_eq!(months_to_save(0, 150), 0);
    assert_eq!(months_to_save(300, 150), 2);
}
```

Two words steer a loop from inside. **`break`** leaves the loop at once.
**`continue`** skips the rest of this round and starts the next one.

```rust
fn main() {
    let mut picked = Vec::new();
    for n in 1..100 {
        if n % 3 != 0 {
            continue; // not a multiple of 3: next round
        }
        if n > 12 {
            break; // far enough: leave the loop
        }
        picked.push(n);
    }
    assert_eq!(picked, [3, 6, 9, 12]);
}
```

### Tracing a loop by hand

When a loop does not do what you expect, write down the variables round by
round. For `digits(40_321)`:

```text
 before the loop    rest = 40321   count = 1
 round 1            rest = 4032    count = 2
 round 2            rest = 403     count = 3
 round 3            rest = 40      count = 4
 round 4            rest = 4       count = 5
 check: 4 >= 10 is false, the loop ends, the result is 5
```

This takes a minute and finds most loop mistakes: a start value that is one
off, a condition that should be `>=` and is `>`, a variable that never
changes.

### Mistakes people really make

**An `if` without `else` whose value is used.**

```rust,compile_fail,E0317
fn main() {
    let speed = 70;
    let verdict = if speed > 50 { "too fast" };
    println!("{verdict}");
}
```

```text
error[E0317]: `if` may be missing an `else` clause
 --> src/main.rs:3:19
  |
3 |     let verdict = if speed > 50 { "too fast" };
  |                   ^^^^^^^^^^^^^^^^----------^^
  |                   |               |
  |                   |               found here
  |                   expected `&str`, found `()`
  |
  = note: `if` expressions without `else` evaluate to `()`
```

**A number as a condition.**

```rust,compile_fail,E0308
fn main() {
    let errors = 3;
    if errors {
        println!("there were errors");
    }
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:3:8
  |
3 |     if errors {
  |        ^^^^^^ expected `bool`, found integer
```

Fix: `if errors > 0`.

**One round too few or too many.** `1..n` stops before `n`. To include `n`,
write `1..=n`. No compiler message helps here; the program runs and gives a
wrong result. Trace the first and the last round by hand.

**A loop that never ends.** If the body of a `while` or `loop` never changes
the variable in the condition, the loop runs for ever, and the program seems
to hang. Stop it with Ctrl+C in the terminal. In this lesson the tests give
up after five seconds and tell you which call did not finish. (The rest of
that message, about locks and channels, belongs to later chapters.) If the
endless loop adds to a counter in every round, you may see another message
first: `attempt to add with overflow`. The counter then grew past the
largest value of its type before the five seconds were over. The cause is
the same loop.

### When to use which

- **`for`**: you know the sequence before the loop starts (a range, a list).
  It cannot run for ever, and it cannot be off by one at the end unless the
  range is wrong. Prefer it.
- **`while`**: you repeat as long as a condition holds, and you do not know
  the number of rounds.
- **`loop`**: the exit is in the middle of the body, or the loop should run
  until something happens.

### Read more

- [The Rust Reference: `if` expressions](https://doc.rust-lang.org/reference/expressions/if-expr.html)
- [The Rust Reference: Loop expressions](https://doc.rust-lang.org/reference/expressions/loop-expr.html)
- [The Rust Reference: Range expressions](https://doc.rust-lang.org/reference/expressions/range-expr.html)

## Lesson 04 · Text: &str, String and formatting

### Two types for text

Rust has two main types for text, and you will use both every day.

- **`String`** owns its text. The characters are stored in memory that
  belongs to this one `String`. A `String` can grow and change.
- **`&str`** (say "string slice") is a **view** of text that is stored
  somewhere else. It does not own the text and cannot make it longer.

A literal such as `"tea"` is a `&str`. Its text is stored inside the program
itself and is there for the whole run. That is what the `'static` in
`&'static str` meant in Chapter 1.

```text
 let label = "tea";                   let mut order = String::from("tea");
                                      order.push_str(" with milk");

 label: &str ──► t e a                order: String ──► t e a   w i t h   m i l k
        (stored in the program;              (memory that belongs to `order`;
         cannot change)                       it grew when text was added)
```

You move between the two like this:

```rust
fn main() {
    // From &str to String: make an owned copy of the text.
    let owned: String = String::from("tea");
    let also_owned: String = "tea".to_string();
    assert_eq!(owned, also_owned);

    // From String to &str: lend the text with `&`. Nothing is copied.
    let view: &str = &owned;
    assert_eq!(view, "tea");
}
```

The `&` means "lend": the function or variable that receives a `&str` may
look at the text, and the `String` stays where it is. Chapter 5 is about
lending. For now, two rules of thumb are enough:

- A function that only **reads** text takes `&str`. Then callers can pass a
  literal, or a `String` with `&` in front.
- A function that **makes new** text returns `String`.

### Building text

```rust
fn main() {
    let mut line = String::new(); // empty
    line.push_str("tea");
    line.push(' '); // one char, in single quotes
    line.push_str("time");
    assert_eq!(line, "tea time");
    assert_eq!(line.len(), 8);
}
```

`len()` counts **bytes**, not characters. For English letters and digits
that is the same number. A letter such as `é` takes two bytes. When you need
the number of characters, use `text.chars().count()`. Chapter 3 explains why.

### Formatting

`format!` builds a `String` from a pattern. `println!` uses the same patterns
and prints the result with a line end. Inside the pattern, each pair of
braces is a place for one value.

```rust
fn main() {
    let item = "pen";
    let price = 1.5;
    let count = 3;

    // A variable's name inside the braces, or values after the pattern.
    assert_eq!(format!("{count} x {item}"), "3 x pen");
    assert_eq!(format!("{} x {}", count, item), "3 x pen");

    // After a colon: how to print the value.
    assert_eq!(format!("{price:.2}"), "1.50"); // two decimals
    assert_eq!(format!("{:.2}", 2.468), "2.47"); // rounded, not cut off
    assert_eq!(format!("{count:>5}"), "    3"); // width 5, to the right
    assert_eq!(format!("{item:<5}|"), "pen  |"); // width 5, to the left
    assert_eq!(format!("{count:03}"), "003"); // width 3, filled with zeros

    // {:?} is the "debug" form, made for programmers.
    assert_eq!(format!("{:?}", item), "\"pen\"");
}
```

`{}` prints a value in the form meant for users. `{:?}` prints it in a form
meant for programmers: a text appears with its quotation marks, and lists
and your own types can be printed this way too (lessons 05 and 06). Test
reports use `{:?}`, which is why a failed `assert_eq!` on texts shows quotes.

### Useful methods

All of these work on a `&str`, and also on a `String`:

```rust
fn main() {
    let text = "  Rust is fun  ";
    assert_eq!(text.trim(), "Rust is fun"); // without spaces at the ends
    assert!(text.contains("is"));
    assert!(text.trim().starts_with("Rust"));
    assert_eq!(text.to_uppercase(), "  RUST IS FUN  "); // a new String
    assert!(!text.is_empty());

    // Split into words, at any white space.
    let mut words = Vec::new();
    for word in text.split_whitespace() {
        words.push(word);
    }
    assert_eq!(words, ["Rust", "is", "fun"]);
}
```

### A worked example

A function that reads two texts and a number and makes a new text, and a
function that builds its result in a loop.

```rust
/// One line of a receipt, such as "  3 x pen        4.50".
fn receipt_line(item: &str, count: u32, unit_price: f64) -> String {
    let total = f64::from(count) * unit_price;
    format!("{count:>3} x {item:<8}{total:>7.2}")
}

/// The title with a line of `=` signs of the same length under it.
fn underlined(title: &str) -> String {
    let mut result = String::from(title);
    result.push('\n'); // a line end
    for _ in 0..title.chars().count() {
        result.push('=');
    }
    result
}

fn main() {
    assert_eq!(receipt_line("pen", 3, 1.5), "  3 x pen        4.50");

    let heading = String::from("Receipt");
    // `&heading` lends the String to a `&str` parameter.
    assert_eq!(underlined(&heading), "Receipt\n=======");
    // The caller still has its String.
    assert_eq!(heading, "Receipt");
}
```

`{total:>7.2}` combines a width and a number of decimals. `for _ in ...`
repeats the body without using the counter.

### Mistakes people really make

**A literal where a `String` is required.**

```rust,compile_fail,E0308
fn main() {
    let city: String = "Oslo";
    println!("{city}");
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:2:24
  |
2 |     let city: String = "Oslo";
  |               ------   ^^^^^^ expected `String`, found `&str`
  |               |
  |               expected due to this
  |
help: try using a conversion method
  |
2 |     let city: String = "Oslo".to_string();
  |                              ++++++++++++
```

**A `String` where a `&str` is required.** The message is the mirror image,
"expected `&str`, found `String`", and its help says "consider borrowing
here": write `&city`.

**Joining two texts with `+`.**

```rust,compile_fail,E0369
fn main() {
    let first = "night";
    let second = "fall";
    let word = first + second;
    println!("{word}");
}
```

```text
error[E0369]: cannot add `&str` to `&str`
 --> src/main.rs:4:22
  |
4 |     let word = first + second;
  |                ----- ^ ------ &str
  |                |     |
  |                |     `+` cannot be used to concatenate two `&str` strings
  |                &str
  |
  = note: string concatenation requires an owned `String` on the left
```

Fix: `format!("{first}{second}")`. It is the clearest way to join texts.

**Asking for a character by position.** `word[0]` does not compile
(`E0277`, "the type `str` cannot be indexed by `{integer}`"). A character can
take more than one byte, so position 0 has no simple meaning. Walk the
characters with `for c in word.chars()`.

### Read more

- [`String`](https://doc.rust-lang.org/std/string/struct.String.html) and
  [`str`](https://doc.rust-lang.org/std/primitive.str.html) in the standard
  library.
- [`std::fmt`](https://doc.rust-lang.org/std/fmt/index.html) lists everything
  that can stand between the braces.

## Lesson 05 · Vectors: a list that can grow

### The idea

A **vector**, written `Vec<T>`, is a list of values of one type `T`. It can
grow and shrink while the program runs. `Vec<i32>` is a list of `i32`
values; `Vec<String>` is a list of texts. A type that is written with such a
placeholder for another type is called **generic**.

```rust
fn main() {
    let mut scores: Vec<i32> = Vec::new(); // empty
    scores.push(70);
    scores.push(85);
    scores.push(90);
    assert_eq!(scores.len(), 3);
    assert_eq!(scores[0], 70); // positions start at 0
    assert_eq!(scores[2], 90); // the last position is len() - 1

    scores[1] = 88; // change one element
    assert_eq!(scores, [70, 88, 90]);

    // The vec! macro makes a vector with given elements.
    let primes = vec![2, 3, 5, 7];
    assert_eq!(primes.len(), 4);
    assert!(!primes.is_empty());

    // `vec![value; count]` repeats one value.
    let zeros = vec![0; 3];
    assert_eq!(zeros, [0, 0, 0]);
}
```

A position in a list is called an **index**. Indexes have the type `usize`
and start at 0, so a vector with 3 elements has the indexes 0, 1 and 2.
Reading an index that does not exist is a mistake that the compiler cannot
see, because the length is only known when the program runs. The program
panics at that line:

```text
thread 'main' (123627027) panicked at src/main.rs:5:32:
index out of bounds: the len is 3 but the index is 3
```

`push` and assignment to an element change the vector, so the variable must
be `mut`.

### Walking through a vector

A `for` loop can run over the elements directly. Write `&` in front of the
vector: the loop then **looks at** each element and the vector is still
yours afterwards.

```rust
fn main() {
    let scores = vec![70, 85, 90];
    let mut sum = 0;
    for score in &scores {
        sum += score;
    }
    assert_eq!(sum, 245);
    assert_eq!(scores.len(), 3); // still usable
}
```

Prefer this form to a loop over indexes (`for i in 0..scores.len()`). Without
an index, no index can be wrong.

In such a loop, `score` is not an `i32`. It is a `&i32`: a **reference**, a
value that points at an element inside the vector. Arithmetic works through
a reference (`sum += score` is fine). A comparison needs both sides to have
the same type, so there you write `*score`. The star means "the value that
this reference points at". You also write the star when you want to store
the number itself in another place, as in `best.push(*score)`:

```rust
fn main() {
    let scores = vec![70, 85, 90];
    let mut high = 0;
    let mut best = Vec::new();
    for score in &scores {
        if *score >= 85 {
            high += 1;
            best.push(*score);
        }
    }
    assert_eq!(high, 2);
    assert_eq!(best, [85, 90]);
}
```

### Lending a list to a function: slices

A function that only reads a list takes the parameter type `&[T]`, for
example `&[i32]`. This is a **slice**: a view of a row of elements that are
stored somewhere else, just as `&str` is a view of text. The caller lends a
vector with `&`:

```rust
fn largest(values: &[i32]) -> i32 {
    let mut best = values[0]; // the caller promises at least one element
    for value in values {
        if *value > best {
            best = *value;
        }
    }
    best
}

fn main() {
    let scores = vec![70, 85, 90, 61];
    assert_eq!(largest(&scores), 90);
    // A fixed list in square brackets (an array) can be lent the same way.
    assert_eq!(largest(&[3, -1]), 3);
    assert_eq!(scores.len(), 4);
}
```

`values` is already a reference, so the loop is written `for value in values`
without another `&`.

### A worked example

A function that builds a new vector from an old one. It needs positions,
because each result is made from two neighbours, so here a loop over indexes
is the right tool.

```rust
/// The change from each reading to the next:
/// [10, 13, 9] gives [3, -4].
fn changes(readings: &[i32]) -> Vec<i32> {
    let mut result = Vec::new();
    for i in 1..readings.len() {
        result.push(readings[i] - readings[i - 1]);
    }
    result
}

fn main() {
    assert_eq!(changes(&[10, 13, 9]), [3, -4]);
    assert_eq!(changes(&[5]), []);
    assert_eq!(changes(&[]), []);
}
```

```text
 readings:   [ 10 | 13 |  9 ]        index 0, 1, 2
 i = 1:      13 - 10 =  3
 i = 2:       9 - 13 = -4
 result:     [  3 | -4 ]
```

The loop starts at 1, so `i - 1` is never below 0. For a list with one
element or none, the range `1..len` is empty and the result is an empty
vector.

### Mistakes people really make

**Changing a vector that is not `mut`.**

```rust,compile_fail,E0596
fn main() {
    let temperatures = Vec::new();
    temperatures.push(21);
    println!("{}", temperatures.len());
}
```

```text
error[E0596]: cannot borrow `temperatures` as mutable, as it is not declared as mutable
 --> src/main.rs:3:5
  |
3 |     temperatures.push(21);
  |     ^^^^^^^^^^^^ cannot borrow as mutable
  |
help: consider changing this to be mutable
  |
2 |     let mut temperatures = Vec::new();
  |         +++
```

**Comparing a reference with a number.**

```rust,compile_fail,E0308
fn count_warm(temperatures: &[i32], limit: i32) -> usize {
    let mut count = 0;
    for t in temperatures {
        if t > limit {
            count += 1;
        }
    }
    count
}

fn main() {
    println!("{}", count_warm(&[21, 19, 23], 20));
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:4:16
  |
4 |         if t > limit {
  |                ^^^^^ expected `&i32`, found `i32`
  |
help: consider dereferencing the borrow
  |
4 |         if *t > limit {
  |            +
```

**A loop without `&` takes the vector away.** `for t in temperatures` hands
the whole vector to the loop, and it cannot be used after the loop. The
compiler reports `E0382`, "borrow of moved value", and its help says to write
`&temperatures`. Chapter 4 explains the rule behind this.

**One index too far.** `values[values.len()]` is always out of bounds; the
last element is at `values.len() - 1`. A range `0..=values.len()` makes the
same mistake in a loop.

### Read more

- [`Vec`](https://doc.rust-lang.org/std/vec/struct.Vec.html) in the standard
  library.
- [Slices](https://doc.rust-lang.org/std/primitive.slice.html) in the
  standard library.

## Lesson 06 · Structs and methods

### The idea

A **struct** is a type that you define yourself. It groups several values
under one name, and each value gets a name of its own, called a **field**.

```rust
struct Song {
    title: String,
    seconds: u32,
}

fn main() {
    let mut song = Song {
        title: String::from("Intro"),
        seconds: 95,
    };
    assert_eq!(song.title, "Intro");
    assert_eq!(song.seconds, 95);

    song.seconds += 10; // allowed because `song` is `mut`
    assert_eq!(song.seconds, 105);
}
```

The `struct` block describes the type: which fields exist and which type
each has. A value of the type is built by writing the type's name and a
value for **every** field. A field is read and changed with a dot. Type names
start with a capital letter (`Song`); variables, functions and fields are
written in small letters with underscores (`pages_read`).

A struct should own its data, so a text field is a `String`, not a `&str`.

### Methods

Functions that belong to a type are written in an `impl` block. A function
whose first parameter is `self` is a **method**: you call it with a dot on a
value.

```rust
struct Timer {
    seconds: u32,
    running: bool,
}

impl Timer {
    /// Builds a timer. No `self`: this is called as `Timer::new(...)`.
    fn new(seconds: u32) -> Timer {
        Timer {
            seconds,
            running: true,
        }
    }

    /// `&self`: this method only reads the timer.
    fn is_finished(&self) -> bool {
        self.seconds == 0
    }

    /// `&mut self`: this method changes the timer.
    fn tick(&mut self) {
        if self.seconds > 0 {
            self.seconds -= 1;
        }
        if self.seconds == 0 {
            self.running = false;
        }
    }
}

fn main() {
    let mut timer = Timer::new(2);
    assert!(!timer.is_finished());
    timer.tick();
    timer.tick();
    assert!(timer.is_finished());
    assert!(!timer.running);
    timer.tick(); // nothing happens at 0
    assert_eq!(timer.seconds, 0);
}
```

The first parameter says what the method may do with the value:

| First parameter | The method can | Called on |
| --- | --- | --- |
| `&self` | read the fields | any value of the type |
| `&mut self` | read and change the fields | a value in a `mut` variable |
| (no `self`) | nothing with an existing value; it usually builds one | the type: `Timer::new(2)` |

Inside a method, `self` is the value the method was called on, and
`self.seconds` is its field. The value before the dot in a call, such as
`timer` in `timer.tick()`, is called the **receiver**. A function without
`self` is called an
*associated function*. By convention, the one that builds a value is named
`new`.

In `Timer::new`, the line `seconds,` is a short form of `seconds: seconds`.
It is allowed when a variable has the same name as the field.

### Printing and comparing: derive

A new struct can do very little. It cannot be printed with `{:?}` and it
cannot be compared with `==`, because nobody has said what those should
mean. For the common cases you ask the compiler to write the code, with a
`derive` line above the struct:

```rust
#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let a = Point { x: 3, y: -1 };
    let b = Point { x: 3, y: -1 };

    // PartialEq: `==` compares field by field.
    assert_eq!(a, b);
    assert!(a != Point { x: 0, y: 0 });

    // Debug: `{:?}` prints the name and all fields.
    assert_eq!(format!("{:?}", a), "Point { x: 3, y: -1 }");
}
```

`Debug` gives the type a `{:?}` form. `PartialEq` gives it `==` and `!=`,
which compare all fields. `assert_eq!` needs both: `PartialEq` to compare,
and `Debug` to print the two values when they differ. That is why nearly
every struct in this course starts with `#[derive(Debug, PartialEq)]`.

### A worked example

A struct, a constructor, a reading method, a changing method, and a method
that makes a text.

```rust
#[derive(Debug, PartialEq)]
struct Account {
    owner: String,
    cents: u32,
}

impl Account {
    fn new(owner: &str) -> Account {
        Account {
            owner: owner.to_string(),
            cents: 0,
        }
    }

    fn deposit(&mut self, cents: u32) {
        self.cents += cents;
    }

    /// Takes out `cents` when there is enough money. Returns whether it did.
    fn withdraw(&mut self, cents: u32) -> bool {
        if cents <= self.cents {
            self.cents -= cents;
            true
        } else {
            false
        }
    }

    fn statement(&self) -> String {
        format!("{}: {}.{:02}", self.owner, self.cents / 100, self.cents % 100)
    }
}

fn main() {
    let mut account = Account::new("Ada");
    account.deposit(1250);
    assert!(account.withdraw(245));
    assert!(!account.withdraw(5000)); // not enough: nothing changes
    assert_eq!(account.statement(), "Ada: 10.05");
    assert_eq!(
        account,
        Account {
            owner: String::from("Ada"),
            cents: 1005
        }
    );
}
```

```text
 account: Account
 ├── owner: String ──► A d a
 └── cents: u32        1005
```

`withdraw` checks before it subtracts. A `u32` cannot go below zero, so the
subtraction must run only when it is safe. This is the same care as in the
capstone of Chapter 1.

Addition needs the same care at the other end. A sum above `u32::MAX` is an
overflow, so `deposit` would panic for an account that is nearly full. When
a sum can become that large, compare first, or use `saturating_add`, which
stops at the largest value:

```rust
fn main() {
    let nearly_full: u32 = u32::MAX - 1;
    assert_eq!(nearly_full.saturating_add(5), u32::MAX);
    assert_eq!(3_u32.saturating_add(5), 8);
}
```

### Mistakes people really make

**Leaving out a field.** Every field needs a value:

```rust,compile_fail,E0063
struct Song {
    title: String,
    seconds: u32,
}

fn main() {
    let song = Song {
        title: String::from("Intro"),
    };
    println!("{}", song.title);
}
```

```text
error[E0063]: missing field `seconds` in initializer of `Song`
 --> src/main.rs:7:16
  |
7 |     let song = Song {
  |                ^^^^ missing `seconds`
```

**Printing a struct without `Debug`.**

```rust,compile_fail,E0277
struct Song {
    title: String,
    seconds: u32,
}

fn main() {
    let song = Song {
        title: String::from("Intro"),
        seconds: 95,
    };
    println!("{:?}", song);
}
```

```text
error[E0277]: `Song` doesn't implement `Debug`
  --> src/main.rs:11:22
   |
11 |     println!("{:?}", song);
   |               ----   ^^^^ `Song` cannot be formatted using `{:?}` because it doesn't implement `Debug`
   |               |
   |               required by this formatting parameter
   |
   = help: the trait `Debug` is not implemented for `Song`
help: consider annotating `Song` with `#[derive(Debug)]`
```

**Calling a changing method on a value that is not `mut`.** The message is
`E0596`, "cannot borrow `song` as mutable, as it is not declared as mutable",
the same as for `push` on a vector. Fix: `let mut song`.

**Calling a method like a plain function.** `minutes(song)` for a method
`minutes(&self)` gives `E0425`, "cannot find function `minutes` in this
scope", with the help "use the `.` operator to call the method": write
`song.minutes()`.

**A field inside the braces of `format!`.** Only a plain variable name can
stand inside the braces. A field such as `self.title` is not accepted there:

```rust,compile_fail
struct Song {
    title: String,
    seconds: u32,
}

impl Song {
    fn label(&self) -> String {
        format!("{self.title}")
    }
}

fn main() {
    let song = Song {
        title: String::from("Intro"),
        seconds: 95,
    };
    println!("{} {}", song.label(), song.seconds);
}
```

```text
error: invalid format string: field access isn't supported
 --> src/main.rs:8:19
  |
8 |         format!("{self.title}")
  |                   ^^^^^^^^^^ not supported in format string
  |
  = note: consider moving this expression to a local variable and then using the local here instead
help: consider using a positional formatting argument instead
  |
8 -         format!("{self.title}")
8 +         format!("{0}", self.title)
  |
```

Fix: put the field after the pattern, as the help proposes. `{0}` means "the
first value after the pattern"; empty braces do the same here:
`format!("{}", self.title)`.

### Read more

- [The Rust Reference: Structs](https://doc.rust-lang.org/reference/items/structs.html)
- [The Rust Reference: Implementations](https://doc.rust-lang.org/reference/items/implementations.html)
- [The Rust Reference: Derive](https://doc.rust-lang.org/reference/attributes/derive.html)
- [`Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html) and
  [`PartialEq`](https://doc.rust-lang.org/std/cmp/trait.PartialEq.html) in the
  standard library.

## Lesson 07 · Enums and match

### The idea

A struct says "this value has a title **and** a length". An **enum** says
"this value is one of these things, **and nothing else**". Each of the
possibilities is called a **variant**.

```rust
#[derive(Debug, PartialEq)]
enum Weather {
    Sun,
    Rain,
    Snow,
}

fn main() {
    let today = Weather::Rain;
    assert_eq!(today, Weather::Rain);
    assert!(today != Weather::Sun);
}
```

A variant is written with the enum's name in front: `Weather::Rain`. A value
of type `Weather` is always exactly one of the three. It cannot be a fourth
thing, and it cannot be "nothing".

In many languages you would use numbers or texts for this ("0 means sun").
An enum is better in two ways. A misspelled variant does not compile. And
the compiler knows the complete list, so it can check that your code handles
every case.

### match

`match` compares a value with a list of **patterns**, from top to bottom.
The first pattern that fits is chosen, and the code after its `=>` runs. One
pattern with its code is called an **arm**. `match` is an expression: the
value of the chosen arm is the value of the whole `match`.

```rust
enum Weather {
    Sun,
    Rain,
    Snow,
}

fn advice(weather: Weather) -> &'static str {
    match weather {
        Weather::Sun => "take a hat",
        Weather::Rain => "take an umbrella",
        Weather::Snow => "take gloves",
    }
}

fn main() {
    assert_eq!(advice(Weather::Sun), "take a hat");
    assert_eq!(advice(Weather::Rain), "take an umbrella");
    assert_eq!(advice(Weather::Snow), "take gloves");
}
```

A `match` must be **exhaustive**: every possible value must fit some arm. If
you leave out `Weather::Snow`, the program does not compile. This is the
most useful property of enums. When you add a variant later, the compiler
shows you every `match` that must learn about it.

### Variants that carry data

A variant can hold values. Then the enum says not only which case it is, but
also carries the data that belongs to that case. A pattern in a `match` takes
the data out and gives it names:

```rust
enum Drink {
    Water,
    /// Coffee with a number of espresso shots.
    Coffee(u32),
    /// Tea, with the name of the blend and whether milk is added.
    Tea(String, bool),
}

/// The price in cents.
fn price(drink: Drink) -> u32 {
    match drink {
        Drink::Water => 0,
        Drink::Coffee(shots) => 200 + 50 * shots,
        Drink::Tea(_blend, true) => 230,
        Drink::Tea(_blend, false) => 200,
    }
}

fn main() {
    assert_eq!(price(Drink::Water), 0);
    assert_eq!(price(Drink::Coffee(2)), 300);
    assert_eq!(price(Drink::Tea(String::from("mint"), true)), 230);
    assert_eq!(price(Drink::Tea(String::from("mint"), false)), 200);
}
```

In the arm `Drink::Coffee(shots)`, the name `shots` is a new variable that
holds the number inside the variant. In the two tea arms, the second position
is a fixed value, `true` or `false`, so the arm fits only teas with that
value. Two arms together cover all teas.

### Patterns for numbers and pairs

`match` works on any value, not only on enums. Useful patterns:

| Pattern | Fits |
| --- | --- |
| `0` | exactly this value |
| `"tea"` | exactly this text, when the value is a `&str` |
| `1..=9` | any value in this range |
| `2 \| 3` | this value or that one |
| `_` | anything; the value is not used |
| a name, such as `other` | anything; the value gets this name |
| `(0, y)` | a pair whose first part is 0; the second part gets the name `y` |

```rust
fn describe(month: u32) -> &'static str {
    match month {
        12 | 1 | 2 => "winter",
        3..=5 => "spring",
        6..=8 => "summer",
        9..=11 => "autumn",
        _ => "not a month",
    }
}

fn main() {
    assert_eq!(describe(1), "winter");
    assert_eq!(describe(4), "spring");
    assert_eq!(describe(11), "autumn");
    assert_eq!(describe(13), "not a month");
}
```

A `u32` has about four thousand million values, so a `match` on a number
nearly always ends with `_`. Because arms are tried from top to bottom, `_`
belongs last. An arm after it can never be reached. The program still
compiles, but the compiler warns you:

```rust
fn size(n: u32) -> &'static str {
    match n {
        0 => "none",
        _ => "some",
        1 => "one", // never chosen: `_` above already matches 1
    }
}

fn main() {
    assert_eq!(size(1), "some");
}
```

```text
warning: unreachable pattern
 --> src/main.rs:5:9
  |
4 |         _ => "some",
  |         - matches any value
5 |         1 => "one", // never chosen: `_` above already matches 1
  |         ^ no value can reach this
  |
  = note: `#[warn(unreachable_patterns)]` (part of `#[warn(unused)]`) on by default
```

Treat this warning as a mistake in your code: move the `_` arm to the end.

A `match` on a text looks the same. The last arm here uses a name in place
of `_`, so that the arm can use the value:

```rust
fn opening_hours(day: &str) -> String {
    match day {
        "sat" => String::from("10 to 14"),
        "sun" => String::from("closed"),
        other => format!("9 to 18 on {other}"),
    }
}

fn main() {
    assert_eq!(opening_hours("sun"), "closed");
    assert_eq!(opening_hours("tue"), "9 to 18 on tue");
}
```

### A worked example

An enum with data, used by two functions. Each `match` lists every variant.

```rust
#[derive(Debug, PartialEq)]
enum Move {
    Forward(u32),
    Back(u32),
    Stay,
}

/// The new position on a line after one move.
fn apply(position: i64, step: Move) -> i64 {
    match step {
        Move::Forward(distance) => position + i64::from(distance),
        Move::Back(distance) => position - i64::from(distance),
        Move::Stay => position,
    }
}

/// The move that undoes this one.
fn opposite(step: Move) -> Move {
    match step {
        Move::Forward(distance) => Move::Back(distance),
        Move::Back(distance) => Move::Forward(distance),
        Move::Stay => Move::Stay,
    }
}

fn main() {
    assert_eq!(apply(10, Move::Forward(5)), 15);
    assert_eq!(apply(10, Move::Back(12)), -2);
    assert_eq!(apply(10, Move::Stay), 10);
    assert_eq!(opposite(Move::Forward(3)), Move::Back(3));
}
```

### Mistakes people really make

**A missing variant.**

```rust,compile_fail,E0004
enum Weather {
    Sun,
    Rain,
    Snow,
}

fn advice(weather: Weather) -> &'static str {
    match weather {
        Weather::Sun => "hat",
        Weather::Rain => "umbrella",
    }
}

fn main() {
    println!("{}", advice(Weather::Snow));
}
```

```text
error[E0004]: non-exhaustive patterns: `Weather::Snow` not covered
  --> src/main.rs:8:11
   |
 8 |     match weather {
   |           ^^^^^^^ pattern `Weather::Snow` not covered
   |
note: `Weather` defined here
  --> src/main.rs:1:6
   |
 1 | enum Weather {
   |      ^^^^^^^
...
 4 |     Snow,
   |     ---- not covered
```

**Writing the variant without the enum's name.** In a pattern, a bare word is
a new variable name, and a variable fits anything. So `Sun => "hat"` would
catch every weather. The compiler stops this case:

```rust,compile_fail,E0170
enum Weather {
    Sun,
    Rain,
}

fn advice(weather: Weather) -> &'static str {
    match weather {
        Sun => "hat",
        Rain => "umbrella",
    }
}

fn main() {
    println!("{}", advice(Weather::Rain));
}
```

```text
error[E0170]: pattern binding `Sun` is named the same as one of the variants of the type `Weather`
 --> src/main.rs:8:9
  |
8 |         Sun => "hat",
  |         ^^^ help: to match on the variant, qualify the path: `Weather::Sun`
```

**Using `_` on your own enum out of habit.** It compiles, and it switches off
the check that makes enums valuable: when someone adds a variant, the `_` arm
takes it silently. On your own enums, list the variants.

### When to use which

- **`if`**: one condition, or a chain of unrelated conditions.
- **`match`**: one value decides between several cases. Always for enums.
- **List every variant**: on enums that you or your team may extend.
- **`_`**: on numbers, texts and other types with very many values.

### Read more

- [The Rust Reference: Enumerations](https://doc.rust-lang.org/reference/items/enumerations.html)
- [The Rust Reference: `match` expressions](https://doc.rust-lang.org/reference/expressions/match-expr.html)
- [The Rust Reference: Patterns](https://doc.rust-lang.org/reference/patterns.html)

## Lesson 08 · Option and Result at first contact

### The problem

Some questions have no answer. What is the first element of an empty list?
What number is the text "seven"? Many languages answer with a special value
(`null`, `None`, `-1`) that looks like a normal value and is easy to forget,
or they throw an exception that the signature does not mention.

Rust has no null. Instead, the standard library defines two ordinary enums,
and a function says in its **return type** that the answer may be missing.

```rust
// How the standard library defines them (shortened):
//
// enum Option<T> {
//     Some(T),   // there is a value, and here it is
//     None,      // there is no value
// }
//
// enum Result<T, E> {
//     Ok(T),     // it worked, and here is the value
//     Err(E),    // it failed, and here is the reason
// }

fn main() {
    let found: Option<i32> = Some(7);
    let missing: Option<i32> = None;
    assert!(found.is_some());
    assert!(missing.is_none());

    let good: Result<i32, String> = Ok(7);
    let bad: Result<i32, String> = Err(String::from("no number"));
    assert!(good.is_ok());
    assert!(bad.is_err());
}
```

`T` and `E` stand for "any type". `Option<i32>` is "an `i32` or nothing".
`Result<i32, String>` is "an `i32`, or an error described by a `String`".
The four variants are used so often that you write them without the enum's
name: `Some(7)`, not `Option::Some(7)`.

The important point: an `Option<i32>` **is not an `i32`**. You cannot add 1
to it. The compiler makes you say what happens in both cases before you get
at the number. A forgotten "no value" case becomes a compile error, not a
crash in front of a user.

### Getting the value out

```rust
fn main() {
    let mut stack = vec![3];
    let top: Option<i32> = stack.pop(); // Some(3)
    let nothing: Option<i32> = stack.pop(); // None: the vector is empty

    // 1. match: handle both cases.
    let text = match top {
        Some(number) => format!("got {number}"),
        None => String::from("empty"),
    };
    assert_eq!(text, "got 3");

    // 2. if let: do something only in the Some case.
    let mut seen = 0;
    if let Some(number) = top {
        seen = number;
    }
    assert_eq!(seen, 3);

    // 3. unwrap_or: the value, or a spare value.
    assert_eq!(top.unwrap_or(0), 3);
    assert_eq!(nothing.unwrap_or(0), 0);

    // 4. unwrap and expect: the value, or a panic.
    assert_eq!(top.unwrap(), 3);
    assert_eq!(top.expect("the stack had one element"), 3);
}
```

`match` is the basic tool, and the other three are shorter forms of it.
`unwrap()` says "I am sure there is a value; panic if not". `expect("...")`
does the same with your own message. Both are fine in tests and in small
experiments. In other code they turn a case you did not think about into a
crash, so prefer `match` or `unwrap_or`.

All of this works the same way for `Result`, with `Ok` and `Err` in the
patterns.

### Text to number: parse

`parse` turns a text into a value of another type. It can fail, so it
returns a `Result`. You must say which type you want, either on the variable
or directly at the call:

```rust
fn main() {
    let a: Result<i32, _> = "42".parse();
    let b = "42".parse::<i32>(); // the same; `::<i32>` names the type
    assert_eq!(a, Ok(42));
    assert_eq!(b, Ok(42));

    assert!("forty-two".parse::<i32>().is_err());
    assert!("".parse::<i32>().is_err());
    assert!("4.5".parse::<i32>().is_err());
    assert_eq!("4.5".parse::<f64>(), Ok(4.5));
}
```

The `_` in `Result<i32, _>` lets the compiler fill in the error type.

### Writing functions that can fail

You return `Some(...)`/`None` or `Ok(...)`/`Err(...)` like any other value.

```rust
/// The position of the first negative number, if there is one.
fn first_negative(values: &[i32]) -> Option<usize> {
    let mut index = 0;
    for value in values {
        if *value < 0 {
            return Some(index); // leave the function early
        }
        index += 1;
    }
    None
}

/// A percentage from a text: a whole number from 0 to 100.
fn parse_percent(text: &str) -> Result<u32, String> {
    match text.parse::<u32>() {
        Ok(number) if number <= 100 => Ok(number),
        Ok(number) => Err(format!("{number} is more than 100")),
        Err(_) => Err(format!("`{text}` is not a whole number")),
    }
}

fn main() {
    assert_eq!(first_negative(&[4, 0, -2, -9]), Some(2));
    assert_eq!(first_negative(&[4, 0]), None);

    assert_eq!(parse_percent("40"), Ok(40));
    assert_eq!(
        parse_percent("140"),
        Err(String::from("140 is more than 100"))
    );
    assert_eq!(
        parse_percent("many"),
        Err(String::from("`many` is not a whole number"))
    );
}
```

Two new details. `return Some(index);` leaves the function from inside the
loop; this is the "leave early" use of `return`. And an arm can have an extra
condition after `if`: `Ok(number) if number <= 100` fits only when the
condition is true as well.

### The question mark

When a function calls several things that can fail, matching on each one
makes the code long. Inside a function that itself returns a `Result`, you
can write `?` after an expression of type `Result`. It means: **if this is
`Ok`, give me the value inside; if it is `Err`, return that error from my
function now.**

```rust
fn parse_percent(text: &str) -> Result<u32, String> {
    match text.parse::<u32>() {
        Ok(number) if number <= 100 => Ok(number),
        Ok(number) => Err(format!("{number} is more than 100")),
        Err(_) => Err(format!("`{text}` is not a whole number")),
    }
}

/// The part of `amount` that both percentages leave, one after the other.
fn discounted_twice(amount: u32, first: &str, second: &str) -> Result<u32, String> {
    let first = parse_percent(first)?;
    let second = parse_percent(second)?;
    let after_first = amount - amount * first / 100;
    Ok(after_first - after_first * second / 100)
}

fn main() {
    assert_eq!(discounted_twice(200, "50", "10"), Ok(90));
    assert_eq!(
        discounted_twice(200, "x", "10"),
        Err(String::from("`x` is not a whole number"))
    );
    assert_eq!(
        discounted_twice(200, "50", "101"),
        Err(String::from("101 is more than 100"))
    );
}
```

The error type of the inner call must fit the error type of your function;
here both are `String`. Chapter 7 covers error handling in depth.

### Mistakes people really make

**Using an `Option` as if it were the value.**

```rust,compile_fail,E0369
fn main() {
    let mut stack = vec![3];
    let top = stack.pop();
    let next = top + 1;
    println!("{next}");
}
```

```text
error[E0369]: cannot add `{integer}` to `Option<{integer}>`
 --> src/main.rs:4:20
  |
4 |     let next = top + 1;
  |                --- ^ - {integer}
  |                |
  |                Option<{integer}>
```

**`unwrap` on a `None`.** It compiles, and panics when it runs:

```rust,should_panic
fn main() {
    let mut stack: Vec<i32> = Vec::new();
    let top = stack.pop().unwrap();
    println!("{top}");
}
```

```text
thread 'main' (123659996) panicked at src/main.rs:3:27:
called `Option::unwrap()` on a `None` value
```

**`?` in a function that does not return a `Result`.**

```rust,compile_fail,E0277
fn main() {
    let n: i32 = "12".parse()?;
    println!("{n}");
}
```

```text
error[E0277]: the `?` operator can only be used in a function that returns `Result` or `Option` (or another type that implements `FromResidual`)
 --> src/main.rs:2:30
  |
1 | fn main() {
  | --------- this function should return `Result` or `Option` to accept `?`
2 |     let n: i32 = "12".parse()?;
  |                              ^ cannot use the `?` operator in a function that returns `()`
```

**`parse` without a type.** `let n = "12".parse();` gives `E0284`, "type
annotations needed": the compiler cannot know which type you want. Write
`"12".parse::<i32>()`.

### When to use which

- **`Option`**: a missing value is normal and needs no explanation (the
  first element of a list, a search).
- **`Result`**: something went wrong, and the caller should learn why.
- **`match`**, **`if let`**, **`unwrap_or`**: in code that must keep running.
- **`?`**: to pass an error on to your caller.
- **`unwrap`**, **`expect`**: in tests, and where a missing value would be a
  mistake in the program itself.

### Read more

- [`Option`](https://doc.rust-lang.org/std/option/index.html) and
  [`Result`](https://doc.rust-lang.org/std/result/index.html) in the standard
  library; both pages start with a guide.
- [`str::parse`](https://doc.rust-lang.org/std/primitive.str.html#method.parse)
- [The Rust Reference: The question mark operator](https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator)

## Lesson 09 · Writing your own tests

### The idea

Until now the course gave you the tests. Professional Rust is written the
other way round: you write the function **and** the tests that show it is
right. A test is ordinary Rust code, and the tools are already familiar from
reading reports.

```rust
/// Return the price after a discount of `percent` percent.
/// Panics when `percent` is larger than 100.
fn discounted(price: u32, percent: u32) -> u32 {
    assert!(percent <= 100, "a discount cannot be larger than 100 percent");
    price - price * percent / 100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_discount_keeps_the_price() {
        assert_eq!(discounted(250, 0), 250);
    }

    #[test]
    fn a_full_discount_is_free() {
        assert_eq!(discounted(250, 100), 0);
    }

    #[test]
    fn ten_percent_of_a_round_price() {
        assert_eq!(discounted(200, 10), 180);
        assert_ne!(discounted(200, 10), 200);
    }

    #[test]
    #[should_panic]
    fn a_discount_above_100_percent_panics() {
        discounted(250, 101);
    }
}
```

```text
running 4 tests
test tests::a_full_discount_is_free ... ok
test tests::no_discount_keeps_the_price ... ok
test tests::ten_percent_of_a_round_price ... ok
test tests::a_discount_above_100_percent_panics - should panic ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### The parts

- **`mod tests { ... }`** is a module: a named section of a file. Tests live
  in their own module, so that they stay apart from the code they test.
- **`#[cfg(test)]`** above the module means: compile this module only for
  `cargo test`. In a normal build the tests are not there at all.
- **`use super::*;`** makes everything from the surrounding file visible
  inside the module. Without it, the tests cannot see `discounted`.
- **`#[test]`** marks a function as a test. A test function takes no
  parameters. It passes when it returns, and it fails when it panics.
- **`assert!(condition)`**, **`assert_eq!(a, b)`** and **`assert_ne!(a, b)`**
  panic when the condition is false, when the two values differ, or when
  they are equal. Each takes an optional message as a last argument; it is
  printed when the check fails.
- **`#[should_panic]`** turns a test round: it passes only when the body
  panics. Use it to check that wrong input is refused.

When one assertion in a test fails, the test stops there. Later assertions
in the same function do not run. So keep one behaviour in each test and give
the test a name that states it. In a report, `a_full_discount_is_free ...
FAILED` should tell you what is broken before you open any file.

### Choosing the cases

A test that checks one ordinary input says little. Go through the contract
of the function, sentence by sentence, and choose inputs for:

- **an ordinary case** in the middle;
- **each boundary**: where the contract says "at least 18" or "more than
  300", test the last value on one side and the first value on the other;
- **nothing and zero**: an empty text, an empty list, 0;
- **each rule and each exception**: one input for every sentence of the
  contract;
- **the refused inputs**: whatever must give `None`, `Err` or a panic.

A good question to ask about your tests: **which wrong version of the
function would still pass?** If you can think of one, a case is missing.

### A test that can check any candidate

In Rust a function is a value. It can be passed to another function. The
type of "a function that takes an `i32` and returns an `i32`" is written
`fn(i32) -> i32`, and a parameter of that type is called like any function.

With this you can write your checks once and run them against several
versions of a function: the correct one must pass, and each wrong one must
fail.

```rust
/// Checks that `candidate` behaves like an absolute-value function.
fn check_absolute(candidate: fn(i32) -> i32) {
    assert_eq!(candidate(5), 5, "a positive number stays");
    assert_eq!(candidate(-5), 5, "a negative number loses its sign");
    assert_eq!(candidate(0), 0, "zero stays zero");
}

fn correct(n: i32) -> i32 {
    if n < 0 { -n } else { n }
}

/// A wrong version: it flips every sign.
fn flips_everything(n: i32) -> i32 {
    -n
}

fn main() {
    check_absolute(correct); // passes: nothing happens

    // `catch_unwind` runs the call and reports whether it panicked.
    let outcome = std::panic::catch_unwind(|| check_absolute(flips_everything));
    assert!(outcome.is_err(), "the checks must catch the wrong version");
}
```

A function's name without parentheses, `correct`, is the function as a value.
The piece `|| check_absolute(flips_everything)` is a **closure**: a small
function without a name, written where it is needed. The two bars hold its
parameters, and here it has none. Chapter 14 is about closures; in this
lesson you only read this one line.

If `check_absolute` had only the first assertion with a negative input, it
would still catch `flips_everything`. Would it catch a version that returns
its input unchanged? Each assertion is there to catch one family of wrong
versions.

### Comparing floats

`f64` values are stored in binary and are often a tiny bit off:

```rust
fn main() {
    let sum: f64 = 0.1 + 0.2;
    assert!(sum != 0.3); // it is 0.30000000000000004
    assert!((sum - 0.3).abs() < 1e-9); // the usual way to compare
}
```

In a test, compare floats by checking that their difference is very small.
`1e-9` is 0.000000001, and `.abs()` removes the sign of the difference.

### Mistakes people really make

**Forgetting `#[test]`.** The function is then an ordinary function that
nobody calls. Nothing fails; the report simply does not list it. The only
sign is a warning:

```text
warning: function `half_price` is never used
 --> src/main.rs:9:8
  |
9 |     fn half_price() {
  |        ^^^^^^^^^^
```

Count the tests in the report after you add one. Inside the lesson files of
this course you do not even get the warning: the chapter switches off the
warnings about unused code, because unfinished lessons would produce too
many of them. There, the count in the report is the only sign.

**Forgetting `use super::*;`.**

```text
error[E0425]: cannot find function `discounted` in this scope
 --> src/main.rs:9:20
  |
9 |         assert_eq!(discounted(250, 50), 125);
  |                    ^^^^^^^^^^ not found in this scope
```

**A `#[should_panic]` test for code that does not panic.** It fails with a
report of its own kind:

```text
---- tests::a_full_discount_panics stdout ----
note: test did not panic as expected at src/main.rs:11:8
```

The opposite danger is larger: a `#[should_panic]` test passes for *any*
panic, also for one caused by a mistake of yours. Keep such tests to one
line, the call that must panic.

**Testing only what works.** Tests written by the author of a function tend
to repeat the cases the author thought of while writing it. The boundaries
and the refused inputs are where the mistakes are.

### Read more

- [The Rust Reference: Testing attributes](https://doc.rust-lang.org/reference/attributes/testing.html)
- [The Cargo book: Tests](https://doc.rust-lang.org/cargo/guide/tests.html)
- [`assert_ne!`](https://doc.rust-lang.org/std/macro.assert_ne.html) in the
  standard library.
- [The Rust Reference: Function pointer types](https://doc.rust-lang.org/reference/types/function-pointer.html)

## Lesson 10 · Capstone: a text statistics tool

### What is different

In every lesson so far, the types and the signatures were given, and you
filled in bodies. In the capstone only **one** signature is given, the
function `run`, and the tests call nothing else. Everything behind it is
yours: which types exist, which functions, what they are called, and how
they are tested. That is much closer to real work. Read the contract above
`run` in the lesson file completely before you write anything.

### A way to start

1. **Types first.** The commands are a fixed list of possibilities: an
   `enum`. The three counts belong together: a `struct`. Write these two
   down before any logic.
2. **One function for each job**, each small enough to test alone: one that
   turns the command word into your enum (it can fail, so it returns a
   `Result`), one that counts, one that finds the longest word (there may be
   none, so `Option`), one that calculates the average.
3. **Test the pieces** in `mod my_tests`. A function that takes a text and
   returns a number is much easier to test than the whole tool.
4. **`run` last.** It becomes short: turn the word into a command, `match`
   on the command, call the piece that does the work, format the result.

### The toolbox

You have met most of these methods. Two are new: `lines()` and
`chars().count()`.

```rust
fn main() {
    let text = "Roses are red\nViolets are blue\n";

    // Lines, without their line ends.
    let mut lines = Vec::new();
    for line in text.lines() {
        lines.push(line);
    }
    assert_eq!(lines, ["Roses are red", "Violets are blue"]);

    // Many loops over text only count. `.count()` does that directly.
    assert_eq!(text.lines().count(), 2);
    assert_eq!(text.split_whitespace().count(), 6);

    // Characters, not bytes.
    assert_eq!("Zoë".chars().count(), 3);
    assert_eq!("Zoë".len(), 4);

    // A function on every character.
    let mut vowels = 0;
    for c in text.chars() {
        if "aeiouAEIOU".contains(c) {
            vowels += 1;
        }
    }
    assert_eq!(vowels, 12);

    // usize to f64 for an average: `as` converts between number types.
    let per_line = vowels as f64 / text.lines().count() as f64;
    assert_eq!(format!("{per_line:.1}"), "6.0");

    // A line end inside a text is written \n.
    assert_eq!(format!("a: {}\nb: {}", 1, 2), "a: 1\nb: 2");
}
```

`as` is a second way to convert numbers. Unlike `f64::from`, it is allowed
between all number types, and for that reason it can lose information
without telling you. For small counts turned into `f64` it is safe.
Chapter 3 shows where it is not.

A function may return a piece of the text it was given as a `&str`, without
copying it:

```rust
fn first_word(text: &str) -> Option<&str> {
    text.split_whitespace().next()
}

fn main() {
    assert_eq!(first_word("  tea time"), Some("tea"));
    assert_eq!(first_word("   "), None);
}
```

The result is a view into `text`. `.next()` asks a sequence for its next
element and returns an `Option`. If this is more than you want to think
about now, return `Option<String>` and make a copy with `.to_string()`;
Chapter 5 explains views properly.

### From tested functions to a real program

The tests call `run` directly. A program that people can use needs a `main`
that fetches the command and the text from the outside world and prints the
answer. Outside the course directory, create a package with
`cargo new textstats`, copy your functions into `src/main.rs`, and add a
`main` like this one:

```rust,no_run
// Your own functions go here. This short stand-in for `run` only makes the
// example complete.
fn run(command: &str, text: &str) -> Result<String, String> {
    Ok(format!("{command}: {} bytes", text.len()))
}

fn main() {
    // The words after the program's name. `skip(1)` leaves out the name.
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.len() != 1 {
        eprintln!("usage: textstats COMMAND < FILE");
        std::process::exit(2);
    }

    // Everything that arrives on standard input, as one String.
    let text = match std::io::read_to_string(std::io::stdin()) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read the input: {error}");
            std::process::exit(1);
        }
    };

    match run(&arguments[0], &text) {
        Ok(output) => println!("{output}"),
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(1);
        }
    }
}
```

Then run it. The words after `--` go to your program, and `<` sends a file
to its standard input:

```sh
cargo run -- count < src/main.rs
cargo run -- longest < Cargo.toml
cargo run -- dance < Cargo.toml
```

Windows PowerShell does not accept `<` for input. There, send the file
through a pipe: `Get-Content src/main.rs | cargo run -- count`.

`eprintln!` prints to the error output, so that error messages do not get
mixed into the result. `std::process::exit(1)` ends the program and tells
the terminal that it failed. `.collect()` gathers a sequence into a vector.
Chapters 15 and 18 cover these tools.

### Mistakes people really make

**Starting with `run` and writing everything inside it.** The function
grows to sixty lines, and no part of it can be tested alone. Types first,
then small functions.

**Measuring words with `len()`.** That counts bytes. The contract says
characters.

**`>=` when looking for the longest word.** With `>=`, a later word of the
same length replaces the earlier one, and the contract asks for the first.

**Dividing by zero words.** An empty text has no average. Decide what your
average function returns then, before you write the division.

### After this chapter

Do the code review lesson (C01) and the mastery check in the chapter guide.
Then you are ready for Chapter 3. Three things change there:

- **The pace.** The text of Chapter 3 is shorter for each idea, and it goes
  deeper. It expects that you can read every example of this chapter without
  help. If one of the lessons here still feels uncertain, redo it first with
  `cargo progress redo ch02 l05` (with the number of that lesson).
- **Two hints, not three.** From Chapter 3 on, each lesson has two hints,
  and HINTS.md also explains every prediction under *Check your
  predictions*.
- **Technique tests.** Some tests no longer check only the result. They also
  check how you got it, for example how often your code asked for new
  memory. Their failure messages say what they measured.

The first lesson of Chapter 3 is about how the compiler finds the type of a
value that you did not write a type for. It uses what you know from lessons
01 and 05 here: number types and their sizes, `Vec::new`, `push` and a
`for` loop over a range.

### Read more

- [`str::lines`](https://doc.rust-lang.org/std/primitive.str.html#method.lines)
  and
  [`str::split_whitespace`](https://doc.rust-lang.org/std/primitive.str.html#method.split_whitespace)
- [`std::env::args`](https://doc.rust-lang.org/std/env/fn.args.html)
- [`std::io::read_to_string`](https://doc.rust-lang.org/std/io/fn.read_to_string.html)
