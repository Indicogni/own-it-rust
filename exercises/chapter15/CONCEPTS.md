# Chapter 15 concepts — Iterators and adapters

[Chapter guide](README.md) · [Hints](HINTS.md)

Read one section, then do its lesson. Every code block on this page is compiled and run by the course checks, so you can paste any of them into [the scratch example](../../examples/ch15_scratch.rs) and change it.

## What you will be able to do

An *iterator* is a value that hands out a sequence of items, one at a time, when asked. Rust's standard library builds almost all work on sequences from this one idea: loops, searching, summing, converting one collection into another.

After this chapter you will be able to:

- choose between borrowing, editing, draining, and consuming a collection, and say what the item type is in each case;
- build a pipeline from *adapters* (methods that return a new iterator) and end it with a *consumer* (a method that pulls the items and returns a result);
- predict how many items a pipeline pulls, and why an unfinished pipeline does nothing;
- stop early, look ahead, and continue with the same iterator;
- collect into any collection, including `Result<Vec<_>, _>`;
- write your own iterator types, return iterators from functions, and add your own adapter method to every iterator.

## Before you start

Ownership and moves (Chapter 4), references and lifetimes (Chapter 5), `Option` and `Result` (Chapters 7 and 10), slice patterns (Chapter 10), traits and generics (Chapter 11), and closures (Chapter 14).

## Time

These are estimates for reading the section, doing the lesson, and writing your notes.

| Lesson | Estimate |
| --- | --- |
| 01 Sources | 35 min |
| 02 Generating sequences | 40 min |
| 03 Mapping, filtering, flattening | 40 min |
| 04 Stopping, peeking, resuming | 50 min |
| 05 Reversal, pairing, inspection, repetition | 40 min |
| 06 Accumulators, extrema, comparisons | 40 min |
| 07 Searching and positions | 40 min |
| 08 Folds and short circuits | 45 min |
| 09 Collecting, extending, partitioning | 40 min |
| 10 Implementing a double-ended iterator | 50 min |
| 12 Writing iterators | 90 min |
| 11 Capstone | 60 min |
| R01, C01, and the drills | 2 h |

## Lesson 01 · Borrowed, mutable, owned, and draining sources

**The model.** An iterator is any type that implements the trait `Iterator`. The trait has one required method, `next`, which returns `Some(item)` for the next item and `None` when the sequence has ended.

```rust
let mut it = [10, 20].iter();
assert_eq!(it.next(), Some(&10));
assert_eq!(it.next(), Some(&20));
assert_eq!(it.next(), None);
```

A collection is not an iterator. It can *make* one, and it can make several kinds. The kind decides the item type, and the item type decides what you may do with each item. This is the ownership model from Chapters 4 and 5, applied one item at a time.

```text
call                 item type     the collection afterwards
v.iter()             &T            unchanged, still usable
v.iter_mut()         &mut T        still usable, items may be edited
v.drain(a..b)        T             still usable, the range is removed
v.into_iter()        T             moved, cannot be used again
```

A `for` loop calls `IntoIterator::into_iter` on the value after `in`. For a `Vec<T>` that is the consuming kind. `&v` and `&mut v` also implement `IntoIterator`, and they give `iter()` and `iter_mut()`. So `for x in &v` borrows, and `for x in v` consumes.

**Worked example.**

```rust
let mut names = vec![String::from("ana"), String::from("bo"), String::from("cy")];

// Borrow: each item is a `&String`.
let letters: usize = names.iter().map(|name| name.len()).sum();
assert_eq!(letters, 7);

// Borrow mutably: each item is a `&mut String`, so it can be edited in place.
for name in names.iter_mut() {
    name.make_ascii_uppercase();
}

// Drain: the first two Strings are moved out. The vector keeps the rest.
let first_two: Vec<String> = names.drain(..2).collect();
assert_eq!(first_two, ["ANA", "BO"]);
assert_eq!(names, ["CY"]);

// Consume: each item is a `String`, and `names` is gone after the loop.
let mut joined = String::new();
for name in names {
    joined.push_str(&name);
}
assert_eq!(joined, "CY");
```

`drain` is the one to remember. It borrows the vector mutably and gives you the removed elements *by value*, with their original heap buffers. When the `Drain` value is dropped, the whole range is removed from the vector, also the part you did not read.

**Mistakes people make.**

Using a vector after a `for` loop consumed it:

```rust,compile_fail,E0382
fn main() {
    let names = vec![String::from("Ana"), String::from("Bo")];
    for name in names {
        println!("{name}");
    }
    println!("{}", names.len());
}
```

```text
error[E0382]: borrow of moved value: `names`
  3 |     for name in names {
    |                 ----- `names` moved due to this implicit call to `.into_iter()`
...
  6 |     println!("{}", names.len());
    |                    ^^^^^ value borrowed here after move
help: consider iterating over a slice of the `Vec<String>`'s content to avoid moving into the `for` loop
    |
  3 |     for name in &names {
    |                 +
```

Trying to edit through a shared borrow:

```rust,compile_fail,E0594
fn main() {
    let mut stock = vec![3, 0, 7];
    for count in &stock {
        *count += 1;
    }
    println!("{stock:?}");
}
```

```text
error[E0594]: cannot assign to `*count`, which is behind a `&` reference
3 |     for count in &stock {
  |                  ------ this iterator yields `&` references
4 |         *count += 1;
  |         ^^^^^^^^^^^ `count` is a `&` reference, so it cannot be written to
help: use a mutable iterator instead
  |
3 |     for count in &mut stock {
  |                   +++
```

Using the vector while a `Drain` is still alive. The `Drain` holds the mutable borrow until its last use:

```rust,compile_fail,E0502
fn main() {
    let mut queue = vec![1, 2, 3, 4];
    let batch = queue.drain(..2);
    println!("{}", queue.len());
    println!("{:?}", batch.collect::<Vec<_>>());
}
```

```text
error[E0502]: cannot borrow `queue` as immutable because it is also borrowed as mutable
3 |     let batch = queue.drain(..2);
  |                 ----- mutable borrow occurs here
4 |     println!("{}", queue.len());
  |                    ^^^^^ immutable borrow occurs here
5 |     println!("{:?}", batch.collect::<Vec<_>>());
  |                      ----- mutable borrow later used here
```

Finish with the `Drain` first (collect it), then use the vector.

**When to use which.** Read with `iter()`. Edit in place with `iter_mut()`. Use `into_iter()` when you no longer need the collection and want the items themselves, without cloning. Use `drain` when you want some items out and the collection, with its allocated buffer, kept.

