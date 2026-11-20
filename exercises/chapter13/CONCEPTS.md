# Chapter 13 concepts — Everyday traits

[Chapter guide](README.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Read one section, then do its lesson. Every code block here was compiled and run with Rust 1.99.0, edition 2024.

## What you will be able to do

The standard library defines a small set of traits that almost every Rust program uses. This chapter calls them *utility traits*. They fall into three groups:

- **Lifetime of a value:** `Drop` (cleanup code), `Sized` (size known at compile time), `Clone` and `Copy` (duplication), `Default` (a starting value).
- **Views:** `Deref`, `DerefMut`, `AsRef`, `AsMut`, `Borrow`, `BorrowMut`. Each one lends a reference to something inside a value.
- **Conversions:** `From`, `Into`, `TryFrom`, `TryInto`, `ToOwned`, and the `Cow` type that is built on `ToOwned`.

After this chapter you will be able to:

- say exactly when a value is destroyed, also during a panic and for values that have no name;
- choose between `Copy` and `Clone` for your own types, and write functions that accept `str` and `[T]`;
- choose the correct view trait for a parameter, a map key, or a wrapper type;
- write conversions that cannot fail and conversions that can, without losing data silently;
- return borrowed data when nothing changed and owned data only when it is needed.

## Before you start

Ownership and moves (chapter 4), references and lifetimes (chapter 5), `panic!` and `Result` (chapter 7), structs and `RefCell` (chapter 9), enums and patterns (chapter 10), traits, generics and trait bounds (chapter 11). Lesson 09 passes closures to functions. You only need to know that a closure is a value you can call, like `work()`; chapter 14 explains closures fully.

## Time

These are estimates for reading the section, doing the lesson, and writing your notes.

| Lesson | Estimate |
| --- | --- |
| 01 · Destruction and resource lifetimes | 45 minutes |
| 09 · Drop order: unwinding and temporaries | 75 minutes |
| 02 · Sized, Clone, and Copy | 40 minutes |
| 03 · Deref, DerefMut, and Default | 40 minutes |
| 04 · AsRef and AsMut views | 30 minutes |
| 05 · Borrow and borrowed lookup | 40 minutes |
| 06 · Infallible and fallible conversions | 45 minutes |
| 07 · ToOwned and clone-on-write | 45 minutes |
| 08 · Capstone | 45 minutes |
| Review, code review, and drills | 2 hours |

## Study order

Lesson 09 was added after the others, so its number is out of order. It continues lesson 01, and this page puts it second. If it feels heavy, do it after lesson 07 instead.

## Lesson 01 · Destruction and resource lifetimes

**Assumed:** every value has exactly one owner, and a move transfers ownership (chapter 4).

**The model.** When the owner of a value goes away, Rust *drops* the value. Dropping has two steps. First, if the type implements the `Drop` trait, Rust calls its `drop(&mut self)` method. This method is called the *destructor*. Second, Rust drops every field of the value, one after another. You write only the first step. The compiler writes the second step and inserts all the calls.

The important question is *when*. These rules cover most code:

- A local variable is dropped when its block ends. Variables of the same block are dropped in **reverse** order of declaration.
- The fields of a struct are dropped in declaration order, first field first.
- A move does not drop anything. The new owner drops the value later, and the old variable is empty.
- Assigning a new value to a variable drops the old value.
- `drop(x)` moves `x` into a function with an empty body, so `x` is dropped at that line.

**Worked example.** A reservation gives its seat back when it ends. The seat list lets us observe each destructor call.

```rust
use std::cell::RefCell;

/// A seat that returns to the free list when the reservation ends.
struct Reservation<'a> {
    seat: u8,
    free: &'a RefCell<Vec<u8>>,
}

impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        self.free.borrow_mut().push(self.seat);
    }
}

let free = RefCell::new(Vec::new());
{
    let a = Reservation { seat: 1, free: &free };
    let b = Reservation { seat: 2, free: &free };
    let moved = a; // a move: seat 1 still has exactly one owner
    assert!(free.borrow().is_empty());
    drop(b); // ends `b` now
    assert_eq!(*free.borrow(), [2]);
    let _c = Reservation { seat: 3, free: &free };
} // `_c` is dropped first, then `moved`
assert_eq!(*free.borrow(), [2, 3, 1]);
```

```text
line                 owners alive     free list
let a = seat 1       a                []
let b = seat 2       a, b             []
let moved = a        moved, b         []         (a is empty: nothing to drop later)
drop(b)              moved            [2]
let _c = seat 3      moved, _c        [2]
}                    (none)           [2, 3, 1]  (_c first, then moved)
```

Seat 1 is returned once, although two variables held it. The destructor belongs to the value, not to a variable.

**Mistakes people really make**

*Calling the destructor yourself.* After `r.drop()` the variable `r` would still own the value, and the compiler would drop it a second time at the end of the block. So the call is not allowed:

```rust,compile_fail,E0040
struct Reservation {
    seat: u8,
}

impl Drop for Reservation {
    fn drop(&mut self) {
        println!("seat {} is free again", self.seat);
    }
}

fn main() {
    let r = Reservation { seat: 4 };
    r.drop();
}
```

```text
error[E0040]: explicit use of destructor method
  --> src/main.rs:13:7
   |
13 |     r.drop();
   |       ^^^^ explicit destructor calls not allowed
   |
help: consider using `drop` function
   |
13 -     r.drop();
13 +     drop(r);
```

The fix is `drop(r)`. It takes ownership, so `r` cannot be used or dropped again.

*Moving a field out of a type that implements `Drop`.* The destructor needs the whole value. If one field were gone, `drop(&mut self)` would see a value with a hole in it.

```rust,compile_fail,E0509
struct Upload {
    name: String,
    bytes: Vec<u8>,
}

impl Drop for Upload {
    fn drop(&mut self) {
        println!("closing {}", self.name);
    }
}

fn main() {
    let upload = Upload { name: String::from("a.txt"), bytes: vec![1, 2] };
    let bytes = upload.bytes;
    println!("{}", bytes.len());
}
```

```text
error[E0509]: cannot move out of type `Upload`, which implements the `Drop` trait
  --> src/main.rs:14:17
   |
14 |     let bytes = upload.bytes;
   |                 ^^^^^^^^^^^^
   |                 |
   |                 cannot move out of here
   |                 move occurs because `upload.bytes` has type `Vec<u8>`, which does not implement the `Copy` trait
   |
help: consider borrowing here
   |
14 |     let bytes = &upload.bytes;
   |                 +
help: consider cloning the value if the performance cost is acceptable
   |
14 |     let bytes = upload.bytes.clone();
   |                             ++++++++
```

Borrow the field (`&upload.bytes`), or swap a cheap empty value into its place with `std::mem::take(&mut upload.bytes)`. For that, `upload` must be declared `mut`.

*Writing `let _ = guard_value();`.* The pattern `_` does not bind anything, so the value is dropped at the end of that statement. A name that starts with an underscore, such as `_guard`, is a real variable and keeps the value alive until the block ends.

**When to use which**

| You want | Use |
| --- | --- |
| cleanup at the end of a block | nothing: let the variable go out of scope |
| cleanup earlier than the end of the block | `drop(value)`, or a smaller inner block |
| to implement `Drop` | only when your type owns something that no field releases by itself (a lock you took, a counter you raised, a temporary file) |
| to skip the destructor | `std::mem::forget(value)`: safe, but the resource is never released |

**Read more**

- [`Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html) and [`std::mem::drop`](https://doc.rust-lang.org/std/mem/fn.drop.html)
- [The Rust Reference: Destructors](https://doc.rust-lang.org/reference/destructors.html)
- [`std::mem::forget`](https://doc.rust-lang.org/std/mem/fn.forget.html) and [`std::mem::take`](https://doc.rust-lang.org/std/mem/fn.take.html)

## Lesson 09 · Drop order: unwinding and temporaries

**Assumed:** lesson 01, `panic!` and `catch_unwind` from chapter 7, and `RefCell` from chapter 9. A `RefCell` hands out a *guard* (`Ref` or `RefMut`) for each borrow, and it panics if you ask for a second guard that conflicts with a living one.

**Unwinding.** A panic does not stop the program at once. It *unwinds* the stack: it leaves each function call, starting with the innermost one, and drops the values of that call on the way. Inside one call the order is the usual one, reverse order of declaration. Only values that exist are dropped. A variable that was not yet created, or whose value was moved away, is skipped. Unwinding stops at a `catch_unwind` call or at the end of the thread. (A program built with `panic = "abort"` does not unwind at all: it stops at the panic and runs no destructors.)

This gives a reliable place for cleanup. Code after a call does not run when the call panics. A destructor does.

**Worked example.** The guard below removes half-finished work unless `commit` is called.

```rust
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

/// Removes everything added to `items` after `begin`, unless `commit` is called.
struct Rollback<'a> {
    items: &'a RefCell<Vec<&'static str>>,
    keep: usize,
    armed: bool,
}

impl<'a> Rollback<'a> {
    fn begin(items: &'a RefCell<Vec<&'static str>>) -> Self {
        let keep = items.borrow().len();
        Rollback { items, keep, armed: true }
    }
    fn commit(mut self) {
        self.armed = false;
    } // `self` is dropped here, with `armed` switched off
}

impl Drop for Rollback<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.items.borrow_mut().truncate(self.keep);
        }
    }
}

