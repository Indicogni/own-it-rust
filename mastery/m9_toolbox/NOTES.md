# M9 notes — The professional toolbox

Your workbook. Write answers in your own words, *before* checking the hints. When you come back to this module for review, answer the mastery questions at the bottom again from memory, then compare with what you wrote here.

## l01 · Cargo in depth: workspaces, features, profiles, build scripts

**Predictions I got wrong, and why:**

- 

**Explain:** Explain feature unification to a teammate who asks "why does my crate build alone but behave differently in the workspace build?".

> 

**Edge case I added:**

- 

**Labs (1.1, 1.2): what the tool showed me that I did not expect:**

- 

## l02 · Toolchains, the editor, and reading generated code

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what rust-analyzer needs from a project before it can show types, and why a team commits `rust-toolchain.toml`.

> 

**Edge case I added:**

- 

**Labs (2.1, 2.2, 2.3): what the tool showed me that I did not expect:**

- 

## l03 · Lints and formatting as policy

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why `#![deny(warnings)]` in source code is a trap, and where "warnings are errors" belongs instead.

> 

**Edge case I added:**

- 

## l04 · Testing at scale: snapshots, fixtures, flaky tests, coverage, mutants

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what line coverage measures and what a surviving mutant tells you that coverage cannot.

> 

**Edge case I added:**

- 

**Labs (4.1, 4.2, 4.3, 4.4): what the tool showed me that I did not expect:**

- 

## l05 · Fuzzing and sanitizers

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what "coverage-guided" means, and why a round-trip check finds bugs that "it did not crash" cannot.

> 

**Edge case I added:**

- 

**Labs (5.1, 5.2, 5.3, 5.4): what the tool showed me that I did not expect:**

- 

## l06 · Debugging: panics, backtraces, hooks, and a debugger

**Predictions I got wrong, and why:**

- 

**Explain:** Explain the difference between the place where a program fails and the place where it went wrong, using the stockroom bug.

> 

**Edge case I added:**

- 

**Labs (6.1): what the tool showed me that I did not expect:**

- 

## l07 · Documentation: rustdoc, links, examples, README, changelog

**Predictions I got wrong, and why:**

- 

**Explain:** Explain who reads each of these, and when: the README, the crate-level docs, an item's docs, an example program, the changelog.

> 

**Edge case I added:**

- 

**Labs (7.1): what the tool showed me that I did not expect:**

- 

## l08 · Supply chain and policy: lock files, advisories, licences, MSRV

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what `Cargo.lock` protects you from, what it does not protect you from, and who decides when it changes.

> 

**Edge case I added:**

- 

**Labs (8.1, 8.2, 8.3, 8.4, 8.5): what the tool showed me that I did not expect:**

- 

## l09 · CI and release: pipelines, packaging, profiles, cross-compilation

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what makes a build reproducible, and which inputs of a CI run you have pinned after this lesson.

> 

**Edge case I added:**

- 

**Labs (9.1, 9.2): what the tool showed me that I did not expect:**

- 

## l10 · Capstone: turn `capstone/` into a well-run workspace

**My plan, written before I started (the order of the work):**

- 

**Explain:** Which three of the changes you made would you do first in a real project that is in this state, and why those?

> 

**What I would do differently next time:**

- 

## Tools on my machine

Versions I installed for the labs, and anything that behaved differently from LABS.md:

| Tool | Version | Notes |
| --- | --- | --- |
|  |  |  |

## Mastery check (answer from memory)

Model answers are at the end of [HINTS.md](HINTS.md).

**1. Two members of a workspace depend on the same crate with different features. What gets built when you run `cargo build -p one-member`, and what when you run `cargo build --workspace`? Why must features be additive?**

> 

**2. Your manifest says `unsafe_code = "forbid"` and CI runs `cargo clippy -- -D warnings`. A new stable compiler adds a lint that fires in your code. What happens to users of your published crate, and what happens in your CI? What decides the answer?**

> 

**3. A test suite has 100% line coverage. Give two different reasons why a bug in the covered code can still pass, and name the tool that would find each.**

> 

**4. A fuzzer runs your parser for an hour and reports nothing. List three things that must be true before that result means anything.**

> 

**5. A program panics with "index out of bounds" at a line that looks correct. Describe your next three steps, and the mistake to avoid.**

> 

**6. What does `Cargo.lock` guarantee, and what does it not? Who should commit it, and which flag makes CI respect it?**

> 

**7. Your crate says `rust-version = "1.85"`. Name two independent ways the promise can be broken without any test failing, and the check that catches each.**

> 

**8. List everything in a CI run that can change without a commit, and how you pin each.**

>
