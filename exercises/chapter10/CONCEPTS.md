# Chapter 10 concepts — Enums and pattern matching

[Chapter guide](README.md) · [Hints](HINTS.md)

Read one section, then do the lesson with the same number. Each section gives the idea, one worked example, the mistakes people really make with the compiler's own words, and links to the official documentation. The worked examples are never the exercise itself.

## What you will be able to do

After this chapter you will be able to:

- describe a value that is "one of several things" as an enum, and put the right data inside each variant;
- build recursive data, such as trees, with `Box`;
- write generic enums, and read `Option<T>` and `Result<T, E>` as the ordinary enums they are;
- take values apart with patterns: literals, tuples, structs, references, arrays, slices, ranges, alternatives, guards and `@` bindings;
- say which patterns are allowed in `let`, in parameters, in `for`, in `if let`, in `while let` and in `let ... else`;
- keep a public enum open for new variants with `#[non_exhaustive]`.

## Before you start

Chapter 3 (integers, arrays, slices, `String` and `&str`), Chapter 4 (ownership, moves, `std::mem::replace`), Chapter 5 (references and lifetimes), Chapter 6 (`match`, `if let` and `let ... else` as control flow), Chapter 7 (`Option`, `Result` and `?`) and Chapter 9 (structs, `impl` blocks, `derive`, `Cell`).

## Time

These are estimates for a first pass, including predictions and notes.

| Lesson | Estimate |
| --- | --- |
| 01 · Variants and payloads | 50 min |
| 02 · Recursive data with enums | 30 min |
| 03 · Generic enums | 30 min |
| 04 · Literal, tuple and struct patterns | 35 min |
| 05 · Matching references and mutating payloads | 40 min |
| 06 · Array and slice patterns | 30 min |
| 07 · Guards, alternatives and @ bindings | 35 min |
| 08 · Pattern contexts and public compatibility | 40 min, plus 30 min for the optional lab |
| 09 · Capstone: generic ordered tree | 75 min |
| R01, C01 and the drills | 40 min + 40 min + 30 min |

## Lesson 01 · Variants and payloads

**Assumed:** structs (Chapter 9), moves (Chapter 4), `match` as an expression (Chapter 6).

A struct says "this value has a name **and** an age **and** an address". An enum says "this value is a circle **or** a square **or** a triangle". Each alternative is called a *variant*. A value of an enum type is always exactly one of its variants.

A variant can carry data, called its *payload*. There are three shapes:

- a *unit-like* variant has no payload: `Cash`;
- a *tuple-like* variant has unnamed fields: `Voucher(String)`;
- a *struct-like* variant has named fields: `Card { last_digits: u16 }`.

In memory, an enum value holds a *discriminant* (a small number that says which variant this is) and the payload of that one variant. The whole type is as large as its largest variant, plus room for the discriminant if the compiler cannot hide it somewhere.

### Worked example

```rust
enum Payment {
    Cash,
    Card { last_digits: u16 },
    Voucher(String),
}

// Looking only: `payment` is borrowed, and no payload leaves it.
fn fee_cents(payment: &Payment) -> u32 {
    match payment {
        Payment::Cash => 0,
        Payment::Card { .. } => 30,
        Payment::Voucher(_) => 0,
    }
}

// Consuming: this function owns `payment`, so an arm may take a payload out.
fn receipt_line(payment: Payment) -> String {
    match payment {
        Payment::Cash => String::from("paid in cash"),
        Payment::Card { last_digits } => format!("card ending in {last_digits:04}"),
        Payment::Voucher(mut code) => {
            // `code` is the String from inside the variant. It was moved
            // here, so this arm owns it and may even change it.
            code.push_str(" (voucher)");
            code
        }
    }
}

fn main() {
    let card = Payment::Card { last_digits: 42 };
    assert_eq!(fee_cents(&card), 30);
    assert_eq!(receipt_line(card), "card ending in 0042");
    assert_eq!(fee_cents(&Payment::Cash), 0);
    assert_eq!(receipt_line(Payment::Cash), "paid in cash");
    let voucher = Payment::Voucher(String::from("SPRING"));
    assert_eq!(receipt_line(voucher), "SPRING (voucher)");
}
```

```text
Payment::Card { last_digits: 42 }        Payment::Voucher(String::from("SPRING"))

+-----+-------------+ - - - - +          +-----+---------+-----+----------+
| tag | last_digits | unused  |          | tag | pointer | len | capacity |
+-----+-------------+ - - - - +          +-----+----|----+-----+----------+
                                                    v
                                              heap: S P R I N G
```

Both values have the same size, because they have the same type. The sketch is a model: the real arrangement is the compiler's choice. On the pinned toolchain a `Payment` takes 24 bytes on a 64-bit target, exactly as much as a `String`, because the compiler stores the tag in bit patterns that a `String` never uses. That is an observation, not a promise.

A `match` on an enum must handle every variant. This is called *exhaustiveness*, and it is the main reason to use an enum: when you add a variant later, the compiler lists every `match` that needs a decision.

### Fieldless enums and numbers

An enum in which no variant has a payload is called *fieldless*. You may give its variants numbers and convert a variant to its number with `as`. A variant without a number gets the previous number plus one; the first variant gets 0.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Shelf {
    Floor = 1,
    Middle, // 2
    Top,    // 3
}

fn shelf_from_number(n: u8) -> Option<Shelf> {
    match n {
        1 => Some(Shelf::Floor),
        2 => Some(Shelf::Middle),
        3 => Some(Shelf::Top),
        _ => None,
    }
}

fn main() {
    assert_eq!(Shelf::Top as u8, 3);
    assert_eq!(shelf_from_number(2), Some(Shelf::Middle));
    assert_eq!(shelf_from_number(9), None);
    // Derived ordering compares the discriminants first.
    assert!(Shelf::Floor < Shelf::Top);
}
```

Derived `PartialOrd` and `Ord` compare the discriminants first, and the payloads only when both values are the same variant. If you give no numbers, the discriminants follow the order of declaration, so an earlier variant is smaller than a later one. If you give numbers, the numbers decide: with `enum E { A = 2, B = 1 }`, `E::B` is the smaller value. `std::mem::discriminant(&value)` gives a token that says which variant a value is, and ignores the payload.

### Mistakes people make

**Casting an enum that carries data.** `as` works only for fieldless enums.

```rust,compile_fail,E0605
enum Payment {
    Cash,
    Voucher(String),
}

fn main() {
    let n = Payment::Cash as u8;
}
```

```text
error[E0605]: non-primitive cast: `Payment` as `u8`
 --> src/main.rs:7:13
  |
7 |     let n = Payment::Cash as u8;
  |             ^^^^^^^^^^^^^^^^^^^ an `as` expression can be used to convert enum types to numeric types only if the enum type is unit-only or field-less
```

Fix: write a function with a `match` that returns the number you want.

**Casting a number to an enum.** `2 as Shelf` is rejected with the same error code, because most numbers are not a valid variant. Fix: write a checked conversion such as `shelf_from_number` above.

**Forgetting a variant.**

```rust,compile_fail,E0004
enum Payment {
    Cash,
    Card { last_digits: u16 },
    Voucher(String),
}

