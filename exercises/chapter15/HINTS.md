# Chapter 15 hints — Iterators and adapters

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Try [CONCEPTS.md](CONCEPTS.md) and the compiler diagnostics first. Open one level at a time. *Check your predictions* is for after the `p_predictions` test has given its verdict; *Check your explanation* is for after you have written your own in NOTES.md.

## 01 · Borrowed, mutable, owned, and draining sources

<details><summary>Hint 1</summary>

Choose an iteration source based on whether values should be borrowed, mutably borrowed, or transferred.

</details>
<details><summary>Hint 2</summary>

Use iter() with a widening map for the sum, iter_mut() for editing, and drain(range).collect() for removing owned elements. Collect the drained items before you move the vector into the result.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `[1, 5]`: dropping a `Drain` removes its whole range, 1..4, also the items that were never read. P2 is `(true, false)`: `into_iter` moves each `String`, which keeps its heap buffer; `to_vec` clones, and every clone gets a new buffer. P3 is `0`: `map` and `sum` keep their state on the stack, and no intermediate collection exists.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`iter()` yields `&T`, `iter_mut()` yields `&mut T`, and `into_iter()` on a `Vec<T>` yields `T`. Each is a separate type because each holds a different kind of access: many shared borrows can coexist, a mutable borrow must be unique, and an owning iterator has taken the vector itself. So reading and editing use different iterators, and the borrow checker enforces the rules for each. `drain(range)` borrows the vector mutably and hands out the removed elements *by value*: the caller owns them, with their original heap buffers. When the Drain is dropped, the whole range is removed even if it was not fully consumed, and the tail shifts down.

</details>

## 02 · Generating finite sequences

<details><summary>Hint 1</summary>

from_fn calls a closure for each next request. successors yields a seed then repeatedly applies a function returning Option.

</details>
<details><summary>Hint 2</summary>

Use checked_mul in successors to stop at u32 overflow, then take_while for the limit. The take_while closure leaves the function inside the returned iterator, so it needs `move`. For countdown, keep Option state to include zero exactly once: `checked_sub(1)` on 0 gives the None you need.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `(0, 3)`: creating the generator calls nothing, and `take(3)` asks for exactly three items. P2 is `(6, 243)`: 1, 3, 9, 27, 81, 243; the next power, 729, does not fit in a u8, so `checked_mul` returns None. P3 is `(0, 3, 3)`: creation runs no step. The items are 10, 6, 2. The step runs once for each of them: twice it returns Some, and for 2 it returns None, because 2 − 4 does not fit in a u8.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A generator ends when its closure returns `None`, so termination has to be built into the state: `successors` stops when the step returns `None`, and `from_fn` stops when its closure does. Use checked arithmetic (`checked_mul`, `checked_sub`) so the step that *would* overflow becomes `None` instead of wrapping or panicking. `take_while` then applies the limit. For the countdown, `Option<u8>` state records "nothing left" separately from the value 0, so 0 is yielded once and the iterator stays exhausted. Both generators are lazy: nothing runs until something calls `next`. The moment at which `successors` runs its step for an item, when that item is yielded or when the next one is requested, is a detail of the standard library that the documentation does not fix; do not put side effects there that depend on it.

</details>

## 03 · Mapping, filtering, and flattening

<details><summary>Hint 1</summary>

filter changes which items remain; map changes their shape; flat_map turns each item into zero or more items.

</details>
<details><summary>Hint 2</summary>

Parse with .ok(), filter nonnegative values, and split each line with split_whitespace. Flatten the Option layer rather than inventing placeholder values.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `(0, 1, 3)`: the closure runs only when an item is pulled, once for `next` and twice more for the two items that `count` pulls. P2 is `[1, 3]`: a `Result` is iterable, with one item for `Ok` and none for `Err`, so `flat_map` drops the failed parse. P3 is `(4, 3)`: `split(' ')` cuts at every single space and keeps the empty piece between the two spaces; `split_whitespace` treats a run of whitespace as one separator.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`filter` drops items whose predicate is false; `filter_map` drops items whose closure returns `None`, so invalid tokens vanish at the parse step and negatives vanish at the filter. `flat_map` replaces each item with zero or more items, so empty lines contribute nothing. `Option<T>` implements `IntoIterator` with zero or one item, so flattening an Option removes the `None` layer without inventing a placeholder; `Result` works the same way, with `Err` yielding nothing. Every adapter is lazy: its closure runs only when a consumer pulls an item through it.