fn add_all(items: &RefCell<Vec<&'static str>>, new: &[&'static str]) {
    let rollback = Rollback::begin(items);
    for &item in new {
        assert!(!item.is_empty(), "empty item"); // can panic in the middle
        items.borrow_mut().push(item);
    }
    rollback.commit();
}

let items = RefCell::new(vec!["tea"]);
add_all(&items, &["milk", "rice"]);
assert_eq!(*items.borrow(), ["tea", "milk", "rice"]);

let result = panic::catch_unwind(AssertUnwindSafe(|| add_all(&items, &["salt", "", "oil"])));
assert!(result.is_err());
assert_eq!(*items.borrow(), ["tea", "milk", "rice"]); // "salt" was removed again
```

`catch_unwind` accepts only closures that are marked as safe to observe after a panic. A closure that uses a `RefCell` does not have this mark, because a panic could leave the data half-changed. `AssertUnwindSafe` is your statement that you have handled this. Here the guard is the reason the statement is true.

**Temporaries.** A *temporary* is a value that has no name, such as the guard in `cell.borrow().len()`. It must be dropped somewhere. The rules for edition 2024:

```text
where the temporary is created                when it is dropped
--------------------------------------------  ----------------------------------------
a statement, also `let x = <expression>;`     at the `;` of that statement
`let x = &<temporary>;`                       together with `x` (its life is extended)
`match <expression> { ... }`                  after the whole `match` statement
`while let P = <expression> { ... }`          after each turn of the loop body
`if let P = <expression> { A } else { B }`    after A, and before B starts
the last expression of a block or function    before the local variables of that block
```

```rust
use std::cell::RefCell;

let scores = RefCell::new(vec![3_usize, 5]);

// Two statements: the first guard is dropped at the first `;`.
let count = scores.borrow().len();
scores.borrow_mut().push(count);
assert_eq!(*scores.borrow(), [3, 5, 2]);

// The guard made in the condition is dropped before the `else` block starts.
if let Some(&big) = scores.borrow().iter().find(|&&score| score > 100) {
    println!("found {big}");
} else {
    scores.borrow_mut().push(100);
}
assert_eq!(*scores.borrow(), [3, 5, 2, 100]);
```

The last two rows of the table are new in edition 2024. In edition 2021 the `else` block above panics, and a guard in the last expression of a function lives longer than the function's local variables. Many older texts describe the 2021 behaviour.

**Mistakes people really make**

*Holding a guard across a `match`.* The guard made in the first line lives until the `match` ends, so the arm cannot borrow again.

```rust,should_panic
use std::cell::RefCell;

fn main() {
    let title = RefCell::new(String::from("draft"));
    match title.borrow().as_str() {
        "draft" => title.borrow_mut().push_str(" (unsaved)"),
        _ => {}
    }
}
```

```text
thread 'main' (122248537) panicked at src/main.rs:6:26:
RefCell already borrowed
```

The number in parentheses is the id of the thread. It is different on every run, so yours will not match.

Compute a plain value first, in its own statement: `let is_draft = *title.borrow() == "draft";`. Then test `is_draft`. The same trap exists for one-line calls such as `cell.borrow_mut().push(cell.borrow().len())`, and for lock guards in chapter 19, where the result is a deadlock instead of a panic.

*Borrowing from a temporary.* The `String` below has no owner, so it is dropped at the `;` while `first` still points into it.

```rust,compile_fail,E0716
fn main() {
    let first = String::from("alpha beta").split(' ').next();
    println!("{first:?}");
}
```

```text
error[E0716]: temporary value dropped while borrowed
 --> src/main.rs:2:17
  |
