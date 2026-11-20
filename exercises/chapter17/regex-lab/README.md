# Chapter 17 lab — Regular expressions

[Chapter guide](../README.md) · [Concepts](../CONCEPTS.md)

The search methods of lesson 03 find fixed text or single characters. When the thing to find has a structure, such as "some small letters, then `=`, then some digits", the usual tool is a **regular expression** (short: regex): a small language for describing text shapes. Rust's standard library has no regex support. The common choice is the [`regex`](https://docs.rs/regex/1.13.1/regex/) crate, and this lab is a small separate package that depends on it. The package pins regex 1.13.1.

The lab takes about 60 minutes (an estimate). Do it after lesson 03; task C also uses `parse` from lesson 05.

## What you need to know

The examples below were compiled and run with regex 1.13.1 on the course toolchain. They use a different pattern from the lab tasks.

**Compile once, then search.** `Regex::new` turns a pattern into a matcher. It returns a `Result`, because a pattern can have a syntax error. Write patterns as raw strings (`r"..."`), so that a backslash stays a backslash.

```rust
use regex::Regex;

// An hour and a minute such as 09:30. `\d` alone would also match digits of
// other scripts, so the pattern says [0-9].
let time = Regex::new(r"(?P<hour>[0-9]{2}):(?P<minute>[0-9]{2})").unwrap();
let text = "Zürich 09:30, Genève 14:05";

// 1. Only yes or no.
assert!(time.is_match(text));

// 2. Where, and what. Positions are byte offsets, as in lesson 03.
let first = time.find(text).unwrap();
assert_eq!((first.start(), first.end(), first.as_str()), (8, 13, "09:30"));
assert_eq!(time.find_iter(text).count(), 2);

// 3. The groups inside a match, by name or by number (0 is the whole match).
let caps = time.captures(text).unwrap();
assert_eq!((&caps["hour"], &caps["minute"], &caps[0]), ("09", "30", "09:30"));
let hours: Vec<&str> = time
    .captures_iter(text)
    .map(|caps| caps.name("hour").unwrap().as_str())
    .collect();
assert_eq!(hours, ["09", "14"]);

// Replacing. `${name}` inserts a group. The result is a Cow<str> (lesson 08):
// it borrows the input when nothing matched.
assert_eq!(
    time.replace_all(text, "${hour}h${minute}"),
    "Zürich 09h30, Genève 14h05"
);

// A pattern with a syntax error is an Err, not a panic.
assert!(Regex::new("[0-9").is_err());
```

A regex searches anywhere in the text unless the pattern says otherwise: `^` and `$` tie it to the start and the end. A match proves only the *shape*. The pattern above also matches `99:99`; whether the numbers make sense is a separate check that you do after matching.

**Which method?** `is_match` is the cheapest and answers yes or no. `find` and `find_iter` give the matched text and its byte range. `captures` and `captures_iter` also give the groups, and do the most work. Use the cheapest one that answers your question.

**One shared regex.** Compiling a pattern costs far more than using it, so a function must not call `Regex::new` each time it runs. A `static` would be the natural home, but a static needs a constant initializer, and `Regex::new` cannot run at compile time. [`std::sync::LazyLock`](https://doc.rust-lang.org/std/sync/struct.LazyLock.html) solves this: it stores a closure, runs it on first use, and keeps the result. If several threads arrive at the same time, one of them runs the closure and the others wait for it.

```rust
use regex::Regex;
use std::sync::LazyLock;

static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-z]+").unwrap());

fn count_words(text: &str) -> usize {
    WORD.find_iter(text).count() // WORD is compiled on the first call only
}

assert_eq!(count_words("one two"), 2);
assert_eq!(count_words("three"), 1);
```

## The tasks

Predict before you write code: does `find` return scalar positions or byte offsets? Does `[0-9]` mean the same as `\d`? When is the static regex compiled? Can two threads both run its initializer?

Implement the four contracts in `src/lib.rs`. Keep the supplied statics, and keep the supplied line at the top of `build_pattern` that counts compilations. Run from the repository root:

```sh
cargo test --manifest-path exercises/chapter17/regex-lab/Cargo.toml
cargo test --release --manifest-path exercises/chapter17/regex-lab/Cargo.toml
```

Cargo downloads the `regex` crate on the first run, which needs network access once. Add `--offline` when the crate is already in your Cargo cache. The starter compiles and its tests fail at `todo!()`. Until `build_pattern` is implemented, the initializer of the shared regex panics, so the other tests report that panic or a "poisoned" `LazyLock` (one whose initializer panicked earlier).

1. **Build** (task A): compile a pattern for a key of ASCII small letters, then `=`, then one or more ASCII decimal digits. The pattern must find such pairs anywhere in a text, so do not anchor it. Name the two groups `key` and `value`.
2. **Find** (task B): return every whole match with its byte range. Check the first test: after `"é "` the first match starts at byte 3.
3. **Capture** (task C): parse each captured value as `u32` and skip the values that do not fit. The pattern accepts a number of any length; the range check is a second step.
4. **Replace** (task D): rewrite every match as `key:value` with a replacement that uses the named groups. Text around the matches stays as it is.
5. **Share** (tests e and f): tasks B, C and D must use the shared `PAIRS` regex. Test e starts four threads and checks that the initializer ran once. Test f calls the three functions sixty times and checks that the pattern was not compiled again for each call.

## Optional labs

No test checks these. Try them in a scratch test inside this package.

- Compare `Regex::new(r"\d+")` and `Regex::new(r"[0-9]+")` on the text `"٧ 7"`. The first finds two runs of digits, the second only the ASCII `7`.
- Add a pattern for a whole record, with `^` and `$`, and see which inputs of task B it rejects.
- Write a pattern for a date such as `2024-02-30`. It matches. Explain why recognising the shape of a date is not the same as checking that the date exists.

## Hints

<details><summary>Hint 1</summary>

Write the pattern as a raw string. A named group is written `(?P<name>...)`. Byte offsets of a match are always char boundaries, so they are safe for slicing. Matching the shape of a number does not check its range.

</details>
<details><summary>Hint 2</summary>

`find_iter` gives matches with `start()`, `end()` and `as_str()`. `captures_iter` gives the groups; index them by name, then `parse::<u32>().ok()` and skip a `None`. `replace_all` takes a replacement text in which `${key}` and `${value}` stand for the groups; write the braces, because without them the name extends over every following letter, digit and underscore. Return owned Strings where the signatures ask for them.

</details>

## Solution

A model answer is in [solutions/exercises/chapter17/regex-lab/src/lib.rs](../../../solutions/exercises/chapter17/regex-lab/src/lib.rs). Open it only after an honest attempt and after both hints.
