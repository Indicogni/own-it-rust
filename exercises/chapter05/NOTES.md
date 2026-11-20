# Chapter 5 notes — Borrowing with references

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · Borrowing and dereferencing

**Predictions I got wrong, and why:**

- 

**Explain:** Explain who owns storage and why a method receiver can be dereferenced automatically while assignment through &mut needs *.

> 

**Edge case I added:**

- 

## l02 · Reference bindings and identity

**Predictions I got wrong, and why:**

- 

**Explain:** Explain reference binding mutability, referent mutability, nested references, and std::ptr::eq.

> 

**Edge case I added:**

- 

## l03 · Optional references and fat pointers

**Predictions I got wrong, and why:**

- 

**Explain:** Explain borrowed temporaries, Option<&T>, slice length metadata, and why a borrow does not own the referent.

> 

**Edge case I added:**

- 

## l04 · Lifetimes through functions

**Predictions I got wrong, and why:**

- 

**Explain:** Describe the input-to-output lifetime relationship and why it does not allocate or extend any lifetime.

> 

**Edge case I added:**

- 

## l05 · Structs that borrow

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why the struct has a lifetime parameter and how fields remain tied to one owner.

> 

**Edge case I added:**

- 

## l06 · Independent lifetimes and elision

**Predictions I got wrong, and why:**

- 

**Explain:** State the function lifetime elision rules and explain when independent lifetimes let one result outlive another input.

> 

**Edge case I added:**

- 

## l07 · Sharing versus mutation

**Predictions I got wrong, and why:**

- 

**Explain:** Explain last use, reallocation invalidation, exclusive access, and why runtime capacity is not enough to permit an overlapping borrow.

> 

**Edge case I added:**

- 

## l08 · Disjoint mutable borrows

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what split_at_mut guarantees, where that guarantee comes from, and why the caller needs no unsafe code.

> 

**Edge case I added:**

- 

## l09 · Capstone: borrowed roster queries

**Predictions I got wrong, and why:**

- 

**Explain:** Compare returning a borrow, returning an index, and cloning an owner. Explain invalidation of indices on removal and the caller obligations of each API.

> 

**Edge case I added:**

- 

## r01 · Spaced retrieval: Chapters 3 and 4 inside Chapter 5

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

**Explain the difference between pointing a reference at another value and changing the value it points at. Show both in code.**

> 

**Explain where Rust dereferences for you and where you must write `*`, what `==` and `std::ptr::eq` compare, and what a `&[T]` or `&str` holds besides an address.**

> 

**Explain why a function cannot return a reference to its own local variable, and what a lifetime annotation does and does not do.**

> 

**State the three lifetime elision rules, and give a case where a struct needs two lifetime parameters instead of one.**

> 

**State the rule about shared and exclusive references, and give three ways to fix a borrow conflict without cloning the whole collection.**

> 

**Explain why `&mut v[0]` and `&mut v[1]` cannot be in use together, and name two library methods that solve the problem.**

> 

**Redo the capstone after a break without looking at your implementation. Then invent a changed requirement and explain the trade-off.**

>