2 |     let first = String::from("alpha beta").split(' ').next();
  |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^                  - temporary value is freed at the end of this statement
  |                 |
  |                 creates a temporary value which is freed while still in use
3 |     println!("{first:?}");
  |                ----- borrow later used here
  |
help: consider using a `let` binding to create a longer lived value
  |
2 ~     let binding = String::from("alpha beta");
3 ~     let first = binding.split(' ').next();
```

Give the `String` a name in its own `let`, then borrow from that variable.

*Panicking inside a destructor.* If `drop` panics while another panic is already unwinding, the process stops immediately and no other destructor runs.

```rust,no_run
struct Loud;

impl Drop for Loud {
    fn drop(&mut self) {
        panic!("panic inside drop");
    }
}

fn main() {
    let _loud = Loud;
    panic!("first panic");
}
```

```text
thread 'main' (122248708) panicked at src/main.rs:11:5:
first panic

thread 'main' (122248708) panicked at src/main.rs:5:9:
panic inside drop

thread 'main' (122248708) panicked at /rustc/b940084d7eb6a299eb4bfeb8e34901bc051e7ac4/library/core/src/panicking.rs:233:5:
panic in a destructor during cleanup
thread caused non-unwinding panic. aborting.
```

Keep destructors simple. Do not call `unwrap` in them on anything that can fail.

**Read more**

- [The Rust Reference: temporary scopes](https://doc.rust-lang.org/reference/destructors.html#temporary-scopes) and [temporary lifetime extension](https://doc.rust-lang.org/reference/destructors.html#temporary-lifetime-extension)
- Edition Guide: [`if let` temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-if-let-scope.html) and [tail expression temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-tail-expr-scope.html)
- [`std::panic::catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html) and [`AssertUnwindSafe`](https://doc.rust-lang.org/std/panic/struct.AssertUnwindSafe.html)
- [The Rustonomicon: Unwinding](https://doc.rust-lang.org/nomicon/unwinding.html)

## Lesson 02 · Sized, Clone, and Copy

**Assumed:** generic functions and trait bounds (chapter 11); `String`, `Vec` and slices (chapter 3).

**`Sized`.** A type is *sized* when the compiler knows how many bytes one value needs. `u64`, `String` and `[u8; 4]` are sized. `str` and `[T]` are *unsized*: their length is known only at run time. You cannot hold an unsized value in a variable. You hold it behind a pointer such as `&str` or `Box<[T]>`, and that pointer also stores the length. For this reason it is two machine words wide and is called a *fat pointer*.

Every generic type parameter has a hidden bound `T: Sized`. Write `T: ?Sized` to remove it. The function can then use `T` only behind a reference or another pointer.

```rust
fn larger<'a, T: PartialOrd + ?Sized>(a: &'a T, b: &'a T) -> &'a T {
    if a >= b { a } else { b }
}

assert_eq!(larger("pear", "apple"), "pear"); // T = str
assert_eq!(larger(&[1, 9][..], &[1, 2, 3][..]), [1, 9]); // T = [i32]
assert_eq!(*larger(&2.5, &1.5), 2.5); // T = f64
```

**`Clone` and `Copy`.** Both make a second value. The difference is who asks and what it may cost.

- `Clone` is explicit. You call `.clone()`, and the method can run any code. For a `Vec<String>` it allocates a new buffer and clones each string.
- `Copy` is implicit. It tells the compiler that copying the bytes of the value gives a second valid value. Assignment and passing to a function then copy and leave the source usable. `Copy` has no method, so you cannot change what it does.

A type can be `Copy` only if every field is `Copy` and the type does not implement `Drop`.

```rust
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rgb(u8, u8, u8); // three bytes: a copy is trivial

#[derive(Debug, Clone, PartialEq)]
struct Palette {
    name: String, // owns heap memory, so Palette can be Clone but not Copy
    colors: Vec<Rgb>,
}

let red = Rgb(255, 0, 0);
let also_red = red; // implicit copy; `red` is still usable
assert_eq!(red, also_red);

let warm = Palette { name: "warm".into(), colors: vec![red] };
let mut second = warm.clone(); // explicit; allocates a new String and a new Vec
second.name.push_str(" (edited)");
assert_eq!(warm.name, "warm");

let shared = Rc::new(warm);
let other = Rc::clone(&shared); // copies one pointer and raises a counter
assert!(Rc::ptr_eq(&shared, &other));
assert_eq!(Rc::strong_count(&shared), 2);
```

```text
warm.clone()                           Rc::clone(&shared)

warm ──► "warm"   [Rgb]                shared ─┐
second ► "warm"   [Rgb]  (new copies)          ├─► count 2 │ Palette  (one value)
                                       other ──┘
```

So "clone" does not always mean "deep copy". It means whatever the type's `Clone` implementation does.

**Mistakes people really make**

*Passing `str` to a parameter that is silently `Sized`.*

```rust,compile_fail,E0277
fn describe<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

fn main() {
    println!("{}", describe("text"));
}
```

```text
error[E0277]: the size for values of type `str` cannot be known at compilation time
 --> src/main.rs:6:29
  |
6 |     println!("{}", describe("text"));
  |                    -------- ^^^^^^ doesn't have a size known at compile-time
  |                    |
  |                    required by a bound introduced by this call
  |
  = help: the trait `Sized` is not implemented for `str`
note: required by an implicit `Sized` bound in `describe`
 --> src/main.rs:1:13
  |
1 | fn describe<T: std::fmt::Debug>(value: &T) -> String {
  |             ^ required by the implicit `Sized` requirement on this type parameter in `describe`
help: consider relaxing the implicit `Sized` restriction
  |
1 | fn describe<T: std::fmt::Debug + ?Sized>(value: &T) -> String {
  |                                ++++++++
```

The last `help` shows the fix: add `?Sized` to the bound.

*Deriving `Copy` for a type with a field that owns heap memory.*

```rust,compile_fail,E0204
#[derive(Clone, Copy)]
struct Player {
    name: String,
    score: u32,
}

fn main() {}
```

```text
error[E0204]: the trait `Copy` cannot be implemented for this type
 --> src/main.rs:2:8
  |
1 | #[derive(Clone, Copy)]
  |                 ---- in this derive macro expansion
2 | struct Player {
  |        ^^^^^^
3 |     name: String,
  |     ------------ this field does not implement `Copy`
```

Remove `Copy` and call `.clone()` where you need a second value.

*Expecting `#[derive(Clone)]` on a generic type to work for every `T`.* The derive adds the bound `T: Clone`, even when no field needs it. An `Rc<T>` can be cloned for any `T`, but the derived code does not know that.

```rust,compile_fail,E0599
use std::rc::Rc;

struct Canvas; // not Clone

#[derive(Clone)]
struct Shared<T>(Rc<T>);

fn main() {
    let a = Shared(Rc::new(Canvas));
    let b = a.clone();
}
```

```text
error[E0599]: the method `clone` exists for struct `Shared<Canvas>`, but its trait bounds were not satisfied
  --> src/main.rs:10:15
   |
 3 | struct Canvas; // not Clone
   | ------------- doesn't satisfy `Canvas: Clone`
...
 6 | struct Shared<T>(Rc<T>);
   | ---------------- method `clone` not found for this struct because it doesn't satisfy `Shared<Canvas>: Clone`
...
10 |     let b = a.clone();
   |               ^^^^^ method cannot be called on `Shared<Canvas>` due to unsatisfied trait bounds
   |
note: trait bound `Canvas: Clone` was not satisfied
  --> src/main.rs:6:15
   |
 5 | #[derive(Clone)]
   |          ----- in this derive macro expansion
 6 | struct Shared<T>(Rc<T>);
   |               ^ type parameter would need to implement `Clone`
   = help: consider manually implementing the trait to avoid undesired bounds
help: consider annotating `Canvas` with `#[derive(Clone)]`
   |
 3 + #[derive(Clone)]
 4 | struct Canvas; // not Clone
```

Write the implementation by hand, without the extra bound:

```rust
use std::rc::Rc;
struct Canvas; // not Clone
struct Shared<T>(Rc<T>);

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(Rc::clone(&self.0))
    }
}