fn fee_cents(payment: &Payment) -> u32 {
    match payment {
        Payment::Cash => 0,
        Payment::Card { .. } => 30,
    }
}
```

```text
error[E0004]: non-exhaustive patterns: `&Payment::Voucher(_)` not covered
  --> src/lib.rs:8:11
   |
 8 |     match payment {
   |           ^^^^^^^ pattern `&Payment::Voucher(_)` not covered
...
 4 |     Voucher(String),
   |     ------- not covered
```

Fix: add the missing arm. Do not add a catch-all `_` arm only to make the error go away. A `_` arm also accepts every variant that is added later, so the compiler can no longer remind you.

### When to use which

- Use an enum when the states exclude each other. A struct with a `kind` field and several `Option` fields can hold combinations that mean nothing, such as "cash, with card digits".
- Use a tuple-like variant for one obvious payload, and a struct-like variant when there are two or more fields or the meaning is not obvious.
- Do not store or send the bytes of an enum. The default layout is not specified and may change between compiler versions. One layout fact is documented: for a short list of types such as `Box<T>` and `&T`, `Option` of that type is the same size as the type itself, because `None` is stored as the null pointer. This is called the *null-pointer optimization*.

### Read more

- [Enumerations](https://doc.rust-lang.org/reference/items/enumerations.html) in the Rust Reference
- [`std::mem::discriminant`](https://doc.rust-lang.org/std/mem/fn.discriminant.html)
- [Representation of `Option`](https://doc.rust-lang.org/std/option/index.html#representation)
- [Type layout](https://doc.rust-lang.org/reference/type-layout.html) in the Rust Reference

## Lesson 02 · Recursive data with enums

**Assumed:** Lesson 01, and `Box<T>` as a pointer that owns one heap value (first used in Chapter 3).

The compiler must know the size of every type. An enum is as large as its largest variant. Now suppose a variant contains the enum itself:

```rust,compile_fail,E0072
enum Parcel {
    Item { grams: u32 },
    Wrapped(Parcel),
}
```

```text
error[E0072]: recursive type `Parcel` has infinite size
 --> src/lib.rs:1:1
  |
1 | enum Parcel {
  | ^^^^^^^^^^^
2 |     Item { grams: u32 },
3 |     Wrapped(Parcel),
  |             ------ recursive without indirection
  |
help: insert some indirection (e.g., a `Box`, `Rc`, or `&`) to break the cycle
  |
3 |     Wrapped(Box<Parcel>),
  |             ++++      +
```

A `Parcel` would have to be large enough to hold a `Parcel`, which would have to hold a `Parcel`, and so on. No finite size works. The fix is *indirection*: store a pointer to the inner value. A `Box<Parcel>` is one pointer wide, whatever a `Parcel` is. A `Vec<Parcel>` also has a fixed size (pointer, length, capacity), so it works too.

### Worked example

```rust
enum Parcel {
    Item { grams: u32 },
    Wrapped(Box<Parcel>), // exactly one parcel inside
    Bundle(Vec<Parcel>),  // any number of parcels inside
}

fn weight(parcel: &Parcel) -> u32 {
    match parcel {
        Parcel::Item { grams } => *grams,
        // 20 grams of paper around the inner parcel.
        Parcel::Wrapped(inner) => 20 + weight(inner),
        Parcel::Bundle(parts) => {
            let mut total = 0;
            for part in parts {
                total += weight(part);
            }
            total
        }
    }
}

fn main() {
    let gift = Parcel::Bundle(vec![
        Parcel::Item { grams: 250 },
        Parcel::Wrapped(Box::new(Parcel::Item { grams: 100 })),
    ]);
    assert_eq!(weight(&gift), 370);
}
```

```text
gift (on the stack)
  Bundle ---> vector buffer (heap): [ Item { 250 } | Wrapped --+ ]
                                                               |
                                                               v
                                                   box (heap): Item { 100 }
```

Three things to notice:

- **Ownership goes downward.** Each parent owns its children. When `gift` is dropped, the vector buffer and the box are freed too. Nothing else points at them. The fields of one value are dropped in the order in which they are declared. So for a variant such as `Wrapped(Label, Box<Parcel>)`, the label is dropped first, and then the box with everything inside it.
- **The function has the shape of the type.** There is one arm per variant and one recursive call per child. This is called *structural recursion*.
- **`inner` is a `&Box<Parcel>`,** but `weight` wants a `&Parcel`. The call compiles because a reference to a box converts to a reference to its content automatically.

The type is finite, but a value can be as deep as you like: the size of `Parcel` says nothing about how many parcels a value contains.

### Mistakes people make

**Leaving out the indirection.** This is error E0072, shown above.

**Forgetting `Box::new` when building a value.**

```rust,compile_fail,E0308
enum Parcel {
    Item { grams: u32 },
    Wrapped(Box<Parcel>),
}

fn main() {
    let gift = Parcel::Wrapped(Parcel::Item { grams: 100 });
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:7:32
  |
7 |     let gift = Parcel::Wrapped(Parcel::Item { grams: 100 });
  |                --------------- ^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `Box<Parcel>`, found `Parcel`
  |                |
  |                arguments to this enum variant are incorrect
...
help: store this in the heap by calling `Box::new`
  |
7 |     let gift = Parcel::Wrapped(Box::new(Parcel::Item { grams: 100 }));
  |                                +++++++++                           +
```

**Trusting the depth of the input.** A recursive function uses one stack frame per level. The automatic drop of a nested value is recursive as well. A value that is nested a million levels deep can overflow the stack, and the compiler cannot warn you. If the depth comes from outside your program, limit it, or use a loop with your own stack. Lesson 08 shows such a loop.

### When to use which

| Children | Use |
| --- | --- |
| exactly one | `Box<T>` |
| zero or one | `Option<Box<T>>` |
| any number | `Vec<T>` |
| shared between several parents | `Rc<T>` (Chapter 4) |

### Read more

- [`Box<T>`](https://doc.rust-lang.org/std/boxed/index.html)
- [Error code E0072](https://doc.rust-lang.org/error_codes/E0072.html)
- [Recursive types](https://doc.rust-lang.org/reference/types.html#recursive-types) in the Rust Reference

## Lesson 03 · Generic enums

**Assumed:** Lesson 01, and generic structs and functions (Chapter 9).

An enum can have type parameters, like a struct. You have used two generic enums since Chapter 7. The standard library declares them like this:

```rust
enum Option<T> {
    None,
    Some(T),
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

There is nothing special about them. Both also derive the comparison traits, so the rule about derived ordering from Lesson 01 applies to them when their payload types can be compared. Each choice of types gives a separate type: `Option<u8>` and `Option<String>` have nothing in common except their definition. Different variants may use different parameters, as `Ok(T)` and `Err(E)` do.

### Worked example

```rust
#[derive(Debug, PartialEq)]
enum Outcome<T, W> {
    Clean(T),
    Warned(T, W),
}

// Moves the value out. No bound on `T` or `W` is needed, because every type
// can be moved.
fn into_value<T, W>(outcome: Outcome<T, W>) -> T {
    match outcome {
        Outcome::Clean(value) => value,
        Outcome::Warned(value, _) => value,
    }
}

// Borrows the warning if there is one. The result points into `outcome`.
fn warning<T, W>(outcome: &Outcome<T, W>) -> Option<&W> {
    match outcome {
        Outcome::Clean(_) => None,
        Outcome::Warned(_, warning) => Some(warning),
    }
}

// Changes the warning type and keeps the value type.
fn count_warning<T>(outcome: Outcome<T, String>) -> Outcome<T, usize> {
    match outcome {
        Outcome::Clean(value) => Outcome::Clean(value),
        Outcome::Warned(value, text) => Outcome::Warned(value, text.len()),
    }
}

fn main() {
    let report: Outcome<Vec<u8>, String> =
        Outcome::Warned(vec![1, 2, 3], String::from("truncated"));
    assert_eq!(warning(&report), Some(&String::from("truncated")));
    let counted = count_warning(report);
    assert_eq!(counted, Outcome::Warned(vec![1, 2, 3], 9));
    assert_eq!(into_value(counted), vec![1, 2, 3]);
}
```

Look at the first arm of `count_warning`: `Outcome::Clean(value) => Outcome::Clean(value)`. It seems to do nothing, but the two sides have different types. The left side is a pattern for `Outcome<T, String>`. The right side builds an `Outcome<T, usize>`. The payload moves from one into the other, and no data is copied.

A `derive` on a generic enum adds a bound for each parameter. `Outcome<T, W>` above is `Debug` only when both `T` and `W` are `Debug`.

### Mistakes people make

**Giving the compiler too little to infer the type.**

```rust,compile_fail,E0282
fn main() {
    let nothing = None;
}
```

```text
error[E0282]: type annotations needed for `Option<_>`
 --> src/main.rs:2:9
  |
2 |     let nothing = None;
  |         ^^^^^^^   ---- type must be known at this point
  |
help: consider giving `nothing` an explicit type, where the type for type parameter `T` is specified
  |
2 |     let nothing: Option<T> = None;
  |                +++++++++++
```

`None` alone does not say what `T` is. Fix: write `let nothing: Option<u32> = None;` or `None::<u32>`. The same happens with `Outcome::Clean(5)`, which says nothing about `W`.

**Adding a bound that the body does not need.**

```rust,compile_fail,E0277
enum Outcome<T, W> {
    Clean(T),
    Warned(T, W),
}

fn into_value<T: Copy, W>(outcome: Outcome<T, W>) -> T {
    match outcome {
        Outcome::Clean(value) => value,
        Outcome::Warned(value, _) => value,
    }
}

fn main() {
    let report: Outcome<String, u8> = Outcome::Clean(String::from("ok"));
    let text = into_value(report);
}
```

```text
error[E0277]: the trait bound `String: Copy` is not satisfied
  --> src/main.rs:15:27
   |
15 |     let text = into_value(report);
   |                ---------- ^^^^^^ the trait `Copy` is not implemented for `String`
   |                |
   |                required by a bound introduced by this call
   |
note: required by a bound in `into_value`
  --> src/main.rs:6:18
   |
 6 | fn into_value<T: Copy, W>(outcome: Outcome<T, W>) -> T {
   |                  ^^^^ required by this bound in `into_value`
```

The body only moves `value`, so the bound has no purpose and only shuts out useful callers. Fix: remove it. Add a bound when the body calls something that needs it, and not earlier.

**Trying to move a payload out through a reference.** If `warning` returned `Option<W>`, it would have to move the warning out of a value that the caller still owns. Lesson 05 explains this error (E0507). Fix: return a reference, as `warning` does, or take the enum by value.

### When to use which

- `Option<T>`: a value may be absent, and that is normal.
- `Result<T, E>`: one side means failure. The name tells the reader which side is which, and `?` works with it.
- Your own enum: both sides are normal results, or there are more than two cases, or the names should say something about your problem.

### Read more

- [`Option`](https://doc.rust-lang.org/std/option/enum.Option.html) and [`Result`](https://doc.rust-lang.org/std/result/enum.Result.html)
- [Generic parameters](https://doc.rust-lang.org/reference/items/generics.html) in the Rust Reference
- [Error code E0282](https://doc.rust-lang.org/error_codes/E0282.html)

## Lesson 04 · Literal, tuple and struct patterns

**Assumed:** Lesson 01, and tuples and structs (Chapters 3 and 9).

A *pattern* is a small picture of a value. Matching a value against a pattern does two things at once: it **tests** whether the value has that shape, and it **binds** names to parts of the value.

| Pattern | Matches | Binds |
| --- | --- | --- |
| `0`, `'x'`, `"go"`, `true` | exactly that value | nothing |
| `count` (a plain name) | any value | the value, as `count` (but see below) |
| `_` | any value | nothing, and the value is not moved |
| `(a, _, c)` | a tuple, part by part | `a` and `c` |
| `Point { x, y: 0 }` | a struct whose `y` is 0 | `x` (short for `x: x`) |
| `Point { x: left, .. }` | any `Point` | field `x`, under the name `left` |

Arms are tried from top to bottom, and the first arm that matches wins. So specific arms must come before general ones.

### Worked example

```rust
struct Reading {
    sensor: u8,
    celsius: i32,
    ok: bool,
}

fn summary(reading: Reading) -> String {
    match reading {
        // Tests one field, binds another, ignores the rest.
        Reading { ok: false, sensor, .. } => format!("sensor {sensor} is faulty"),
        Reading { celsius: 0, .. } => String::from("freezing point"),
        // `sensor: id` binds the field `sensor` to the new name `id`.
        Reading { sensor: id, celsius, .. } => format!("sensor {id}: {celsius} C"),
    }
}

// A pattern in a parameter: each tuple is taken apart in the signature.
fn midpoint((x1, y1): (i32, i32), (x2, y2): (i32, i32)) -> (i32, i32) {
    ((x1 + x2) / 2, (y1 + y2) / 2)
}

fn main() {
    let faulty = Reading { sensor: 3, celsius: 0, ok: false };
    assert_eq!(summary(faulty), "sensor 3 is faulty");
    let cold = Reading { sensor: 4, celsius: 0, ok: true };
    assert_eq!(summary(cold), "freezing point");
    let warm = Reading { sensor: 5, celsius: 21, ok: true };
    assert_eq!(summary(warm), "sensor 5: 21 C");
    assert_eq!(midpoint((0, 0), (4, 10)), (2, 5));
}
```

### A plain name: binding or comparison?

A plain name in a pattern creates a **new binding**, unless that name already refers to a constant, a unit struct or an enum variant. In that case the pattern **compares**. The compiler decides this by looking the name up. It does not look at upper or lower case. Case is only a convention, and this rule is the reason why the convention matters: `UPPER_CASE` for constants, `CamelCase` for variants, `snake_case` for variables.

```rust
const LIMIT: u16 = 500;

fn check(code: u16) -> &'static str {
    match code {
        LIMIT => "at the limit", // LIMIT is a constant: this compares
        _ => "other",
    }
}

fn main() {
    assert_eq!(check(500), "at the limit");
    assert_eq!(check(404), "other");
}
```

### `_` and `_name` are different

`_` is not a name. It binds nothing, so the value stays where it was and is not moved. `_name` is an ordinary binding. Only the "unused variable" warning is switched off. The value moves into `_name` and is dropped when `_name` goes out of scope.

### Mistakes people make

**Using a variable as if it were a constant.** This compiles. The only signal is a warning.

```rust
fn check(code: u16) -> &'static str {
    let limit = 500;
    match code {
        limit => "at the limit", // a NEW `limit` that matches every code
        _ => "other",
    }
}

fn main() {
    assert_eq!(check(404), "at the limit"); // not what the author wanted
}
```

```text
warning: unreachable pattern
 --> src/main.rs:5:9
  |
4 |         limit => "at the limit", // a NEW `limit` that matches every code
  |         ----- matches any value
5 |         _ => "other",
  |         ^ no value can reach this
  |
note: there is a binding of the same name; if you meant to pattern match against the value of that binding, that is a feature of constants that is not available for `let` bindings
```

Fix: make it a `const`, or compare in a guard (`c if c == limit`, Lesson 07).

**Writing a variant without its enum.**

```rust,compile_fail,E0170
enum Light {
    Red,
    Green,
}

fn go(light: Light) -> bool {
    match light {
        Green => true,
        Red => false,
    }
}
```

```text
error[E0170]: pattern binding `Green` is named the same as one of the variants of the type `Light`
 --> src/lib.rs:8:9
  |
8 |         Green => true,
  |         ^^^^^ help: to match on the variant, qualify the path: `Light::Green`
```

`Green` alone is not in scope, so it would be a new binding that matches everything. The compiler refuses this particular case. Fix: write `Light::Green`.

**Binding with `_name` and then using the original.**

```rust,compile_fail,E0382
fn main() {
    let job = (7, String::from("resize"));
    let (id, _name) = job;
    println!("{id} {}", job.1);
}
```

```text
error[E0382]: borrow of moved value: `job.1`
 --> src/main.rs:4:25
  |
3 |     let (id, _name) = job;
  |              ----- value moved here
4 |     println!("{id} {}", job.1);
  |                         ^^^^^ value borrowed here after move
```

Fix: write `(id, _)`. With `_`, nothing moves, and `job.1` is still usable.

### When to use which

- One field: `point.x` is enough. Several fields, or a test on their values: use a pattern.
- Named fields survive a change in field order and say what each part means. Tuple patterns are fine for two or three parts with an obvious order.
- `..` ignores the remaining fields. If you list every field instead, the compiler tells you when a field is added. Choose on purpose.

### Read more

- [Patterns](https://doc.rust-lang.org/reference/patterns.html) in the Rust Reference, especially [identifier patterns](https://doc.rust-lang.org/reference/patterns.html#identifier-patterns), [wildcard patterns](https://doc.rust-lang.org/reference/patterns.html#wildcard-pattern) and [struct patterns](https://doc.rust-lang.org/reference/patterns.html#struct-patterns)
- [`match` expressions](https://doc.rust-lang.org/reference/expressions/match-expr.html)

## Lesson 05 · Matching references and mutating payloads

**Assumed:** Lesson 04, and `&T` and `&mut T` (Chapter 5).

Very often you do not own the enum. You have a `&Job` or a `&mut Job`. You cannot move a payload out through a reference, because the owner would be left with a hole. So Rust has this rule:

> When the matched value is a reference, and the pattern describes the shape of the value behind it (a variant, a struct, a tuple or a slice, written without `&`), the compiler looks through the reference, and every name inside that pattern binds as a reference of the same kind.

A plain name alone is not such a pattern. In `let r = &job;` nothing is looked through, and `r` is simply the reference.

| You match on | `Job::Queued(name)` binds `name` as |
| --- | --- |
| a `Job` (owned) | `String`, moved out |
| a `&Job` | `&String` |
| a `&mut Job` | `&mut String` |

The Rust Reference calls this the *default binding mode*. Many people say *match ergonomics*.

### Worked example

```rust
#[derive(Debug, PartialEq)]
enum Job {
    Queued(String),
    Running { name: String, retries: u8 },
    Done,
}

// `job` is a `&Job`, so `name` is a `&String` each time.
fn title(job: &Job) -> &str {
    match job {
        Job::Queued(name) => name,
        Job::Running { name, .. } => name,
        Job::Done => "(finished)",
    }
}

// `job` is a `&mut Job`, so `retries` is a `&mut u8` that points into it.
fn retry(job: &mut Job) {
    if let Job::Running { retries, .. } = job {
        *retries = retries.saturating_add(1);
    }
}

// A change of state that needs the old payload.
fn start(job: &mut Job) {
    // Take the old state out and leave a cheap, valid value behind.
    match std::mem::replace(job, Job::Done) {
        Job::Queued(name) => *job = Job::Running { name, retries: 0 },
        other => *job = other, // every other state goes back unchanged
    }
}

fn main() {
    let mut job = Job::Queued(String::from("resize"));
    assert_eq!(title(&job), "resize");
    retry(&mut job); // not running yet: nothing changes
    start(&mut job);
    retry(&mut job);
    assert_eq!(job, Job::Running { name: String::from("resize"), retries: 1 });
    start(&mut job); // already running: nothing changes
    assert_eq!(title(&job), "resize");
}
```

`start` shows a problem you will meet again and again. To build `Running { name, .. }` you need to **own** the name, but you only have a `&mut Job`. `std::mem::replace` (Chapter 4) solves it: it puts a valid value into the place and hands you the old value, owned. `Option::take` is the same idea for an `Option`.

```text
before          replace(job, Done) returns        after the Queued arm
*job: Queued ──► owned Queued("resize")           *job: Running { name: "resize", retries: 0 }
                 *job is Done for a moment              (the same String buffer, moved twice)
```

### The older, explicit spelling

Before this rule existed, people wrote the references by hand: `&Job::Queued(ref name)` or `ref mut retries`. A `&` in a pattern removes one reference. `ref` makes a binding borrow, not move. You will see this in older code. With a `&` pattern and `Copy` fields you get copies: after `let &(a, b) = &(1u8, 2u8);` both `a` and `b` are plain `u8` values, while `let (a, b) = &(1u8, 2u8);` gives two `&u8`.

### Mistakes people make

**Moving out through a reference.**

```rust,compile_fail,E0507
enum Job {
    Queued(String),
    Running { name: String, retries: u8 },
    Done,
}

fn start(job: &mut Job) {
    match *job {
        Job::Queued(name) => *job = Job::Running { name, retries: 0 },
        _ => {}
    }
}
```

```text
error[E0507]: cannot move out of `job` as enum variant `Queued` which is behind a mutable reference
 --> src/lib.rs:8:11
  |
8 |     match *job {
  |           ^^^^
9 |         Job::Queued(name) => *job = Job::Running { name, retries: 0 },
  |                     ---- data moved here because `name` has type `String`, which does not implement the `Copy` trait
```

The compiler suggests matching `job` without the `*`. That makes `name` a `&mut String`, which is still not an owned `String`. Fix: borrow if a reference is enough, clone if you need a second copy, or use `mem::replace` as in `start` above.

**Writing through a shared reference.**

```rust,compile_fail,E0594
enum Job {
    Running { name: String, retries: u8 },
    Done,
}

fn retry(job: &Job) {
    if let Job::Running { retries, .. } = job {
        *retries += 1;
    }
}
```

```text
error[E0594]: cannot assign to `*retries`, which is behind a `&` reference
 --> src/lib.rs:8:9
  |
8 |         *retries += 1;
  |         ^^^^^^^^^^^^^ `retries` is a `&` reference, so it cannot be written to
```

Fix: take `&mut Job`. The kind of the outer reference decides the kind of every binding.

**Mixing the two spellings.** In edition 2024, `ref` inside a pattern that already borrows is an error:

```rust,compile_fail
fn main() {
    let slot = &Some(String::from("x"));
    if let Some(ref text) = slot {
        println!("{text}");
    }
}
```

```text
error: cannot explicitly borrow within an implicitly-borrowing pattern
 --> src/main.rs:3:17
  |
3 |     if let Some(ref text) = slot {
  |                 ^^^ explicit `ref` binding modifier not allowed when implicitly borrowing
```

Fix: remove `ref`. The binding is already a reference.

### When to use which

- To look: match on `&value`.
- To change a payload in place: match on `&mut value` and assign through the binding (`*n = ...`).
- To take the value apart for good: match on the value itself.
- To own a payload while you only hold `&mut`: `std::mem::replace`, `std::mem::take` or `Option::take`.

### Read more

- [Binding modes](https://doc.rust-lang.org/reference/patterns.html#binding-modes) in the Rust Reference
- [`std::mem::replace`](https://doc.rust-lang.org/std/mem/fn.replace.html) and [`Option::take`](https://doc.rust-lang.org/std/option/enum.Option.html#method.take)
- [Match ergonomics in edition 2024](https://doc.rust-lang.org/edition-guide/rust-2024/match-ergonomics.html) in the Edition Guide
- [Error code E0507](https://doc.rust-lang.org/error_codes/E0507.html)

## Lesson 06 · Array and slice patterns

**Assumed:** Lessons 04 and 05, and arrays `[T; N]` and slices `[T]` (Chapter 3).

An array has its length in its type. A pattern for `[u8; 4]` must describe exactly four elements (or use `..`), so the length can never be the reason why it fails. If its parts are only names and `_`, it always matches. A slice has a length that is known only when the program runs. So each slice pattern covers some lengths, and a `match` must cover all of them.

| Pattern | Lengths it matches |
| --- | --- |
| `[]` | 0 |
| `[x]` | 1 |
| `[a, b]` | 2 |
| `[first, ..]` | 1 or more |
| `[first, .., last]` | 2 or more |
| `[first, rest @ ..]` | 1 or more, and `rest` is everything after `first` |

`..` stands for "any number of elements, also none". It may appear once in a slice pattern. `name @ ..` gives those elements a name.

### Worked example

```rust
fn describe(samples: &[i32]) -> String {
    match samples {
        [] => String::from("no samples"),
        [only] => format!("one sample: {only}"),
        [first, .., last] => format!("from {first} to {last}"),
    }
}

// `tail` is a view of the remaining elements. Nothing is copied.
fn sum(values: &[i32]) -> i32 {
    match values {
        [] => 0,
        [head, tail @ ..] => head + sum(tail),
    }
}

// An array pattern always matches, so a plain `let` is enough.
fn length_field(header: [u8; 4]) -> u16 {
    let [_, _, high, low] = header;
    u16::from_be_bytes([high, low])
}

fn main() {
    assert_eq!(describe(&[]), "no samples");
    assert_eq!(describe(&[7]), "one sample: 7");
    assert_eq!(describe(&[3, 9, 4]), "from 3 to 4");
    assert_eq!(sum(&[1, 2, 3, 4]), 10);
    assert_eq!(length_field([1, 0, 0x01, 0x2c]), 300);

    let samples = vec![3, 9, 4];
    // A Vec is not a slice. Match on a slice of it.
    assert_eq!(describe(samples.as_slice()), "from 3 to 4");
}
```

```text
values:  [ 1 | 2 | 3 | 4 ]
           ^   ^-----------^
         head       tail      tail is a &[i32] that points into the same memory
```

In `sum`, `values` is a `&[i32]`, so the rule from Lesson 05 applies: `head` is a `&i32` and `tail` is a `&[i32]`. On an **array** that is matched by value, `rest @ ..` binds an array of the remaining elements, not a slice.

### Mistakes people make

**Using the wrong number of elements for an array.**

```rust,compile_fail,E0527
fn main() {
    let header = [1u8, 0, 2, 44];
    let [version, flags, length] = header;
}
```

```text
error[E0527]: pattern requires 3 elements but array has 4
 --> src/main.rs:3:9
  |
3 |     let [version, flags, length] = header;
  |         ^^^^^^^^^^^^^^^^^^^^^^^^ expected 4 elements
```

Fix: name every element, or end the pattern with `..`.

**Not covering every length of a slice.**

```rust,compile_fail,E0004
fn describe(samples: &[i32]) -> &'static str {
    match samples {
        [] => "none",
        [_] => "one",
        [_, _] => "two",
    }
}
```

```text
error[E0004]: non-exhaustive patterns: `&[_, _, _, ..]` not covered
 --> src/lib.rs:2:11
  |
2 |     match samples {
  |           ^^^^^^^ pattern `&[_, _, _, ..]` not covered
  |
  = note: the matched value is of type `&[i32]`
```

The compiler names the missing shape: three or more elements. Fix: add an arm such as `[_, _, ..] => "many"`.

**Using a slice pattern in a plain `let`.**

```rust,compile_fail,E0005
fn main() {
    let samples = vec![3, 1, 4];
    let [first, ..] = samples.as_slice();
    println!("{first}");
}
```

```text
error[E0005]: refutable pattern in local binding
 --> src/main.rs:3:9
  |
3 |     let [first, ..] = samples.as_slice();
  |         ^^^^^^^^^^^ pattern `&[]` not covered
...
help: you might want to use `let...else` to handle the variant that isn't matched
  |
3 |     let [first, ..] = samples.as_slice() else { todo!() };
  |                                          ++++++++++++++++
```

The slice may be empty, and `let` has no way to handle that. Fix: `let ... else`, a `match`, or `samples.first()`.

### When to use which

- Slice patterns are best for a few fixed shapes at the start or the end: "empty", "exactly two", "first and the rest".
- For one element, `first()`, `last()` and `get(i)` return an `Option` and say the same thing more directly.
- For "every element", use a loop.
- A `Vec<T>` cannot be matched with a slice pattern directly. Match `v.as_slice()` or `&v[..]`.

### Read more

- [Slice patterns](https://doc.rust-lang.org/reference/patterns.html#slice-patterns) and [rest patterns](https://doc.rust-lang.org/reference/patterns.html#rest-pattern) in the Rust Reference
- [`slice::split_first`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_first)
- [Error code E0527](https://doc.rust-lang.org/error_codes/E0527.html)

## Lesson 07 · Guards, alternatives and @ bindings

**Assumed:** Lessons 04 to 06.

Four more tools make patterns say exactly what you mean.

- **Alternatives:** `A | B` matches when `A` or `B` matches. It is also called an *or-pattern*. Every alternative must bind the same names with the same types, because the code after `=>` is the same for all of them.
- **Ranges:** `10..=20` matches every value from 10 to 20, including both ends. `65..` matches 65 and everything above. Ranges are mostly used with integers and `char`.
- **Guards:** `pattern if condition`. First the pattern is tested, then the condition. If the condition is false, matching continues with the next arm.
- **`@` bindings:** `name @ subpattern` tests the subpattern and binds the whole matched value to `name`.

### Worked example

```rust
#[derive(Debug, PartialEq)]
enum Fare {
    Free,
    Reduced(u8),
    Full,
}

fn fare(age: u8, has_pass: bool) -> Fare {
    match (age, has_pass) {
        (0..=5, _) | (_, true) => Fare::Free,
        // The range test and the value in one step.
        (age @ (6..=17 | 65..), false) => Fare::Reduced(age),
        (_, false) => Fare::Full,
    }
}

fn position(point: (i32, i32)) -> &'static str {
    match point {
        (0, 0) => "origin",
        // A guard can compare two bindings. A pattern alone cannot.
        (x, y) if x == y => "diagonal",
        (x, _) if x > 0 => "right half",
        _ => "elsewhere",
    }
}

fn main() {
    assert_eq!(fare(3, false), Fare::Free);
    assert_eq!(fare(40, true), Fare::Free);
    assert_eq!(fare(12, false), Fare::Reduced(12));
    assert_eq!(fare(70, false), Fare::Reduced(70));
    assert_eq!(fare(40, false), Fare::Full);

    assert_eq!(position((2, 2)), "diagonal");
    assert_eq!(position((5, -1)), "right half");
    assert_eq!(position((-5, 1)), "elsewhere");

    // `matches!` is a `match` that answers true or false.
    assert!(matches!('q', 'a'..='z' | 'A'..='Z'));
    assert!(matches!(fare(12, false), Fare::Reduced(age) if age < 18));
    assert!(!matches!(fare(70, false), Fare::Reduced(age) if age < 18));
}
```

`fare` has no guard, so the compiler can check that the three arms cover every pair. It really does the arithmetic on the ranges.

Two facts about guards surprise people:

- A guard belongs to the **whole** arm. `A | B if condition` means "(`A` or `B`), and then the condition". It does not mean "`A`, or (`B` with the condition)".
- With an or-pattern, the guard may run more than once: once for each alternative that matches, until it is true. Keep guards free of side effects.

`matches!(value, pattern)` and `matches!(value, pattern if condition)` expand to a `match` with one arm that gives `true` and a `_` arm that gives `false`. Clippy suggests it when you write that `match` by hand.

### Mistakes people make

**Binding a name in only one alternative.**

```rust,compile_fail,E0408
enum Input {
    Key(char),
    Click { x: u16, y: u16 },
    Tick,
}

fn describe(input: &Input) -> String {
    match input {
        Input::Key(c) | Input::Click { .. } => format!("user input {c}"),
        Input::Tick => String::from("timer"),
    }
}
```

```text
error[E0408]: variable `c` is not bound in all patterns
 --> src/lib.rs:9:25
  |
9 |         Input::Key(c) | Input::Click { .. } => format!("user input {c}"),
  |                    -    ^^^^^^^^^^^^^^^^^^^ pattern doesn't bind `c`
  |                    |
  |                    variable not in all patterns
```

For a click there is no `c`. (A second error, E0381, follows from the same cause.) Fix: split the arm in two, or ignore the payload in both alternatives.

**Expecting guards to make a match complete.**

```rust,compile_fail,E0004
fn sign(n: i32) -> &'static str {
    match n {
        n if n < 0 => "negative",
        n if n == 0 => "zero",
        n if n > 0 => "positive",
    }
}
```

```text
error[E0004]: non-exhaustive patterns: `i32::MIN..=i32::MAX` not covered
 --> src/lib.rs:2:11
  |
2 |     match n {
  |           ^ pattern `i32::MIN..=i32::MAX` not covered
  |
  = note: the matched value is of type `i32`
  = note: match arms with guards don't count towards exhaustivity
```

A guard is ordinary code, and the compiler does not try to prove things about it. Fix: make the last arm unguarded, or use patterns that the compiler can check: `i32::MIN..=-1`, `0` and `1..`.

**Leaving a gap between ranges.** Here the compiler helps, because ranges are patterns:

```rust,compile_fail,E0004
fn band(age: u8) -> &'static str {
    match age {
        0..=17 => "minor",
        19..=64 => "adult",
        65..=255 => "senior",
    }
}
```

```text
error[E0004]: non-exhaustive patterns: `18_u8` not covered
 --> src/lib.rs:2:11
  |
2 |     match age {
  |           ^^^ pattern `18_u8` not covered
```

### When to use which

- Prefer a pattern when the test is about shape or about constants. The compiler then checks that nothing is missing.
- Use a guard to compare with a run-time value or to compare two bindings with each other.
- Use `@` when you need both the test and the tested value.
- Use `matches!` when the only result you need is a `bool`.

### Read more

- [Match guards](https://doc.rust-lang.org/reference/expressions/match-expr.html#match-guards) in the Rust Reference
- [Or-patterns](https://doc.rust-lang.org/reference/patterns.html#or-patterns), [range patterns](https://doc.rust-lang.org/reference/patterns.html#range-patterns) and [identifier patterns with `@`](https://doc.rust-lang.org/reference/patterns.html#identifier-patterns)
- [`matches!`](https://doc.rust-lang.org/std/macro.matches.html)

## Lesson 08 · Pattern contexts and public compatibility

**Assumed:** Lessons 04 to 07, and `if let` and `let ... else` from Chapter 6.

Patterns are not only for `match`. Every `let`, every parameter and every `for` loop contains one. What a place accepts depends on one question: can the pattern fail?

- An *irrefutable* pattern matches every value of its type: `x`, `(a, b)`, `Point { x, y }`, `[a, b, c]` for an array of three.
- A *refutable* pattern can fail: `Some(x)`, `0`, `[first, ..]` for a slice.

| Place | Refutable pattern allowed? | When it does not match |
| --- | --- | --- |
| `let pattern = value;` | no | |
| function and closure parameters | no | |
| `for pattern in iterator` | no | |
| `match` arm | yes | the next arm is tried |
| `if let pattern = value` | yes | the `else` branch runs, if there is one |
| `while let pattern = value` | yes | the loop ends |
| `let pattern = value else { ... };` | yes | the `else` block runs, and it must leave: `return`, `break`, `continue` or a panic |

### Worked example

```rust
/// Counts leading "../" steps and returns the rest of the path.
fn climb(mut path: &str) -> (usize, &str) {
    let mut levels = 0;
    // The loop runs for as long as the pattern matches.
    while let Some(rest) = path.strip_prefix("../") {
        levels += 1;
        path = rest;
    }
    (levels, path)
}

/// The first letter in upper case, or '?' for an empty word.
fn initial(word: &str) -> char {
    // If the pattern fails, the else block must leave the function.
    let Some(first) = word.chars().next() else {
        return '?';
    };
    // `first` is in scope here, with no extra nesting.
    first.to_ascii_uppercase()
}

/// A let chain (edition 2024): all the steps must succeed, from left to right.
fn port(address: &str) -> Option<u16> {
    if let Some((_, tail)) = address.rsplit_once(':')
        && let Ok(port) = tail.parse::<u16>()
        && port != 0
    {
        Some(port)
    } else {
        None
    }
}

fn main() {
    assert_eq!(climb("../../src/lib.rs"), (2, "src/lib.rs"));
    assert_eq!(initial("rust"), 'R');
    assert_eq!(initial(""), '?');
    assert_eq!(port("localhost:8080"), Some(8080));
    assert_eq!(port("localhost:http"), None);
    assert_eq!(port("localhost"), None);

    // `for` takes an irrefutable pattern: each item is a pair.
    let mut total = 0;
    for (name, score) in [("ana", 3), ("bo", 5)] {
        total += score + name.len();
    }
    assert_eq!(total, 13);
}
```

A `while let` loop is also the usual way to walk a nested value without recursion: push the root on a `Vec`, and loop for as long as `pop` returns `Some`. Each step may push more work. That is the "loop with your own stack" from Lesson 02.

### Mistakes people make

**A refutable pattern in a plain `let`.**

```rust,compile_fail,E0005
fn main() {
    let text = "width=80";
    let Some((key, value)) = text.split_once('=');
    println!("{key} {value}");
}
```

```text
error[E0005]: refutable pattern in local binding
 --> src/main.rs:3:9
  |
3 |     let Some((key, value)) = text.split_once('=');
  |         ^^^^^^^^^^^^^^^^^^ pattern `None` not covered
  |
  = note: `let` bindings require an "irrefutable pattern", like a `struct` or an `enum` with only one variant
```

Fix: decide what happens for `None`. Use `let ... else`, `if let` or `match`.

**An `else` block that does not leave.**

```rust,compile_fail,E0308
fn initial(word: &str) -> char {
    let Some(first) = word.chars().next() else {
        println!("empty word");
    };
    first
}
```

```text
error[E0308]: `else` clause of `let...else` does not diverge
 --> src/lib.rs:2:48
  |
2 |       let Some(first) = word.chars().next() else {
  |  ________________________________________________^
3 | |         println!("empty word");
4 | |     };
  | |_____^ expected `!`, found `()`
  |
  = help: try adding a diverging expression, such as `return` or `panic!(..)`
```

After the `else` block there would be no `first`, so the code below could not run. Fix: end the block with `return`, `break`, `continue` or a panic.

### Public enums and `#[non_exhaustive]`

Exhaustive matching has a cost for library authors. If you publish an enum and later add a variant, every `match` in other people's code that lists all variants stops compiling. The attribute `#[non_exhaustive]` tells the users in advance that the list may grow:

- On an **enum**: code in other crates must add a wildcard arm (`_ =>`) when it matches the enum. Inside the crate that defines it, nothing changes, and matches stay exhaustive.
- On a **struct** or a struct-like variant: other crates cannot build it with a struct expression, and their patterns must end with `..`. So you can add fields later.

The price is that your users lose the compiler's reminder for that enum. Use the attribute for lists that are likely to grow, such as kinds of errors. Do not use it for a list that is complete by nature, such as `Less`, `Equal`, `Greater`. And do not add a `_` arm to a match on your **own** enum "to be safe": it switches off the check that this chapter is about.

### Optional lab: see it across two crates

The attribute has no effect inside one crate, so this lab needs two packages. Make them in an empty directory outside the course repository.

```sh
cargo new --lib events
cargo new events_app
```

Add the dependency under `[dependencies]` in `events_app/Cargo.toml`:

```toml
events = { path = "../events" }
```

Put this in `events/src/lib.rs`:

```rust,ignore
#[non_exhaustive]
#[derive(Debug)]
pub enum Event {
    Open,
    Closed,
}

#[non_exhaustive]
#[derive(Debug)]
pub struct Settings {
    pub retries: u8,
}

impl Settings {
    pub fn new(retries: u8) -> Settings {
        Settings { retries }
    }
}

// Inside the defining crate this match is complete without `_`.
pub fn label(event: &Event) -> &'static str {
    match event {
        Event::Open => "open",
        Event::Closed => "closed",
    }
}
```

And this in `events_app/src/main.rs`:

```rust,ignore
use events::{Event, Settings};

fn label(event: &Event) -> &'static str {
    match event {
        Event::Open => "open",
        Event::Closed => "closed",
    }
}

fn main() {
    println!("{}", label(&Event::Open));
    let settings = Settings { retries: 3 };
    let Settings { retries } = settings;
    println!("{retries}");
}
```

Run `cargo build` in `events_app`. You get three errors (trimmed here):

```text
error[E0639]: cannot create non-exhaustive struct using struct expression
  --> src/main.rs:12:20
   |
12 |     let settings = Settings { retries: 3 };
   |                    ^^^^^^^^^^^^^^^^^^^^^^^

error[E0638]: `..` required with struct marked as non-exhaustive
  --> src/main.rs:13:9
   |
13 |     let Settings { retries } = settings;
   |         ^^^^^^^^^^^^^^^^^^^^

error[E0004]: non-exhaustive patterns: `&_` not covered
 --> src/main.rs:4:11
  |
4 |     match event {
  |           ^^^^^ pattern `&_` not covered
  |
  = note: the matched value is of type `&Event`
  = note: `Event` is marked as non-exhaustive, so a wildcard `_` is necessary to match exhaustively
```

Repair `events_app`: add a `_ => "something newer"` arm, build the settings with `Settings::new(3)`, and write the pattern as `Settings { retries, .. }`. Then add a variant `Paused` to `Event` in the library, with its arm in the library's own `label`. Build the application again. It still compiles, and the new variant goes to the wildcard arm. Last, remove `#[non_exhaustive]` from `Event` and remove the wildcard arm in the application: now the application breaks when the library adds a variant. That break is exactly what the attribute prevents.

### One more irrefutable case

A pattern is also irrefutable when the other variants cannot exist. `std::convert::Infallible` is an enum with no variants, so a `Result<T, Infallible>` can only be `Ok`:

```rust
use std::convert::Infallible;

fn main() {
    let parsed: Result<u8, Infallible> = Ok(7);
    let Ok(n) = parsed; // no `else` needed: `Err` cannot be built
    assert_eq!(n, 7);
}
```

### Read more

- [Refutability](https://doc.rust-lang.org/reference/patterns.html#refutability) in the Rust Reference
- [`let` statements and `let ... else`](https://doc.rust-lang.org/reference/statements.html#let-statements)
- [`if let` and let chains](https://doc.rust-lang.org/reference/expressions/if-expr.html)
- [Let chains in edition 2024](https://doc.rust-lang.org/edition-guide/rust-2024/let-chains.html) in the Edition Guide
- [The `non_exhaustive` attribute](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute)

## Lesson 09 · Capstone: generic ordered tree

**Assumed:** the whole chapter, and lifetimes in signatures (Chapter 5).

The capstone is a *binary search tree*: a set of values in which every node keeps smaller values in its left subtree and larger values in its right subtree. This section explains the ideas you need with a different structure, a linked list. The tree is your work.

### Where the Box goes

```rust
enum List {
    Nil,
    Cons(Box<Cell>),
}

struct Cell {
    value: u32,
    next: List,
}

impl List {
    /// Appends at the end and returns the new length.
    fn push_back(&mut self, value: u32) -> usize {
        match self {
            // The empty place is where the value belongs. Overwrite it.
            List::Nil => {
                *self = List::Cons(Box::new(Cell { value, next: List::Nil }));
                1
            }
            // `cell` is a `&mut Box<Cell>`. Continue in the rest of the list.
            List::Cons(cell) => 1 + cell.next.push_back(value),
        }
    }

    /// Appends a reference to every value to `out`, in order.
    fn collect<'a>(&'a self, out: &mut Vec<&'a u32>) {
        if let List::Cons(cell) = self {
            out.push(&cell.value);
            cell.next.collect(out);
        }
    }
}

fn main() {
    let mut list = List::Nil;
    assert_eq!(list.push_back(10), 1);
    assert_eq!(list.push_back(20), 2);
    assert_eq!(list.push_back(30), 3);
    let mut seen = Vec::new();
    list.collect(&mut seen);
    assert_eq!(seen, [&10, &20, &30]);
}
```

```text
list: Cons ──► Cell { 10, next: Cons } ──► Cell { 20, next: Cons } ──► Cell { 30, next: Nil }
      (one heap block per value; the final Nil is stored inside the last cell)
```

`List` and `Cell` refer to each other, and the `Box` breaks the cycle (Lesson 02). The `Box` wraps the **whole cell**, not the `next` field. That has two effects. `Nil` needs no heap memory, so each value costs exactly one allocation. And on the pinned toolchain a `List` is one pointer wide, because `Nil` can be stored as the null pointer, like `None` in `Option<Box<T>>` (Lesson 01). For your own enum this size is an observation, not a promise of the language.

The other layout, `Cons { value: u32, next: Box<List> }`, also compiles. But then even the final `Nil` lives in its own heap block: one extra allocation that holds no data. In a tree with two children per node, that wastes one allocation for every empty child. The capstone's `Tree` puts the `Box` around the node for this reason.

### Recursive mutable matching

`push_back` matches on `self`, which is a `&mut List`. By the rule from Lesson 05, `cell` is a mutable reference into the list, so the recursive call changes the list in place. In the `Nil` arm there is no payload and nothing is borrowed, so `*self = ...` may overwrite the whole value.

### Why `collect` needs `'a`

The references that are pushed into `out` point into the list. The signature must say so: `&'a self` and `Vec<&'a u32>` use the same lifetime, which means "these references live no longer than the borrow of the list". Without the name, the compiler gives each reference its own lifetime and rejects the `push`:

```rust,compile_fail
enum List {
    Nil,
    Cons(Box<Cell>),
}

struct Cell {
    value: u32,
    next: List,
}

impl List {
    fn collect(&self, out: &mut Vec<&u32>) {
        if let List::Cons(cell) = self {
            out.push(&cell.value);
            cell.next.collect(out);
        }
    }
}
```

```text
error: lifetime may not live long enough
  --> src/lib.rs:14:13
   |
12 |     fn collect(&self, out: &mut Vec<&u32>) {
   |                -                    - let's call the lifetime of this reference `'2`
   |                |
   |                let's call the lifetime of this reference `'1`
13 |         if let List::Cons(cell) = self {
14 |             out.push(&cell.value);
   |             ^^^^^^^^^^^^^^^^^^^^^ argument requires that `'1` must outlive `'2`
...
help: consider introducing a named lifetime parameter and update trait if needed
   |
12 |     fn collect<'a>(&'a self, out: &mut Vec<&'a u32>) {
   |               ++++  ++                      ++
```

### One more mistake: assigning to the reference, not through it

```rust,compile_fail,E0384
enum List {
    Nil,
    Cons(Box<Cell>),
}

struct Cell {
    value: u32,
    next: List,
}

impl List {
    fn push_back(&mut self, value: u32) {
        match self {
            List::Nil => {
                self = &mut List::Cons(Box::new(Cell { value, next: List::Nil }));
            }
            List::Cons(cell) => cell.next.push_back(value),
        }
    }
}
```

```text
error[E0384]: cannot assign to immutable argument `self`
  --> src/lib.rs:15:17
   |
15 |                 self = &mut List::Cons(Box::new(Cell { value, next: List::Nil }));
   |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ cannot assign to immutable argument
```

`self = ...` would only point the local reference at something else. The list itself would not change. (The compiler prints a second error for this code, E0716, because the new list is a temporary value that is freed at the end of the statement.) Fix: write `*self = ...` to store the new value **in the place** that `self` points to.

### Three-way comparison

`a.cmp(&b)` compares two values once and returns a `std::cmp::Ordering`, an enum with the variants `Less`, `Equal` and `Greater`. A `match` on it must handle all three, so no case can be forgotten.

```rust
use std::cmp::Ordering;

fn advice(guess: u32, secret: u32) -> &'static str {
    match guess.cmp(&secret) {
        Ordering::Less => "higher",
        Ordering::Greater => "lower",
        Ordering::Equal => "correct",
    }
}

fn main() {
    assert_eq!(advice(3, 7), "higher");
    assert_eq!(advice(9, 7), "lower");
    assert_eq!(advice(7, 7), "correct");
}
```

A chain of `<` and `==` also works, but it compares twice and nothing checks that you covered every case.

### Plan your tree

Answer these questions before you write code.

1. **Insert.** Which arm is "the empty place", and what do you store there? In the other arm, what does each `Ordering` mean: which child do you continue in, and when do you stop without storing anything? What does the function return in each case?
2. **Contains.** It is the same walk with shared references. At each node, at most one child can hold the value. The tests count this.
3. **Inorder.** Everything on the left is smaller than the node, and everything on the right is larger. In which order must you visit the three parts to get sorted output? Why does the signature need `'a`?

The **shape** of the tree depends on the order of insertion. Each new value goes down from the root until it finds an empty place. If the values arrive in a mixed order, the tree stays bushy, and a search visits about log2(n) nodes. If they arrive already sorted, every value goes to the same side, and the tree becomes a long chain: a search then visits up to n nodes, and the recursion is n levels deep. Trees that rebalance themselves avoid this. In production code, use the standard `BTreeSet` or `BTreeMap`, which also keep many values in one node and so use the processor cache better.

The mastery track ([`mastery/m2_unsafe`](../../mastery/m2_unsafe/README.md)) returns to linked structures and builds `Box` and `Vec` themselves from raw memory.

### Read more

- [`std::cmp::Ordering`](https://doc.rust-lang.org/std/cmp/enum.Ordering.html) and [`Ord::cmp`](https://doc.rust-lang.org/std/cmp/trait.Ord.html#tymethod.cmp)
- [`BTreeSet`](https://doc.rust-lang.org/std/collections/struct.BTreeSet.html)
- [Lifetime elision](https://doc.rust-lang.org/reference/lifetime-elision.html) in the Rust Reference
- [Error code E0384](https://doc.rust-lang.org/error_codes/E0384.html)
