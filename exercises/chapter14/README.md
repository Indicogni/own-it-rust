# Chapter 14 — Closures and captures

[All chapters](../../README.md) · [Previous chapter](../chapter13/README.md) · [Next chapter](../chapter15/README.md)

A closure is a function value that can use the variables around the place where it is written. Rust code is full of them: sorting rules, iterator steps, thread bodies, callbacks, lazy defaults. In this chapter you learn what a closure really is (a small struct that the compiler builds from the variables it captures), how each variable gets into that struct (by shared borrow, by mutable borrow, or by value), and how this decides the ways a closure can be called, copied, returned and stored. At the end you can read a signature such as `F: FnMut(&str) -> bool` or `Box<dyn Fn(i32) -> i32 + '_>` and say exactly which closures it accepts and why.

The teaching text is in [CONCEPTS.md](CONCEPTS.md), one section per lesson. This chapter has **7 lessons with 18 implementation tasks**, a review lesson, a code-review lesson, and 11 compiler drills: **42 automated checks** in total, plus predictions, explanations, and optional labs.

**Time (estimate):** about 7 to 9 hours for the whole chapter, in sessions of 45 to 60 minutes. [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter14 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations or calls, compare buffer addresses, or measure the size of a closure, so a solution that clones a capture or avoids the closure fails. Then run `cargo clippy --test chapter14` and fix warnings in your code.
5. **Drill the compiler.** `cargo drill ch14` shows small programs; predict whether each compiles (and which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md). If you have time, do the lesson's optional lab in [the scratch example](../../examples/ch14_scratch.rs).

After the numbered lessons, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: fix a teammate's working-but-wrong code). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson | Check |
| --- | --- | --- |
| [ ] | [01 · Borrowed captures and sorting](l01_borrowed.rs) | `cargo test --test chapter14 l01_` |
| [ ] | [02 · Mutation, move, and consuming captures](l02_capture_modes.rs) | `cargo test --test chapter14 l02_` |
| [ ] | [03 · Choosing callable bounds](l03_call_traits.rs) | `cargo test --test chapter14 l03_` |
| [ ] | [04 · Returning borrowed and erased closures](l04_returning.rs) | `cargo test --test chapter14 l04_` |
| [ ] | [05 · Copy and Clone for captures](l05_cloning.rs) | `cargo test --test chapter14 l05_` |
| [ ] | [06 · Callback registration and invocation](l06_callbacks.rs) | `cargo test --test chapter14 l06_` |
| [ ] | [07 · Capstone: configurable text processing](l07_capstone.rs) | `cargo test --test chapter14 l07_` |
| [ ] | [R01 · Review: Chapters 10–13 from memory](r01_review.rs) | `cargo test --test chapter14 r01_` |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) | `cargo test --test chapter14 c01_` |
| [ ] | [Compiler drills](../../drills/ch14/) | `cargo drill ch14` |

Run commands from the repository root:

```sh
cargo test --test chapter14 l01_
cargo test --test chapter14 -- --list
cargo test --test chapter14
cargo test --release --test chapter14
cargo clippy --test chapter14
cargo drill ch14
cargo run --example ch14_scratch
```

The starters compile; tests fail intentionally at `todo!()` and `guess()` until you fill them in. Keep the supplied tests intact, and do not change the code in a `mod supplied` block: lessons 01 and 02 use it to give you a type that can be reached only through a closure. Use [progressive hints](HINTS.md) only after a first attempt. Passing tests alone do not show mastery; the predictions and your written explanations matter as much.

Some optional labs fail to compile or panic on purpose. Keep them in the scratch example and restore a compiling example afterward. Do not put code that must not compile into the test modules. All tests are deterministic and need only the standard library.

## Solutions

Model answers for every lesson, the review and the code review are in [solutions/exercises/chapter14/](../../solutions/exercises/chapter14/), and fixed drills with their expected outcomes are in [solutions/drills/ch14/](../../solutions/drills/ch14/). Open a solution only after an honest attempt, when your own version passes or when you have been stuck for more than about twenty minutes after reading both hints. Then compare: a different answer that passes the tests and clippy is fine. Read the comments in the solution, because they explain choices that the tests cannot see.

## Topic map

Each row names a topic and the place where you really practise it. "P1" means the first prediction of that lesson. The labs are optional and are not checked by tests.

| Topic | Where you practise it |
| --- | --- |
| Capturing by shared borrow; captures of single struct or tuple fields | Lesson 01 (task A, P1–P2) |
| Closures as sorting rules: `sort_by`, `sort_by_key`, stable order | Lesson 01 (task B, P3); C01 |
| Capturing by mutable borrow, and when that borrow ends | Lesson 02 (task A, P1); drills d02–d03 |
| `move`: owning captures, and what it does to `Copy` values | Lesson 02 (task B, P2–P3); drills d05, d08 |
| Closures that use up a capture and can run only once | Lesson 02 (task C, P4); drills d01, d07 |
| Choosing the weakest of `Fn`, `FnMut`, `FnOnce` for a parameter | Lesson 03 (tasks A–C); lesson 07; drill d04 |
| Function items, function pointers, and the size of callable values | Lesson 03 (P1, P3); drill d10 |
| Lending a closure with `&mut F` | Lesson 03 (P2) |
| Returning closures: `impl Fn` for one type, `Box<dyn Fn>` for several | Lesson 04 (tasks A–B, P2–P3); drill d06; C01 |
| Lifetimes of returned closures in edition 2024; `+ '_` on boxed closures | Lesson 04 (task A, P1); drill d11 |
| `Copy` and `Clone` closures; copying a closure copies its state | Lesson 05 (tasks A1–B2, P1–P3) |
| Storing callbacks as trait objects; `'static` captures; when captures are dropped | Lesson 06 (tasks A–B, P1–P3) |
| Shared state between callbacks with `Rc<RefCell<..>>`, and its run-time borrow check | Lesson 06 (P2); drill d09 |
| A function that fixes the order of calls while the caller supplies the policy | Lesson 07 (capstone) |
| Reviewing closure code: clones in closures, a wrong predicate, a needless `Box` | C01 |
| Earlier chapters from memory: `Cow`, blanket implementations, enum patterns with guards | R01 |
| Optional labs: borrow conflicts with closures, `+ use<>`, a registry with a lifetime parameter, generic or boxed callbacks | lab notes in each lesson header, done in [the scratch example](../../examples/ch14_scratch.rs) |

Not practised here: higher-ranked bounds (`for<'a> Fn(&'a T) -> &'a U`) are explained briefly in CONCEPTS.md, lesson 03, and practised in the mastery track ([`mastery/m1_types`](../../mastery/m1_types/README.md)). Closures on other threads (`Send`, scoped threads) belong to chapter 19, and `async` closures to chapter 20.

## Mastery check

Answer from memory in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Determine whether a closure captures each variable by shared borrow, mutable borrow, or move.
- [ ] Choose the weakest useful Fn/FnMut/FnOnce bound.
- [ ] Explain why move does not imply FnOnce-only.
- [ ] Return closures with clear lifetimes and distinguish generic from boxed dispatch.
- [ ] Register and invoke callbacks without invalidating captured references.
- [ ] Redo the capstone from its contract after a few days without consulting your earlier implementation.
- [ ] Present your predictions, failed compiler experiments, and one new edge case for review.