let a = Shared(Rc::new(Canvas));
let b = a.clone();
assert!(Rc::ptr_eq(&a.0, &b.0));
```

**When to use which.** Make a type `Copy` when it is small, holds only plain data, and you are sure it will stay that way: coordinates, colours, identifiers. `Copy` is a promise in your public interface, and removing it later breaks the code of your users. For everything else derive `Clone`. Add `?Sized` to a type parameter when the function only uses the value behind a reference.

**Read more**

- [`Sized`](https://doc.rust-lang.org/std/marker/trait.Sized.html) and [The Rust Reference: dynamically sized types](https://doc.rust-lang.org/reference/dynamically-sized-types.html)
- [`Clone`](https://doc.rust-lang.org/std/clone/trait.Clone.html) and [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html)

## Lesson 03 · Deref, DerefMut, and Default

**Assumed:** methods and `impl` blocks (chapter 9), associated types in traits (chapter 11).

**`Deref`.** The operator `*x` works on references. `Deref` makes it work on your own type too. The trait has one method, `deref(&self) -> &Self::Target`, and `*x` then means `*x.deref()`. The method returns a reference, so it lends the target. It never gives ownership away.

Two compiler features build on this trait:

- **Deref coercion.** Where the compiler expects `&Target` and finds `&YourType`, it inserts the `deref` call for you. This happens for function arguments, for `let` statements with a type, and for return values.
- **Method lookup.** For `value.method()` the compiler first looks for the method on the type of `value`. If it finds none, it follows `Deref` and looks on the target. A method of the wrapper therefore wins over a method of the target with the same name.

`DerefMut` adds `deref_mut(&mut self) -> &mut Self::Target`. It does the same for mutable access.

**Worked example.** The wrapper below keeps its numbers sorted. It implements `Deref` to a slice, so every reading method of slices works. It does *not* implement `DerefMut`, so no caller can change the order.

```rust
use std::ops::Deref;

/// A list that is always sorted. It lends its numbers as a read-only slice.
#[derive(Debug, Default)]
struct Sorted {
    items: Vec<i32>,
}

impl Sorted {
    fn insert(&mut self, value: i32) {
        let at = self.items.partition_point(|&item| item < value);
        self.items.insert(at, value);
    }
}

impl Deref for Sorted {
    type Target = [i32];
    fn deref(&self) -> &[i32] {
        self.items.as_slice()
    }
}

fn total(values: &[i32]) -> i32 {
    values.iter().sum()
}

let mut sorted = Sorted::default(); // derived Default: an empty Vec
for value in [5, 1, 3] {
    sorted.insert(value);
}
assert_eq!(sorted.len(), 3); // a slice method, found through Deref
assert_eq!(sorted.binary_search(&3), Ok(1)); // correct, because the order is guaranteed
assert_eq!(total(&sorted), 9); // &Sorted is coerced to &[i32]
assert_eq!(sorted[..2], [1, 3]); // indexing goes through Deref too
```

**`Default`.** `Default::default()` returns a sensible starting value for a type. Numbers give zero, `bool` gives `false`, `Option` gives `None`, and collections and `String` are empty. `#[derive(Default)]` uses the default of every field. For an enum you mark one variant.

```rust
#[derive(Debug, Default, PartialEq)]
enum Mode {
    #[default]
    Quiet,
    Verbose,
}

#[derive(Debug, Default, PartialEq)]
struct Options {
    mode: Mode,
    jobs: u8,
    tag: Option<String>,
}

assert_eq!(Options::default(), Options { mode: Mode::Quiet, jobs: 0, tag: None });

let missing: Option<Vec<u8>> = None;
assert!(missing.unwrap_or_default().is_empty());

let mut words = vec!["a", "b"];
let taken = std::mem::take(&mut words); // leaves Vec::default() in `words`
assert_eq!((taken.len(), words.len()), (2, 0));
```

A default is a meaningful value, not "all bytes zero". Write `impl Default` by hand when a field needs another start value. Do not implement it at all when no value is a good default.

**Mistakes people really make**

*Expecting deref coercion to satisfy a trait bound.* Coercion changes `&String` to `&str` only where the expected type is already known to be `&str`. For a generic parameter the compiler first chooses `T = String` and then checks the bound. It does not try other types.

```rust,compile_fail,E0277
trait Shout {
    fn shout(&self) -> String;
}

impl Shout for str {
    fn shout(&self) -> String {
        self.to_uppercase()
    }
}

fn announce<T: Shout + ?Sized>(text: &T) -> String {
    text.shout()
}

fn main() {
    let name = String::from("ada");
    println!("{}", announce(&name));
}
```

```text
error[E0277]: the trait bound `String: Shout` is not satisfied
  --> src/main.rs:17:29
   |
17 |     println!("{}", announce(&name));
   |                    -------- ^^^^^ the trait `Shout` is not implemented for `String`
   |                    |
   |                    required by a bound introduced by this call
   |
help: the trait `Shout` is implemented for `str`
  --> src/main.rs:5:1
   |
 5 | impl Shout for str {
   | ^^^^^^^^^^^^^^^^^^
note: required by a bound in `announce`
  --> src/main.rs:11:16
   |
11 | fn announce<T: Shout + ?Sized>(text: &T) -> String {
   |                ^^^^^ required by this bound in `announce`
```

