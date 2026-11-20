# Chapter 23 concepts — Unsafe Rust

[Chapter guide](README.md) · [Hints](HINTS.md) · [Labs](LABS.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every code block in this file was compiled and
run with the course toolchain (Rust 1.99.0, edition 2024), and every compiler
message was copied from that compiler. Long messages are shortened: a line with
`...` marks a cut in the middle, and the end of a message is often left out.
Reports from Miri come from the nightly toolchain named in [LABS.md](LABS.md),
Lab 1.

## What you will be able to do

After this chapter you can:

- say exactly which operations `unsafe` permits, and which checks stay active;
- write an `unsafe fn` with a `# Safety` contract, and a safe function that
  fulfils that contract for its callers;
- read and write memory through raw pointers, rebuild slices from a pointer
  and a length, and prove that two `&mut` do not overlap;
- move values by hand with `ptr::read` and `MaybeUninit` without dropping a
  value twice;
- explain why `Cell` needs `UnsafeCell`, and what `repr(C)`,
  `repr(transparent)` and `repr(packed)` promise;
- write a SAFETY comment that another person can check, and run Miri to find
  what tests miss.

## Before you start

Chapters 3 to 5: integer and slice types, ownership,
moves, references and lifetimes. Chapter 13: `Drop`, `Copy` and `Clone`. You
should be able to explain why Rust forbids two `&mut` to the same value. This
chapter does not need threads; where a thread appears, the text says the one
thing you need to know.

## Time

These are estimates for a learner who has finished the earlier
chapters. They include reading the section, the predictions, the code, and
your notes.

| Lesson | Estimate |
| --- | --- |
| 01 · A safe invariant boundary | 45 minutes |
| 02 · Raw pointer reads with contracts | 45 minutes |
| 03 · Raw slice reconstruction | 60 minutes |
| 04 · A flagged borrow | 50 minutes |
| 05 · Layout, unsafe traits, unions | 60 minutes |
| 06 · Moving values and initialization | 60 minutes |
| 07 · A fixed-capacity byte gap buffer | 75 minutes |
| 09 · Interior mutability and layout attributes | 75 minutes |
| 08 · Capstone | 60 minutes |
| R01 review, C01 code review, drills | 120 minutes |
| Lab 1 (Miri) | 45 minutes |

## Words used in this chapter

- **Undefined behavior (UB)**: something a program must never do, such as
  reading memory that was freed. The compiler assumes that it never happens.
  If it does happen, the language makes no promise about the result: the
  program may crash, give wrong answers, or seem to work.
- **Raw pointer**: a value of type `*const T` or `*mut T`. It holds an
  address, like a reference, but the compiler checks nothing about it.
- **Allocation**: one block of memory that was obtained as a unit: one local
  variable, one heap buffer, one static.
- **Aligned**: an address is aligned for `T` if it is a multiple of
  `align_of::<T>()`.
- **Initialized**: the bytes were written and form a valid value of the type.
- **Invariant**: a fact about a type's private data that is true whenever
  code outside the type can observe it.
- **Sound**: a function or type is sound if no safe code that uses it can
  cause undefined behavior, whatever that code does.
- **SAFETY comment**: the comment above an `unsafe` block that says why the
  operation is valid here.

## Lesson 01 · A safe invariant boundary

### The idea

In safe Rust the compiler proves that your program has no undefined behavior.
Some correct programs contain a step that the compiler cannot prove. For these
steps Rust has the keyword `unsafe`. It does not mean "dangerous code". It
means: "the compiler cannot check this step, so I checked it".

Inside an `unsafe` block you may do a few things that are forbidden elsewhere.
The main ones are:

1. dereference a raw pointer;
2. call a function that is declared `unsafe fn`;
3. read or write a `static mut`;
4. read a field of a `union`;
5. and, with `unsafe impl`, implement a trait that is declared `unsafe trait`.

Nothing else changes. Types are still checked. The borrow checker still works.
Lifetimes still apply. `unsafe` adds these tools; it removes no rule.

The usual shape of good unsafe code is a **safe boundary**: a type with a
private field, an invariant about that field, and public functions that are
all safe to call. The unsafe step inside relies on the invariant. Callers
cannot break the invariant, because they cannot reach the field.

### A worked example

A roster always has at least one name. Because of that, asking for the first
name never needs a bounds check.

```rust
mod roster {
    /// Invariant: `names` is never empty.
    pub struct Roster {
        names: Vec<String>,
    }

    impl Roster {
        pub fn new(first: String) -> Self {
            Roster { names: vec![first] }
        }

        pub fn add(&mut self, name: String) {
            self.names.push(name);
        }

        pub fn leader(&self) -> &str {
            // SAFETY: `get_unchecked(0)` requires that index 0 is in bounds.
            // The invariant says the vector is never empty: `new` starts
            // with one name, `add` only pushes, and the field is private,
            // so no code outside this module can remove a name.
            unsafe { self.names.get_unchecked(0) }
        }
    }
}

fn main() {
    let mut team = roster::Roster::new(String::from("Amara"));
    team.add(String::from("Ben"));
    assert_eq!(team.leader(), "Amara");
}
```

```text
        any caller, also a careless one
             │   new, add, leader   (all safe to call)
   ┌─────────▼────────────────────────────────┐
   │ module roster                            │
   │   private field   names: Vec<String>     │
   │   invariant       names.len() >= 1       │
   │   unsafe step     get_unchecked(0)       │  relies on the invariant
   └──────────────────────────────────────────┘
```

The `unsafe` block is one line, but the proof covers the whole module. Add
this function, which contains no `unsafe` at all, and `leader` can read
memory that is not there:

```rust
pub struct Roster {
    names: Vec<String>,
}

impl Roster {
    pub fn clear(&mut self) {
        self.names.clear(); // breaks the invariant
    }
}
```

So when you review unsafe code, you review every function that can touch the
private data, not only the block.

### Mistakes people make

**Using an unsafe operation outside a block.** The compiler tells you which
operation it is:

```rust,compile_fail,E0133
fn main() {
    let reading = 21_i32;
    let p = &raw const reading;
    let copy = *p;
    println!("{copy}");
}
```

```text
error[E0133]: dereference of raw pointer is unsafe and requires unsafe block
 --> src/main.rs:4:16
  |
4 |     let copy = *p;
  |                ^^ dereference of raw pointer
  |
  = note: raw pointers may be null, dangling or unaligned; they can violate aliasing rules and cause data races: all of these are undefined behavior
```

The fix is `let copy = unsafe { *p };` with a SAFETY comment: `p` was made
from a reference to a live variable.

**Believing that `unsafe` switches off the borrow checker.** It does not:

```rust,compile_fail,E0499
fn main() {
    let mut scores = vec![3, 5];
    unsafe {
        let first = &mut scores;
        let second = &mut scores;
        first.push(1);
        second.push(2);
    }
}
```

```text
warning: unnecessary `unsafe` block
...
error[E0499]: cannot borrow `scores` as mutable more than once at a time
 --> src/main.rs:5:22
  |
4 |         let first = &mut scores;
  |                     ----------- first mutable borrow occurs here
5 |         let second = &mut scores;
  |                      ^^^^^^^^^^^ second mutable borrow occurs here
6 |         first.push(1);
  |         ----- first borrow later used here
```

**Making the block large.** Put only the unsafe operation in the block. A
large block hides which line needs the proof.

### When to use which

Use the checked function (`names[0]`, `names.first()`, `str::from_utf8`)
unless you have a reason. Good reasons are rare: a measured cost in a hot
loop, or an operation that safe Rust cannot express at all. In this chapter
you write the unchecked form on purpose, to practise the proof.

### Read more

- [The Rust Reference: Unsafety](https://doc.rust-lang.org/reference/unsafety.html)
- [The Rust Reference: Behavior considered undefined](https://doc.rust-lang.org/reference/behavior-considered-undefined.html)
- [The Rustonomicon: Working with Unsafe](https://doc.rust-lang.org/nomicon/working-with-unsafe.html)

## Lesson 02 · Raw pointer reads with contracts

### The idea

A reference `&T` is an address plus guarantees: it is not null, it is
aligned, the value behind it is alive and initialized, and nobody changes the
value while the reference lives. A raw pointer `*const T` or `*mut T` is the
address without the guarantees. It has no lifetime. It may be null. It may be
**dangling**, which means that the value it pointed to is gone.

Making a raw pointer is safe, because an address alone can do no harm:

- `&raw const x`, `&x as *const T`, or `std::ptr::from_ref(&x)`;
- `slice.as_ptr()`;
- `p.wrapping_add(n)`, which computes another address with no promise;
- `p.cast::<U>()`, which keeps the address and changes the type that the
  pointer claims to point to. The rules below then apply to `U`.

Using it is unsafe. Before you read a `T` through `p`, all of this must be
true:

1. `p` is not null and is aligned for `T`.
2. The `size_of::<T>()` bytes at `p` lie inside one allocation that is still
   alive, and `p` was derived from a pointer or reference to that allocation.
3. The bytes are an initialized, valid `T`.
4. Nobody writes to those bytes during the read, and no `&mut` to them is in
   use.

The method `p.add(n)` is also unsafe. It promises the compiler that the
result stays inside the same allocation, or exactly one element past its end.
Breaking that promise is undefined behavior even if you never read.

A function that needs such promises from its caller is an `unsafe fn`. Its
documentation has a `# Safety` section that lists what the caller must
guarantee. Each caller answers with a SAFETY comment.

### A worked example

`sum_raw` cannot check its pointer, so it states a contract. `sum` is the
safe wrapper: a slice already guarantees everything the contract asks for.

```rust
/// Adds up `count` values starting at `first`.
///
/// # Safety
/// `first` must point to `count` initialized `i32` values in one live
/// allocation, and nothing may write to them during the call.
unsafe fn sum_raw(first: *const i32, count: usize) -> i64 {
    let mut total = 0_i64;
    for i in 0..count {
        // SAFETY: `i < count`, and the caller promises that `count` values
        // starting at `first` are in bounds, initialized and not changing.
        total += i64::from(unsafe { *first.add(i) });
    }
    total
}

fn sum(values: &[i32]) -> i64 {
    // SAFETY: `as_ptr` and `len` describe exactly the elements of `values`.
    // The shared borrow keeps them alive and unchanged during the call.
    unsafe { sum_raw(values.as_ptr(), values.len()) }
}

fn main() {
    assert_eq!(sum(&[4, -1, 10]), 13);
    assert_eq!(sum(&[]), 0);
}
```

```text
 values: &[i32] ──►  [  4  | -1  | 10  ]      one allocation, three elements
                      ▲     ▲     ▲     ▲
        first.add(n): 0     1     2     3     3 is "one past the end":
                                              you may compute it, never read it
```

A null pointer can be tested. `as_ref` turns a pointer that is either null or
valid into an `Option`:

```rust
fn main() {
    let port = 8080_u16;
    let present: *const u16 = &port;
    let missing: *const u16 = std::ptr::null();
    // SAFETY: each pointer is either null or points to a live u16.
    let (a, b) = unsafe { (present.as_ref(), missing.as_ref()) };
    assert_eq!((a, b), (Some(&8080), None));
}
```

### Mistakes people make

**Treating `add` as harmless arithmetic.** It is an unsafe function:

```rust,compile_fail,E0133
fn main() {
    let samples = [10_i32, 20, 30];
    let second = samples.as_ptr().add(1);
    println!("{second:p}");
}
```

```text
error[E0133]: call to unsafe function `std::ptr::const_ptr::<impl *const T>::add` is unsafe and requires unsafe block
 --> src/main.rs:3:18
  |
3 |     let second = samples.as_ptr().add(1);
  |                  ^^^^^^^^^^^^^^^^^^^^^^^ call to unsafe function
```

Put the call in an `unsafe` block and say why the offset stays in bounds. If
you only want an address and make no promise, use `wrapping_add`.

**Leaving the body of an `unsafe fn` unmarked.** `unsafe fn` restricts the
caller. Inside the function, each unsafe operation still wants its own block.
In edition 2024 the compiler warns:

```text
warning[E0133]: dereference of raw pointer is unsafe and requires unsafe block
 --> src/main.rs:4:5
  |
4 |     *p
  |     ^^ dereference of raw pointer
...
note: an unsafe function restricts its caller, but its body is safe by default
...
  = note: `#[warn(unsafe_op_in_unsafe_fn)]` (part of `#[warn(rust_2024_compatibility)]`) on by default
```

**"It is not null, so I may read it."** Null is only one of the ways a
pointer can be invalid. A pointer to a local variable of a function that has
returned is not null, and reading it is undefined behavior. No compiler
message warns you. The only defence is to know where the pointer came from.

### When to use which

- `&raw const x` when you want a pointer and no reference in between; `as`
  casts and `ptr::from_ref` when you already hold a reference.
- `add` when you can prove that the result is in bounds (this lets the
  compiler optimize); `wrapping_add` when you cannot or need not.
- At work, prefer `slice.get(i)`. The raw form is for code that has no slice.

### Read more

- [std: the pointer primitive type](https://doc.rust-lang.org/std/primitive.pointer.html)
- [std::ptr: safety rules for pointers](https://doc.rust-lang.org/std/ptr/index.html#safety)
- [The Rust Reference: the `unsafe` keyword](https://doc.rust-lang.org/reference/unsafe-keyword.html)
- [The Edition Guide: unsafe operations in unsafe functions](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html)

## Lesson 03 · Raw slice reconstruction

### The idea

A slice reference is two numbers: the address of the first element and the
number of elements. `std::slice::from_raw_parts(ptr, len)` builds a `&[T]`
from those two numbers, and `from_raw_parts_mut` builds a `&mut [T]`. Both
are unsafe, because you promise everything that a slice reference guarantees.
It helps to check six points, one at a time:

1. **Bounds.** All `len` elements lie inside one allocation.
2. **Provenance.** The pointer was derived from a pointer or reference that
   is allowed to use all of those elements.
3. **Alignment.** The pointer is aligned for `T`, and it is not null. This
   holds even when `len` is 0.
4. **Initialization.** All `len` elements are valid values of `T`.
5. **Lifetime.** The memory stays alive, and for `&mut` stays unused by
   others, for the whole lifetime you give the result.
6. **Non-overlap.** A `&mut` is exclusive. No other reference may reach the
   same elements while it lives.

Point 6 is the one people get wrong. Two `&mut` that overlap are undefined
behavior from the moment both exist. It does not matter whether you write
through them.

The borrow checker enforces point 6 in safe code, but it reasons about whole
variables. It cannot see that two parts of one slice are different:

```rust,compile_fail,E0499
fn main() {
    let mut row = [1, 2, 3, 4];
    let left = &mut row[..2];
    let right = &mut row[2..];
    left[0] = 9;
    right[0] = 8;
}
```

```text
error[E0499]: cannot borrow `row` as mutable more than once at a time
 --> src/main.rs:4:22
  |
3 |     let left = &mut row[..2];
  |                     --- first mutable borrow occurs here
4 |     let right = &mut row[2..];
  |                      ^^^ second mutable borrow occurs here
5 |     left[0] = 9;
  |     ------- first borrow later used here
  |
  = help: use `.split_at_mut(position)` to obtain two mutable non-overlapping sub-slices
```

The help text names a std function that solves this. Inside, it uses raw
pointers and a proof. In this lesson you write such a function yourself.

### A worked example

`pair_mut` gives mutable access to two different elements at once.

```rust
fn pair_mut<T>(items: &mut [T], i: usize, j: usize) -> Option<(&mut T, &mut T)> {
    if i == j || i >= items.len() || j >= items.len() {
        return None;
    }
    let start = items.as_mut_ptr();
    // SAFETY: both indices are in bounds, so both pointers point to
    // initialized elements of the slice that `start` came from. `i != j`,
    // so the two `&mut T` refer to different elements and do not overlap.
    // Both results borrow from `items`, which cannot be used while they live.
    unsafe { Some((&mut *start.add(i), &mut *start.add(j))) }
}

fn main() {
    let mut levels = [1, 2, 3];
    let (low, high) = pair_mut(&mut levels, 0, 2).unwrap();
    std::mem::swap(low, high);
    assert_eq!(levels, [3, 2, 1]);
    assert!(pair_mut(&mut levels, 1, 1).is_none());
}
```

```text
 items: &mut [T] ──►  [ 1 | 2 | 3 ]
                        ▲       ▲
               start.add(0)   start.add(2)     two different elements:
                  &mut T        &mut T         the two borrows do not overlap
```

### Mistakes people make

**Forgetting one case of the overlap check.** Remove `i == j ||` from
`pair_mut` and call `pair_mut(&mut levels, 1, 1)`. The compiler accepts it.
A test that adds 10 through one reference and 100 through the other printed
`[1, 112, 3]` on our machine, in debug and in release, as if nothing were
wrong. Miri reports the real situation:

```text
error: Undefined Behavior: trying to retag from <565> for Unique permission at alloc205[0x4], but that tag does not exist in the borrow stack for this location
 --> src/main.rs:8:19
  |
8 |     unsafe { Some((&mut *start.add(i), &mut *start.add(j))) }
  |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
help: <565> was later invalidated at offsets [0x4..0x8] by a Unique retag
```

In plain words: the second `&mut` to the element made the first one invalid.

**Checking only one index.** If only `i` is checked, `start.add(j)` can leave
the allocation. Check every value that goes into `add`.

**Using a null pointer for an empty slice.** `from_raw_parts(ptr::null(), 0)`
is undefined behavior. For an empty slice use a pointer that is non-null and
aligned, for example `std::ptr::NonNull::<T>::dangling().as_ptr()`, or a
pointer into the original slice.

### When to use which

At work, use the safe functions: `split_at_mut`, `split_first_mut`,
`get_disjoint_mut`, `swap`, `chunks_mut`. They contain this kind of proof,
written and reviewed once. Write your own only for a split that std does not
offer.

### Read more

- [std::slice::from_raw_parts_mut](https://doc.rust-lang.org/std/slice/fn.from_raw_parts_mut.html)
- [slice::split_at_mut](https://doc.rust-lang.org/std/primitive.slice.html#method.split_at_mut)
- [The Rustonomicon: Splitting Borrows](https://doc.rust-lang.org/nomicon/borrow-splitting.html)

## Lesson 04 · A flagged borrow

### The idea

This lesson uses three facts.

**Alignment leaves address bits unused.** A valid pointer to a type with
alignment 4 has an address that is a multiple of 4. In binary, such an
address ends in `00`. Those bits carry no information, so a program can store
a small tag in them, if it removes the tag before it uses the pointer.

**A pointer is more than an address.** It also carries **provenance**: the
permission to use one particular allocation. Two pointers with the same
address but different provenance are not interchangeable. You cannot see
provenance in the bits, but the language rules and Miri track it. The methods
`addr()`, `map_addr(f)` and `with_addr(a)` read or change the address and
keep the provenance. A cast to an integer and back does not keep it in the
same way (see the mistakes below).

**A raw pointer has no lifetime.** A struct that holds only a raw pointer
tells the borrow checker nothing about the value it points to. The marker
type `PhantomData<&'a T>` fixes that. It occupies no memory. It tells the
compiler: "check this struct as if it held a `&'a T`".

### Worked examples

`map_addr` on its own: round an address down to a multiple of 16. Only the
address changes. Computing such a pointer is safe, like `wrapping_add`;
reading through it would need a proof.

```rust
fn main() {
    let block = [0_u8; 64];
    let inside: *const u8 = &block[37];
    let rounded = inside.map_addr(|addr| addr & !15);
    assert_eq!(rounded.addr() % 16, 0);
    assert!(inside.addr() - rounded.addr() < 16);
}
```

`PhantomData` on its own: a cursor that walks over a slice with two raw
pointers, in the style of `std::slice::Iter`.

```rust
use std::marker::PhantomData;

struct Cursor<'a, T> {
    next: *const T,
    end: *const T,
    // No bytes. Tells the borrow checker that we borrow a `T` for 'a.
    borrow: PhantomData<&'a T>,
}

impl<'a, T> Cursor<'a, T> {
    fn new(items: &'a [T]) -> Self {
        let range = items.as_ptr_range();
        Cursor { next: range.start, end: range.end, borrow: PhantomData }
    }

    fn step(&mut self) -> Option<&'a T> {
        if self.next == self.end {
            return None;
        }
        // SAFETY: `next` lies inside the slice that `new` borrowed for 'a,
        // and it is not the end, so it points to a live element.
        let item = unsafe { &*self.next };
        // SAFETY: `next != end`, so one step forward is at most one past
        // the end of the same slice.
        self.next = unsafe { self.next.add(1) };
        Some(item)
    }
}

fn main() {
    let temps = [18, 21, 19];
    let mut cursor = Cursor::new(&temps);
    assert_eq!(cursor.step(), Some(&18));
    assert_eq!(cursor.step(), Some(&21));
}
```

(This sketch ignores element types of size zero, which std handles
separately.)

### Mistakes people make

**Leaving out the marker.** Without a field that uses `'a`, the struct does
not compile:

```rust,compile_fail,E0392
struct Cursor<'a, T> {
    next: *const T,
    end: *const T,
}
```

```text
error[E0392]: lifetime parameter `'a` is never used
 --> src/lib.rs:1:15
  |
1 | struct Cursor<'a, T> {
  |               ^^ unused lifetime parameter
  |
  = help: consider removing `'a`, referring to it in a field, or using a marker such as `PhantomData`
```

Do not "fix" this by removing `'a`. Without the lifetime, a `Cursor` could
outlive the slice. With the marker, the compiler stops that mistake. A
function that tries to return `Cursor::new(&local)` gets:

```text
error[E0515]: cannot return value referencing local variable `local`
```

That error is the purpose of the marker. Keep it.

**Going through an integer.** `(p as usize | 1) as *const u32` gives a pointer
with the right address. The cast to `usize` *exposes* the provenance, and the
cast back asks for "some exposed provenance that fits". This is permitted, but
the compiler and Miri can no longer follow the pointer. Under Miri's strict
mode the program stops:

```text
error: unsupported operation: integer-to-pointer casts and `ptr::with_exposed_provenance` are not supported with `-Zmiri-strict-provenance`
```

`map_addr` avoids the question: the pointer never stops being a pointer.

**Dereferencing the tagged pointer.** An address with the low bit set is not
aligned for `u32`. Clear every tag bit first, then dereference.

### When to use which

Tagged pointers are a space optimization for special code, such as
interpreters and lock-free data structures. In ordinary code, store the flag
next to the reference: `(&u32, bool)` is clear and safe. `PhantomData` is
common: a type that borrows through a raw pointer needs it to carry the
lifetime.

### Read more

- [std::ptr: provenance](https://doc.rust-lang.org/std/ptr/index.html#provenance)
- [std::ptr: strict provenance](https://doc.rust-lang.org/std/ptr/index.html#strict-provenance)
- [pointer::map_addr](https://doc.rust-lang.org/std/primitive.pointer.html#method.map_addr)
- [std::marker::PhantomData](https://doc.rust-lang.org/std/marker/struct.PhantomData.html)
- [The Rustonomicon: PhantomData](https://doc.rust-lang.org/nomicon/phantom-data.html)

## Lesson 05 · Layout, unsafe traits, unions

### The idea: layout

Every type has a **size** (how many bytes a value occupies) and an
**alignment** (its address must be a multiple of this number). A struct puts
each field at an **offset** that is a multiple of that field's alignment, so
the compiler sometimes inserts unused bytes, called **padding**. The size of
a struct is a multiple of its alignment.

With the default layout, the compiler may order the fields as it likes, and
the order is not documented. `#[repr(C)]` fixes the order: fields appear as
you wrote them.

```rust
use std::mem::{align_of, offset_of, size_of};

#[repr(C)]
struct Sample {
    ready: u8,
    value: u64,
    id: u16,
}

#[repr(C)]
struct Reordered {
    value: u64,
    id: u16,
    ready: u8,
}

fn main() {
    assert_eq!(offset_of!(Sample, ready), 0);
    assert_eq!(offset_of!(Sample, value), align_of::<u64>());
    // The numbers in the diagram hold where u64 has alignment 8.
    if align_of::<u64>() == 8 {
        assert_eq!((size_of::<Sample>(), size_of::<Reordered>()), (24, 16));
    }
}
```

```text
Sample: 24 bytes, alignment 8
 bytes   0       1..8        8..16      16..18    18..24
        [ready] [padding]   [ value ]  [  id  ]  [padding]

Reordered: 16 bytes, alignment 8
 bytes   0..8       8..10    10       11..16
        [ value ]  [  id  ] [ready]  [padding]
```

Measure layout with `size_of`, `align_of` and `offset_of!`. Do not compute it
in your head and then rely on the result.

### The idea: valid values

Not every pattern of bits is a value of a type. A `bool` is the byte 0 or 1.
A `char` is not every `u32`. A reference is never null. An enum has only its
listed variants. Producing an invalid value is undefined behavior at once,
even if you never look at it. Integers and floats accept every pattern.
Padding bytes count as uninitialized.

### Unsafe traits

An `unsafe fn` puts the proof on the caller. An `unsafe trait` puts it on the
person who implements the trait. Generic unsafe code can then rely on the
promise for every type that has the bound.

```rust
/// # Safety
/// `index` must always return a value below 4.
unsafe trait Quadrant {
    fn index(&self) -> usize;
}

#[derive(Clone, Copy)]
enum Compass {
    North,
    East,
    South,
    West,
}

// SAFETY: the four variants have the discriminants 0, 1, 2 and 3.
unsafe impl Quadrant for Compass {
    fn index(&self) -> usize {
        *self as usize
    }
}

fn count<Q: Quadrant>(seen: &mut [u32; 4], item: &Q) {
    // SAFETY: `Q: Quadrant` promises `index() < 4`, the length of `seen`.
    unsafe { *seen.get_unchecked_mut(item.index()) += 1 };
}

fn main() {
    let mut seen = [0; 4];
    for wind in [Compass::East, Compass::West, Compass::East] {
        count(&mut seen, &wind);
    }
    assert_eq!(seen, [0, 2, 0, 1]);
}
```

If `Quadrant` were an ordinary trait, anyone could implement `index` to
return 99 in safe code, and `count` would be unsound. Unsafe code must never
trust a promise that safe code is able to break.

### Unions

A `union` stores all its fields in the same bytes. Writing a field is safe.
Reading one is unsafe, because the compiler does not know which field was
written last, and the read does not check the bits.

```rust
union Raw {
    number: u32,
    bytes: [u8; 4],
}

fn main() {
    let raw = Raw { number: 0x0102_0304 };
    // SAFETY: both fields are 4 bytes without padding, `number` initialized
    // all of them, and every byte pattern is a valid `[u8; 4]`.
    let bytes = unsafe { raw.bytes };
    assert_eq!(bytes, 0x0102_0304_u32.to_ne_bytes());
}
```

### Mistakes people make

**Implementing an unsafe trait without `unsafe`:**

```text
error[E0200]: the trait `Quadrant` requires an `unsafe impl` declaration
  --> src/main.rs:15:1
   |
15 | impl Quadrant for Compass {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: the trait `Quadrant` enforces invariants that the compiler can't check. Review the trait documentation and make sure this implementation upholds those invariants before adding the `unsafe` keyword
```

Read the note. The compiler asks for a review, not only for a keyword.

**Reading a union field outside `unsafe`:**

```text
error[E0133]: access to union field is unsafe and requires unsafe block
 --> src/main.rs:8:17
  |
8 |     let bytes = raw.bytes;
  |                 ^^^^^^^^^ access to union field
  |
  = note: the field may not be properly initialized: using uninitialized data will cause undefined behavior
```

**Reading a field that does not accept all patterns.** With a field
`flag: bool`, writing `number: 2` and then reading `flag` is undefined
behavior. There is no compiler message. Miri reports it.

### When to use which

For bits of numbers, use the safe methods: `to_ne_bytes`, `from_le_bytes`,
`f32::to_bits`. Use `repr(C)` when the layout is part of an agreement with
other code, such as a C library or a file format. Use an `enum` instead of a
`union` whenever Rust code decides what is stored.

### Read more

- [The Rust Reference: Type layout](https://doc.rust-lang.org/reference/type-layout.html)
- [The Rust Reference: Unions](https://doc.rust-lang.org/reference/items/unions.html)
- [std::mem::offset_of](https://doc.rust-lang.org/std/mem/macro.offset_of.html)
- [The Rustonomicon: Data layout](https://doc.rust-lang.org/nomicon/data.html)

## Lesson 06 · Moving values and initialization

### The idea

In Chapter 4 a move copied the header of a value to a new place, and the
compiler then treated the old place as empty. With raw pointers you do the
two halves yourself:

- `std::ptr::read(p)` copies the bits out of `*p` and gives you a `T`. The
  compiler does **not** treat `*p` as empty afterwards. For a moment there
  are two copies of one value. You must make sure that only one of them is
  ever used or dropped again.
- `std::ptr::write(p, value)` puts `value` into `*p` and does **not** drop
  what was there before. That is what you want for memory that holds no
  value yet. The assignment `*p = value` is different: it drops the old
  value first.

Two wrapper types tell the compiler to stay out of the way:

- `MaybeUninit<T>` is memory with room for a `T` that may hold no `T` yet.
  It never drops its contents. `write` fills it; `assume_init` and
  `assume_init_read` are your (unsafe) claim that it is filled.
- `ManuallyDrop<T>` holds a `T` whose destructor will not run on its own.

The error to fear is the **double drop**: two places both believe that they
own one value, and both drop it. For a `String` that frees one heap buffer
twice. The opposite error, never dropping a value, is a **leak**. A leak
wastes memory, but it is not undefined behavior.

### A worked example

A type with a `Drop` impl does not let you move a field out. The compiler is
right to refuse: after the move, `Drop` would see a struct with a hole.

```rust,compile_fail,E0509
struct Job {
    name: String,
    attempts: u32,
}

impl Drop for Job {
    fn drop(&mut self) {
        println!("job {} finished after {} attempts", self.name, self.attempts);
    }
}

fn into_name(job: Job) -> String {
    job.name
}
```

```text
error[E0509]: cannot move out of type `Job`, which implements the `Drop` trait
  --> src/lib.rs:13:5
   |
13 |     job.name
   |     ^^^^^^^^
   |     |
   |     cannot move out of here
   |     move occurs because `job.name` has type `String`, which does not implement the `Copy` trait
```

If the right behavior is "take the name and cancel the job's own cleanup",
you can do the move by hand:

```rust
use std::mem::ManuallyDrop;

struct Job {
    name: String,
    attempts: u32,
}

impl Drop for Job {
    fn drop(&mut self) {
        println!("job {} finished after {} attempts", self.name, self.attempts);
    }
}

fn into_name(job: Job) -> String {
    // From here on, nothing drops `job` automatically.
    let job = ManuallyDrop::new(job);
    // SAFETY: `job.name` is a live, initialized String. `job` is wrapped in
    // `ManuallyDrop` and is not used again, so after this read the String
    // has exactly one owner: the return value.
    unsafe { std::ptr::read(&job.name) }
}

fn main() {
    let job = Job { name: String::from("sync"), attempts: 2 };
    assert_eq!(into_name(job), "sync");
}
```

An ownership ledger makes the argument visible. Write one line per step:

```text
step                               who owns the text "sync"
let job = ManuallyDrop::new(job)   job.name (and job will never be dropped)
ptr::read(&job.name)               two copies of the header exist for a moment
return                             only the returned String: job is never touched again
```

If `Job` had a second `String` field, this function would leak it. The ledger
would show a line with no owner at the end.

`MaybeUninit` in its smallest form:

```rust
use std::mem::MaybeUninit;

fn main() {
    let mut slot = MaybeUninit::<String>::uninit(); // room for a String, no String yet
    slot.write(String::from("ready"));
    // SAFETY: the slot was written on the line above.
    let text = unsafe { slot.assume_init() };
    assert_eq!(text, "ready");
}
```

### The order rule

A container records which of its slots hold values, for example with a
length. A raw move and the update of that record are two steps. If something
panics between them, the container's destructor runs with a wrong record.
So: **no code that can panic, and no code supplied by the user, may run
between a raw move and the update of the record.** When you can choose,
update the record first. Then a panic can only leak.

For a `Vec`, the record is its length. The unsafe method `set_len(n)` changes
the length and touches no element: it drops nothing and initializes nothing.
You promise that exactly the first `n` slots hold values that the vector
should own.

### Mistakes people make

- **Reading a value out and leaving the record unchanged.** The container
  drops the value later, and so does the new owner. There is no compiler
  message. The program usually stops with a message from the allocator, for
  example `pointer being freed was not allocated`, or with `SIGABRT`.
- **Calling `assume_init` before every byte was written.** The claim is
  false, so this is undefined behavior even for a type like `u8`.
- **Assigning into memory that holds no value.** `*p = value` first drops the
  "old value", which is garbage. Use `ptr::write` or `MaybeUninit::write`.

### When to use which

Safe tools cover almost every case: `mem::replace`, `mem::take` and
`Option::take` (Chapter 4), `Vec::pop`, `Vec::swap_remove`, `Vec::drain`, and
`std::array::from_fn` for building an array. Raw moves are for code that
implements such tools.

### Read more

- [std::ptr::read](https://doc.rust-lang.org/std/ptr/fn.read.html) and [std::ptr::write](https://doc.rust-lang.org/std/ptr/fn.write.html)
- [std::mem::MaybeUninit](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html)
- [std::mem::ManuallyDrop](https://doc.rust-lang.org/std/mem/struct.ManuallyDrop.html)
- [The Rustonomicon: Exception Safety](https://doc.rust-lang.org/nomicon/exception-safety.html)

## Lesson 07 · A fixed-capacity byte gap buffer

### The idea

A text editor inserts and deletes at the cursor all the time. In a plain
`Vec<u8>`, every insertion in the middle shifts all later bytes. A **gap
buffer** avoids that. It keeps the free space of the buffer as one block, the
**gap**, exactly at the cursor. Two indices describe it: `start` is the first
free byte, `end` is the first used byte after the gap.

```text
storage: [ a | b | _ | _ | _ | d | e ]     text "abde", cursor between b and d
           0   1   2   3   4   5   6
                   ▲           ▲
                 start = 2   end = 5       the gap is storage[2..5]

insert 'c' at the cursor: one write, nothing else moves
         [ a | b | c | _ | _ | d | e ]     start = 3, end = 5: text "abcde"

move the cursor one step left: the byte before the gap jumps to its end
         [ a | b | _ | _ | c | d | e ]     start = 2, end = 4: text "abcde"
```

The text is everything before `start` followed by everything from `end`. The
bytes inside the gap are leftovers. They are still initialized `u8` values,
so in a byte buffer the gap needs no special care. (For a gap buffer of
`String`s it would: that is Lab 3.)

Moving the cursor by several positions copies a block of bytes from one side
of the gap to the other. The source block and the destination block lie in
the same buffer, and when the gap is small they overlap.

### Copying inside one buffer

Rust has two raw copy functions. Both take `(source, destination, count)`,
and `count` is a number of elements, not of bytes.

- `std::ptr::copy` works even when the two ranges overlap. The result is as
  if the source were first copied to a temporary buffer.
- `std::ptr::copy_nonoverlapping` requires that the ranges do not overlap.
  It is undefined behavior to call it on overlapping ranges.

### A worked example

Insert one byte into the used part of a line and shift the rest one place to
the right. The source and the destination overlap in all but one byte.

```rust
/// Inserts `byte` at `index` into the first `len` bytes of `line`.
/// Returns false if the line is full or the index is past the end.
fn insert_at(line: &mut [u8], len: usize, index: usize, byte: u8) -> bool {
    if len >= line.len() || index > len {
        return false;
    }
    let base = line.as_mut_ptr();
    // SAFETY: the source is `index..len` and the destination is
    // `index + 1..len + 1`. `len < line.len()`, so both ranges lie inside
    // `line`. They overlap, and `ptr::copy` allows that.
    unsafe { std::ptr::copy(base.add(index), base.add(index + 1), len - index) };
    line[index] = byte;
    true
}

fn main() {
    let mut line = *b"abde____";
    assert!(insert_at(&mut line, 4, 2, b'c'));
    assert_eq!(&line[..5], b"abcde");
    assert!(!insert_at(&mut line, 8, 0, b'!'));
}
```

```text
before:   a b d e _ _ _ _       copy 2 bytes from index 2 to index 3
              └─┴──►            source 2..4, destination 3..5: both contain index 3
after:    a b d d e _ _ _       then line[2] = 'c'  gives  a b c d e
```

### Mistakes people make

**Using `copy_nonoverlapping` because it sounds faster.** This program shifts
four bytes one place to the right with the wrong function:

```rust,no_run
fn main() {
    let mut line = *b"abcdef__";
    let p = line.as_mut_ptr();
    // WRONG: the ranges 2..6 and 3..7 overlap.
    unsafe { std::ptr::copy_nonoverlapping(p.add(2), p.add(3), 4) };
    println!("{}", String::from_utf8_lossy(&line));
}
```

A debug build has an extra check and stops:

```text
unsafe precondition(s) violated: ptr::copy_nonoverlapping requires that both pointer arguments are aligned and non-null and the specified memory ranges do not overlap

This indicates a bug in the program. This Undefined Behavior check is optional, and cannot be relied on for safety.
...
thread caused non-unwinding panic. aborting.
```

A release build (`-O`) has no such check. On our machine it printed
`abccdef_`, which is the answer you wanted. That is the worst result: the
program has undefined behavior and every test passes. Another compiler
version or another length may give a different result. Read the second line
of the message again: the check "cannot be relied on".

**Swapping source and destination.** The C function `memcpy` takes the
destination first. Rust's `ptr::copy` takes the source first.

**Counting bytes.** For `*mut u32`, a count of 4 means four `u32` values, 16
bytes.

### When to use which

`slice.copy_within(2..4, 3)` does the same work as the example and is safe:
use it at work. `copy_from_slice` copies between two different slices. The
raw functions are needed when the memory is not a plain slice of initialized
`Copy` values, as in Lab 3.

### Read more

- [std::ptr::copy](https://doc.rust-lang.org/std/ptr/fn.copy.html)
- [std::ptr::copy_nonoverlapping](https://doc.rust-lang.org/std/ptr/fn.copy_nonoverlapping.html)
- [slice::copy_within](https://doc.rust-lang.org/std/primitive.slice.html#method.copy_within)

## Lesson 09 · Interior mutability and layout attributes

This lesson has three short parts. Each part is one rule of the language that
unsafe code must know.

### Part 1: `UnsafeCell`, the only way to write behind `&`

A shared reference `&T` promises that the value does not change while the
reference lives. The compiler uses that promise when it optimizes. So turning
a `&T` into a `&mut T` is undefined behavior, and the compiler rejects the
obvious attempt even inside `unsafe`:

```text
error: casting `&T` to `&mut T` is undefined behavior, even if the reference is unused, consider instead using an `UnsafeCell`
 --> src/main.rs:4:27
  |
4 |     let sneaky = unsafe { &mut *(shared as *const u32 as *mut u32) };
  |                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
  = note: `#[deny(invalid_reference_casting)]` on by default
```

The message names the one exception. `std::cell::UnsafeCell<T>` tells the
compiler: "the contents may change even behind a shared reference". Its
method `get(&self)` returns a `*mut T`. Every type with interior mutability
(`Cell`, `RefCell`, `Mutex`, the atomic types) contains an `UnsafeCell`.

`UnsafeCell` removes one promise. The other rules stay, and now you must keep
them: no `&mut` to the contents while any other reference to them is in use,
and no two threads that access the contents without synchronization.

```rust
use std::cell::UnsafeCell;

/// Counts events through a shared reference.
///
/// Invariant: no reference to `count` exists outside a method call.
struct Tally {
    count: UnsafeCell<u64>,
}

impl Tally {
    fn new() -> Self {
        Tally { count: UnsafeCell::new(0) }
    }

    fn record(&self) -> u64 {
        let slot: *mut u64 = self.count.get();
        // SAFETY: `slot` points to the live u64 inside `self`. No reference
        // to it exists (invariant), and `Tally` is not `Sync`, so no other
        // thread can be inside `record`. These are the only accesses.
        unsafe {
            *slot += 1;
            *slot
        }
    }
}

fn main() {
    let tally = Tally::new();
    let (a, b) = (&tally, &tally); // two shared references to one value
    a.record();
    assert_eq!(b.record(), 2);
}
```

The SAFETY comment uses two facts. **No reference escapes:** if `Tally` had a
method that returned `&u64`, a caller could hold it while `record` writes,
and the `&u64` promise would be broken. A cell hands out copies, not
references. **The type is not `Sync`:** a type is `Sync` if `&T` may be used
from several threads at once. With two threads in `record`, this can happen:

```text
thread 1 reads 5        thread 2 reads 5
thread 1 writes 6       thread 2 writes 6      one event is lost
```

Two unsynchronized accesses, at least one a write, are a **data race**, and a
data race is undefined behavior. `UnsafeCell` is not `Sync`, so every type
that contains one is not `Sync` either, and the compiler refuses to share it.
For std's `Cell` the refusal looks like this:

```text
error[E0277]: `Cell<u32>` cannot be shared between threads safely
   --> src/main.rs:5:21
    |
  5 |         scope.spawn(|| hits.set(hits.get() + 1));
    |               ----- ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `Cell<u32>` cannot be shared between threads safely
...
    = help: the trait `Sync` is not implemented for `Cell<u32>`
    = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicU32` instead
```

Writing `unsafe impl Sync` for such a type silences the compiler and keeps
the bug. It is correct only for a type that synchronizes by itself, as a
mutex does.

For a cell of any `T`, one more point matters: dropping the old value runs
user code, and that code may call the cell again. So a `set` first stores the
new value and gets the old one back, and only then drops the old one.

### Part 2: `repr(transparent)`

A struct with one field often has the same layout as that field, but with
the default layout this is not promised. `#[repr(transparent)]` promises it:
the same size, the same alignment, and the same way of being passed to
functions. With the promise, a pointer to the field may be cast to a pointer
to the wrapper.

```rust
#[repr(transparent)]
struct Celsius(f64);

fn as_celsius(raw: &f64) -> &Celsius {
    // SAFETY: `repr(transparent)` guarantees that `Celsius` has the size and
    // alignment of `f64`, and every `f64` is a valid `Celsius`. The result
    // has the lifetime of `raw` and is also a shared reference.
    unsafe { &*std::ptr::from_ref(raw).cast::<Celsius>() }
}

fn main() {
    let reading = 21.5;
    assert_eq!(as_celsius(&reading).0, 21.5);
}
```

Remove the attribute, and this program still compiles, passes its test, and
passes Miri. Only the SAFETY comment becomes false. This is a case that no
tool finds for you. Also check the second half of the comment: if the wrapper
has an invariant ("never below -273.15"), not every `f64` is valid, and the
cast needs a check first.

### Part 3: `repr(packed)`

`#[repr(packed)]` removes all padding and lowers the alignment of the struct
to 1. The fields then sit at whatever offset comes next, so they are usually
**not aligned**. References must always be aligned, so a reference to such a
field must never exist:

```rust,compile_fail,E0793
#[repr(C, packed)]
struct Entry {
    tag: u8,
    size: u32,
}

fn main() {
    let entry = Entry { tag: 7, size: 4096 };
    let size = &entry.size;
    println!("{size}");
}
```

```text
error[E0793]: reference to field of packed struct is unaligned
 --> src/main.rs:9:16
  |
9 |     let size = &entry.size;
  |                ^^^^^^^^^^^
  |
  = note: this struct is 1-byte aligned, but the type of this field may require higher alignment
  = note: creating a misaligned reference is undefined behavior (even if that reference is never dereferenced)
  = help: copy the field contents to a local variable, or replace the reference with a raw pointer and use `read_unaligned`/`write_unaligned` (loads and stores via `*p` must be properly aligned even when using raw pointers)
```

The help text lists both repairs:

```rust
#[repr(C, packed)]
struct Entry {
    tag: u8,
    size: u32,
}

fn main() {
    let entry = Entry { tag: 7, size: 4096 };
    // Repair 1: copy the field. The compiler emits an unaligned load.
    let size = entry.size;
    assert_eq!(size, 4096);
    // Repair 2: a raw pointer made without a reference, then an unaligned read.
    let field = &raw const entry.size;
    // SAFETY: `field` points to the four initialized bytes of `entry.size`;
    // `read_unaligned` does not need an aligned address.
    assert_eq!(unsafe { field.read_unaligned() }, 4096);
    assert_eq!((size_of::<Entry>(), align_of::<Entry>()), (5, 1));
}
```

Macros such as `println!` and `assert_eq!` take references to their
arguments, so `assert_eq!(entry.size, 4096)` gives the same error. Write
`{ entry.size }`, which copies the field first.

The same rule is the reason for `read_unaligned` in general: `*p` and
`p.read()` require an aligned `p`, wherever `p` came from.

### When to use which

- Use `Cell`, `RefCell`, the atomic types, `Mutex` and `OnceLock` from std.
  Write your own type on `UnsafeCell` only when none of them fits.
- Use `repr(transparent)` for a wrapper type that must be cast to or from its
  field, or passed to C in place of it.
- Use `repr(packed)` only for a byte format that leaves no choice. Often it
  is simpler to keep a normal struct and to decode with `u32::from_le_bytes`.

### Read more

- [std::cell::UnsafeCell](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html)
- [The Rust Reference: Interior mutability](https://doc.rust-lang.org/reference/interior-mutability.html)
- [The Rustonomicon: Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)
- [The Rust Reference: the transparent representation](https://doc.rust-lang.org/reference/type-layout.html#the-transparent-representation)
- [The Rust Reference: the alignment modifiers](https://doc.rust-lang.org/reference/type-layout.html#the-alignment-modifiers)
- [The Rust Reference: raw borrow operators](https://doc.rust-lang.org/reference/expressions/operator-expr.html#raw-borrow-operators)
- [std::ptr::read_unaligned](https://doc.rust-lang.org/std/ptr/fn.read_unaligned.html)

## Lesson 08 · Capstone: initialized-prefix storage

The capstone combines lessons 01, 03 and 06 in one small container. It has
room for `N` values in an array of `MaybeUninit<T>` and a length `len`. This
section explains how to approach it. It does not show the code.

**Start from the invariant.** Write it down before any code:

```text
            0             len              N
 data:    [ T | T | T | ? | ? | ? | ? ]
            initialized    not initialized
            owned by the   never read,
            container      never dropped
```

`len <= N`, and exactly the slots before `len` hold values. `len` is the only
record of that fact. The array itself knows nothing: a `MaybeUninit<T>` never
drops anything.

**Ask three questions about each method.**

1. Which slots does the method read or write?
2. Does the invariant say that those slots are in the state the method needs
   (holding a value for a read, empty for a write)?
3. Is the invariant true again at every point where the method can return or
   panic?

**Look for the dangerous moment.** A method that stores a value has a moment
when a slot is filled but not yet counted. A method that removes a value has
a moment when the slot's value has moved out, and the question is whether the
slot is still counted. For each of these moments, ask what the supplied
`Drop` would do if the function stopped right there. One of the two answers
is "a value is leaked", which is acceptable. The other is "a value is dropped
twice", which is not. Choose the order of your two statements so that the
second situation cannot exist. This is the order rule from lesson 06.

**Build the slice view last.** It is `from_raw_parts` with the six points of
lesson 03. The interesting points are "initialization" and "bounds": both
follow from the invariant, and from nothing else.

**Then try to break it.** Add a test of your own with a type that counts its
drops. Fill the container, pop some values, drop the rest, and check that the
count is exactly the number of values you created. Run the lesson under Miri
(Lab 1).

### Read more

- [std::mem::MaybeUninit: initializing an array element by element](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html#initializing-an-array-element-by-element)
- [The Rustonomicon: Implementing Vec](https://doc.rust-lang.org/nomicon/vec/vec.html)
