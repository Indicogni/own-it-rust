# Capstone "nostd_collections": specification

This document is the contract. The acceptance tests in `tests/` check what
is written here and nothing else. Where a behaviour is called
*unspecified*, the tests accept every sound choice.

Every `rust` code block in this file is also a test: `cargo test` compiles
and runs the blocks against your crate (section 9.2). The blocks marked as
"must not compile" are tests too.

Contents:

1. [What you build](#1-what-you-build)
2. [Rules for the crate](#2-rules-for-the-crate)
3. [`CapacityError`](#3-capacityerror)
4. [`ArrayVec`](#4-arrayvec)
5. [`RingBuffer`](#5-ringbuffer)
6. [`Arena`](#6-arena)
7. [Programs that must not compile](#7-programs-that-must-not-compile)
8. [Contracts of the whole crate](#8-contracts-of-the-whole-crate)
9. [The tests](#9-the-tests)
10. [Milestones](#10-milestones)
11. [Stretch goals](#11-stretch-goals)
12. [Non-goals](#12-non-goals)
13. [Evidence to keep](#13-evidence-to-keep)

## 1. What you build

A library crate named `nostd_collections` for programs that have no heap:
firmware, a kernel before its allocator exists, an interrupt handler. It
offers three data structures whose memory is part of the value itself, so
they can live on the stack or in a `static`.

| Type | What it is |
| --- | --- |
| `ArrayVec<T, N>` | A vector with room for at most `N` elements. |
| `RingBuffer<T, N>` | A double-ended queue with room for at most `N` elements. |
| `Arena<N>` | A bump allocator over `N` bytes. It hands out references that live as long as the arena is borrowed. |

A fourth type, `CapacityError<T>`, reports "no room" and gives the rejected
value back.

```rust
use nostd_collections::{Arena, ArrayVec, RingBuffer};

// A vector with room for four numbers. It is 4 * 4 bytes plus a length.
let mut primes: ArrayVec<u32, 4> = ArrayVec::new();
primes.push(2);
primes.push(3);
primes.push(5);
assert_eq!(primes.iter().sum::<u32>(), 10);
assert!(primes.try_push(7).is_ok());
assert!(primes.try_push(11).is_err()); // full: the value comes back

// The three most recent log lines.
let mut recent: RingBuffer<&str, 3> = RingBuffer::new();
for line in ["boot", "init", "ready", "tick"] {
    recent.push_overwrite(line);
}
assert_eq!(recent.front(), Some(&"init"));
assert_eq!(recent.back(), Some(&"tick"));

// 64 bytes of scratch memory for values of any type.
let arena: Arena<64> = Arena::new();
let counter = arena.alloc(41u64).unwrap();
let name = arena.alloc_str("arena").unwrap();
*counter += 1;
assert_eq!((*counter, &*name), (42, "arena"));
```

This is a blank-file project. You decide the fields of every type, the
invariants, the private helpers, the modules, and the iterator types. Only
the public names and signatures in this document are fixed.

## 2. Rules for the crate

1. **`#![no_std]`.** The crate uses `core` only. It does not use `std` and
   it does not use `alloc`: no `Box`, no `Vec`, no `String` inside the
   library. (The tests run on your own computer and do use `std`.)
2. **No dependencies.** `[dependencies]` in `Cargo.toml` stays empty.
3. **Four public names at the crate root:** `ArrayVec`, `RingBuffer`,
   `Arena`, `CapacityError`. The tests import exactly these. You may add
   more public items (iterator types, modules).
4. **The signatures in sections 3 to 6 are fixed** as written: the names,
   the parameter types, the return types, and the absence of trait bounds.
   For example, `ArrayVec::new` has no bound on `T`, so a design that needs
   `T: Default` or `T: Copy` does not meet the specification. Where this
   document says "an iterator", the type is yours: a named type or an
   `impl Trait` return type.
5. **Elements are stored in place.** A slot of a collection costs exactly
   `size_of::<T>()` bytes (section 8.1). The intended tool is
   `core::mem::MaybeUninit`.
6. **Unsafe code.** You will need `unsafe`. Every `unsafe` block gets a
   `// SAFETY:` comment that names the facts it relies on, and
   `unsafe_op_in_unsafe_fn` stays denied (it is set in `Cargo.toml`). The
   crate must be free of undefined behaviour for every program that uses
   only its safe API. Miri is the judge (section 9.3).
7. **Panics.** A function panics only where this document says so. A
   documented panic message *contains* the quoted text; the rest of the
   message is yours.

Terms used below:

- The *length* of a collection is the number of elements in it. Its
  *capacity* is `N`. A collection is *full* when the length equals `N`.
- A *zero-sized type* (ZST) is a type whose values take no memory, for
  example `()`.
- *Dropped exactly once* means: the destructor of the value ran one time.
  Running it twice (a *double drop*) is undefined behaviour for most types.
  Never running it is a *leak*; a leak is safe, but this document forbids it
  in most places.

## 3. `CapacityError`

The error of every fallible operation that takes a value by move. It owns
the value that did not fit.

```text
pub struct CapacityError<T> { /* yours */ }

impl<T> CapacityError<T> {
    pub fn into_inner(self) -> T;
}
impl<T> core::fmt::Debug   for CapacityError<T>
impl<T> core::fmt::Display for CapacityError<T>
impl<T> core::error::Error for CapacityError<T>
```

- `into_inner` returns the rejected value.
- `Display` writes exactly `insufficient capacity`.
- `Debug` writes any text that is not empty. It must exist for **every**
  `T`, also for a `T` without `Debug`: `Result::unwrap` and `Result::expect`
  need the error to be `Debug`, and they must work for all element types.
- The three trait implementations have no bounds on `T`.
- `CapacityError<T>` is `Send`, `Sync` and `Unpin` exactly when `T` is. You
  get this for free if you do not use raw pointers in it.
- No public constructor is required. The tests never build the error
  themselves.

```rust
use nostd_collections::ArrayVec;

struct Sensor; // no Debug, no Clone

let mut v: ArrayVec<Sensor, 1> = ArrayVec::new();
v.try_push(Sensor).unwrap(); // compiles although Sensor has no Debug

let error = v.try_push(Sensor).unwrap_err();
assert_eq!(error.to_string(), "insufficient capacity");
let _sensor: Sensor = error.into_inner(); // nothing was lost
```

## 4. `ArrayVec`

`ArrayVec<T, const N: usize>` is a vector of at most `N` elements of type
`T`, stored inside the `ArrayVec` value.

### 4.1 Construction and sizes

```text
pub const fn new() -> Self;
pub fn capacity(&self) -> usize;
pub fn len(&self) -> usize;
pub fn is_empty(&self) -> bool;
pub fn is_full(&self) -> bool;
impl<T, const N: usize> Default for ArrayVec<T, N>
```

- `new` creates an empty vector. It must be a `const fn`, so that a vector
  can be the initial value of a `static` or a `const`.
- `capacity` returns `N`. `is_full` is `len() == N`.
- `Default::default()` is `new()`.
- `N == 0` is valid. Such a vector is empty and full at the same time; no
  push succeeds.
- Zero-sized element types are valid. The capacity limits them like every
  other type, the length counts them, and their destructors run. For a ZST,
  `N` may be as large as `usize::MAX`.

### 4.2 Push, pop and the slice view

```text
pub fn push(&mut self, value: T);
pub fn try_push(&mut self, value: T) -> Result<(), CapacityError<T>>;
pub fn pop(&mut self) -> Option<T>;
pub fn as_slice(&self) -> &[T];
pub fn as_mut_slice(&mut self) -> &mut [T];
impl<T, const N: usize> core::ops::Deref    for ArrayVec<T, N>   // Target = [T]
impl<T, const N: usize> core::ops::DerefMut for ArrayVec<T, N>
```

- `try_push` appends `value` behind the last element. If the vector is
  full, it changes nothing and returns the value inside the error.
- `push` does the same, but a full vector is a bug of the caller: it panics
  with a message that contains `capacity`.
- `pop` removes the last element and returns it, or returns `None` if the
  vector is empty.
- `as_slice`, `as_mut_slice`, `deref` and `deref_mut` give the `len()`
  elements in index order as one slice. The slice points into the
  `ArrayVec` value itself; neighbouring elements are `size_of::<T>()` bytes
  apart. Through `Deref`, every method of slices works on a vector:
  indexing, `iter`, `iter_mut`, `sort`, `contains`, `first`, and so on.

```rust
use nostd_collections::ArrayVec;

let mut v: ArrayVec<i32, 8> = ArrayVec::new();
for n in [5, 3, 9] {
    v.push(n);
}
v.sort();                       // a slice method, through DerefMut
assert_eq!(*v, [3, 5, 9]);
assert_eq!(v[0], 3);
assert_eq!(v.pop(), Some(9));
assert_eq!(v.len(), 2);
```

### 4.3 Ownership of the elements

```text
pub fn truncate(&mut self, len: usize);
pub fn clear(&mut self);
impl<T, const N: usize> Drop for ArrayVec<T, N>      // or an equivalent
```

The vector owns its elements.

- Dropping the vector drops each element exactly once, in index order.
- A value that leaves the vector (through `pop`, `remove`, `swap_remove`,
  `drain`, `into_iter`, `into_inner`) belongs to the caller. The vector
  never drops it.
- A value that is rejected is returned (inside `CapacityError`). The vector
  never drops it.
- `truncate(len)` does nothing if `len >= self.len()`. Otherwise it drops
  the elements from index `len` on, in index order, and the length becomes
  `len`.
- `clear()` is `truncate(0)`.

**Destructors that panic.** If the destructor of an element panics during
`truncate`, during `clear`, or while the vector itself is dropped, the panic
continues to the caller, and:

- no element is dropped twice;
- when the vector is being dropped, the elements behind the panicking one
  are still dropped;
- after `truncate` or `clear`, the vector is valid and usable. Which of the
  elements that were not yet dropped are still in it is unspecified, but by
  the time the vector itself has been dropped, every element has been
  dropped exactly once.

(If a second destructor panics during the same unwinding, Rust aborts the
program. That is not your problem to solve.)

### 4.4 Insert and remove

```text
pub fn insert(&mut self, index: usize, value: T);
pub fn try_insert(&mut self, index: usize, value: T) -> Result<(), CapacityError<T>>;
pub fn remove(&mut self, index: usize) -> T;
pub fn swap_remove(&mut self, index: usize) -> T;
```

- `try_insert` puts `value` at `index`; the elements from `index` on move
  up by one. `index == len()` appends. If the vector is full, it changes
  nothing and returns the value inside the error.
- `insert` does the same, but panics with a message that contains
  `capacity` if the vector is full.
- Both panic with a message that contains `out of bounds` if
  `index > len()`. **The index is checked first:** a wrong index panics
  with `out of bounds` even if the vector is also full.
- `remove` takes the element at `index` out and returns it; the elements
  behind it move down by one and keep their order.
- `swap_remove` takes the element at `index` out and returns it; the last
  element moves into the free place. It moves one element only, and does
  not keep the order.
- `remove` and `swap_remove` panic with a message that contains
  `out of bounds` if `index >= len()`.
- None of the four drops or clones an element.

### 4.5 `retain`

```text
pub fn retain<F: FnMut(&T) -> bool>(&mut self, keep: F);
```

Calls `keep` once for each element, in index order. Elements for which it
returns `true` stay, in their order. Each other element is dropped exactly
once, before `keep` is called for the next element.

**If `keep` panics** at some element, the panic continues to the caller.
Afterwards the vector holds, in this order: the elements kept so far, the
element at which `keep` panicked, and the elements not yet visited. The
elements rejected before the panic have been dropped once. Nothing is
leaked and nothing is dropped twice.

```rust
use nostd_collections::ArrayVec;

let mut v: ArrayVec<u32, 8> = (1..=6).collect();
v.retain(|n| n % 2 == 0);
assert_eq!(*v, [2, 4, 6]);
```

### 4.6 `drain`

```text
pub fn drain<R: core::ops::RangeBounds<usize>>(&mut self, range: R) -> /* an iterator */;
```

Removes the elements in `range` and returns an iterator that owns them.

- `range` is any range of indices: `a..b`, `a..=b`, `a..`, `..b`, `..=b`,
  `..`. Let `start` be its first index and `end` the index behind its last.
  If `start > end` or `end > len()`, `drain` panics with a message that
  contains `out of bounds` and changes nothing. An empty range (`start ==
  end`, including `len()..`) is allowed.
- The iterator implements `Iterator<Item = T>`, `DoubleEndedIterator` and
  `ExactSizeIterator`. It yields the elements of the range in index order
  (`next_back` yields them from the end of the range). After it has
  returned `None`, it keeps returning `None`.
- The iterator borrows the vector mutably: the vector cannot be used while
  the iterator lives (section 7).
- **When the iterator is dropped,** it drops the elements of the range that
  it did not yield, in index order, and the vector then holds the elements
  before the range followed by the elements behind it. This also happens
  if the iterator was never used: `drop(v.drain(1..3))` removes two
  elements.
- **Leak safety.** A caller can leak the iterator with
  `core::mem::forget`, so its destructor never runs. That is safe Rust, and
  the vector must stay sound: the elements before the range are still
  there; an element that the iterator already yielded is not in the vector
  any more; no element can be dropped twice. Which of the elements from
  `start` on remain is unspecified (they may all be leaked).

```rust
use nostd_collections::ArrayVec;

let mut v: ArrayVec<char, 8> = "abcdef".chars().collect();
let middle: String = v.drain(1..4).collect();
assert_eq!(middle, "bcd");
assert_eq!(*v, ['a', 'e', 'f']);

v.drain(1..);                   // dropped at once: removes 'e' and 'f'
assert_eq!(*v, ['a']);
```

### 4.7 Iteration

```text
impl<T, const N: usize>     IntoIterator for ArrayVec<T, N>            // Item = T
impl<'a, T, const N: usize> IntoIterator for &'a ArrayVec<T, N>        // Item = &'a T
impl<'a, T, const N: usize> IntoIterator for &'a mut ArrayVec<T, N>    // Item = &'a mut T
```

- All three yield the elements in index order.
- The iterator of `ArrayVec<T, N>` itself (iteration by value) implements
  `DoubleEndedIterator` and `ExactSizeIterator`. After it has returned
  `None`, it keeps returning `None`. When it is dropped, it drops the
  elements it did not yield, each exactly once, in index order.

### 4.8 Standard traits and arrays

```text
impl<T, const N: usize> Extend<T>       for ArrayVec<T, N>
impl<T, const N: usize> FromIterator<T> for ArrayVec<T, N>
impl<T, const N: usize> From<[T; N]>    for ArrayVec<T, N>
impl<T: Clone, const N: usize>             Clone     for ArrayVec<T, N>
impl<T: core::fmt::Debug, const N: usize>  Debug     for ArrayVec<T, N>
impl<T: PartialEq, const N: usize>         PartialEq for ArrayVec<T, N>
impl<T: Eq, const N: usize>                Eq        for ArrayVec<T, N>
pub fn into_inner(self) -> Result<[T; N], Self>;
```

- `extend` appends the items of an iterator one by one. When an item does
  not fit, it panics with a message that contains `capacity`; the items
  before it stay in the vector. If the iterator itself panics, the items
  appended so far stay in the vector.
- `from_iter` (used by `collect`) builds a vector from an iterator. It
  panics with a message that contains `capacity` as soon as an item does
  not fit. The items collected so far, and the item that did not fit, are
  dropped exactly once.
- `From<[T; N]>` builds a full vector from an array, in order.
  `into_inner` is the way back: a full vector returns `Ok` with its
  elements as an array; any other vector returns `Err` with itself,
  unchanged. Neither of the two clones or drops an element. For `N == 0`,
  `into_inner` returns `Ok([])`.
- `clone` returns a vector with clones of the elements, in order. If
  `T::clone` panics, the clones made so far are dropped exactly once, and
  the original is untouched.
- `Debug` prints what the slice of the elements prints, for `{:?}` and for
  `{:#?}`: `[1, 2, 3]`. The free places are not printed.
- Two vectors are equal when their slices are equal. What the free places
  contain does not matter.

```rust
use nostd_collections::ArrayVec;

let v = ArrayVec::from([1u8, 2, 3]);
assert!(v.is_full());
assert_eq!(format!("{v:?}"), "[1, 2, 3]");
assert_eq!(v.clone(), v);
assert_eq!(v.into_inner().ok(), Some([1, 2, 3]));

let short: ArrayVec<u8, 3> = [1, 2].into_iter().collect();
assert!(short.into_inner().is_err()); // not full: no array
```

## 5. `RingBuffer`

`RingBuffer<T, const N: usize>` is a double-ended queue of at most `N`
elements of type `T`, stored inside the `RingBuffer` value. Elements can be
added and removed at both ends in constant time. Nothing is ever moved to
make room.

### 5.1 The storage model, construction and sizes

The buffer has `N` *slots*, numbered `0` to `N - 1`. Its elements occupy
slots that follow each other; behind slot `N - 1` comes slot `0`. The
element that `pop_front` would return is the *front*, the one that
`pop_back` would return is the *back*.

```text
N = 8, five elements a b c d e, front in slot 6:

slot      0   1   2   3   4   5   6   7
        +---+---+---+---+---+---+---+---+
        | c | d | e |   |   |   | a | b |
        +---+---+---+---+---+---+---+---+
                  ^ back          ^ front
```

- The buffer always has a *front slot*, also while it is empty: an empty
  buffer remembers where its next element goes. In a new buffer the front
  slot is slot 0.
- `push_back` writes to the slot behind the back. In an empty buffer it
  writes to the front slot. So the first element pushed into a new buffer
  with `push_back` goes into slot 0.
- `push_front` writes to the slot before the front slot, and that slot
  becomes the front slot (before slot 0 comes slot `N - 1`). So the first
  element pushed into a new buffer with `push_front` goes into slot
  `N - 1`.
- `pop_front` frees the front slot; the next slot becomes the front slot,
  also when the buffer is empty afterwards. The front slot is **not** set
  back to slot 0 when the last element leaves.
- `pop_back` frees the back slot. The front slot stays where it is.

This model is fixed because `as_slices` (section 5.3) shows it. Only
`clear` (section 5.4) is free to choose a new front slot, and a clone
(section 5.6) does not have to copy it.

```text
pub const fn new() -> Self;
pub fn capacity(&self) -> usize;
pub fn len(&self) -> usize;
pub fn is_empty(&self) -> bool;
pub fn is_full(&self) -> bool;
impl<T, const N: usize> Default for RingBuffer<T, N>
```

- `new` must be a `const fn`. `capacity` returns `N`; `is_full` is
  `len() == N`; `Default::default()` is `new()`.
- `N == 0` is valid: the buffer is empty and full at the same time, and no
  function may divide by zero.
- Zero-sized element types are valid, as in section 4.1. For a ZST, `N` may
  be as large as `usize::MAX`, and the arithmetic on slot numbers must not
  overflow then.

### 5.2 The two ends

```text
pub fn push_back(&mut self, value: T) -> Result<(), CapacityError<T>>;
pub fn push_front(&mut self, value: T) -> Result<(), CapacityError<T>>;
pub fn push_overwrite(&mut self, value: T) -> Option<T>;
pub fn pop_front(&mut self) -> Option<T>;
pub fn pop_back(&mut self) -> Option<T>;
```

- `push_back` and `push_front` add `value` at their end. If the buffer is
  full, they change nothing and return the value inside the error. (For a
  queue, "full" is an ordinary event and not a bug, so there are no
  panicking variants.)
- `pop_front` and `pop_back` remove the element at their end and return it,
  or return `None` if the buffer is empty.
- `push_overwrite` always accepts the newest value:
  - if there is room, it works like `push_back` and returns `None`;
  - if the buffer is full and `N > 0`, it removes the front element,
    appends `value` at the back, and returns `Some` with the removed
    element;
  - if `N == 0`, the buffer cannot keep anything: it returns `Some(value)`.

```rust
use nostd_collections::RingBuffer;

let mut queue: RingBuffer<u32, 3> = RingBuffer::new();
queue.push_back(1).unwrap();
queue.push_back(2).unwrap();
queue.push_front(0).unwrap();
assert!(queue.push_back(3).is_err());         // full
assert_eq!(queue.pop_front(), Some(0));
assert_eq!(queue.pop_back(), Some(2));

assert_eq!(queue.push_overwrite(7), None);    // room: nothing leaves
assert_eq!(queue.push_overwrite(8), None);
assert_eq!(queue.push_overwrite(9), Some(1)); // full: the oldest leaves
```

### 5.3 Reading

```text
pub fn get(&self, index: usize) -> Option<&T>;
pub fn get_mut(&mut self, index: usize) -> Option<&mut T>;
pub fn front(&self) -> Option<&T>;
pub fn back(&self) -> Option<&T>;
pub fn as_slices(&self) -> (&[T], &[T]);
pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]);
```

- Indices count from the front: `get(0)` is the front element and
  `get(len() - 1)` is the back element, whatever slots they are in. An
  index that is not smaller than `len()` gives `None`.
- `front()` is `get(0)`. `back()` is the last element, or `None` if the
  buffer is empty.
- `as_slices` returns the contents as two slices into the buffer itself.
  The first slice holds the elements from the front slot up to the end of
  the storage (or up to the back, if that comes first). The second slice
  holds the elements that continue at slot 0. So:
  - the first slice followed by the second is the contents from front to
    back;
  - the second slice is empty unless the contents wrap around the end of
    the storage;
  - the first slice is empty only if the buffer is empty.
- `as_mut_slices` returns the same two slices, mutable.

Two examples that follow from the storage model, and that the tests check:

```rust
use nostd_collections::RingBuffer;

let mut ring: RingBuffer<u32, 4> = RingBuffer::new();
for n in [1, 2, 3, 4] {
    ring.push_back(n).unwrap();
}
// Only pushed at the back since `new`: slots 0 to 3, one run.
assert_eq!(ring.as_slices(), (&[1, 2, 3, 4][..], &[][..]));

ring.pop_front();
ring.pop_front();
ring.push_back(5).unwrap();
ring.push_back(6).unwrap();
// 3 and 4 are in slots 2 and 3; 5 and 6 wrapped to slots 0 and 1.
assert_eq!(ring.as_slices(), (&[3, 4][..], &[5, 6][..]));
assert_eq!(ring.get(2), Some(&5));
```

### 5.4 Ownership of the elements

```text
pub fn clear(&mut self);
impl<T, const N: usize> Drop for RingBuffer<T, N>    // or an equivalent
```

The buffer owns its elements, with the same rules as section 4.3:

- Dropping the buffer drops each element exactly once, **from front to
  back** (not in slot order).
- A value that leaves the buffer (`pop_front`, `pop_back`, the element that
  `push_overwrite` returns, `into_iter`) belongs to the caller. A rejected
  value is returned. The buffer drops neither.
- `clear()` drops every element, from front to back. Afterwards the buffer
  is empty and its whole capacity can be used again. Which slot the next
  element goes into is unspecified.
- **Destructors that panic** during `clear` or while the buffer is dropped:
  the same three rules as in section 4.3. In particular, the elements in
  the *second* slice are still dropped when a destructor in the first slice
  panics.

### 5.5 Iterators

```text
pub fn iter(&self) -> /* an iterator, Item = &T */;
pub fn iter_mut(&mut self) -> /* an iterator, Item = &mut T */;
impl<T, const N: usize>     IntoIterator for RingBuffer<T, N>           // Item = T
impl<'a, T, const N: usize> IntoIterator for &'a RingBuffer<T, N>       // Item = &'a T
impl<'a, T, const N: usize> IntoIterator for &'a mut RingBuffer<T, N>   // Item = &'a mut T
```

- All five yield the elements from front to back.
- All five iterators implement `DoubleEndedIterator` and
  `ExactSizeIterator`. `size_hint` returns `(len, Some(len))` with the
  exact number of elements left. `next` and `next_back` meet in the middle:
  every element is yielded once. After an iterator has returned `None`, it
  keeps returning `None`.
- The iterator of `iter` also implements `Clone`, without a bound on `T`. A
  clone continues at the same position and is independent from then on.
- `IntoIterator` for `&RingBuffer` and `&mut RingBuffer` behave like `iter`
  and `iter_mut`.
- The iterator of `RingBuffer<T, N>` itself (iteration by value) drops the
  elements it did not yield when it is dropped, each exactly once, from
  front to back.

Note that `core::iter::Chain` of two slice iterators is not an
`ExactSizeIterator`. You need a type of your own.

```rust
use nostd_collections::RingBuffer;

let mut ring: RingBuffer<u32, 3> = RingBuffer::new();
for n in 1..=5 {
    ring.push_overwrite(n);     // holds 3, 4, 5; the contents wrap
}
let mut iter = ring.iter();
assert_eq!(iter.len(), 3);
assert_eq!(iter.next(), Some(&3));
assert_eq!(iter.next_back(), Some(&5));
assert_eq!(iter.next(), Some(&4));
assert_eq!(iter.next(), None);

for n in &mut ring {
    *n *= 10;
}
assert_eq!(ring.into_iter().rev().collect::<Vec<_>>(), [50, 40, 30]);
```

### 5.6 Standard traits

```text
impl<T: Clone, const N: usize>             Clone     for RingBuffer<T, N>
impl<T: core::fmt::Debug, const N: usize>  Debug     for RingBuffer<T, N>
impl<T: PartialEq, const N: usize>         PartialEq for RingBuffer<T, N>
impl<T: Eq, const N: usize>                Eq        for RingBuffer<T, N>
```

- `clone` returns a buffer with clones of the elements, in the same order.
  Which slots the clone uses is unspecified. If `T::clone` panics, the
  clones made so far are dropped exactly once, and the original is
  untouched.
- `Debug` prints the contents from front to back like a slice, for `{:?}`
  and for `{:#?}`: `[3, 4, 5]`.
- Two buffers are equal when they hold equal elements in the same order.
  Which slots the elements are in does not matter.

## 6. `Arena`

`Arena<const N: usize>` is a *bump allocator*: it owns `N` bytes and a
number, `used`, that says how many of them are taken. An allocation takes
the next free bytes and raises `used`. Single values are never freed;
`reset` frees everything at once.

The arena's value is that it turns one fixed block of memory into many
values of different types, with references that the borrow checker ties to
the arena.

### 6.1 Construction and sizes

```text
pub const fn new() -> Self;
pub fn capacity(&self) -> usize;
pub fn used(&self) -> usize;
pub fn remaining(&self) -> usize;
impl<const N: usize> Default for Arena<N>
impl<const N: usize> core::fmt::Debug for Arena<N>
```

- `new` creates an arena with `used() == 0`. It must be a `const fn`.
  `Default::default()` is `new()`.
- `capacity` returns `N`. `used` returns the number of bytes taken,
  padding included. `remaining` is `capacity() - used()`.
- `Debug` writes any text that is not empty. (Printing the capacity and
  `used` is useful. Printing the bytes is not.)
- The `N` bytes are stored inside the `Arena` value. `N == 0` is valid.

### 6.2 The placement rule

The storage is a row of `N` bytes with *offsets* `0` to `N - 1`. **Offset 0
is at an address that is a multiple of 16.** This makes placement a pure
function of the allocations made so far: it does not depend on where the
arena happens to be in memory.

A value with size `size > 0` and alignment `align <= 16` is placed like
this:

1. `start` = `used`, rounded up to the next multiple of `align`.
2. If `start + size > N`, the value does not fit. Nothing changes; `used`
   keeps its value (the padding is not taken either).
3. Otherwise the value occupies the offsets `start .. start + size`, and
   `used` becomes `start + size`.

For an alignment above 16, step 1 rounds the *address* up instead of the
offset. The value is aligned, but how much padding it needs depends on the
address of the arena (at most `align - 1` bytes).

A value with `size == 0` always fits, wherever `used` is. It changes
nothing: no bytes and no padding are taken.

The computation must not overflow: a request that is too large is refused,
whatever its size.

Example for `Arena<32>`, with the usual alignments (`u16`: 2, `u32`: 4,
`u64`: 8):

| Call | `start` | `used` afterwards | Note |
| --- | --- | --- | --- |
| `alloc(1u8)` | 0 | 1 | |
| `alloc(2u32)` | 4 | 8 | 3 bytes of padding |
| `alloc(3u16)` | 8 | 10 | |
| `alloc(4u64)` | 16 | 24 | 6 bytes of padding |
| `alloc([5u8; 8])` | 24 | 32 | the arena is full |
| `alloc(6u8)` | | 32 | refused |
| `alloc(())` | | 32 | a ZST always fits |

```rust
use nostd_collections::Arena;

let arena: Arena<32> = Arena::new();
arena.alloc(1u8).unwrap();
arena.alloc(2u32).unwrap();
assert_eq!(arena.used(), 8);
arena.alloc(3u16).unwrap();
assert_eq!((arena.used(), arena.remaining()), (10, 22));
```

### 6.3 `alloc`

```text
pub fn alloc<T>(&self, value: T) -> Result<&mut T, CapacityError<T>>;
```

- Places a value of `size_of::<T>()` bytes and alignment `align_of::<T>()`
  by the rule of section 6.2, moves `value` there, and returns a mutable
  reference to it. If it does not fit, the value comes back inside the
  error and nothing changes.
- `alloc` takes `&self`, so many references into one arena can be alive at
  the same time. Each of them is unique: no two allocations share a byte.
  A reference stays valid while later allocations are made.
- The returned reference lives as long as the borrow of the arena, as the
  signature says. It cannot outlive the arena (section 7).
- The address of the reference is a multiple of `align_of::<T>()`, also
  for a zero-sized `T`, and it is never null.

`&mut T` from `&self` is the one place in this capstone where the usual
rule "shared in, shared out" is broken on purpose. Clippy rejects such a
signature by default (the lint `clippy::mut_from_ref`); the facade allows
the lint on the four allocation functions, and so may you. In exchange,
your code must contain the argument why this is sound for your design:
write it as documentation on the type, and let every `SAFETY` comment refer
to it.

### 6.4 Slices and strings

```text
pub fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> Option<&mut [T]>;
pub fn alloc_str(&self, src: &str) -> Option<&mut str>;
pub fn alloc_slice_fill_with<T, F>(&self, len: usize, fill: F) -> Option<&mut [T]>
where
    F: FnMut(usize) -> T;
```

All three place **one** value by the rule of section 6.2 and return `None`
if it does not fit (nothing changes then).

- `alloc_slice_copy` copies `src`. The value has size
  `src.len() * size_of::<T>()` and alignment `align_of::<T>()`.
- `alloc_str` copies the bytes of `src` (alignment 1) and returns them as a
  `str`.
- `alloc_slice_fill_with` makes a slice of `len` elements; element `i` is
  `fill(i)`. The value has size `len * size_of::<T>()` and alignment
  `align_of::<T>()`.
  - If the slice does not fit, or if its size in bytes overflows `usize`,
    the function returns `None` **without calling `fill`**.
  - Otherwise it calls `fill(0)`, `fill(1)`, ... once each, in this order.
  - **The room is reserved before `fill` is called for the first time.**
    `fill` may therefore allocate from the same arena; what it allocates
    lies behind the slice.
  - If `fill` panics, the panic continues to the caller. The arena stays
    usable and earlier allocations are untouched. The elements made so far
    are not dropped (section 6.5). `used()` is at most what it would be
    after a successful call.

```rust
use nostd_collections::Arena;

let arena: Arena<128> = Arena::new();
let squares = arena.alloc_slice_fill_with(4, |i| (i * i) as u32).unwrap();
let name = arena.alloc_str("bump").unwrap();
let copy = arena.alloc_slice_copy(&[1u8, 2, 3]).unwrap();
name.make_ascii_uppercase();
squares[0] = 100;
assert_eq!(squares, [100, 1, 4, 9]);
assert_eq!((&*name, &*copy), ("BUMP", &[1, 2, 3][..]));
assert_eq!(arena.used(), 16 + 4 + 3);
```

### 6.5 Destructors and `reset`

```text
pub fn reset(&mut self);
```

- **The arena never runs the destructor of a stored value:** not when the
  arena is dropped, and not in `reset`. Storing a value with a destructor
  is allowed and has the effect of `core::mem::forget`. (Running
  destructors would be unsound for values that hold references to other
  values of the same arena, and it would need bookkeeping that a bump
  allocator does not have. See the stretch goals.)
- `reset` sets `used` to 0. The next allocation starts at offset 0 again.
- `reset` takes `&mut self`. That is its whole safety argument: a mutable
  borrow is possible only when no reference into the arena is alive
  (section 7).

## 7. Programs that must not compile

The most important property of this crate cannot be seen at run time: the
lifetimes in its signatures let the borrow checker reject programs that
would use memory after it was freed or reused. If you change a signature
in a way that loses this (for example, by returning a reference with a
lifetime that the caller can choose), every run-time test still passes.

The following programs are tests as well. Each block marked `compile_fail`
passes when the compiler rejects it.

First the allowed versions, so that you can see that the rejected programs
are rejected for the right reason:

```rust
use nostd_collections::{Arena, ArrayVec, RingBuffer};

let mut arena: Arena<16> = Arena::new();
let number = arena.alloc(7u32).unwrap();
*number += 1;
arena.reset(); // fine: `number` is not used after this line
let moved = arena; // fine for the same reason
assert_eq!(moved.used(), 0);

let mut v = ArrayVec::from([1u32, 2, 3, 4]);
let mut drain = v.drain(1..3);
drain.next();
drop(drain);
v.push(9); // fine: the drain is gone
assert_eq!(*v, [1, 4, 9]);

let mut ring: RingBuffer<u32, 4> = RingBuffer::new();
ring.push_back(1).unwrap();
assert_eq!(ring.iter().next(), Some(&1));
ring.push_back(2).unwrap(); // fine: the iterator is gone

let v: ArrayVec<String, 2> = ArrayVec::new();
std::thread::spawn(move || drop(v)).join().unwrap(); // String is Send
```

### 7.1 A reference cannot outlive its arena

```rust,compile_fail,E0597
use nostd_collections::Arena;

let dangling: &mut u32;
{
    let arena: Arena<16> = Arena::new();
    dangling = arena.alloc(7u32).unwrap();
} // the arena and its bytes end here
*dangling += 1;
```

### 7.2 `reset` needs every reference to be dead

```rust,compile_fail,E0502
use nostd_collections::Arena;

let mut arena: Arena<16> = Arena::new();
let number = arena.alloc(7u32).unwrap();
arena.reset();
*number += 1; // would write to bytes that are free again
```

### 7.3 A borrowed arena cannot move

The bytes are inside the arena. Moving the arena moves them, and a
reference would still point to the old place.

```rust,compile_fail,E0505
use nostd_collections::Arena;

let arena: Arena<16> = Arena::new();
let number = arena.alloc(7u32).unwrap();
let moved = arena;
*number += 1;
```

### 7.4 An arena cannot be shared between threads

```rust,compile_fail,E0277
use nostd_collections::Arena;

let arena: Arena<16> = Arena::new();
std::thread::scope(|scope| {
    scope.spawn(|| {
        arena.alloc(1u8).unwrap();
    });
});
```

### 7.5 A vector cannot be used while it is drained

```rust,compile_fail,E0499
use nostd_collections::ArrayVec;

let mut v = ArrayVec::from([1u32, 2, 3, 4]);
let mut drain = v.drain(1..3);
v.push(9);
drain.next();
```

### 7.6 A buffer cannot change while it is iterated

```rust,compile_fail,E0502
use nostd_collections::RingBuffer;

let mut ring: RingBuffer<u32, 4> = RingBuffer::new();
ring.push_back(1).unwrap();
let mut iter = ring.iter();
ring.push_back(2).unwrap();
iter.next();
```

### 7.7 A collection of values that are not `Send` is not `Send`

```rust,compile_fail,E0277
use nostd_collections::ArrayVec;
use std::rc::Rc;

let mut v: ArrayVec<Rc<u32>, 2> = ArrayVec::new();
v.push(Rc::new(1));
std::thread::spawn(move || drop(v));
```

The stable compiler checks only that these programs fail to compile. The
nightly compiler also checks the error code in the first line of each
block: `cargo +nightly-2026-11-20 test --doc`.

## 8. Contracts of the whole crate

### 8.1 Sizes

The memory of each type is its storage and a little bookkeeping. With
`W = size_of::<usize>()`:

| Type | Lower bound of `size_of` | Upper bound of `size_of` |
| --- | --- | --- |
| `ArrayVec<T, N>` | `size_of::<[T; N]>()` | `size_of::<[T; N]>() + 2 * W` |
| `RingBuffer<T, N>` | `size_of::<[T; N]>()` | `size_of::<[T; N]>() + 3 * W` |
| `Arena<N>` | `N` | `N` rounded up to a multiple of 16, plus 16 |

The bounds for the collections hold for element types whose alignment is
at most that of `usize`. They exclude designs that pay per slot, such as
`[Option<T>; N]`.

### 8.2 `Send` and `Sync`

- `ArrayVec<T, N>` and `RingBuffer<T, N>` are `Send` exactly when `T` is
  `Send`, and `Sync` exactly when `T` is `Sync`.
- `Arena<N>` is `Send`: an arena that nobody borrows can move to another
  thread. `Arena<N>` is **not** `Sync`: `alloc` changes the arena through
  `&self` without synchronisation.

If your types contain no raw pointers, the compiler derives all of this,
and you write no `unsafe impl`. If you do write one, its `SAFETY` comment
has to carry the proof.

### 8.3 Variance

`ArrayVec<T, N>` and `RingBuffer<T, N>` are covariant in `T`, like
`Vec<T>`: an `ArrayVec<&'static str, N>` can be used where an
`ArrayVec<&'a str, N>` is expected. A test file contains functions that
compile only if this holds.

### 8.4 Working together

The types must compose: collections of collections drop every inner
element exactly once, and a collection can hold references that an arena
handed out.

### 8.5 No heap

No function of the crate asks the global allocator for memory. One test
binary installs a counting allocator and checks this on your computer. The
stronger proof is the build for a target without a heap (section 11).

## 9. The tests

### 9.1 Acceptance tests

Run everything from this directory.

| Command | What it checks |
| --- | --- |
| `cargo test --test m1_array_vec` | Sections 3 and 4.1 to 4.3. |
| `cargo test --test m2_array_vec_ops` | Sections 4.4 to 4.8, and the panicking destructors of 4.3. |
| `cargo test --test m3_ring` | Sections 5.1 to 5.4. |
| `cargo test --test m4_ring_iter` | Sections 5.5 and 5.6. |
| `cargo test --test m5_arena` | Section 6. |
| `cargo test --test m6_contracts` | Sections 8.1 to 8.4. |
| `cargo test --test m6_no_alloc` | Section 8.5. |
| `cargo test --doc` | The examples of this document, and section 7. |
| `cargo test` | All of the above. `cargo test --release` must pass too. |

The tests are deterministic: the three "random operations" tests use a
fixed seed and compare your type with `Vec`, `VecDeque`, or the placement
rule after every step. Each test body runs with a time limit of 20
seconds, so an endless loop in your code fails with a message and does not
hang the run.

The helper types of the tests are in `tests/common/mod.rs`. `Ledger` and
`Tracked` count how often each element was dropped; a failure message such
as "2 element(s) were dropped more than once" comes from there.

If a test program ends with a signal or an access violation instead of a
failure message, your unsafe code has undefined behaviour that happened to
crash. Run that test under Miri to find the place.

### 9.2 The examples of this document

`src/lib.rs` ends with a module declaration:

```text
#[cfg(doctest)]
mod spec_examples;
```

The file `src/spec_examples.rs` has one line of code, which attaches this
document to the module as its documentation. `cargo test` therefore runs
the code blocks of this document as documentation tests of your crate. A
failing block is reported with its line number in this file:

```text
test src/../SPEC.md - spec_examples (line 43) ... FAILED
```

Keep the declaration and the file when you restructure the crate.

### 9.3 Miri

```text
rustup toolchain install nightly-2026-11-20 --component miri
cargo +nightly-2026-11-20 miri test
```

Miri runs the same tests in an interpreter that checks every memory access
against the rules of the language: reads of uninitialised memory, accesses
outside an allocation, misaligned references, double frees, leaks, and
violations of the aliasing rules for references. **A clean Miri run is
required** for the capstone. Under Miri the time limit is off and the
random tests are shorter; the whole run takes less than a minute on a
current laptop.

Tests that pass normally and fail under Miri are the reason this capstone
exists. Typical findings are listed in HINTS.md.

## 10. Milestones

Work in this order. Each milestone ends with its tests green in debug and
release, a clean `cargo clippy --all-targets`, and a clean Miri run of the
test files so far.

| Milestone | You write | Tests that turn green |
| --- | --- | --- |
| **M1** The core of `ArrayVec` | `CapacityError`; storage and invariant; `new`, sizes, `push`, `try_push`, `pop`, the slice view, `truncate`, `clear`, `Drop` | `m1_array_vec` (19 tests); the example of section 3 |
| **M2** All of `ArrayVec` | `insert`, `remove`, `swap_remove`, `retain`, `drain`, the three `IntoIterator`s, `Extend`, `FromIterator`, `From<[T; N]>`, `into_inner`, `Clone`, `Debug`, `PartialEq`; panic and leak safety | `m2_array_vec_ops` (34 tests); the examples of section 4; section 7.5 |
| **M3** The core of `RingBuffer` | storage model and invariant; both ends, `push_overwrite`, `get`, the two slices, `clear`, `Drop` | `m3_ring` (25 tests); the examples of sections 5.2 and 5.3 |
| **M4** `RingBuffer` iterators and traits | `iter`, `iter_mut`, the three `IntoIterator`s, `Clone`, `Debug`, `PartialEq` | `m4_ring_iter` (12 tests); the example of section 5.5; section 7.6 |
| **M5** `Arena` | storage, placement, `alloc`, the slice functions, `reset`; the written soundness argument | `m5_arena` (20 tests); the examples of section 6; sections 7.1 to 7.4 |
| **M6** Contracts and evidence | whatever the size, auto-trait, variance and no-heap tests still ask for; Miri; the evidence of section 13 | `m6_contracts` (11 tests), `m6_no_alloc` (4 tests); the example of section 1; all of `cargo test` and `cargo +nightly-2026-11-20 miri test` |

In the starter, almost every test fails at a `todo!()`. Nine checks pass
from the start because they check something that the empty facade already
has:

- the two self-tests of the test helpers (`the_probe_macro_works`,
  `the_counting_allocator_counts`);
- two auto-trait tests of `m6_contracts`
  (`collections_are_send_and_sync_exactly_when_their_elements_are`,
  `capacity_error_passes_the_auto_traits_of_its_value_on`);
- the five blocks of section 7 whose signatures the facade already gives:
  7.1, 7.2, 7.3, 7.6 and 7.7.

They are guards: they must stay green while you work.

## 11. Stretch goals

Not tested. Pick what interests you.

1. **Build for a machine without an operating system.** This is the proof
   that the crate is really `no_std`:

   ```text
   rustup target add aarch64-unknown-none
   cargo build --target aarch64-unknown-none
   cargo build --release --target aarch64-unknown-none
   ```

   Also build for a 32-bit target, for example `thumbv7em-none-eabihf`
   (Arm Cortex-M4): `usize` has 4 bytes there. Then use the crate in a real
   `#![no_std]` `#![no_main]` program, for example in the kernel capstone.
2. **Formatting without a heap.** Implement `core::fmt::Write` for
   `ArrayVec<u8, N>` (or build an `ArrayString<N>` on top of it), so that
   `write!(buffer, "{value:>8.3}")` works on a stack buffer. Decide what a
   write that does not fit should do, and document it.
3. **`RingBuffer::make_contiguous`.** Rotate the elements in place until
   the second slice is empty, and return `&mut [T]`. Do it with at most
   `len()` moves.
4. **More of the `Vec` and `VecDeque` API:** `dedup_by`, `split_off`,
   `drain` for `RingBuffer`, `extend_from_slice` for `T: Copy`, `Hash`,
   `PartialOrd`, equality between different capacities.
5. **More `const fn`.** Find out how many functions of `ArrayVec` can be
   `const fn` on the pinned toolchain, so that a table can be built at
   compile time.
6. **An arena that runs destructors.** Add
   `alloc_with_drop<T: 'static>(&self, value: T)`, which records the value
   in the arena itself so that `reset` and `Drop` can run its destructor.
   Explain why the bound `'static` is needed.
7. **A queue for two contexts.** Split a ring buffer into a `Producer` and
   a `Consumer` half that can be used from two threads (or from a main
   loop and an interrupt handler) without a lock, with atomic indices.
   Mastery module M3 has the background. Check it with loom or with Miri's
   many-seeds mode.
8. **Property tests.** Replace the fixed-seed model tests by property
   tests that shrink a failing sequence of operations to a minimal one
   (mastery module M9).

## 12. Non-goals

The tests do not ask for these, and the reference does not have them:
growing beyond `N`; falling back to the heap; freeing single arena
allocations; running destructors in the arena; use from more than one
thread at a time; a stable layout (`repr(C)`) for foreign code;
`serde` support; performance equal to hand-tuned crates. The API is a
subset of what `Vec` and `VecDeque` offer, not a copy.

## 13. Evidence to keep

The rubric asks for evidence, not for claims. Keep these in a file
`EVIDENCE.md` next to your code:

1. The summary lines of `cargo test` and of `cargo test --release`.
2. The output of `cargo +nightly-2026-11-20 miri test`, with the Miri version
   (`cargo +nightly-2026-11-20 miri --version`).
3. The output of `cargo clippy --all-targets`.
4. An inventory of your unsafe code: for each file, the number of `unsafe`
   blocks, and for each type its invariant in one or two sentences.
5. For three functions of your choice, the place where a panic could
   happen (in a closure, a `clone`, a destructor) and what state the
   collection is in at that moment.
6. If you did stretch goal 1: the commands and their output.
