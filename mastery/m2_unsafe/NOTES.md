# Mastery M2 notes — Unsafe Rust by rebuilding the standard library

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md)

Your workbook. Write answers in your own words, *before* checking the hints. For this module the notes have two entries that the chapters do not have: the invariant of each type, written before the code, and what you expected Miri to say about each lab, written before you ran it. When you come back to the module for review, answer the mastery questions at the bottom again from memory, then compare with what you wrote here.

## l01 · UnsafeCell: building Cell and RefCell

**Predictions I got wrong, and why:**

- 

**The invariant, written before the code:**

> 

**Explain:** State the invariant that makes `MyCell` sound without any run-time check, and the invariant that `MyRefCell` checks at run time. Why must `set` drop the old value only after the new value is in place?

> 

**Miri lab (`miri_l01_cell`): what I expected, what Miri reported, what the SAFETY comment left out:**

- 

**An `unsafe` block whose SAFETY comment I had to rewrite, and what was missing:**

- 

**Edge case I added:**

- 

## l02 · Layout, alloc, dealloc and NonNull: building Box

**Predictions I got wrong, and why:**

- 

**The invariant, written before the code:**

> 

**Explain:** List the three facts in the invariant of `MyBox` and say which method establishes each one and which methods rely on it. How do `into_inner`, `into_raw` and `leak` avoid running `Drop for MyBox`?

> 

**Miri lab (`miri_l02_box`): what I expected, what Miri reported, what the SAFETY comment left out:**

- 

**An `unsafe` block whose SAFETY comment I had to rewrite, and what was missing:**

- 

**Edge case I added:**

- 

## l03 · A growable buffer: building Vec

**Predictions I got wrong, and why:**

- 

**The invariant, written before the code:**

> 

**Explain:** Write the invariant of your `MyVec`. For `push`, `insert`, `remove` and `truncate`, say at which moment the invariant is broken, and why no code that could panic runs during that moment.

> 

**Miri lab (`miri_l03_vec`): what I expected, what Miri reported, what the SAFETY comment left out:**

- 

**An `unsafe` block whose SAFETY comment I had to rewrite, and what was missing:**

- 

**Edge case I added:**

- 

## l04 · IntoIter and Drain: panic safety and leak safety

**Predictions I got wrong, and why:**

- 

**The invariant, written before the code:**

> 

**Explain:** For `Drain`, say what the vector looks like (a) while the drain is alive, (b) after the drain was dropped, (c) after the drain was leaked with `mem::forget`. Why is (c) safe, and what does it cost?

> 

**Miri lab (`miri_l04_drain`): what I expected, what Miri reported, what the SAFETY comment left out:**

- 

**An `unsafe` block whose SAFETY comment I had to rewrite, and what was missing:**

- 

**Edge case I added:**

- 

## l05 · Shared ownership: building Rc and Weak

**Predictions I got wrong, and why:**

- 

**The invariant, written before the code:**

> 

**Explain:** Give the invariant that links the two counts to the state of the value and of the allocation. Why does `get_mut` have to look at the weak count, and why must the counts never wrap around?

> 

**Miri lab (`miri_l05_rc`): what I expected, what Miri reported, what the SAFETY comment left out:**

- 

**An `unsafe` block whose SAFETY comment I had to rewrite, and what was missing:**

- 

**Edge case I added:**

- 

## l06 · The aliasing rules, as Miri enforces them

**Predictions I got wrong, and why:**

- 

**The invariant, written before the code:**

> 

**Explain:** In your own words: when does creating a reference, or using an old one, take away the permission of a raw pointer? Then describe one specimen that the two models judge differently, and say what a careful author does about such code.

> 

**Miri lab (`miri_l06_queue`): what I expected, what Miri reported, what the SAFETY comment left out:**

- 

**An `unsafe` block whose SAFETY comment I had to rewrite, and what was missing:**

- 

**Edge case I added:**

- 

**Task M: a specimen whose verdict surprised me, and the rule I learned:**

- 

## l07 · Send and Sync: writing the proof

**Predictions I got wrong, and why:**

- 

**The invariant, written before the code:**

> 

**Explain:** For each `unsafe impl` you wrote, give the proof in two sentences. For each bound, name a concrete `T` that would cause a data race or a wrong-thread destructor if the bound were missing.

> 

**Miri lab (`miri_l07_sync`): what I expected, what Miri reported, what the SAFETY comment left out:**

- 

**An `unsafe` block whose SAFETY comment I had to rewrite, and what was missing:**

- 

**Edge case I added:**

- 

## l08 · Capstone: a bump arena

**Predictions I got wrong, and why:**

- 

**The invariant, written before the code:**

> 

**Explain:** Give the invariant of your arena. Then explain the two design decisions in the contract: why `alloc` never runs destructors, and why `alloc_with_drop` needs `T: 'static`. Show the safe program that would read freed memory if that bound were missing.

> 

**Miri lab (`miri_l08_arena`): what I expected, what Miri reported, what the SAFETY comment left out:**

- 

**An `unsafe` block whose SAFETY comment I had to rewrite, and what was missing:**

- 

**Edge case I added:**

- 

## c01 · Code review: a teammate's ring buffer

**My review, written before running the tests.** For each `unsafe` block and `unsafe impl`: the SAFETY comment I tried to write, and where it failed:

- 

**Which problems the native tests showed, which only Miri showed, and which neither showed:**

- 

**My exploits (safe code that makes Miri report undefined behaviour), with the first line of each report:**

- 

**The invariant of `Ring`, as I wrote it for the fix:**

> 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**Say what a `&T` promises, what `UnsafeCell` changes about that promise and what it does not change, and why `Cell::get` needs `T: Copy`.**

> 

**Give the three rules of the allocator that cause most bugs, and say how a box and a vector handle zero-sized types.**

> 

**For a vector slot, say when to use `write`, `read`, an assignment, `copy` and `copy_nonoverlapping`, and what goes wrong with the wrong choice.**

> 

**Explain panic safety, leak safety, a drop guard and leak amplification, with `Drain` as the example.**

> 

**Explain the two counters of `Rc`: what each counts, when the value is dropped, when the allocation is freed, why `get_mut` looks at both, and why the counts must not wrap.**

> 

**Name the three parts of a Miri borrow report, describe the two aliasing models in two sentences each, and give the rules of thumb that both accept.**

> 

**Derive the `Send` and `Sync` bounds for a lock and for its guard, and name a type that breaks each bound if it is missing.**

> 

**For the arena, give the argument for `&self -> &mut T`, the reason `alloc` runs no destructors, and the reason `alloc_with_drop` needs `'static`.**

> 

**Say what a clean Miri run shows and what it does not show.**

> 

**Redo the capstone after a break, from an empty file and without your notes. Then change one requirement (for example: the arena must be `Send`) and say what has to change.**

> 
