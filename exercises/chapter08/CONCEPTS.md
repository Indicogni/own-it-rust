# Chapter 8 concepts — Modules, crates and Cargo

[Chapter guide](README.md) · [Hints](HINTS.md) · [Lab](label_lab/README.md)

Read one section, then do the lesson with the same number. Every Rust example here is checked with `rustdoc --test`; every diagnostic was copied from the compiler this course pins (Rust 1.99, edition 2024) and shortened.

## What you will be able to do

So far each program was one file. Real programs are many files, several programs that share code, and code written by other people. This chapter is about the tools Rust gives you to organise all that and, more importantly, to decide what the rest of the world may depend on.

After this chapter you can:

- say what a package, a crate, a target, an edition and a profile are, and find each one in a `Cargo.toml`;
- split code into modules and files, and choose the visibility of every item on purpose;
- design a small public surface (a *facade*) in front of private implementation code;
- choose between `const` and `static`, and between `#[cfg]` and `cfg!`;
- write unit tests, integration tests and doc-tests, and know what each kind can prove;
- read a dependency requirement, explain what `Cargo.lock` is for, and build a workspace with a feature and an optional dependency.

## Before you start

Chapters 3 to 7: basic types, ownership and moves, references, expressions (`match`, `if let`), and `Result` with `?`. No knowledge of traits or generics is needed.

## Time

Reading a section takes 10 to 15 minutes.

| Part | Estimate |
| --- | --- |
| Lessons 01 to 07 | 30 to 45 minutes each, including reading |
| Lesson 08 (capstone) | 45 minutes |
| Lab `label_lab` | 2 to 3 hours |
| Review R01, code review C01 | 45 minutes each |
| Drills | 45 minutes |

## Lesson 01 · Crates, editions and profiles

### The mental model

Five words are easy to mix up. Each one answers a different question.

- A **package** is a directory with a `Cargo.toml`. It is what Cargo builds, tests and publishes. The question it answers: *what belongs together and has one version number?*
- A **target** is one thing a package can produce: a library, a binary, an example, a test, a benchmark. A package has at most one library and any number of the others.
- A **crate** is what the compiler `rustc` compiles in one run: one root file plus every module reachable from it. Cargo compiles each target as its own crate. So one package usually means several crates.
- The **edition** (2015, 2018, 2021, 2024) selects a dialect of the language: which words are keywords, a few defaults, some rules for temporaries. It is set per crate, by the `edition` key. Crates of different editions link together without problems, and a new compiler still compiles every old edition.
- A **profile** is a named set of compiler settings. `cargo build` uses `dev` (no optimisation, extra checks), and `cargo test` uses `test`, which takes its settings from `dev`. `--release` uses `release` (optimised, fewer checks).

Two more keys describe compilers, and neither is the edition. The *toolchain* is the compiler that is installed and runs; this course pins it in `rust-toolchain.toml`. The `rust-version` key in `Cargo.toml` is a promise: "this package works with this compiler version or newer". Cargo refuses to build a package with a compiler older than its `rust-version`.

### Worked example

A package for a weather station might look like this:

```text
weather/
├── Cargo.toml            [package] name = "weather-station", edition = "2024"
├── src/lib.rs            library, crate name `weather_station`
├── src/main.rs           binary, program `weather-station`   (uses the library)
├── src/bin/calibrate.rs  binary, program `calibrate`         (uses the library)
├── examples/demo.rs      example `demo`
└── tests/readings.rs     integration test `readings`         (uses the library)
```

That is one package, five targets and five crates. `cargo test` compiles even more: the library a second time with its unit tests switched on, and so on. Note the library's name: a package name may contain `-`, but a crate name is a Rust identifier, so Cargo turns `-` into `_`. Other code writes `use weather_station::...`.

A crate can ask the compiler about its own build. These macros are replaced by plain values while the crate is compiled:

```rust
/// Converts tenths of a degree to whole degrees, rounding toward zero.
fn whole_degrees(tenths: i32) -> i32 {
    // Checked in `dev` and `test` builds, removed in `release` builds.
    debug_assert!(tenths.abs() < 2_000, "sensor range is -200.0 to 200.0");
    tenths / 10
}

fn main() {
    assert_eq!(whole_degrees(-215), -21);

    // A `bool` that is fixed when the crate is compiled.
    let extra_checks: bool = cfg!(debug_assertions);
    println!("debug assertions: {extra_checks}");

    // Where am I? All three are compile-time text and numbers.
    println!("{} at {}:{}", module_path!(), file!(), line!());

    // Cargo sets this variable while it compiles a crate. Outside Cargo it is
    // not set, so this example asks politely with `option_env!`.
    let name: Option<&'static str> = option_env!("CARGO_PKG_NAME");
    println!("package: {name:?}");
}
```

`debug_assertions` is a setting of the profile, not of the edition and not of the optimiser. It is on in `dev` and `test`, off in `release` and `bench`, and a `[profile.*]` table in `Cargo.toml` can change it.

### Mistakes people really make

**Thinking the edition is a compiler version.** The newest compiler compiles edition 2015 code today. What changes with the edition is the language dialect. For example, `gen` became a reserved word in edition 2024:

```rust,compile_fail
fn main() {
    let gen = 5;
    println!("{gen}");
}
```

```text
error: expected identifier, found reserved keyword `gen`
 --> src/main.rs:2:9
  |
2 |     let gen = 5;
  |         ^^^ expected identifier, found reserved keyword
  |
help: escape `gen` to use it as an identifier
  |
2 |     let r#gen = 5;
  |         ++
```

The same file compiles in edition 2021 with the same compiler. The fix is another name, or the raw identifier `r#gen` when the name comes from older code you cannot change.

**Using the package name where the crate name is needed.** If the package is `weather-station`, then `use weather-station::Reading;` is a syntax error, and a wrong guess such as `use weatherstation::Reading;` gives:

```text
error[E0432]: unresolved import `weatherstation`
 --> src/main.rs:1:5
  |
1 | use weatherstation::Reading;
  |     ^^^^^^^^^^^^^^ use of unresolved module or unlinked crate `weatherstation`
```

