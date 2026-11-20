# Chapter 6 concepts — Expressions and control flow

[Chapter guide](README.md) · [Hints](HINTS.md) · [Your notes](NOTES.md)

Read one section, then do the lesson with the same number. Every code block
here is compiled and run by `rustdoc --test`, and every compiler message was
copied from the pinned toolchain (Rust 1.99.0, edition 2024) and shortened.
A snippet that is not a complete program was compiled inside `fn main`, so a
line number in its message can be one higher than the line in the snippet.

## What you will be able to do

In Rust, almost every construct produces a value: a block, an `if`, a `match`,
even a `loop`. This chapter teaches you to read and write code in that style.

After this chapter you will be able to:

- say what value and type a block, `if`, `match` or `loop` has, and how one
  semicolon changes it;
- choose between `match`, `if let`, `let else` and `while let`;
- leave loops and blocks early with `break`, `continue`, labels and `return`;
- say when a temporary value is dropped in a tail expression, an `if let` and
  a `match`;
- read operator precedence, short-circuit evaluation and `as` casts without
  guessing;
- explain what `const { }` and `unsafe { }` blocks do and do not do;
- write a small closure and say what it captures.

## Before you start

Chapter 3 (integer and float types, overflow, `&str` and
`String`), Chapter 4 (ownership, moves, `Drop`, `Rc`) and Chapter 5
(references and borrowing).

## Time

These are estimates for a careful first pass, including the
predictions and the notes.

| Part | Estimate |
| --- | --- |
| Lesson 01 · Expression blocks and declarations | 40 min |
| Lesson 02 · if and match as values | 40 min |
| Lesson 03 · if let and let else | 50 min |
| Lesson 04 · for, while, break and continue | 45 min |
| Lesson 05 · Loop values and labeled blocks | 50 min |
| Lesson 06 · const and unsafe expression blocks | 40 min |
| Lesson 07 · Operators, calls, fields and assignment | 50 min |
| Lesson 08 · Casts and closure expressions | 50 min |
| Lesson 09 · Capstone | 60 to 90 min |
| Review, code review and the 11 drills | about 2 hours |

## Lesson 01 · Expression blocks and declarations

**Assumed:** `let`, functions, `&str` and `String`, and that a value is dropped
when its owner goes out of scope (Chapter 4).

An **expression** is code that produces a value: `2 + 3`, `text.len()`, a
function call. A **statement** does something and produces no value. Rust has
few statements: `let` declarations, item declarations (such as `fn` and
`const`), and an expression followed by a semicolon.

A **block** `{ ... }` is an expression. It runs its statements in order. If the
last thing in the block is an expression *without* a semicolon, that expression
is the **tail expression** and its value is the value of the block. If there is
no tail expression, the value is `()`. The type `()` is called **unit**. It has
exactly one value, also written `()`, and it takes no memory.

```rust
let weight_grams: u32 = 2_300;
let price_cents = {
    let kilos = weight_grams.div_ceil(1000); // 3
    let base = 450;
    base + kilos * 120 // no semicolon: the value of the block
};
assert_eq!(price_cents, 810);
// `kilos` and `base` do not exist here: they belonged to the block.
```

A function body is a block too, so the same rule gives a function its return
value. You need `return` only to leave early.

### Shadowing is not mutation

A second `let` with the same name creates a *new* variable that hides the old
one. This is called **shadowing**. The new variable may have a different type.
`mut` is different: it lets you change the value of one variable, and the type
cannot change.

```rust
let input = "  42 ";
let input = input.trim(); // a new &str variable
let input: u32 = input.parse().unwrap(); // same name, now a u32
assert_eq!(input + 1, 43);

let mut total = 0; // one variable whose value changes
total += input;
assert_eq!(total, 42);
```

```text
let input = "  42 ";        input (1st): &str --> "  42 "
let input = input.trim();   input (2nd): &str --> "42"     (1st is hidden)
let input: u32 = ...;       input (3rd): u32  =   42       (2nd is hidden)
```

A shadow made inside an inner block ends with that block. After the block, the
name refers to the outer variable again.

### Items inside a block

An **item** is a declaration such as `fn`, `const`, `struct` or `enum`. You may
declare items inside a block. An item is visible in the *whole* block, even
before the line that declares it. A `let` variable is visible only after its
`let`.

```rust
fn seconds(hours: u32) -> u32 {
    let total = hours * PER_HOUR; // used before its declaration: allowed
    const PER_HOUR: u32 = 3_600;
    total
}
assert_eq!(seconds(2), 7_200);
```

An item cannot read the local variables around it. Only a closure can do that
(Lesson 08).

### Temporaries in a tail expression

A **temporary** is a value that no variable names, such as the `String` made by
`name.to_uppercase()` in `name.to_uppercase().len()`. In edition 2024, the
temporaries of a block's tail expression are dropped *before* the block's local
variables.

```rust
struct Trace(&'static str);
impl Drop for Trace {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn label_len() -> usize {
    let _local = Trace("local");
    Trace("temporary").0.len() // tail expression with a temporary
}

fn main() {
    assert_eq!(label_len(), 9);
}
```

```text
drop temporary
drop local
```

This order matters when the temporary borrows a local: the borrow ends before
the local is destroyed. Editions before 2024 used the opposite order, so older
texts may describe a different rule.

### Mistakes people make

**A semicolon after the value.** The block then has type `()`.

```rust,compile_fail,E0308
fn price_cents(weight_grams: u32) -> u32 {
    let kilos = weight_grams.div_ceil(1000);
    450 + kilos * 120;
}
```

```text
error[E0308]: mismatched types
1 | fn price_cents(weight_grams: u32) -> u32 {
  |    -----------                       ^^^ expected `u32`, found `()`
  |    |
  |    implicitly returns `()` as its body has no tail or `return` expression
3 |     450 + kilos * 120;
  |                      - help: remove this semicolon to return this value
```

**Reading a local from a nested `fn`.** Use a parameter, or a closure.

```rust,compile_fail,E0434
let rate = 3;
fn scaled(n: u32) -> u32 {
    n * rate
}
```

