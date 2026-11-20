# Chapter 22 concepts — Writing macros

[Chapter guide](README.md) · [Hints](HINTS.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. The sections follow the study order: lesson 08
comes before the capstone, which is lesson 07. Every code block in this file
was compiled and run with the course toolchain (Rust 1.99.0, edition 2024), and
every compiler message was copied from that compiler. Long messages are
shortened: a line with `...` marks a cut inside a message, and notes at the end
of a message are often left out.

## What you will be able to do

After this chapter you can:

- write a `macro_rules!` macro that behaves like a function call: each argument
  is evaluated once, and the macro's own variables stay out of the caller's way;
- accept any number of arguments with a repetition, including an optional
  trailing comma;
- generate items (functions, structs, modules, constants) from a short
  description;
- write a recursive macro for nested input, and a token muncher for input whose
  parts have different shapes;
- export a macro so that it works from any module and from any crate;
- read the errors that the compiler gives for a macro call that does not match.

## Before you start

Expressions and blocks (Chapter 6), modules and
visibility (Chapter 8), structs, enums and patterns (Chapters 9 and 10), and
traits (Chapter 11). Two lessons use `BTreeMap`; you need only `new` and
`insert`. The review lesson draws on Chapters 15, 19 and 21.

## Time

These are estimates. They include reading the section, the
predictions, the code, and your notes.

| Lesson | Estimate |
| --- | --- |
| 01 · Expansion and evaluation once | 40 minutes |
| 02 · Repetition and separators | 45 minutes |
| 03 · Type and identifier fragments | 50 minutes |
| 04 · Recursive structured literals | 60 minutes |
| 05 · Traits handle macro leaf types | 35 minutes |
| 06 · Exports and hygiene (with the export lab) | 60 minutes |
| 08 · Patterns, items, and token munchers | 100 minutes |
| 07 · Capstone | 45 minutes |
| R01 review, C01 code review, drills | 120 minutes |

## Words used in this chapter

- **Token**: the smallest piece of source text that has a meaning: a name, a
  number, a string, an operator such as `+`, a bracket.
- **Token tree**: one token that is not a bracket, or a whole group from an
  opening bracket to its closing bracket, with everything inside.
- **Rule**: one line of a macro, written `(matcher) => { transcriber };`.
- **Matcher**: the left side of a rule. It describes the input that the rule
  accepts.
- **Transcriber**: the right side of a rule. It is the code that the macro call
  is replaced with.
- **Fragment**: a named part of the input, written `$name:kind` in the matcher,
  for example `$value:expr`.
- **Expansion**: replacing a macro call with the code from the transcriber. The
  word is also used for the resulting code.

## Lesson 01 · Expansion and evaluation once

### The idea

A function works on values while the program runs. A macro works on source
code while the program is compiled. The compiler sees the call `name!(...)`,
finds the first rule whose matcher fits the tokens between the brackets, and
puts the rule's transcriber in place of the call. Only then does it check names
and types, exactly as if you had typed the expanded code yourself.

In the matcher, `$a:expr` means "one expression; call it `$a`". In the
transcriber, `$a` means "put that expression here". If the transcriber says
`$a` twice, the caller's expression is in the program twice, and it runs twice.

### Worked example

The macro `gap!` gives the distance between two numbers. The first version
looks correct:

```rust
macro_rules! gap_naive {
    ($a:expr, $b:expr) => {
        if $a > $b { $a - $b } else { $b - $a }
    };
}

let mut reads = 0;
let mut sensor = || {
    reads += 1;
    reads * 10
};
let gap = gap_naive!(sensor(), 25);
assert_eq!(gap, 5); // not 15
assert_eq!(reads, 2); // the sensor was read twice
```

The call became `if sensor() > 25 { sensor() - 25 } else { 25 - sensor() }`.
The first reading was 10, the second was 20, and the result mixes the two.

The repair is to store each argument in a local variable, once, and to use the
variable after that:

```rust
macro_rules! gap {
    ($a:expr, $b:expr) => {{
        let (a, b) = ($a, $b);
        if a > b { a - b } else { b - a }
    }};
}

let mut reads = 0;
let mut sensor = || {
    reads += 1;
    reads * 10
};
let a = 1000;
let gap = gap!(sensor(), 25);
assert_eq!(gap, 15);
assert_eq!(reads, 1);
assert_eq!(a, 1000); // the macro's `a` is not this `a`
```

```text
you write:        let gap = gap!(sensor(), 25);

the compiler      let gap = {
continues with:       let (a, b) = (sensor(), 25);
                      if a > b { a - b } else { b - a }
                  };
```

Two things in this version need a comment.

**The double braces.** The outer braces only mark where the transcriber begins
and ends. The inner braces are part of the output: they make a block, and a
block may contain `let` statements and still be used as an expression.

**The names `a` and `b`.** A local variable that the macro itself declares is a
different variable from any variable in the caller, even if both are spelled
the same. This property is called *hygiene*. The last assertion shows it: the
caller's `a` is still 1000. Lesson 06 returns to hygiene.

### Mistakes people make

**A `let` without the inner braces.** With a single pair of braces, the
expansion is two statements in a place where one expression is expected:

```rust,compile_fail
macro_rules! gap {
    ($a:expr, $b:expr) => {
        let (a, b) = ($a, $b);
        if a > b { a - b } else { b - a }
    };
}
fn main() {
    let n = gap!(4, 9);
    println!("{n}");
}
```

```text
error: expected expression, found `let` statement
 --> src/main.rs:3:9
  |
3 |         let (a, b) = ($a, $b);
  |         ^^^
...
8 |     let n = gap!(4, 9);
  |             ---------- in this macro invocation
```

Fix: write `=> {{ ... }};` so that the expansion is one block.

**Thinking of an argument as a value.** An argument is code, and it is placed
inside the caller's function. A `return`, a `break` or a `?` in an argument
therefore acts on the caller:

```rust
macro_rules! gap {
    ($a:expr, $b:expr) => {{
        let (a, b) = ($a, $b);
        if a > b { a - b } else { b - a }
    }};
}

fn find(limit: i32) -> i32 {
    let n = gap!(if limit > 10 { return -1 } else { limit }, 1);
    n + 1
}
assert_eq!(find(3), 3);
assert_eq!(find(50), -1); // the `return` left `find`, not the macro
```

A function argument cannot do this, because it is evaluated before the call.

**Expecting text replacement.** An `expr` fragment is one finished expression,
not a piece of text. `$x * $x` with `2 + 3` for `$x` means `(2 + 3) * (2 + 3)`.
You do not need extra parentheses around `$x`, as you would in a C macro.

### Built-in macros that describe the call site

`line!()`, `column!()`, `file!()` and `module_path!()` expand to the position of
the code that calls them. When you write them inside your own macro, they
report the place where *your macro* was called, which is nearly always what you
want. `stringify!(tokens)` turns its tokens into a string literal without
evaluating them.

### When to use which

Write a function when a function can do the job: it has types in its
signature, it gives clearer errors, and editors understand it better. Write a
macro when you need something that a function cannot have: a variable number
of arguments, new syntax, the caller's file and line, or generated items.

### Read more

- [Macros by example](https://doc.rust-lang.org/reference/macros-by-example.html) in the Rust Reference.
- [`line!`](https://doc.rust-lang.org/std/macro.line.html), [`module_path!`](https://doc.rust-lang.org/std/macro.module_path.html) and [`stringify!`](https://doc.rust-lang.org/std/macro.stringify.html) in the standard library.

## Lesson 02 · Repetition and separators

### The idea

A function has a fixed number of parameters. A macro can accept "any number of
these" with a *repetition*. In the matcher, `$( ... ),*` means: the part in
the parentheses, zero or more times, with a comma between two neighbours. In
the transcriber, `$( ... )*` writes its content once for each time the matcher
part was found.

| Operator | Meaning |
| --- | --- |
| `$( ... )*` | zero or more times |
| `$( ... )+` | one or more times |
| `$( ... )?` | zero times or once |

The token just before `*` or `+` is the separator. It is optional, and it need
not be a comma: `$( ... );*` separates with semicolons. `?` takes no separator.

A separator stands *between* elements. `1, 2, 3,` has a comma after the last
element, and `$($x:expr),*` does not accept it. The usual addition is
`$(,)?` at the end of the matcher: "an optional comma".

This lesson assumes lesson 01.

### Worked example

`longest!` gives the length of the longest of its string arguments. The
transcriber repeats one statement:

```rust
macro_rules! longest {
    ($($text:expr),* $(,)?) => {{
        let mut best = 0;
        $( best = best.max($text.len()); )*
        best
    }};
}

assert_eq!(longest!("fig", "banana", "kiwi",), 6);
assert_eq!(longest!(), 0);
```

```text
longest!("fig", "banana", "kiwi",)

    {
        let mut best = 0;
        best = best.max("fig".len());       first repetition
        best = best.max("banana".len());    second
        best = best.max("kiwi".len());      third
        best
    }
```

Each `$text` appears once in its statement, so each argument is evaluated
once, in the order in which it was written. With no arguments, the block
contains no `best = ...` statement at all, and the compiler warns that `mut`
is not needed. `#[allow(unused_mut)]` on the `let` silences that.

Repetitions can be nested. Each `$` variable must then be used inside as many
`$( ... )` layers as it was matched in:

```rust
macro_rules! grid {
    ($([$($cell:expr),*]),*) => {
        vec![$(vec![$($cell),*]),*]
    };
}

let g = grid![[1, 2, 3], [4, 5, 6]];
assert_eq!(g.len(), 2);
assert_eq!(g[1], [4, 5, 6]);
```

`$cell` was matched two layers deep (a row, then a cell), so the transcriber
uses it two layers deep.

### Mistakes people make

**No rule for the trailing comma.**

```rust,compile_fail
macro_rules! list {
    ($($x:expr),*) => { vec![$($x),*] };
}
fn main() {
    let v = list![1, 2, 3,];
    println!("{v:?}");
}
```

```text
error: unexpected end of macro invocation
 --> src/main.rs:5:27
  |
1 | macro_rules! list {
  | ----------------- when calling this macro
...
5 |     let v = list![1, 2, 3,];
  |                           ^ missing tokens in macro arguments
  |
note: while trying to match meta-variable `$x:expr`
```

After the last comma the matcher expects one more expression. Fix: add
`$(,)?` after the repetition.

**A repeated variable used outside a repetition.**

```rust,compile_fail
macro_rules! pairs {
    ($($name:ident = $value:expr),*) => { vec![(stringify!($name), $value)] };
}
fn main() {
    let v = pairs!(a = 1, b = 2);
    println!("{v:?}");
}
```

```text
error: variable `name` is still repeating at this depth
 --> src/main.rs:2:60
  |
2 |     ($($name:ident = $value:expr),*) => { vec![(stringify!($name), $value)] };
  |                                                            ^^^^^
```

`$name` stands for many names. The transcriber must say where they go:
`vec![$((stringify!($name), $value)),*]`.

**An empty call that leaves a type open.** The expansion of an empty call has
no elements, so nothing tells the compiler the element type:

```rust,compile_fail,E0282
macro_rules! list {
    ($($x:expr),* $(,)?) => { vec![$($x),*] };
}
fn main() {
    let v = list![];
    println!("{}", v.len());
}
```

```text
error[E0282]: type annotations needed for `Vec<_>`
 --> src/main.rs:5:9
  |
5 |     let v = list![];
  |         ^
```

This is not a fault of the macro. The caller writes the type:
`let v: Vec<u8> = list![];`.

### When to use which

Use `*` when an empty call has a sensible meaning (an empty list), and `+`
when it does not (the largest of no numbers). With `+`, an empty call is
rejected at compile time with "unexpected end of macro invocation".

### Read more

- [Repetitions](https://doc.rust-lang.org/reference/macros-by-example.html#repetitions) in the Rust Reference.

## Lesson 03 · Type and identifier fragments

### The idea

Lessons 01 and 02 used one kind of fragment, `expr`. There are more. The kind
tells the macro's parser what to read from the input:

| Fragment | Reads | Examples of input |
| --- | --- | --- |
| `expr` | one expression | `x + 1`, `f(2)`, `"text"` |
| `ty` | one type | `u8`, `Vec<String>`, `&'static str` |
| `ident` | one identifier or keyword | `total`, `Meters`, `type` |
| `literal` | one literal, with an optional `-` | `7`, `-5`, `"hi"`, `true` |
| `tt` | one token tree | `x`, `+`, `(a, b)`, `[1, 2]` |

Lesson 08 adds `pat`, `item` and `vis`.

A macro call can stand where an item is expected, for example at the top of a
module. Its expansion must then consist of items. This is how a macro
generates functions, structs and `impl` blocks. With `ident` the caller gives
the *name* of the generated item, and with `ty` a type that it uses.

The macro checks only the shape of its input: "here is an identifier, here is
a type". All other rules of Rust (types, privacy, borrowing) are checked
afterwards, on the expanded code.

### Worked example

Many integer types need the same small function. One macro writes it for each:

```rust
macro_rules! sum_fn {
    ($name:ident : $ty:ty) => {
        fn $name(values: &[$ty]) -> Option<$ty> {
            let mut total: $ty = 0;
            for &value in values {
                total = total.checked_add(value)?;
            }
            Some(total)
        }
    };
}

sum_fn!(sum_u8: u8);
sum_fn!(sum_i64: i64);

assert_eq!(sum_u8(&[100, 100]), Some(200));
assert_eq!(sum_u8(&[200, 100]), None); // 300 does not fit in a u8
assert_eq!(sum_i64(&[-5, 7]), Some(2));
```

The transcriber has a single pair of braces here, because the expansion is an
item, not an expression. The caller can call `sum_u8` because the caller wrote
that name. A name that comes in through `$name:ident` belongs to the caller. A
name that the macro writes itself, such as `total`, belongs to the macro.

### A captured fragment is closed

After `$e:expr` has matched `x`, the macro holds "an expression", not the token
`x`. If it passes `$e` on to a second macro, that macro can match it as `expr`
again, but not as `ident` and not as a specific token:

```rust
macro_rules! is_zero_token {
    (0) => { true };
    ($other:tt) => { false };
}
macro_rules! through_expr {
    ($e:expr) => { is_zero_token!($e) };
}
macro_rules! through_tt {
    ($t:tt) => { is_zero_token!($t) };
}

assert!(is_zero_token!(0));
assert!(through_tt!(0)); // a tt stays open
assert!(!through_expr!(0)); // an expr is closed: no longer the token `0`
```

Only `tt`, `ident` and `lifetime` captures stay open. There is one exception
worth knowing: a closed `expr` that is a literal can still match a
`$x:literal` matcher.

### The `expr` fragment and the edition

Since edition 2024, `expr` also accepts the expression `_` and `const { ... }`
blocks. The fragment `expr_2021` keeps the older, narrower meaning. You meet
it in macros that were written before 2024 and migrated automatically.

```rust
macro_rules! now {
    ($e:expr) => { "expr" };
    (_) => { "underscore" };
}
macro_rules! before {
    ($e:expr_2021) => { "expr" };
    (_) => { "underscore" };
}
assert_eq!(now!(_), "expr");
assert_eq!(before!(_), "underscore");
```

### Mistakes people make

**The input has the wrong shape for the fragment.** An expression is not a
type:

```rust,compile_fail
macro_rules! sum_fn {
    ($name:ident : $ty:ty) => {
        fn $name(values: &[$ty]) -> Option<$ty> { values.first().copied() }
    };
}
sum_fn!(sum_u8: 1 + 2);
fn main() {}
```

```text
error: no rules expected `1`
 --> src/main.rs:6:17
  |
1 | macro_rules! sum_fn {
  | ------------------- when calling this macro
...
6 | sum_fn!(sum_u8: 1 + 2);
  |                 ^ no rules expected this token in macro call
  |
note: while trying to match meta-variable `$ty:ty`
```

Read the `note` line first: it names the fragment that could not be read.

**`ident` where a type is needed.** `ident` reads exactly one name. If the
matcher says `$ty:ident`, then `u8` works, but
`sum_fn!(total: std::num::Wrapping<u8>)` stops at the first `::`:

```text
error: no rules expected `::`
...
note: while trying to match meta-variable `$ty:ident`
```

Fix: use `ty` for anything that is used as a type.

**Trusting the macro to check meaning.** `ident` accepts keywords such as
`type` and raw identifiers such as `r#type`. `ty` accepts the name of a type
that does not exist. The errors for such input come later, from the expanded
code, and they point into your macro.

### When to use which

Use the most specific fragment that fits: `ident` for a name that the macro
defines or passes to `stringify!`, `ty` for types, `literal` when only a
literal makes sense, `expr` for values. Use `tt` when the input must be passed
on to another macro and matched again there.

### Read more

- [Metavariables](https://doc.rust-lang.org/reference/macros-by-example.html#metavariables) and [forwarding a matched fragment](https://doc.rust-lang.org/reference/macros-by-example.html#forwarding-a-matched-fragment) in the Rust Reference.
- [Macro fragment specifiers](https://doc.rust-lang.org/edition-guide/rust-2024/macro-fragment-specifiers.html) in the Edition Guide.

## Lesson 04 · Recursive structured literals

### The idea

Some input is nested: a list that contains lists, an expression that contains
expressions. One matcher cannot describe "any depth". A recursive macro can:
it has one rule for each form, and where a form contains smaller parts, its
transcriber calls the same macro again for each part.

Two facts make this work.

**A bracketed group is one token tree.** `tt` reads one token that is not a
bracket, or one complete group in `()`, `[]` or `{}`, however much is inside.
So a rule can take "the next part, whatever it is" as `$part:tt` and hand it to
the recursive call, which looks inside.

```text
f  (x, y)  +  [1, 2]
│    │     │     │
tt   tt    tt    tt        four token trees

-2    is two token trees:  -   2
```

**Rules are tried from the top.** The first rule that matches the whole input
wins. Specific rules must therefore stand before general ones.

This lesson assumes lessons 01 to 03, in particular that a `tt` capture stays
open when it is passed on.

### Worked example

`calc!` evaluates a small prefix notation: `(add a b)` and `(mul a b)`, where
`a` and `b` are numbers or further groups.

```rust
macro_rules! calc {
    ((add $a:tt $b:tt)) => { calc!($a) + calc!($b) };
    ((mul $a:tt $b:tt)) => { calc!($a) * calc!($b) };
    ($n:literal) => { $n };
}

assert_eq!(calc!((add 1 (mul 2 3))), 7);
assert_eq!(calc!((mul (add 1 2) (add 3 4))), 21);
assert_eq!(calc!(5), 5);
```

```text
calc!((add 1 (mul 2 3)))
  rule 1:  $a = 1    $b = (mul 2 3)
  becomes  calc!(1) + calc!((mul 2 3))
              │              │
           rule 3         rule 2:  $a = 2   $b = 3
              1           becomes  calc!(2) * calc!(3)
                                      2          3
```

The matcher `((add $a:tt $b:tt))` has two pairs of parentheses. The outer pair
belongs to the rule syntax. The inner pair must be present in the input.

The words `add` and `mul` in the matchers are plain tokens. The input must
contain exactly these tokens at these places. No function named `add` exists.

### Mistakes people make

**An element that is more than one token tree.**

```rust,compile_fail
macro_rules! calc {
    ((add $a:tt $b:tt)) => { calc!($a) + calc!($b) };
    ((mul $a:tt $b:tt)) => { calc!($a) * calc!($b) };
    ($n:literal) => { $n };
}
fn main() {
    println!("{}", calc!((add 1 -2)));
}
```

```text
error: no rules expected `2`
 --> src/main.rs:7:34
  |
1 | macro_rules! calc {
  | ----------------- when calling this macro
...
7 |     println!("{}", calc!((add 1 -2)));
  |                                  ^ no rules expected this token in macro call
  |
note: while trying to match `)`
```

`$b:tt` took the `-`, and then the rule expected `)` but found `2`. The caller
can write `(add 1 (-2))`: a parenthesised group is one tree. The last rule
would then need to accept more than a bare literal, for example `($e:expr)`.
Say such limits in the macro's documentation.

**The general rule first.** If a rule `($e:expr) => { ... }` stood at the top
of a macro, it would accept every input that is an expression, and a later
rule for a special word such as `nothing` would never be used: `nothing` is
also a valid expression (a path). There is no error message for this. The
result is simply wrong, or a "cannot find value" error appears after the
expansion.

**Passing a nested part on as `expr`.** If the recursive call receives a closed
`expr` capture (lesson 03), the specific rules can no longer look at its
tokens. Take nested parts as `tt`.

**Recursion without an end.** Every recursive call must receive smaller input,
or a rule must end the recursion. Otherwise:

```rust,compile_fail
macro_rules! forever {
    ($($t:tt)*) => { forever!(x $($t)*) };
}
fn main() {
    forever!();
}
```

```text
error: recursion limit reached while expanding `forever!`
 --> src/main.rs:2:22
  |
2 |     ($($t:tt)*) => { forever!(x $($t)*) };
  |                      ^^^^^^^^^^^^^^^^^^
...
  = help: consider increasing the recursion limit by adding a `#![recursion_limit = "256"]` attribute to your crate (`main`)
```

The limit is 128 nested expansions unless the crate changes it. It also
bounds honest recursion, so a recursive macro is for input of modest depth.

### When to use which

Use a plain repetition for a flat list. Use recursion when elements can
contain elements. If the nested syntax grows beyond a few forms, a macro
becomes hard to read and its errors hard to understand; a function that takes
ordinary data, or a parser that runs at run time, is then the better tool.

### Read more

- [Macro invocation and token trees](https://doc.rust-lang.org/reference/macros.html) in the Rust Reference.
- [The `recursion_limit` attribute](https://doc.rust-lang.org/reference/attributes/limits.html#the-recursion_limit-attribute) in the Rust Reference.

## Lesson 05 · Traits handle macro leaf types

### The idea

A macro sees tokens. It does not see types. When the input is `value`, the
macro cannot know whether `value` is a number, a string or a `bool`, because
that is decided later, after expansion.

Often a macro must still treat arguments of different types differently. The
solution is a division of work:

- the **macro** handles the shape of the input: how many arguments, in what
  order, with what separators;
- a **trait** handles the types: the macro wraps each argument in one call of
  a trait method or of a generic function, and the compiler chooses the right
  implementation for each argument, as it does for hand-written code.

This lesson assumes lesson 02 and traits from Chapter 11.

### Worked example

`mean!` accepts numbers of different types and returns their mean as `f64`.
The standard trait `From` does the type work: `f64::from(x)` exists for every
number type that converts to `f64` without loss.

```rust
macro_rules! mean {
    ($($x:expr),+ $(,)?) => {{
        let values = [$(f64::from($x)),+];
        values.iter().sum::<f64>() / values.len() as f64
    }};
}

let small: u8 = 1;
assert_eq!(mean!(small, 2.5_f32, 3_i32, -0.5), 1.5);
assert_eq!(mean!(7_u16), 7.0);
```

```text
mean!(small, 2.5_f32, 3_i32, -0.5)

    [ f64::from(small),      uses  impl From<u8>  for f64
      f64::from(2.5_f32),    uses  impl From<f32> for f64
      f64::from(3_i32),      uses  impl From<i32> for f64
      f64::from(-0.5) ]      uses  impl From<f64> for f64 (a value converts to its own type)
```

The macro wrote the same text four times. The compiler made four different
calls from it. To support a new type, you add an `impl`, and the macro stays
as it is. When no standard trait fits, you write a small trait of your own,
with one method, and implement it for each supported type.

The macro uses `+`, not `*`: the mean of no numbers has no value.

### Mistakes people make

**An argument of a type without the implementation.** The macro accepts it,
because the shape is right. The error comes from the expanded code:

```rust,compile_fail,E0277
macro_rules! mean {
    ($($x:expr),+ $(,)?) => {{
        let values = [$(f64::from($x)),+];
        values.iter().sum::<f64>() / values.len() as f64
    }};
}
fn main() {
    println!("{}", mean!(1_u8, 10_u64));
}
```

```text
error[E0277]: the trait bound `f64: From<u64>` is not satisfied
 --> src/main.rs:3:25
  |
3 |         let values = [$(f64::from($x)),+];
  |                         ^^^ the trait `From<u64>` is not implemented for `f64`
...
8 |     println!("{}", mean!(1_u8, 10_u64));
  |                    ------------------- in this macro invocation
```

This is a good error: it names the type and the missing trait. A `u64` does
not always fit in an `f64` exactly, so the standard library offers no `From`.
The caller converts explicitly: `mean!(1_u8, 10_u64 as f64)`.

**Deciding by spelling.** A rule such as `(true) => { ... }` matches the
*token* `true`. It does not match a variable that holds `true`. Rules with
literal tokens are right for syntax (lesson 04's `add` and `mul`). They are
wrong for anything that depends on a value or a type.

**Forgetting that a bare number has no type yet.** `7` alone is "some
integer". If exactly one implementation could apply to an integer, the
compiler takes it. If several could, it does not search for the best one: it
decides that the number is an `i32` (an `f64` for a number with a decimal
point) and then looks for an implementation for that type. So `f64::from(7)`
compiles, because `f64: From<i32>` exists, and `u64::from(7)` is an `E0277`
error, because `u64: From<i32>` does not. Write a suffix (`7_u8`) when the
type matters.

### When to use which

Put a decision in the macro when it depends on what the caller *wrote* (a
keyword, the number of arguments, a bracket). Put it in a trait when it depends
on what a value *is*. A generic function `fn convert<T: MyTrait>(value: T)`
between the two is useful: the macro calls the function, the function has a
clear signature, and an unsupported type produces one clear `E0277` error.

### Read more

- [`From`](https://doc.rust-lang.org/std/convert/trait.From.html) in the standard library.
- [Trait bounds](https://doc.rust-lang.org/reference/trait-bounds.html) in the Rust Reference.

## Lesson 06 · Exports and hygiene

### The idea

An expansion contains names. Where is each name looked up? There are two
answers, for two kinds of names.

**Local variables are hygienic.** A `let` variable that the macro writes can
be seen only by code that the macro writes. Code from the caller cannot see
it, and it cannot see the caller's variables. You saw this in lesson 01.

**Everything else is looked up where the macro is called.** The name of a
function, a type, a constant or a module in the transcriber is resolved after
expansion, in the module that contains the *call*. If the macro says
`helper(x)`, the caller's module must have a `helper` in scope. It may have
none, or a different one.

The fix for the second point is the special name `$crate`. It stands for the
crate in which the macro is *defined*, under whatever name the caller knows
that crate. A path that starts with `$crate::` always leads to the same item.

A macro itself is found in one of two ways:

- **Textual scope.** A `macro_rules!` definition is visible from the line
  after it to the end of its module, including child modules that come later
  in the text. A call above the definition does not see it.
- **By path.** `#[macro_export]` puts the macro at the root of its crate, in
  whichever module it was defined. Other modules call it as `crate::name!`,
  and other crates as `the_crate::name!`.

This lesson assumes lessons 01 and 02 and modules from Chapter 8.

### Worked example

```rust
mod geometry {
    pub fn area_of(w: u32, h: u32) -> u32 {
        w * h
    }

    #[macro_export]
    macro_rules! square_area {
        ($side:expr) => {{
            let side = $side;
            $crate::geometry::area_of(side, side)
        }};
    }
}

mod shop {
    // A different function with the same name, in the calling module.
    #[allow(dead_code)]
    fn area_of(_: u32, _: u32) -> u32 {
        0
    }

    pub fn tile() -> u32 {
        let side = 100;
        crate::square_area!(side - 97)
    }
}

fn main() {
    assert_eq!(shop::tile(), 9);
}
```

```text
crate root            square_area!   (placed here by #[macro_export])
├── mod geometry      area_of        <── $crate::geometry::area_of
└── mod shop          area_of        (not used by the expansion)
        tile() calls crate::square_area!(side - 97)
```

The caller's `side` (100) and the macro's `side` (3) are two variables.
`shop::area_of` plays no part, because the expansion names its helper with a
full path. The helper is `pub`: the expanded code is in `shop`, so it must be
allowed to call the function from there.

### Mistakes people make

**A bare helper name.**

```rust,compile_fail,E0425
mod geometry {
    pub fn area_of(w: u32, h: u32) -> u32 {
        w * h
    }
    #[macro_export]
    macro_rules! square_area {
        ($side:expr) => {{
            let side = $side;
            area_of(side, side)
        }};
    }
}
fn main() {
    println!("{}", square_area!(3));
}
```

```text
error[E0425]: cannot find function `area_of` in this scope
  --> src/main.rs:9:13
   |
 9 |             area_of(side, side)
   |             ^^^^^^^ not found in this scope
...
14 |     println!("{}", square_area!(3));
   |                    --------------- in this macro invocation
```

The message points at the macro, but "this scope" is the place of the call.
Inside `geometry` the same macro would work, which is why this mistake often
survives the author's own tests. Fix: `$crate::geometry::area_of`.

**`crate::` in place of `$crate::`.** Inside one crate the two mean the same.
In an exported macro they do not: after expansion in another crate, `crate`
means *that* crate. Tests in the defining crate cannot show the difference.
Only a second crate can; the export lab of this lesson is such a crate.

**A private helper.** With the full path but without `pub`:

```text
error[E0603]: function `area_of` is private
  --> src/main.rs:9:31
   |
 9 |             $crate::geometry::area_of(side, side)
   |                               ^^^^^^^ private function
```

For callers in other crates, every module on the path must be `pub` too.
Mark such helpers `#[doc(hidden)]` if they are not meant for direct use.

**A call above the definition.**

```rust,compile_fail
fn main() {
    println!("{}", shout!("hi"));
}
macro_rules! shout {
    ($s:expr) => { $s.to_uppercase() };
}
```

```text
error: cannot find macro `shout` in this scope
 --> src/main.rs:2:20
  |
2 |     println!("{}", shout!("hi"));
  |                    ^^^^^ consider moving the definition of `shout` before this call
```

### When to use which

- A macro for one file: define it above its first use. Nothing else is needed.
- A macro for several modules of one crate: define it in a module and add
  `pub(crate) use name;` below the definition. Other modules then call it by
  path, `crate::that_module::name!`, and it is not part of the public API.
- A macro for other crates: `#[macro_export]`, and `$crate::` in front of
  every item that the transcriber names.

```rust
mod util {
    macro_rules! double {
        ($x:expr) => { $x * 2 };
    }
    pub(crate) use double;
}

fn main() {
    assert_eq!(util::double!(4), 8);
}
```

### Read more

- [Scoping, exporting, and importing](https://doc.rust-lang.org/reference/macros-by-example.html#scoping-exporting-and-importing) and [hygiene](https://doc.rust-lang.org/reference/macros-by-example.html#hygiene) in the Rust Reference.

## Lesson 08 · Patterns, items, and token munchers

### Three more fragments

| Fragment | Reads | Typical use |
| --- | --- | --- |
| `pat` | one pattern, including `A \| B` | the caller chooses what to match or bind |
| `item` | one whole item, with its attributes and visibility | pass items through and add something in front |
| `vis` | a visibility such as `pub` or `pub(crate)`, **or nothing** | give generated items the caller's visibility |

Names that a caller's pattern binds belong to the caller, so the caller's own
expressions can use them:

```rust
macro_rules! each {
    ($p:pat in $source:expr => $body:expr) => {
        for $p in $source {
            $body;
        }
    };
}

let mut total = 0;
each!((count, price) in [(2, 10), (1, 5)] => total += count * price);
assert_eq!(total, 25);
```

An `item` capture is closed, like an `expr` (lesson 03): the macro can place
it in the output, but it cannot read the item's name. When the macro needs a
part, it matches the parts instead, and `vis` is one of them:

```rust
macro_rules! debuggable {
    ($($it:item)*) => { $( #[derive(Debug)] $it )* };
}
macro_rules! text_const {
    ($(#[$meta:meta])* $vis:vis $name:ident = $text:literal) => {
        $(#[$meta])*
        $vis const $name: &str = $text;
    };
}

debuggable! {
    struct Point { x: i32, y: i32 }
    pub enum Mode { Fast }
}
mod labels {
    text_const!(pub GREETING = "hello");
    text_const!(SECRET = "xyzzy"); // `$vis` matched nothing: private
    pub fn secret_len() -> usize { SECRET.len() }
}

assert_eq!(format!("{:?} {:?}", Point { x: 1, y: 2 }, Mode::Fast), "Point { x: 1, y: 2 } Fast");
assert_eq!((labels::GREETING, labels::secret_len()), ("hello", 5));
```

`$(#[$meta:meta])*` reads attributes. A doc comment is an attribute too.

### What may follow a fragment

The parser must know where a fragment ends, so the language limits the token
that may come after it in a matcher. After `expr`: only `=>`, `,` or `;`.
After `pat`: `=>`, `,`, `=`, `if` or `in`. After `ty`: a longer list that
includes `,`, `;`, `=`, `>` and `{`. A matcher that breaks the rule is rejected
when the macro is defined:

```rust,compile_fail
macro_rules! bad {
    ($e:expr $name:ident) => {};
}
fn main() {}
```

```text
error: `$e:expr` is followed by `$name:ident`, which is not allowed for `expr` fragments
 --> src/main.rs:2:14
  |
2 |     ($e:expr $name:ident) => {};
  |              ^^^^^^^^^^^ not allowed after `expr` fragments
  |
  = note: allowed there are: `=>`, `,` or `;`
```

### Token munchers and internal rules

A repetition treats every element alike and remembers nothing. It cannot
handle elements of different shapes, cannot number them, and cannot build a
result in another order. For these jobs a macro processes its input step by
step: a rule takes one element from the front, and calls the macro again with
the rest (`$($rest:tt)*`). This is a *token muncher*.

What one step must tell the next is passed in the recursive call, in front of
the remaining input. This carried state is an *accumulator*. Rules that only
the macro itself calls start with a marker that no caller would write; by
convention `@` and a word. They are *internal rules*.

`reversed!` builds an array in reverse order. The state is the list of
elements moved so far, kept in square brackets:

```rust
macro_rules! reversed {
    // Internal rules first.
    (@acc [$($done:expr),*]) => { [$($done),*] };
    (@acc [$($done:expr),*] $head:expr $(, $tail:expr)*) => {
        reversed!(@acc [$head $(, $done)*] $($tail),*)
    };
    // Entry rule: start with an empty accumulator.
    ($($item:expr),* $(,)?) => { reversed!(@acc [] $($item),*) };
}

assert_eq!(reversed!(1, 2, 3), [3, 2, 1]);
```

```text
reversed!(1, 2, 3)
reversed!(@acc []         1, 2, 3)
reversed!(@acc [1]        2, 3)
reversed!(@acc [2, 1]     3)
reversed!(@acc [3, 2, 1])
[3, 2, 1]
```

The array is written only at the end, because a macro call in expression
position must expand to one complete expression. A muncher in *item* position
is freer: each step may emit a finished item and then call the next step.

### Mistakes people make

**An entry rule above the internal rules.** A rule that does not fit is
normally skipped. But when a fragment parser has started on the input and then
fails, the compiler stops with an error and tries no further rule:

```rust,compile_fail
macro_rules! debuggable {
    ($($it:item)*) => { $( #[derive(Debug)] $it )* };
    (@version) => { 2 };
}
fn main() {
    println!("{}", debuggable!(@version));
}
```

```text
error: expected an item keyword
 --> src/main.rs:6:32
  |
2 |     ($($it:item)*) => { $( #[derive(Debug)] $it )* };
  |        -------- while parsing argument for this `item` macro fragment
...
6 |     println!("{}", debuggable!(@version));
  |                                ^
```

Fix: put rules that begin with a marker first.

**Looking inside a closed capture.** Passing `$it:item` to a rule that expects
`struct $name:ident;` fails with "no rules expected `item` metavariable". Match
the parts in the first place, or take the input as `$($t:tt)*`.

**Two separate lists in one repetition.** A transcriber cannot loop over one
repetition inside another one that was matched beside it. This attempt at a
multiplication table has a list of columns and a list of rows:

```rust,compile_fail
macro_rules! table {
    ([$($col:expr),*] $($row:expr),*) => {
        [$( [$($row * $col),*] ),*]
    };
}
fn main() {
    println!("{:?}", table!([1, 10] 2, 3));
}
```

```text
error: attempted to repeat an expression containing no syntax variables matched as repeating at this depth
 --> src/main.rs:3:15
  |
3 |         [$( [$($row * $col),*] ),*]
  |               ^^^^^^^^^^^^^
```

The outer `$( ... ),*` already walks through both lists, so nothing is left
for the inner one. A variable that was matched *outside* every repetition is
different: it may be written inside a repetition, and it is repeated each
time. So capture one whole list as a single `tt`, and let an internal rule
open it again, once for each element of the other list:

```rust
macro_rules! table {
    (@row [$($col:expr),*] $row:expr) => { [$($row * $col),*] };
    ($cols:tt $($row:expr),*) => { [$( table!(@row $cols $row) ),*] };
}
assert_eq!(table!([1, 10] 2, 3), [[2, 20], [3, 30]]);
```

**`pat_param` by accident.** `pat_param` reads a pattern without a top-level
`|`. Use it only when a `|` must follow the pattern in your own syntax.

**A long list.** Each step is one level of recursion, and the limit is 128
(lesson 04). A muncher suits some dozens of elements.

### When to use which

Use a repetition when all elements are alike. Use a muncher when elements
differ in shape, when an element's meaning depends on the ones before it, or
when the output order differs from the input order. If run-time code can do
the job (a loop over a slice), prefer it: a muncher is harder to read and
slower to compile.

### Read more

- [Follow-set ambiguity restrictions](https://doc.rust-lang.org/reference/macros-by-example.html#follow-set-ambiguity-restrictions) and [metavariables](https://doc.rust-lang.org/reference/macros-by-example.html#metavariables) in the Rust Reference.
- [Visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html) and [items](https://doc.rust-lang.org/reference/items.html) in the Rust Reference.

## Lesson 07 · Capstone: structured events

The capstone combines lessons 01, 02, 03 and 05 in one macro and one plain
function. It teaches no new rule. This section describes an order of work,
not the answer.

**1. Read the syntax as a grammar.** The matcher is given:
`kind ; name : value , name : value , ...`. Say for each part which fragment
reads it and why. The kind and the values are `expr`. A field name is `ident`,
because the macro turns it into text and never evaluates it.

**2. Write the expansion of one call by hand.** Take `event!("job"; id: 7)`
and write the block that you want the compiler to see. Mark the places that
repeat. This is the most useful habit in macro writing: first the code, then
the macro that produces it.

**3. Fix the order of evaluation.** The contract says: the kind first, then
each value once, in source order. Lesson 01 gives the tool (bind to a local
once), and lesson 02 the rule that a repeated statement runs in the order of
the arguments. Decide where the kind is bound relative to the repetition.

**4. Keep the macro thin.** The macro only collects; the function
`make_event` builds the value. Because the function takes ownership of its
inputs, nothing needs to be copied on the way. A thin macro in front of an
ordinary function is easier to test and gives better errors than a macro that
does everything.

**5. Let types do the type work.** A value can be a number, a `&str`, a
`char`. The macro cannot know which (lesson 05). One method of one standard
trait turns each into a `String`. A value without that trait gives an ordinary
`E0277` error, which is the behaviour you want.

**6. Think about wrong input.** With a string where a field name belongs, the
call matches no rule:

```text
error: no rules expected `"id"`
 --> src/main.rs:5:27
  |
5 |     let _ = event!("job"; "id": 7);
  |                           ^^^^ no rules expected this token in macro call
```

The lesson's compiler lab asks you to improve this with a last rule that
accepts anything and expands to `compile_error!`. That built-in macro stops
the compilation with your own message:

```rust,compile_fail
macro_rules! percent {
    ($n:literal) => { $n as f64 / 100.0 };
    ($($other:tt)*) => { compile_error!("percent! takes one number literal, for example percent!(40)") };
}
fn main() {
    println!("{}", percent!(forty));
}
```

```text
error: percent! takes one number literal, for example percent!(40)
```

A catch-all rule must be the last rule, and it must not stand above internal
rules (lesson 08).

When your tests pass, do the lesson's EXPLAIN task: write down the grammar,
the evaluation order, and what a duplicate field name means. Then ask what
would have to change if the macro were exported to other crates (lesson 06).

### Read more

- [`compile_error!`](https://doc.rust-lang.org/std/macro.compile_error.html) and [`stringify!`](https://doc.rust-lang.org/std/macro.stringify.html) in the standard library.
