# Chapter 22 — Writing macros

[All chapters](../../README.md) · [Previous chapter](../chapter21/README.md) · [Next chapter](../chapter23/README.md)

You have used macros since your first program: `println!`, `vec!`, `assert_eq!`. In this chapter you write your own with `macro_rules!`. A macro is code that produces code: the compiler replaces each macro call with the code that the macro describes, and then checks the result like anything else you write. That gives macros abilities that functions lack, such as taking any number of arguments, accepting a small syntax of their own, and generating functions, types and modules. It also brings traps that functions do not have: an argument that silently runs twice, a helper function that is found in the wrong module, an error message that points into the macro. You start with a macro that behaves like a function call. You go on to repetition, to the kinds of input a macro can ask for, to recursion over nested input, and to exporting a macro for other crates. Lesson 08 goes further than most everyday macros do: it takes a list apart one token at a time and carries state from step to step.

**Before you start:** Chapters 6, 8, 9, 10 and 11. The review lesson uses Chapters 15, 19 and 21. **Time:** about 9 to 10 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example, and shows the compiler errors you are likely to meet.
2. **Predict.** Open the lesson and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run the lesson, for example `cargo test --test chapter22 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations, evaluations or threads, so a macro that copies, evaluates an argument twice, or runs tasks one after another fails, and the failure message says what was expected. Then run `cargo clippy --test chapter22` and fix the warnings in code that you wrote. Warnings about macros that still contain `todo!()` disappear when you implement them.
5. **Drill the compiler.** `cargo drill ch22` shows small programs; predict whether each compiles (and with which error), then fix it.
6. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

Do the lessons in the order of the table. Lesson 08 comes before the capstone (lesson 07), although no lesson needs code from another one. After the numbered lessons, do the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: repair a teammate's macros, which work but are written badly). Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson | Coding tasks |
| --- | --- | --- |
| [ ] | [01 · Expansion and evaluation once](l01_expansion.rs) | 2 |
| [ ] | [02 · Repetition and separators](l02_repetition.rs) | 2 |
| [ ] | [03 · Type and identifier fragments](l03_fragments.rs) | 3 |
| [ ] | [04 · Recursive structured literals](l04_recursive.rs) | 4 |
| [ ] | [05 · Traits handle macro leaf types](l05_traits.rs) | 2 |
| [ ] | [06 · Exports and hygiene](l06_exports.rs), then the [export lab](export_lab/README.md) | 2 |
| [ ] | [08 · Patterns, items, and token munchers](l08_munchers.rs) | 3 |
| [ ] | [07 · Capstone: structured events](l07_capstone.rs) | 2 |
| [ ] | [R01 · Review: Chapters 15, 19, and 21 from memory](r01_review.rs) | 3 |
| [ ] | [C01 · Code review: a teammate's macro pull request](c01_code_review.rs) | 3 |
| [ ] | [Compiler drills](../../drills/ch22/) — `cargo drill ch22` | 10 |

Run commands from the repository root:

```sh
cargo test --test chapter22 l01_
cargo test --test chapter22 -- --list
cargo test --test chapter22
cargo test --release --test chapter22
cargo clippy --test chapter22
cargo drill ch22
cargo run --example ch22_scratch
cargo test --manifest-path exercises/chapter22/export_lab/Cargo.toml
```

The last command belongs to lesson 06. An exported macro must work when *another crate* calls it, and a test inside the chapter's own crate cannot prove that. The export lab is a second, small workspace that calls your `ch22_event!` from a crate of its own. Lesson 06 is finished when both commands pass.

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in. Two tasks of lesson 08 start from a rule that compiles but does too little, so their tests fail with a message instead. In the code review lesson, several tests fail because the supplied code has bugs. Keep the supplied tests as they are. Every task uses only the standard library and needs no network. Compiler experiments belong in [the scratch example](../../examples/ch22_scratch.rs); leave it compiling when you finish.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first.

## Topic map

"Optional lab" means an experiment or a small project that you do by hand. No test checks it.

| Topic | Where you practise it |
| --- | --- |
| A macro call is replaced by code; an argument that is written twice runs twice | Lesson 01 task A and prediction P1; C01 (`max_of!`); drill d03 |
| An `expr` argument stays one expression, with no surprise from operator precedence | Lesson 01 prediction P2; drill d04 |
| Built-in macros that report the place of the call (`file!`, `line!`, `module_path!`) | Lesson 01 task B; lesson 06 prediction P3 |
| Repetition, separators, and an optional trailing comma | Lesson 02 tasks A and B; lesson 05 task B; drill d05 |
| Nested repetition, and what counts as one token tree | Lesson 02 predictions; lesson 04 predictions P1 and P4 |
| The `ty`, `ident` and `literal` fragments; generating a struct with methods | Lesson 03 |
| A captured fragment is closed when it is passed to another macro | Lesson 03 prediction P2; lesson 04 prediction P3 |
| Recursive rules for nested input, and the order of rules | Lesson 04; R01 task C |
| Leaving decisions about types to traits | Lesson 05; drills d07 and d10 |
| Hygiene: a macro's local variables and the caller's are separate | Predictions in lessons 01, 03, 06 and 08; drills d02 and d08 |
| Names of functions are looked up at the place of the call; `$crate` | Lesson 06 task B with the export lab; C01 (`ch22_tags!`); drill d06 |
| Where a macro is visible: textual order, `#[macro_export]` | Drills d01 and d09 |
| The `pat` fragment, with guards and `A \| B` patterns | Lesson 08 task A |
| The `item` fragment, attribute lists, and an internal rule | Lesson 08 task B |
| The `vis` fragment, and generating a module of constants | Lesson 08 task C |
| A token muncher that carries state (an accumulator) | Lesson 08 task C and prediction P3 |
| What may follow a fragment in a matcher; `pat_param`; rule order with internal rules | Explained in CONCEPTS, lesson 08, with compiler messages; optional lab in the lesson 08 header |
| The edition-2024 meaning of `expr`, and `expr_2021` | Explained in CONCEPTS, lesson 03, with a checked example. No task. |
| A last rule with `compile_error!` for better messages | Explained in CONCEPTS, capstone section; optional lab in the lesson 07 header |
| Designing a small macro: grammar, order of evaluation, duplicates | Lesson 07 (capstone) |
| Reviewing macro code: double evaluation, helper paths, needless allocation, a swallowed error | C01 |
| Recall of Chapters 15, 19 and 21: futures, scoped threads, lazy iterators | R01 |
| Looking at expansions; `stringify!`, `concat!`, `env!`, `include_str!`, `cfg!`, `dbg!` | [LABS.md](LABS.md) Lab 1 (optional lab) |
| A first derive macro and attribute macro | [LABS.md](LABS.md) Labs 2 and 3 (optional labs; they need crates from the network); tested lessons in mastery module [M5](../../mastery/m5_proc_macros/README.md) |

