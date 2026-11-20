# Chapter 3 — Numbers, text and arrays

[All chapters](../../README.md) · [Previous chapter](../chapter02/README.md) · [Next chapter](../chapter04/README.md)

Every Rust program is built from a small set of types: integers and floats, booleans, tuples, arrays, vectors, slices, text, and a few kinds of pointer. This chapter teaches what each of them stores, what it costs, and how it behaves at its limits: a sum that does not fit, a cast that drops bits, an index past the end, a string that is not ASCII. You read a short explanation, predict what small pieces of code will do, write the code yourself, and then explain it in your own words. When you finish, you can choose the right type for a job and say why.

- **Teaching text:** [CONCEPTS.md](CONCEPTS.md), one section per lesson.
- **Time:** about 11 hours for a first pass, over several sittings. This is an estimate; CONCEPTS.md gives one per lesson.
- **Before you start:** chapters 1 and 2 (running tests, functions, `let`, `if`, loops).

## Work through the chapter

Every lesson uses the same loop. It is built on *retrieval practice*: you keep what you predict, struggle with, and explain, not what you reread.

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md).
2. **Predict.** Open the lesson file and answer its `p_predictions` test: replace each `guess()` with a value. Don't run anything yet.
3. **Implement** the `todo!()` bodies from their contracts and run `cargo test --test chapter03 l01_`. A failed prediction is feedback: write in [NOTES.md](NOTES.md) why you were wrong, then open *Check your predictions* in [HINTS.md](HINTS.md).
4. **Check technique.** Some tests count heap allocations or compare pointers, so a copy-everything solution fails. Then run `cargo clippy --test chapter03` and fix the warnings that point at code you wrote. The warnings in `c01_code_review.rs` are planted for the code-review lesson; leave them until you get there.
5. **Explain** in [NOTES.md](NOTES.md), add one edge-case test of your own, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

After the numbered lessons:

- **Drill the compiler.** `cargo drill ch03` shows ten small programs, one at a time. For each, predict whether it compiles, panics, or runs (and which error code, if you can), then fix it. The drills draw on the whole chapter, so do them once the lessons are done.
- **Review a teammate's code** in the code-review lesson (C01): working-looking code with real bugs.

From chapter 4 on, each chapter also has a review lesson (R01) that brings earlier chapters back from memory. Run `cargo progress` to see where you are and which chapters are due for spaced review.

The unfinished exercises compile, but their tests fail on purpose at `todo!()` and `guess()`. Filtering by lesson (`l01_`) runs only that lesson. Leave the supplied tests as they are; you can add cases of your own. If you are stuck, reread the lesson's section in CONCEPTS.md, then open [HINTS.md](HINTS.md) one level at a time.

## Route through the chapter

Do the lessons in the order of this table. Check a lesson off after its tests **and** its explanation are complete. A lesson can take more than one sitting; the overflow, pointer and conversion lessons are the densest.