</details>

## 04 · Stopping, peeking, and resuming

<details><summary>Hint 1</summary>

peek can examine the next item while keeping it available for a later consumer.

</details>
<details><summary>Hint 2</summary>

Use skip_while then take_while for the active region. For take_digits, loop on `chars.next_if(..)`: it takes the next char only when the test accepts it, so the first non-digit is never removed. A loop that calls `peek` must also call `next`, or it never ends.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `(vec![1, 2], Some(3))`: `take_while` had to pull the 0 to test it, and that 0 is lost; the original iterator continues at 3. P2 is `3`: `skip_while` drops only the leading zeros, so 1, 0, 2 remain. P3 is `(Some(1), None, Some(3))`: nothing in the `Iterator` contract forbids items after a `None`; only `fuse()` or `FusedIterator` promises that.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Adapters like `take_while` take `self` by value, so calling them on the iterator itself would consume it. `by_ref()` returns `&mut I`, which is also an iterator, so the adapter consumes the borrow and the original iterator remains usable, at whatever position the adapter left it. `take_while` must inspect the first item that fails the predicate, so that item is consumed and lost; `peekable` lets you look at the next item without consuming it. An ordinary iterator may return `Some` again after `None`; `fuse()` guarantees that once it returns `None`, it always will.

</details>

## 05 · Reversal, pairing, inspection, and repetition

<details><summary>Hint 1</summary>

Adapter order changes semantics. zip ends at the shorter input, and cycle repeats only when asked for more items.

</details>
<details><summary>Hint 2</summary>

Enumerate before reversing to keep original indices. Zip the values with `palette.iter().cycle()`; an empty palette cycles to an empty iterator, so zip yields nothing and no special case is needed. Chain provides one iterator over compatible Item types.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `Some(('c', 3))`: `zip` stops when the chars end, so the endless range is cut after three items. P2 is `Some((0, &"c"))`: `enumerate` numbers the items in the order in which they reach it, and after `rev` the first one is "c". P3 is `(Some(4), 4)`: `find` stops pulling at the first match, so `inspect` saw 1, 2, 3, and 4 only.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`rev` and `rposition` need `DoubleEndedIterator`, so they can pull from the back; an unbounded range has no back, so `(1..).rev()` does not compile. `cycle` needs `Clone`, because it restarts from a saved copy of the original iterator. `enumerate` numbers the items it receives, so `enumerate().rev()` keeps original indices while `rev().enumerate()` numbers from the back. `zip` stops when either side ends. `inspect` runs a side effect for each item that actually flows past it, which makes laziness visible: a counter in `inspect` shows how far a consumer pulled.

</details>

## 06 · Accumulators, extrema, and sequence comparisons

<details><summary>Hint 1</summary>

Consumers drive the iterator to exhaustion or until their answer is known. Empty cases often follow algebraic identities.

</details>
<details><summary>Hint 2</summary>

Use explicit result types for sum/product. Choose the first shortest word via min_by_key with `chars().count()` as the key. For compare, turn both arguments into iterators and call `Iterator::cmp`; it stops at the first difference by itself.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `(0, 1, None)`: the empty sum is 0, the empty product is 1, and an empty sequence has no maximum. P2 is `(Some("cc"), Some("a"))`: among equal keys, the maximum methods return the last item and the minimum methods the first. P3 is `(true, false, true)`: a sequence that is a prefix of another is smaller; `any` over nothing finds no example; `all` over nothing finds no counter-example.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

With ties, more than one item is "the minimum", so the result depends on a rule the caller must be able to rely on. The standard library documents it: `min_by_key` and `min_by` return the *first* of equal elements, `max_by_key` and `max_by` the *last*. `max_by_key` compares a key extracted from each item, using `Ord` on the key; `max_by` takes a comparison closure, which lets you compare in ways no key expresses, such as `f64::total_cmp` or case-insensitive text. Empty input gives `None` for extrema, while `sum` and `product` return their identities, 0 and 1.

</details>

## 07 · Searching and iterator positions

<details><summary>Hint 1</summary>

