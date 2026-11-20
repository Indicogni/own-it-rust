# Chapter 17 — Working with text

[All chapters](../../README.md) · [Previous chapter](../chapter16/README.md) · [Next chapter](../chapter18/README.md)

Almost every program reads text and writes text, and text is harder than it looks. A Rust `String` is a buffer of UTF-8 bytes, a `char` is one Unicode scalar value, and what a person calls "a character" can be several of those. In this chapter you learn to keep these levels apart and to work on each of them safely. You count and classify characters, edit a `String` in place without cutting a character in half, search and split text with patterns, and read the byte positions that the search methods return. You turn text into your own types with `FromStr` and your own types into text with `Display`. You learn the formatting language behind `format!` and `println!`, write formatted output into any destination without a temporary `String`, convert between bytes and text, and use `Cow` to allocate only when something really changes. At the end you can write a small parser and a report printer that stay correct when the input is not ASCII.

The teaching text is in [CONCEPTS.md](CONCEPTS.md), one section per lesson. This chapter has **9 lessons with 28 implementation tasks**, a review lesson, a code-review lesson, and 10 compiler drills: **52 automated checks** in total, plus predictions, explanations, optional labs, and one separate lab package for regular expressions.

**Before you start:** Chapters 3 to 5, 7, 11, 14 and 15. The review lesson also uses Chapters 10 and 16. **Time (estimate):** about 10 to 11 hours for the whole chapter with the regex lab, in sessions of 45 to 60 minutes. [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson.

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you only read.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md). It explains the idea, shows a worked example, and shows the compiler errors and panics you are likely to meet.
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Do not run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run the lesson, for example `cargo test --test chapter17 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong.
4. **Check technique.** Some tests count heap allocations or compare buffer pointers, so a solution that builds throwaway Strings where the contract says *borrow* or *no intermediate String* fails, and the failure message says what was expected. Then run `cargo clippy --test chapter17` and fix the warnings in code that you wrote.
5. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare your explanation with *Check your explanation* in [HINTS.md](HINTS.md). If you have time, do the lesson's optional lab in [the scratch example](../../examples/ch17_scratch.rs).

After lesson 08, **drill the compiler**: `cargo drill ch17` shows ten small programs. For each one, predict whether it compiles (and with which error code), runs, or panics; then fix it. After that, do the **capstone**, the **review** lesson (r01: earlier chapters, from memory) and the **code review** lesson (c01: repair a teammate's code that works but is written badly). The **regex lab** can be done at any time after lesson 03. Run `cargo progress` to see where you are and which chapters are due for spaced review.

| Done | Lesson | Check |
| --- | --- | --- |
| [ ] | [01 · Unicode scalar operations and encoding](l01_unicode.rs) | `cargo test --test chapter17 l01_` |
| [ ] | [02 · Constructing and editing owned text](l02_editing.rs) | `cargo test --test chapter17 l02_` |
| [ ] | [03 · Search patterns, byte positions, and replacement](l03_search.rs) | `cargo test --test chapter17 l03_` |
| [ ] | [04 · Text iteration, trimming, and casing](l04_iteration.rs) | `cargo test --test chapter17 l04_` |
| [ ] | [05 · FromStr and textual conversion](l05_parsing.rs) | `cargo test --test chapter17 l05_` |
| [ ] | [06 · The formatting language](l06_formatting.rs) | `cargo test --test chapter17 l06_` |
| [ ] | [07 · Display, fmt::Write, and format_args!](l07_custom_format.rs) | `cargo test --test chapter17 l07_` |
| [ ] | [08 · UTF-8, borrowed text views, and delayed allocation](l08_representations.rs) | `cargo test --test chapter17 l08_` |
| [ ] | [Compiler drills](../../drills/ch17/) | `cargo drill ch17` |
| [ ] | [09 · Capstone: a Unicode-aware report parser](l09_capstone.rs) | `cargo test --test chapter17 l09_` |
| [ ] | [R01 · Review: Chapters 3, 7, 10, 15, 16 from memory](r01_review.rs) | `cargo test --test chapter17 r01_` |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) | `cargo test --test chapter17 c01_` |
| [ ] | [Regex lab (separate package)](regex-lab/README.md) | `cargo test --manifest-path exercises/chapter17/regex-lab/Cargo.toml` |

Run commands from the repository root:

```sh
cargo test --test chapter17 l01_
cargo test --test chapter17 -- --list
cargo test --test chapter17
cargo test --release --test chapter17
cargo clippy --test chapter17
cargo drill ch17
cargo run --example ch17_scratch
```

The starters compile. Their tests fail on purpose at `todo!()` and `guess()` until you fill them in; in the code review lesson, most tests fail because the supplied code has bugs. Keep the supplied tests as they are. Every task of the chapter target uses only the standard library and needs no network.

Open the [hints](HINTS.md) only after a first attempt. Each lesson has two hints, and the second says more than the first. Passing tests alone do not show mastery; the predictions and your written explanations matter as much.

Some optional labs fail to compile or panic on purpose. Keep them in the scratch example and leave it compiling when you finish. Do not put code that must not compile into the test modules.

## Regex lab

The standard library has no regular expressions. The [regex lab](regex-lab/README.md) is a small separate package that uses the `regex` crate: you compile a pattern once, find matches with their byte ranges, read captured groups, and replace matches. It has its own manifest, its own tests and its own short teaching text. It is not run by `cargo test --test chapter17`, and Cargo may need network access once to download the crate.

## Solutions

Model answers for every lesson, the review, the code review and the regex lab are in [solutions/exercises/chapter17/](../../solutions/exercises/chapter17/). The repaired drills and the expected outcome of every drill are in [solutions/drills/ch17/](../../solutions/drills/ch17/). Open a solution only after an honest attempt and after both hints. Then compare it with your version and write in NOTES.md what differs and why: a different answer that passes the tests and clippy can be just as good. Reading a solution first removes the struggle that makes the lesson stay in memory.

## Topic map

Each row names a topic and the place where you really practise it. "P1" means the first prediction of that lesson. An "optional lab" is an experiment that you run by hand in the scratch example; no test checks it.

| Topic | Where you practise it |
| --- | --- |
| Bytes, scalars and byte offsets of UTF-8 text | Lesson 01 (task A, P1); drills d01 and d02 |
| Converting between numbers and `char`; code points that are not scalars | Lesson 01 (task B, P2, P4) |
| Classifying a `char`; ASCII digits and digits of other scripts | Lesson 01 (task C); optional digit lab |
| Case changes that produce more than one scalar | Lesson 01 (task B, P3); lesson 04 (task B); R01 task B |
| One visible character made of several scalars; text direction | Lesson 01 (P1) for the counts; the rest is an optional lab. The standard library has no grapheme, normalization or layout support, and no lesson tests them. |
| Building a `String` with `push` and `push_str`; capacity | Lesson 02 (task C, P4); drills d03, d04 and d05 |
| Editing in place at checked byte boundaries: insert, truncate, pop, remove, drain | Lesson 02 (tasks A and B, P1 to P3) |
| Searching with `char`, text and closure patterns; positions as byte offsets | Lesson 03 (tasks A, B and D, P1 to P4) |
| Replacing a limited number of matches | Lesson 03 (task C) |
| Lines, splitting and the end of the text; CRLF line endings | Lesson 04 (task A, P1 and P2); capstone |
| Trimming whitespace and trimming by pattern, as borrowed slices | Lesson 04 (tasks A and C, P4) |
| Lowercasing: a new `String`, and whole-string versus per-char results | Lesson 04 (P3); drill d06; C01 |
| `FromStr` for your own type; strict number parsing; error kinds | Lesson 05 (task A, P1 to P3); drill d09; R01 task C |
| A text format that parses back to the same value; `to_string` for numbers | Lesson 05 (task B, P4) |
| Parsing numbers in another base | Lesson 05 (task C) |
| Format strings: width, alignment, precision, sign, zero fill, number bases, `Debug` | Lesson 06 (tasks A to C, P1 to P4); lesson 07 (P3); drill d10 |
| Width and precision from arguments | Lesson 06 (task A) |
| How the formatter rounds floats | Lesson 06 (task A, P5) |
| Pointer formatting with `{:p}` | Optional lab in lesson 06 only |
| `Display` for your own type, with the alternate flag | Lesson 07 (task A); drill d07 |
| How `write!` and `Formatter::pad` treat the caller's width | Lesson 07 (P1 and P2) |
| Writing into any `fmt::Write` destination; forwarding `fmt::Arguments`; passing write errors on | Lesson 07 (task B) |
| Bytes to text, strictly and with repair; getting the buffer back from a failed conversion | Lesson 08 (tasks A and B, P1 and P2) |
| Text to bytes without a copy | Lesson 08 (task D, P3) |
| `Cow<str>`: borrow when nothing changes | Lesson 08 (task B, P4); C01 |
| Joining borrowed pieces into one `String` | Lesson 08 (task C) |
| Comparing `String` with `&str` | Drill d08 |
| `OsStr`, `Path` and other text that is not UTF-8 | Explained in CONCEPTS, lesson 08, with an optional lab. Paths are practised in Chapter 18, C strings in Chapter 24. |
| A line-based parser with line numbers for rejected input, and a report | Lesson 09 (capstone) |
| Finding byte-versus-scalar bugs and needless allocations in existing code | C01 |
| Earlier chapters from memory: tree maps, iterator chains, error enums with `?` | R01 |
| Regular expressions: named groups, byte ranges of matches, replacement, one shared compiled regex | [Regex lab](regex-lab/README.md) (separate package with its own tests) |

Not practised here: Unicode normalization, grapheme clusters and display width need crates from outside the standard library; CONCEPTS says where the limits are. The design question "which text types should a public function accept and return" comes back in the mastery track ([`mastery/m8_api`](../../mastery/m8_api/README.md)).

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

- [ ] Explain the difference between bytes, Unicode scalars, grapheme clusters and terminal columns, and say which of them `len()`, `chars().count()` and a format width count.
- [ ] Say which `String` and `str` operations take or return byte offsets, what happens at an offset inside a character, and how you get offsets that are always safe.
- [ ] Explain why changing case can change the length of a text, why lowercasing a whole string can differ from lowercasing each char, and what `String::from_utf8` does with the buffer when it succeeds and when it fails.
- [ ] Implement `FromStr` and `Display` for a small type from memory. Say what `fmt::Error` means, and how a function writes formatted text into any destination without a temporary `String`.
- [ ] Explain when a `Cow<str>` allocates, and why file paths are not `String`s.
- [ ] Explain why a regex is compiled once in a `LazyLock`, and choose between `is_match`, `find` and `captures` for a given job.
- [ ] Redo the capstone after a break without looking at your first solution. Then invent a changed requirement and explain what it changes in the design.