**Read more.** [`std::iter` module](https://doc.rust-lang.org/std/iter/index.html) · [`IntoIterator`](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html) · [`Vec::drain`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.drain)

## Lesson 02 · Generating finite sequences

**The model.** Not every sequence comes from a collection. Some are computed. You already know ranges such as `0..5`. The module `std::iter` has more sources:

- `once(x)` yields `x` one time, `repeat(x)` yields clones of `x` forever, and `repeat_n(x, n)` yields it `n` times;
- `from_fn(closure)` calls the closure for every `next`, and the closure returns `Some(item)` or `None`;
- `successors(first, step)` yields `first`, then `step(&previous)`, and so on, until `step` returns `None`.

Two questions matter for every generated sequence. **Where does it end?** A generator ends only when your code returns `None`, so the end must be part of the state. **What happens at the last step?** The step that would overflow or go below zero must become `None`. Checked arithmetic (`checked_add`, `checked_mul`, `checked_sub`) returns an `Option`, which fits exactly.

Generators are lazy, like all iterators: creating one runs none of your code. The closure runs when something asks for an item.

**Worked example.** The Collatz rule: halve an even number, turn an odd number `n` into `3n + 1`, and stop at 1.

```rust
use std::iter;

let steps: Vec<u64> = iter::successors(Some(6u64), |&n| match n {
    1 => None,
    n if n % 2 == 0 => Some(n / 2),
    n => n.checked_mul(3)?.checked_add(1),
})
.collect();
assert_eq!(steps, [6, 3, 10, 5, 16, 8, 4, 2, 1]);
```

The closure returns `Option<u64>`, so `?` works inside it: an overflow ends the sequence instead of producing a wrong number.

`from_fn` has no "previous item" parameter. You keep the state yourself, in variables that the closure captures. Here the state is "the value to yield now, and the one after it". `None` in the state means that the sequence has ended.

```rust
use std::iter;

// Every Fibonacci number that fits in a u8.
let mut state = (Some(0u8), Some(1u8));
let fibonacci = iter::from_fn(move || {
    let (current, next) = state;
    let value = current?;
    state = (next, next.and_then(|n| value.checked_add(n)));
    Some(value)
});
let all: Vec<u8> = fibonacci.collect();
assert_eq!(all.len(), 14);
assert_eq!(all.last(), Some(&233));
```

A simpler state, `(u8, u8)` with a checked addition, would stop one number too early: when `144 + 233` overflows there is no next *pair*, but 233 itself still has to be yielded. Storing each coming value as an `Option` solves that.

**Mistakes people make.**

Arithmetic that overflows at the last step. In a debug build this panics; in a release build it wraps around and the sequence continues with wrong numbers.

```rust,should_panic
// A mark every 60 units, in a u8.
let marks: Vec<u8> = std::iter::successors(Some(0u8), |&n| Some(n + 60))
    .take(10)
    .collect();
```

```text
attempt to add with overflow
```

The marks are 0, 60, 120, 180, 240, and the next one does not fit in a u8. Use `n.checked_add(60)` as the step. It returns `None` at the overflow, and that ends the sequence.

Collecting a sequence that never ends. This compiles, and then it uses memory until the program is stopped:

```rust,no_run
let forever: Vec<u32> = std::iter::repeat(7).collect();
```

Put a bound in front of the consumer: `take(n)`, or `take_while` with a condition that must become false.

Returning a generator whose closure borrows a local variable:

```rust,compile_fail,E0373
fn below(limit: u32) -> impl Iterator<Item = u32> {
    (0..).take_while(|&n| n < limit)
}

fn main() {
    println!("{:?}", below(3).collect::<Vec<_>>());
}
```

```text
error[E0373]: closure may outlive the current function, but it borrows `limit`, which is owned by the current function
2 |     (0..).take_while(|&n| n < limit)
  |                      ^^^^     ----- `limit` is borrowed here
  |                      |
  |                      may outlive borrowed value `limit`
help: to force the closure to take ownership of `limit` (and any other referenced variables), use the `move` keyword
  |
2 |     (0..).take_while(move |&n| n < limit)
  |                      ++++
```

The iterator leaves the function, so its closure must own what it uses. This is the `move` closure from Chapter 14.

**When to use which.** Use a range when the items are consecutive integers. Use `successors` when each item is computed from the one before. Use `from_fn` when the state is more than the previous item. When the state needs a name and methods, write your own iterator type (Lesson 10).

**Read more.** [`from_fn`](https://doc.rust-lang.org/std/iter/fn.from_fn.html) · [`successors`](https://doc.rust-lang.org/std/iter/fn.successors.html) · [`repeat_n`](https://doc.rust-lang.org/std/iter/fn.repeat_n.html) · [`u32::checked_mul`](https://doc.rust-lang.org/std/primitive.u32.html#method.checked_mul)

## Lesson 03 · Mapping, filtering, and flattening

**The model.** An *adapter* takes an iterator and returns a new iterator. A *consumer* (such as `sum`, `count`, `collect`, or a `for` loop) pulls items until it has its answer. A pipeline is a source, some adapters, and one consumer.

Adapters are lazy. Calling `map` runs nothing; it wraps the iterator in a `Map` value that remembers your closure. When the consumer asks for an item, the request travels back through the adapters to the source, and one item travels forward through each closure. Items are processed one at a time, all the way through, and no intermediate collection is built.

The adapters of this lesson:

- `map(f)` changes each item. The closure receives the item itself.
- `filter(p)` keeps the items for which `p` returns true. The closure receives a *reference* to the item, because the item must still exist afterwards.
- `filter_map(f)` does both: the closure returns an `Option`, and `None` drops the item.
- `flat_map(f)` turns each item into zero or more items: the closure returns something iterable, and the results are joined into one sequence.
- `flatten()` removes one layer when the items are already iterable.

`Option` and `Result` are iterable too. `Some(x)` and `Ok(x)` give one item; `None` and `Err(_)` give none. That is why `flatten` on a sequence of `Option`s drops the `None`s.

**Worked example.** A small configuration file, already split into lines.

```rust
let lines = ["name = demo", "# comment", "tags = a, b", "broken line", "tags = c"];

let keys: Vec<&str> = lines
    .iter()
    .filter(|line| !line.starts_with('#'))
    .filter_map(|line| line.split_once('=')) // None for a line without `=`
    .map(|(key, _value)| key.trim())
    .collect();
assert_eq!(keys, ["name", "tags", "tags"]);

let tags: Vec<&str> = lines
    .iter()
    .filter_map(|line| line.strip_prefix("tags ="))
    .flat_map(|list| list.split(',')) // one line gives several tags
    .map(str::trim)
    .collect();
assert_eq!(tags, ["a", "b", "c"]);

let cells = vec![Some(3), None, Some(5)];
let present: Vec<i32> = cells.into_iter().flatten().collect();
assert_eq!(present, [3, 5]);
```

Read a pipeline from top to bottom, and write down the item type after each line. In the first pipeline it is `&&str`, `&&str`, `(&str, &str)`, `&str`. When a pipeline does not compile, this list of types is usually where the answer is.

**Mistakes people make.**

Forgetting the consumer. This compiles with a warning, and the closure never runs:

```rust
let mut total = 0;
[1, 2, 3].iter().map(|n| total += n);
assert_eq!(total, 0);
```

```text
warning: unused `Map` that must be used
  = note: iterators are lazy and do nothing unless consumed
```

For a side effect on each item, use a `for` loop or `for_each`. For a total, use `sum`.

Forgetting that `filter` passes a reference. With `iter()` the item is already `&i32`, so the closure receives `&&i32`:

```rust,compile_fail,E0308
fn main() {
    let ages = [15, 42, 8];
    let adults: Vec<&i32> = ages.iter().filter(|age| age >= 18).collect();
    println!("{adults:?}");
}
```

```text
error[E0308]: mismatched types
3 |     let adults: Vec<&i32> = ages.iter().filter(|age| age >= 18).collect();
  |                                                             ^^ expected `&&_`, found integer
```

Take the references apart in the closure pattern, `|&&age| age >= 18`, or dereference, `**age >= 18`. Method calls such as `line.starts_with('#')` need neither, because method calls dereference for you.

Writing `filter` and then `map` with `unwrap`:

```rust
let parsed: Vec<i32> = ["4", "x", "6"]
    .iter()
    .map(|text| text.parse::<i32>())
    .filter(|result| result.is_ok())
    .map(|result| result.unwrap())
    .collect();
assert_eq!(parsed, [4, 6]);
```

It works, but the test and the `unwrap` must be kept in step by hand. `filter_map(|text| text.parse().ok())` says the same in one step and cannot panic.

**When to use which.** Use `map` when every item stays. Use `filter` when the test does not change the item. Use `filter_map` when the test and the conversion are the same operation, as with parsing. Use `flat_map` when one item becomes several. Use `flatten` when the items are already collections or `Option`s.

**Read more.** [`Iterator::map`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.map) · [`filter_map`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.filter_map) · [`flat_map`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.flat_map) · [Laziness, in the `std::iter` documentation](https://doc.rust-lang.org/std/iter/index.html#laziness)

## Lesson 04 · Stopping, peeking, and resuming

**The model.** Four adapters cut a sequence:

- `take(n)` yields at most `n` items, and after the last of them it does not ask its source again; `skip(n)` drops the first `n`.
- `take_while(p)` yields items until `p` is false for the first time; `skip_while(p)` drops items until `p` is false for the first time, and then yields everything, also later items for which `p` would be true again.

All four take the iterator *by value*. After `lines.take(2)` the variable `lines` has been moved. To continue with the same iterator, lend it instead: `by_ref()` returns `&mut I`, and a mutable reference to an iterator is itself an iterator. The adapter then consumes the borrow, and the original continues where the adapter stopped.

One detail decides many designs. `take_while` must pull an item to test it. When the test fails, that item has already left the source, and `take_while` does not yield it. The item is lost.

When you need to look at an item *without* taking it, wrap the iterator with `peekable()`. `peek()` returns a reference to the next item and leaves it in place. `next_if(p)` takes the next item only if `p` accepts it, and `next_if_eq(&x)` only if it equals `x`.

Last, `fuse()`. The `Iterator` contract does not say what `next` returns after the first `None`; a type may yield items again. `fuse()` wraps any iterator so that it returns `None` forever after the first `None`.

**Worked example.** A document has header lines, one empty line, and a body. Here it is useful that `take_while` consumes the item that stops it: the empty line disappears.

```rust
let text = "title: Notes\nauthor: Kim\n\nfirst line\nsecond line";
let mut lines = text.lines();

let header: Vec<&str> = lines.by_ref().take_while(|line| !line.is_empty()).collect();
let body: Vec<&str> = lines.collect();

assert_eq!(header, ["title: Notes", "author: Kim"]);
assert_eq!(body, ["first line", "second line"]);
```

Merging two sorted sequences needs `peek`: you must compare the two front items and take only one of them.

```rust
let mut left = [1, 4, 9].into_iter().peekable();
let mut right = [2, 3, 10].into_iter().peekable();
let mut merged = Vec::new();

loop {
    let take_left = match (left.peek(), right.peek()) {
        (Some(l), Some(r)) => l <= r,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => break,
    };
    // `extend` accepts an Option: it adds one item, or nothing.
    merged.extend(if take_left { left.next() } else { right.next() });
}
assert_eq!(merged, [1, 2, 3, 4, 9, 10]);
```

**Mistakes people make.**

Using an iterator after an adapter took it by value:

```rust,compile_fail,E0382
fn main() {
    let mut lines = ["title", "author", "", "body"].into_iter();
    let header: Vec<&str> = lines.take(2).collect();
    let rest: Vec<&str> = lines.collect();
    println!("{header:?} {rest:?}");
}
```

```text
error[E0382]: use of moved value: `lines`
   3 |     let header: Vec<&str> = lines.take(2).collect();
     |                                   ------- `lines` moved due to this method call
   4 |     let rest: Vec<&str> = lines.collect();
     |                           ^^^^^ value used here after move
help: you can `clone` the value and consume it, but this might not be your desired behavior
```

The help text offers `clone`, which would start again from the first line. What you want here is `lines.by_ref().take(2)`.

Keeping the reference from `peek` while calling `next`:

```rust,compile_fail,E0499
fn main() {
    let mut chars = "  x".chars().peekable();
    let next = chars.peek();
    chars.next();
    println!("{next:?}");
}
```

```text
error[E0499]: cannot borrow `chars` as mutable more than once at a time
3 |     let next = chars.peek();
  |                ----- first mutable borrow occurs here
4 |     chars.next();
  |     ^^^^^ second mutable borrow occurs here
5 |     println!("{next:?}");
  |                ---- first borrow later used here
```

`peek` returns a reference into the `Peekable`. Copy what you need out of it (`chars.peek().copied()`), or decide and take in one call with `next_if`.

Using `take_while` when the stopping item is still needed. A number followed by a unit is a typical case:

```rust
let mut chars = "42km".chars();
let digits: String = chars.by_ref().take_while(|c| c.is_ascii_digit()).collect();
let unit: String = chars.collect();
assert_eq!((digits.as_str(), unit.as_str()), ("42", "m")); // the `k` is gone
```

When the boundary item belongs to the next part, use `peekable`.

**When to use which.** `take` and `skip` cut by count; `take_while` and `skip_while` cut by condition. Add `by_ref()` whenever the iterator is needed again. Use `peekable` when the decision to take an item depends on the item. Use `fuse` when your code calls `next` again after a `None` and the source gives no promise.

**Read more.** [`Iterator::by_ref`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.by_ref) · [`Peekable`](https://doc.rust-lang.org/std/iter/struct.Peekable.html) · [`take_while`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.take_while) · [`fuse`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fuse)

## Lesson 05 · Reversal, pairing, inspection, and repetition

**The model.** These adapters combine or reorder sequences.

- `rev()` yields the items from the back. It needs a source that can give items from the back: the trait `DoubleEndedIterator`. Slices, vectors, and bounded ranges have it. An endless range such as `1..` does not.
- `enumerate()` pairs each item with a counter that starts at 0.
- `zip(other)` pairs the items of two sequences and stops when either one ends.
- `chain(other)` yields all items of the first sequence, then all items of the second.
- `cycle()` starts again from the beginning each time the sequence ends. It keeps a copy of the original iterator to do so, and therefore needs `Clone`.
- `inspect(f)` passes each item on unchanged and calls `f` with a reference to it on the way. It is a window into a lazy pipeline.
- `copied()` and `cloned()` turn an iterator of `&T` into an iterator of `T`. `copied` is for `Copy` types; `cloned` calls `clone`.

One rule explains every question about order: **an adapter sees only what the adapter before it yields.** `enumerate` counts the items that reach it. Put it before `rev` and the items keep their original numbers. Put it after `rev` and they are numbered in the new order.

**Worked example.** A score table with alternating row colours.

```rust
let names = ["Kim", "Lee", "Ola"];
let scores = [72, 95];
let colours = ["white", "grey"];

let rows: Vec<String> = names
    .iter()
    .zip(scores) // Ola has no score: zip ends after two pairs
    .zip(colours.iter().cycle()) // cycle supplies colours for as long as needed
    .enumerate()
    .map(|(i, ((name, score), colour))| format!("{}. {name} {score} [{colour}]", i + 1))
    .collect();
assert_eq!(rows, ["1. Kim 72 [white]", "2. Lee 95 [grey]"]);

// chain: the two parts must have the same item type.
let launch: Vec<u32> = (1..=3).rev().chain(std::iter::once(0)).collect();
assert_eq!(launch, [3, 2, 1, 0]);

// inspect: watch what a consumer pulls.
let sizes = [3, 8, 20, 5];
let mut pulled = Vec::new();
let first_big = sizes
    .iter()
    .inspect(|n| pulled.push(**n))
    .find(|n| **n > 5);
assert_eq!(first_big, Some(&8));
assert_eq!(pulled, [3, 8]); // 20 and 5 were never looked at
```

`cycle()` alone never ends. It is safe here because `zip` stops when the names end.

**Mistakes people make.**

Reversing something that cannot be walked from the back. `enumerate().rev()` must know the number of the last item before it starts, so the source must also know its exact length (`ExactSizeIterator`). The chars of a string do not: a char takes one to four bytes.

```rust,compile_fail,E0277
fn main() {
    for (i, c) in "abc".chars().enumerate().rev() {
        println!("{i} {c}");
    }
}
```

```text
error[E0277]: the trait bound `Chars<'_>: ExactSizeIterator` is not satisfied
     |
   2 |     for (i, c) in "abc".chars().enumerate().rev() {
     |                                             ^^^ the trait `ExactSizeIterator` is not implemented for `Chars<'_>`
     = note: required for `Enumerate<Chars<'_>>` to implement `DoubleEndedIterator`
```

Use `char_indices().rev()`, which gives byte positions, or collect the chars into a `Vec` first.

Calling `cycle` on an iterator that cannot be cloned:

```rust,compile_fail,E0277
fn main() {
    let mut pending = vec![1, 2];
    let forever = pending.drain(..).cycle();
    println!("{:?}", forever.take(3).collect::<Vec<_>>());
}
```

```text
error[E0277]: the trait bound `std::vec::Drain<'_, {integer}>: Clone` is not satisfied
     |
   3 |     let forever = pending.drain(..).cycle();
     |                                     ^^^^^ the trait `Clone` is not implemented for `std::vec::Drain<'_, {integer}>`
```

A `Drain` owns each item once, so it cannot start again. Cycle over a borrowing iterator instead: `pending.iter().cycle()`.

Trusting `zip` to report a length difference. It does not; the extra items are dropped without any message.

```rust
let labels = ["width", "height", "depth"];
let values = [30, 20];
assert_eq!(labels.iter().zip(values).count(), 2);
```

When different lengths mean a bug, compare the lengths before you zip.

**When to use which.** Use `enumerate` instead of a counter variable. Use `zip` instead of indexing two slices with the same index; it also removes the bounds checks. Use `chain` instead of building a temporary vector from two parts. Use `inspect` for debugging and for counting, not for the real work of the pipeline. Prefer `copied` to `cloned` when the type is `Copy`: it tells the reader that the copy is cheap.

**Read more.** [`DoubleEndedIterator`](https://doc.rust-lang.org/std/iter/trait.DoubleEndedIterator.html) · [`zip`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.zip) · [`cycle`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.cycle) · [`inspect`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.inspect)

## Lesson 06 · Accumulators, extrema, and sequence comparisons

**The model.** A consumer drives the pipeline and returns one value. This lesson covers the ones that reduce a sequence to a number, an item, or a yes/no answer.

- `count()` returns how many items there were.
- `sum()` and `product()` add or multiply. For an empty sequence they return 0 and 1.
- `min()` and `max()` return an `Option`, because an empty sequence has no smallest item. The items must have a total order (`Ord`).
- `min_by_key(f)` and `max_by_key(f)` compare a key that `f` computes from each item. `min_by(cmp)` and `max_by(cmp)` take a comparison function.
- `any(p)` is true if `p` holds for at least one item, `all(p)` if it holds for every item. Both stop as soon as the answer is known. For an empty sequence `any` is false and `all` is true: there is no item that breaks the rule.
- `eq`, `lt`, `le`, `gt`, `ge`, and `cmp` compare two sequences item by item, like words in a dictionary. The first difference decides. If one sequence ends first and all items so far were equal, the shorter one is the smaller.

**Ties.** When several items are equal, the minimum methods return the *first* of them and the maximum methods return the *last*. The standard library documents this, so you may rely on it. When you need a different winner, change the key or reverse the iterator.

**Worked example.**

```rust
use std::cmp::Ordering;

let temps = [21.5, 19.0, 23.25];

// `sum` can produce several result types, so name the one you want.
let total: f64 = temps.iter().sum();
assert_eq!(total / temps.len() as f64, 21.25);

// f64 has no total order because of NaN, so `max()` is not available.
// `total_cmp` is a comparison that does order every f64 value.
let warmest = temps.iter().copied().max_by(|a, b| a.total_cmp(b));
assert_eq!(warmest, Some(23.25));

let cities = [("Oslo", 4), ("Lima", 19), ("Rome", 19)];
let hottest = cities.iter().max_by_key(|(_, temp)| *temp);
assert_eq!(hottest, Some(&("Rome", 19))); // the last of the equal maxima

assert!(cities.iter().all(|(_, temp)| *temp > 0));
assert!(!cities.iter().any(|(name, _)| name.is_empty()));

// Version numbers must be compared part by part, as numbers.
fn parts(version: &str) -> impl Iterator<Item = u32> {
    version.split('.').filter_map(|part| part.parse().ok())
}
assert!(parts("1.10.0").gt(parts("1.9.3"))); // 10 > 9
assert!("1.10.0" < "1.9.3"); // as text, '1' < '9' decides wrongly
assert_eq!(parts("1.2").cmp(parts("1.2.0")), Ordering::Less); // a prefix is smaller
```

**Mistakes people make.**

Leaving the result type of `sum` open:

```rust,compile_fail,E0283
fn main() {
    let prices = [250, 120, 90];
    let total = prices.iter().sum();
    println!("{total}");
}
```

```text
error[E0283]: type annotations needed
     |
   3 |     let total = prices.iter().sum();
     |         ^^^^^                 --- type must be known at this point
     |
     = note: the type must implement `Sum<&i32>`
help: consider giving `total` an explicit type
     |
   3 |     let total: /* Type */ = prices.iter().sum();
     |              ++++++++++++
```

Write `let total: i32 = ...` or `.sum::<i32>()`.

Calling `max` on floating-point numbers:

```rust,compile_fail,E0277
fn main() {
    let temps = [21.5, 19.0, 23.25];
    let warmest = temps.iter().copied().max();
    println!("{warmest:?}");
}
```

```text
error[E0277]: the trait bound `{float}: Ord` is not satisfied
     |
   3 |     let warmest = temps.iter().copied().max();
     |                                         ^^^ the trait `Ord` is not implemented for `{float}`
```

Use `max_by(|a, b| a.total_cmp(b))`, as in the worked example.

Summing in a type that is too small. The sum has the type you ask for, and the additions happen in that type. In a debug build an overflow panics; in a release build it wraps.

```rust,should_panic
let sizes: [u8; 3] = [200, 50, 10];
let total: u8 = sizes.iter().sum();
```

```text
attempt to add with overflow
```

Convert each item to a wider type first: `sizes.iter().map(|&s| u32::from(s)).sum::<u32>()`.

**When to use which.** Use `min_by_key` and `max_by_key` when you compare one property of each item. Use `min_by` and `max_by` when no key can express the comparison, as with floats or with text compared without regard to case. Use the comparison methods when you compare *computed* sequences; two slices or two vectors can be compared directly with `<` and `==`, with the same dictionary rule.

**Read more.** [`Iterator::max_by_key`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.max_by_key) · [`Iterator::cmp`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.cmp) · [`f64::total_cmp`](https://doc.rust-lang.org/std/primitive.f64.html#method.total_cmp) · [`Sum`](https://doc.rust-lang.org/std/iter/trait.Sum.html)

## Lesson 07 · Searching and iterator positions

**The model.** The search consumers stop at the first item that answers the question:

- `find(p)` returns the first item for which `p` is true; `rfind(p)` searches from the back.
- `find_map(f)` returns the first `Some` that `f` produces. It is `filter_map` followed by `next`.
- `position(p)` returns the index of the first match, and `rposition(p)` the index of the last match, counted from the front.
- `nth(n)` skips `n` items and returns the next one; `nth_back(n)` does the same from the back. `last()` returns the final item.

All of them except `last` take `&mut self`. They do not consume the iterator *value*; they advance it. (`last` takes the iterator by value, because nothing is left afterwards.) This is the important difference from indexing. `v[2]` reads and changes nothing. `it.nth(2)` removes three items from the front of the iterator, and the next call starts after them. The items that a search has looked at are gone, including the one it returned.

The methods that work from the back need `DoubleEndedIterator`. `rposition` also needs `ExactSizeIterator`: it finds the item from the back but must report its index from the front, and for that it needs the length.

**Worked example.** A small log.

```rust
let log = ["INFO boot", "WARN disk 91%", "INFO ready", "WARN disk 97%"];

let first = log.iter().position(|line| line.starts_with("WARN"));
let last = log.iter().rposition(|line| line.starts_with("WARN"));
assert_eq!((first, last), (Some(1), Some(3))); // both count from the front

// find returns the first matching item; rfind returns the last one.
let newest = log.iter().rfind(|line| line.starts_with("WARN"));
assert_eq!(newest, Some(&"WARN disk 97%"));

// find_map: search and convert in one step.
let percent = log
    .iter()
    .find_map(|line| line.strip_suffix('%')?.rsplit(' ').next()?.parse::<u8>().ok());
assert_eq!(percent, Some(91));

// nth advances the iterator.
let mut letters = "abcdef".chars();
assert_eq!(letters.nth(1), Some('b')); // a is skipped
assert_eq!(letters.nth(1), Some('d')); // c is skipped: counting starts where we are
assert_eq!(letters.as_str(), "ef");
```

**Mistakes people make.**

Asking for a position from the back on a source that does not know its length:

```rust,compile_fail,E0277
fn main() {
    let path = "src/bin/main.rs";
    let last_slash = path.chars().rposition(|c| c == '/');
    println!("{last_slash:?}");
}
```

```text
error[E0277]: the trait bound `Chars<'_>: ExactSizeIterator` is not satisfied
     |
   3 |     let last_slash = path.chars().rposition(|c| c == '/');
     |                                   ^^^^^^^^^ the trait `ExactSizeIterator` is not implemented for `Chars<'_>`
```

For text, use the string methods: `path.rfind('/')` returns the byte index, which is what you need for slicing.

Comparing through two references. `find` passes a reference to the item, and with `iter()` the item is a reference already:

```rust,compile_fail,E0308
fn main() {
    let ports = [80, 8080, 443];
    let high = ports.iter().find(|port| port > 1024);
    println!("{high:?}");
}
```

```text
error[E0308]: mismatched types
3 |     let high = ports.iter().find(|port| port > 1024);
  |                                                ^^^^ expected `&&_`, found integer
```

Write `|&&port| port > 1024`. `position`, in contrast, passes the item itself, so there the closure takes `|&port|`. Read the signature in the documentation when you are not sure.

Reading a position as an index into the original data after the iterator has already moved:

```rust
let values = [5, 6, 7, 8];
let mut it = values.iter();
it.next();
let index = it.position(|&v| v == 7);
assert_eq!(index, Some(1)); // counted from where the iterator was
assert_eq!(values[2], 7); // the index in the slice is 2
```

Search on a fresh iterator when you want an index into the collection.

**When to use which.** Use `find` when you want the item, `position` when you want to index or split the collection afterwards, and `find_map` when you would otherwise write `filter_map(..).next()`. Use `any` (Lesson 06) when you only need to know whether a match exists. For a single item of a slice, index it; `nth` is for iterators that have no index.

**Read more.** [`Iterator::find_map`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.find_map) · [`position`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.position) · [`rposition`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.rposition) · [`nth`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.nth)

## Lesson 08 · Folds, short circuits, and side effects

**The model.** `fold` is the general consumer. You give it a start value, called the *accumulator*, and a closure. For every item the closure receives the accumulator and the item and returns the new accumulator. After the last item, `fold` returns the accumulator. `sum`, `count`, `max`, and `collect` can all be written as folds.

`fold` walks from the front and `rfold` from the back. For addition the direction does not matter. For operations where order matters, such as subtraction or building text, it does.

`try_fold` is a fold that can stop. Its closure returns an `Option` or a `Result`. `Some(acc)` or `Ok(acc)` continues with the new accumulator. The first `None` or `Err(e)` stops the walk at once, and `try_fold` returns it. Stopping early saves work, and it also means that side effects of later items never happen.

`try_fold` takes `&mut self`. After an early stop the iterator is still yours: the item that caused the stop has been consumed, and the items after it have not been touched.

`for_each(f)` calls `f` for every item, and `try_for_each(f)` does so until `f` returns `None` or an `Err`. They are the consumer forms of a `for` loop.

**Worked example.** Paying costs from a budget.

```rust
let costs = [30u32, 50, 40, 10];

// fold: always visits every item.
let total = costs.iter().fold(0u32, |sum, cost| sum + cost);
assert_eq!(total, 130);

// try_fold with Option: stop when the budget would go below zero.
let left = costs.iter().try_fold(100u32, |budget, &cost| budget.checked_sub(cost));
assert_eq!(left, None);

// try_fold with Result: the error can say what went wrong.
let left: Result<u32, String> = costs.iter().try_fold(100u32, |budget, &cost| {
    budget
        .checked_sub(cost)
        .ok_or(format!("{cost} does not fit into {budget}"))
});
assert_eq!(left, Err("40 does not fit into 20".to_string()));

// rfold: the last item is combined first.
let list = ["a", "b", "c"]
    .iter()
    .rfold(String::from("end"), |tail, head| format!("{head} -> {tail}"));
assert_eq!(list, "a -> b -> c -> end");

// try_for_each: run a fallible action per item, stop at the first error.
use std::fmt::Write;
let mut out = String::new();
let written = costs.iter().try_for_each(|cost| writeln!(out, "cost {cost}"));
assert!(written.is_ok());
assert_eq!(out.lines().count(), 4);
```

**Mistakes people make.**

Forgetting to return the accumulator. A closure body that ends with a statement returns `()`:

```rust,compile_fail,E0308
fn main() {
    let words = ["a", "b"];
    let joined = words.iter().fold(String::new(), |mut text, word| {
        text.push_str(word);
    });
    println!("{joined}");
}
```

```text
error[E0308]: mismatched types
3 |       let joined = words.iter().fold(String::new(), |mut text, word| {
  |  ____________________________________________________________________^
4 | |         text.push_str(word);
5 | |     });
  | |_____^ expected `String`, found `()`
help: consider returning the local binding `text`
  |
4 ~         text.push_str(word);
5 +         text
  |
```

Calling `try_fold` on an iterator that is not declared `mut`:

```rust,compile_fail,E0596
fn main() {
    let costs = [30u32, 50, 40];
    let it = costs.iter();
    let left = it.try_fold(100u32, |budget, &cost| budget.checked_sub(cost));
    println!("{left:?}");
}
```

```text
error[E0596]: cannot borrow `it` as mutable, as it is not declared as mutable
4 |     let left = it.try_fold(100u32, |budget, &cost| budget.checked_sub(cost));
  |                ^^ cannot borrow as mutable
help: consider changing this to be mutable
  |
3 |     let mut it = costs.iter();
  |         +++
```

In a chain such as `costs.iter().try_fold(..)` you do not see this, because a temporary value may be borrowed mutably.

Trying to leave `for_each` with `break`:

```rust,compile_fail,E0267
fn main() {
    let costs = [30u32, 50, 40];
    let mut budget = 100u32;
    costs.iter().for_each(|&cost| {
        if cost > budget {
            break;
        }
        budget -= cost;
    });
}
```

```text
error[E0267]: `break` inside of a closure
4 |     costs.iter().for_each(|&cost| {
  |                           ------- enclosing closure
5 |         if cost > budget {
6 |             break;
  |             ^^^^^ cannot `break` inside of a closure
```

`break` and `continue` belong to loops, and the body here is a closure. A `return` in the closure compiles, but it only ends the call for the current item; the next item still arrives. Use `try_for_each` and return `None` or an `Err` to stop, or write a `for` loop.

**When to use which.** Prefer a named consumer (`sum`, `max`, `any`) when one fits; the reader sees the intent at once. Use `fold` for an accumulation that has no name. Use `try_fold` when an item can make the whole result invalid. Use a plain `for` loop when the body needs `break`, `continue`, `return`, or `?` on the enclosing function; a `for` loop is not less idiomatic than `for_each`.

**Read more.** [`Iterator::fold`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fold) · [`try_fold`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.try_fold) · [`try_for_each`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.try_for_each) · [`DoubleEndedIterator::rfold`](https://doc.rust-lang.org/std/iter/trait.DoubleEndedIterator.html#method.rfold)

## Lesson 09 · Collections, extension, and partition

**The model.** `collect` is a consumer that builds a new collection. Which one? `collect` itself does not know. Its signature says only that the result type `B` must implement `FromIterator<Item>`, the trait for "can be built from an iterator of these items". You choose `B`, with a type annotation on the variable or with the *turbofish* syntax: `collect::<Vec<_>>()`. The `_` asks the compiler to fill in the item type.

Many types implement `FromIterator`:

- `Vec<T>`, `VecDeque<T>`, `BTreeSet<T>`, `HashSet<T>` from items of type `T`;
- `String` from `char`, `&str`, or `String` items;
- `BTreeMap<K, V>` and `HashMap<K, V>` from `(K, V)` pairs;
- `Result<C, E>` from `Result<T, E>` items, where `C` is itself a collection of `T`. The result is `Ok(collection)` if every item is `Ok`. At the first `Err`, collecting stops and that error is returned. `Option<C>` works the same way with `None`.

`extend` is the partner of `collect` for a collection that already exists. It comes from the trait `Extend`, keeps the current contents, and adds every item of the iterator. `partition(p)` splits one sequence into two collections of the same type: items for which `p` is true go to the first one. `unzip()` splits an iterator of pairs into two collections.

**Worked example.**

```rust
use std::collections::{BTreeMap, BTreeSet};

let words = ["pear", "fig", "pear", "kiwi"];

let unique: BTreeSet<&str> = words.iter().copied().collect();
assert_eq!(unique.len(), 3); // a set keeps one of each

let lengths: BTreeMap<&str, usize> = words.iter().map(|w| (*w, w.len())).collect();
assert_eq!(lengths["kiwi"], 4);

let initials: String = words.iter().filter_map(|w| w.chars().next()).collect();
assert_eq!(initials, "pfpk");

// All or nothing: one bad item makes the whole result an error.
let good: Result<Vec<u8>, _> = ["1", "2"].iter().map(|s| s.parse::<u8>()).collect();
assert_eq!(good, Ok(vec![1, 2]));
let bad: Option<Vec<u32>> = ["7", "x"].iter().map(|s| s.parse().ok()).collect();
assert_eq!(bad, None);

// extend adds to what is there.
let mut basket = vec!["plum"];
basket.extend(words.iter().copied().filter(|w| w.len() == 4));
assert_eq!(basket, ["plum", "pear", "pear", "kiwi"]);

let (short, long): (Vec<&str>, Vec<&str>) = words.iter().copied().partition(|w| w.len() <= 3);
assert_eq!((short, long), (vec!["fig"], vec!["pear", "pear", "kiwi"]));

let (names, sizes): (Vec<&str>, Vec<usize>) = lengths.into_iter().unzip();
assert_eq!((names, sizes), (vec!["fig", "kiwi", "pear"], vec![3, 4, 4]));
```

Your own types can join in. Implement `FromIterator`, and `collect` can build them:

```rust
#[derive(Debug, PartialEq)]
struct Stats {
    count: u32,
    total: u64,
}

impl FromIterator<u32> for Stats {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let mut stats = Stats { count: 0, total: 0 };
        for item in items {
            stats.count += 1;
            stats.total += u64::from(item);
        }
        stats
    }
}

let stats: Stats = [4, 6, 10].into_iter().collect();
assert_eq!(stats, Stats { count: 3, total: 20 });
```

**Mistakes people make.**

Not saying what to collect into:

```rust,compile_fail,E0283
fn main() {
    let squares = (1..4).map(|n| n * n).collect();
    println!("{squares:?}");
}
```

```text
error[E0283]: type annotations needed
     |
   2 |     let squares = (1..4).map(|n| n * n).collect();
     |         ^^^^^^^                         ------- type must be known at this point
     |
     = note: the type must implement `FromIterator<i32>`
help: consider giving `squares` an explicit type
     |
   2 |     let squares: Vec<_> = (1..4).map(|n| n * n).collect();
     |                ++++++++
```

Asking for a collection that cannot hold the items. Here the items are `Result`s, not numbers:

```rust,compile_fail,E0277
fn main() {
    let numbers: Vec<i32> = ["1", "2", "x"].iter().map(|s| s.parse::<i32>()).collect();
    println!("{numbers:?}");
}
```

```text
error[E0277]: a value of type `Vec<i32>` cannot be built from an iterator over elements of type `Result<i32, ParseIntError>`
help: the trait `FromIterator<Result<i32, ParseIntError>>` is not implemented for `Vec<i32>`
      but trait `FromIterator<i32>` is implemented for it
```

Decide what a bad item means. To fail as a whole, collect into `Result<Vec<i32>, _>`. To skip bad items, use `filter_map(|s| s.parse().ok())`.

Adding items with a `push` loop:

```rust
let mut out = vec![0u32];
for n in 1..=1000 {
    out.push(n);
}
assert_eq!(out.len(), 1001);
```

This is correct, but the vector has to grow several times, because `push` cannot know how many items follow. `out.extend(1..=1000)` asks the iterator for its `size_hint` and reserves the space once.

**When to use which.** Use `collect` to build a new collection, and `extend` to add to one you have. Collect into `Result<Vec<_>, _>` when one bad item makes the result useless; filter when bad items can be skipped. Do not collect only to iterate again: pass the iterator on, and collect once at the end.

**Read more.** [`Iterator::collect`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.collect) · [`FromIterator`](https://doc.rust-lang.org/std/iter/trait.FromIterator.html) · [`Extend`](https://doc.rust-lang.org/std/iter/trait.Extend.html) · [`partition`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.partition)

## Lesson 10 · Implementing a double-ended iterator

**The model.** To make your own iterator, write a struct that holds the state and implement `Iterator` for it: name the associated type `Item` and write `next`. Every adapter and consumer of this chapter then works on your type, because they are provided methods of the trait.

Four more traits describe what an iterator can do. Each one is a promise that other code relies on.

- `size_hint()` is a method of `Iterator` with a default body. It returns `(lower, Option<upper>)`: at least `lower` items remain and, if `upper` is `Some`, at most that many. The default, `(0, None)`, is always true. Collections use the hint to reserve memory.
- `DoubleEndedIterator` adds `next_back`. Front and back work on the *same* remaining items: an item taken from one end must never appear at the other, and when the ends meet, both return `None`.
- `ExactSizeIterator` promises that `size_hint` returns the exact count as both bounds. It then provides `len()`.
- `FusedIterator` is a marker with no methods. It promises that after the first `None`, `next` returns `None` forever.

A wrong hint must never be able to break memory safety. It is only a hint: `unsafe` code may not trust it, and safe code that trusts a wrong one gets wrong results or a panic, but not undefined behaviour.

**Worked example.** A timer that counts down. The state is two cursors, and the items still to come are the numbers above `low` up to `high`.

```rust
/// Yields high, high - 1, ..., 1.
struct Countdown {
    low: u32,  // every number <= low has been taken from the back
    high: u32, // every number > high has been taken from the front
}

impl Countdown {
    fn new(start: u32) -> Self {
        Countdown { low: 0, high: start }
    }
}

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.high == self.low {
            return None;
        }
        let value = self.high;
        self.high -= 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (self.high - self.low) as usize;
        (left, Some(left))
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        if self.high == self.low {
            return None;
        }
        self.low += 1;
        Some(self.low)
    }
}

impl ExactSizeIterator for Countdown {}
impl std::iter::FusedIterator for Countdown {}

let mut timer = Countdown::new(5);
assert_eq!(timer.len(), 5);
assert_eq!(timer.next(), Some(5));
assert_eq!(timer.next_back(), Some(1));
assert_eq!(timer.len(), 3);
assert_eq!(timer.rev().collect::<Vec<_>>(), [2, 3, 4]);
```

```text
            taken from the back        still to come       taken from the front
numbers:          1                     2    3    4                 5
                      ^ low = 1                   ^ high = 4
empty when high == low
```

Both methods test the same condition and move only their own cursor. That is the whole design: one description of "what is left", used by both ends and by `size_hint`.

A collection that you wrote does not need a new iterator type for a `for` loop. Implement `IntoIterator` for a reference to it and return an iterator that already exists:

```rust
struct Playlist {
    songs: Vec<String>,
}

impl<'a> IntoIterator for &'a Playlist {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.songs.iter()
    }
}

let playlist = Playlist { songs: vec![String::from("Intro"), String::from("Outro")] };
let mut titles = Vec::new();
for song in &playlist {
    titles.push(song.as_str());
}
assert_eq!(titles, ["Intro", "Outro"]);
```

**Mistakes people make.**

Promising an exact size without writing `size_hint`. The impl compiles, because `len` has a default body. That body checks the hint, and the default hint `(0, None)` fails the check:

```rust,should_panic
struct Countdown {
    high: u32,
}

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.high == 0 {
            return None;
        }
        self.high -= 1;
        Some(self.high + 1)
    }
}

impl ExactSizeIterator for Countdown {}

let timer = Countdown { high: 3 };
println!("{}", timer.len());
```

```text
assertion `left == right` failed
  left: None
 right: Some(0)
```

Implementing `IntoIterator` for a type that is already an iterator:

```rust,compile_fail,E0119
struct Ticks {
    left: usize,
}

impl Iterator for Ticks {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        self.left = self.left.checked_sub(1)?;
        Some(self.left)
    }
}

impl IntoIterator for Ticks {
    type Item = usize;
    type IntoIter = Ticks;
    fn into_iter(self) -> Ticks {
        self
    }
}

fn main() {}
```

```text
error[E0119]: conflicting implementations of trait `IntoIterator` for type `Ticks`
   |
13 | impl IntoIterator for Ticks {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: conflicting implementation in crate `core`:
           - impl<I> IntoIterator for I
             where I: Iterator;
```

The standard library already implements `IntoIterator` for every iterator; this is why a `for` loop accepts an iterator directly. Delete your impl.

Giving the two ends separate state. If `next` and `next_back` do not test the same "is anything left" condition, the ends pass each other and items are yielded twice. No compiler message warns you; only a test that takes from both ends does.

**When to use which.** Write a named iterator type when the state deserves a name, when you need `DoubleEndedIterator` or `ExactSizeIterator` and no adapter chain gives them, or when the type is part of a public API. For a single use, `from_fn` or an adapter chain is shorter. Lesson 12 continues with iterators that borrow.

**Read more.** [Implementing `Iterator`, in the `std::iter` documentation](https://doc.rust-lang.org/std/iter/index.html#implementing-iterator) · [`size_hint`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.size_hint) · [`ExactSizeIterator`](https://doc.rust-lang.org/std/iter/trait.ExactSizeIterator.html) · [`FusedIterator`](https://doc.rust-lang.org/std/iter/trait.FusedIterator.html)

## Lesson 12 · Writing iterators: return types, both ends, and your own adapter

Do this lesson before the capstone. It assumes Lesson 10, trait objects from Chapter 11, and `move` closures from Chapter 14.

**Returning an iterator.** The type of an adapter chain is long, and when it contains a closure it cannot be written at all. A function therefore returns `impl Iterator<Item = T>`: "one specific type that implements `Iterator`; the caller does not need its name". The call costs nothing extra, because the compiler knows the real type. In edition 2024 the hidden type may borrow from any argument without extra syntax.

```rust
fn evens(values: &[u32]) -> impl Iterator<Item = u32> {
    values.iter().copied().filter(|v| v % 2 == 0)
}
assert_eq!(evens(&[1, 2, 4, 7]).sum::<u32>(), 6);
```

Calling `evens` runs the body once and builds the pipeline. No item is looked at until the caller pulls.

`impl Iterator` means *one* type. Two branches that build different chains do not fit:

```rust,compile_fail,E0308
fn in_order(values: &[i32], reversed: bool) -> impl Iterator<Item = &i32> {
    if reversed {
        values.iter().rev()
    } else {
        values.iter()
    }
}

fn main() {
    println!("{:?}", in_order(&[1, 2], true).collect::<Vec<_>>());
}
```

```text
error[E0308]: `if` and `else` have incompatible types
2 | /     if reversed {
3 | |         values.iter().rev()
  | |         ------------------- expected because of this
4 | |     } else {
5 | |         values.iter()
  | |         ^^^^^^^^^^^^^ expected `Rev<Iter<'_, i32>>`, found `Iter<'_, i32>`
6 | |     }
  | |_____- `if` and `else` have incompatible types
help: you could change the return type to be a boxed trait object
```

The fix is a trait object: put each iterator in a `Box`, and both become `Box<dyn Iterator<Item = &i32>>`. That costs one heap allocation and a dynamic call for each `next`. There is one more step. A `Box<dyn Trait>` in a return type is assumed to borrow nothing (`'static`) unless you say otherwise:

```rust,compile_fail
fn in_order(values: &[i32], reversed: bool) -> Box<dyn Iterator<Item = &i32>> {
    if reversed {
        Box::new(values.iter().rev())
    } else {
        Box::new(values.iter())
    }
}

fn main() {
    println!("{:?}", in_order(&[1, 2], true).collect::<Vec<_>>());
}
```

```text
error: lifetime may not live long enough
1 |   fn in_order(values: &[i32], reversed: bool) -> Box<dyn Iterator<Item = &i32>> {
  |                       - let's call the lifetime of this reference `'1`
...
  | |_____^ returning this value requires that `'1` must outlive `'static`
help: to declare that the trait object captures data from argument `values`, you can add an explicit `'_` lifetime bound
  |
1 | fn in_order(values: &[i32], reversed: bool) -> Box<dyn Iterator<Item = &i32> + '_> {
  |                                                                              ++++
```

**An iterator that borrows, with two ends.** The struct holds a slice, so it needs a lifetime parameter, and its items carry that same lifetime. This one yields overlapping neighbours: `[a, b, c]` gives `(a, b)` and `(b, c)`.

```rust
struct Neighbours<'a, T> {
    rest: &'a [T],
}

impl<'a, T> Iterator for Neighbours<'a, T> {
    type Item = (&'a T, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let [first, second, ..] = self.rest else {
            return None;
        };
        self.rest = &self.rest[1..];
        Some((first, second))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.rest.len().saturating_sub(1);
        (left, Some(left))
    }
}