Position methods consume inspected items. ExactSizeIterator supports translating reverse positions to forward indices.

</details>
<details><summary>Hint 2</summary>

Use position/rposition separately on fresh iterators; use find_map to stop at the first successful parse. nth and nth_back skip relative to current ends. `rfind` takes `&mut self`, so the parameter of last_negative is declared `mut`; the closure receives a reference to an item that is itself a reference.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `(Some(2), Some(5), 4)`: the first `nth(2)` consumes 0, 1, 2; the second counts from 3 and returns 5; 6, 7, 8, 9 are left. P2 is `(Some(0), Some(3))`: `position` counts from where the iterator stands, and after one `next` the 8 is its first item. `rposition` reports an index counted from the front of the iterator it was called on. P3 is `(Some(5), Some(6))`: `find` consumes the item it returns, so the range continues after it.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Indexing reads a position without changing anything. Searching methods (`position`, `find`, `nth`) *consume* every item they inspect, so later calls start from where the last one stopped: `nth(2)` twice yields the 3rd and 6th items. `rposition` needs `ExactSizeIterator` as well as `DoubleEndedIterator`, so it can convert "k items from the back" into an index from the front; reporting original indices means the result can be used to index the slice. `find_map` combines search and transformation, stopping at the first `Some`.

</details>

## 08 · Folds, short circuits, and side effects

<details><summary>Hint 1</summary>

fold threads an accumulator through each item; try_fold additionally permits early return through Result or Option.

</details>
<details><summary>Hint 2</summary>

Use checked_add with try_fold for overflow; `checked_add` already returns the Option that try_fold wants. For audit, push before the test so the rejected input is included, then stop with Err. For sum_and_remainder, keep the iterator in a `mut` variable: try_fold borrows it, and afterwards it still holds the tail.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `(123, 321)`: `fold` combines from the front and `rfold` from the back, and building a number from digits depends on the order. P2 is `(None, 3)`: 100 + 100 fits, the third addition overflows, and the fourth item is never visited. P3 is `(None, 2)`: the item that caused the failure (255) was consumed; 3 and 4 are still in the iterator.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A fallible consumer stops at the first failure because the answer is already known and later work is wasted, or harmful when the closure has side effects. `try_fold` and `try_for_each` return the first `Err` or `None`, and they consume the failing item but leave the rest in the iterator. `fold` combines from the front, `rfold` from the back; for non-commutative operations such as digit building or string nesting, the direction changes the result. `try_rfold` short-circuits from the back, so the items nearest the right end run first.

</details>

## 09 · Collections, extension, and partition

<details><summary>Hint 1</summary>

The destination type chooses FromIterator. Extend modifies an existing collection instead of creating a new one.

</details>
<details><summary>Hint 2</summary>

Use Result collection for all-or-nothing parsing. Partition copies into even/odd vectors. Extend the existing vector with the supplied iterator.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `(Err("x"), 2)`: collecting into a `Result` stops at the first `Err`, so "y" is never parsed. P2 is `[1, 2, 3]`: the destination decides what the items become, and a `BTreeSet` keeps one of each value in sorted order. P3 is `(None, Some(4))`: summing `Option`s gives `None` as soon as one item is `None`, and otherwise the sum of the values.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`collect` creates a new collection through `FromIterator`, and the destination type (named in an annotation or the turbofish) decides which one: Vec, String, BTreeSet, or `Result<Vec<_>, E>`, which stops at the first `Err`. `extend` adds to a collection that already exists through `Extend`, keeping its contents, and can reserve space from the source's `size_hint`. `partition` sends each item to one of two collections by a predicate, so both outputs must be the same type, and each keeps the input order.

</details>

## 10 · Implementing a double-ended iterator

<details><summary>Hint 1</summary>

Use a half-open interval [front,back), where back is excluded and length is back-front.

</details>
<details><summary>Hint 2</summary>