Do the conversion yourself: `announce(name.as_str())` or `announce(&*name)`.

*Changing data through a type that has only `Deref`.* A shorter copy of the `Sorted` type shows it:

```rust,compile_fail,E0596
use std::ops::Deref;

struct Sorted(Vec<i32>);

impl Deref for Sorted {
    type Target = [i32];
    fn deref(&self) -> &[i32] {
        self.0.as_slice()
    }
}

fn main() {
    let sorted = Sorted(vec![1, 2, 3]);
    sorted.reverse();
}
```

```text
error[E0596]: cannot borrow data in dereference of `Sorted` as mutable
  --> src/main.rs:14:5
   |
14 |     sorted.reverse();
   |     ^^^^^^ cannot borrow as mutable
   |
   = help: trait `DerefMut` is required to modify through a dereference, but it is not implemented for `Sorted`
```

For `Sorted` this error is the intended protection. For a plain wrapper, implement `DerefMut`.

*Using `Deref` to imitate inheritance.* With `Deref`, every method of the target becomes callable on your type, now and in future versions of the target. If your type has rules that some of those methods can break, do not implement `Deref`. Write the few methods you want to offer.

**When to use which**

| Situation | Choice |
| --- | --- |
| a smart pointer, or a wrapper that should behave like its content | `Deref` (and `DerefMut` if edits are harmless) |
| a type with its own rules that can lend a view on request | `AsRef` (lesson 04) or a named method such as `as_slice()` |
| a type where an empty or zero value is useful | `Default`, usually derived |

**Read more**

- [`Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html) and [`DerefMut`](https://doc.rust-lang.org/std/ops/trait.DerefMut.html)
- [The Rust Reference: coercion sites](https://doc.rust-lang.org/reference/type-coercions.html#coercion-sites) and [method call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html)
- [`Default`](https://doc.rust-lang.org/std/default/trait.Default.html)

## Lesson 04 · AsRef and AsMut views

**Assumed:** generic functions with trait bounds (chapter 11).

**The model.** `AsRef<T>` has one method, `as_ref(&self) -> &T`. A type that implements it says: "I can lend you a `&T`, and it costs almost nothing." It is a *view*: no data is copied and no memory is allocated. `AsMut<T>` is the same for `&mut T`.

The trait is useful in parameters. A function that needs text can ask for `S: AsRef<str>`. Callers may then pass a `&str`, a `String`, a `&String`, or their own type, and they do not have to convert first. Inside, the function calls `as_ref()` once and works with a plain `&str`.

One type can implement `AsRef` for several targets. `String` implements `AsRef<str>`, `AsRef<[u8]>` and `AsRef<Path>`, among others. A reference `&X` implements `AsRef<T>` whenever `X` does, so passing a borrowed value works as well.

**Worked example**

```rust
/// Counts the words in anything that can lend a `&str`.
fn word_count<S: AsRef<str>>(text: S) -> usize {
    text.as_ref().split_whitespace().count()
}

/// Your own type can join in.
struct Note {
    body: String,
}

impl AsRef<str> for Note {
    fn as_ref(&self) -> &str {
        &self.body
    }
}

assert_eq!(word_count("two words"), 2); // &str
assert_eq!(word_count(String::from("one")), 1); // String, moved in

let kept = String::from("still mine");
assert_eq!(word_count(&kept), 2); // &String: only lent
assert_eq!(kept, "still mine");

assert_eq!(word_count(Note { body: "a b c".into() }), 3);
```

`AsMut` lets one function edit several kinds of storage in place:

```rust
fn zero_all<B: AsMut<[i32]>>(mut values: B) {
    for value in values.as_mut() {
        *value = 0;
    }
}

let mut list = vec![1, 2];
zero_all(&mut list);
assert_eq!(list, [0, 0]);

let mut array = [7; 3];
zero_all(&mut array[..2]); // only the borrowed part
assert_eq!(array, [0, 0, 7]);
```

**Cost of the generic version.** The compiler builds one copy of a generic function for every type it is called with. For a long function this makes the program larger. A common pattern keeps the generic part to one line:

```rust
pub fn word_count<S: AsRef<str>>(text: S) -> usize {
    fn inner(text: &str) -> usize {
        text.split_whitespace().count()
    }
    inner(text.as_ref())
}
assert_eq!(word_count("a b"), 2);
```

**Mistakes people really make**

*Calling `as_ref()` where nothing says which target you want.* The type has several `AsRef` implementations, and `.len()` exists on more than one target.

```rust,compile_fail,E0282
fn main() {
    let name = String::from("ada");
    let n = name.as_ref().len();
    println!("{n}");
}
```

```text
error[E0282]: type annotations needed
 --> src/main.rs:3:18
  |
3 |     let n = name.as_ref().len();
  |                  ^^^^^^
  |
help: try using a fully qualified path to specify the expected types
  |
3 -     let n = name.as_ref().len();
3 +     let n = <String as AsRef<T>>::as_ref(&name).len();
```

Name the target: `let text: &str = name.as_ref();`. In ordinary code, where you already hold a `String`, the direct methods `as_str()` and `as_bytes()` are clearer. `as_ref()` belongs in generic code.

*Returning a borrow from a parameter that was taken by value.* The function owns `text`, so `text` is dropped when the function returns. A reference into it cannot leave.

```rust,compile_fail,E0106
fn trimmed<S: AsRef<str>>(text: S) -> &str {
    text.as_ref().trim()
}

fn main() {
    println!("{}", trimmed(" a "));
}
```

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:1:39
  |
1 | fn trimmed<S: AsRef<str>>(text: S) -> &str {
  |                                       ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but there is no value for it to be borrowed from
help: consider using the `'static` lifetime, but this is uncommon unless you're returning a borrowed value from a `const` or a `static`
  |
1 | fn trimmed<S: AsRef<str>>(text: S) -> &'static str {
  |                                        +++++++
help: instead, you are more likely to want to change the argument to be borrowed...
  |
1 | fn trimmed<S: AsRef<str>>(text: &S) -> &str {
  |                                 +
help: ...or alternatively, you might want to return an owned value
  |
1 - fn trimmed<S: AsRef<str>>(text: S) -> &str {
1 + fn trimmed<S: AsRef<str>>(text: S) -> String {
```

Take `&S`, or simply `&str`, when the result borrows from the input. Take `S` by value only when the function finishes its work before it returns.

*Reading more into `AsRef` than it promises.* Two values can lend equal-looking views and still be different values with different rules for equality. `AsRef` says nothing about `==`, ordering, or hashing. Lesson 05 covers the trait that does.

**When to use which**

| Parameter type | Good for |
| --- | --- |
| `&str`, `&[T]`, `&Path` | most functions; the caller writes `&value` and the signature is easy to read |
| `impl AsRef<str>`, `impl AsRef<Path>` | public functions that are called with many input types, such as file-system helpers |
| `impl Into<String>` | functions that must *own* the text afterwards (lesson 06) |

**Read more**

- [`AsRef`](https://doc.rust-lang.org/std/convert/trait.AsRef.html) and [`AsMut`](https://doc.rust-lang.org/std/convert/trait.AsMut.html)
- [The `std::convert` module](https://doc.rust-lang.org/std/convert/index.html)

## Lesson 05 · Borrow and borrowed lookup

**Assumed:** `Eq` and `Hash` from chapter 12. This lesson uses only `insert`, `get` and `contains` of `HashMap` and `HashSet`; chapter 16 covers collections fully. For now it is enough to know that a hash map finds a key in two steps: it hashes the key to choose a place in its table, and then it compares with `==` there.

**The model.** `Borrow<T>` looks like `AsRef<T>`: one method, `borrow(&self) -> &T`. The difference is a promise that the compiler cannot check. The borrowed form must behave **exactly like the owner** under `==`, under ordering, and under hashing. If `a == b`, then `a.borrow() == b.borrow()`, and both forms give the same hash.

Collections depend on this promise. Look at the signature of `HashMap::get`:

```text
impl<K, V, S> HashMap<K, V, S> {
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
}
```

The map stores keys of type `K`, but you may search with any `Q` that `K` can be borrowed as. The map hashes your `&Q`, goes to the matching place in its table, and compares there. This finds the entry only if the `Q` form hashes and compares like the stored `K`. `String: Borrow<str>` and `Vec<T>: Borrow<[T]>` keep the promise, so a map with `String` keys can be searched with a `&str` and no `String` has to be built. Every type also implements `Borrow<Self>`, which is why searching with `&K` works.

**Worked example.** A key type of your own needs one small `impl` to allow the same.

```rust
use std::borrow::Borrow;
use std::collections::HashMap;

/// A part number. Equality and hashing are those of the text inside:
/// the derives pass both on to the `String` field.
#[derive(Debug, PartialEq, Eq, Hash)]
struct PartNo(String);

impl Borrow<str> for PartNo {
    fn borrow(&self) -> &str {
        &self.0
    }
}

let mut stock: HashMap<PartNo, u32> = HashMap::new();
stock.insert(PartNo("A-1".into()), 40);

assert_eq!(stock.get("A-1"), Some(&40)); // searched with &str; no PartNo is built
assert_eq!(stock.get(&PartNo("A-1".into())), Some(&40)); // &PartNo still works
assert!(!stock.contains_key("a-1"));
```

`BorrowMut<T>` adds `borrow_mut(&mut self) -> &mut T` with the same promise. It is rare in everyday code. Use it for standalone values only. Never change a key while it is inside a map or set, because the entry would then sit in the wrong place of the table.

**Mistakes people really make**

*Implementing `Borrow` for a type whose equality differs from the target's.* The tag below ignores letter case. Plain `str` does not. The code compiles, and the promise is broken.

```rust
use std::borrow::Borrow;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug)]
struct Tag(String);

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}
impl Eq for Tag {}
impl Hash for Tag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_ascii_lowercase().hash(state);
    }
}
impl Borrow<str> for Tag {
    // Wrong: `str` compares and hashes with letter case.
    fn borrow(&self) -> &str {
        &self.0
    }
}

fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

let (a, b) = (Tag("Rust".into()), Tag("RUST".into()));
assert_eq!(a, b); // equal as tags
let (a_text, b_text): (&str, &str) = (a.borrow(), b.borrow());
assert_ne!(a_text, b_text); // not equal as borrowed text
assert_ne!(hash_of(&a), hash_of(a_text)); // and the two forms hash differently
```

A `HashSet<Tag>` searched with `"Rust"` would look in the wrong place and usually report that the tag is missing. There is no compiler error and no panic. For such a type, implement `AsRef<str>` and leave `Borrow<str>` out.

*Searching a map of `&str` keys with a `&String`.*

```rust,compile_fail,E0277
use std::collections::HashMap;

fn main() {
    let prices: HashMap<&str, u32> = HashMap::from([("tea", 3)]);
    let wanted = String::from("tea");
    println!("{:?}", prices.get(&wanted));
}
```

```text
error[E0277]: the trait bound `&str: Borrow<String>` is not satisfied
    --> src/main.rs:6:33
     |
   6 |     println!("{:?}", prices.get(&wanted));
     |                             --- ^^^^^^^ the trait `Borrow<String>` is not implemented for `&str`
     |                             |
     |                             required by a bound introduced by this call
```

Read the bound from the key's side: the key type `&str` must be borrowable as your `Q`. An owner can be borrowed as its borrowed form, but not the other way round. Pass the borrowed form: `prices.get(wanted.as_str())`.

*Building an owned key only to search.* `map.get(&name.to_string())` compiles and gives the right answer, but it allocates a `String` for every search. `map.get(name)` does the same work without the allocation.

**When to use which**

| Trait | Promise | Typical use |
| --- | --- | --- |
| `AsRef<T>` | a cheap `&T` view | flexible function parameters |
| `Borrow<T>` | a cheap `&T` view that compares, orders and hashes like the owner | keys of maps and sets |
| `Deref` | the type behaves like its target | smart pointers and thin wrappers |

**Read more**

- [`Borrow`](https://doc.rust-lang.org/std/borrow/trait.Borrow.html) and [`BorrowMut`](https://doc.rust-lang.org/std/borrow/trait.BorrowMut.html)
- [`HashMap::get`](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.get)

## Lesson 06 · Infallible and fallible conversions

**Assumed:** `Result`, the `?` operator, and error types (chapter 7); range patterns (chapter 10).

**The model.** Four traits describe conversions that take a value and return another type.

- `From<A> for B` means: every `A` can become a `B`. The conversion cannot fail and keeps the meaning of the value. Example: `u8` to `u32`, `&str` to `String`.
- `TryFrom<A> for B` means: some `A` values can become a `B`. The method returns `Result<B, Self::Error>`. Example: `u32` to `u8`.
- `Into` and `TryInto` are the same conversions, written from the side of the source value: `a.into()` and `a.try_into()`.

You implement only `From` or only `TryFrom`. The standard library derives the rest:

```text
you write    impl From<A> for B
std adds     impl Into<B> for A            calls B::from
std adds     impl TryFrom<A> for B         Error = Infallible, never fails
std adds     impl TryInto<B> for A

