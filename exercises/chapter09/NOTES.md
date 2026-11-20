# Chapter 9 notes — Structs and methods

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · Named structs and update syntax

**Predictions I got wrong, and why:**

- 

**Explain:** Explain construction by field name, field shorthand, update syntax moves, and partial initialization.

> 

**Edge case I added:**

- 

## l02 · Tuple structs, unit structs and layout

**Predictions I got wrong, and why:**

- 

**Explain:** Explain nominal types, zero-sized marker values, alignment and padding, and why Rust layout should not be assumed from declaration order.

> 

**Edge case I added:**

- 

## l03 · Methods and associated items

**Predictions I got wrong, and why:**

- 

**Explain:** Explain the invariant shared by the constructor and mutation method, and how an associated constant belongs to the type.

> 

**Edge case I added:**

- 

## l04 · Smart-pointer self receivers

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why a method may need ownership of a smart pointer rather than a borrow of the pointee.

> 

**Edge case I added:**

- 

## l05 · Generic structs and impl blocks

**Predictions I got wrong, and why:**

- 

**Explain:** Explain unconstrained storage and the difference between generic type parameters and method-specific bounds.

> 

**Edge case I added:**

- 

## l06 · Lifetimes and const parameters

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what the borrowed window owns and why its output reference can outlive the short borrow of the wrapper.

> 

**Edge case I added:**

- 

## l07 · Deriving common traits

**Predictions I got wrong, and why:**

- 

**Explain:** Explain field-by-field derived traits, Debug versus Display, and why derived Default is not automatically a valid domain default.

> 

**Edge case I added:**

- 

## l08 · Cell and RefCell

**Predictions I got wrong, and why:**

- 

**Explain:** Explain interior mutability, runtime borrow guards, try_borrow_mut errors, and why RefCell is not a thread synchronization primitive.

> 

**Edge case I added:**

- 

## l10 · Weak and parent links

**Predictions I got wrong, and why:**

- 

**Explain:** Explain which direction of a tree owns and which only observes, what the strong and weak counts measure, and why upgrade returns an Option.

> 

**Edge case I added:**

- 

## l09 · Capstone: validated inventory

**Predictions I got wrong, and why:**

- 

**Explain:** Explain the fields you chose, the constructor invariant, atomic failure of restock, and why Cell is appropriate for a single-threaded read statistic.

> 

**Edge case I added:**

- 

## r01 · Spaced retrieval: Chapters 3, 5, 7 and 8 inside Chapter 9

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

**Choose named, tuple, and unit structs and explain why tuple structs create distinct types.**

> 

**Keep invariants behind methods and choose owned, shared, or exclusive receivers deliberately.**

> 

**Use smart-pointer self receivers without accidentally cloning the underlying data.**

> 

**Combine type, lifetime, and const parameters and explain the role of each.**

> 

**State what derived traits do and which field traits they require.**

> 

**Use Cell and RefCell while explaining runtime borrow errors and the absence of arbitrary shared mutation.**

> 

**Link values with Rc in the owning direction and Weak in the other, and explain how a strong cycle leaks.**

> 

**Redo the final capstone after a break without looking at your implementation; invent a changed requirement and explain the tradeoff.**

>