For next, return front then increment. For next_back, decrement back then return it. Both first check front<back.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `((0, None), (0, Some(10)))`: `from_fn` cannot know anything about its closure, so it gives the weakest hint. `filter` may drop every item or none, so the lower bound is 0 and the upper bound is the source's. P2 is `(Some(1), Some(3), Some(2), None)`: both ends take from the same remaining items, and after 1, 3, and 2 nothing is left. P3 is `(5, Some(5))`: `chain` adds the hints of its two parts.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`size_hint` may be loose: `(0, None)` is always a valid hint, so callers can use it only for optimizations such as pre-allocation. `ExactSizeIterator` promises the exact count, and `len()` and `rposition` rely on that, so it must stay exact after every `next` and `next_back`. A half-open interval `[front, back)` makes both ends share one remaining range: the iterator is empty exactly when `front == back`, and the two ends cannot yield the same value twice. Implementing `FusedIterator` promises that after `None`, every later call also returns `None`; callers and adapters such as `fuse` may rely on it.

</details>

## 12 · Writing iterators: return types, both ends, and your own adapter

<details><summary>Hint 1</summary>

A and B are short: build the adapter chain and return it. In A the closure must own `min`. In B each branch needs its own `Box::new(..)`. For Pairs, decide first what "the items still to come" are, as one piece of state that both ends shorten; think about a slice with an odd number of elements before you write `next_back`. For DedupRuns, take the first item of the run, then remove its equal followers.

</details>
<details><summary>Hint 2</summary>

Pairs: keep a slice whose length is always even, and cut the unpaired element off once, when the value is built. Then `next` matches `[first, second, rest @ ..]`, `next_back` matches `[rest @ .., first, second]`, and the length is `rest.len() / 2`. Write a small constructor and call it from `pairs`. DedupRuns: `let first = self.inner.next()?;`, then loop while `self.inner.next_if_eq(&first)` returns Some. For size_hint, start from `self.inner.size_hint()`; `Peekable` already counts an item that was peeked. The extension method builds the struct with `self.peekable()`.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `(100, 101, 103)`: the function body runs once, at the call, and adds 100. The closure runs once for each item that is pulled: once for `next`, twice more for `sum`. P2 is `(Some(40), Some(10), 2)`: `map` over a slice iterator still works from both ends and still knows its length. P3 is `(Some(&[5]), Some(&[4, 5]))`: `chunks` starts cutting at the front, so the short chunk is at the back; `rchunks` starts cutting at the back, so its first chunk is complete. Which end you cut from decides where the leftover is.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Return `impl Iterator` when the function builds one concrete type: the caller gets the real type without its name, with no allocation and no dynamic calls. Return `Box<dyn Iterator>` when branches build different types, or when the iterator must be stored somewhere that needs one nameable type; the price is one allocation and a dynamic call for each item, and the box needs `+ '_` (or a named lifetime) when the iterator borrows. Write a named struct when callers must name the type or when you promise more traits. A method can be added to every iterator only through a new trait: you cannot add methods to a trait that another crate defines, and "every iterator" is not one type on which you could write an inherent method. An extension trait with a provided method, plus a blanket `impl<I: Iterator> Ext for I {}`, makes the method available on every iterator once the trait is in scope.

</details>

## 11 · Capstone: a lazy measurement report

<details><summary>Hint 1</summary>

Compose one pipeline whose stages match the contract in order. Write the item type after each stage before you write code.

</details>
<details><summary>Hint 2</summary>

Enumerate original lines before filtering so the line numbers stay original, and add 1. Trim and parse inside one filter_map that returns the (number, value) pair. Compare with the minimum in a `move` closure. Put `take(limit)` last: it stops asking its source as soon as it has yielded `limit` rows. Compute the mean only after checking for empty input.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `(0, 3)`: `take(0)` never asks its source. `take(2)` asks until it has two rows: "4" is kept, "x" is dropped, "7" is kept, and then it stops, so "9" and "1" are never read. P2 is `(vec![1, 3], vec![0, 1])`: `enumerate` numbers what reaches it, so the order of the two adapters decides whether the numbers are source positions or positions among the survivors. P3 is `(true, false, false)`: 0.0 / 0.0 is NaN, and every comparison with NaN is false.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`enumerate` numbers the items it receives, so it must come first, while every source line is still present; numbering after filtering would number the survivors instead. The rest of the pipeline only describes work: trim, skip comments, parse with `filter_map`, keep values at or above the minimum, then `take(limit)`. Nothing runs until a consumer starts pulling, and `take` stops pulling after `limit` rows, so later lines are never read. `selected_rows` is the one place that collects. `report` then runs its consumers over the collected rows: `sum` over widened i64 values, and the mean only when there is at least one row.