you write    impl TryFrom<A> for B         with your own Error type
std adds     impl TryInto<B> for A
```

A consequence of the first block: do not write `From` and `TryFrom` for the same pair of types. Drill d05 shows what the compiler says.

**Worked example.** A percentage holds a number from 0 to 100. Creating one from an `i32` can fail, so that direction is `TryFrom`. Turning one into a fraction always works, so that direction is `From`.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
struct Percent(u8); // rule: the number is 0..=100

#[derive(Debug, PartialEq)]
enum PercentError {
    Negative,
    TooLarge,
}

impl TryFrom<i32> for Percent {
    type Error = PercentError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            ..0 => Err(PercentError::Negative),
            0..=100 => Ok(Percent(value as u8)), // the pattern proves that it fits
            _ => Err(PercentError::TooLarge),
        }
    }
}

impl From<Percent> for f64 {
    fn from(percent: Percent) -> f64 {
        f64::from(percent.0) / 100.0
    }
}

/// Accepts anything that converts into f64 without failing.
fn scale(amount: f64, factor: impl Into<f64>) -> f64 {
    amount * factor.into()
}

assert_eq!(Percent::try_from(40), Ok(Percent(40)));
assert_eq!(Percent::try_from(-1), Err(PercentError::Negative));

let parsed: Result<Percent, _> = 250.try_into(); // TryInto came for free
assert_eq!(parsed, Err(PercentError::TooLarge));

assert_eq!(scale(80.0, Percent(25)), 20.0); // From<Percent> for f64 gave Into<f64>
assert_eq!(scale(80.0, 0.5_f32), 40.0);
```

The function `scale` shows the usual division of work: **implement `From`, ask for `Into`** in a bound. A bound on `Into` accepts every type that has a `From` implementation.

The `?` operator uses the same trait: when a function returns `Result<_, E>` and an inner call fails with another error type, `?` converts it with `E::from`.

**Mistakes people really make**

*Asking for a conversion that can lose data.* The standard library offers `From` between number types only where every value fits.

```rust,compile_fail,E0277
fn main() {
    let big: i64 = 70_000;
    let small: i32 = big.into();
    println!("{small}");
}
```

```text
error[E0277]: the trait bound `i32: From<i64>` is not satisfied
 --> src/main.rs:3:26
  |
3 |     let small: i32 = big.into();
  |                          ^^^^ the trait `From<i64>` is not implemented for `i32`
  |
  = help: `i32` implements trait `From<T>`:
            From<bool>
            From<i16>
            From<i8>
            From<u16>
            From<u8>
  = note: required for `i64` to implement `Into<i32>`
```

Use `i32::try_from(big)` and handle the `Err`. Do not silence the error with `big as i32`. The `as` operator never fails. It cuts off the high bits of an integer that does not fit, and you get a wrong number without any warning.

*Implementing a conversion between two types that are not yours.*

```rust,compile_fail,E0117
impl From<u16> for String {
    fn from(n: u16) -> String {
        n.to_string()
    }
}

fn main() {}
```

```text
error[E0117]: only traits defined in the current crate can be implemented for types defined outside of the crate
 --> src/main.rs:1:1
  |
1 | impl From<u16> for String {
  | ^^^^^---------^^^^^------
  |      |             |
  |      |             `String` is not defined in the current crate
  |      `u16` is not defined in the current crate
  |
  = note: impl doesn't have any local type before any uncovered type parameters
  = note: for more information see https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules
  = note: define and implement a trait or new type instead
```

A crate may implement a trait only if the trait or one of the types belongs to that crate. This is called the *orphan rule*. Wrap one side in a type of your own (as `Percent` wraps `u8`), or write an ordinary function.

*Hiding a failure inside `From`.* A `from` body that calls `unwrap`, or that quietly replaces a bad value with another one, breaks the meaning of the trait. Callers read `.into()` as "this always works and nothing is lost". If the input can be wrong, the trait is `TryFrom`.

**When to use which**

| Conversion | Tool |
| --- | --- |
| always succeeds, nothing lost | `From` (callers get `Into`) |
| can fail | `TryFrom` with an error type that says why |
| parse from text | `FromStr`, so that `text.parse::<T>()` works (chapter 17) |
| number to number, and you have decided that cutting or rounding is correct | `as`, with a comment |
| the meaning is not obvious from the two types | a named method such as `from_millis` |

**Read more**

