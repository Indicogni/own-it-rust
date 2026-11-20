# Chapter 8 notes — Modules, crates and Cargo

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · Crates, editions and profiles

**Predictions I got wrong, and why:**

- 

**Explain:** Distinguish package, crate, target, compiler version, edition, rust-version, and build profile.

> 

**Edge case I added:**

- 

## l02 · Nested modules and private implementation

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why a module boundary should expose operations that preserve invariants instead of the raw representation.

> 

**Edge case I added:**

- 

## l03 · Paths, imports and reexports

**Predictions I got wrong, and why:**

- 

**Explain:** Explain absolute paths, self/super/crate, imported aliases, prelude names, and the role of a facade reexport.

> 

**Edge case I added:**

- 

## l04 · Constants and statics

**Predictions I got wrong, and why:**

- 

**Explain:** Explain shared static storage and why a const is a value definition, without assuming identity for references to a const.

> 

**Edge case I added:**

- 

## l05 · Attributes, cfg and included files

**Predictions I got wrong, and why:**

- 

**Explain:** Explain conditional compilation versus runtime branching, environment capture at compile time, and file inclusion paths.

> 

**Edge case I added:**

- 

## l06 · Tests and executable documentation

**Predictions I got wrong, and why:**

- 

**Explain:** Explain unit tests, integration tests and doc-tests, including what only a public consumer can prove.

> 

**Edge case I added:**

- 

## l07 · Dependency requirements and lockfiles

**Predictions I got wrong, and why:**

- 

**Explain:** Explain compatible version requirements, the pre-1.0 exception, lockfile reproducibility, and why path dependencies are useful for offline experiments.

> 

**Edge case I added:**

- 

## l08 · Capstone: library facade and workspace

**Predictions I got wrong, and why:**

- 

**Explain:** Explain the public facade and the kind of test that proves its privacy. Say exactly what a package archive contains.

> 

**Edge case I added:**

- 

## r01 · Spaced retrieval: Chapters 4, 6 and 7 inside Chapter 8

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

**Explain package versus crate versus module, and name the crates that Cargo builds from one package with a library, a binary and an integration test.**

> 

**Control visibility with private modules, private fields and public reexports; say which kind of test can prove that something is hidden.**

> 

**Split modules into files without changing any path that callers use.**

> 

**Distinguish const from static, and compile-time cfg from a runtime if.**

> 

**Write unit tests, integration tests and doc-tests, and say what each kind can see.**

> 

**Explain a version requirement, the lockfile, and why a workspace has only one lockfile.**

> 

**Add a feature that switches on an optional dependency, and explain why features must only add.**

> 

**Say what a build script is, when it runs, and how it passes a value to the crate.**

> 

**Redo the final capstone after a break without looking at your implementation; invent a changed requirement and explain the tradeoff.**

>