impl<T> DoubleEndedIterator for Neighbours<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let [.., first, second] = self.rest else {
            return None;
        };
        self.rest = &self.rest[..self.rest.len() - 1];
        Some((first, second))
    }
}

impl<T> ExactSizeIterator for Neighbours<'_, T> {}

let heights = [3, 5, 4, 9];
let mut steps = Neighbours { rest: &heights };
assert_eq!(steps.len(), 3);
assert_eq!(steps.next_back(), Some((&4, &9)));
assert_eq!(steps.next(), Some((&3, &5)));
assert_eq!(steps.collect::<Vec<_>>(), [(&5, &4)]);
```

```text
heights:            3   5   4   9     rest is a window on the caller's slice
after next_back:   [3   5   4]        shortened at the back
after next:            [5   4]        shortened at the front; nothing is copied
```

As in Lesson 10, one piece of state (`rest`) describes what is left, and both ends shorten it. Before you design such a type, decide what each end does when the leftover does not divide evenly.

**Your own adapter.** You cannot add a method to `Iterator`: the trait belongs to the standard library. You can write a new trait with one provided method and implement it for every iterator. This is the *extension trait* pattern from Chapter 11.

```rust
/// Yields the difference between each item and the one before it.
struct Deltas<I> {
    inner: I,
    previous: Option<i64>,
}

