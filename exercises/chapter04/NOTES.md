# Chapter 4 notes — Ownership and moves

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · Ownership scopes

**Predictions I got wrong, and why:**

- 

**Explain:** Draw the owner tree for a vector of strings and explain when each part of it is destroyed.

> 

**Edge case I added:**

- 

## l02 · Moves across assignments and fields

**Predictions I got wrong, and why:**

- 

**Explain:** Explain partial moves, what happens to the remaining fields, and the difference between moving a tuple and destructuring it.

> 

**Edge case I added:**

- 

## l03 · Moves and control flow

**Predictions I got wrong, and why:**

- 

**Explain:** Explain how the rule "a variable must hold a value when it is used" applies to every path through an `if` and to every loop iteration.

> 

**Edge case I added:**

- 

## l04 · Moving from indexed collections

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why every slot of a vector must always hold a value, and compare the order and the cost of `pop`, `remove` and `swap_remove`.

> 

**Edge case I added:**

- 

## l05 · Leaving a replacement behind

**Predictions I got wrong, and why:**

- 

**Explain:** Describe the rule that `replace` and `take` keep true, and say what the caller sees in the place afterward.

> 

**Edge case I added:**

- 

## l06 · Copy and explicit cloning

**Predictions I got wrong, and why:**

- 

**Explain:** Explain which types are copied on assignment, why a type must ask for `Copy`, and why a cloned String is independent of the original.

> 

**Edge case I added:**

- 

## l07 · Rc and Arc

**Predictions I got wrong, and why:**

- 

**Explain:** Explain strong counts, what happens when the last owner is dropped, what `Arc` costs compared with `Rc`, and why two values that own each other through `Rc` are never freed.

> 

**Edge case I added:**

- 

## l08 · Capstone: owned dispatch queue

**Predictions I got wrong, and why:**

- 

**Explain:** Draw every allocation and every owner after `dispatch`. Explain why sharing a configuration that nobody changes does not mean that the job labels, which are changed, should be shared too.

> 

**Edge case I added:**

- 

## r01 · Spaced retrieval: Chapter 3 inside Chapter 4

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

**Draw the stack and the heap before and after a move, a clone, and a drop of a `String`.**

> 

**Say which operations leave the source variable without a value, and why such a variable can be given a new value.**

> 

**Take an owned element out of a vector, and an owned value out of a place behind `&mut`, without cloning and without leaving the place empty.**

> 

**Explain why `Copy` cannot be derived for a type with a `String` field, and how cloning an `Rc` differs from cloning the value inside it.**

> 

**Say when to use `Rc` and when `Arc`, why neither lets you change the shared value, and why two values that own each other through `Rc` are never freed.**

> 

**Redo the capstone after a break without looking at your first solution. Then invent a changed requirement and explain what it changes in the ownership design.**

>
