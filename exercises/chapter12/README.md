# Chapter 12 — Operators for your own types

[All chapters](../../README.md) · [Previous chapter](../chapter11/README.md) · [Next chapter](../chapter13/README.md)

In Rust, `a + b`, `a == b`, `a < b` and `a[i]` are not special syntax for built-in types. Each one is a call to a trait method, and any type can implement the trait. In this chapter you give your own types arithmetic, bit operators, equality, ordering and indexing. The harder part is not the syntax but the promises behind it: equal values must hash equally, `<` must agree with `sort`, and floating-point numbers break rules that integers keep. You will also learn where operators stop, and a named method is the better design. Type signatures and trait scaffolds are provided so that each lesson compiles; you write the bodies.

**Time (estimate):** about 5 to 6 hours for the whole chapter, including the reading, the review lesson, the code review and the drills.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter12 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations or compare pointers, so a solution that copies where it could borrow fails. Then run `cargo clippy --test chapter12` and fix warnings in your code.
5. **Drill the compiler.** `cargo drill ch12` shows small programs; predict whether each compiles (and which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

After the numbered lessons, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: fix a teammate's working-but-wrong code). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Unary, binary, and assignment operators](l01_arithmetic.rs) |
| [ ] | [02 · Equivalence and NaN](l02_equality.rs) |
| [ ] | [03 · Partial and total comparisons](l03_ordering.rs) |
| [ ] | [04 · Index and IndexMut](l04_indexing.rs) |
| [ ] | [05 · Capstone: a small bit-set API](l05_capstone.rs) |
| [ ] | [R01 · Review: Chapters 5, 10, and 11 from memory](r01_review.rs) |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) |
| [ ] | [Compiler drills](../../drills/ch12/) — `cargo drill ch12` |

```sh
cargo test --test chapter12 l01_
cargo test --test chapter12 -- --list
cargo test --test chapter12
cargo test --release --test chapter12
cargo clippy --test chapter12
cargo drill ch12
cargo run --example ch12_scratch
```

The starters compile; tests fail intentionally at `todo!()` and `guess()` until you fill them in. Keep the supplied tests intact, with one exception: when you have answered every prediction in a file, `guess` is an unused import, and you delete it from the `use` line (the chapter allows the unused import, so the compiler does not warn). Every runnable task uses the standard library and needs no network. Each lesson header also suggests an optional lab for [the scratch example](../../examples/ch12_scratch.rs); some labs deliberately fail to compile or panic, so restore the file to compiling code afterward. Never put intentional compilation failures in the test modules.

If a test run stops with `has overflowed its stack`, or a test reports that an operator `was still running after 10 s`, one of your methods calls itself without end. The first entry in [HINTS.md](HINTS.md) explains the usual cause.

Use [progressive hints](HINTS.md) only after a first attempt.

## Solutions

Model answers for every lesson, the review, the code review and the drills are in [solutions/exercises/chapter12/](../../solutions/exercises/chapter12/) and [solutions/drills/ch12/](../../solutions/drills/ch12/). Open one only after an honest attempt: when your tests pass, or when you have been stuck for a while even with both hints. Then compare the model answer with your version and write down what differs, and why.

## Topic map

| Topic | Where you practise it |
| --- | --- |
| Arithmetic operators for your own type: `+`, unary `-`, `*` with a different right-hand type | 01 tasks A, B, D; drills d02, d04, d05 |
| Compound assignment as a separate trait: `+=`, `\|=` | 01 task C; 05 task D; C01 (`+=` in place); drill d10 |
| Which operands an operator moves; reusing an operand's buffer | 01 predictions; C01 (`Poly`); drills d01, d06, d07 |
| Bit operators for a newtype: `\|`, `&`, `!` | 05 tasks A to C; optional labs 01 and 05 (`^`, `&=`) |
| Equality with a chosen identity; when a type may claim `Eq` | 02 task A; R01 task B; C01 (`Tag`) |
| NaN, the two zeros, and float equality | 02 task B and predictions; drill d03 |
| `Hash` that agrees with a hand-written `==` | C01 (`Tag`); optional lab 02 |
| A total order: `Ord`, and `PartialOrd` that delegates to it | 03 tasks A, B; drill d08 |
| The partial order of floats; `total_cmp` | 03 task C and predictions; optional lab 03 |
| Derived order of enums, order of strings, ties in `max_by_key` and `min_by_key` | 03 predictions |
| `Index` and `IndexMut` with a checked index that panics | 04 tasks A, B; R01 task C; drill d09 |
| Indexing with ranges; an `Output` without a fixed size | 04 prediction P1; optional lab 04 |
| Shift width in debug and release builds | 05 task E |
| Operators that cannot be overloaded | Reading in CONCEPTS, Lesson 01; optional labs 01 and 05 |
| Generic bounds on operator traits (`T: Add<Output = T>`) | R01 task A |

## Mastery check

- [ ] Explain operand ownership and why Add<Rhs> has an associated Output.
- [ ] Keep PartialEq, Eq, PartialOrd, and Ord consistent, including hashing implications.
- [ ] Explain why floats cannot promise Eq or a natural total order.
- [ ] Implement indexing that returns a reference and assignment that mutates in place.
- [ ] Name operators that Rust does not allow user code to overload.
- [ ] Redo the capstone from its contract after a few days without consulting your earlier implementation.
- [ ] Present your predictions, failed compiler experiments, and one new edge case for review.

Model answers for the first five items are at the end of [HINTS.md](HINTS.md). Read them only after you have written your own.
