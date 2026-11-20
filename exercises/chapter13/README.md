# Chapter 13 — Everyday traits

[All chapters](../../README.md) · [Previous chapter](../chapter12/README.md) · [Next chapter](../chapter14/README.md)

A handful of standard-library traits appear in almost every Rust program, often without being named: a value is cleaned up at a closing brace, a `&String` is accepted where a `&str` is expected, a map with `String` keys is searched with a string literal, and `.into()` produces the type that the context needs. This chapter makes those traits explicit. You will learn when a value is destroyed, also during a panic and for values that have no name. You will learn what `Copy`, `Clone` and `Sized` promise, how `Deref`, `AsRef` and `Borrow` lend views of data and how they differ, how to write conversions that cannot fail and conversions that can, and how `Cow` lets a function return its input without copying it. The chapter ends with a small type that needs several of these traits at once, and with the question that matters most in practice: which traits a type should *not* implement.

**Time (estimate):** about 8 to 9 hours for the whole chapter, including the reading, the review lesson, the code review and the drills. [CONCEPTS.md](CONCEPTS.md) lists an estimate for each lesson.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter13 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations or compare pointers, so a solution that copies where it could borrow fails. Then run `cargo clippy --test chapter13` and fix warnings in your code.
5. **Drill the compiler.** `cargo drill ch13` shows small programs; predict whether each compiles (and which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

After the numbered lessons, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: fix a teammate's working-but-wrong code). Run `cargo progress` to see where you are and which chapters are due for spaced review.

The table lists the lessons in the order to study them. Lesson 09 was added after the others were numbered. It continues lesson 01, so do it second. It is the longest lesson of the chapter; if it feels heavy, do it after lesson 07 instead.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Destruction and resource lifetimes](l01_drop.rs) |
| [ ] | [09 · Drop order: unwinding and temporaries](l09_drop_scopes.rs) |
| [ ] | [02 · Sized, Clone, and Copy](l02_sized_clone.rs) |
| [ ] | [03 · Deref, DerefMut, and Default](l03_deref_default.rs) |
| [ ] | [04 · AsRef and AsMut views](l04_views.rs) |
| [ ] | [05 · Borrow and borrowed lookup](l05_borrow.rs) |
| [ ] | [06 · Infallible and fallible conversions](l06_conversions.rs) |
| [ ] | [07 · ToOwned and clone-on-write](l07_cow.rs) |
| [ ] | [08 · Capstone: validated names with cheap views](l08_capstone.rs) |
| [ ] | [R01 · Review: Chapters 9, 11, and 12 from memory](r01_review.rs) |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) |
| [ ] | [Compiler drills](../../drills/ch13/) — `cargo drill ch13` |

```sh
cargo test --test chapter13 l01_
cargo test --test chapter13 -- --list
cargo test --test chapter13
cargo test --release --test chapter13
cargo clippy --test chapter13
cargo drill ch13
cargo run --example ch13_scratch
```

The starters compile; tests fail intentionally at `todo!()` and `guess()` until you fill them in. Keep the supplied tests intact. Every runnable task uses the standard library and needs no network. Each lesson header also suggests an optional lab for [the scratch example](../../examples/ch13_scratch.rs); some labs deliberately fail to compile or panic, so restore the file to compiling code afterward. Never put intentional compilation failures in the test modules.

Lesson 09 has tests that panic on purpose and catch the panic. When such a test fails for another reason, its output also shows a line with `deliberate test panic`. That line is expected; read the message after it.

Use [progressive hints](HINTS.md) only after a first attempt.

## Solutions

Model answers for every lesson, the review, the code review and the drills are in [solutions/exercises/chapter13/](../../solutions/exercises/chapter13/) and [solutions/drills/ch13/](../../solutions/drills/ch13/). Open one only after an honest attempt: when your tests pass, or when you have been stuck for a while even with both hints. Then compare the model answer with your version and write down what differs, and why.

## Topic map

"Optional lab" means a manual experiment from a lesson header. No test checks it.

| Topic | Where you practise it |
| --- | --- |
| When a destructor runs: end of scope, reverse order of declaration, moves, explicit `drop` | 01 tasks A, B; R01 task B; drill d02 |
| Drop order of struct fields; `let _`; assignment; `mem::forget` | 01 predictions |
| Cleanup that also runs during a panic: guard types, drop order while unwinding | 09 tasks A, B and predictions P1, P2 |
| Where temporaries are dropped: statements, `match`, `if let`, `while let`, last expression of a block (edition 2024) | 09 tasks C, D and predictions P3, P4; drills d11, d12, d13 |
| A destructor that panics during unwinding; edition 2021 compared with 2024 | Reading in CONCEPTS, lesson 09; optional lab 09 |
| `?Sized` parameters; thin and fat references | 02 task A and prediction P1; drill d03 |
| `Clone` as an independent copy; `Copy` as an implicit copy; `clone_from` | 02 tasks B, C and prediction P2; drill d09 |
| Why `Copy` and `Drop` exclude each other | Drill d01; optional lab 02 |
| `Rc::clone` compared with cloning the data; `derive(Clone)` on generic types | Reading in CONCEPTS, lesson 02; optional lab 02 |
| `Deref` and `DerefMut` to a slice; deref coercion; method lookup through `Deref` | 03 tasks A, B and prediction P1; drill d04 |
| `Default` written by hand; defaults of standard types; struct update syntax | 03 task C and prediction P2; R01 task C |
| `AsRef<[u8]>` and `AsMut<[u8]>` parameters; views that do not copy | 04 tasks A, B and prediction P2 |
| `AsRef<Path>` | 04 prediction P1; optional lab 04 |
| Searching a map or set with a borrowed key; `Borrow` and `BorrowMut` as bounds | 05 tasks A to C and predictions; drill d06 |
| Implementing `Borrow` for your own key type, and when that is wrong | Reading in CONCEPTS, lesson 05; optional labs 05 and 08 |
| `From` and `Into`; moving a buffer through a conversion without copying it | 06 tasks A, C; R01 task A; drill d10 |
| `TryFrom` and `TryInto` with range checks; `as` compared with a checked conversion | 06 task B and predictions; C01 (`Port`); drills d05, d08 |
| `ToOwned`; `Cow` that borrows unchanged input; `to_mut` and `into_owned` | 07 tasks A to C and predictions; C01 (`unix_newlines`); drill d07 |
| Reading through a slice parameter instead of cloning | C01 (`total_chars`) |
| Choosing the traits for one type: validation, a cheap view, allocation only when needed | 08 capstone |

Variance, `PhantomData` and the drop check, and building `Box`, `Vec` and `Rc` from raw memory are not part of this chapter. They belong to the mastery track ([`mastery/m1_types`](../../mastery/m1_types/README.md) and [`mastery/m2_unsafe`](../../mastery/m2_unsafe/README.md)).

## Mastery check

- [ ] Predict destruction order, including explicit drop, moves, and unwinding.
- [ ] Say where a temporary value is dropped in a statement, a `match`, an `if let`, and the last expression of a block.
- [ ] Explain why Copy forbids Drop and how Clone can do real work.
- [ ] Use ?Sized when an API need not require statically sized values.
- [ ] Distinguish cheap reference views, identity-preserving borrows, and owned conversions.
- [ ] Explain where Cow avoids allocation and where it must allocate.
- [ ] Redo the capstone from its contract after a few days without consulting your earlier implementation.
- [ ] Present your predictions, failed compiler experiments, and one new edge case for review.

Model answers for the first six items are at the end of [HINTS.md](HINTS.md). Read them only after you have written your own.