</details>

## R01 · Spaced review of Chapters 10–14

<details><summary>Hint 1</summary>

A `move` closure owns its captured `best`, and `filter` calls it once per item, so it can update that state on every call. `TryFrom` returns `Result`, and collecting an iterator of Results into `Result<Vec<_>, _>` stops at the first Err. `?` works inside a `try_fold` closure that returns Option.

</details>
<details><summary>Hint 2</summary>

Start `best` as `None`, not `i32::MIN`, or a leading `i32::MIN` would be rejected; `Option::is_none_or` reads well here. `(0..=100).contains(&value)` checks the range before the `as u8` cast, or let `u8::try_from` do the conversion. For eval: `try_fold(Vec::new(), |mut stack, op| { match *op { ... } Some(stack) })`, pop with `stack.pop()?`, then accept only `[v]` with a slice pattern.

</details>
<details><summary>Check your predictions (after running the test)</summary>

P1 is `(6, 6, 0)`: the `move` closure holds its own copy of `total`. `add(5)` makes that copy 5. A closure whose captures are all `Copy` is `Copy` itself, so `copy` starts as a second closure with state 5; each then adds 1 to its own state. The outer `total` was never touched. P2 is `(true, 44)`: 300 does not fit in a u8, so `try_into` fails, while `as` keeps the low eight bits, 300 − 256. P3 is `(true, false)`: a derived ordering on an enum follows the order of the variants, and tuples compare their first fields first, so 2 < 1 decides.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one function invents a result for empty input, one indexes past the end of a slice. The other problems are intermediate collections: two functions build a Vec they don't need.

</details>
<details><summary>Hint 2</summary>

`filter_map(|t| t.trim().parse().ok())` replaces filter + unwrap; fold a `(sum, count)` pair and return `None` when the count is 0. `zip` stops at the shorter slice. For distinct words, `HashSet::insert` returns false for duplicates, `take(n)` stops early, and returning `Vec<&str>` avoids cloning.

</details>
<details><summary>Check your review</summary>

`average` returns `Some(NaN)` for empty input (0.0 / 0.0) instead of None. It filters on `is_ok()` and then `unwrap`s; clippy flags that pair and suggests `flatten()`, and `filter_map(|t| t.trim().parse().ok())` folds the parse in as well. It also collects a Vec only to sum it and take its length; one fold over `(sum, count)` needs no allocation. `first_distinct` clones the whole input, scans `seen` linearly for every word (quadratic), and reads all the words even when the first `n` are already known; it can take `&[String]` (clippy flags the `&Vec` parameter), return borrowed `&str`s, and stop with `take(n)`. `scoreboard` indexes `scores[i]` for every name and panics when scores are shorter; `names.iter().zip(scores).map(...).collect()` follows the contract and allocates the output once.

</details>

## Drills

Run `cargo drill ch15`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · An adapter nobody consumes</summary>Panics at the assertion: `map` only builds a lazy adapter, and nothing pulls items through it, so the closure never runs (rustc warns that the iterator is unused). Use a consumer: `for_each`, or a plain `for` loop for side effects.</details>
<details><summary>d02 · Looping over a vector by value</summary>E0382. `for name in names` calls `names.into_iter()`, which consumes the vector. Loop over `&names` to borrow it.</details>
<details><summary>d03 · collect without a destination</summary>E0283. `collect` can build any `FromIterator` type, and nothing here says which one. Annotate the binding (`let doubled: Vec<i32>`) or use the turbofish, `collect::<Vec<_>>()`.</details>
<details><summary>d04 · What filter's closure receives</summary>E0308. `iter()` yields `&i32`, and `filter` passes a reference to each item, so `s` is `&&i32` and cannot be compared with an integer. Dereference (`**s > 5`) or destructure in the pattern: `|&&s| s > 5`.</details>
<details><summary>d05 · Reversing an endless range</summary>E0277. `rev` needs `DoubleEndedIterator`, and `1..` has no last element to start from. Give the range an end: `(1..=10).rev()`.</details>
<details><summary>d06 · The length of a chain</summary>E0599. `Chain` does not implement `ExactSizeIterator`, because the sum of two lengths could overflow `usize`, so there is no `len()`. Use `count()`, which consumes the iterator, or `size_hint()`, which here is `(5, Some(5))`.</details>
<details><summary>d07 · Growing a vector while iterating it</summary>E0502. The loop holds a shared borrow of `queue` for its whole duration, and `push` needs a mutable one. Pushing could reallocate the buffer the iterator points into. Decide first, then push after the loop (or collect the additions and `extend` afterward).</details>
<details><summary>d08 · Collecting borrowed items into owned ones</summary>E0277. `iter()` over a `Vec<String>` yields `&String`, and a `Vec<String>` cannot be built from references. Use `.cloned()` to get owned Strings, or collect into `Vec<&String>` if borrowing is enough.</details>
<details><summary>d09 · Resuming after by_ref</summary>Compiles. `by_ref()` lends the iterator to `take(1)`, so `lines` is still usable afterward and continues from the second item.</details>
<details><summary>d10 · A computed step of zero</summary>Panics: integer division gives `3 / 4 == 0`, and `step_by(0)` panics because the iterator would never advance. Guard the computed value, for example `(values.len() / 4).max(1)`.</details>

