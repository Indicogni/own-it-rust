# Chapter 22 notes — Writing macros

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · Expansion and evaluation once

**Predictions I got wrong, and why:**

- 

**Explain:** Trace expansion and evaluation separately. Explain why syntax substitution can change control flow.

> 

**Edge case I added:**

- 

## l02 · Repetition and separators

**Predictions I got wrong, and why:**

- 

**Explain:** Explain nesting/separator syntax and the type-inference issue for an empty map.

> 

**Edge case I added:**

- 

## l03 · Type and identifier fragments

**Predictions I got wrong, and why:**

- 

**Explain:** Explain which tokens the macro parses and which rules the Rust compiler still checks after expansion.

> 

**Edge case I added:**

- 

## l04 · Recursive structured literals

**Predictions I got wrong, and why:**

- 

**Explain:** Explain the recursive grammar and its deliberate limitations (negative/compound expressions must be grouped).

> 

**Edge case I added:**

- 

## l05 · Traits handle macro leaf types

**Predictions I got wrong, and why:**

- 

**Explain:** Explain syntax/type separation and the trait bound error for an unsupported leaf.

> 

**Edge case I added:**

- 

## l06 · Exports and hygiene

**Predictions I got wrong, and why:**

- 

**Explain:** Explain $crate resolution and the different roles of hygienic locals and referenced item paths.

> 

**Edge case I added:**

- 

## l08 · Patterns, items, and token munchers

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what your muncher carries from one step to the next, why internal rules stand before the entry rule, and when you capture an item whole instead of matching its parts.

> 

**Edge case I added:**

- 

## l07 · Capstone: structured events

**Predictions I got wrong, and why:**

- 

**Explain:** Specify the event grammar, evaluation order, exported-helper paths, and diagnostics for unsupported syntax.

> 

**Edge case I added:**

- 

## r01 · Spaced retrieval: Chapters 15, 19, and 21 inside Chapter 22

**Predictions I got wrong, and why:**

- 

**Edge case I added:**

- 

## c01 · A teammate's macro pull request

**My review, written before running the tests:**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**How does a macro call differ from a function call, in particular for how often an argument is evaluated and for `return` inside an argument?**

> 

**Which fragment fits an expression, a type, a name, a token tree, a pattern, a whole item, and a visibility? Which of them stay open when they are passed to another macro?**

> 

**Where can a repetition be empty, and how does a macro accept a trailing comma?**

> 

**How do recursive rules and traits divide the work between syntax and types?**

> 

**Why does an exported macro name its helpers with `$crate`, and why can a test in the same crate not show the difference from `crate`?**

> 

**What does hygiene cover, and what does it not cover?**

> 

**When does a macro need a token muncher or an internal rule, and where must internal rules stand?**

> 

**When is a procedural macro worth a crate of its own?**

>