Not in this chapter: tested lessons on procedural macros. They are in the mastery track ([`mastery/m5_proc_macros`](../../mastery/m5_proc_macros/README.md)).

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

1. How does a macro call differ from a function call, in particular for how often an argument is evaluated and for `return` inside an argument?
2. Which fragment fits an expression, a type, a name, a token tree, a pattern, a whole item, and a visibility? Which of them stay open when they are passed to another macro?
3. Where can a repetition be empty, and how does a macro accept a trailing comma?
4. How do recursive rules and traits divide the work between syntax and types?
5. Why does an exported macro name its helpers with `$crate`, and why can a test in the same crate not show the difference from `crate`?
6. What does hygiene cover, and what does it not cover?
7. When does a macro need a token muncher or an internal rule, and where must internal rules stand?
8. When is a procedural macro worth a crate of its own?

Redo the capstone from its contract in a later session. Explain one design decision, and add a case that the supplied tests do not cover.

## Solutions

Model solutions are in [solutions/exercises/chapter22/](../../solutions/exercises/chapter22/) (one file per lesson, with the predictions filled in) and [solutions/drills/ch22/](../../solutions/drills/ch22/) (each drill repaired, plus `expected.tsv` with the outcome of every drill). The solved lesson 06 also passes the export lab. Use the solutions after an honest attempt and after the hints: compare your macro with the model, and note in NOTES.md what differs and why. Reading a solution first removes the struggle that makes the lesson stay in memory.
