# Chapter 11 concepts — Generic code with traits

[Chapter guide](README.md) · [Hints](HINTS.md)

Read one section, then do its lesson. Each section explains the idea, shows one worked example that is *not* the exercise, and lists the mistakes people really make with the compiler's own words.

## What you will be able to do

**After this chapter you can** define a trait and implement it for your own and for other people's types, write functions that work for every type with a given ability, choose between a generic parameter and a trait object and say what each costs, read a "trait bound is not satisfied" error and repair it, and return iterators and closures without naming their types.

Some advanced trait topics are not in this chapter: variance, higher-ranked bounds (`for<'a>`), generic associated types, trait-object upcasting, and sealed traits. They are taught in the mastery track, in [`mastery/m1_types`](../../mastery/m1_types/README.md).

## Before you start

You should already know ownership and moves (chapter 4), references and lifetimes such as `&'a str` (chapter 5), `Result` and `?` (chapter 7), structs with methods (chapter 9), and enums with `match` (chapter 10). Closures and iterators appear here only in simple forms; chapters 14 and 15 treat them fully.

## Time

These are estimates.

| Part | Reading | Exercise |
| --- | --- | --- |
| 01 · Traits, implementations, and defaults | 15 min | 30 min |
| 02 · Static and dynamic dispatch | 20 min | 45 min |
| 03 · Self, supertraits, and associated items | 15 min | 35 min |
| 04 · Extension traits and qualified calls | 20 min | 35 min |
| 05 · Associated types and generic traits | 15 min | 35 min |
| 06 · Opaque results, lifetimes, and precise capture | 20 min | 50 min |
| 07 · Capstone | 5 min | 60 min |
| Review, code review, and drills | | 2 hours |

## Lesson 01 · Traits, implementations, and defaults

**Assumed:** structs, methods with `&self`, and `format!`.

A **trait** is a named list of method signatures. It says what a type can do. It does not say what the type contains. An **implementation** (`impl Trait for Type`) supplies the bodies for one type. Any number of unrelated types can implement the same trait, and one type can implement many traits.

A trait method is one of two kinds:

- A **required** method has no body in the trait. Every implementation must write it.
- A **provided** method (also called a default method) has a body in the trait. An implementation gets it for free. It may also replace it; this is called an **override**.

A provided method can call required methods through `self`. So the trait author writes the shared logic once, and each type fills in only the part that differs.

To write a function for *every* type that has an ability, use a **generic parameter** with a **bound**: `fn f<T: Trait>(x: &T)`. The bound `T: Trait` is a promise in both directions. The caller must pass a type that implements the trait. The function body may use only what the bound promises.

Here is the worked example. `cents` is required. `label` is provided and calls `cents`.

```rust
trait Priced {
    fn cents(&self) -> u64;

    fn label(&self) -> String {
        format!("{}.{:02}", self.cents() / 100, self.cents() % 100)
    }
}

struct Coffee;
struct Book {
    pages: u64,
}

impl Priced for Coffee {
    fn cents(&self) -> u64 {
        350
    }
}

impl Priced for Book {
    fn cents(&self) -> u64 {
        self.pages * 4
    }
    // An override: books show a different label.
    fn label(&self) -> String {
        format!("book, {} cents", self.cents())
    }
}

fn total<T: Priced>(items: &[T]) -> u64 {
    items.iter().map(|item| item.cents()).sum()
}

fn main() {
    assert_eq!(Coffee.label(), "3.50"); // provided body
    assert_eq!(Book { pages: 200 }.label(), "book, 800 cents"); // override
    assert_eq!(total(&[Coffee, Coffee]), 700);
}
```

Three facts to keep:

1. Implementing a trait adds methods, not data. `Coffee` has no fields, and it still has none after `impl Priced for Coffee`.
2. When generic code calls `item.label()`, it runs the override if the type has one, and the provided body if not. The function does not need to know which.
3. A trait can be implemented for many types at once with a **blanket implementation**: `impl<T: Priced> Trait2 for T { ... }` gives `Trait2` to every type that is `Priced`. The standard library does this to give `to_string()` to every type that implements `Display`.

### Mistakes people really make

**Leaving out a required method.** The compiler names what is missing.

```rust,compile_fail,E0046
trait Priced {
    fn cents(&self) -> u64;
    fn label(&self) -> String {
        format!("{}.{:02}", self.cents() / 100, self.cents() % 100)
    }
}
struct Coffee;
impl Priced for Coffee {}
```

```text
error[E0046]: not all trait items implemented, missing: `cents`
  |
2 |     fn cents(&self) -> u64;
  |     ----------------------- `cents` from trait
...
8 | impl Priced for Coffee {}
  | ^^^^^^^^^^^^^^^^^^^^^^ missing `cents` in implementation
```

**Using a method the bound does not promise.** Inside a generic function, an unbounded `T` could be any type at all, so almost nothing is allowed.