- [`From`](https://doc.rust-lang.org/std/convert/trait.From.html), [`Into`](https://doc.rust-lang.org/std/convert/trait.Into.html) and [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)
- [The Rust Reference: orphan rules](https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules)
- [The Rust Reference: type cast expressions](https://doc.rust-lang.org/reference/expressions/operator-expr.html#type-cast-expressions)

## Lesson 07 · ToOwned and clone-on-write

**Assumed:** `Clone` (lesson 02), `Deref` (lesson 03), enums with data (chapter 10).

**`ToOwned`.** `Clone` turns a `&T` into a `T`. That cannot work for `&str`, because a bare `str` is unsized and cannot be returned. `ToOwned` solves this with an associated type named `Owned`: the method `to_owned(&self)` returns `Self::Owned`. For `str` that type is `String`, for `[T]` it is `Vec<T>`, and for `Path` it is `PathBuf`. For every type that implements `Clone`, `Owned` is the type itself and `to_owned()` is the same as `clone()`.

**`Cow`.** Many functions return their input unchanged most of the time and a changed version only sometimes. If such a function returns `String`, it must allocate on every call. `Cow` ("clone on write") is an enum that can hold either form:

```text
enum Cow<'a, B: ToOwned + ?Sized> {
    Borrowed(&'a B),          for B = str:  Borrowed(&'a str)   points at the input
    Owned(B::Owned),                        Owned(String)       owns new text
}
```

`Cow<'a, B>` implements `Deref<Target = B>`, so readers call the methods of `B` and do not care which variant they hold. Two methods give you owned data:

- `to_mut()` returns `&mut B::Owned`. If the value is borrowed, it first clones the data and switches to `Owned`. If it is already owned, it changes nothing.
- `into_owned()` consumes the `Cow` and returns `B::Owned`. It clones only if the value was borrowed.

**Worked example.** The function limits every number to a maximum. Most inputs need no change.

```rust
use std::borrow::Cow;

/// Limits every value to `max`. Input that is already fine is returned borrowed.
fn clamp_all(values: &[i32], max: i32) -> Cow<'_, [i32]> {
    let mut out = Cow::Borrowed(values);
    for index in 0..values.len() {
        if values[index] > max {
            // The first call copies the slice into a new Vec. Later calls reuse that Vec.
            out.to_mut()[index] = max;
        }
    }
    out
}

let fine = [1, 2, 3];
let result = clamp_all(&fine, 10);
assert!(matches!(result, Cow::Borrowed(_)));
assert_eq!(result.as_ptr(), fine.as_ptr()); // the same memory: nothing was copied

let result = clamp_all(&[1, 50, 3, 99], 10);
assert!(matches!(result, Cow::Owned(_)));
assert_eq!(*result, [1, 10, 3, 10]);
assert_eq!(result.iter().sum::<i32>(), 24); // reading goes through Deref for both variants

let numbers: Vec<i32> = result.into_owned(); // already owned: no second copy
assert_eq!(numbers.len(), 4);
```

**Mistakes people really make**

*Returning the two forms without wrapping them.* The two branches have different types, and neither one is a `Cow`.

```rust,compile_fail,E0308
use std::borrow::Cow;

fn squeeze(text: &str) -> Cow<'_, str> {
    if text.contains("  ") {
        text.replace("  ", " ")
    } else {
        text
    }
}

fn main() {
    println!("{}", squeeze("a  b"));
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:5:9
  |
3 | fn squeeze(text: &str) -> Cow<'_, str> {
  |                           ------------ expected `Cow<'_, str>` because of return type
4 |     if text.contains("  ") {
5 |         text.replace("  ", " ")
  |         ^^^^^^^^^^^^^^^^^^^^^^^ expected `Cow<'_, str>`, found `String`
  |
  = note: expected enum `Cow<'_, str>`
           found struct `String`
help: call `Into::into` on this expression to convert `String` into `Cow<'_, str>`
  |
5 |         text.replace("  ", " ").into()
  |                                +++++++
```

Write `Cow::Owned(...)` and `Cow::Borrowed(...)`. You can also write `.into()` in both branches, because `Cow<str>` implements `From<String>` and `From<&str>`. The explicit names show the reader where the allocation happens.

*Calling `to_owned()` on a reference to a reference.* In the closure below `word` has type `&&str`. The value behind the outer reference is a `&str`, which is `Clone`, so `to_owned()` returns a `&str` again and not a `String`.

```rust,compile_fail,E0277
fn main() {
    let words: &[&str] = &["a", "b"];
    let owned: Vec<String> = words.iter().map(|word| word.to_owned()).collect();
    println!("{owned:?}");
}
```

```text
error[E0277]: a value of type `Vec<String>` cannot be built from an iterator over elements of type `&str`
    --> src/main.rs:3:71
     |
   3 |     let owned: Vec<String> = words.iter().map(|word| word.to_owned()).collect();
     |                                                                       ^^^^^^^ value of type `Vec<String>` cannot be built from `std::iter::Iterator<Item=&str>`
     |
note: the method call chain might not have had the expected associated types
    --> src/main.rs:3:43
     |
   2 |     let words: &[&str] = &["a", "b"];
     |                          ----------- this expression has type `&[&str; 2]`
   3 |     let owned: Vec<String> = words.iter().map(|word| word.to_owned()).collect();
     |                                    ------ ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `Iterator::Item` changed to `&str` here
     |                                    |
     |                                    `Iterator::Item` is `&&str` here
```

Remove one level first, `(*word).to_owned()`, or ask for the target by name: `word.to_string()`.

*Calling `to_mut()` before you know that a change is needed.* `to_mut()` clones borrowed data at once, even if you then write nothing. A function that always calls it allocates on every call, and the `Cow` in its signature only makes it harder to use. Test first, then call `to_mut()` inside the branch that writes.

**When to use which**

| The function | Return type |
| --- | --- |
| never changes the data | `&str` or `&[T]` |
| always produces new data | `String` or `Vec<T>` |
| usually returns the input and sometimes new data | `Cow<'_, str>` or `Cow<'_, [T]>` |

Use `Cow` when the unchanged case is common and the data can be large. For short strings in code that runs rarely, a plain `String` is simpler and good enough.

**Read more**

- [`ToOwned`](https://doc.rust-lang.org/std/borrow/trait.ToOwned.html)
- [`Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html) and [`Cow::to_mut`](https://doc.rust-lang.org/std/borrow/enum.Cow.html#method.to_mut)

## Lesson 08 · Capstone: validated names with cheap views

**Assumed:** lessons 04, 06 and 07.

The capstone has no new language feature. It asks you to design one small type, `Name`, with the traits of this chapter. The skill is choosing which traits fit and which do not. Before you write code, answer three questions on paper.

**1. Where is the border between unchecked and checked data?** A `Name` must never be blank. So there must be exactly one way to create a `Name` from text, and that way must be able to say no. Lesson 06 tells you which conversion trait can say no. Everything behind this border may trust that the text is valid, and no other function has to check again.

**2. Which operations only read?** Reading the text of a name changes nothing and needs no new memory. Lesson 04 gives you the trait for a cheap view. A test compares pointers to check that your view is the stored text and not a copy.

**3. Which operation creates new text only sometimes?** One function adds a tag to the name, but an empty tag adds nothing. Lesson 07 gives you a return type that can borrow in one case and own in the other. A test counts allocations for the empty tag.

Also plan the cost of validation. Cutting whitespace from the ends of a `&str` gives a shorter `&str` that points into the same text. No allocation is needed until you store the result. The tests allow one allocation for a successful conversion.

Finally, decide which traits `Name` should **not** have, and write the reason in your notes:

- `Default`: what would the default name be?
- `From<&str>` or `From<String>`: can every text become a name?
- `DerefMut` to `String`: what could a caller do with a `&mut String`?
- `Borrow<str>`: this one is acceptable only as long as `Name` compares and hashes exactly like its text (lesson 05). Check how `Name` gets its `==` before you decide.

The EXPLAIN prompt of the lesson asks for this list. A type that offers few, correct traits is easier to use than one that offers every trait.

**Read more**

- [The `std::convert` module](https://doc.rust-lang.org/std/convert/index.html), which compares `AsRef`, `From` and `TryFrom`
- [`str::trim`](https://doc.rust-lang.org/std/primitive.str.html#method.trim)