## Mastery check: model answers

Answer the questions in the [chapter guide](README.md#mastery-check) from memory first, in NOTES.md. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**Laziness and the ownership of Item.** An adapter such as `map` only wraps the iterator and stores the closure; no closure runs until a consumer (`collect`, `sum`, a `for` loop) calls `next`, and then one item at a time travels through the whole pipeline. `iter()` yields `&T`: the collection is borrowed and stays usable. `iter_mut()` yields `&mut T`: the collection is borrowed exclusively and can be edited in place. `into_iter()` on an owned collection yields `T`: the collection is moved into the iterator, and each item is moved to whoever receives it.

**Short-circuit and consumption.** `find`, `position`, `any`, `all`, `try_fold`, `try_for_each`, and `take` stop pulling as soon as the answer is known; `fold`, `sum`, `count`, `max`, `last`, and `for_each` visit everything. The searching methods take `&mut self` and advance the iterator: every item that they looked at, including the one that stopped them, is gone, and the items after it are still there. `take_while` also consumes the item that stops it, and it does not yield that item.

**Consistent Iterator, DoubleEndedIterator, ExactSizeIterator, FusedIterator.** Keep one description of the remaining items, for example two cursors or one slice, and let `next`, `next_back`, and `size_hint` all read and shorten that same state. Then an item can never be yielded from both ends, both ends return `None` at the same moment, and the hint is exact after every call. `ExactSizeIterator` needs the exact `size_hint`; its `len` checks it. `FusedIterator` may be implemented only if `next` keeps returning `None` after the first `None`, which follows when "empty" is a state that no call leaves.

**by_ref and peekable.** `by_ref()` lends the iterator to an adapter that would otherwise take it by value, so the iterator can be used again afterwards. But `by_ref().take_while(..)` still consumes the first item that fails the test, so a delimiter is lost. When the delimiter must stay, wrap the iterator with `peekable()` and take items with `next_if`, or look with `peek` and call `next` only for the items you want.

**Why pipelines need no intermediate collections.** Each adapter is a small struct that holds the previous iterator and, perhaps, a closure. `next` on the outer adapter calls `next` on the inner one, so one item at a time is pulled through all stages, and the state lives on the stack. Memory is allocated only where a consumer builds a collection, normally once, at the final `collect`.

**Choosing a return type and adding an adapter.** Return `impl Iterator<Item = T>` for one concrete adapter chain. Return `Box<dyn Iterator<Item = T> + '_>` when branches produce different types or the iterator must be stored with one nameable type; it costs an allocation and dynamic calls. Use a named struct when callers need the name or more traits. To add a method to all iterators, define an extension trait with a provided method and write `impl<I: Iterator> YourExt for I {}`.

**Redoing the capstone.** A good second attempt starts from the contract alone: list the stages and their item types, put `enumerate` before anything that drops lines and `take` after everything that tests them, and predict the number of lines read before you run the tests. If you needed your first implementation to remember a stage, write down which one; that stage is what to review.

**Presenting your work.** A useful review covers three things: one prediction that was wrong and the rule that explains it; one program that the compiler rejected, with the diagnostic and your fix; and one edge case of your own, with the reason why the existing tests did not cover it.

</details>