Write the name with underscores: `weather_station`. A `[lib] name = "..."` entry can also give the library a completely different name; this course's own support library does that (package `own_it`, library crate `practice`).

**Putting a necessary check in `debug_assert!`.** The check disappears in release builds. Use `debug_assert!` only for conditions that your own code already guarantees, as a second line of defence while developing. Input validation needs `assert!` or, better, a `Result`.

### When to use which

| You want to | Use |
| --- | --- |
| share code between programs | a library target (`src/lib.rs`) |
| ship a program | a binary target (`src/main.rs`, `src/bin/*.rs`) |
| use newer syntax or defaults | a newer `edition` (per crate) |
| promise an oldest supported compiler | `rust-version` |
| make every contributor use one compiler | `rust-toolchain.toml` |
| different settings for development and delivery | profiles |

### Read more

- [Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html) and [the package layout](https://doc.rust-lang.org/cargo/guide/project-layout.html) in the Cargo book
- [Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) and [the `rust-version` field](https://doc.rust-lang.org/cargo/reference/rust-version.html) in the Cargo book
- [The Edition Guide](https://doc.rust-lang.org/edition-guide/)
- [Crates and source files](https://doc.rust-lang.org/reference/crates-and-source-files.html) in the Rust Reference

## Lesson 02 · Nested modules and private implementation

### The mental model

A crate is a tree of **modules**. The root of the tree is the crate's root file. A module is a named container for items: functions, types, constants, other modules.

One rule decides who may use what:

> An item without `pub` can be used in the module that declares it and in every module nested inside that module. Nowhere else.

So privacy looks *down* the tree, never up. A child module can use the private items of its parent and grandparent. A parent cannot use the private items of its child. The reason is practical: a module is a place where you keep a promise (an *invariant*) about some data, and you can only keep it if you know every piece of code that can touch the data. That code is the module and what is inside it.

`pub` and its restricted forms widen the rule one step at a time:

| Written | Visible in |
| --- | --- |
| (nothing) | this module and its descendants |
| `pub(super)` | the parent module and its descendants |
| `pub(crate)` | the whole crate |
| `pub(in crate::a::b)` | the named ancestor module and its descendants |
| `pub` | wherever the *containing module* is visible |

Read the last row twice. `pub` does not mean "visible to the world". A `pub fn` in a private module can be reached only by code that can reach the module.

### Worked example

```rust
mod thermostat {
    // A child module that only `thermostat` uses.
    mod limits {
        pub(super) const MIN_TENTHS: i32 = 50;
        pub(super) const MAX_TENTHS: i32 = 300;
    }

    /// The setting the device accepts for a requested temperature.
    pub fn accepted(tenths: i32) -> i32 {
        tenths.clamp(limits::MIN_TENTHS, limits::MAX_TENTHS)
    }
}

fn main() {
    assert_eq!(thermostat::accepted(999), 300);
    assert_eq!(thermostat::accepted(210), 210);
    assert_eq!(thermostat::accepted(-40), 50);
}
```

```text
crate
├── main                  can use: thermostat, thermostat::accepted
└── thermostat            (private to the crate root and below)
    ├── accepted          pub        -> visible wherever `thermostat` is
    └── limits            (private to `thermostat` and below)
        ├── MIN_TENTHS    pub(super) -> visible in `thermostat`
        └── MAX_TENTHS    pub(super) -> visible in `thermostat`
```

Callers can ask for a setting. They cannot read the limits, so the limits can change, or become a table per device model, without breaking any caller.

### Modules and files

A module's body can stand in a block (`mod limits { ... }`) or in another file. To move it, write only the declaration `mod limits;` and put the body in a file that the compiler finds by name:

```text
src/main.rs               contains:  mod thermostat;
src/thermostat.rs         contains:  mod limits;   pub fn accepted ...
src/thermostat/limits.rs  contains:  pub(super) const MIN_TENTHS ...
```

The older layout `src/thermostat/mod.rs` instead of `src/thermostat.rs` is also accepted. Moving a body to a file changes nothing else: paths, privacy and callers stay exactly the same. A file becomes part of the crate only through a `mod` declaration; a `.rs` file that nobody declares is not compiled at all.

### Mistakes people really make

**Reaching into a child's private items.** Add this line to `main` in the example:

```rust,compile_fail,E0603
mod thermostat {
    mod limits {
        pub(super) const MAX_TENTHS: i32 = 300;
    }
    pub fn accepted(tenths: i32) -> i32 {
        tenths.min(limits::MAX_TENTHS)
    }
}

fn main() {
    println!("{}", thermostat::accepted(999));
    println!("{}", thermostat::limits::MAX_TENTHS);
}
```

```text
error[E0603]: module `limits` is private
  --> src/main.rs:12:32
   |
12 |     println!("{}", thermostat::limits::MAX_TENTHS);
   |                                ^^^^^^  ---------- constant `MAX_TENTHS` is not publicly re-exported
   |                                |
   |                                private module
```

Do not answer this error by adding `pub` until it compiles. First ask whether the caller should know the value at all. If it should, add a function such as `thermostat::max_tenths()` and keep the module private.

**Declaring a module whose file does not exist** (or has another name):

```text
error[E0583]: file not found for module `schedule`
 --> src/main.rs:1:1
  |
1 | mod schedule;
  | ^^^^^^^^^^^^^
  |
  = help: to create the module `schedule`, create file "src/schedule.rs" or "src/schedule/mod.rs"
```

The opposite mistake is quieter: you create `src/schedule.rs` and forget `mod schedule;`. Nothing fails; the file is simply ignored, and the first use of its items says "cannot find". Write the `mod` line in the *parent* module, once. Other modules then reach it with a path or with `use`, not with a second `mod`.

**Expecting `pub fn` in a private module to be public.** It is not; see the table above. This is useful: you can write `pub` freely inside a private module and decide in one place, at the module declaration, how far the whole group is visible.

### When to use which

Start private. Widen only when the compiler shows you a real caller, and then choose the narrowest form that works: `pub(super)` for a helper of the parent, `pub(crate)` for something the whole crate shares, `pub` for what you are ready to support for outside users.

### Read more

- [Modules](https://doc.rust-lang.org/reference/items/modules.html) in the Rust Reference, including the rules for module files
- [Visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html) in the Rust Reference

## Lesson 03 · Paths, imports and reexports

### The mental model

A **path** names an item by its position in the module tree, like a file path names a file. It can start in four places:

| Start | Meaning |
| --- | --- |
| `crate::` | the root of the current crate |
| `self::` | the current module |
| `super::` | the parent of the current module |
| a crate name, such as `std::` | the root of another crate |

A path that starts with none of these is looked up in the current scope first.

`use` does not create anything and does not load anything. It adds a **name** to the current module that points at an existing item. Two consequences follow. First, an alias (`use a::Thing as Other;`) and the original are one and the same item. Second, a `use` line is valid only in the module where it is written; a child module does not inherit the imports of its parent.

`pub use` makes that extra name visible to others. This is a **reexport**, and it is the tool for a *facade*: users write a short, stable path, while the item is defined somewhere deeper that stays private and free to move.

Some names need no import at all. The **prelude** is a small list from the standard library that every module gets: `Option`, `Some`, `None`, `Result`, `Vec`, `String`, `drop`, and more. When the same name is available from several sources, the closest one wins: items and `use` declarations in the module come first, then names brought in by a glob import (`use a::*;`), then the prelude.

Structs add one more rule. `pub struct` makes the *type* visible; each **field** is private unless it has its own `pub`. Code outside the defining module then cannot build the struct with a literal and cannot read the field. It must use the functions you provide.

### Worked example

```rust
mod geometry {
    mod shapes {
        #[derive(Debug, PartialEq)]
        pub struct Circle {
            radius: f64, // private: every Circle has a radius > 0
        }

        impl Circle {
            pub fn new(radius: f64) -> Option<Circle> {
                if radius > 0.0 && radius.is_finite() {
                    Some(Circle { radius })
                } else {
                    None
                }
            }

            pub fn diameter(&self) -> f64 {
                self.radius * 2.0
            }
        }
    }

    // The facade: one public name, for a type defined in a private module.
    pub use shapes::Circle;
}

// An alias in this module. `Round` and `geometry::Circle` are one type.
use geometry::Circle as Round;

fn main() {
    let wheel: Round = Round::new(0.35).expect("a positive radius");
    let same: geometry::Circle = wheel; // no conversion: it is the same type
    assert_eq!(same.diameter(), 0.7);
    assert_eq!(geometry::Circle::new(-1.0), None);

    // The compiler knows the item by its defining path, whatever you call it.
    assert!(std::any::type_name::<Round>().ends_with("geometry::shapes::Circle"));
}
```

```text
name used by callers          item
geometry::Circle  ──────┐
Round             ──────┼──>  struct Circle   (defined in geometry::shapes)
shapes::Circle    ──────┘     only `geometry` can use this last path
```

### Mistakes people really make

**Building a struct around its constructor.** Outside the module, a struct literal needs access to every field:

```rust,compile_fail,E0451
mod geometry {
    pub struct Circle {
        radius: f64,
    }
}

fn main() {
    let c = geometry::Circle { radius: -1.0 };
}
```

```text
error[E0451]: field `radius` of struct `Circle` is private
 --> src/main.rs:8:32
  |
8 |     let c = geometry::Circle { radius: -1.0 };
  |                                ^^^^^^ private field
```

The error is the feature. Fix the caller (use `Circle::new`), not the struct. If you make the field `pub`, nothing can guarantee a positive radius any more.

**Reexporting more than the item allows.** `pub use` cannot make an item more visible than its own declaration:

```rust,compile_fail,E0364
mod geometry {
    mod shapes {
        pub(super) struct Circle;
    }
    pub use shapes::Circle;
}

fn main() {}
```

```text
error[E0364]: `Circle` is private, and cannot be re-exported
 --> src/main.rs:5:13
  |
5 |     pub use shapes::Circle;
  |             ^^^^^^^^^^^^^^
```

Declare the item `pub` inside its private module, and let the reexport decide how far it travels.

**Expecting a child module to see the parent's imports.**

```rust,compile_fail,E0425
use std::collections::HashMap;

mod index {
    pub fn empty() -> HashMap<String, u32> {
        HashMap::new()
    }
}

fn main() {
    let _map: HashMap<String, u32> = index::empty();
}
```

```text
error[E0425]: cannot find type `HashMap` in this scope
 --> src/main.rs:4:23
  |
4 |     pub fn empty() -> HashMap<String, u32> {
  |                       ^^^^^^^ not found in this scope
  |
help: consider importing one of these structs
  |
4 +     use std::collections::HashMap;
```

Repeat the `use` inside `mod index`. A test module has the same need, which is why test modules start with `use super::*;`.

### When to use which

- Inside a crate, prefer `crate::` paths in `use` lines. They stay correct when you move the file that contains them. Use `super::` for close relatives, such as a test module and its parent.
- Import types by name (`use std::collections::HashMap;`). For functions, import the module and keep one prefix (`use std::fs;` then `fs::read`), so the origin stays visible at the call.
- Keep glob imports for test modules and for modules designed as preludes. Two globs that offer the same name make every use of that name an error (drill d09).
- Give outside users one short path per item with `pub use`, and keep the defining modules private.

### Read more

- [Paths](https://doc.rust-lang.org/reference/paths.html) and [use declarations](https://doc.rust-lang.org/reference/items/use-declarations.html) in the Rust Reference
- [The standard prelude](https://doc.rust-lang.org/std/prelude/index.html)
- [Preludes](https://doc.rust-lang.org/reference/names/preludes.html) in the Rust Reference

## Lesson 04 · Constants and statics

### The mental model

Both `const` and `static` give a name to a value that is computed while the program is compiled. They differ in what the name stands for.

A **`const`** is a *value with a name*. Wherever you write the name, the compiler behaves as if you had written the value there. A const is not one place in memory. If you take a reference to a const twice, the two references may or may not point to the same address, and you must not rely on either result.

A **`static`** is a *place with a name*. There is exactly one copy, at one address, and it exists from program start to program end. A reference to a static has the lifetime `'static`. Because any thread can reach it, the type of a (non-`mut`) static must be `Sync`, which means "safe to share between threads" (chapter 19 explains the trait in detail). A static is never dropped; its destructor does not run at exit.

```text
const LIMIT: u32 = 10;            static TABLE: [u8; 3] = [1, 2, 3];

use A: ... 10 ...                 use A: ──┐
use B: ... 10 ...                 use B: ──┼──> one [1, 2, 3] in the program
(the value is copied into         use C: ──┘    at one fixed address
 each place that uses it)
```

The initialiser of either one runs inside the compiler. It may use literals, arithmetic, other consts, and calls to functions declared `const fn`. It may not allocate memory or call ordinary functions.

### Worked example

```rust
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};

const SECONDS_PER_HOUR: u64 = 3_600;

/// A `const fn` can run at compile time and also at run time.
const fn hours(n: u64) -> u64 {
    n * SECONDS_PER_HOUR
}

const SHIFT: u64 = hours(8); // computed by the compiler
static WEEKDAYS: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];

/// One counter for the whole program. An atomic can change through `&`.
static REQUESTS: AtomicU64 = AtomicU64::new(0);

/// A value that needs run-time work is built on first use, exactly once.
static SHOUTED: LazyLock<Vec<String>> =
    LazyLock::new(|| WEEKDAYS.iter().map(|day| day.to_uppercase()).collect());

fn first_day() -> &'static str {
    WEEKDAYS[0]
}

fn main() {
    assert_eq!(SHIFT, 28_800);
    assert_eq!(hours(1), 3_600); // the same function, called at run time

    // There is one table. Every read of it, from any function, finds the
    // same entry, and that entry points at the same text: no copy is made.
    assert_eq!(first_day(), "Mon");
    assert!(std::ptr::eq(first_day(), WEEKDAYS[0]));

    REQUESTS.fetch_add(1, Ordering::Relaxed);
    REQUESTS.fetch_add(1, Ordering::Relaxed);
    assert_eq!(REQUESTS.load(Ordering::Relaxed), 2);

    assert_eq!(SHOUTED[4], "FRI");
}
```

### Mistakes people really make

**Calling an ordinary function in an initialiser.**

```rust,compile_fail,E0015
fn seconds(hours: u64) -> u64 {
    hours * 3600
}

const SHIFT_SECONDS: u64 = seconds(8);

fn main() {
    println!("{SHIFT_SECONDS}");
}
```

```text
error[E0015]: cannot call non-const function `seconds` in constants
 --> src/main.rs:5:28
  |
5 | const SHIFT_SECONDS: u64 = seconds(8);
  |                            ^^^^^^^^^^
  |
note: function `seconds` is not const
 --> src/main.rs:1:1
  |
1 | fn seconds(hours: u64) -> u64 {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: calls in constants are limited to constant functions, tuple structs and tuple variants
```

If the function only does arithmetic, mark it `const fn`. If it needs the heap, the clock or a file, the value cannot be a const at all: use a `static` with `LazyLock`, as `SHOUTED` above.

**Putting a non-thread-safe type in a static.**

```rust,compile_fail,E0277
use std::cell::RefCell;

static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());

fn main() {
    LOG.borrow_mut().push("start".to_string());
}
```

```text
error[E0277]: `RefCell<Vec<String>>` cannot be shared between threads safely
 --> src/main.rs:3:13
  |
3 | static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
  |             ^^^^^^^^^^^^^^^^^^^^ `RefCell<Vec<String>>` cannot be shared between threads safely
  |
  = help: the trait `Sync` is not implemented for `RefCell<Vec<String>>`
  = note: shared static variables must have a type that implements `Sync`
```

Use the thread-safe relative: an atomic for a number or flag, `Mutex<Vec<String>>` for a collection (`Mutex::new` is a `const fn`, so it works in a static). `static mut` is not the way out. Every use of a `static mut` needs `unsafe`, and in edition 2024 even taking a reference to one is an error by default.

**Trying to change a const.** A const is copied to each place of use, so a change reaches only a temporary copy:

```rust
use std::sync::atomic::{AtomicU32, Ordering};

const VISITS: AtomicU32 = AtomicU32::new(0);

fn main() {
    VISITS.fetch_add(1, Ordering::Relaxed);
    VISITS.fetch_add(1, Ordering::Relaxed);
    assert_eq!(VISITS.load(Ordering::Relaxed), 0); // still zero
}
```

The compiler accepts this and warns:

```text
warning: mutation of an interior mutable `const` item with call to `fetch_add`
  = note: each usage of a `const` item creates a new temporary
  = note: only the temporaries and never the original `const VISITS` will be modified
help: for a shared instance of `VISITS`, consider making it a `static` item instead
```

### When to use which

| Need | Choose |
| --- | --- |
| a number, a size, a small fixed value | `const` |
| an array length such as `[0u8; SIZE]` | `const`, or a call to a `const fn`: the compiler must know the length |
| a pattern in `match` | `const` (a static is a place, and cannot be a pattern) |
| one shared table, or a value whose address matters | `static` |
| a program-wide counter or flag | `static` with an atomic type |
| a program-wide value built at run time | `static` with `LazyLock` or `OnceLock` |
| changing shared data of another type | `static` with `Mutex` |

When both work, prefer `const`.

### Read more

- [Constant items](https://doc.rust-lang.org/reference/items/constant-items.html) and [static items](https://doc.rust-lang.org/reference/items/static-items.html) in the Rust Reference
- [Constant evaluation](https://doc.rust-lang.org/reference/const_eval.html) in the Rust Reference
- [`std::sync::LazyLock`](https://doc.rust-lang.org/std/sync/struct.LazyLock.html) and [`std::sync::OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)
- [Disallow references to `static mut`](https://doc.rust-lang.org/edition-guide/rust-2024/static-mut-references.html) in the Edition Guide

## Lesson 05 · Attributes, cfg and included files

### The mental model

An **attribute** is a note for the compiler, attached to the thing that follows it: `#[derive(Debug)]`, `#[test]`, `#[allow(dead_code)]`. The form `#![...]`, with `!`, applies to the thing it is written *inside*, usually the whole crate or module.

The attribute `#[cfg(predicate)]` is **conditional compilation**. If the predicate is false, the item, statement or field that follows is deleted from the program, very early: before names are looked up and before types are checked. The deleted code must be valid syntax and nothing more.

The macro `cfg!(predicate)` is different. It becomes the literal `true` or `false`. Nothing is deleted; both branches of an `if cfg!(...)` are compiled and type-checked, and the optimiser later removes the branch that can never run.

Predicates are fixed for one compilation:

| Predicate | True when |
| --- | --- |
| `unix`, `windows` | the target operating system belongs to that family |
| `target_os = "linux"`, `target_pointer_width = "64"` | the target has that property |
| `debug_assertions` | the profile enables debug assertions |
| `test` | the crate is compiled as a test (`cargo test`) |
| `feature = "name"` | Cargo enabled that feature of this package |
| `not(p)`, `all(p, q)`, `any(p, q)` | the usual logic |
| `true`, `false` | always, never |

In older code you will see `all()` for "always" and `any()` for "never"; they still work. Cargo also tells the compiler which names and values can exist, so that a typing mistake is reported instead of silently counting as false.

Three more tools move information into the program at compile time. `env!("NAME")` becomes the text of an environment variable as it was when the crate was compiled (a missing variable is a compile error; `option_env!` gives an `Option` instead). `include_str!("file")` becomes the contents of a file as a `&'static str`, and `include_bytes!` as bytes. The path is relative to the source file that contains the macro call.

### Worked example

```rust
/// Two definitions, and exactly one of them exists in any build.
#[cfg(windows)]
fn home_variable() -> &'static str {
    "USERPROFILE"
}

#[cfg(not(windows))]
fn home_variable() -> &'static str {
    "HOME"
}

/// One definition; the value differs per target.
fn line_ending() -> &'static str {
    if cfg!(windows) { "\r\n" } else { "\n" }
}

/// `cfg_attr(p, attr)` applies `attr` only when `p` holds. Here the type can
/// be compared and printed in test builds, and only there.
#[cfg_attr(test, derive(Debug, PartialEq))]
struct Reading {
    tenths: i32,
}

fn main() {
    assert!(!home_variable().is_empty());
    assert!(line_ending().ends_with('\n'));
    assert_eq!(Reading { tenths: 215 }.tenths, 215);

    // `cfg!` and a matching `#[cfg]` statement always agree.
    let mut seen = false;
    #[cfg(target_pointer_width = "64")]
    {
        seen = true;
    }
    assert_eq!(seen, cfg!(target_pointer_width = "64"));
}
```

### Mistakes people really make

**Using `cfg!` where code must disappear.** A function that exists only in some builds cannot be called inside `if cfg!(...)`, because the call is compiled in every build. Here a helper exists only in test builds:

```rust,compile_fail,E0425
#[cfg(test)]
fn reset_counters() {}

fn main() {
    if cfg!(test) {
        reset_counters();
    }
}
```

An ordinary (non-test) build fails:

```text
error[E0425]: cannot find function `reset_counters` in this scope
 --> src/main.rs:6:9
  |
6 |         reset_counters();
  |         ^^^^^^^^^^^^^^ not found in this scope
  |
note: found an item that was configured out
 --> src/main.rs:2:4
  |
1 | #[cfg(test)]
  |       ---- the item is gated here
2 | fn reset_counters() {}
  |    ^^^^^^^^^^^^^^
```

Put `#[cfg(...)]` on the call as well, or, better, give every configuration a function with the same name, as `home_variable` above. Then the calling code has no conditions in it.

**A typing mistake in a predicate.** A misspelt name or value is simply false, and the code quietly disappears. Under Cargo the compiler warns:

```text
warning: unexpected `cfg` condition value: `linus`
 --> src/main.rs:1:7
  |
1 | #[cfg(target_os = "linus")]
  |       ^^^^^^^^^^^^-------
  |                   |
  |                   help: there is a expected value with a similar name: `"linux"`
```

The same check reports `feature = "fast"` when `Cargo.toml` has no feature called `fast`. Read these warnings; they usually mean that code you wrote is not in the program.

**Reading the environment at the wrong time.** `env!` and `option_env!` read a variable on the machine and at the moment of *compilation*, and the result is frozen into the program. `std::env::var` reads it when the program *runs*. Asking for a variable that was not set while compiling gives:

```text
error: environment variable `STATION_ID` not defined at compile time
  = help: use `std::env::var("STATION_ID")` to read the variable at run time
```

### When to use which

- Code that cannot compile in some configuration (a platform API, an optional dependency): `#[cfg]`.
- A value that differs but compiles everywhere: `cfg!` in ordinary code. It keeps all branches checked on every platform, which catches mistakes early.
- A choice the *user of your crate* makes: a Cargo feature (see the lab section).
- Information that only Cargo knows, or generated code: a build script (see the lab section).

### Read more

- [Conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html) and [attributes](https://doc.rust-lang.org/reference/attributes.html) in the Rust Reference
- [Checking conditional configurations](https://doc.rust-lang.org/rustc/check-cfg.html) in the rustc book
- [Environment variables Cargo sets for crates](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates) in the Cargo book
- [`include_str!`](https://doc.rust-lang.org/std/macro.include_str.html), [`env!`](https://doc.rust-lang.org/std/macro.env.html) and [`option_env!`](https://doc.rust-lang.org/std/macro.option_env.html)

## Lesson 06 · Tests and executable documentation

### The mental model

Cargo knows three kinds of tests. They differ in *where the test code stands*, and therefore in what it can see.

| Kind | Where | Compiled as | Can use |
| --- | --- | --- | --- |
| Unit test | inside `src/`, usually in `#[cfg(test)] mod tests` | part of the crate | everything, private items too |
| Integration test | `tests/*.rs` | one separate crate per file | only the library's public API |
| Doc-test | code blocks in `///` and `//!` comments of a library | code outside the library, like a small program per block | only the library's public API |

A unit test checks that a piece works. An integration test checks the promise you make to users, from the user's side of the fence: if an integration test can do something, so can any user. A doc-test does the same and is also the documentation, so the examples that users read cannot become wrong without a test failing.

Some facts that follow from "outside the library":

- Integration tests and doc-tests start with `use your_crate::...`. A unit test starts with `use super::*;`.
- They need a *library* target. A package with only `src/main.rs` has nothing they could import. This is one more reason to keep `main.rs` thin and put the logic in `src/lib.rs`.
- Private items cannot be tested from outside. That is not a problem to work around; it tells you which kind of test to write.

A test is a function marked `#[test]`. It passes when it returns and fails when it panics. `assert_eq!(left, right)` panics when the values differ and prints both; `assert_ne!` is its opposite. Plain `assert!(condition)` prints only the text of the condition. All three macros take an optional message after the required arguments.

### Worked example

A small library file, with all three kinds of test in or around it:

````rust
/// Returns the arithmetic mean of `values`, or `None` for an empty slice.
///
/// ```
/// use stats::mean;
///
/// assert_eq!(mean(&[1.0, 2.0, 6.0]), Some(3.0));
/// assert_eq!(mean(&[]), None);
/// ```
pub fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(sum(values) / values.len() as f64)
}

// Private: only code inside this crate can call it.
fn sum(values: &[f64]) -> f64 {
    values.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_of_nothing_is_zero() {
        assert_eq!(sum(&[]), 0.0); // a unit test may call a private function
    }

    #[test]
    fn mean_of_one_value_is_that_value() {
        assert_eq!(mean(&[4.5]), Some(4.5), "a single reading");
    }
}

fn main() {
    // What `tests/public.rs` could check: only `mean` is visible to it.
    assert_eq!(mean(&[2.0, 4.0]), Some(3.0));
}
````

If this file were `src/lib.rs` of a package named `stats`, `cargo test` would run the two unit tests and then the example in the comment, as a doc-test named `src/lib.rs - mean (line 3)`.

Inside a documentation code block, the word after the opening fence changes how the block is tested:

| Fence | Meaning |
| --- | --- |
| (nothing) or `rust` | compile and run; must not panic |
| `should_panic` | compile and run; must panic |
| `no_run` | compile only (for code that needs a network or runs forever) |
| `compile_fail` | must *not* compile |
| `ignore` | not tested (avoid; prefer `no_run` or `text`) |

A line that starts with `# ` inside the block is compiled but hidden in the rendered page. Use it for set-up that would distract the reader, never to hide the behaviour you are documenting.

### Mistakes people really make

**Testing a private item from `tests/`.** With a library whose module `validate` is private:

```text
error[E0603]: module `validate` is private
   --> tests/public_api.rs:128:25
    |
128 |     let _ = label_core::validate::clean("x");
    |                         ^^^^^^^^  ----- function `clean` is not publicly re-exported
    |                         |
    |                         private module
```

Move that test into a unit test module inside the crate. Do not make the item public only to test it.

**Comparing with `assert!` and getting no information.** The same wrong expectation, written twice:

```text
assertion failed: mean(&[1.0, 2.0]) == Some(2.0)
```

```text
assertion `left == right` failed: mean of 1 and 2
  left: Some(1.5)
 right: Some(2.0)
```

The second output comes from `assert_eq!(mean(&[1.0, 2.0]), Some(2.0), "mean of 1 and 2")`. Use `assert_eq!` and `assert_ne!` whenever you compare two values.

**Trusting a `compile_fail` example too much.** It passes when the code fails to compile for *any* reason, including a spelling mistake in the example. Keep such examples to one or two lines with a single possible error, and re-check them when you rename things.

**Expecting examples in a binary to run.** Doc-tests are collected from library targets only. Comments in `src/main.rs` are never executed.

### When to use which

- A rule of the public API that users rely on: an integration test, and a doc-test if it also explains the API well.
- An internal helper with tricky cases: a unit test next to it.
- Something that must be impossible for users (building a type without validation, calling a hidden helper): a `compile_fail` doc-test.
- A whole program's behaviour (arguments, output, exit status): an integration test that starts the binary. The lab shows how.

### Read more

- [Documentation tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html) in the rustdoc book
- [Tests](https://doc.rust-lang.org/cargo/guide/tests.html) in the Cargo book, and [`cargo test`](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [Testing attributes](https://doc.rust-lang.org/reference/attributes/testing.html) in the Rust Reference
- [`assert_eq!`](https://doc.rust-lang.org/std/macro.assert_eq.html)

## Lesson 07 · Dependency requirements and lockfiles

### The mental model

A dependency line in `Cargo.toml` does not name one version. It states a **requirement**: a range of versions that you accept.

```toml
[dependencies]
parser = "1.2.3"
```

Version numbers follow *semantic versioning*: `MAJOR.MINOR.PATCH`. A release that breaks existing users must raise the leftmost non-zero number. A release that only adds things raises the next number. So `"1.2.3"` is read as "1.2.3, or any later version that should still be compatible". This default form is called a *caret* requirement and can be written `"^1.2.3"`.

| Requirement | Accepts |
| --- | --- |
| `"1.2.3"` | from 1.2.3, below 2.0.0 |
| `"1.2"` | from 1.2.0, below 2.0.0 |
| `"1"` | from 1.0.0, below 2.0.0 |
| `"0.2.3"` | from 0.2.3, below 0.3.0 |
| `"0.0.3"` | only 0.0.3 |
| `"~1.2.3"` | from 1.2.3, below 1.3.0 |
| `"=1.2.3"` | only 1.2.3 |
| `">=1.2, <1.5"` | from 1.2.0, below 1.5.0 |

Look at the two rows that start with zero. Before 1.0 the first non-zero number plays the part of the major version, because a young crate is expected to break things more often.

From all versions that satisfy every requirement in the dependency graph, Cargo chooses one per dependency, normally the newest, and writes the result into **`Cargo.lock`**. From then on every build uses exactly the locked versions, on every machine, until someone runs `cargo update` or changes a requirement. The manifest says what you accept; the lockfile says what you got.

A **path dependency** names a directory instead of a registry: `parser = { path = "../parser" }`. Nothing is downloaded, which makes it the right tool for crates you develop together and for experiments without a network. You can give both: `{ path = "../parser", version = "1.2" }`. Cargo then uses the directory and also checks that the crate found there satisfies the requirement.

### Worked example

Version numbers are numbers. This sounds obvious until a list of versions is sorted as text. The example parses versions into tuples, sorts them properly, and picks the newest one that a *wildcard* requirement `1.*` accepts.

```rust
/// Parses "MAJOR.MINOR.PATCH" with plain decimal numbers.
fn parse(version: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version.split('.').map(|part| part.parse::<u32>().ok());
    let triple = (parts.next()??, parts.next()??, parts.next()??);
    // A fourth part means the text is not MAJOR.MINOR.PATCH.
    if parts.next().is_some() { None } else { Some(triple) }
}

/// The newest version with major number `major`, as the requirement
/// `major.*` would select it.
fn newest_with_major(available: &[&str], major: u32) -> Option<(u32, u32, u32)> {
    available
        .iter()
        .filter_map(|text| parse(text))
        .filter(|version| version.0 == major)
        .max() // tuples compare field by field, as numbers
}

fn main() {
    assert_eq!(parse("1.10.0"), Some((1, 10, 0)));
    assert_eq!(parse("1.x.0"), None);
    assert_eq!(parse("1.2"), None);

    let available = ["0.9.4", "1.2.0", "1.10.1", "1.9.7", "2.0.0"];
    assert_eq!(newest_with_major(&available, 1), Some((1, 10, 1)));
    assert_eq!(newest_with_major(&available, 3), None);

    // As text, the wrong version would win: '9' sorts after '1'.
    let newest_as_text = available.iter().filter(|v| v.starts_with("1.")).max();
    assert_eq!(newest_as_text, Some(&"1.9.7"));
}
```

### Mistakes people really make

**Reading `"1.2.3"` as "exactly 1.2.3".** It is a lower bound and an upper bound. The exact version in use is in `Cargo.lock`; `cargo tree` prints it. If you really need one exact version, write `"=1.2.3"`, and expect conflicts when another crate needs a different one.

**Changing the manifest and building with `--locked`.** `--locked` means "fail instead of changing `Cargo.lock`". Continuous-integration scripts use it so that a build never silently uses versions nobody reviewed. After you add or change a dependency, the lockfile must change, so:

```text
error: cannot update the lock file .../label_lab/Cargo.lock because --locked was passed to prevent this
help: to generate the lock file without accessing the network, remove the --locked flag and use --offline instead.
```

Run the command once without `--locked`, look at the difference in `Cargo.lock`, and commit both files together.

**A path dependency whose version does not match.** With a requirement of `"0.4"` on a local crate that says `version = "0.3.0"`:

```text
error: failed to select a version for the requirement `label_core = "^0.4"`
candidate versions found which didn't match: 0.3.0
location searched: .../label_lab/label_core
required by package `label_cli v0.3.0 (.../label_lab/label_cli)`
```

Note how the message prints the requirement with its hidden caret: `^0.4`.

### When to use which

- Default: the plain caret form, as loose as is true. `"1.2"` if you use something added in 1.2; not `"1.2.7"` because that happens to be installed.
- `=` and `<` bounds: only as a short-term defence against a known bad release. They make your crate hard to combine with others.
- Commit `Cargo.lock`. For a program this gives repeatable builds. When your library is used as a dependency, its lockfile is ignored; the lockfile of the final program decides.
- A workspace has one `Cargo.lock`, at its root, for all members.

### Read more

- [Specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) in the Cargo book
- [`Cargo.toml` versus `Cargo.lock`](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html) in the Cargo book
- [Dependency resolution](https://doc.rust-lang.org/cargo/reference/resolver.html) and [SemVer compatibility](https://doc.rust-lang.org/cargo/reference/semver.html) in the Cargo book

## Lesson 08 · Capstone: library facade and workspace

The capstone joins lessons 02, 03 and 06 in one small design. There is no new language feature here; the task is to place code well.

**The shape.** Think of three layers:

```text
caller (a test, later a binary)
        │  uses only the facade
        ▼
facade: a short `pub use` at the top level, plus functions that combine
        the public operations (the batch function lives here)
        │
        ▼
private module: validation and formatting; free to change at any time
```

The facade function for one label does two jobs in a fixed order: validate, then format. Validation is a private helper that returns an `Option`. Chapter 7 showed how `?` and the methods of `Option` connect two such steps without a `match`.

The batch function must stop at the first invalid input and report *where* it was. Two things need care. The position must be attached to the failure at the moment it happens: turn "no value" into "error with index" there (`Option::ok_or` does that conversion). And the rest of the input must not be processed. A plain `for` loop with an early `return` does this. So does an iterator chain that is collected into a `Result`, because collecting into `Result` stops at the first `Err` (chapter 7 showed this for parsing numbers). A technique test counts allocations to make sure no label after the failure is built.

**Library or binary?** Ask of every line: would a second program want this? Rules about labels (what is valid, how a tag looks) belong in the library. Reading arguments, printing, and choosing an exit status belong in the binary. A binary written this way is a few lines long and almost impossible to get wrong, and all the logic can be tested without starting a process.

**Proving privacy.** A test *inside* the crate can call the private helper, so it cannot show that outsiders are locked out. The proof must come from outside: an integration test or a `compile_fail` doc-test, both of which are separate crates. The lab `label_lab` turns this capstone into a real workspace and has exactly these tests.

**What is in a package archive.** `cargo package --list` prints every file that would be uploaded for a package: the manifest (rewritten into a normalised form, with the original kept as `Cargo.toml.orig`), a `Cargo.lock`, the sources, and by default every other file in the package directory that version control tracks. Check that list before any first release; step 8 of the lab does it locally, without publishing anything.

### Read more

- [`cargo package`](https://doc.rust-lang.org/cargo/commands/cargo-package.html) and [the `include` and `exclude` fields](https://doc.rust-lang.org/cargo/reference/manifest.html#the-exclude-and-include-fields) in the Cargo book
- [`Option::ok_or`](https://doc.rust-lang.org/std/option/enum.Option.html#method.ok_or)
- Design of public APIs in depth (API guidelines, which changes break users, feature design) is the subject of the mastery module [`mastery/m8_api`](../../mastery/m8_api/README.md).

## Lab · A workspace with a feature

Read this before the lab in [`label_lab/`](label_lab/README.md). It introduces four Cargo ideas that the lessons could not exercise inside one test crate.

### Workspaces

A **workspace** is a set of packages that are built together. They share one `Cargo.lock` and one `target/` directory, so they always agree on dependency versions and compile shared dependencies once. The root `Cargo.toml` lists the members:

```toml
[workspace]
members = ["engine", "cli"]
resolver = "3"
```

A root manifest with `[workspace]` and no `[package]` is called a *virtual manifest*. It should name the `resolver` explicitly, because there is no package edition to take the default from: without the key, Cargo falls back to the oldest resolver and prints a warning. Commands run at the root apply to all members; `-p name` selects one.

Members usually share settings. Instead of repeating them, define them once at the root and let each member opt in, key by key:

```toml
# root Cargo.toml
[workspace.package]
edition = "2024"
license = "MIT"

[workspace.dependencies]
engine = { path = "engine", version = "0.3" }

[workspace.lints.rust]
unsafe_code = "forbid"
```

```toml
# cli/Cargo.toml
[package]
name = "cli"
version = "0.3.0"
edition.workspace = true
license.workspace = true

[dependencies]
engine.workspace = true

[lints]
workspace = true
```

Nothing is inherited silently: a member that does not write `edition.workspace = true` does not get the shared edition. Paths in `[workspace.dependencies]` are relative to the root manifest. A member can add keys of its own to an inherited dependency with the long form, for example `engine = { workspace = true, optional = true }`.

### Features and optional dependencies

A **feature** is a named switch of a package. The package declares its features; whoever depends on it decides which to turn on. In the code, a feature is a `cfg` predicate.

```toml
[features]
default = []                  # features that are on unless the user says otherwise
slug = ["dep:label_slug"]     # turning `slug` on also turns the dependency on

[dependencies]
label_slug = { path = "../label_slug", version = "0.3", optional = true }
```

```rust
/// Exists only in builds where the feature is on.
#[cfg(feature = "pretty")]
pub fn pretty(text: &str) -> String {
    format!("[ {text} ]")
}

/// Exists in every build and reports how it was built.
pub fn describe() -> &'static str {
    if cfg!(feature = "pretty") { "with pretty" } else { "plain" }
}

fn main() {
    // This file is tested without any feature, so `pretty` does not exist here.
    assert_eq!(describe(), "plain");
}
```

An **optional dependency** is not compiled at all unless a feature turns it on. The entry `"dep:label_slug"` in a feature's list does that. Without the `dep:` prefix (`slug = ["label_slug"]`), Cargo creates a second feature named after the dependency, and users may start to depend on that accidental name. A feature can also turn on a feature of a dependency: `slug = ["label_core/slug"]`.

Users choose on the command line (`--features slug`, `--all-features`, `--no-default-features`) or in their own manifest (`label_core = { version = "0.3", features = ["slug"] }`).

**Features must only add.** In one build, Cargo compiles each package once, with the *union* of all features that anything in the build asked for. This is called *feature unification*. If crate A needs your library without `slug` and crate B needs it with `slug`, both get the build with `slug`. So turning a feature on must never remove an item or change the meaning of existing behaviour; otherwise A breaks because of B. For the same reason there is no such thing as a "no-std" or "disable-x" feature done right: write the positive feature (`std`, `x`) and make it a default.

Test both ways. Code behind `#[cfg(feature = "...")]` is not even type-checked in a build without the feature, so a default `cargo test` tells you nothing about it.

### Build scripts

A file named `build.rs` in a package root is a **build script**: a small Rust program that Cargo compiles and runs *before* it compiles the package. It runs on the machine that does the building. Its job is to prepare what the crate needs: generate code, compile a C library (chapter 24), or pass information on.

The script talks to Cargo by printing lines that start with `cargo::`:

```rust,no_run
// build.rs
fn main() {
    // Run again only if this file changes. Without any rerun-if line, Cargo
    // runs the script again after every change to any file of the package.
    println!("cargo::rerun-if-changed=build.rs");

    // Cargo gives build scripts variables that the crate cannot see, such as
    // TARGET (for example "x86_64-unknown-linux-gnu").
    let target = std::env::var("TARGET").expect("Cargo sets TARGET for build scripts");

    // Make a value readable in the crate through env!("BUILT_FOR").
    println!("cargo::rustc-env=BUILT_FOR={target}");
}
```

Two different moments are involved, so be exact about them. The script reads `TARGET` with `std::env::var` when the *script runs*. The crate reads `BUILT_FOR` with `env!` when the *crate is compiled*, a moment later. Generated files go into the directory named by the variable `OUT_DIR`, never into `src/`.

Keep build scripts small. Every user of your crate pays for compiling and running them, and they are the first place where a build stops being repeatable.

### Testing a binary

Integration tests cannot import a binary crate, but they can run it. When Cargo builds an integration test for a package that has binaries, it builds those binaries first and passes each path to the test in a compile-time variable `CARGO_BIN_EXE_<name>`. The test starts the program with `std::process::Command` (chapter 18 covers it in detail) and checks the output and the exit status:

```rust,no_run
use std::process::Command;

// In a real integration test: Command::new(env!("CARGO_BIN_EXE_label_cli"))
let output = Command::new("label_cli").arg("Bay 7").output().expect("the program starts");
assert_eq!(output.status.code(), Some(0));
assert_eq!(String::from_utf8_lossy(&output.stdout), "tag:Bay 7\n");
```

### Read more

- [Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html) in the Cargo book, including the `package`, `dependencies` and `lints` tables
- [Features](https://doc.rust-lang.org/cargo/reference/features.html) in the Cargo book, especially "Optional dependencies" and "Feature unification"
- [Build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html) in the Cargo book
- [Environment variables Cargo sets for crates](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates) (`CARGO_BIN_EXE_<name>`) and [for build scripts](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts)
