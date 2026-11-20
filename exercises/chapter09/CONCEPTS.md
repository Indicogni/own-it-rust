# Chapter 9 concepts — Structs and methods

[Chapter guide](README.md) · [Hints](HINTS.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every code block in this file was compiled and
run with the course toolchain (Rust 1.99.0, edition 2024), and every compiler
message was copied from that compiler. Long messages are shortened: the line
with the file path and the notes at the end are left out, and a line with `...`
marks a cut in the middle.

## What you will be able to do

After this chapter you can:

- define a struct with named fields, with positional fields, or with no fields,
  and say which form fits a problem;
- build one struct value from another without copying heap data, and say which
  fields were moved;
- write methods and choose between `self`, `&self` and `&mut self`;
- protect a rule about your data (an *invariant*) with a constructor and
  private fields;
- make a struct generic over a type, a lifetime and a constant;
- let the compiler write `Debug`, `Clone`, `PartialEq` and similar traits, and
  know when the derived version is wrong for your type;
- change a value through a shared reference with `Cell` and `RefCell`, and
  explain the run-time errors that `RefCell` can produce;
- link values with `Rc` and `Weak` so that a tree is freed when its last owner
  goes away.

## Before you start

Chapter 4 (moves, `Copy`, `Clone`, `Box`, `Rc` and
`Arc`), Chapter 5 (references, lifetimes on functions, structs that borrow),
Chapter 7 (`Option` and `Result`) and Chapter 8 (modules and `pub`). Traits are
taught in Chapter 11. Here you only need to know that a trait is a named set of
methods that a type can implement, such as `Clone` with its method `clone`.

## Time

These are estimates for a learner who has finished Chapters 3 to 8.
They include reading the section, the predictions, the code, and your notes.

| Lesson | Estimate |
| --- | --- |
| 01 · Named structs and update syntax | 45 minutes |
| 02 · Tuple structs, unit structs and layout | 50 minutes |
| 03 · Methods and associated items | 60 minutes |
| 04 · Smart-pointer self receivers | 45 minutes |
| 05 · Generic structs and impl blocks | 45 minutes |
| 06 · Lifetimes and const parameters | 60 minutes |
| 07 · Deriving common traits | 50 minutes |
| 08 · Cell and RefCell | 75 minutes |
| 10 · Weak and parent links | 90 minutes |
| 09 · Capstone | 90 minutes |
| R01 review, C01 code review, drills | 2 to 3 hours |

Lesson 10 was added after the capstone got its number. Do it before the
capstone, as the table shows.

## Lesson 01 · Named structs and update syntax

**Assumed:** moves and `Copy` types (Chapter 4), `String` and its heap buffer.

A **struct** is a type that you define by listing its parts. Each part is a
**field** with a name and a type. A struct value owns all of its fields: the
fields are stored together, they move together, and they are dropped together.

```rust
struct Recipe {
    title: String,
    author: String,
    servings: u8,
}

fn main() {
    let title = String::from("Lentil soup");
    let soup = Recipe {
        servings: 4, // fields may come in any order
        title,       // shorthand for `title: title`
        author: String::from("Mina"),
    };
    assert_eq!(soup.title, "Lentil soup");
    assert_eq!(soup.servings, 4);
}
```

The expression `Recipe { ... }` is a **struct expression**. Three rules:

- You must give every field. There are no default values unless you ask for
  them (lesson 07).
- The order in the expression is free. The order in the *definition* is the
  one that counts: fields are dropped in the order of the definition.
- When a variable has the same name as a field, `title` alone means
  `title: title`. This is **field init shorthand**.

You read and write a field with a dot: `soup.servings`. To write, the whole
variable must be `mut`. Rust has no way to mark one field as mutable.

### Update syntax

Often you want a new value that differs from an old one in one or two fields.
**Struct update syntax** does that: list the fields you want to set, then write
`..` and the value that supplies the rest.

```rust
struct Recipe {
    title: String,
    author: String,
    servings: u8,
}

fn main() {
    let soup = Recipe {
        title: String::from("Lentil soup"),
        author: String::from("Mina"),
        servings: 4,
    };
    let text_address = soup.title.as_ptr();

    let party = Recipe { servings: 12, ..soup };

    assert_eq!(party.title.as_ptr(), text_address); // same text, new owner
    assert_eq!(party.servings, 12);
    assert_eq!(soup.servings, 4); // this field of `soup` can still be read
}
```

`..soup` is not a clone. For each field you did not list, Rust does what a
plain assignment would do: it **moves** the field, or copies it if the type is
`Copy`. Moving a `String` copies three machine words (pointer, capacity,
length). The text on the heap stays where it is.

```text
before                             after `Recipe { servings: 12, ..soup }`

soup                               soup (partly moved)
+----------+                       +----------+
| title   -+--> "Lentil soup"      | title    |   moved out
| author  -+--> "Mina"             | author   |   moved out
| servings |  4                    | servings |   4, still readable
+----------+                       +----------+
                                   party
                                   +----------+
                                   | title   -+--> "Lentil soup"  (same buffer)
                                   | author  -+--> "Mina"         (same buffer)
                                   | servings |  12
                                   +----------+
```

After the update `soup` is **partly moved**. You cannot use `soup` as a whole
any more, and you cannot use the fields that moved. The fields that did not
move are still owned by `soup`. You may use them, and they are dropped when
`soup` goes out of scope.

A pattern can also take a struct apart. `let Recipe { title, servings, .. } =
soup;` moves `title` out, copies `servings`, and ignores the rest.

### Mistakes people make

**Using a field that `..` moved.**

```rust,compile_fail,E0382
struct Recipe {
    title: String,
    author: String,
    servings: u8,
}

fn main() {
    let soup = Recipe {
        title: String::from("Lentil soup"),
        author: String::from("Mina"),
        servings: 4,
    };
    let party = Recipe { servings: 12, ..soup };
    println!("{} by {}", soup.title, party.author);
}
```

```text
error[E0382]: borrow of moved value: `soup.title`
   |
13 |     let party = Recipe { servings: 12, ..soup };
   |                 ------------------------------- value moved here
14 |     println!("{} by {}", soup.title, party.author);
   |                          ^^^^^^^^^^ value borrowed here after move
   |
   = note: move occurs because `soup.title` has type `String`, which does not implement the `Copy` trait
```

Fix it in one of three ways: read from `party.title`; use `soup.title` before
the update; or, if both values really need the text, give the new value its own
copy with `title: soup.title.clone()` and let `..soup` supply only the rest.

**Leaving a field out.**

```text
error[E0063]: missing field `author` in initializer of `Recipe`
  |
7 |     let soup = Recipe {
  |                ^^^^^^ missing `author`
```

Give the field, or end the expression with `..other_value`.

**Assigning to a field of a variable that is not `mut`.**

```text
error[E0594]: cannot assign to `soup.servings`, as `soup` is not declared as mutable
   |
10 |     soup.servings = 6;
   |     ^^^^^^^^^^^^^^^^^ cannot assign
   |
help: consider changing this to be mutable
   |
 6 |     let mut soup = Recipe {
   |         +++
```

### Read more

- [Struct expressions](https://doc.rust-lang.org/reference/expressions/struct-expr.html) in the Rust Reference, including update syntax.
- [Structs](https://doc.rust-lang.org/reference/items/structs.html) in the Rust Reference.
- [Destructors](https://doc.rust-lang.org/reference/destructors.html): the order in which fields and variables are dropped.

## Lesson 02 · Tuple structs, unit structs and layout

**Assumed:** lesson 01; tuples (Chapter 3).

Rust has two more forms of struct.

A **tuple struct** has fields without names. You reach them by position: `.0`,
`.1`, and so on. A **unit struct** has no fields at all.

```rust
struct Rgb(u8, u8, u8);
struct Seconds(u64);
struct Millis(u64);
struct Muted; // a unit struct

fn to_millis(duration: Seconds) -> Millis {
    Millis(duration.0 * 1000)
}

fn main() {
    let teal = Rgb(0, 128, 128);
    assert_eq!(teal.1, 128);

    let Millis(raw) = to_millis(Seconds(3)); // a pattern opens the wrapper
    assert_eq!(raw, 3000);

    let _marker = Muted; // the only value of this type
}
```

### Types are told apart by name

`Seconds` and `Millis` both hold one `u64`, but they are different types. Rust
compares struct types by their **name**, never by their content. This is
called **nominal typing**. It is the reason to write a tuple struct with one
field, a **newtype**: the compiler now stops you from passing seconds where
milliseconds are expected. The wrapper costs nothing at run time: a newtype is
as big as its one field. (The attribute `#[repr(transparent)]`, met in
Chapter 23, turns that observation into a guarantee.)

A unit struct is useful when the type itself carries the meaning. A value of
it has nothing to store, so it has size zero. Such a type is a **zero-sized
type**. Its values take no memory, whether they sit in a variable or in a
collection.

### Size, alignment and padding

Every type has a **size** in bytes and an **alignment**. An alignment of 4
means that a value of the type must start at an address that is a multiple
of 4. For the integer types, the alignment is usually the size: 1 for `u8`, 2
for `u16`, 4 for `u32`.

For a struct, three rules follow:

1. the alignment of the struct is the largest alignment among its fields;
2. each field starts at a multiple of its own alignment;
3. the size of the struct is a multiple of the struct's alignment.

To satisfy rules 2 and 3 the compiler inserts unused bytes, called
**padding**. With `#[repr(C)]` the fields keep the order you wrote:

```rust
#[repr(C)]
struct InOrder {
    a: u8,
    b: u16,
    c: u8,
}

struct Free {
    a: u8,
    b: u16,
    c: u8,
}

fn main() {
    assert_eq!(align_of::<InOrder>(), 2);
    assert_eq!(size_of::<InOrder>(), 6);
    assert!(size_of::<Free>() <= size_of::<InOrder>());
}
```

```text
#[repr(C)] struct InOrder

offset   0     1     2     3     4     5
       +-----+-----+-----------+-----+-----+
       |  a  | pad |     b     |  c  | pad |     size 6, alignment 2
       +-----+-----+-----------+-----+-----+
```

`b` needs an even offset, so one padding byte follows `a`. The total must be
a multiple of 2, so one more follows `c`.

Without an attribute a struct has the default layout, often written
`repr(Rust)`. Here the compiler may put the fields in any order, and it
usually picks one with less padding (for `Free`, typically `b`, `a`, `c` in 4
bytes). The Rust Reference promises almost nothing about this layout. Treat a
number you measure with `size_of` as an observation about one compiler, not as
a guarantee. Use `repr(C)` when the exact layout is part of a contract, for
example when C code or a file format reads the bytes. It is not a way to make
code faster.

### Mistakes people make

**Passing one newtype where another is expected.**

```rust,compile_fail,E0308
struct Seconds(u64);
struct Millis(u64);

fn sleep_for(duration: Millis) -> u64 {
    duration.0
}

fn main() {
    let pause = Seconds(2);
    sleep_for(pause);
}
```

```text
error[E0308]: mismatched types
   |
10 |     sleep_for(pause);
   |     --------- ^^^^^ expected `Millis`, found `Seconds`
   |     |
   |     arguments to this function are incorrect
```

This error is the newtype doing its job. Convert on purpose, with a function
such as `to_millis`.

**Expecting the wrapper to behave like the value inside.** A newtype starts
with no operations at all.

```text
error[E0369]: cannot add `Seconds` to `Seconds`
   |
 3 |     let total = Seconds(30) + Seconds(12);
   |                 ----------- ^ ----------- Seconds
   |                 |
   |                 Seconds
   |
note: an implementation of `Add` might be missing for `Seconds`
```

Work on the inner values (`Seconds(a.0 + b.0)`), or give the type the operator
(Chapter 12).

**Building a tuple struct whose field is private.** Like named fields,
positional fields are private to their module unless you write `pub`.

```text
error[E0603]: tuple struct constructor `Seconds` is private
  |
2 |     pub struct Seconds(u64);
  |                        --- a constructor is private if any of the fields is private
...
5 |     let wait = clock::Seconds(30);
  |                       ^^^^^^^ private tuple struct constructor
```

Write `pub struct Seconds(pub u64);`, or keep the field private and offer a
constructor function. The second choice lets the module check the value.

### When to use which

| Form | Use it when |
| --- | --- |
| Named fields | Each part has its own meaning. This is the normal choice. |
| Tuple struct | You wrap one value to get a distinct type (a newtype), or the positions are obvious, as in `Rgb(r, g, b)`. |
| Unit struct | The type is a marker and has no data. |
| Plain tuple | The group needs no name at all, for example two return values. |

### Read more

- [Type layout](https://doc.rust-lang.org/reference/type-layout.html) in the Rust Reference: size, alignment and the `repr` attributes.
- [`std::mem::size_of`](https://doc.rust-lang.org/std/mem/fn.size_of.html) and [`std::mem::align_of`](https://doc.rust-lang.org/std/mem/fn.align_of.html).
- [repr(Rust)](https://doc.rust-lang.org/nomicon/repr-rust.html) in the Rustonomicon: why the default layout is free to reorder fields.

## Lesson 03 · Methods and associated items

**Assumed:** lessons 01 and 02; `&` and `&mut` references (Chapter 5); `Option`.

Functions that belong to a type go in an **`impl` block**. Inside it, `Self`
is another name for the type.

```rust
#[derive(Debug, PartialEq)]
struct Rect {
    width: u32,
    height: u32,
}

impl Rect {
    /// An associated constant: one value for the whole type.
    const UNIT: Rect = Rect { width: 1, height: 1 };

    /// An associated function. It has no `self`, so you call it on the type.
    fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        Some(Self { width, height })
    }

    /// Reads the value.
    fn area(&self) -> u32 {
        self.width * self.height
    }

    /// Changes the value in place.
    fn widen(&mut self, by: u32) {
        self.width += by;
    }

    /// Uses the value up.
    fn into_sides(self) -> (u32, u32) {
        (self.width, self.height)
    }
}

fn main() {
    let mut r = Rect::new(3, 4).expect("both sides are positive");
    assert_eq!(r.area(), 12);
    r.widen(2);
    assert_eq!(r.into_sides(), (5, 4)); // `r` is moved here
    assert_eq!(Rect::UNIT.area(), 1);
    assert!(Rect::new(0, 9).is_none());
}
```

Everything inside an `impl` block is an **associated item** of the type. A
function whose first parameter is called `self` is a **method**, and you call
it with a dot. The form of `self` is the **receiver**:

| You write | It means | The call `r.m()` passes | Use it to |
| --- | --- | --- | --- |
| `&self` | `self: &Self` | `&r` | read |
| `&mut self` | `self: &mut Self` | `&mut r` | change in place |
| `self` | `self: Self` | `r` itself (a move) | consume or convert |

A function without `self`, such as `new`, is called through the type:
`Rect::new(3, 4)`. A constant is reached the same way: `Rect::UNIT`. You may
write several `impl` blocks for one type. They all add to the same type.

### How a method call finds its method

When you write `r.area()`, you do not write `&r`. The compiler adds the
reference for you. For a call `x.m()` where `x` has type `T`, it tries
receivers in this order and takes the first that fits:

1. a method `m` that takes `T` by value;
2. a method that takes `&T`;
3. a method that takes `&mut T`;
4. if `T` can be dereferenced (a `Box<U>`, a `&U`, ...), the same three steps
   again for `U`.

At each single step, a method written in an `impl Type` block (an **inherent**
method) is preferred over a method that comes from a trait. Notice that this
preference applies within one step only. A method found at step 1 wins over
any method that would be found at step 2, whoever defines it.

### Invariants

An **invariant** is a statement that is true for every value of a type. For
`Rect` it is "both sides are greater than zero". Three things together keep
it true:

- the fields are private to the module (Chapter 8), so other modules cannot
  write a struct expression or assign to a field;
- every function that creates a value checks the rule, like `new`;
- every `&mut self` method keeps the rule. A method that can refuse a change
  must check first and write afterwards, so that a refused call changes
  nothing.

Code that receives a `Rect` can then rely on the rule without checking again.

### Mistakes people make

**Using a value after a `self` method consumed it.**

```rust,compile_fail,E0382
struct Rect {
    width: u32,
    height: u32,
}

impl Rect {
    fn into_sides(self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn area(&self) -> u32 {
        self.width * self.height
    }
}

fn main() {
    let r = Rect { width: 3, height: 4 };
    let sides = r.into_sides();
    println!("{:?} {}", sides, r.area());
}
```

```text
error[E0382]: borrow of moved value: `r`
   |
16 |     let r = Rect { width: 3, height: 4 };
   |         - move occurs because `r` has type `Rect`, which does not implement the `Copy` trait
17 |     let sides = r.into_sides();
   |                   ------------ `r` moved due to this method call
18 |     println!("{:?} {}", sides, r.area());
   |                                ^ value borrowed here after move
   |
note: `Rect::into_sides` takes ownership of the receiver `self`, which moves `r`
```

Call the reading methods first, or change the method to `&self` if it does not
need to own the value.

**Forgetting `self.` inside a method.** Fields are not in scope by their bare
names.

```text
error[E0425]: cannot find value `width` in this scope
  |
7 |         width * height
  |         ^^^^^
  |
help: you might have meant to use the available field
  |
7 |         self.width * height
  |         +++++
```

### Read more

- [Implementations](https://doc.rust-lang.org/reference/items/implementations.html) and [associated items](https://doc.rust-lang.org/reference/items/associated-items.html) in the Rust Reference.
- [Method call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html): the exact lookup order.

## Lesson 04 · Smart-pointer self receivers

**Assumed:** lesson 03; `Box`, `Rc`, `Arc` and their counts (Chapter 4).

The receiver of a method does not have to be `Self`, `&Self` or `&mut Self`.
It can also be a smart pointer to `Self`:

```text
fn m(self: Box<Self>)     the method takes a Box that owns the value
fn m(self: Rc<Self>)      the method takes one Rc owner
fn m(self: Arc<Self>)     the method takes one Arc owner
```

Such a method is called on the pointer, and it **consumes that one pointer**.
It does not copy the value behind it. There are two reasons to want this.

**The method must keep a handle to the value.** A method with `&self` only
borrows. It cannot store a new owner somewhere. A method with `self: Rc<Self>`
receives an owner and can put it where it likes:

```rust
use std::rc::Rc;

struct Listener {
    name: String,
}

impl Listener {
    /// Takes one owner of the listener and stores it in the room.
    fn join(self: Rc<Self>, room: &mut Vec<Rc<Listener>>) {
        room.push(self); // moves the pointer; the Listener does not move
    }

    fn name(&self) -> &str {
        &self.name
    }
}

fn main() {
    let mut room = Vec::new();
    let ana = Rc::new(Listener { name: String::from("ana") });

    Rc::clone(&ana).join(&mut room); // make a second owner and give it away

    assert_eq!(Rc::strong_count(&ana), 2);
    assert!(Rc::ptr_eq(&ana, &room[0]));
    assert_eq!(ana.name(), "ana"); // `&self` methods work through an Rc
}
```

```text
stack                         heap
ana     --------------------> +------------------------+
room[0] --------------------> | strong 2 | Listener    |
                              +------------------------+
`join` moved one pointer into the vector. The Listener was never copied.
```

**The method must move something out of a heap value.** A `Box` is the only
owner of its content, so a `Box<Self>` receiver may take the value apart:

```rust
struct Stage {
    label: String,
    next: Option<Box<Stage>>,
}

impl Stage {
    /// Consumes this boxed stage. Returns its label and the rest of the chain.
    fn split(self: Box<Self>) -> (String, Option<Box<Stage>>) {
        let stage = *self; // move the Stage out of the box
        (stage.label, stage.next)
    }
}

fn main() {
    let chain = Box::new(Stage {
        label: String::from("parse"),
        next: Some(Box::new(Stage { label: String::from("check"), next: None })),
    });
    let (first, rest) = chain.split();
    assert_eq!(first, "parse");
    assert_eq!(rest.expect("one more stage").label, "check");
}
```

An `Rc` or `Arc` receiver cannot do this, because other owners may still be
looking at the value.

Methods that take `&self` need none of this. The lookup from lesson 03 goes
through the pointer for you (step 4), which is why `ana.name()` works.

### Mistakes people make

**Calling the method on a plain value.**

```text
error[E0599]: no method named `join` found for struct `Listener` in the current scope
   |
 2 | struct Listener {
   | --------------- method `join` not found for this struct
...
 6 |     fn join(self: Rc<Self>, room: &mut Vec<Rc<Listener>>) {
   |        ---- the method is available for `Rc<Listener>` here
...
13 |     ana.join(&mut room);
   |         ^^^^ method not found in `Listener`
   |
help: consider wrapping the receiver expression with the appropriate type
   |
13 |     Rc::new(ana).join(&mut room);
   |     ++++++++   +
```

**Giving away your only pointer and then using it.**

```rust,compile_fail,E0382
use std::rc::Rc;

struct Listener {
    name: String,
}

impl Listener {
    fn join(self: Rc<Self>, room: &mut Vec<Rc<Listener>>) {
        room.push(self);
    }
}

fn main() {
    let mut room = Vec::new();
    let ana = Rc::new(Listener { name: String::from("ana") });
    ana.join(&mut room);
    println!("{}", ana.name);
}
```

```text
error[E0382]: the type `Rc` does not implement `Copy`
   |
15 |     let ana = Rc::new(Listener { name: String::from("ana") });
   |         --- this move could be avoided by cloning the original `Rc`, which is inexpensive
16 |     ana.join(&mut room);
   |         --------------- `ana` moved due to this method call
17 |     println!("{}", ana.name);
   |                    ^^^ value borrowed here after move
   |
   = note: consider using `Rc::clone`
```

Write `Rc::clone(&ana).join(&mut room)` when you want to keep `ana`.

**Moving a field out of an `Rc` receiver.**

```text
error[E0507]: cannot move out of an `Rc`
  |
7 |         self.name
  |         ^^^^^^^^^ move occurs because value has type `String`, which does not implement the `Copy` trait
```

Only unique ownership (`self` or `self: Box<Self>`) allows a move out.

### When to use which

- `&self` or `&mut self`: almost always. The caller may hold the value in any
  kind of pointer.
- `self: Box<Self>`: the method consumes a value that lives on the heap, and
  you do not want to move it to the stack first.
- `self: Rc<Self>` or `self: Arc<Self>`: the method must keep or pass on a
  shared owner, for example to register the value somewhere or, with `Arc`, to
  hand it to another thread (Chapter 19). Return `self` if the caller should
  get the owner back. Do not clone it again inside.

### Read more

- [Methods](https://doc.rust-lang.org/reference/items/associated-items.html#methods) in the Rust Reference: the list of allowed receiver types.
- [`std::rc`](https://doc.rust-lang.org/std/rc/index.html) and [`std::sync::Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html).

## Lesson 05 · Generic structs and impl blocks

**Assumed:** lesson 03; `Vec<T>` and `Option<T>` as users see them;
`std::mem::replace` (Chapter 4, lesson 05).

A **generic struct** has a type parameter. `struct Versioned<T>` is not one
type but a pattern for many: `Versioned<String>`, `Versioned<u32>`, and so on.
The compiler produces separate machine code for each `T` you use, so a generic
struct costs nothing at run time.

```rust
use std::mem;

struct Versioned<T> {
    current: T,
    history: Vec<T>,
}

impl<T> Versioned<T> {
    fn new(first: T) -> Self {
        Versioned { current: first, history: Vec::new() }
    }

    /// Installs `next` and keeps the old value. The old value is moved, not
    /// copied, so `T` needs no `Clone`.
    fn set(&mut self, next: T) {
        let old = mem::replace(&mut self.current, next);
        self.history.push(old);
    }

    /// Goes back one step. Returns false if there is no older value.
    fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some(previous) => {
                self.current = previous;
                true
            }
            None => false,
        }
    }
}

// Only these methods compare values, so only this block asks for PartialEq.
impl<T: PartialEq> Versioned<T> {
    fn is_changed(&self) -> bool {
        self.history.last().is_some_and(|old| *old != self.current)
    }
}

fn main() {
    let mut title = Versioned::new(String::from("draft"));
    title.set(String::from("final"));
    assert!(title.is_changed());
    assert!(title.undo());
    assert_eq!(title.current, "draft");
    assert!(!title.undo());

    struct Blob; // a type with no traits at all
    let mut blob = Versioned::new(Blob);
    blob.set(Blob); // fine: `new` and `set` ask nothing of T
}
```

Read `impl<T> Versioned<T>` in two parts. `impl<T>` *declares* a type
parameter. `Versioned<T>` *uses* it. Together: "for every type `T`, these are
methods of `Versioned<T>`".

### What can you do with a `T` you know nothing about?

More than it seems. Without any bound you can own a `T`, move it, store it,
put it in a `Vec` or an `Option`, hand out a `&T`, and drop it. That covers
all of `new`, `set` and `undo`. Inside `set` there is one difficulty: you
cannot move `self.current` out through `&mut self`, because the field would be
left empty. `mem::replace` solves it by moving the new value in and the old
value out in one step. `Option::take` and `Option::replace` do the same for a
field of type `Option<T>`: `take` leaves `None` behind, and `replace` leaves
the new value behind. A third method that works in place is
`Option::get_or_insert(v)`. It stores `v` only if the option is `None`, and
then returns a `&mut` to the value that is inside.

```rust
let mut seat: Option<String> = None;
assert_eq!(seat.replace(String::from("ana")), None); // was empty
assert_eq!(seat.take().as_deref(), Some("ana"));      // empty again
seat.get_or_insert(String::from("ben")).push('!');    // stored, then changed
assert_eq!(seat.as_deref(), Some("ben!"));
```

You need a **bound** only when the code calls a trait's method on a `T`:
comparing needs `PartialEq`, copying needs `Clone`. Put the bound on the
smallest thing that needs it:

- on one `impl` block, for a group of methods (as above);
- on one method, with `where T: PartialEq` after its signature;
- not on the struct definition. A bound there must be repeated on every `impl`
  block and every function that mentions the struct, and it rejects types that
  the storage itself could hold perfectly well.

You can also write an `impl` block for one concrete type, such as
`impl Versioned<f64> { ... }`. Its methods exist only for that type.

### Mistakes people make

**Leaving the parameter off the `impl`.** There are two versions of this
mistake, and the compiler leads you from the first to the second.

```text
error[E0107]: missing generics for struct `Versioned`
  |
5 | impl Versioned {
  |      ^^^^^^^^^ expected 1 generic argument
```

```text
error[E0425]: cannot find type `T` in this scope
  |
5 | impl Versioned<T> {
  |                ^ not found in this scope
  |
help: you might be missing a type parameter
  |
5 | impl<T> Versioned<T> {
  |     +++
```

**Using an operation that `T` does not promise.**

```rust,compile_fail,E0369
struct Versioned<T> {
    current: T,
    history: Vec<T>,
}

impl<T> Versioned<T> {
    fn is_changed(&self) -> bool {
        self.history.last().is_some_and(|old| *old != self.current)
    }
}

fn main() {}
```

```text
error[E0369]: binary operation `!=` cannot be applied to type `T`
  |
8 |         self.history.last().is_some_and(|old| *old != self.current)
  |                                               ---- ^^ ------------ T
  |                                               |
  |                                               T
  |
help: consider restricting type parameter `T` with trait `PartialEq`
  |
6 | impl<T: std::cmp::PartialEq> Versioned<T> {
  |       +++++++++++++++++++++
```

Add the bound, but think about where. If you put it on the block that also
holds `set`, then `Versioned<Blob>` loses `set` too.

**Calling a bounded method on a type that does not meet the bound.** The
method exists, but not for this `T`:

```text
error[E0599]: the method `is_changed` exists for struct `Versioned<Blob>`, but its trait bounds were not satisfied
   |
 1 | struct Versioned<T> {
   | ------------------- method `is_changed` not found for this struct
...
10 | struct Blob;
   | ----------- doesn't satisfy `Blob: PartialEq`
...
13 |     v.is_changed();
   |       ^^^^^^^^^^ method cannot be called on `Versioned<Blob>` due to unsatisfied trait bounds
```

### Read more

- [Generic parameters](https://doc.rust-lang.org/reference/items/generics.html) in the Rust Reference.
- [`std::mem::replace`](https://doc.rust-lang.org/std/mem/fn.replace.html), [`Option::take`](https://doc.rust-lang.org/std/option/enum.Option.html#method.take), [`Option::replace`](https://doc.rust-lang.org/std/option/enum.Option.html#method.replace) and [`Option::get_or_insert`](https://doc.rust-lang.org/std/option/enum.Option.html#method.get_or_insert).

## Lesson 06 · Lifetimes and const parameters

**Assumed:** lesson 05; lifetime parameters on functions and on structs that
borrow (Chapter 5); arrays `[T; N]` and slices `&[T]` (Chapter 3).

A struct can have three kinds of parameters. Each answers a different
question:

| Kind | Example | Question it answers |
| --- | --- | --- |
| Lifetime | `'a` | How long is the borrowed data valid? |
| Type | `T` | What kind of value is stored? |
| Const | `const N: usize` | Which number is fixed at compile time? |

In the list, lifetimes come first: `struct Window<'a, T, const N: usize>`.

### A lifetime parameter: the struct borrows

A struct that holds a reference must name the lifetime of that reference.
`Header<'a>` below says: "a `Header` points into some text, and it cannot be
used after that text is gone".

```rust
struct Header<'a> {
    key: &'a str,
    value: &'a str,
}

impl<'a> Header<'a> {
    fn parse(line: &'a str) -> Option<Self> {
        let (key, value) = line.split_once(':')?;
        Some(Header { key: key.trim(), value: value.trim() })
    }

    /// The result borrows the original line (`'a`), not this Header.
    fn value(&self) -> &'a str {
        self.value
    }
}

fn main() {
    let line = String::from("Content-Length: 42");
    let value;
    {
        let header = Header::parse(&line).unwrap();
        assert_eq!(header.key, "Content-Length");
        value = header.value();
    } // `header` is gone here, but `line` is still alive
    assert_eq!(value, "42");
}
```

Look at the return type of `value`. The struct owns no text. It holds two
references into `line`. A method can hand out a copy of such a reference, and
that copy is valid for `'a`, the life of `line`. It does not depend on how
long the `Header` itself lives.

### A const parameter: a number in the type

An array type includes its length: `[u32; 4]` and `[u32; 5]` are different
types. A **const parameter** lets a struct or function work for every length
while the compiler still knows each one.

```rust
struct Histogram<const N: usize> {
    buckets: [u32; N],
}

impl<const N: usize> Histogram<N> {
    fn new() -> Self {
        Histogram { buckets: [0; N] }
    }

    /// Counts a value from 0.0 up to, but not including, 1.0.
    fn record(&mut self, value: f64) {
        if N > 0 && (0.0..1.0).contains(&value) {
            self.buckets[(value * N as f64) as usize] += 1;
        }
    }
}

fn main() {
    let mut quarters: Histogram<4> = Histogram::new();
    for value in [0.1, 0.3, 0.35, 0.9, 1.5] {
        quarters.record(value);
    }
    assert_eq!(quarters.buckets, [1, 2, 0, 1]);
    // N is known to the compiler, so it is not stored in the value.
    assert_eq!(size_of::<Histogram<4>>(), 4 * size_of::<u32>());
}
```

Compare two ways to refer to a run of elements:

```text
&[T; N]   one pointer. The length N is part of the type.
&[T]      a pointer and a length. The length is stored next to the pointer
          and is only known at run time.
```

### Mistakes people make

**Letting elision tie the result to `&self`.** If you leave the lifetime out
of the return type, the elision rules (Chapter 5) give it the lifetime of
`&self`:

```rust,compile_fail,E0597
struct Header<'a> {
    key: &'a str,
    value: &'a str,
}

impl<'a> Header<'a> {
    fn parse(line: &'a str) -> Option<Self> {
        let (key, value) = line.split_once(':')?;
        Some(Header { key: key.trim(), value: value.trim() })
    }

    fn value(&self) -> &str {
        self.value
    }
}

fn main() {
    let line = String::from("Content-Length: 42");
    let value;
    {
        let header = Header::parse(&line).unwrap();
        value = header.value();
    }
    println!("{value}");
}
```

```text
error[E0597]: `header` does not live long enough
   |
21 |         let header = Header::parse(&line).unwrap();
   |             ------ binding `header` declared here
22 |         value = header.value();
   |                 ^^^^^^ borrowed value does not live long enough
23 |     }
   |     - `header` dropped here while still borrowed
24 |     println!("{value}");
   |                ----- borrow later used here
```

The method body is fine. Only the signature is too strict: it promises less
than the code can give. Write `-> &'a str`.

**Mixing lengths.**

```text
error[E0308]: mismatched types
  |
5 |     let h: Histogram<3> = Histogram { buckets: [0; 4] };
  |                                                ^^^^^^ expected an array with a size of 3, found one with a size of 4
```

**Doing arithmetic with a const parameter in a type.** Stable Rust accepts a
const parameter in a type only on its own, as `N`:

```text
error: generic parameters may not be used in const operations
  |
5 |     fn with_overflow(self) -> [u32; N + 1] {
  |                                     ^ cannot perform const operation using `N`
  |
  = help: const parameters may only be used as standalone arguments here, i.e. `N`
```

### When to use which

- A fixed length that the compiler should check: `[T; N]` with a const
  parameter.
- A length that is only known at run time, borrowed: `&[T]`.
- A length that changes, owned: `Vec<T>`.

### Read more

- [Const generics](https://doc.rust-lang.org/reference/items/generics.html#const-generics) in the Rust Reference.
- [Lifetime elision](https://doc.rust-lang.org/reference/lifetime-elision.html) in the Rust Reference.
- [The array type](https://doc.rust-lang.org/std/primitive.array.html) in the standard library.

## Lesson 07 · Deriving common traits

**Assumed:** lessons 01 to 05; `Clone` and `Copy` (Chapter 4).

Many traits have one obvious implementation for a struct: do the same thing to
every field. The attribute `#[derive(...)]` asks the compiler to write that
implementation.

| Trait | What the derived code does | What every field must be |
| --- | --- | --- |
| `Debug` | `{:?}` prints the type name and each field | `Debug` |
| `Clone` | `clone()` clones each field | `Clone` |
| `Copy` | assignment copies the bits; the old value stays usable | `Copy` (and the struct must also be `Clone`) |
| `PartialEq` | `==` compares each pair of fields | `PartialEq` |
| `Eq` | adds no new behaviour; states that `==` has no exceptions such as NaN | `Eq` |
| `PartialOrd`, `Ord` | `<` compares fields in the order of the definition; the first difference decides | the same trait |
| `Hash` | feeds each field to the hasher | `Hash` |
| `Default` | `default()` uses each field's default | `Default` |

```rust
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct Version {
    major: u16,
    minor: u16,
    patch: u16,
}

fn main() {
    let old = Version { major: 1, minor: 9, patch: 3 };
    let new = Version { major: 1, minor: 10, patch: 0 };

    assert!(old < new); // `major` ties, so `minor` decides
    assert_eq!(Version::default(), Version { major: 0, minor: 0, patch: 0 });
    assert_eq!(format!("{old:?}"), "Version { major: 1, minor: 9, patch: 3 }");

    let copy = old; // Copy: `old` is still usable
    let seen: HashSet<Version> = [old, copy, new].into_iter().collect();
    assert_eq!(seen.len(), 2);
}
```

Two points about the derived comparison. It follows the order in which you
declared the fields, so moving `patch` to the top would change which version
is "greater". And it compares **every** field.

`Debug` is output for programmers. Text for users is the job of `Display`,
which cannot be derived, because no rule could know how your type should look
to a user. Chapter 17 covers it.

### A derive is not always what you want

A derive treats all fields alike. Sometimes that is wrong:

- A field may not be part of what the value *is*, for example a cached result.
  A derived `==` would still compare it. In that case write `impl PartialEq`
  by hand and compare only the fields that count. If the type is also `Hash`,
  hash exactly those fields too: values that are equal must hash equally.
- A derived `Default` gives every number 0 and every `String` the empty text.
  If the type has an invariant, that value may break it. The `Rect` of
  lesson 03 must not derive `Default`: it would create a rectangle of 0 by 0.
- `Clone` on a struct that holds an `Rc` clones the pointer. The two structs
  then share one value. That is correct, but be sure it is what you mean.

On a generic struct, a derive adds its trait as a bound on **every** type
parameter. `#[derive(Clone)] struct Handle<T> { shared: Rc<T> }` produces
`impl<T: Clone> Clone for Handle<T>`, although an `Rc<T>` can be cloned for
any `T`. When that bound is too strict, write the impl yourself:

```rust
use std::rc::Rc;

struct Texture; // not Clone

struct Handle<T> {
    shared: Rc<T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Handle { shared: Rc::clone(&self.shared) }
    }
}

fn main() {
    let a = Handle { shared: Rc::new(Texture) };
    let b = a.clone();
    assert!(Rc::ptr_eq(&a.shared, &b.shared));
}
```

### Mistakes people make

**Deriving `Copy` for a struct that owns heap data.**

```rust,compile_fail,E0204
#[derive(Clone, Copy)]
struct Version {
    major: u16,
    label: String,
}

fn main() {}
```

```text
error[E0204]: the trait `Copy` cannot be implemented for this type
  |
1 | #[derive(Clone, Copy)]
  |                 ---- in this derive macro expansion
2 | struct Version {
  |        ^^^^^^^
3 |     major: u16,
4 |     label: String,
  |     ------------- this field does not implement `Copy`
```

Remove `Copy`. The struct can still be `Clone`.

**A field type that lacks the trait.**

```text
error[E0277]: `Build` doesn't implement `Debug`
  |
2 | #[derive(Debug)]
  |          ----- in this derive macro expansion
...
5 |     build: Build,
  |     ^^^^^^^^^^^^ the trait `Debug` is not implemented for `Build`
  |
help: consider annotating `Build` with `#[derive(Debug)]`
```

The error is reported on the outer struct, but the fix belongs on the inner
type.

**Deriving `Eq` with a floating-point field.** `f64` has `PartialEq` but not
`Eq`, because NaN is not equal to itself.

```text
error[E0277]: the trait bound `f64: Eq` is not satisfied
    |
  1 | #[derive(PartialEq, Eq)]
    |                     -- in this derive macro expansion
...
  4 |     value: f64,
    |     ^^^^^^^^^^ the trait `Eq` is not implemented for `f64`
```

Derive only `PartialEq`.

**The hidden bound on a generic derive.**

```text
error[E0599]: the method `clone` exists for struct `Handle<Texture>`, but its trait bounds were not satisfied
  |
2 | struct Texture;
  | -------------- doesn't satisfy `Texture: Clone`
3 | #[derive(Clone)]
4 | struct Handle<T> {
  | ---------------- method `clone` not found for this struct because it doesn't satisfy `Handle<Texture>: Clone`
...
9 |     let b = a.clone();
  |               ^^^^^ method cannot be called on `Handle<Texture>` due to unsatisfied trait bounds
...
  = help: consider manually implementing the trait to avoid undesired bounds
```

### Read more

- [The `derive` attribute](https://doc.rust-lang.org/reference/attributes/derive.html) in the Rust Reference.
- The "Derivable" sections of [`PartialEq`](https://doc.rust-lang.org/std/cmp/trait.PartialEq.html#derivable), [`Ord`](https://doc.rust-lang.org/std/cmp/trait.Ord.html#derivable) and [`Default`](https://doc.rust-lang.org/std/default/trait.Default.html#derivable).
- [`std::fmt::Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html).

## Lesson 08 · Cell and RefCell

**Assumed:** lesson 03; the borrow rule from Chapter 5: at any moment a value
has either any number of `&` references or exactly one `&mut` reference.

Normally you need `&mut` to change a value. Sometimes that is too strict. A
method that only reads, from the caller's point of view, may still want to
count its calls or remember a result. A value with several owners (an `Rc`)
can never give out `&mut` at all. For these cases the standard library has
types with **interior mutability**: the value inside can change although you
only hold a `&` to the outside. This lesson covers the two that work within
one thread.

**`Cell<T>`** never gives you a reference to its content. You can only move
whole values in and out: `set` puts one in, `replace` swaps one in and returns
the old one, `get` returns a copy (so it needs `T: Copy`), and `take` leaves
the default value behind (so it needs `T: Default`). Since nobody can hold a
reference into a `Cell`, a swap can never invalidate one. A `Cell<T>` is
exactly as big as a `T`.

**`RefCell<T>`** does give out references, wrapped in **guards**. `borrow()`
returns a `Ref<T>` that works like `&T`. `borrow_mut()` returns a `RefMut<T>`
that works like `&mut T`. The `RefCell` counts the guards that are alive and
applies the borrow rule while the program runs: many `Ref`, or one `RefMut`.
A guard ends its borrow when it is dropped.

```rust
use std::cell::{Cell, RefCell};

struct Report {
    rows: Vec<u32>,
    total: Cell<Option<u32>>,    // a small Copy value, replaced as a whole
    audit: RefCell<Vec<String>>, // a Vec that we must push into
}

impl Report {
    fn new(rows: Vec<u32>) -> Self {
        Report { rows, total: Cell::new(None), audit: RefCell::new(Vec::new()) }
    }

    /// Adds up the rows once and remembers the answer. Note the `&self`.
    fn total(&self) -> u32 {
        if let Some(cached) = self.total.get() {
            return cached;
        }
        let sum = self.rows.iter().sum();
        self.total.set(Some(sum));
        self.audit.borrow_mut().push(format!("summed {} rows", self.rows.len()));
        sum
    }
}

fn main() {
    let report = Report::new(vec![3, 4, 5]);
    assert_eq!(report.total(), 12);
    assert_eq!(report.total(), 12);
    assert_eq!(report.audit.borrow().len(), 1); // the sum was computed once
}
```

### How long does a guard live?

Most `RefCell` problems are about this question.

- A guard that you bind to a variable lives until the variable goes out of
  scope, or until you call `drop(guard)`.
- A guard that is only a temporary inside an expression, as in
  `cell.borrow().len()`, lives until the end of the statement it is in.

Two kinds of expression keep such a temporary alive for longer than their
first line:

| Where the temporary guard is created | When it is released |
| --- | --- |
| `let n = cell.borrow().len();` | at the semicolon |
| `match cell.borrow().len() { ... }` | after the whole `match` |
| `if let Some(x) = cell.borrow().first() { A } else { B }` | after block `A`, before block `B` runs |

The last row is the rule of edition 2024, which this course uses. In edition
2021 and earlier the temporary also lived through block `B`. Much older
material describes that behaviour.

### When the rule is broken

`borrow` and `borrow_mut` **panic** if the borrow rule would be broken.
`try_borrow` and `try_borrow_mut` return `Err` in the same situation, so the
program can react.

```rust,should_panic
use std::cell::RefCell;

fn main() {
    let scores = RefCell::new(vec![3, 9, 4]);
    let best = scores.borrow(); // this guard stays alive ...
    scores.borrow_mut().push(*best.iter().max().unwrap()); // ... so this panics
}
```

```text
thread 'main' (122373688) panicked at src/main.rs:6:12:
RefCell already borrowed
```

The number in parentheses is a thread id. It is different on every run.

The fix is to end the first borrow before the second begins. Copy what you
need out of the guard in one statement:

```rust
use std::cell::RefCell;

fn main() {
    let scores = RefCell::new(vec![3, 9, 4]);
    let best = *scores.borrow().iter().max().unwrap(); // guard released at `;`
    scores.borrow_mut().push(best);
    assert_eq!(scores.borrow().len(), 4);
    assert!(scores.try_borrow_mut().is_ok());
}
```

The borrow rule itself has not gone away. `RefCell` checks it later, at run
time, and you pay with a small counter and a possible panic.

### Mistakes people make

**Trying to change a plain field through `&self`.**

```rust,compile_fail,E0594
struct Page {
    title: String,
    hits: u32,
}

impl Page {
    fn title(&self) -> &str {
        self.hits += 1;
        &self.title
    }
}

fn main() {}
```

```text
error[E0594]: cannot assign to `self.hits`, which is behind a `&` reference
  |
8 |         self.hits += 1;
  |         ^^^^^^^^^^^^^^ `self` is a `&` reference, so it cannot be written to
  |
help: consider changing this to be a mutable reference
  |
7 |     fn title(&mut self) -> &str {
  |               +++
```

The compiler suggests `&mut self`. That is right when the change is part of
the value's real state. For a statistic, make the field a `Cell<u32>` and keep
`&self`.

**Calling `get` on a `Cell` whose content is not `Copy`.**

```text
error[E0599]: the method `get` exists for struct `Cell<String>`, but its trait bounds were not satisfied
    |
  4 |     let copy = name.get();
    |                     ^^^
...
    = note: the following trait bounds were not satisfied:
            `String: Copy`
```

Use `take` or `replace` to move the value out, or use a `RefCell` when you
need to look at the value in place.

**Sharing one between threads.** Neither type is safe for that, and the
compiler knows it:

```text
error[E0277]: `RefCell<Vec<u32>>` cannot be shared between threads safely
    |
  6 |         s.spawn(|| log.borrow_mut().push(1));
    |           ----- ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `RefCell<Vec<u32>>` cannot be shared between threads safely
    |           |
    |           required by a bound introduced by this call
    |
    = help: the trait `Sync` is not implemented for `RefCell<Vec<u32>>`
    = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` instead
```

Chapter 19 covers `Mutex`, `RwLock` and the atomic types, which do this job
between threads.

### When to use which

1. Can the caller simply pass `&mut`? Then use a plain field. This is the
   normal case.
2. Is the value small and `Copy`, or is it always replaced as a whole? Use
   `Cell`. It has no run-time check and cannot panic.
3. Do you need a reference into the value, for example to push into a `Vec`?
   Use `RefCell`, and keep every guard short.
4. Is more than one thread involved? Use neither (Chapter 19).

Both types are built on `UnsafeCell`, the one primitive that makes a change
behind `&` legal. Chapter 23 builds a small `Cell` from it, and the mastery
track ([`mastery/m2_unsafe`](../../mastery/m2_unsafe/README.md)) builds both.

### Read more

- [`std::cell`](https://doc.rust-lang.org/std/cell/index.html): the module documentation compares all the cell types.
- [`Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html) and [`RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html).
- [Interior mutability](https://doc.rust-lang.org/reference/interior-mutability.html) in the Rust Reference.
- [`if let` temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-if-let-scope.html) in the Edition Guide.

## Lesson 10 · Weak and parent links

**Assumed:** `Rc` and its strong count (Chapter 4); `RefCell` and guard
lifetimes (lesson 08). Do this lesson before the capstone.

An `Rc<T>` shares ownership by counting. Each `Rc` that points at the value
adds one to the **strong count**. The value is dropped when the count reaches
zero. This works as long as ownership never runs in a circle. If `a` owns `b`
and `b` owns `a`, each keeps the other's count at one or more. When your own
variables are gone, the two values still hold each other. Nothing can reach
them and nothing drops them. This is a **reference cycle**. It is a memory
leak, and the compiler does not report it.

```text
a strong cycle (leak)              one direction weak (no leak)

   a ---Rc---> b                      owner ---Rc---> part
   ^           |                        ^              |
   +----Rc-----+                        +----Weak------+

each count stays at 1 for ever     dropping `owner` drops both
```

The cure is to decide which direction **owns**. The other direction gets a
**`Weak<T>`**: a pointer that can find the value but does not keep it alive.

- `Rc::downgrade(&rc)` makes a `Weak` from an `Rc`. It adds one to the **weak
  count** and leaves the strong count alone.
- `weak.upgrade()` returns `Option<Rc<T>>`. You get `Some` with a new strong
  pointer if the value is still alive, and `None` if it has been dropped.
- `Weak::new()` makes a `Weak` that points at nothing. Its `upgrade` always
  returns `None`.

A `Weak` cannot be dereferenced. The value may be gone, so you must ask
first, and `upgrade` is how you ask.

### Worked example: subscribers that may disappear

A channel sends text to inboxes. The channel must not keep an inbox alive: if
its owner drops it, the channel should simply forget it.

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Inbox {
    messages: RefCell<Vec<String>>,
}

struct Channel {
    subscribers: RefCell<Vec<Weak<Inbox>>>,
}

impl Channel {
    fn subscribe(&self, inbox: &Rc<Inbox>) {
        self.subscribers.borrow_mut().push(Rc::downgrade(inbox));
    }

    /// Delivers to every inbox that still exists and forgets the others.
    /// Returns the number of inboxes that received the text.
    fn publish(&self, text: &str) -> usize {
        let mut delivered = 0;
        self.subscribers.borrow_mut().retain(|weak| match weak.upgrade() {
            Some(inbox) => {
                inbox.messages.borrow_mut().push(text.to_string());
                delivered += 1;
                true
            }
            None => false, // the inbox was dropped: remove the dead link
        });
        delivered
    }
}

fn main() {
    let channel = Channel { subscribers: RefCell::new(Vec::new()) };
    let ana = Rc::new(Inbox { messages: RefCell::new(Vec::new()) });
    channel.subscribe(&ana);
    {
        let ben = Rc::new(Inbox { messages: RefCell::new(Vec::new()) });
        channel.subscribe(&ben);
        assert_eq!(Rc::strong_count(&ben), 1); // subscribing added no owner
        assert_eq!(Rc::weak_count(&ben), 1);
        assert_eq!(channel.publish("hello"), 2);
    } // `ben` is dropped here, although the channel still has a Weak to it

    assert_eq!(channel.publish("anyone?"), 1);
    assert_eq!(ana.messages.borrow().len(), 2);
    assert_eq!(channel.subscribers.borrow().len(), 1);
}
```

### The same idea in a tree

In a tree, ownership points **down** and observation points **up**. A parent
holds an `Rc` to each child, so the children live as long as the parent. A
child holds a `Weak` to its parent, so it can walk up without owning anything.
Nodes are usually shared *and* changed, which gives the type
`Rc<RefCell<Node>>`: `Rc` for several handles to one node, `RefCell` to change
the node through any handle. The parent link then has the type
`Weak<RefCell<Node>>`, and `Weak::new()` is a natural "no parent".

```text
            root         strong 1  (your variable)
           /    \        weak   2  (one from each child)
      Rc  v      v  Rc
        left    right    each: strong 2 (root's list and your variable), weak 0

        parent links are Weak and point up to root
```

### Mistakes people make

**A strong link in both directions.** There is no compiler message for this.
You notice it only by looking: a strong count that is higher than the number
of owners you intended, or a `Drop` implementation that never runs. The tests
of this lesson check both.

**Using a `Weak` as if it were the value.**

```rust,compile_fail,E0609
use std::rc::{Rc, Weak};

struct Hub {
    name: String,
}

fn main() {
    let hub = Rc::new(Hub { name: String::from("lab") });
    let link: Weak<Hub> = Rc::downgrade(&hub);
    println!("{}", link.name);
}
```

```text
error[E0609]: no field `name` on type `std::rc::Weak<Hub>`
   |
10 |     println!("{}", link.name);
   |                         ^^^^ unknown field
```

Upgrade first and handle both answers: `if let Some(hub) = link.upgrade()
{ ... }`. Calling `upgrade().unwrap()` turns "the owner is gone" into a panic.

**Holding a `RefCell` guard while you borrow the same cell again.** With
`Rc<RefCell<Node>>` this is easy to do by accident:

```rust,should_panic
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    parent: Weak<RefCell<Node>>,
    visits: u32,
}

fn main() {
    let root = Rc::new(RefCell::new(Node { parent: Weak::new(), visits: 0 }));
    let leaf = Rc::new(RefCell::new(Node { parent: Rc::downgrade(&root), visits: 0 }));

    if let Some(parent) = leaf.borrow().parent.upgrade() {
        parent.borrow_mut().visits += 1;
        leaf.borrow_mut().visits += 1; // panics: `leaf` is still borrowed
    }
}
```

```text
thread 'main' (122373812) panicked at src/main.rs:15:14:
RefCell already borrowed
```

The guard from `leaf.borrow()` is a temporary in the `if let` condition, so it
lives until the end of the `if` block (lesson 08). Finish reading in a
statement of its own:

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    parent: Weak<RefCell<Node>>,
    visits: u32,
}

fn main() {
    let root = Rc::new(RefCell::new(Node { parent: Weak::new(), visits: 0 }));
    let leaf = Rc::new(RefCell::new(Node { parent: Rc::downgrade(&root), visits: 0 }));

    let parent = leaf.borrow().parent.upgrade(); // guard released at `;`
    if let Some(parent) = parent {
        parent.borrow_mut().visits += 1;
        leaf.borrow_mut().visits += 1;
    }
    assert_eq!((root.borrow().visits, leaf.borrow().visits), (1, 1));
}
```

### When to use which

- `Rc` alone: shared data with no way back, such as several lists that share
  a tail.
- `Rc` one way and `Weak` the other way: parent links, observers, caches.
  Anything that should find a value without deciding how long it lives.
- No pointers at all: keep all nodes in one `Vec` and let them refer to each
  other by index (`usize`). Much real Rust code does this. There is one owner,
  no counting and no `RefCell`.
- Across threads: `Arc` with `std::sync::Weak`, and a `Mutex` in place of the
  `RefCell` (Chapter 19).

### Read more

- [`std::rc::Weak`](https://doc.rust-lang.org/std/rc/struct.Weak.html) and the [`std::rc` module documentation](https://doc.rust-lang.org/std/rc/index.html), which discusses cycles.
- [`Rc::downgrade`](https://doc.rust-lang.org/std/rc/struct.Rc.html#method.downgrade), [`Rc::weak_count`](https://doc.rust-lang.org/std/rc/struct.Rc.html#method.weak_count) and [`Rc::new_cyclic`](https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic), which builds a value that needs a `Weak` to itself.

## Lesson 09 · Capstone: validated inventory

**Assumed:** every lesson of this chapter.

The capstone gives you the method signatures and the tests. The fields are
your decision. No new idea is needed. Work in this order.

1. **Write down the invariant.** Read the contracts and state, in one or two
   sentences, what must be true of every inventory value. The constructor's
   contract tells you most of it.
2. **Choose a field for each piece of state.** Count the things a value must
   remember; the tests can only see them through the methods. For each one,
   pick the smallest type that can hold it. The return types of the methods
   are a strong hint.
3. **Sort the methods by receiver.** Which ones only read? Which ones change
   the real state? Which one finishes the value? One method reads from the
   caller's point of view and still has to count something. That is the case
   lesson 08 is about, and its "When to use which" list tells you which type
   to choose. One test measures the size of your struct and fails for the
   heavier choice.
4. **Write the constructor.** Clean the input first, then check it, then
   build the value. Allocate once, and only for an input you accept.
5. **Write the method that can refuse.** Compute the new value into a local
   variable with a checked operation (Chapter 3). Assign to the field only
   when that succeeded. A refused call must leave the value exactly as it was.
6. **Write the consuming method.** With a `self` receiver you own every
   field, so moving one out is allowed and costs nothing.

When all tests pass, do what the mastery check asks: change one requirement,
for example a maximum stock, and find every place that must change with it.
That list of places is the real lesson of an invariant.

### Read more

- [`u16::checked_add`](https://doc.rust-lang.org/std/primitive.u16.html#method.checked_add) and [`u8::saturating_add`](https://doc.rust-lang.org/std/primitive.u8.html#method.saturating_add).
- [`str::trim`](https://doc.rust-lang.org/std/primitive.str.html#method.trim).
