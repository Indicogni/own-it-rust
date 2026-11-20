# Chapter 11 — Generic code with traits

[All chapters](../../README.md) · [Previous chapter](../chapter10/README.md) · [Next chapter](../chapter12/README.md)

A trait names something a type can do, and a generic function works for every type that can do it. Together they are how Rust code is reused without giving up speed or type checking, and they stand behind most of the standard library: iterators, formatting, operators, error handling. In this chapter you implement traits for your own types and for types you did not write, choose between a generic parameter and a trait object and learn what each one costs, use associated types and constants, and return iterators and closures whose types cannot be written down. The teaching text is [CONCEPTS.md](CONCEPTS.md); read one section, then do its lesson. Type signatures and trait scaffolds are given so that every lesson compiles from the start; the optional labs ask you to rebuild the bounds yourself.

**Time (an estimate):** about 9 hours for the whole chapter: close to 7 hours for lessons 01 to 07 with their reading, and about 2 hours for the review, the code review, and the drills. The table at the top of [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter11 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations or compare pointers, so a solution that copies where it could borrow fails. Then run `cargo clippy --test chapter11` and fix warnings in your code.
5. **Drill the compiler.** `cargo drill ch11` shows small programs; predict whether each compiles (and which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

After the numbered lessons, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: fix a teammate's working-but-wrong code). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Traits, implementations, and defaults](l01_interfaces.rs) |
| [ ] | [02 · Static and dynamic dispatch](l02_dispatch.rs) |
| [ ] | [03 · Self, supertraits, and associated items](l03_self.rs) |
| [ ] | [04 · Extension traits and qualified calls](l04_extensions.rs) |
| [ ] | [05 · Associated types and generic traits](l05_relationships.rs) |
| [ ] | [06 · Opaque results, lifetimes, and precise capture](l06_bounds.rs) |
| [ ] | [07 · Capstone: a generic validation pipeline](l07_capstone.rs) |
| [ ] | [R01 · Review: Chapters 7, 9, and 10 from memory](r01_review.rs) |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) |
| [ ] | [Compiler drills](../../drills/ch11/) — `cargo drill ch11` |

```sh
cargo test --test chapter11 l01_
cargo test --test chapter11 -- --list
cargo test --test chapter11
cargo test --release --test chapter11
cargo clippy --test chapter11
cargo drill ch11
cargo run --example ch11_scratch
```

The starters compile; tests fail intentionally at `todo!()` and `guess()` until you fill them in. Keep the supplied tests intact. Every runnable task uses the standard library and needs no network. Each lesson header also describes an optional lab. Labs are not checked by tests. Do them in [the scratch example](../../examples/ch11_scratch.rs); some labs deliberately fail to compile or panic, so restore the file to compiling code afterward. Never put intentional compilation failures in the test modules.

Use [progressive hints](HINTS.md) only after a first attempt.

## Topic map

| Topic | Where you practise it |
| --- | --- |
| Implementing a trait; required and provided methods; overrides | 01 |
| Generic functions with a trait bound | 01 task B, 02 task A, 03 task C, R01 task B |
| Blanket implementations (you read and predict one; you do not write one) | 01 prediction P3 |
| Static dispatch and trait objects; fat pointers; the cost of `Box<dyn Trait>` | 02 |
| `?Sized` bounds; writing through `dyn Write`; `write_all` | 02 task C |
| Which traits can be trait objects (dyn compatibility) | drill d01; optional labs in 02 and 03; the explanation in 07 |
| `Self`, associated functions, associated constants, supertraits | 03 |
| A provided method written once for every implementor | 03 task B |
| Extension traits on types from the standard library | 04 task A, drill d06 |
| Qualified calls and the method lookup order | 04 task B and predictions |
| Trait methods need the trait in scope | drill d02 |
| Coherence and the orphan rule; the newtype repair | drills d03 and d11; optional lab in 04 |
| Associated types | 05 task A, 07, drill d07 |
| Generic trait parameters with several implementations | 05 tasks B and C |
| Bounds on an associated type | optional lab in 05 |
| Returning `impl Trait`: iterators and closures | 06 tasks A and B, C01, drill d05 |
| Precise capture with `use<..>` in edition 2024 | 06 task A, drill d09; optional lab in 06 |
| Asking for the smallest set of bounds | 06 task C, R01 task B, C01, drill d04 |
| Total order for floats when a bound asks for `Ord` | drill d08 |
| Run-time type checks with `dyn Any` and `Box<dyn Error>` | drill d10; the tests of R01 task C |
| One pipeline with a generic source and trait-object rules | 07 capstone |
| Recursive enums, generic structs, and error propagation from earlier chapters | R01 |
| Reviewing a pull request that uses traits badly | C01 |

Not in this chapter: variance, higher-ranked bounds (`for<'a>`), generic associated types, trait-object upcasting, sealed traits, and typestate. They belong to the mastery track ([`mastery/m1_types`](../../mastery/m1_types/README.md)).

## Solutions

Model answers for every lesson, the review, and the code review are in [solutions/exercises/chapter11/](../../solutions/exercises/chapter11/). The repaired drills and their expected outcomes are in [solutions/drills/ch11/](../../solutions/drills/ch11/). Open a solution only after an honest attempt at the task, and after the hints. Then compare it with your version line by line: a different answer that passes the tests may be just as good.

## Mastery check

- [ ] Explain a trait object’s data/vtable pair and a generic function’s monomorphization.
- [ ] Design a dyn-compatible interface, then explain which methods would break compatibility.
- [ ] Choose associated types versus a generic trait parameter and justify each bound.
- [ ] Explain Self, supertraits, associated constants, extension traits, and coherence.
- [ ] Demonstrate how return-position impl Trait captures lifetimes in edition 2024.
- [ ] Redo the capstone from its contract after a few days without consulting your earlier implementation.
- [ ] Present your predictions, failed compiler experiments, and one new edge case for review.

Model answers for the first five items are at the end of [HINTS.md](HINTS.md). Write your own first.

## Next

Take checkpoint 2 (`checkpoints/cp2_after_ch11`) at least two days after you finish this chapter. It tests chapters 1 to 11 without hints. Read the [checkpoint guide](../../checkpoints/README.md#when-to-take-one) first, and open the checkpoint's `SPEC.md` only when you start the timer. Until then, continue with [chapter 12](../chapter12/README.md).
