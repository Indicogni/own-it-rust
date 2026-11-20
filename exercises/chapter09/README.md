# Chapter 9 — Structs and methods

[All chapters](../../README.md) · [Previous chapter](../chapter08/README.md) · [Next chapter](../chapter10/README.md)

A struct is how you give a group of values one name, one owner and one set of rules. This chapter takes you from declaring a struct to designing one: you build values with named fields, wrap single values in distinct types, attach methods and choose their receivers, make structs generic over types, lifetimes and constants, let the compiler write common traits for you, and change data behind a shared reference with `Cell` and `RefCell`. The last lesson before the capstone links structs into a tree with `Rc` and `Weak`. In two lessons the fields are yours to choose, because deciding what a type stores is the real skill. Everything you need is taught in this directory; nothing depends on outside reading.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read the lesson's section in [CONCEPTS.md](CONCEPTS.md).** It explains the idea, works through an example that is different from the exercise, and shows the compiler errors people usually meet.
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter09 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations, compare pointers or measure sizes, so a clone-everything solution fails. Then run `cargo clippy --test chapter09` and fix warnings in your code.
5. **Drill the compiler.** `cargo drill ch09` shows small programs; predict whether each compiles (and which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

Do the lessons in the order of the table: lesson 10 comes before the capstone (the capstone keeps its file name, `l09_capstone.rs`). After the capstone, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: fix a teammate's working-but-wrong code). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson |
| --- | --- |
| [ ] | [01 · Named structs and update syntax](l01_named.rs) |
| [ ] | [02 · Tuple structs, unit structs and layout](l02_kinds_layout.rs) |
| [ ] | [03 · Methods and associated items](l03_methods.rs) |
| [ ] | [04 · Smart-pointer self receivers](l04_smart_self.rs) |
| [ ] | [05 · Generic structs and impl blocks](l05_generics.rs) |
| [ ] | [06 · Lifetimes and const parameters](l06_lifetime_const.rs) |
| [ ] | [07 · Deriving common traits](l07_derive.rs) |
| [ ] | [08 · Cell and RefCell](l08_interior.rs) |
| [ ] | [10 · Weak and parent links](l10_weak_parents.rs) |
| [ ] | [09 · Capstone: validated inventory](l09_capstone.rs) |
| [ ] | [R01 · Review: Chapters 3, 5, 7 and 8 from memory](r01_review.rs) |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) |
| [ ] | [Compiler drills](../../drills/ch09/) — `cargo drill ch09` |

```sh
cargo test --test chapter09 l01_
cargo test --test chapter09 -- --list
cargo test --test chapter09
cargo test --release --test chapter09
cargo clippy --test chapter09
cargo drill ch09
cargo run --example ch09_scratch
```

The starters compile; tests fail intentionally at `todo!()` and `guess()` until you fill them in. Keep the supplied tests intact. Every runnable task uses the standard library and needs no network. Each lesson header also describes an optional lab: free-form experiments, some of which must not compile. Do those in [the scratch example](../../examples/ch09_scratch.rs), not in the lesson files, and restore it to compiling code afterward.

Use [progressive hints](HINTS.md) only after a first attempt.

**Time.** Plan for about 12 to 13 hours in total: roughly 45 to 90 minutes for each of the nine lessons including its reading, 90 minutes for the capstone, and 2 to 3 hours for the review lesson, the code review and the drills. These are estimates; [CONCEPTS.md](CONCEPTS.md) lists one per lesson.

## Topic map

| Topic | Where you practise it |
| --- | --- |
| Struct expressions, field shorthand, update syntax (`..old`) and what it moves | 01 tasks A and B; drills d01, d08 |
| Partial moves and the order in which fields are dropped | 01 predictions |
| Tuple structs as distinct types (newtypes) | 02 task A; drill d03 |
| Unit structs and zero-sized values | 02 task B and predictions |
| Size, alignment and padding; `repr(C)` | 02 predictions; optional lab in the lesson 02 header |
| Associated functions, constants, and `self`, `&self`, `&mut self` receivers | 03 tasks A to C; drills d04, d05, d09 |
| Keeping an invariant behind a constructor and private fields | 03 tasks A and B; 09; R01 task A; drill d02 |
| How a method call picks between an inherent and a trait method | 03 predictions |
| `self: Box<Self>`, `self: Rc<Self>` and `self: Arc<Self>` | 04 tasks A to C and predictions |
| Generic structs and where to put trait bounds | 05 tasks A and B; optional lab in the lesson 05 header |
| Structs that borrow: lifetime parameters | 06 task B; R01 task C; drill d10 |
| Const parameters and array lengths in types | 06 task A and predictions |
| Derived `Clone`, `Debug`, `PartialEq`, `Eq`, `Ord`, `Default`; what each needs from the fields | 07 tasks and predictions; drill d06 |
| Writing `PartialEq` by hand when a derive compares too much | C01 |
| `Cell` and `RefCell`: change through `&self`, run-time borrow errors, guard lifetimes | 08 tasks and predictions; drill d07; C01 |
| `Rc` with `Weak` back-links, strong and weak counts, reference cycles | 10 tasks and predictions; drill d11 |
| Choosing the fields of a type yourself | 10 task A; 09 |
| Visibility seen from another crate | optional lab in the lesson 09 header (not checked by tests) |

Not in this chapter: typestate, `PhantomData`, and variance are in the mastery track ([`mastery/m1_types`](../../mastery/m1_types/README.md)); building `Cell`, `RefCell` and `Rc` from raw parts is in [`mastery/m2_unsafe`](../../mastery/m2_unsafe/README.md); `repr(transparent)` and `repr(packed)` are in Chapter 23.

## Solutions

Model answers for every lesson, the review, the code review and the drills are in [`solutions/exercises/chapter09/`](../../solutions/exercises/chapter09/) and [`solutions/drills/ch09/`](../../solutions/drills/ch09/). Open a solution only after an honest attempt: first your own version, then the hints, then the solution, and then explain to yourself every difference between it and your code. For the two lessons where you choose the fields, the solution shows one good design, not the only one.

## Mastery check

Answer from memory, in writing. Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Choose named, tuple, and unit structs and explain why tuple structs create distinct types.
- [ ] Keep invariants behind methods and choose owned, shared, or exclusive receivers deliberately.
- [ ] Use smart-pointer self receivers without accidentally cloning the underlying data.
- [ ] Combine type, lifetime, and const parameters and explain the role of each.
- [ ] State what derived traits do and which field traits they require.
- [ ] Use Cell and RefCell while explaining runtime borrow errors and the absence of arbitrary shared mutation.
- [ ] Link values with Rc in the owning direction and Weak in the other, and explain how a strong cycle leaks.
- [ ] Redo the final capstone after a break without looking at your implementation; invent a changed requirement and explain the tradeoff.
