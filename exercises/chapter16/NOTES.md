# Chapter 16 notes — Standard collections

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · Vector access and structural editing

**Predictions I got wrong, and why:**

- 

**Explain:** Explain length versus capacity and how each selected operation changes both ownership and ordering.

> 

**Edge case I added:**

- 

## l02 · Growth, removal, and borrow validity

**Predictions I got wrong, and why:**

- 

**Explain:** Why does Rust reject potential invalidation without proving whether this particular push reallocates? How does splitting prove disjoint mutable access?

> 

**Edge case I added:**

- 

## l03 · Sorting, binary search, and slices

**Predictions I got wrong, and why:**

- 

**Explain:** Why does stable sort preserve equal-key ordering? When can an unstable sort be preferable?

> 

**Edge case I added:**

- 

## l04 · Queues and ring buffers

**Predictions I got wrong, and why:**

- 

**Explain:** Why can VecDeque support efficient front removal where Vec shifts elements? When is make_contiguous useful?

> 

**Edge case I added:**

- 

## l05 · Priority queues and heap ordering

**Predictions I got wrong, and why:**

- 

**Explain:** Why should you pop to obtain priority order? How does Reverse adapt ordering without writing a second collection?

> 

**Edge case I added:**

- 

## l06 · Hash maps, tree maps, and entries

**Predictions I got wrong, and why:**

- 

**Explain:** When does a BTreeMap range query justify ordered keys? How can borrowed lookup avoid temporary String allocation?

> 

**Edge case I added:**

- 

## l07 · Set algebra and equal-but-distinct values

**Predictions I got wrong, and why:**

- 

**Explain:** Why must equality and hashing use the same identity fields? Why is mutating an element’s identity while stored a logic error?

> 

**Edge case I added:**

- 

## l08 · Hashing and custom builders

**Predictions I got wrong, and why:**

- 

**Explain:** Why does a collision not change map correctness? Why would this deliberately weak teaching hasher be inappropriate for untrusted keys?

> 

**Edge case I added:**

- 

## l09 · Capstone: a task scheduler and audit

**Predictions I got wrong, and why:**

- 

**Explain:** What complexity do heap insertion/pop and hash lookup suggest? Which output order must be made explicit? What does a cancelled task cost in your design?

> 

**Edge case I added:**

- 

## r01 · Spaced retrieval: earlier chapters inside Chapter 16

**Predictions I got wrong, and why:**

- 

**Edge case I added:**

- 

## c01 · A teammate's pull request

**My review, written before running the tests:**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**Choose between `Vec`, `VecDeque`, `BinaryHeap`, `HashMap`, `BTreeMap` and the two set types for a task, from the operations that the task needs and what they cost.**

> 

**Say which operations keep the order of elements, which may move the elements to a new buffer, and why the compiler rejects a change to a collection while a reference into it is alive.**

> 

**Get two mutable references into one slice without `unsafe`, in two different ways.**

> 

**Sort by several fields, explain stable versus unstable sorting, and find the exact range of equal elements in a sorted slice.**

> 

**Update a map with one lookup, and produce the same output from a `HashMap` on every run.**

> 

**State the rule that connects `Eq` and `Hash`, and say what `insert` and `replace` do with a value that equals a stored one.**

> 

**Explain what `Hash`, `Hasher` and `BuildHasher` each do, why a collision cannot make a map wrong, and when a weak hash function is dangerous.**

> 

**Redo the capstone after a break without looking at your first solution. Then explain what changes in your design if a second `submit` may also lower a task's priority.**

>
