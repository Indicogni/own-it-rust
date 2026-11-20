# Chapter 23 notes — Unsafe Rust

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · A safe invariant boundary

**Predictions I got wrong, and why:**

- 

**Explain:** State the private invariant, identify each unsafe operation, and explain why all public operations preserve the invariant.

> 

**Edge case I added:**

- 

## l02 · Raw pointer reads with contracts

**Predictions I got wrong, and why:**

- 

**Explain:** List every caller obligation of `read_u32` and show how the safe wrapper `at` discharges each one.

> 

**Edge case I added:**

- 

## l03 · Raw slice reconstruction

**Predictions I got wrong, and why:**

- 

**Explain:** Prove bounds, provenance, alignment, initialization, lifetime, and non-overlap for `split_checked`, one at a time.

> 

**Edge case I added:**

- 

## l04 · A flagged borrow

**Predictions I got wrong, and why:**

- 

**Explain:** Explain how PhantomData lets the borrow checker track the lifetime, and how `map_addr` keeps the pointer's provenance.

> 

**Edge case I added:**

- 

## l05 · Layout, unsafe traits, unions

**Predictions I got wrong, and why:**

- 

**Explain:** State the contract of the unsafe trait `Plain`, and explain why `bool` would break its promise that every bit pattern is valid.

> 

**Edge case I added:**

- 

## l06 · Moving values and initialization

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why no code that can panic may run between a raw move and the update of the length that records which slots are initialized.

> 

**Edge case I added:**

- 

## l07 · A fixed-capacity byte gap buffer

**Predictions I got wrong, and why:**

- 

**Explain:** State the logical length, the cursor position, and the gap invariant. Explain why every byte of the storage stays initialized in this byte-only version.

> 

**Edge case I added:**

- 

## l09 · Interior mutability and layout attributes

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why `MyCell<T>` may change its contents through `&self`, why it must not be `Sync`, and why it never gives out a reference to its contents. Then say what `repr(transparent)` guarantees for the cast in task B, and what `repr(packed)` takes away in task C.

> 

**Edge case I added:**

- 

## l08 · Capstone: initialized-prefix storage

**Predictions I got wrong, and why:**

- 

**Explain:** Write the invariant (`len <= N`, and exactly `data[..len]` is initialized). Then audit `push`, `pop`, and the supplied `Drop` for a normal return and for a panic.

> 

**Edge case I added:**

- 

## r01 · Spaced retrieval: Chapters 17, 21, and 22 inside Chapter 23

**Predictions I got wrong, and why:**

- 

**Edge case I added:**

- 

## c01 · A teammate's unsafe pull request

**My review, written before running the tests:**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**Which guarantees remain your responsibility inside an `unsafe` block, and which checks does the compiler still perform there?**

> 

**How does the contract of an unsafe function differ from the invariant of a safe abstraction?**

> 

**What makes a read through a raw pointer valid? Name provenance, bounds, alignment, initialization, lifetime and aliasing.**

> 

**When must a raw move change the record of which slots are initialized, so that no value is dropped twice?**

> 

**How do size, alignment, padding and valid bit patterns limit the reinterpretation of memory?**

> 

**How does a gap buffer keep its two initialized regions disjoint?**

> 

**What must stay true if an operation supplied by the user panics?**

> 

**What do tests and Miri show, and which general safety claims still need a written proof?**

> 

**Why does interior mutability need `UnsafeCell`, why is a cell without a lock not `Sync`, and what do `repr(transparent)` and `repr(packed)` change?**

>