| Done | Lesson | What you should be able to do |
| --- | --- | --- |
| [ ] | [01 · Inference](l01_inference.rs) | Separate explicit function contracts from inferred local types; resolve ambiguous numeric methods. |
| [ ] | [02 · Integers](l02_integers.rs) | Read bases and suffixes, reason about ranges, and recognize deliberate truncating casts. |
| [ ] | [03 · Overflow](l03_overflow.rs) | Choose wrapping, strict, checked, saturating, or overflowing arithmetic; handle shift distances. |
| [ ] | [04 · Floating point](l04_floats.rs) | Use explicit numeric conversions, tolerance, NaN classification, and signed zero. |
| [ ] | [05 · Booleans, tuples, unit](l05_bool_tuples.rs) | Write explicit conditions, destructure multiple results, and recognize `()`. |
| [ ] | [06 · Pointer types](l06_pointers.rs) | Distinguish borrowing, heap ownership, shared ownership, optional references, and raw pointers. |
| [ ] | [07 · Arrays](l07_arrays.rs) | Use fixed lengths, repeated initialization, nested storage, and checked indexing. |
| [ ] | [08 · Vectors](l08_vectors.rs) | Build, edit, grow, and pop vectors; distinguish length from capacity. |
| [ ] | [09 · Slices](l09_slices.rs) | Accept arrays and vectors through one API and work on borrowed regions of their data. |
| [ ] | [10 · Unicode and memory](l10_unicode.rs) | Separate UTF-8 bytes, Unicode scalar values, and valid string slicing boundaries. |
| [ ] | [11 · Strings and literals](l11_strings.rs) | Build owned text, borrow input, use raw literals, and remove a whole Unicode scalar. |
| [ ] | [12 · Boundary types and aliases](l12_boundary_types.rs) | Choose text, bytes, paths, OS strings, or C strings at system boundaries; use an alias. |
| [ ] | [14 · Checked conversions and numeric edge cases](l14_conversions.rs) | Convert with `try_from` and `parse`, predict float-to-integer casts, avoid the `i32::MIN` traps, pick `rem_euclid` and `total_cmp`, and name a byte order. |
| [ ] | [13 · Capstone](l13_capstone.rs) | Combine these decisions into a small sensor-report API. |
| [ ] | [C01 · Code review: a teammate's pull request](c01_code_review.rs) | Find overflow, Unicode, and copying bugs in working-looking code; fix them and clear clippy. |
| [ ] | [Compiler drills](../../drills/ch03/) — `cargo drill ch03` | Predict which programs compile, panic, or run, then fix them. |

Lesson 14 was added after the capstone was numbered, so its file name comes last. Study it before the capstone, as the table shows.

```sh
cargo test --test chapter03 l01_
cargo test --test chapter03 -- --list
cargo test --test chapter03
cargo test --release --test chapter03
cargo clippy --test chapter03
cargo drill ch03
cargo run --example ch03_scratch
```

The release run matters: explicit overflow methods should keep the same behavior across build profiles.
The exercises use recent standard-library methods such as `strict_mul`, so use the repository's pinned Rust 1.99.0 toolchain (see the setup instructions in the [course guide](../../README.md)).
No extra crates or network access are needed.

## Topic map

| Topic | Where you practise it |
| --- | --- |
| Type inference, default numeric types, method calls on literals | Lesson 01; drill d01 |
| Integer widths and ranges, literal bases and suffixes, integer `as` casts | Lesson 02; drill d02 |
| Overflow policies (checked, wrapping, saturating, overflowing, strict) and shift distances | Lesson 03; lesson 13; C01; drill d03 |
| Floating point: classification, NaN, signed zero, tolerance, explicit conversion | Lesson 04; drill d04 |
| `bool` conditions and casts, tuples, the unit type | Lesson 05; drills d05 and d06 |
| References, `Box`, `Rc`, `Arc`, optional references | Lesson 06 |
| Raw pointers | Lesson 06, predictions only; reading through one is an optional lab |
| Arrays: fixed length, repeat syntax, nesting, checked access | Lesson 07; drill d07 |
| Vectors: building, editing, length and capacity | Lesson 08 |
| Slices: borrowed regions, slice parameters, in-place edits | Lesson 09; C01 |
| UTF-8 bytes, Unicode scalar values, char boundaries | Lesson 10; C01; drills d08, d09 and d10 |
| `String` and `&str`, `format!`, escaped and raw literals | Lesson 11; lesson 13 |
| Byte strings, paths, OS strings, C strings, type aliases | Lesson 12 |
| `try_from`, float-to-integer casts, `i32::MIN`, `rem_euclid`, `total_cmp`, byte order, `parse` | Lesson 14 |
| Reading compiler diagnostics for these types | Compiler drills; the optional labs below |

Left for later chapters: the ownership rules behind `Box`, `Rc` and `Arc` (chapter 4), the borrowing rules behind references and slices (chapter 5), structs as a replacement for tuples and aliases (chapter 9), text processing in depth (chapter 17), and using raw pointers (chapter 23).

## Compiler labs (optional)

Each lesson header ends with a short `COMPILER LAB`. These are optional experiments with no tests: you type a few lines into the scratch program and read what the compiler says.

```sh
cargo run --example ch03_scratch
```

Some labs are supposed to fail compilation or panic. Record **which** happens and why, then restore the scratch file to a compiling state. Keep intentional failures out of the exercise suite.

## Solutions

Model answers for every lesson, the capstone, the code review and the drills are in [`solutions/exercises/chapter03/`](../../solutions/exercises/chapter03/) and [`solutions/drills/ch03/`](../../solutions/drills/ch03/). Each file is the exercise file with the bodies and predictions filled in.

Use them after an honest attempt: your own version passes, or you have used both hints and are still stuck. Then compare. A different solution that passes the tests is not wrong; ask what each version makes easier to read or harder to misuse.

## Mastery check

After the capstone, answer these without opening the chapter:

1. Which type declarations does Rust require, and where can inference fill in the rest?
2. What ranges do `u8` and `i8` represent? Why are indices `usize`? Does `as` always preserve a number's value?
3. Which overflow API fits a pixel intensity, a cyclic counter, and a size computation that must reject overflow?
4. Why are NaN, signed zero, and approximate equality worth thinking about?
5. How do a tuple, an array, and `()` differ?
6. Who cleans up data reached through `&T`, `Box<T>`, `Rc<T>`, and `Arc<T>`? What distinguishes a raw pointer?
7. What is stored in `[T; N]`, `Vec<T>`, and `&[T]`? Which of them owns the elements?
8. Why is `&[T]` often a better parameter than `&Vec<T>`?
9. What does string length count? How do a `char`, a Unicode scalar value, and a user-perceived character relate?
10. What is the relationship between `String` and `&str`? Where does string-literal text live?
11. When would `PathBuf`, `OsString`, `Vec<u8>`, or `CString` be more appropriate than `String`?
12. Does a type alias create a distinct type with its own validation rules?
13. How do `From`, `TryFrom`, and `as` differ when a value does not fit? What does a float-to-integer cast do with a value that is too large, and with NaN? Why is `%` the wrong tool for a clock?

Model answers are at the end of [HINTS.md](HINTS.md#mastery-check-model-answers), in a collapsed block.

In a later session, redo the capstone from its contract without looking at your implementation. Try two transfer exercises: accept `&[i16]` and explicitly design negative-value behavior; then replace the capstone's tuple with a named struct when you reach the structs chapter.