```rust,compile_fail,E0599
trait Priced {
    fn cents(&self) -> u64;
}
fn total<T>(items: &[T]) -> u64 {
    items.iter().map(|item| item.cents()).sum()
}
```

```text
error[E0599]: no method named `cents` found for reference `&T` in the current scope
  |
5 |     items.iter().map(|item| item.cents()).sum()
  |                                  ^^^^^ method not found in `&T`
  |
  = help: items from traits can only be used if the type parameter is bounded by the trait
help: the following trait defines an item `cents`, perhaps you need to restrict type parameter `T` with it:
  |
4 | fn total<T: Priced>(items: &[T]) -> u64 {
  |           ++++++++
```

The fix is the bound the compiler suggests.

**Forgetting to import the trait.** A trait method can be called with dot syntax only when the trait is in scope. If the trait lives in another module, write a `use` for the trait, not only for the type. The message is `items from traits can only be used if the trait is in scope`, and the compiler prints the `use` line to add.

### Read more

- [Traits](https://doc.rust-lang.org/reference/items/traits.html) and [implementations](https://doc.rust-lang.org/reference/items/implementations.html) in the Rust Reference
- [The `trait` keyword](https://doc.rust-lang.org/std/keyword.trait.html) in the standard library documentation

## Lesson 02 · Static and dynamic dispatch

**Assumed:** lesson 01, `Box<T>` (chapter 4), and slices.

**Dispatch** means choosing which function body runs for a call such as `plan.cost(10)`. Rust offers two ways.

**Static dispatch** uses a generic parameter. For each concrete type that the program uses as `T`, the compiler makes a separate copy of the function. This is called **monomorphization**. In each copy the type is known, so the call is a direct call and can be inlined. The price is one copy of the machine code per type, and one rule: a `&[T]` or `Vec<T>` holds a single type.

**Dynamic dispatch** uses a **trait object**, written `dyn Trait`. The concrete type is forgotten at compile time and found again at run time. A trait object has no fixed size, so you always hold it behind a pointer: `&dyn Trait`, `&mut dyn Trait`, or `Box<dyn Trait>`. That pointer is a **fat pointer**: two machine words.

```text
 plan: &dyn Tariff
 +------------+------------+
 | data       | vtable     |
 +-----|------+-----|------+
       v            v
  PerMinute(4)    vtable of PerMinute as Tariff
                  +--------------------------+
                  | drop, size, alignment    |
                  | cost -> PerMinute::cost  |
                  +--------------------------+
```

The **vtable** is a table the compiler builds once per (type, trait) pair. A call through `dyn Tariff` loads the function address from the table and jumps to it. Provided methods have entries too, so an override is found correctly.

```rust
trait Tariff {
    fn cost(&self, minutes: u32) -> u32;
}

struct Flat(u32);
struct PerMinute(u32);

impl Tariff for Flat {
    fn cost(&self, _minutes: u32) -> u32 {
        self.0
    }
}
impl Tariff for PerMinute {
    fn cost(&self, minutes: u32) -> u32 {
        self.0 * minutes
    }
}

// Static: one compiled copy per T. All plans in the slice have the same type.
fn cheapest<T: Tariff>(plans: &[T], minutes: u32) -> Option<u32> {
    plans.iter().map(|plan| plan.cost(minutes)).min()
}

// Dynamic: one compiled copy. The plans may have different types.
fn cheapest_mixed(plans: &[Box<dyn Tariff>], minutes: u32) -> Option<u32> {
    plans.iter().map(|plan| plan.cost(minutes)).min()
}

fn main() {
    assert_eq!(cheapest(&[PerMinute(4), PerMinute(6)], 100), Some(400));

    let mixed: Vec<Box<dyn Tariff>> = vec![Box::new(Flat(900)), Box::new(PerMinute(4))];
    assert_eq!(cheapest_mixed(&mixed, 100), Some(400));
    assert_eq!(cheapest_mixed(&mixed, 300), Some(900));
}
```

### `?Sized`: letting a generic accept a trait object

Every generic type parameter has a hidden bound `T: Sized`, meaning "the size is known at compile time". `dyn Trait` is not `Sized`. So a function `fn banner<W: Write>(out: &mut W)` rejects a `&mut dyn Write`. Writing `W: Write + ?Sized` removes the hidden bound. The function then accepts both concrete writers and trait objects. This is fine whenever the function uses `W` only behind a reference.

### Which traits can be trait objects

A trait is **dyn compatible** when a vtable can be built for it. The main rules:

- every method has a receiver such as `&self`, `&mut self` or `Box<Self>`;
- no method mentions `Self` anywhere else, has generic type parameters of its own, or returns `impl Trait`;
- the trait has no associated constants;
- `Sized` is not a supertrait.

A single method that breaks the rules can be excluded with `where Self: Sized`. Concrete types can still call it, and trait objects cannot.

### One standard trait object: `dyn Any`

`std::any::Any` is implemented for every `'static` type. A `&dyn Any` or `Box<dyn Any>` can be asked at run time whether it holds one exact type:

```rust
use std::any::Any;

let values: Vec<Box<dyn Any>> = vec![Box::new(7_u8), Box::new("seven"), Box::new(9_u8)];
let bytes: Vec<u8> = values
    .iter()
    .filter_map(|value| value.downcast_ref::<u8>().copied())
    .collect();
assert_eq!(bytes, [7, 9]);
```

The check compares exact types. A `u8` is not found by `downcast_ref::<u16>()`. `Box<dyn Error>` offers the same `downcast_ref`, which the review lesson uses.

### A note on `Write`

Task C of the exercise writes to a `std::io::Write`. Its `write` method may accept only *part* of the bytes and returns how many it took. `write_all` calls `write` again until everything is accepted or an error occurs.

### Mistakes people really make

**Putting two types in one `Vec<T>`.**

```rust,compile_fail,E0308
struct Flat(u32);
struct PerMinute(u32);

fn main() {
    let plans = vec![Flat(900), PerMinute(4)];
}
```

```text
error[E0308]: mismatched types
  |
5 |     let plans = vec![Flat(900), PerMinute(4)];
  |                                 ^^^^^^^^^^^^ expected `Flat`, found `PerMinute`
```

Use `Vec<Box<dyn Tariff>>`, or an enum when the set of types is closed.

**Passing a trait object to a plain generic.**

```rust,compile_fail,E0277
use std::io::Write;
fn banner<W: Write>(out: &mut W) -> std::io::Result<()> {
    out.write_all(b"== report ==\n")
}
fn main() {
    let mut buffer = Vec::new();
    let out: &mut dyn Write = &mut buffer;
    banner(out).unwrap();
}
```

```text
error[E0277]: the size for values of type `dyn std::io::Write` cannot be known at compilation time
  |
8 |     banner(out).unwrap();
  |     ------ ^^^ doesn't have a size known at compile-time
...
help: consider relaxing the implicit `Sized` restriction
  |
2 | fn banner<W: Write + ?Sized>(out: &mut W) -> std::io::Result<()> {
  |                    ++++++++
```

**Making a trait object from a trait that is not dyn compatible.** The error is E0038, and it names the method and the reason. For a method `fn cheaper<T: Tariff>(&self, other: &T) -> bool` the reason reads `...because method `cheaper` has generic type parameters`. For a method that returns `Self` it reads `...because method `scaled` references the `Self` type in its return type`. Drill d01 lets you repair a trait of the second kind.

### When to use which

| Use a generic `T: Trait` when | Use `dyn Trait` when |
| --- | --- |
| all values have one type per call | values of different types sit in one collection |
| the call is hot and should be inlined | code size or compile time matters more than the call cost |
| you need `Self`-returning or generic methods | the type is chosen at run time (plugins, configuration) |

### Read more

- [Trait object types](https://doc.rust-lang.org/reference/types/trait-object.html) and [dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility) in the Rust Reference
- [`Sized`](https://doc.rust-lang.org/std/marker/trait.Sized.html), [`Any`](https://doc.rust-lang.org/std/any/trait.Any.html) and [`Write::write_all`](https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all)

## Lesson 03 · Self, supertraits, and associated items

**Assumed:** lessons 01 and 02.

Inside a trait, `Self` is a placeholder for "the type that implements this trait". A trait can contain more than methods. Everything inside it is called an **associated item**:

- A **method** has a receiver: `self`, `&self` or `&mut self`. You call it on a value: `value.name()`.
- An **associated function** has no receiver. You call it on the type: `Type::new()`, or `T::new()` in generic code. Constructors are the usual case.
- An **associated constant** is a value fixed per implementing type: `T::LIMIT`. It lives in the program, not inside each value, so it adds nothing to the size of the type.

A **supertrait** is a requirement on implementors. `trait Limited: PartialOrd + Debug` means "a type may implement `Limited` only if it also implements `PartialOrd` and `Debug`". Two things follow. A provided method in `Limited` may use `<` and `{:?}` on `Self`. And generic code with the bound `T: Limited` may use them too, without listing them again.

```rust
use std::fmt::Debug;

trait Limited: PartialOrd + Debug + Sized {
    const LOW: Self;
    const HIGH: Self;

    // Associated function: no receiver. Call it as `T::span_text()`.
    fn span_text() -> String {
        format!("{:?}..={:?}", Self::LOW, Self::HIGH)
    }

    // Method: takes `self` by value and returns a `Self` by value.
    fn limited(self) -> Self {
        if self < Self::LOW {
            Self::LOW
        } else if self > Self::HIGH {
            Self::HIGH
        } else {
            self
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd)]
struct Percent(u8);

impl Limited for Percent {
    const LOW: Self = Percent(0);
    const HIGH: Self = Percent(100);
}

fn describe<T: Limited>(value: T) -> String {
    format!("{:?} within {}", value.limited(), T::span_text())
}

fn main() {
    assert_eq!(Percent(140).limited(), Percent(100));
    assert_eq!(describe(Percent(7)), "Percent(7) within Percent(0)..=Percent(100)");
    assert_eq!(size_of::<Percent>(), 1); // the constants are not stored in the value
}
```

`Percent` writes two constants and receives two working functions. The provided bodies are written against `Self`, so they work for every implementor.

### Why `Sized` is in the supertrait list

A function that takes or returns a value *by value* must know its size. Inside a trait, `Self` might later be an unsized type such as `dyn Trait` or `str`. So:

- A **declaration** such as `fn make() -> Self;` needs no `Sized` bound. Only the implementation has a body, and the implementing type is known there.
- A **provided body** that moves, assigns or returns a whole `Self` does need it. Either add `Sized` as a supertrait (every implementor must be sized), or add `where Self: Sized` to that one method.

### Why such a trait cannot be a trait object

`dyn Limited` is rejected for several separate reasons, and each one alone is enough. `Sized` is a supertrait. The trait has associated constants. `span_text` has no receiver, so there is no value whose vtable could be used. And `limited` returns `Self`, whose size a trait object does not know. Putting `where Self: Sized` on one function removes only the reason that this function causes.

### Mistakes people really make

**Calling an associated function with dot syntax.**

```rust,compile_fail,E0599
trait Limited: PartialOrd + Sized {
    const LOW: Self;
    fn lowest() -> Self {
        Self::LOW
    }
}
#[derive(PartialEq, PartialOrd)]
struct Percent(u8);
impl Limited for Percent {
    const LOW: Self = Percent(0);
}
fn main() {
    let p = Percent(40);
    let low = p.lowest();
}
```

```text
error[E0599]: no method named `lowest` found for struct `Percent` in the current scope
   |
14 |     let low = p.lowest();
   |                 ^^^^^^ this is an associated function, not a method
   |
   = note: found the following associated functions; to be used as methods, functions must have a `self` parameter
help: use associated function syntax instead
   |
14 -     let low = p.lowest();
14 +     let low = Percent::lowest();
```

**Returning `Self` from a provided body without `Sized`.**

```rust,compile_fail,E0277
trait Blank {
    fn blank() -> Self;
    fn fresh() -> Self {
        Self::blank()
    }
}
```

```text
error[E0277]: the size for values of type `Self` cannot be known at compilation time
  |
3 |     fn fresh() -> Self {
  |                   ^^^^ doesn't have a size known at compile-time
  |
  = note: the return type of a function must have a statically known size
help: consider further restricting `Self`
  |
3 |     fn fresh() -> Self where Self: Sized {
  |                        +++++++++++++++++
```

Note that line 2, the declaration without a body, is accepted.

**Expecting a trait with constants or constructors to work as `dyn`.**

```rust,compile_fail,E0038
trait Limited {
    const LOW: u8;
    fn lowest() -> Self;
    fn get(&self) -> u8;
}
fn read(value: &dyn Limited) -> u8 {
    value.get()
}
```

```text
error[E0038]: the trait `Limited` is not dyn compatible
  |
1 | trait Limited {
  |       ------- this trait is not dyn compatible...
2 |     const LOW: u8;
  |           ^^^ ...because it contains associated const `LOW`
3 |     fn lowest() -> Self;
  |        ^^^^^^ ...because associated function `lowest` has no `self` parameter
  = help: consider moving `LOW` to another trait
```

The usual repair is to split the trait: one small dyn-compatible trait with the `&self` methods, and a second trait for constants and constructors.

### Read more

- [Associated items](https://doc.rust-lang.org/reference/items/associated-items.html) and [supertraits](https://doc.rust-lang.org/reference/items/traits.html#supertraits) in the Rust Reference
- [The `Self` type](https://doc.rust-lang.org/reference/paths.html#self-1) in the Rust Reference

## Lesson 04 · Extension traits and qualified calls

**Assumed:** lessons 01 to 03, modules and `use` (chapter 8).

You cannot add an inherent method to a type from another crate. `impl str { ... }` is allowed only in the crate that defines `str`. You *can* define your own trait and implement it for that type. Such a trait is called an **extension trait**. After `use your::Trait;`, the new method works with dot syntax like any other.

```rust
mod text {
    pub trait Initials {
        fn initials(&self) -> String;
    }

    impl Initials for str {
        fn initials(&self) -> String {
            self.split_whitespace()
                .filter_map(|word| word.chars().next())
                .collect()
        }
    }
}

use text::Initials; // without this line, `.initials()` is not found

fn main() {
    assert_eq!("portable network graphics".initials(), "png");
    // A String works too: method lookup follows String -> str.
    assert_eq!(String::from("read the manual").initials(), "rtm");
}
```

### Coherence and the orphan rule

Rust guarantees that, in a whole program, there is at most one implementation of a given trait for a given type. This property is called **coherence**. Without it, two crates could each implement `Display` for `Vec<u32>`, and the program would have no way to choose.

The **orphan rule** is how the compiler checks coherence one crate at a time. "Local" means "defined in the current crate". An `impl Trait<T1, ..., Tn> for T0` is allowed when:

- the trait is local, or
- at least one of the types `T0, T1, ..., Tn` is local, and no bare type parameter (a `T` not wrapped in another type) appears before the first local type in that list.

So the rule is wider than "your trait or your type". With a local type in the trait's parameters, you may implement a foreign trait for a foreign type:

```rust
struct Level(u8);

impl From<Level> for u8 {
    fn from(level: Level) -> u8 {
        level.0
    }
}

let n: u8 = Level(3).into();
assert_eq!(n, 3);
```

`From` and `u8` are both foreign, but `Level` is local, and the only type before it in the list is `u8`, which is not a type parameter. A few pointer types are transparent for this rule: `&T`, `&mut T`, `Box<T>` and `Pin<T>` count as local when `T` is local, and they do not count as wrapping a bare type parameter.

When neither the trait nor any type is yours, wrap the foreign type in a one-field struct of your own, a **newtype**, and implement the trait for that.

### Which method does a call pick?

For `value.name()`, the compiler builds a list of candidate receiver types: the type of `value`, then what it dereferences to, and so on (`String`, then `str`). For each candidate type, in order, it looks for a method that takes that type by value, then one that takes `&` of it, then `&mut` of it. At each of those steps, inherent methods are checked before trait methods. The first match wins.

Two results follow. An inherent method and a trait method with the same name and the same receiver do not conflict: the inherent one is chosen. And two *trait* methods with the same name and receiver are ambiguous, because neither has priority.

A **qualified call** names the trait and passes the receiver as an ordinary argument: `Trait::name(&value)`. The longest form, `<Type as Trait>::name(&value)`, names the type as well. It always works. You need it most for an associated function that two traits both name: such a function has no receiver, so `Trait::name()` works only when the compiler can find the type some other way, for example from an annotation on the result.

```rust
trait Json {
    fn encode(&self) -> String;
}
trait Csv {
    fn encode(&self) -> String;
}
struct Point(i32, i32);

impl Json for Point {
    fn encode(&self) -> String {
        format!("[{},{}]", self.0, self.1)
    }
}
impl Csv for Point {
    fn encode(&self) -> String {
        format!("{},{}", self.0, self.1)
    }
}

let p = Point(3, 4);
assert_eq!(Json::encode(&p), "[3,4]");
assert_eq!(<Point as Csv>::encode(&p), "3,4");
```

### Mistakes people really make

**Calling a method that two traits in scope both provide.** With the types above, `p.encode()` gives:

```text
error[E0034]: multiple applicable items in scope
   |
20 |     println!("{}", p.encode());
   |                      ^^^^^^ multiple `encode` found
   |
help: disambiguate the method for candidate #1
   |
20 -     println!("{}", p.encode());
20 +     println!("{}", Csv::encode(&p));
```

**Implementing a foreign trait for a foreign type.**

```rust,compile_fail,E0117
use std::str::FromStr;
impl FromStr for Vec<bool> {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(s.chars().map(|c| c == '1').collect())
    }
}
```

```text
error[E0117]: only traits defined in the current crate can be implemented for types defined outside of the crate
  |
2 | impl FromStr for Vec<bool> {
  | ^^^^^^^^^^^^^^^^^---------
  |                  |
  |                  `Vec` is not defined in the current crate
  |
  = note: impl doesn't have any local type before any uncovered type parameters
  = note: define and implement a trait or new type instead
```

Fix it with a newtype (`struct Bits(Vec<bool>);`) or with your own extension trait.

**Forgetting the `use` for an extension trait.** Remove `use text::Initials;` from the first example and the compiler answers:

```text
error[E0599]: no method named `initials` found for reference `&'static str` in the current scope
   = help: items from traits can only be used if the trait is in scope
help: trait `Initials` which provides `initials` is implemented but not in scope; perhaps you want to import it
   |
 1 + use crate::text::Initials;
```

### Read more

- [Orphan rules](https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules) and [trait implementation coherence](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence) in the Rust Reference
- [Method call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html) (the lookup order) and [qualified paths](https://doc.rust-lang.org/reference/paths.html#qualified-paths)

## Lesson 05 · Associated types and generic traits

**Assumed:** lessons 01 to 04, and `Option`.

A trait often has to mention a second type: the item an iterator yields, the result of an addition, the value a parser produces. There are two ways to connect that second type to the trait, and they mean different things.

An **associated type** is declared inside the trait (`type Value;`) and fixed by each implementation (`type Value = i64;`). Think of it as an *output*: once you know the implementing type, the associated type is decided. A type can implement the trait only once, so there is exactly one answer.

A **generic trait parameter** is written after the trait name (`trait ConvertTo<T>`). Think of it as an *input*: `ConvertTo<f64>` and `ConvertTo<String>` are two different traits, and one type may implement both. The user of the trait chooses which one is meant.

```rust
// Output style: each decoder produces exactly one kind of value.
trait Decoder {
    type Value;
    fn decode(&self, raw: &str) -> Option<Self::Value>;
}

struct IntDecoder;
struct YesNo;

impl Decoder for IntDecoder {
    type Value = i64;
    fn decode(&self, raw: &str) -> Option<i64> {
        raw.trim().parse().ok()
    }
}
impl Decoder for YesNo {
    type Value = bool;
    fn decode(&self, raw: &str) -> Option<bool> {
        match raw.trim() {
            "yes" => Some(true),
            "no" => Some(false),
            _ => None,
        }
    }
}

// Generic code names the output as `D::Value`.
fn decode_all<D: Decoder>(decoder: &D, raws: &[&str]) -> Vec<D::Value> {
    raws.iter().filter_map(|raw| decoder.decode(raw)).collect()
}

// Input style: one type can convert to several targets.
trait ConvertTo<T> {
    fn convert(&self) -> T;
}
struct Meters(f64);

impl ConvertTo<f64> for Meters {
    fn convert(&self) -> f64 {
        self.0
    }
}
impl ConvertTo<String> for Meters {
    fn convert(&self) -> String {
        format!("{} m", self.0)
    }
}

fn main() {
    assert_eq!(decode_all(&IntDecoder, &["4", "x", " 9 "]), [4, 9]);
    assert_eq!(decode_all(&YesNo, &["yes", "maybe", "no"]), [true, false]);

    let height = Meters(1.8);
    let number: f64 = height.convert(); // the annotation selects the impl
    let text: String = height.convert();
    assert_eq!((number, text.as_str()), (1.8, "1.8 m"));
}
```

### Working with an associated type in bounds

- `D::Value` (in full, `<D as Decoder>::Value`) names the type.
- `D: Decoder<Value = i64>` accepts only decoders whose output is `i64`.
- `where D::Value: Display` puts a bound on the output. This is a bound on a *different type* than `D`, so it must go in a `where` clause or be written as `D: Decoder<Value: Display>`.

Many standard traits combine both styles. `Add<Rhs>` has the input `Rhs` (what you add) and the associated type `Output` (what you get). `Iterator` has only the output `Item`: a given iterator yields one kind of item.

### Mistakes people really make

**Trying to implement an output-style trait twice.** Different associated types do not make the two impls different.

```rust,compile_fail,E0119
trait Decoder {
    type Value;
    fn decode(&self, raw: &str) -> Option<Self::Value>;
}
struct Flexible;
impl Decoder for Flexible {
    type Value = i64;
    fn decode(&self, raw: &str) -> Option<i64> {
        raw.trim().parse().ok()
    }
}
impl Decoder for Flexible {
    type Value = bool;
    fn decode(&self, raw: &str) -> Option<bool> {
        raw.trim().parse().ok()
    }
}
```

```text
error[E0119]: conflicting implementations of trait `Decoder` for type `Flexible`
   |
 6 | impl Decoder for Flexible {
   | ------------------------- first implementation here
...
12 | impl Decoder for Flexible {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^ conflicting implementation for `Flexible`
```

If one type really needs several results, the trait needs a parameter: `Decoder<T>`.

**Leaving the choice open with an input-style trait.** With the two `ConvertTo` impls above, `let value = height.convert();` gives:

```text
error[E0283]: type annotations needed
   |
17 |     let value = height.convert();
   |         ^^^^^          ------- type must be known at this point
   |
note: multiple `impl`s satisfying `Meters: ConvertTo<_>` found
help: consider giving `value` an explicit type
```

This is the cost of the input style: sometimes the caller must say which impl is meant.

**Using the output type without a bound.** Generic code knows nothing about `D::Value` unless a bound says so.

```rust,compile_fail,E0599
trait Decoder {
    type Value;
    fn decode(&self, raw: &str) -> Option<Self::Value>;
}
fn show_all<D: Decoder>(decoder: &D, raws: &[&str]) -> Vec<String> {
    raws.iter()
        .filter_map(|raw| decoder.decode(raw))
        .map(|value| value.to_string())
        .collect()
}
```

```text
error[E0599]: `<D as Decoder>::Value` doesn't implement `std::fmt::Display`
  |
8 |         .map(|value| value.to_string())
  |                            ^^^^^^^^^ method cannot be called on `<D as Decoder>::Value` due to unsatisfied trait bounds
  |
  = note: the following trait bounds were not satisfied:
          `<D as Decoder>::Value: std::fmt::Display`
```

Add `where D::Value: std::fmt::Display` to the function.

### When to use which

| Question | Associated type | Trait parameter |
| --- | --- | --- |
| How many impls per type? | one | one per parameter value |
| Who decides the type? | the implementor | the caller |
| Does inference need help? | rarely | sometimes (E0283) |
| Standard examples | `Iterator::Item`, `Deref::Target` | `From<T>`, `PartialEq<Rhs>` |

### Read more

- [Associated types](https://doc.rust-lang.org/reference/items/associated-items.html#associated-types) in the Rust Reference
- [`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html), [`Add`](https://doc.rust-lang.org/std/ops/trait.Add.html) and [`From`](https://doc.rust-lang.org/std/convert/trait.From.html) as models of each style

## Lesson 06 · Opaque results, lifetimes, and precise capture

**Assumed:** lessons 01 to 05, lifetimes in signatures (chapter 5), and simple closures.

Some types cannot be written down. Every closure has a type that only the compiler knows, and a chain such as `.iter().map(..).filter(..)` has a long type full of closure types. To return such a value, write `impl Trait` as the return type:

```rust
fn evens(limit: u32) -> impl Iterator<Item = u32> {
    (0..limit).filter(|n| n % 2 == 0)
}

fn at_least(min: usize) -> impl Fn(&str) -> bool {
    move |text| text.len() >= min
}

assert_eq!(evens(7).collect::<Vec<_>>(), [0, 2, 4, 6]);
let long = at_least(4);
assert!(long("rust") && !long("go"));
```

This is an **opaque type**. The function body decides the one concrete type. The caller is told only "it implements this trait". There is no boxing and no vtable: the caller's code is compiled against the real type, and only its name is hidden.

Two consequences:

- **One type per function.** All `return` paths must produce the same concrete type. If two branches build different iterators, return `Box<dyn Iterator<Item = T>>` instead.
- **A closure is a struct of its captures.** `at_least` returns a value that contains one `usize`. The `move` keyword puts `min` itself into the closure. Without `move` the closure would hold a reference to a local variable that is about to disappear, and the compiler rejects that (E0373). If the captured value is itself a reference, `move` copies the reference, and the closure then borrows whatever that reference points to.

`impl Trait` in *argument* position is something else: `fn total(values: impl Iterator<Item = u32>)` is a short way to write a generic parameter without a name.

### What the hidden type may borrow: capture

An opaque type may contain references, so the compiler must know how long the returned value can live. It does not look inside the function body for this. It uses the signature alone.

In edition 2024 the rule is: a return-position `impl Trait` **captures** every generic parameter in scope, including every lifetime, also the unnamed lifetimes of `&` arguments. "Captures" means "is allowed to use". The caller must then treat the result as if it borrowed from *all* of those arguments.

That is correct for this function, whose iterator really does borrow `lines`:

```rust
fn longer_than(lines: &[String], min: usize) -> impl Iterator<Item = &str> {
    lines
        .iter()
        .map(|line| line.as_str())
        .filter(move |line| line.len() > min)
}

let lines = vec![String::from("a"), String::from("abc")];
assert_eq!(longer_than(&lines, 1).collect::<Vec<_>>(), ["abc"]);
```

It is too strict for the next one. The result does not use `label` after the function returns, but the default rule says it might:

```rust,compile_fail,E0502
fn numbered(start: u32, label: &str) -> impl Iterator<Item = u32> {
    println!("numbering {label}");
    start..
}
fn main() {
    let mut label = String::from("rows");
    let numbers = numbered(1, &label);
    label.push('!');
    println!("{:?}", numbers.take(2).collect::<Vec<_>>());
}
```

```text
error[E0502]: cannot borrow `label` as mutable because it is also borrowed as immutable
  |
7 |     let numbers = numbered(1, &label);
  |                               ------ immutable borrow occurs here
8 |     label.push('!');
  |     ^^^^^^^^^^^^^^^ mutable borrow occurs here
9 |     println!("{:?}", numbers.take(2).collect::<Vec<_>>());
  |                      ------- immutable borrow later used here
  |
note: this call may capture more lifetimes than intended, because Rust 2024 has adjusted the `impl Trait` lifetime capture rules
help: use the precise capturing `use<...>` syntax to make the captures explicit
  |
1 | fn numbered(start: u32, label: &str) -> impl Iterator<Item = u32> + use<> {
  |                                                                   +++++++
```

A **precise capture** list, `+ use<...>`, replaces the default. It names exactly the generic parameters the hidden type may use. `use<>` means "none", and `use<'a>` means "only `'a`".

```rust
fn numbered(start: u32, label: &str) -> impl Iterator<Item = u32> + use<> {
    println!("numbering {label}");
    start..
}

let mut label = String::from("rows");
let numbers = numbered(1, &label);
label.push('!'); // allowed: `numbers` does not borrow `label`
assert_eq!(numbers.take(2).collect::<Vec<_>>(), [1, 2]);
```

Two limits apply today. Every *type* parameter in scope must be listed, so only lifetimes can be left out. And a lifetime that appears in the bounds, such as `'a` in `Item = &'a str`, cannot be left out.

### `use<'a>` is not `+ 'a`

They look similar and say different things. `use<'a>` is a permission for the function body: the hidden type may contain data borrowed for `'a`. `+ 'a` is a promise to the caller: the hidden type is valid for at least `'a`. The form `+ 'static` is the common one. It promises that the result holds no short-lived borrow, which is what `std::thread::spawn` requires of its closure.

### Finding the smallest bound

Write the generic function first with no bounds. The compiler then lists what the body needs:

```rust,compile_fail,E0599
fn joined<T>(items: &[T]) -> String {
    let parts: Vec<String> = items.iter().map(|item| item.to_string()).collect();
    parts.join(", ")
}
```

```text
error[E0599]: the method `to_string` exists for reference `&T`, but its trait bounds were not satisfied
  = note: the following trait bounds were not satisfied:
          `T: std::fmt::Display`
          which is required by `T: ToString`
```

Add what the note names (`T: Display`) and nothing more. Each extra bound shuts out callers for no benefit.

### Mistakes people really make

**Two concrete types behind one `impl Trait`.**

```rust,compile_fail,E0308
fn steps(reverse: bool) -> impl Iterator<Item = char> {
    if reverse {
        "abc".chars().rev()
    } else {
        "abc".chars()
    }
}
```

```text
error[E0308]: `if` and `else` have incompatible types
  |
2 | /     if reverse {
3 | |         "abc".chars().rev()
  | |         ------------------- expected because of this
4 | |     } else {
5 | |         "abc".chars()
  | |         ^^^^^^^^^^^^^ expected `Rev<Chars<'_>>`, found `Chars<'_>`
6 | |     }
  | |_____- `if` and `else` have incompatible types
  |
help: you could change the return type to be a boxed trait object
  |
1 - fn steps(reverse: bool) -> impl Iterator<Item = char> {
1 + fn steps(reverse: bool) -> Box<dyn Iterator<Item = char>> {
```

**Capturing more than intended.** This is E0502 above, or E0597 when the over-borrowed value is dropped too early. Read the `note:` line about Rust 2024 and add a `use<...>` list.

**Forgetting `move` on a returned closure.** Without it, `at_least` fails with E0373, `closure may outlive the current function, but it borrows `min``, and the help line suggests `move`.

### Read more

- [Impl trait types](https://doc.rust-lang.org/reference/types/impl-trait.html) in the Rust Reference, including [capturing](https://doc.rust-lang.org/reference/types/impl-trait.html#capturing) and [precise capturing](https://doc.rust-lang.org/reference/types/impl-trait.html#precise-capturing)
- [RPIT lifetime capture rules](https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html) in the Edition Guide

## Lesson 07 · Capstone: a generic validation pipeline

The capstone joins the two kinds of dispatch in one function, and you decide how the data moves. No new feature is needed. Plan on paper before you write code.

**The source is generic.** `S: Source<Item = String>` reads as "any source whose associated type is `String`". The pipeline is compiled once for each source type, and it knows that every pulled value is an owned `String`. Ask yourself: who owns each string after `pull` returns it?

**The rules are trait objects.** A slice of `&dyn Rule` can hold rules of different types, and the caller can choose them at run time. `Rule` has one `&self` method with no generics, so it is dyn compatible. Ask yourself: what should "every rule accepts" mean when there are no rules? Iterator methods such as `all` and `any` have a defined answer for an empty input; look it up before you rely on it.

**Ownership decides the cost.** A rule only needs to *look* at a string, so lend it a `&str`. A string that passes must end in the output vector. A `String` can be moved there without copying its text. The tests check this: they compare buffer addresses and count allocations, so a solution that clones each string fails.

**The loop must end.** A source signals the end by returning `None`. Write the loop so that this is the only way out, and check what your code does when the source is empty from the start. The tests run your loop on a helper thread and fail with a message after ten seconds if it never ends.

Suggested order: write `accepted`, then `collect_source`, then `run`. Run `cargo test --test chapter11 l07_` after each one.

When all tests pass, try the lab in the lesson header: add a second source type and a new rule *without* changing `run`. If that works, your pipeline depends only on the two traits, which is the point of the chapter.

### Read more

- [`Iterator::all`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.all) and [`std::iter::from_fn`](https://doc.rust-lang.org/std/iter/fn.from_fn.html)
- [`while let`](https://doc.rust-lang.org/reference/expressions/loop-expr.html#while-let-patterns) in the Rust Reference