impl<I: Iterator<Item = i64>> Iterator for Deltas<I> {
    type Item = i64;

    fn next(&mut self) -> Option<i64> {
        if self.previous.is_none() {
            self.previous = Some(self.inner.next()?);
        }
        let current = self.inner.next()?;
        let previous = self.previous.replace(current)?;
        Some(current - previous)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // The first inner item produces no output, unless it was already taken.
        let waiting = usize::from(self.previous.is_none());
        let (lower, upper) = self.inner.size_hint();
        (lower.saturating_sub(waiting), upper.map(|n| n.saturating_sub(waiting)))
    }
}

trait DeltasExt: Iterator<Item = i64> + Sized {
    fn deltas(self) -> Deltas<Self> {
        Deltas { inner: self, previous: None }
    }
}

// The blanket impl: every iterator of i64 gets the method.
impl<I: Iterator<Item = i64>> DeltasExt for I {}

let readings = [10i64, 13, 13, 9];
let changes: Vec<i64> = readings.iter().copied().deltas().collect();
assert_eq!(changes, [3, 0, -4]);
assert_eq!(readings.iter().copied().deltas().size_hint(), (3, Some(3)));
```

An adapter owns the iterator it wraps, does nothing until `next` is called, and passes on an honest `size_hint`. A caller in another module must import the trait first; otherwise the compiler reports E0599 with the help text `items from traits can only be used if the trait is in scope`.

**When to use which.**

| Return type | Use it when | Cost |
| --- | --- | --- |
| `impl Iterator<Item = T>` | there is one concrete type; this is the normal choice | none |
| `Box<dyn Iterator<Item = T> + '_>` | branches build different types, or the iterator is stored in a struct field or a collection | one allocation, a dynamic call for each item |
| a named struct | callers must name the type, or no adapter chain behaves as you need and you write `next` yourself | more code |

