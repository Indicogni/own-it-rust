# Chapter 5 — Borrowing with references

[All chapters](../../README.md) · [Previous chapter](../chapter04/README.md) · [Next chapter](../chapter06/README.md)

Chapter 4 showed that every value has one owner, and that passing a value moves it. That alone would make programs clumsy: a function could not even measure a string without taking it away from its caller. This chapter adds the missing tool. A *reference* lets code use a value without owning it, for reading (`&T`) or for changing it in place (`&mut T`). You will learn what a reference is in memory, how the compiler proves that no reference outlives the value it points at, how to read lifetime annotations in signatures and structs, and how to work with the rule that a value is either shared or exclusively borrowed, never both. The exercises are small text and list utilities; the tests check not only the results but also that you borrowed instead of copying.

**Before you start:** chapters 3 and 4. **Time:** about 9 to 11 hours for a careful first pass (an estimate; [CONCEPTS.md](CONCEPTS.md) has a per-lesson breakdown).

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter05 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations or compare addresses, so a solution that clones everything fails. Then run `cargo clippy --test chapter05` and fix the warnings in the lesson you just finished. Warnings in `c01_code_review.rs` are intended; they belong to the code review lesson.
5. **Drill the compiler.** `cargo drill ch05` shows small programs; predict whether each compiles (and which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

After the numbered lessons, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: fix a teammate's working-but-wrong code). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Borrowing and dereferencing](l01_basics.rs) |
| [ ] | [02 · Reference bindings and identity](l02_identity.rs) |
| [ ] | [03 · Optional references and fat pointers](l03_optional_fat.rs) |
| [ ] | [04 · Lifetimes through functions](l04_lifetime_flow.rs) |
| [ ] | [05 · Structs that borrow](l05_borrowed_structs.rs) |
| [ ] | [06 · Independent lifetimes and elision](l06_distinct_lifetimes.rs) |
| [ ] | [07 · Sharing versus mutation](l07_exclusive.rs) |
| [ ] | [08 · Disjoint mutable borrows](l08_disjoint.rs) |
| [ ] | [09 · Capstone: borrowed roster queries](l09_capstone.rs) |
| [ ] | [R01 · Review: Chapters 3 and 4 from memory](r01_review.rs) |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) |
| [ ] | [Compiler drills](../../drills/ch05/) — `cargo drill ch05` |

```sh
cargo test --test chapter05 l01_
cargo test --test chapter05 -- --list
cargo test --test chapter05
cargo test --release --test chapter05
cargo clippy --test chapter05
cargo drill ch05
cargo run --example ch05_scratch
```

The starters compile. Tests fail on purpose at `todo!()` and `guess()` until you fill them in. Work in lesson order and keep the supplied tests intact. Everything uses only the standard library and needs no network.

Use the [hints](HINTS.md) only after a first attempt: two hints per lesson, then a model explanation.

## Topic map

| Topic | Where you practise it |
| --- | --- |
| Lending a value for reading or writing; where you must write `*` | Lesson 01; drill d01 |
| Pointing a reference at another value; references to references; equal values versus the same object | Lesson 02 |
| `Option<&T>` instead of null; slices as an address plus a length; borrowing temporaries | Lesson 03 |
| Lifetimes in function signatures; why a reference to a local cannot be returned | Lesson 04; drills d02 and d06 |
| Structs that hold references | Lesson 05; drill d08 |
| Separate lifetime parameters; the elision rules | Lesson 06; drill d07 |
| Shared or exclusive, never both; a borrow ends at its last use; reborrowing | Lesson 07; drills d03, d05 and d09 |
| Several exclusive references into one slice; fields versus elements; `split_at_mut` and `get_disjoint_mut` | Lesson 08; drills d04 and d10 |
| Reference, index or clone: how to refer to an element of a collection that changes | Lesson 09 (capstone) |
| Two-phase borrows; reborrow versus move of `&mut`; a borrow returned on one path only | Drills d11, d12 and d13 |
| Reviewing code: needless clones, `&String` and `&Vec` parameters, a hidden panic | C01 |
| Recall of chapters 3 and 4: moving out of a vector, overflow, UTF-8 boundaries | R01 |
| Provoking and reading compiler errors yourself | Optional labs: the `COMPILER / TRANSFER LAB` line in each lesson header, done in [the scratch example](../../examples/ch05_scratch.rs). No test checks them. |

Not in this chapter: bounds between lifetimes (`'a: 'b`, `T: 'a`), variance, and `for<'a>` bounds. They are part of the mastery track ([`mastery/m1_types`](../../mastery/m1_types/README.md)). Mutation through a shared reference (`Cell`, `RefCell`) comes in chapter 9.

## Mastery check

Answer from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Explain the difference between pointing a reference at another value and changing the value it points at. Show both in code.
- [ ] Explain where Rust dereferences for you and where you must write `*`, what `==` and `std::ptr::eq` compare, and what a `&[T]` or `&str` holds besides an address.
- [ ] Explain why a function cannot return a reference to its own local variable, and what a lifetime annotation does and does not do.
- [ ] State the three lifetime elision rules, and give a case where a struct needs two lifetime parameters instead of one.
- [ ] State the rule about shared and exclusive references, and give three ways to fix a borrow conflict without cloning the whole collection.
- [ ] Explain why `&mut v[0]` and `&mut v[1]` cannot be in use together, and name two library methods that solve the problem.
- [ ] Redo the capstone after a break without looking at your implementation. Then invent a changed requirement and explain the trade-off.

## Solutions

Model solutions for every lesson, the review, the code review and all drills are in [`solutions/exercises/chapter05/`](../../solutions/exercises/chapter05/) and [`solutions/drills/ch05/`](../../solutions/drills/ch05/). Open them only after an honest attempt and after the hints: comparing your working answer with a model answer teaches more than reading the model first.