```text
error[E0434]: can't capture dynamic environment in a fn item
4 |         n * rate
  |             ^^^^
  = help: use the `|| { ... }` closure form instead
```

**Assigning a new type to a `mut` variable.** Write a new `let` instead.

```rust,compile_fail,E0308
let mut input = "  42 ";
input = input.trim().len();
```

```text
error[E0308]: mismatched types
2 |     let mut input = "  42 ";
  |                     ------- expected due to this value
3 |     input = input.trim().len();
  |             ^^^^^^^^^^^^^^^^^^ expected `&str`, found `usize`
```

### When to use which

- Use a block expression to compute one value from a few private steps.
- Use shadowing when a value changes form (text, then trimmed text, then a
  number). Use `mut` when one value changes over time (a counter, a total).
- Use a nested `fn` or `const` for a helper that only one function needs.

### Read more

- [Block expressions](https://doc.rust-lang.org/reference/expressions/block-expr.html)
  and [statements](https://doc.rust-lang.org/reference/statements.html) in the Rust Reference
- [Temporary scopes](https://doc.rust-lang.org/reference/destructors.html#temporary-scopes)
  in the Rust Reference
- [Tail expression temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-tail-expr-scope.html)
  in the Edition Guide

## Lesson 02 · if and match as values

**Assumed:** Lesson 01 (blocks have values), integer types and ranges of
values (Chapter 3).

`if` is an expression. Its value is the value of the branch that runs. So when
you use the value, both branches must have the same type, and the `else` must
exist. An `if` without `else` has type `()`, because that is all it can produce
when the condition is false. The condition must be a `bool`. Rust never treats
a number or a pointer as true or false.

```rust
let celsius = -4;
let state = if celsius <= 0 { "ice" } else { "water" };
assert_eq!(state, "ice");
```

`match` compares one value, called the **scrutinee**, with a list of
**patterns**. Each pattern with its code is called an **arm**. The rules:

1. The scrutinee is evaluated once.
2. The arms are tried from top to bottom. The first pattern that matches wins,
   and only that arm runs.
3. The patterns together must cover every possible value. This is called
   **exhaustiveness**, and the compiler checks it.
4. A `match` is an expression, so every arm must produce the same type.

The patterns you need in this chapter:

| Pattern | Matches |
| --- | --- |
| `7` | exactly that value |
| `1 \| 2` | either value |
| `3..=9` | every value from 3 to 9, both ends included |
| `n` | anything, and names it `n` |
| `n @ 3..=9` | a value from 3 to 9, and names it `n` |
| `_` | anything, without naming it |

An arm may also have a **guard**: `pattern if condition`. The arm is chosen
only when the pattern matches and the condition is true.

```rust
fn fare_cents(age: u8, has_pass: bool) -> u32 {
    match age {
        0..=5 => 0,
        6..=17 if has_pass => 100,
        6..=17 => 180,
        years @ 65.. if years >= 90 => 0,
        65.. => 150,
        _ => 320,
    }
}
assert_eq!(fare_cents(4, false), 0);
assert_eq!(fare_cents(12, true), 100);
assert_eq!(fare_cents(12, false), 180);
assert_eq!(fare_cents(93, false), 0);
assert_eq!(fare_cents(70, false), 150);
assert_eq!(fare_cents(30, true), 320);
```

Trace `fare_cents(12, false)`: `0..=5` does not match. `6..=17 if has_pass`
matches the pattern, but the guard is false, so the search continues. The next
arm, `6..=17`, matches and has no guard. The result is 180, and no later arm is
looked at.

### Mistakes people make

**A number as a condition.** Compare it with something.

```rust,compile_fail,E0308
let pending = 3;
if pending {
    println!("work to do");
}
```

```text
error[E0308]: mismatched types
3 |     if pending {
  |        ^^^^^^^ expected `bool`, found integer
```

The fix is `if pending > 0`.

**Branches with different types.**

```rust,compile_fail,E0308
let age = 30;
let fare = if age < 12 { 150 } else { "full" };
```

```text
error[E0308]: `if` and `else` have incompatible types
3 |     let fare = if age < 12 { 150 } else { "full" };
  |                              ---          ^^^^^^ expected integer, found `&str`
  |                              |
  |                              expected because of this
```

**A broad pattern before a narrow one.** This compiles, but with a warning,
and the second arm can never run:

```rust
fn fare(age: u8) -> u32 {
    match age {
        _ => 300,
        0..=11 => 150,
    }
}
assert_eq!(fare(5), 300); // probably not what the author wanted
```

```text
warning: unreachable pattern
3 |         _ => 300,
  |         - matches any value
4 |         0..=11 => 150,
  |         ^^^^^^ no value can reach this
```

**Trusting a guard to complete a match.** The compiler does not reason about
guard conditions. This match covers every `u8` in practice, but it is rejected:

```rust,compile_fail,E0004
fn fare(age: u8) -> u32 {
    match age {
        0..=11 => 150,
        n if n >= 12 => 300,
    }
}
```

```text
error[E0004]: non-exhaustive patterns: `12_u8..=u8::MAX` not covered
2 |     match age {
  |           ^^^ pattern `12_u8..=u8::MAX` not covered
  = note: the matched value is of type `u8`
  = note: match arms with guards don't count towards exhaustivity
```

The fix is to make the last arm a pattern without a guard: `12.. => 300` or
`_ => 300`.

### When to use which

- Two outcomes that depend on a `bool`: `if` / `else`.
- Several outcomes that depend on *one value*: `match`. You get the
  exhaustiveness check, and the scrutinee is evaluated once.
- Several unrelated conditions: an `if` / `else if` chain. The conditions are
  evaluated in turn until one is true. If you write the same function call in
  two conditions, it can run twice.

### Read more

- [if expressions](https://doc.rust-lang.org/reference/expressions/if-expr.html)
  and [match expressions](https://doc.rust-lang.org/reference/expressions/match-expr.html)
  in the Rust Reference
- [Patterns](https://doc.rust-lang.org/reference/patterns.html) in the Rust Reference

## Lesson 03 · if let and let else

**Assumed:** Lesson 02 (`match` and patterns), `Option` and `Result` as used in
Chapters 3 to 5, `Rc::strong_count` (Chapter 4), and the text methods
`str::find` and `str::split_once`, which return an `Option` (Chapter 5).

A pattern is **irrefutable** if it matches every value of its type: `x`,
`(a, b)`, `_`. A pattern is **refutable** if it can fail: `Some(x)` does not
match `None`. A plain `let` needs an irrefutable pattern, because the code
after it must be able to use the variables.

For a refutable pattern you have three tools besides `match`.

**`if let`** runs a block only when the pattern matches. The new variables
exist only inside that block. An `else` block is optional.

```rust
let nickname: Option<&str> = Some("Lin");
let shown = if let Some(nick) = nickname {
    nick
} else {
    "guest"
};
assert_eq!(shown, "Lin");
```

**`let else`** is the opposite shape. The variables go into the *surrounding*
scope, so the rest of the function can use them. In exchange, the `else` block
must **diverge**: it must leave with `return`, `break`, `continue` or a panic.
It cannot produce a value, because there would be nothing to bind.

```rust
fn fahrenheit(reading: Option<f64>) -> String {
    let Some(celsius) = reading else {
        return String::from("no reading");
    };
    // `celsius` is an f64 from here to the end of the function.
    format!("{:.1} F", celsius * 9.0 / 5.0 + 32.0)
}
assert_eq!(fahrenheit(Some(21.5)), "70.7 F");
assert_eq!(fahrenheit(None), "no reading");
```

**`while let`** repeats a block for as long as the pattern matches:

```rust
let mut queue = vec!["c", "b", "a"];
let mut order = String::new();
while let Some(job) = queue.pop() {
    order.push_str(job);
}
assert_eq!(order, "abc");
```

### How long does a temporary in the condition live?

Lesson 01 defined a temporary: a value that no variable names. The value you
test in an `if let` or a `match` often creates one. The two constructs treat it
differently in edition 2024:

- in a `match`, a temporary in the scrutinee lives until the end of the whole
  `match`, so it is still alive in every arm;
- in an `if let`, it lives through the first block, but it is dropped *before*
  the `else` block runs.

You can see this with `Rc`, whose strong count shows how many owners are alive.
`Rc::clone(&shared)` below is a temporary second owner.

```rust
use std::rc::Rc;
let shared = Rc::new(vec![10, 20, 30]);

let in_then = if let Some(_first) = Rc::clone(&shared).first() {
    Rc::strong_count(&shared)
} else {
    0
};
let in_else = if let Some(_missing) = Rc::clone(&shared).get(9) {
    0
} else {
    Rc::strong_count(&shared)
};
let in_arm = match Rc::clone(&shared).get(9) {
    Some(_) => 0,
    None => Rc::strong_count(&shared),
};
assert_eq!((in_then, in_else, in_arm), (2, 1, 2));
```

```text
if let PATTERN = <temporary> { first block } else { else block }
                 |------- temporary alive ------|

match <temporary> { arm, arm, arm }
      |-------- temporary alive ---|
```

This matters most for temporaries that hold a borrow or a lock: in a `match`
they block other uses of the data in every arm. Chapters 9 and 19 return to
this with `RefCell` and `Mutex`. Editions before 2024 kept the `if let`
temporary alive through the `else` block as well.

### Mistakes people make

**Using an `if let` variable after its block.**

```rust,compile_fail,E0425
let reading: Option<f64> = Some(21.5);
if let Some(celsius) = reading {
    println!("{celsius}");
}
println!("{}", celsius + 1.0);
```

```text
error[E0425]: cannot find value `celsius` in this scope
6 |     println!("{}", celsius + 1.0);
  |                    ^^^^^^^ not found in this scope
```

If the rest of the function needs the value, use `let else`.

**A refutable pattern in a plain `let`.**

```rust,compile_fail,E0005
let reading: Option<f64> = Some(21.5);
let Some(celsius) = reading;
```

```text
error[E0005]: refutable pattern in local binding
3 |     let Some(celsius) = reading;
  |         ^^^^^^^^^^^^^ pattern `None` not covered
  = note: `let` bindings require an "irrefutable pattern", like a `struct` or an `enum` with only one variant
help: you might want to use `let...else` to handle the variant that isn't matched
  |
3 |     let Some(celsius) = reading else { todo!() };
  |                                 ++++++++++++++++
```

**An `else` block that falls through.** If the `else` of a `let else` only
prints a message, the compiler reports that it "does not diverge". End it with
`return`, `break`, `continue` or a panic. If you want a fallback *value*, you
want `if let ... else` or `unwrap_or`, not `let else`.

### When to use which

- Every case needs its own code: `match`.
- One case matters and the others share a fallback: `if let` with `else`.
- A check at the top of a function, after which the value is needed
  everywhere: `let else` with `return`.
- Take items until there are none: `while let`.

### Read more

- [if let](https://doc.rust-lang.org/reference/expressions/if-expr.html#if-let-patterns)
  and [let else](https://doc.rust-lang.org/reference/statements.html#let-statements)
  in the Rust Reference
- [Refutability](https://doc.rust-lang.org/reference/patterns.html#refutability)
  in the Rust Reference
- [if let temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-if-let-scope.html)
  in the Edition Guide

## Lesson 04 · for, while, break and continue

**Assumed:** slices and vectors (Chapter 3), borrowing a collection with `&`
(Chapter 5), Lessons 01 to 03.

Rust has three loops. This lesson covers two of them.

**`for pattern in iterable`** takes the items of something one by one. The most
common things to loop over are a range and a borrowed collection:

- `0..n` is a **half-open range**: it starts at 0 and stops *before* `n`;
- `0..=n` is an **inclusive range**: it includes `n`;
- `&items` gives a shared reference to each item, and `&mut items` gives a
  mutable reference to each item;
- `items` without `&` *moves* the collection into the loop and gives each item
  by value.

**`while condition`** checks the condition, runs the body if it is true, and
repeats. The condition is checked before every pass, including the last check
that ends the loop.

Inside either loop, **`continue`** skips the rest of the current pass and goes
to the next one. **`break`** leaves the loop at once. Both loops have type
`()`: they do things, they do not produce a value.

```rust
let lines = [
    "fn main() {",
    "    // TODO: parse",
    "    run();",
    "    // TODO: report",
    "    // TODO: tests",
    "}",
];
// Line numbers of the first two TODO comments.
let mut found = Vec::new();
for (index, line) in lines.iter().enumerate() {
    if !line.contains("TODO") {
        continue; // not interesting: next line
    }
    if found.len() == 2 {
        break; // enough: stop reading
    }
    found.push(index + 1);
}
assert_eq!(found, [2, 4]);
```

`enumerate` pairs each item with its position, starting at 0. The pattern
`(index, line)` takes the pair apart.

A `while` loop needs you to make progress yourself. This one finds the greatest
common divisor of two numbers:

```rust
let (mut a, mut b) = (84u32, 36u32);
while b != 0 {
    (a, b) = (b, a % b);
}
assert_eq!(a, 12);
```

```text
pass    a    b    check b != 0
  -    84   36    true
  1    36   12    true
  2    12    0    false: the loop ends
```

Why does it end? `a % b` is always smaller than `b`, so `b` gets smaller on
every pass and must reach 0. Give yourself this argument for every `while` loop
you write: name the thing that changes, and say why the condition must become
false.

### Mistakes people make

**One step too far.** An index runs from 0 to `len - 1`. The range `0..=len`
includes `len`, which is not a valid index. This compiles and then panics:

```rust,should_panic
let scores = [70, 85, 90];
let mut total = 0;
for i in 0..=scores.len() {
    total += scores[i];
}
```

```text
index out of bounds: the len is 3 but the index is 3
```

Loop over the items instead of the indexes when you can:
`for score in &scores { total += score; }`.

**Moving the collection into the loop.**

```rust,compile_fail,E0382
let lines = vec![String::from("TODO: tests"), String::from("done")];
for line in lines {
    println!("{line}");
}
println!("{} lines", lines.len());
```

```text
error[E0382]: borrow of moved value: `lines`
3 |     for line in lines {
  |                 ----- `lines` moved due to this implicit call to `.into_iter()`
...
6 |     println!("{} lines", lines.len());
  |                          ^^^^^ value borrowed here after move
help: consider iterating over a slice of the `Vec<String>`'s content to avoid moving into the `for` loop
  |
3 |     for line in &lines {
  |                 +
```

**`continue` before the step of a `while` loop.** The compiler cannot see this
one. If you write `while i < n { if skip(i) { continue; } ...; i += 1; }`, the
`continue` jumps over `i += 1`, so `i` never changes and the loop never ends.
A `for` loop cannot have this bug, because the loop itself moves to the next
item.

**Filters that hide a stop condition.** When one value means "stop" and other
values mean "skip", write the two tests so that they cannot both be true, or
test for "stop" first. A wide "skip" test placed first can swallow the stop
value.

### When to use which

- You have a collection or a range: `for`. It cannot run past the end and
  cannot forget to advance.
- You repeat until a condition on your own variables changes: `while`.
- You repeat until something inside the body decides to stop, or you need the
  loop to produce a value: `loop` (Lesson 05).

### Read more

- [Loop expressions](https://doc.rust-lang.org/reference/expressions/loop-expr.html)
  in the Rust Reference
- [Range expressions](https://doc.rust-lang.org/reference/expressions/range-expr.html)
  in the Rust Reference
- [`Iterator::enumerate`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.enumerate)
  in the standard library documentation

## Lesson 05 · Loop values and labeled blocks

**Assumed:** Lesson 04 (`for`, `while`, `break`, `continue`), Lesson 03
(`let else`).

**`loop`** repeats its body forever. The only way out is `break` or `return`
(or a panic). Because the compiler knows this, `loop` can be an expression with
a useful value: `break value` ends the loop and makes `value` the value of the
whole `loop`.

```rust
// The first Fibonacci number greater than a limit.
let limit = 1_000u32;
let (mut previous, mut current) = (0u32, 1u32);
let first_above = loop {
    if current > limit {
        break current;
    }
    (previous, current) = (current, previous + current);
};
assert_eq!(first_above, 1597);
```

`for` and `while` cannot do this. They may end without any `break`, and then
there would be no value. So they always have type `()`.

### The never type

`break`, `continue`, `return` and `panic!()` are expressions too. They never
finish with a value, because control goes somewhere else. Their type is the
**never type**, written `!`. A `!` expression is accepted wherever any type is
expected. That is why one arm of a `match` can give a number and another arm
can `return`:

```rust
fn double(text: &str) -> Option<u32> {
    let n: u32 = match text.trim().parse() {
        Ok(n) => n,
        Err(_) => return None, // type `!`, accepted where u32 is expected
    };
    Some(n * 2)
}
assert_eq!(double(" 21 "), Some(42));
assert_eq!(double("x"), None);
```

It is also why the `else` block of `let else` must diverge: its type must be
`!`.

> **Note: newer Rust versions.** Rust 1.100 (November 2026) stabilised the
> never type `!` and made `std::convert::Infallible` an alias of it. This
> course is pinned to 1.99, where you may write `!` only as the return type of
> a function, such as `fn stop() -> ! { panic!() }`. Writing it anywhere else,
> for example `let x: ! = stop();`, is error E0658 ("the `!` type is
> experimental") on 1.99 and compiles on 1.100. What this section says about
> `break`, `continue`, `return` and `panic!()` is the same on both versions.
> See the
> [Rust 1.100.0 release notes](https://doc.rust-lang.org/stable/releases.html#version-11000-2026-11-12).

### Labels

A plain `break` or `continue` acts on the innermost loop. To act on an outer
loop, give that loop a **label**. A label is a name that starts with `'`, and
it is written before the loop with a colon.

```rust
let grid = [[5, 8, 2], [7, 3, 9], [4, 6, 1]];
let mut position = None;
'rows: for (r, row) in grid.iter().enumerate() {
    for (c, &cell) in row.iter().enumerate() {
        if cell == 3 {
            position = Some((r, c));
            break 'rows; // leaves both loops
        }
    }
}
assert_eq!(position, Some((1, 1)));
```

A block can have a label too. `break 'label value` leaves the block early with
that value. If no `break` runs, the value is the tail expression as usual.

```rust
fn problem(name: &str) -> Option<&'static str> {
    'check: {
        if name.is_empty() {
            break 'check Some("empty");
        }
        if name.len() > 12 {
            break 'check Some("too long");
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric()) {
            break 'check Some("letters and digits only");
        }
        None
    }
}
assert_eq!(problem(""), Some("empty"));
assert_eq!(problem("ada_l"), Some("letters and digits only"));
assert_eq!(problem("ada99"), None);
```

`return` is different from all of these: it leaves the whole function, however
many loops and blocks are around it.

### Mistakes people make

**`while true` instead of `loop`.** To the type checker, `while true` is an
ordinary `while`, which might end. So the compiler believes the function can
reach its closing brace without a value:

```rust,compile_fail,E0308
fn first_multiple(step: u32, above: u32) -> u32 {
    let mut n = step;
    while true {
        if n > above {
            return n;
        }
        n += step;
    }
}
```

```text
warning: denote infinite loops with `loop { ... }`
3 |     while true {
  |     ^^^^^^^^^^ help: use `loop`

error[E0308]: mismatched types
1 |   fn first_multiple(step: u32, above: u32) -> u32 {
  |                                               --- expected `u32` because of return type
3 | /     while true {
... |
8 | |     }
  | |_____^ expected `u32`, found `()`
  = note: `while` loops evaluate to unit type `()`
```

With `loop` the same function compiles, because a `loop` without `break` has
type `!`.

**A label that does not exist.** Declaring the label on the loop is your job.

```rust,compile_fail,E0426
let grid = [[1, 2], [3, 4]];
for row in grid {
    for cell in row {
        if cell == 3 {
            break 'rows;
        }
    }
}
```

```text
error[E0426]: use of undeclared label `'rows`
6 |                 break 'rows;
  |                       ^^^^^ undeclared label `'rows`
```

**A search loop that can pass its answer.** A `loop` that searches upward has
no built-in limit. If the test can never become true, for example because the
arithmetic in the test overflows, the loop panics in a debug build or runs
forever in a release build. Do the arithmetic of the test in a type that is
wide enough.

### When to use which

- The loop computes one result and stops when it has it: `loop` with
  `break value`.
- An inner loop must stop or skip an outer loop: a label.
- Straight-line code with several early exits to the same place: a labeled
  block. If the block is the whole function body, a plain `return` is simpler.
- If the labeled code grows long, move it into its own function and use
  `return`.

### Read more

- [Loop expressions, labels and `break` values](https://doc.rust-lang.org/reference/expressions/loop-expr.html)
  in the Rust Reference
- [The never type](https://doc.rust-lang.org/reference/types/never.html) in the Rust Reference
- [`return` expressions](https://doc.rust-lang.org/reference/expressions/return-expr.html)
  in the Rust Reference

## Lesson 06 · const and unsafe expression blocks

**Assumed:** arrays `[T; N]` and their fixed length (Chapter 3), references and
the borrow rules (Chapter 5).

This lesson is about two kinds of block that change *how* code is treated, not
what it computes.

### const blocks

Some places in a program need a value that the compiler knows before the
program runs: the length of an array, the value of a `const` item. Such a place
is called a **const context**. An **inline const block**, `const { ... }`, lets
you write a compile-time computation in the middle of ordinary code. The
compiler evaluates it, and the result is used as if you had written the literal
value.

```rust
const DAYS: usize = 7;

// The length is computed by the compiler. The type is [u32; 14].
let totals = [0u32; const { DAYS * 2 }];
assert_eq!(totals.len(), 14);

// `[value; N]` normally needs a value that is Copy. A const block is also
// accepted: every element starts as that constant value.
let buckets: [Vec<String>; 3] = [const { Vec::new() }; 3];
assert!(buckets.iter().all(|bucket| bucket.is_empty()));
```

A const block can use literals, `const` items, and functions declared
`const fn`. It cannot read a `let` variable, because that value exists only
when the program runs. If the computation fails, for example by overflowing,
you get a compile error instead of a wrong program:

```rust,compile_fail,E0080
let table = [0u8; const { (250u8 + 10) as usize }];
```

```text
error[E0080]: attempt to compute `250_u8 + 10_u8`, which would overflow
2 |     let table = [0u8; const { (250u8 + 10) as usize }];
  |                               ^^^^^^^^^^^^ evaluation of `main::{constant#0}::{constant#0}` failed here
```

### unsafe blocks

Safe Rust guarantees memory safety because the compiler checks every
operation. A few operations cannot be checked by the compiler. An
**`unsafe` block** allows them. Inside `unsafe { }` you may additionally:

1. read or write through a **raw pointer** (`*const T` or `*mut T`);
2. call a function or method that is declared `unsafe`;
3. read or write a `static mut` variable;
4. read a field of a `union`.

(A fifth unsafe operation, implementing an `unsafe trait`, is written
`unsafe impl` and needs no block.)

The list is short, and nothing else changes. `unsafe` does not switch off type
checking or the borrow checker, and it does not make code faster. It moves a
duty from the compiler to you: *you* must make sure the rules of the operation
hold. By convention you write the reason in a `// SAFETY:` comment directly
above the block.

```rust
let bytes = [b'o', b'k'];
// SAFETY: both bytes are ASCII letters, so the slice is valid UTF-8,
// which is what from_utf8_unchecked requires.
let text = unsafe { std::str::from_utf8_unchecked(&bytes) };
assert_eq!(text, "ok");

// The safe version checks at run time and needs no proof from you.
assert_eq!(std::str::from_utf8(&bytes), Ok("ok"));
```

A raw pointer is an address with a type, without the guarantees of a
reference. `&raw const x` and `&raw mut x` create one from a variable. Creating
a raw pointer is safe. *Using* it to reach the value is the unsafe step. Before
you do that, four things must be true: the pointer is not null, it is correctly
aligned for its type, it points to a live, initialized value of that type, and
the access does not break the borrow rules (for example, no write while a
shared reference to the same value is in use).

Raw pointers also allow arithmetic. The method `add` counts in elements, not
in bytes: for a `*const u32`, `p.add(1)` is the address 4 bytes after `p`,
where the next `u32` would be. `add` is an unsafe method, because the result
must stay inside the same array or other allocation.

```text
let reading = 21.5_f64;          reading: [ 21.5 ]  <-- one live, aligned f64
let p = &raw const reading;      p: *const f64 ------^
unsafe { *p }                    allowed only because all four facts hold
```

Chapter 23 is about unsafe code, and the mastery track builds `Vec` and `Rc`
from raw allocation. For now, learn to recognise the operations and the duty
that comes with them.

### Mistakes people make

**A runtime value in a const block.**

```rust,compile_fail,E0435
let days = 7;
let totals = [0u32; const { days * 2 }];
```

```text
error[E0435]: attempt to use a non-constant value in a constant
3 |     let totals = [0u32; const { days * 2 }];
  |                                 ^^^^ non-constant value
help: consider using `const` instead of `let`
```

**An unsafe operation outside `unsafe`.**

```rust,compile_fail,E0133
let reading = 21.5f64;
let pointer = &raw const reading;
let copy = *pointer;
```

```text
error[E0133]: dereference of raw pointer is unsafe and requires unsafe block
4 |     let copy = *pointer;
  |                ^^^^^^^^ dereference of raw pointer
  = note: raw pointers may be null, dangling or unaligned; they can violate aliasing rules and cause data races: all of these are undefined behavior
```

**Expecting `unsafe` to silence the borrow checker.** It does not. This is
rejected in exactly the same way with or without the block, and the compiler
adds a warning that the block is useless:

```rust,compile_fail,E0499
let mut scores = vec![70, 85, 90];
let first = &mut scores[0];
unsafe {
    scores.push(100);
}
*first += 1;
```

```text
warning: unnecessary `unsafe` block

error[E0499]: cannot borrow `scores` as mutable more than once at a time
3 |     let first = &mut scores[0];
  |                      ------ first mutable borrow occurs here
4 |     unsafe {
5 |         scores.push(100);
  |         ^^^^^^ second mutable borrow occurs here
6 |     }
7 |     *first += 1;
  |     ----------- first borrow later used here
```

### When to use which

- A value needed in several places: a named `const` item. A value needed once,
  where a short formula is clearer than a number: an inline `const { }` block.
- A safe function does the job (`get`, `from_utf8`, `checked_add`): use it.
  Reach for `unsafe` only when you have an invariant that the compiler cannot
  see and a measured reason to skip the check.

### Read more

- [Const blocks](https://doc.rust-lang.org/reference/expressions/block-expr.html#const-blocks)
  and [unsafe blocks](https://doc.rust-lang.org/reference/expressions/block-expr.html#unsafe-blocks)
  in the Rust Reference
- [Unsafety](https://doc.rust-lang.org/reference/unsafety.html) in the Rust Reference
- [Raw borrow operators](https://doc.rust-lang.org/reference/expressions/operator-expr.html#raw-borrow-operators)
  in the Rust Reference
- [What unsafe Rust can do](https://doc.rust-lang.org/nomicon/what-unsafe-does.html)
  in the Rustonomicon

## Lesson 07 · Operators, calls, fields and assignment

**Assumed:** integer types, overflow behaviour and binary notation
(Chapter 3), `&` and `*` on references (Chapter 5).

When an expression has several operators, **precedence** decides which operator
takes its operands first. From tightest to loosest, the levels you meet most
often are:

| Level | Operators |
| --- | --- |
| 1 | method calls `x.f()`, field access `x.f`, calls `f(x)`, indexing `x[i]` |
| 2 | unary `-`, `!`, `*`, `&`, `&mut` |
| 3 | `as` |
| 4 | `*`, `/`, `%` |
| 5 | `+`, `-` |
| 6 | `<<`, `>>` |
| 7 | `&` |
| 8 | `^` |
| 9 | `\|` |
| 10 | `==`, `!=`, `<`, `>`, `<=`, `>=` |
| 11 | `&&` |
| 12 | `\|\|` |
| 13 | `..`, `..=` |
| 14 | `=`, `+=`, `-=` and the other assignments |

Three consequences are worth remembering:

- A method call binds tighter than a unary operator, so `-x.abs()` means
  `-(x.abs())`.
- The bit operators `&`, `^` and `|` bind tighter than comparisons. So
  `flags & MASK != 0` means `(flags & MASK) != 0`. In C it means something
  else.
- Arithmetic binds tighter than shifts: `1 + 2 << 3` is `(1 + 2) << 3`.

When a reader might hesitate, add parentheses. They cost nothing.

### Evaluation order and short-circuiting

Operands are evaluated from left to right. `a && b` evaluates `b` only if `a`
is true, and `a || b` evaluates `b` only if `a` is false. This is called
**short-circuit evaluation**. It lets the left side protect the right side:

```rust
fn positive_at(items: &[i32], index: usize) -> bool {
    // The length test runs first. When it is false, items[index] is never
    // evaluated, so a bad index cannot panic here.
    index < items.len() && items[index] > 0
}
assert!(positive_at(&[4, 0, 9], 2));
assert!(!positive_at(&[4, 0, 9], 1));
assert!(!positive_at(&[4, 0, 9], 7));
```

On `bool` values, the single `&` and `|` compute the same results but always
evaluate both sides.

Assignment is the exception to left-to-right order. In `place = value`, the
value on the right is evaluated first, and then the place on the left:

```rust
fn noted(log: &mut Vec<&'static str>, name: &'static str, n: usize) -> usize {
    log.push(name);
    n
}
let mut log = Vec::new();
let mut slots = [0; 3];
slots[noted(&mut log, "index", 1)] = noted(&mut log, "value", 9);
assert_eq!(slots, [0, 9, 0]);
assert_eq!(log, ["value", "index"]);
```

### Bits

`&` keeps the bits that are set in both operands. `|` keeps the bits set in
either. `^` keeps the bits that differ. `!` flips every bit. `<<` and `>>` move
bits left and right. On an unsigned type, `>>` fills with zeros. On a signed
type, it copies the sign bit.

```rust
const READ: u8 = 0b001;
const WRITE: u8 = 0b010;
const EXECUTE: u8 = 0b100;

let mut mode = READ | EXECUTE; // 0b101
assert!(mode & WRITE == 0); // (mode & WRITE) == 0
mode |= WRITE; // switch one bit on
mode &= !EXECUTE; // switch one bit off
assert_eq!(mode, 0b011);

// A 3-bit field stored in bits 5 to 7 of a byte.
let header: u8 = 0b1011_0010;
assert_eq!((header >> 5) & 0b111, 0b101);
```

```text
header          1 0 1 1 0 0 1 0
header >> 5     0 0 0 0 0 1 0 1
& 0b111         0 0 0 0 0 1 0 1   = 5
```

### Calls, fields and references

`value.method()` borrows or dereferences `value` automatically to fit the
method. That is why `text.len()` works when `text` is a `String`, a `&String`
or a `&&str`. Field access `pair.0` and indexing `items[i]` look through
references in the same way. The operators you write yourself are `&x` (borrow),
`&mut x` (borrow mutably) and `*r` (reach the value behind a reference).

```rust
let pair = (String::from("ada"), [3, 4, 5]);
let borrowed = &pair;
assert_eq!(borrowed.0.len(), 3); // field access through the reference
assert_eq!(borrowed.1[2], 5);
let last: &i32 = &borrowed.1[2]; // & applies to the whole place borrowed.1[2]
assert_eq!(*last + 1, 6);
```

### Assignment

`=` stores a value. An assignment is an expression, but its type is `()`, not
the type of the stored value. The compound forms `+=`, `-=`, `*=`, `/=`, `%=`,
`&=`, `|=`, `^=`, `<<=` and `>>=` combine an operator with assignment. Rust has
no `++` and no `--`.

### Mistakes people make

**`=` where `==` was meant.** The type `()` makes this a compile error, not a
silent bug:

```rust,compile_fail,E0308
let mut retries = 3;
if retries = 0 {
    println!("giving up");
}
```

```text
error[E0308]: mismatched types
3 |     if retries = 0 {
  |        ^^^^^^^^^^^ expected `bool`, found `()`
help: you might have meant to compare for equality
  |
3 |     if retries == 0 {
  |                 +
```

For the same reason `a = b = 5` does not set two variables. And comparisons do
not chain: write `low < x && x < high`.

**A cast directly before `<`.** After `as usize`, the parser reads `<` as the
start of generic arguments, as in `Vec<u8>`.

```rust,compile_fail
let index: u32 = 2;
let len: usize = 5;
if index as usize < len {
    println!("in range");
}
```

```text
error: `<` is interpreted as a start of generic arguments for `usize`, not a comparison
4 |     if index as usize < len {
  |                       ^ ----- interpreted as generic arguments
  |                       |
  |                       not interpreted as comparison
help: try comparing the cast value
  |
4 |     if (index as usize) < len {
  |        +              +
```

**`count++`.**

```rust,compile_fail
let mut count = 0;
count++;
```

```text
error: Rust has no postfix increment operator
3 |     count++;
  |          ^^ not a valid postfix operator
help: use `+= 1` instead
```

### Read more

- [Expression precedence](https://doc.rust-lang.org/reference/expressions.html#expression-precedence)
  in the Rust Reference
- [Operator expressions](https://doc.rust-lang.org/reference/expressions/operator-expr.html)
  in the Rust Reference
- [Method call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html)
  in the Rust Reference

## Lesson 08 · Casts and closure expressions

**Assumed:** integer ranges, two's complement and floats (Chapter 3), moves and
`Copy` (Chapter 4), borrowing (Chapter 5).

### Casts with `as`

`value as Type` converts between primitive types. A cast never panics and never
reports an error. If the value does not fit, you get a *different value*, by
these rules:

| From | To | Result |
| --- | --- | --- |
| integer | wider integer | same value (`-1i8 as i32` is -1) |
| integer | narrower integer | only the low bits are kept (`300i32 as u8` is 44) |
| integer | same width, other sign | same bits, read differently (`-1i32 as u32` is 4294967295) |
| float | integer | fraction dropped; too large or too small gives the nearest end of the range; NaN gives 0 |
| integer | float | nearest float (large integers lose precision) |
| `bool` | integer | 0 or 1 |
| `char` | integer | the code point, cut to the target width |
| `u8` | `char` | the character with that code |

```rust
assert_eq!(300i32 as u8, 44); // 300 = 256 + 44
assert_eq!(200u8 as i8, -56); // same bits, read as signed
assert_eq!(3.99f64 as i32, 3); // toward zero
assert_eq!(-3.99f64 as i32, -3);
assert_eq!(1e10f64 as i32, i32::MAX); // stops at the end of the range
assert_eq!(f64::NAN as i32, 0);
assert_eq!((true as u8, 'A' as u8, 97u8 as char), (1, 65, 'a'));
assert_eq!(16_777_217i32 as f32, 16_777_216.0); // f32 cannot hold this integer
```

When the value might not fit and that would be a bug, do not use `as`. Use
`From` for conversions that can never fail and `TryFrom` for conversions that
can:

```rust
assert_eq!(i64::from(7i32), 7); // always fits, so no error is possible
assert!(u8::try_from(300i32).is_err()); // tells you instead of changing the value
assert_eq!(char::from_u32(0x20AC), Some('€'));
assert_eq!(char::from_u32(0xD800), None); // not every number is a character
```

### Closures

A **closure** is a function value written in place: `|parameters| body`. Unlike
a `fn` item, it can use the local variables around it. The variables it uses
are its **captures**, and they are stored inside the closure value. The
compiler picks the weakest capture that makes the body work:

- the body only reads a variable: the closure holds a shared reference to it;
- the body changes a variable: the closure holds a mutable reference;
- the body gives the variable away (moves it): the closure takes ownership.

The keyword **`move`** before the closure makes it take every captured variable
by value. For a `Copy` type that is a copy; for other types it is a move.

```rust
// Reads `rate`: captured by shared reference, so `rate` stays usable.
let rate = 0.2;
let with_tax = |price: f64| price * (1.0 + rate);
assert_eq!(with_tax(10.0), 12.0);
assert_eq!(rate, 0.2);

// Changes `total`: captured by mutable reference. The closure itself
// must be `mut`, because calling it changes what it has captured.
let mut total = 0;
let mut add = |n: i32| total += n;
add(5);
add(7);
assert_eq!(total, 12); // usable again after the closure's last call

// `move` puts the String inside the closure, so the closure can be returned.
fn prefixer(prefix: String) -> impl Fn(&str) -> String {
    move |name| format!("{prefix}{name}")
}
let tag = prefixer(String::from("user-"));
assert_eq!(tag("ada"), "user-ada");
assert_eq!(tag("lin"), "user-lin"); // callable many times
```

```text
with_tax:  [ &rate ]                  a reference to the f64 outside
add:       [ &mut total ]             an exclusive reference to the i32
tag:       [ prefix: String ]         owns the String (pointer, length, capacity)
```

`impl Fn(&str) -> String` means "some closure type that can be called many
times with a `&str`". `move` does not limit how often a closure can be called.
A closure can be called only once when its *body* uses up something it owns:

```rust,compile_fail,E0382
let label = String::from("report");
let consume = move || label; // the body gives `label` away
let first = consume();
let second = consume();
```

```text
error[E0382]: use of moved value: `consume`
4 |     let first = consume();
  |                 --------- `consume` moved due to this call
5 |     let second = consume();
  |                  ^^^^^^^ value used here after move
note: closure cannot be invoked more than once because it moves the variable `label` out of its environment
```

Chapter 14 covers the three closure traits (`Fn`, `FnMut`, `FnOnce`) in depth.

### Mistakes people make

**Returning a closure that borrows a local.** The local dies when the function
returns, so the closure must own what it uses.

```rust,compile_fail,E0373
fn scaler(factor: f64) -> impl Fn(f64) -> f64 {
    |x| x * factor
}
```

```text
error[E0373]: closure may outlive the current function, but it borrows `factor`, which is owned by the current function
2 |     |x| x * factor
  |     ^^^     ------ `factor` is borrowed here
  |     |
  |     may outlive borrowed value `factor`
help: to force the closure to take ownership of `factor` (and any other referenced variables), use the `move` keyword
  |
2 |     move |x| x * factor
  |     ++++
```

**A changing closure that is not `mut`.**

```rust,compile_fail,E0596
let mut total = 0;
let add = |n: i32| total += n;
add(5);
```

```text
error[E0596]: cannot borrow `add` as mutable, as it is not declared as mutable
3 |     let add = |n: i32| total += n;
  |                        ----- calling `add` requires mutable binding due to mutable borrow of `total`
4 |     add(5);
  |     ^^^ cannot borrow as mutable
help: consider changing this to be mutable
  |
3 |     let mut add = |n: i32| total += n;
  |         +++
```

**Casts that are not allowed.** An integer cannot become a `bool`, and only a
`u8` can become a `char`, because not every number is a valid character.

```rust,compile_fail,E0054
let flag = 1u8;
let on = flag as bool;
```

```text
error[E0054]: cannot cast `u8` as `bool`
help: compare with zero instead
  |
3 -     let on = flag as bool;
3 +     let on = flag != 0;
```

### When to use which

- The target type holds every value of the source type: `Type::from(value)`.
- It might not fit, and that is an error: `Type::try_from(value)`.
- You *want* the low bits, or the documented float rule: `as`, with a comment.
- A helper needs local variables, or is passed to another function: a closure.
  A helper that needs none: a `fn` item.

### Read more

- [Type cast expressions](https://doc.rust-lang.org/reference/expressions/operator-expr.html#type-cast-expressions)
  in the Rust Reference
- [`From`](https://doc.rust-lang.org/std/convert/trait.From.html) and
  [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html) in the standard library documentation
- [Closure expressions](https://doc.rust-lang.org/reference/expressions/closure-expr.html)
  and [closure types](https://doc.rust-lang.org/reference/types/closure.html) in the Rust Reference

## Lesson 09 · Capstone: command expression engine

**Assumed:** everything in Lessons 01 to 08, and `str::parse` and
`checked_add` from Chapter 3.

The capstone gives you a list of text lines and asks for one result: a sum or
the number of the line that went wrong. No new language feature is needed. The
work is to decide, for every kind of line, which control-flow tool it needs.

A good way to start is to separate two questions.

1. **What is this one line?** Write that as a function from text to a value
   that names every case. The capstone gives you the type
   `Result<Option<i32>, ()>` for this: "a number", "nothing to add" and "not
   valid" are three different answers. Trim first, so that every later test
   sees clean text.
2. **What does the loop do with each answer?** Make a small table before you
   write code:

```text
kind of line        effect on the loop
------------        ------------------
blank or comment    go to the next line
number              add it, then go to the next line
stop command        leave the loop with the sum so far
anything else       leave the function with an error
```

Each row is one construct from this chapter: `continue`, an addition, `break`
and `return`. `let else` fits the rows that leave early, because it keeps the
happy path flat.

Three details decide whether the tests pass.

- **Line numbers.** The error carries the number of the line as a person
  counts: the first line is 1, and skipped lines still count. Ask the loop for
  the position of each line instead of counting yourself, and convert from a
  zero-based position in one place only.
- **Overflow.** The sum is an `i32`. The contract wants the line where the sum
  first leaves the `i32` range. A wider total checked at the end would find
  *that* something went wrong but not *where*, and it would miss a sum that
  goes out of range and comes back.
- **No allocation.** Every line is already a `&str`. Trimming gives another
  `&str` into the same text. Nothing needs a `String` or a `Vec`.

When you finish, read your loop once more and name the statement that makes
each pass move forward. Then do the lab in the lesson header: adding one new
command is the real test of whether your structure is clear.

### Read more

- [`str::parse`](https://doc.rust-lang.org/std/primitive.str.html#method.parse)
  and [`i32::checked_add`](https://doc.rust-lang.org/std/primitive.i32.html#method.checked_add)
  in the standard library documentation
- [`Iterator::enumerate`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.enumerate)
  in the standard library documentation