An `impl` return type can promise more than `Iterator`. List the traits that callers may use, for example `impl DoubleEndedIterator<Item = T> + ExactSizeIterator`; the type that you return must implement all of them.

An iterator whose items borrow from the iterator itself (a *lending* iterator) cannot be written with the `Iterator` trait. The mastery track covers it in [`mastery/m1_types`](../../mastery/m1_types/README.md), together with how trait objects work inside.

**Read more.** [`impl Trait` in the Reference](https://doc.rust-lang.org/reference/types/impl-trait.html) · [Edition 2024: lifetime capture rules](https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html) · [Default trait object lifetimes](https://doc.rust-lang.org/reference/lifetime-elision.html#default-trait-object-lifetimes) · [`Peekable::next_if_eq`](https://doc.rust-lang.org/std/iter/struct.Peekable.html#method.next_if_eq)

## Lesson 11 · Capstone: a lazy measurement report

The capstone combines the chapter: a generic source, a pipeline that you return from a function, a strict rule about how much input may be read, and consumers that build a report. This section describes an approach. It does not contain the solution.

**Read the contract as a list of stages.** Each sentence of the contract for `rows` is one adapter. Write the stages on paper first, and next to each stage write the item type that leaves it. Then decide the order by asking, for each rule, *what must still be present when this rule is applied?* Line numbers refer to the source text, so the numbering stage must see every source line. The limit counts rows that passed every test, so the stage that enforces it must come after all tests.

**Keep it one pipeline.** The function returns `impl Iterator`, so there is no place for a `collect` in the middle, and the tests count how many lines are pulled from the source. Two facts from earlier lessons are enough to pass them. First, building a pipeline pulls nothing (Lesson 03). Second, an adapter that has yielded its last item does not ask its source again (Lesson 04). If your pipeline reads a line too many, find the stage that needs one more item to learn that it has finished, and replace it.

**Parsing.** A line can be wrong in three ways: it is blank, it is a comment, or it is not a number. Decide for each one which stage removes it, and remember from Lesson 03 which adapter both tests and converts. The closure that compares with `minimum` leaves the function inside the returned iterator, so it must own what it uses (Lesson 02).

**The report.** `selected_rows` is the only place that collects. `report` then reads the collected rows more than once, which is fine: a `Vec` can make as many iterators as you ask for. Two details need care. The sum must not overflow, so choose the type in which the additions happen (Lesson 06). And the mean of no rows does not exist; dividing by a zero count in floating point does not panic, it gives `NaN`, which is why the contract asks for `None`.

**Check yourself before running the tests.** Answer `p_predictions` on paper, then trace this input by hand with `minimum = 5` and `limit = 2`, and write down which lines are read:

```text
line 1:   "# readings"
line 2:   "7"
line 3:   ""
line 4:   "x"
line 5:   "5"
line 6:   "9"
```

The rows are `(2, 7)` and `(5, 5)`, and line 6 must never be read.

**Read more.** [`Iterator::enumerate`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.enumerate) · [`Iterator::take`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.take) · [`str::lines`](https://doc.rust-lang.org/std/primitive.str.html#method.lines)
