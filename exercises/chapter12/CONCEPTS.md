# Chapter 12 concepts — Operators for your own types

[Chapter guide](README.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Read one section, then do the lesson with the same number. Each section gives you
the mental model, one worked example that is *not* the exercise, the mistakes
people really make with the compiler's own words, and links to the official
documentation.

## What you will be able to do

**After this chapter you can:**

- give your own types `+`, `-`, `*`, unary `-`, `!`, `+=` and the bit operators, and
  say which operands each one moves;
- write `==` for a type whose identity is not "all fields", and keep `Eq` and `Hash`
  honest with it;
- give a type a total order, or a partial one, and keep `<`, `sort` and `==` in
  agreement;
- make `value[index]` work for reading and for writing;
- name the operators that Rust does not let you overload.

## Before you start

You need from earlier chapters: moves and `Copy` (chapter 4), references
(chapter 5), structs and methods (chapter 9), enums and patterns (chapter 10), and
traits with associated types and `#[derive]` (chapter 11).

## Time

These are estimates.

| Lesson | Reading | Exercise |
| --- | --- | --- |
| 01 · Unary, binary, and assignment operators | 15 min | 20 min |
| 02 · Equivalence and NaN | 15 min | 20 min |
| 03 · Partial and total comparisons | 15 min | 30 min |
| 04 · Index and IndexMut | 15 min | 30 min |
| 05 · Capstone: a small bit-set API | 5 min | 30 min |

## Lesson 01 · Unary, binary, and assignment operators

An operator is a short way to call a trait method. The compiler rewrites every
operator expression into a method call:

```text
you write    the compiler calls                  trait in std::ops
a + b        Add::add(a, b)                      Add<Rhs = Self>
a - b        Sub::sub(a, b)                      Sub<Rhs = Self>
a * b        Mul::mul(a, b)                      Mul<Rhs = Self>
-a           Neg::neg(a)                         Neg
!a           Not::not(a)                         Not
a += b       AddAssign::add_assign(&mut a, b)    AddAssign<Rhs = Self>
```

`!` has two meanings in the standard library: for `bool` it is logical "not", and
for an integer it flips every bit.

To *overload* an operator means to implement its trait for your type. This is the
shape of `Add`:

```rust
// The shape of `std::ops::Add`, under another name so that this block compiles.
trait MyAdd<Rhs = Self> {
    type Output;
    fn add(self, rhs: Rhs) -> Self::Output;
}
```

Read three facts from it.

1. `self` and `rhs` are taken **by value**. `a + b` moves both operands, unless
   their types are `Copy`.
2. `Rhs` is a type parameter that defaults to `Self`. One type can implement `Add`
   several times, once for each type allowed on the right.
3. `Output` is an associated type. For one pair of operand types there is exactly
   one result type, and the implementation chooses it. It does not have to be
   `Self`.

The assignment traits are separate traits. `add_assign` takes `&mut self`, changes
the left operand in place, and returns nothing, so it has no `Output`.
Implementing `Add` does not give you `+=`.

### Worked example: points in time

```rust
use std::ops::{Add, AddAssign, Sub};

/// A length of time, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Millis(u64);

/// A point in time: milliseconds since the program started.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Stamp(u64);

// point + length = point: the right operand has another type.
impl Add<Millis> for Stamp {
    type Output = Stamp;
    fn add(self, rhs: Millis) -> Stamp {
        Stamp(self.0 + rhs.0)
    }
}

// point - point = length: the result has another type.
impl Sub for Stamp {
    type Output = Millis;
    fn sub(self, earlier: Stamp) -> Millis {
        Millis(self.0 - earlier.0)
    }
}

// point += length changes the point in place.
impl AddAssign<Millis> for Stamp {
    fn add_assign(&mut self, rhs: Millis) {
        self.0 += rhs.0;
    }
}

fn main() {
    let start = Stamp(1_000);
    let mut now = start + Millis(250);
    now += Millis(50);
    assert_eq!(now, Stamp(1_300));
    assert_eq!(now - start, Millis(300));
    // `Stamp` is `Copy`, so the operators copied `start` and it is still usable.
    assert_eq!(start, Stamp(1_000));
}
```

There is no `Stamp + Stamp`, because adding two points in time means nothing. The
compiler rejects it. The arithmetic inside an operator method follows the usual
integer rules from chapter 3: `self.0 - earlier.0` panics in a debug build if
`earlier` is the later point.

For a type that owns heap memory, a by-value operand is useful: the method can keep
the left operand's buffer. A common shape is an owned left side and a borrowed
right side:

```rust
use std::ops::Add;

/// The lengths of the legs of a walk, in kilometres.
#[derive(Debug)]
struct Trail(Vec<u32>);

impl Add<&Trail> for Trail {
    type Output = Trail;
    fn add(mut self, rhs: &Trail) -> Trail {
        self.0.extend_from_slice(&rhs.0);
        self
    }
}

fn main() {
    let mut legs = Vec::with_capacity(4);
    legs.extend([3, 5]);
    let morning = Trail(legs);
    let evening = Trail(vec![2]);
    let buffer = morning.0.as_ptr();

    let day = morning + &evening;
    assert_eq!(day.0, [3, 5, 2]);
    assert_eq!(day.0.as_ptr(), buffer, "the sum lives in the morning's buffer");
    assert_eq!(evening.0, [2], "the right side was only borrowed");
}
```

```text
before    morning ──► [3, 5, _, _]     evening ──► [2]
          let day = morning + &evening;
after     day ──────► [3, 5, 2, _]     evening ──► [2]     morning: moved away
```

The standard library gives `String + &str` the same shape.

### Mistakes people make

**Using an operand after the operator moved it.** Add one line at the end of
`main` in the `Trail` program, a `println!` that reads `morning.0.len()`:

```text
error[E0382]: borrow of moved value: `morning`
   |
18 |     let morning = Trail(legs);
   |         ------- move occurs because `morning` has type `Trail`, which does not implement the `Copy` trait
...
22 |     let day = morning + &evening;
   |               ------------------ `morning` moved due to usage in operator
...
26 |     println!("{} legs, {} of them in the morning", day.0.len(), morning.0.len());
   |                                                                 ^^^^^^^^^ value borrowed here after move
```

Fix: derive `Clone` and clone the operand you still need
(`morning.clone() + &evening`), or also implement the operator for two
references, which must then build a new value.

**Using the operator inside its own method.** In `impl Add for Millis`, the body
`self + rhs` is a call to the method you are writing:

```text
warning: function cannot return without recursing
  |
8 |     fn add(self, rhs: Millis) -> Millis {
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ cannot return without recursing
9 |         self + rhs
  |         ---------- recursive call site
  |
  = note: `#[warn(unconditional_recursion)]` on by default
```

It is only a warning, and it appears only in the simplest cases. A debug build
then stops with `has overflowed its stack`. A release build can loop forever. Fix:
work on the fields, `Millis(self.0 + rhs.0)`.

**Expecting `+=` from `Add`.**

```text
error[E0368]: binary assignment operation `+=` cannot be applied to type `Millis`
   |
15 |     total += Millis(250);
   |     -----^^^^^^^^^^^^^^^
   |     |
   |     cannot use `+=` on type `Millis`
   |
note: an implementation of `AddAssign` might be missing for `Millis`
```

Fix: implement `AddAssign`. In the same way, `Mul<u64> for Millis` gives you
`m * 3` but not `3 * m`; that needs a second impl, `Mul<Millis> for u64`.

### When to use which

- Small `Copy` types: implement the operator for the type itself.
- Types with a buffer: take the left operand by value and reuse its buffer. Add
  impls for references only if callers need to keep both operands.
- Implement the `...Assign` trait whenever the left operand can be changed in place.
- Overload an operator only when a reader would guess its meaning from mathematics
  or from the standard library. Otherwise write a named method.
- Some operators cannot be overloaded: `&&` and `||` (they may skip their right
  side, which a method call cannot do), `=` (always a move or a copy), `as`, the
  borrow operators `&` and `&mut`, and the range operator `..`, which always builds
  a range value.

**Read more:** [`std::ops`](https://doc.rust-lang.org/std/ops/index.html) ·
[`Add`](https://doc.rust-lang.org/std/ops/trait.Add.html) ·
[`AddAssign`](https://doc.rust-lang.org/std/ops/trait.AddAssign.html) ·
[operator expressions in the Rust Reference](https://doc.rust-lang.org/reference/expressions/operator-expr.html)

## Lesson 02 · Equivalence and NaN

`a == b` is `PartialEq::eq(&a, &b)`, and `a != b` is `PartialEq::ne(&a, &b)`. Notice
the references: comparing never moves its operands. You implement only `eq`. The
trait provides `ne` as "not `eq`", and you should not write your own.

`#[derive(PartialEq)]` compares every field, in order. Write the impl by hand when
the *identity* of a value is something else: fewer fields, or a normalized form.
Compare fields inside `eq`. Writing `self == other` there is the recursion mistake
from Lesson 01, and this time the compiler gives no warning.

An `eq` has rules to keep. No compiler checks them, but collections and other code
rely on them:

- **symmetric**: if `a == b`, then `b == a`;
- **transitive**: if `a == b` and `b == c`, then `a == c`.

`Eq` adds one more rule and no methods:

- **reflexive**: `a == a` for every value.

`impl Eq for T {}` is therefore a promise, not code. Hash maps, hash sets and
B-tree maps demand that promise from their keys.

Floating-point numbers cannot make it. A float can hold NaN ("not a number", for
example the result of `0.0 / 0.0`), and NaN is not equal to anything, not even to
itself. So `f32` and `f64` implement `PartialEq` but not `Eq`. Float `==` compares
numeric values by the IEEE 754 rules. It is not a comparison of bits.

```rust
let nan = 0.0_f64 / 0.0;
assert!(nan.is_nan());
assert!(nan != nan);
```

### Worked example: equal fractions

One half and two quarters are different field values but the same number.

```rust
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy)]
struct Fraction {
    num: i32,
    den: i32, // always greater than zero
}

impl PartialEq for Fraction {
    fn eq(&self, other: &Fraction) -> bool {
        // a/b == c/d exactly when a*d == c*b. An i64 holds any product of two i32.
        i64::from(self.num) * i64::from(other.den) == i64::from(other.num) * i64::from(self.den)
    }
}

// Every fraction equals itself, so the promise of `Eq` holds.
impl Eq for Fraction {}

impl Hash for Fraction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Equal values must hash equally, so hash the reduced form:
        // 2/4 and 1/2 both hash as (1, 2).
        let (num, den) = (i64::from(self.num), i64::from(self.den));
        let divisor = gcd(num.abs(), den);
        (num / divisor, den / divisor).hash(state);
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

// A second impl compares a fraction with a whole number.
impl PartialEq<i32> for Fraction {
    fn eq(&self, whole: &i32) -> bool {
        i64::from(self.num) == i64::from(*whole) * i64::from(self.den)
    }
}

fn main() {
    let half = Fraction { num: 1, den: 2 };
    let two_quarters = Fraction { num: 2, den: 4 };
    assert_eq!(half, two_quarters);
    assert!(Fraction { num: 6, den: 2 } == 3);

    let mut seen = HashSet::new();
    seen.insert(half);
    seen.insert(two_quarters);
    assert_eq!(seen.len(), 1, "equal keys are stored once");
}
```

The `Hash` impl is the important part. A hash set looks only where the hash sends
it. If two equal values hash differently, the set normally never compares them,
and stores both.

`PartialEq<i32> for Fraction` gives you `fraction == 3`. It does not give you
`3 == fraction`; for symmetry you would add `impl PartialEq<Fraction> for i32`.

### Mistakes people make

**Deriving `Eq` for a type with a float field.**

```text
error[E0277]: the trait bound `f32: Eq` is not satisfied
   |
 1 | #[derive(Debug, PartialEq, Eq)]
   |                            -- in this derive macro expansion
...
 4 |     volts: f32,
   |     ^^^^^^^^^^ the trait `Eq` is not implemented for `f32`
```

Fix: derive only `PartialEq`. If the type must be a map key, store something that
is `Eq`, for example an integer number of millivolts.

**Writing `eq` by hand but deriving `Hash`.** The derived `Hash` uses all fields,
so `1/2` and `2/4` would be equal with different hashes. `rustc` accepts this. With
the worked example changed to `#[derive(Hash)]`, the set above normally ends with
two elements. `cargo clippy` refuses the code:

```text
error: you are deriving `Hash` but have implemented `PartialEq` explicitly
  |
1 | #[derive(Debug, Clone, Copy, Hash)]
  |                              ^^^^
  |
note: `PartialEq` implemented here
```

Fix: write `Hash` by hand from the same identity that `eq` compares.

**Comparing through a different number of references.** `==` needs an impl for the
exact pair of types:

```text
error[E0277]: can't compare `&&{integer}` with `{integer}`
  |
3 |     let sixes = rolls.iter().filter(|roll| roll == 6).count();
  |                                                 ^^ no implementation for `&&{integer} == {integer}`
  |
help: consider dereferencing here
  |
3 |     let sixes = rolls.iter().filter(|roll| **roll == 6).count();
  |                                            ++
```

Fix: dereference one side, or borrow the other, until both sides have the same
type.

### When to use which

- All fields count: `#[derive(PartialEq, Eq)]`, and `Hash` in the same derive.
- Identity is a subset or a normalized form: write `PartialEq` by hand, and `Hash`
  by hand from the same identity.
- `==` is not reflexive for some value: implement `PartialEq` only, never `Eq`.

**Read more:** [`PartialEq`](https://doc.rust-lang.org/std/cmp/trait.PartialEq.html) ·
[`Eq`](https://doc.rust-lang.org/std/cmp/trait.Eq.html) ·
[`Hash` and `Eq`](https://doc.rust-lang.org/std/hash/trait.Hash.html#hash-and-eq) ·
[`f64`](https://doc.rust-lang.org/std/primitive.f64.html)

## Lesson 03 · Partial and total comparisons

`<`, `<=`, `>` and `>=` call `PartialOrd::partial_cmp`, which returns
`Option<Ordering>`. `Ordering` is an enum with the variants `Less`, `Equal` and
`Greater`. The answer `None` means "these two values cannot be ordered", and then
all four operators give `false`.

`Ord::cmp` returns a plain `Ordering`. A type with `Ord` has a **total order**:
every pair of values has an answer. `sort`, `min`, `max`, `binary_search` and the
keys of a `BTreeMap` need `Ord`.

The four comparison traits form a ladder. Each trait requires the ones it points
to:

```text
PartialEq  ◄──  Eq             ==  !=       Eq adds: a == a, always
    ▲            ▲
PartialOrd ◄──  Ord            <  <=  >  >=  Ord adds: every pair is ordered
```

They must also tell one story. `a.cmp(&b)` is `Equal` exactly when `a == b`, and
`a.partial_cmp(&b)` is `Some(a.cmp(&b))`. If they disagree, nothing is unsafe, but
sorting and map lookups give results that nobody can predict, and a sort may
panic.

`#[derive(PartialOrd, Ord)]` compares the fields of a struct from top to bottom:
the first field that differs decides. For an enum, a variant written earlier is
smaller than a variant written later. (If you give the variants explicit numbers,
such as `Low = 5`, the numbers decide.) If you can put the fields in the order
that you want compared, the derive is all you need.

```rust
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    major: u32,
    minor: u32,
}

assert!(Version { major: 1, minor: 9 } < Version { major: 2, minor: 0 });
assert!(Version { major: 2, minor: 0 } < Version { major: 2, minor: 3 });
```

Tuples, vectors, slices and strings already have an order of this kind. They compare
element by element, and the first difference decides. If one value is the start
of the other, the shorter one is smaller. A string compares its UTF-8 bytes, so
its order follows byte values. That is not always the order of a dictionary.

```rust
assert!((1, "b") < (2, "a"));
assert!(vec![1, 2, 9] < vec![1, 3]);
assert!("car" < "cart" && "cart" < "cat");
```

### Worked example: an order for one sort, and a truly partial order

Often you do not need the traits at all, only an order for one call. `sort_by`
takes a closure that returns an `Ordering`. `then_with` asks a second question
only when the first answer is `Equal`:

```rust
use std::cmp::Reverse;

#[derive(Debug, Clone, PartialEq)]
struct Player {
    name: &'static str,
    score: u32,
}

let ana = Player { name: "ana", score: 70 };
let bo = Player { name: "bo", score: 90 };
let cy = Player { name: "cy", score: 70 };

// Highest score first; equal scores in alphabetical order.
let mut board = vec![cy.clone(), ana.clone(), bo.clone()];
board.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(b.name)));
assert_eq!(board, [bo.clone(), ana.clone(), cy.clone()]);

// The same order from a key. `Reverse` flips the order of what it wraps.
let mut again = vec![cy.clone(), bo.clone(), ana.clone()];
again.sort_by_key(|p| (Reverse(p.score), p.name));
assert_eq!(again, board);
```

Some orders really are partial. A time interval is "before" another one only when
it ends before the other starts. Two intervals that overlap are neither before nor
after each other:

```rust
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Interval {
    start: u32,
    end: u32, // never smaller than `start`
}

impl PartialOrd for Interval {
    fn partial_cmp(&self, other: &Interval) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if self.end < other.start {
            Some(Ordering::Less)
        } else if other.end < self.start {
            Some(Ordering::Greater)
        } else {
            None // they overlap
        }
    }
}

fn main() {
    let breakfast = Interval { start: 7, end: 8 };
    let meeting = Interval { start: 9, end: 11 };
    let call = Interval { start: 10, end: 12 };
    assert!(breakfast < meeting);
    assert_eq!(meeting.partial_cmp(&call), None);
    assert!(!(meeting < call) && !(meeting > call) && meeting != call);
}
```

`Interval` has no `Ord`, so `sort()` does not compile for it. That is correct: a
list of overlapping intervals has no single sorted order.

Floats are the partial order you meet most often. `partial_cmp` gives `None` when
either side is NaN. When you need a total order for floats, use `f64::total_cmp`.
It gives every float a fixed place, NaN included, and it also separates some
values that `==` calls equal.

### Mistakes people make

**Sorting floats with `sort()`.**

```text
error[E0277]: the trait bound `{float}: Ord` is not satisfied
   |
 3 |     laps.sort();
   |          ^^^^ the trait `Ord` is not implemented for `{float}`
```

Fix: `laps.sort_by(|a, b| a.total_cmp(b))`. The other common fix,
`a.partial_cmp(b).unwrap()`, panics as soon as the data contains a NaN.

**Writing `partial_cmp` a second time for a type that has `Ord`.** Here `cmp`
looks at two fields and `partial_cmp` at one, so `<` and `sort` disagree.
`cargo clippy` reports it:

```text
warning: non-canonical implementation of `partial_cmp` on an `Ord` type
   |
15 | / impl PartialOrd for Ticket {
16 | |     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
17 | |         self.number.partial_cmp(&other.number)
18 | |     }
19 | | }
   | |_^
```

Fix: put the logic in `cmp` only, and let `partial_cmp` wrap its answer in `Some`.

**Letting `cmp` and `partial_cmp` call each other.** If `partial_cmp` returns
`Some(self.cmp(other))` and `cmp` returns `self.partial_cmp(other).unwrap()`, each
method calls the other without end. The compiler gives no warning. A debug build
stops with `has overflowed its stack`. One of the two must compare the fields.

### When to use which

- The field order is the order you want: derive all four traits together.
- The order belongs to the type, but is not the field order: implement `Ord` by
  hand, delegate `PartialOrd` to it, and keep `==` in agreement.
- The order is needed in one place only: `sort_by`, `sort_by_key`, `max_by_key`,
  with `Reverse` for descending order.
- Some pairs cannot be compared: implement `PartialOrd` only.

**Read more:** [`Ord`](https://doc.rust-lang.org/std/cmp/trait.Ord.html) ·
[`PartialOrd`](https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html) ·
[`Ordering`](https://doc.rust-lang.org/std/cmp/enum.Ordering.html) ·
[`Reverse`](https://doc.rust-lang.org/std/cmp/struct.Reverse.html) ·
[`f64::total_cmp`](https://doc.rust-lang.org/std/primitive.f64.html#method.total_cmp)

## Lesson 04 · Index and IndexMut

`container[index]` is also a method call, with one extra step. The compiler
rewrites it to `*container.index(index)`, or to `*container.index_mut(index)` when
the element is assigned to or borrowed mutably:

```rust
// The shapes of `std::ops::Index` and `IndexMut`, under other names.
trait MyIndex<Idx> {
    type Output: ?Sized;
    fn index(&self, index: Idx) -> &Self::Output;
}
trait MyIndexMut<Idx>: MyIndex<Idx> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output;
}
```

Both methods return a **reference**, and the compiler adds the `*`. So
`container[index]` is a *place*: the element itself, inside the container. You can
read it, borrow it with `&container[index]`, or assign to it. This design has
three consequences.

1. An `index` method cannot compute a new value and return it. It can only point at
   something that the container already stores.
2. `Idx` is a type parameter, so one container can accept several index types: a
   number, a range, a key, an enum.
3. There is no way to answer "absent". An index that does not exist must panic.
   For a caller who is not sure, offer a `get` method that returns an `Option`.

`Output: ?Sized` means the output may be a type without a fixed size. That is how
`&numbers[1..3]` can be a slice, `&[T]`.

### Worked example: a table indexed by an enum

```rust
use std::ops::{Index, IndexMut};

#[derive(Debug, Clone, Copy)]
enum Day {
    Mon,
    Tue,
    Wed,
}

/// Hours worked on each day.
struct Hours([u32; 3]);

impl Index<Day> for Hours {
    type Output = u32;
    fn index(&self, day: Day) -> &u32 {
        // `Day::Mon as usize` is 0, `Tue` is 1, `Wed` is 2: always in range.
        &self.0[day as usize]
    }
}

impl IndexMut<Day> for Hours {
    fn index_mut(&mut self, day: Day) -> &mut u32 {
        &mut self.0[day as usize]
    }
}

fn main() {
    let mut week = Hours([8, 8, 6]);
    week[Day::Wed] = 4; //            *week.index_mut(Day::Wed) = 4
    week[Day::Mon] += 1; //           index_mut again
    let total = week[Day::Mon] + week[Day::Tue] + week[Day::Wed]; // index
    assert_eq!(total, 21);

    let wednesday: &u32 = &week[Day::Wed];
    assert!(std::ptr::eq(wednesday, &week.0[2]), "a place inside `week`");
}
```

```text
week ──► [ 9 | 8 | 4 ]
                   ▲
&week[Day::Wed] ───┘     a reference into the array, not a copy
```

An enum index cannot be out of range, so this type never panics. When the index is
a number, check it and panic with a message that names the problem. Check each
part of a compound index by itself: a position computed from two coordinates can
be inside the storage even when one coordinate is wrong.

A grid is a common case. It is usually stored *row-major*: one flat array holds
all of row 0, then all of row 1, and so on. Here are the positions in the flat
array for a grid with 3 rows and 4 columns:

```text
         col 0   col 1   col 2   col 3
row 0      0       1       2       3
row 1      4       5       6       7
row 2      8       9      10      11
```

To find the position of a cell, count the cells that are stored before it: every
cell of the rows above it, then the cells to its left in its own row. The
coordinate (1, 4) does not exist, because there is no column 4. The same counting
still gives it position 8, which is a real position: the one of cell (2, 0). A
check of the position alone would accept it.

### Mistakes people make

**Returning a computed value from `index`.**

```text
error[E0515]: cannot return reference to temporary value
  |
8 |         &(n * n)
  |         ^-------
  |         ||
  |         |temporary value created here
  |         returns a reference to data owned by the current function
```

Fix: `Index` is the wrong tool for computed values. Write a method that returns
the value.

**Moving an element out through an index.** `names[0]` is the element itself, so
`let first = names[0];` tries to move it out of the vector:

```text
error[E0507]: cannot move out of index of `Vec<String>`
  |
3 |     let first = names[0];
  |                 ^^^^^^^^ move occurs because value has type `String`, which does not implement the `Copy` trait
  |
help: consider borrowing here
  |
3 |     let first = &names[0];
  |                 +
```

Fix: borrow (`&names[0]`), clone, or take the element out with a method such as
`swap_remove`. For a `Copy` output, such as the `u32` above, the read is a copy
and compiles.

**Assigning through an index without `IndexMut`.** With only the `Index` impl
from the worked example:

```text
error[E0594]: cannot assign to data in an index of `Hours`
   |
17 |     week[Day::Wed] = 4;
   |     ^^^^^^^^^^^^^^^^^^ cannot assign
   |
   = help: trait `IndexMut` is required to modify indexed content, but it is not implemented for `Hours`
```

Fix: implement `IndexMut` with the same mapping as `Index`. `HashMap` gives this
error on purpose: it has `Index` but no `IndexMut`, so you insert with `insert`.

### When to use which

- `Index`: the caller knows the element exists, and a wrong index is a bug.
- `get` returning `Option<&T>`: absence is a normal case. Many types offer both.
- A named method: the result is computed, or the index has no obvious meaning.

**Read more:** [`Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) ·
[`IndexMut`](https://doc.rust-lang.org/std/ops/trait.IndexMut.html) ·
[index expressions in the Rust Reference](https://doc.rust-lang.org/reference/expressions/array-expr.html#array-and-slice-indexing-expressions)

## Lesson 05 · Capstone: a small bit-set API

The capstone gives you a contract and no new idea. You put the chapter together
in one small type: a set of the numbers 0 to 7, stored in the eight bits of a
`u8`. Bit *n* is 1 when *n* is in the set.

**The approach.** Wrap the integer in a struct with one field (a *newtype*). The
newtype has no operators of its own, so you choose which ones exist and what they
mean. For a set, the bit operators have natural meanings, and each of your
methods can hand the work to the same operator on the integer inside:

```text
set operation        bits                        operator
union                1 where either side is 1    |
intersection         1 where both sides are 1    &
complement           every bit flipped           !
```

Use the field (`self.0`) inside each method. `self | rhs` inside `bitor` is the
recursion mistake from Lesson 01.

**A different example of the same approach.** Here is a 16-bit mask with two
operators that the capstone does not ask for: `^` for "in exactly one of the
two", and `-` for "in the left set but not in the right set".

```rust
use std::ops::{BitXor, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Mask16(u16);

impl BitXor for Mask16 {
    type Output = Mask16;
    fn bitxor(self, rhs: Mask16) -> Mask16 {
        Mask16(self.0 ^ rhs.0)
    }
}

impl Sub for Mask16 {
    type Output = Mask16;
    fn sub(self, rhs: Mask16) -> Mask16 {
        Mask16(self.0 & !rhs.0)
    }
}

assert_eq!(Mask16(0b0110) ^ Mask16(0b0011), Mask16(0b0101));
assert_eq!(Mask16(0b0110) - Mask16(0b0011), Mask16(0b0100));
```

`PartialEq`, `Eq` and `Hash` are all derived from the single field, so they agree
with each other without any work from you.

**The one trap: shifting.** To test bit *n* you build a value with only that bit
set, by shifting 1 to the left. A shift by the full width of the type, or more,
is an overflow. A debug build panics. A release build does not: it uses only the
low bits of the shift amount, and you get an answer for a different bit.
`checked_shl` shows where the limit is:

```rust
assert_eq!(1u16.checked_shl(15), Some(0b1000_0000_0000_0000));
assert_eq!(1u16.checked_shl(16), None);
```

So decide what an out-of-range bit number means before you shift. The capstone's
contract tells you the answer it wants.

**Why a method and not `set[n]`?** Lesson 04 gives the reason: `Index` must return
a reference to something stored, and no `bool` is stored for each bit.

**Read more:** [`BitOr`](https://doc.rust-lang.org/std/ops/trait.BitOr.html) ·
[`BitAnd`](https://doc.rust-lang.org/std/ops/trait.BitAnd.html) ·
[`Not`](https://doc.rust-lang.org/std/ops/trait.Not.html) ·
[`BitOrAssign`](https://doc.rust-lang.org/std/ops/trait.BitOrAssign.html) ·
[`u8::checked_shl`](https://doc.rust-lang.org/std/primitive.u8.html#method.checked_shl)
