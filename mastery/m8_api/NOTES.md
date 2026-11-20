# Mastery M8 notes — Library design, semver and stewardship

Your workbook. Write answers in your own words, *before* checking the hints. When you come back to this module for review, answer the mastery questions at the bottom again from memory, then compare with what you wrote here.

## l01 · Version numbers as promises

**Predictions I got wrong, and why:**

- 

**For each change: the line of `app.rs` it meets, or why it meets none:**

- 

**Explain:** Two of the thirteen changes are in a class that does not match what happened to the program: one breaks nothing here and is still "major", one breaks the build and is still not "major". Which are they, and what does that tell you about the difference between "this change broke somebody" and "this change needs a major version"?

> 

**Edge case I added:**

- 

## l02 · Traits, bounds and auto traits

**Predictions I got wrong, and why:**

- 

**For each witness: the capability that the change takes away, in one sentence:**

- 

**Explain:** Changes T6 and T8 edit no public signature. Explain, for each, how the compiler decides that the old program is now wrong, and name one thing a maintainer can put into the library's own test suite so that such a change cannot be released by accident.

> 

**Edge case I added:**

- 

## l03 · Room to change

**Predictions I got wrong, and why:**

- 

**Explain:** Every tool in this lesson takes something away from your users today. Say, for each of the four tools, what the user loses and which later change the library gains. When is the price too high?

> 

**Edge case I added:**

- 

## l04 · An error type for a library

**Predictions I got wrong, and why:**

- 

**My design: the types I chose, one design I rejected, and why:**

- 

**Explain:** Why does this contract ask for a struct with a `kind()` method and not for a public enum with one variant per failure? Name one later change that the struct allows in a minor release and the enum does not.

> 

**Edge case I added:**

- 

## l05 · Features that only add

**Predictions I got wrong, and why:**

- 

**My design: the types I chose, one design I rejected, and why:**

- 

**Explain:** `swatch` 0.3.0 has three different feature mistakes. For each one, describe the program that it breaks, and say why the author of `swatch` could run `cargo test` every day and never see it.

> 

**Edge case I added:**

- 

## l06 · Documentation as interface

**Predictions I got wrong, and why:**

- 

**Explain:** Task D runs your examples against five wrong versions of the library. Which of your examples caught which one? What does that say about the difference between an example that shows a function and an example that specifies it?

> 

**Edge case I added:**

- 

## l07 · Deprecation, minimum Rust version, editions

**Predictions I got wrong, and why:**

- 

**Explain:** Task B is checked by a lint. Name one way in which code can pass that lint and still fail to build with Rust 1.74, and say what a project must do to be able to promise its `rust-version` honestly.

> 

**Edge case I added:**

- 

## c01 · An API review

**My review, written before running the tests (one finding per line, with the guideline, for example C-GETTER):**

- 

**The two bugs I found by reading:**

- 

**Findings of the tests that my review missed:**

- 

## l08 · Capstone: a release

**Predictions I got wrong, and why:**

- 

**My design: the types I chose, one design I rejected, and why:**

- 

**Explain:** Which of the five requests could not be fulfilled in the way the requester first imagined, because of a decision that was made in 1.0.0? For each, name the decision, and say what 1.0.0 should have done.

> 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**A manifest says `dep = "0.4.1"`. State which versions Cargo may choose, and the smallest correct next version of `dep` after (a) a bug fix, (b) a new function, (c) a removed function.**

> 

**Given a diff of a public interface, sort each change into major, minor or possibly-breaking, and for each major one write a small program that the change breaks.**

> 

**Explain how a change to a private field can break a user, and what you put into your test suite so that it cannot happen unnoticed.**

> 

**Name the four tools for future-proofing, what each one takes away from the user, and one type for which you would deliberately use none of them.**

> 

**Design an error type for a small parser so that callers can branch on the failure, a reporter can print the chain without repetition, and you can add a failure in a minor release.**

> 

**Explain feature unification with a two-program example, and say why a feature must not change the type of an existing item.**

> 

**Write documentation for a fallible function: a first sentence, `# Errors`, and an example with `?` that would fail if the documented edge case were implemented wrongly.**

> 

**Retire a badly named function over two releases, and say which of your users can still be broken by the first of them.**

> 

**Say what `rust-version` promises, two ways to check the promise, and what only one of them can see.**

> 

**Redo the capstone after a break without looking at your first solution. Then decide what a 2.0.0 of `pacer` should remove, and write its changelog.**

>
