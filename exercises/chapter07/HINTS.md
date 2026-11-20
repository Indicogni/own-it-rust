# Chapter 7 hints — Handling errors

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [Optional labs](LABS.md) · [All hints](../../HINTS.md)

Try the first hint before opening the second. The implementation constraints in each task are part of the exercise. Sections are in study order, so lesson 09 comes before the capstone. Model answers for the README's mastery check are at the end.

## 01 · Panics and invariants

<details><summary>Hint 1</summary>

A panic represents a violated invariant here; use the exact requested message.

</details>
<details><summary>Hint 2</summary>

Use assert with the explicit string. The checked task should return Err, leaving panic for the invariant-only API.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A panic says that the program has a bug: an invariant the code relies on, such as "capacity is never zero", was broken, and continuing could do harm. By default the panicking thread *unwinds*: it runs the destructors of every live local, in reverse order, which is why the drop log in P1 is complete. With `panic = "abort"` the process stops immediately, no destructors run, and `catch_unwind` cannot catch anything. An empty label typed by a user is not a bug but an expected input, so `label` returns `Err` and lets the caller decide. A panic payload is a `&'static str` when the message needs no run-time formatting (a literal, or a format string whose arguments are all literals) and a `String` when a run-time value is formatted in; code that inspects payloads should check for both.

</details>

## 02 · Catching Result errors and printing context

<details><summary>Hint 1</summary>

Result is a normal enum; handle both paths explicitly.

</details>
<details><summary>Hint 2</summary>

Match on `parse::<u16>()`. Build the error message from the supplied name only, not from the standard library's parse wording, which is not a stable interface.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Matching on a `Result` is ordinary control flow: no unwinding happens and both arms are just code. Replacing every error with a fallback is a *recovery policy*, and it is only right when the caller truly does not care why parsing failed, because the reason (empty, invalid digit, overflow) is thrown away. A user-facing message such as "jobs: invalid count" adds context the parse error does not have (which setting failed) and stays stable even if the standard library rewords its `Display` text. A CLI prints diagnostics to stderr so they do not mix with the program's real output on stdout.

</details>

## 03 · Propagating failures with question mark

<details><summary>Hint 1</summary>

Use ? to stop at the first failure without writing repetitive nesting.

</details>
<details><summary>Hint 2</summary>

Parse both halves with trim().parse()?; collect or a loop with ? can preserve the same ParseIntError type.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`expr?` evaluates to the `Ok` value when the result is `Ok`; on `Err(e)` it returns early from the enclosing function with `Err(From::from(e))`. So the function itself must return a `Result` (or an `Option`, for `?` on options) whose error type can be built from `e`. Because `?` returns at the first failure, later inputs are not parsed, which P1 and P2 make visible. Preserving the `ParseIntError` instead of turning it into a default value keeps the failure visible and keeps its kind for the caller.

</details>

## 04 · Result methods and aliases

<details><summary>Hint 1</summary>

map transforms an Ok value; and_then chains another fallible operation.

</details>
<details><summary>Hint 2</summary>

Parse with map_err, then and_then for the nonzero check. For optional parsing, transpose converts Option<Result<T,E>> into Result<Option<T>,E>.

</details>
<details><summary>About the technique (after your tests pass)</summary>

The tests check results only, so a hand-written `match` passes too. The point of this lesson is the method chain. `positive` should read as: parse, `map_err` to the fixed message, `and_then` reject zero. `optional` should be `text.map(positive).transpose()`. Compare with `solutions/exercises/chapter07/l04_methods_aliases.rs` and rewrite your version if it differs in shape.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`map` transforms the `Ok` value and leaves an `Err` untouched; `map_err` does the reverse. `and_then` chains another fallible step: its closure returns a `Result`, so failures from either step come out flat, while `map` with a fallible closure would nest them. `unwrap_or(x)` evaluates `x` before the call, even on `Ok`; `unwrap_or_else(f)` calls `f` only on `Err`. `transpose` swaps `Option<Result<T, E>>` and `Result<Option<T>, E>`, so "absent" stays `Ok(None)` instead of becoming an error. A `type` alias is only another name for the same type: no conversion, no new API.

</details>

## 05 · Multiple error types at a boundary

<details><summary>Hint 1</summary>

A boxed Error trait object can hold different concrete errors and preserve their dynamic identity.

</details>
<details><summary>Hint 2</summary>

Read into a String with Read::read_to_string, then parse with ?. Both error types implement Error.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`?` converts any error that implements `Error` into `Box<dyn Error>` through a blanket `From` impl, so one boundary function can return I/O failures and parse failures through the same `Result`. The box keeps the concrete error alive and its dynamic type available: `is`, `downcast_ref`, and `source()` still work. What it hides is the static set of possibilities: the compiler can no longer check that a caller handles every case, and recovering means guessing types to downcast to. The box also costs an allocation, but only on the error path. Use a concrete error enum where callers recover by kind, and a boxed error at the application boundary where you mostly report. Lesson 09 shows the form of the box that can also cross threads.

</details>

## 06 · Custom errors with a source chain

<details><summary>Hint 1</summary>

Keep parse errors as values inside an enum, and use a separate variant for a valid number rejected by the domain.

</details>
<details><summary>Hint 2</summary>

Use write! for each variant. source returns the inner parse error only for Parse; map_err chooses that variant.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Debug` is for developers and is usually derived; `Display` is the message for users and you write it by hand. `Error::source` exposes the lower-level cause, so a caller or logger can walk the chain without parsing strings, and keeping the `ParseIntError` as a value preserves its kind. Not every error has a source: `TooLarge` is a domain rule broken by a perfectly valid number, so it returns `None`. Typed variants let a caller `match` on `TooLarge(n)` to recover, for example by clamping, while still reporting `Parse` failures.

</details>

## 07 · Impossible errors and deliberate recovery

<details><summary>Hint 1</summary>

An expect needs a local invariant you can explain; a public arbitrary input rarely supplies one. Here the invariant is "every entry of the built-in table is a u16 literal".

</details>
<details><summary>Hint 2</summary>

`literal.parse().expect("built-in limit must be a u16 literal")`: the return type tells `parse` what to produce, and on failure `expect` adds the underlying error after your message, which is what the second test looks for. For batch recovery, retain successful values and count rejected inputs explicitly.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`expect` is justified when failure would mean the program itself is wrong, not its input: parsing an entry of the program's own table can only fail if someone writes a bad entry, so the message states the promise ("built-in limit must be a u16 literal") and the panic points at the bug. Writing `let _ = fallible();` silences the `#[must_use]` warning but does not make the operation succeed; it states that you decided the error does not matter. `let _ =` also drops the value immediately, unlike `let _name =`, which keeps it alive to the end of the scope. `recover_rows` deliberately recovers, but it reports what it discarded as an error count instead of hiding it.

</details>

## 09 · Thread-safe boxed errors and what main returns

<details><summary>Hint 1</summary>

Task A is lesson 05's task with a loop: `?` converts each failure into the box, and `ok_or("...")` turns the `None` from a checked addition into an error that `?` can box. Task B is a single call. In task C, try `?` first and read the diagnostic; then remember that a value can be given a less specific type just by naming that type. In task D, `match` on the outcome and ask the error what it is with `is::<T>()`.

</details>
<details><summary>Hint 2</summary>

A: `for line in text.lines()`, trim, skip empty lines, `let value: u32 = line.parse()?;`, then `total = total.checked_add(value).ok_or("sum overflows u32")?;`. B: `on_worker(move || sum_lines(reader))`. C: `result.map_err(|error| error as Box<dyn Error>)`. D: `let _ = writeln!(stderr, "error: {error}");`, then `ExitCode::from(2)` if `error.is::<std::num::ParseIntError>()`, otherwise `ExitCode::FAILURE`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`dyn Error` names "some error type", so the compiler knows only what that name promises, and it promises nothing about threads; a `Box<dyn Error>` is therefore not `Send`, and `thread::spawn` (here `on_worker`) refuses to hand it back. Adding `+ Send + Sync + 'static` puts three promises into the type: the value may move to another thread, may be shared between threads, and borrows nothing. The promises cost nothing at run time (P1) and nearly every error type keeps them. Conversion works in one direction only: the thread-safe box coerces to the plain one because forgetting a promise is safe, but `?` cannot do it (there is no `From` impl), and the reverse is impossible. A `main` that returns `Result` prints `Error: ` with the error's **Debug** text and exits with status 1; that is fine for a quick tool. Return `ExitCode` when the message or the status is part of the program's interface: print with `Display` on standard error, choose the status, and still let destructors run by returning instead of calling `process::exit`.

</details>

## 08 · Capstone: fallible batch report

<details><summary>Hint 1</summary>

Use a typed enum for domain categories and carry the original one-based row number.

</details>
<details><summary>Hint 2</summary>

Parse each trimmed row, reject zero, and checked_add into u16. Convert each failure at its origin rather than losing the row number.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`ReportError` has a variant per thing a caller might act on (fix the syntax, remove the zero, split the batch), and each carries the row that caused it. Rows are processed in order and the first failure returns immediately, so the caller sees the earliest problem and never a partial sum presented as success. Parsing, the zero check, and `checked_add` each produce their error at the point where the row number is known. A CLI `main` can return `Result<(), Box<dyn Error>>` for quick tools (`ReportError` would first need `Display` and `Error` impls), or match on the report and return an `ExitCode` with a stable message on stderr, as in lesson 09; either way the process exits with a non-zero status on failure.

</details>

## R01 · Spaced review of Chapters 3, 5 and 6

<details><summary>Hint 1</summary>

An early `return Ok(Some(i))` inside the loop means later lines are never parsed. Give the field the lifetime of `record`. `u8::try_from` returns a `Result`, so it composes with `map_err` and `collect`.

</details>
<details><summary>Hint 2</summary>

In A, `let v: i64 = line.trim().parse()?;` inside `for (i, line) in lines.iter().enumerate()`. In B, `record.split(',').nth(index)` gives an `Option<&str>`; on `None`, count the fields with `record.split(',').count()`. In C, `values.iter().enumerate().map(|(i, &v)| u8::try_from(v).map_err(|_| i)).collect()`.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one turns bad input into a panic, and one turns bad input into a success. The third function is correct but builds Strings nobody reads.

</details>
<details><summary>Hint 2</summary>

Use `line.split_once('=')` and reject a value that still contains '='; return a real error type such as an enum with `Malformed` and `Parse(ParseIntError)` variants (or at least propagate with `?` after `map_err`). `limit_or_default` is `value.map(|v| v.trim().parse()).transpose().map(|n| n.unwrap_or(10))`. In `tally`, `match` the parse result and count errors with an integer.

</details>
<details><summary>Check your review</summary>

`setting` calls `unwrap()` on user-controlled text, so an invalid number panics instead of returning `Err` as documented; it also collects a `Vec` for every line and reports errors as `String`s, which callers can only print, not inspect. A typed error (or `Box<dyn Error>`) keeps the `ParseIntError` as a source. `limit_or_default` swallows a present-but-invalid value and silently returns the default, which hides configuration mistakes; the nested `match` is a hand-written `transpose`. `tally` checks `is_ok()` and then unwraps, which clippy flags, and formats every error into a String only to count them. Ignoring an error should be a decision you can see, and it should never cost an allocation.

</details>

## Drills

Run `cargo drill ch07`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · ? needs somewhere to return the error</summary>E0277: `?` returns the error early, but this `main` returns `()`, which cannot hold a `ParseIntError`. The line with `?` must stay, so give the error somewhere to go: `fn main() -> Result<(), std::num::ParseIntError>`, with `Ok(())` as the last line. The compiler suggests `Result<(), Box<dyn std::error::Error>>`, which works too. Lesson 09 explains what `main` does with an error it returns. In a function where the line may change, handling the error locally with `match` or `unwrap_or` is the other fix.</details>
<details><summary>d02 · ? converts errors only with From</summary>E0277: `?` calls `From::from` on the error, and there is no `From<io::Error>` for `ParseIntError`. Return `Result<u32, Box<dyn Error>>` and write `Ok(text.trim().parse()?)`, or define an enum with a variant for each error and implement `From` for both.</details>
<details><summary>d03 · An ignored Result</summary>Compiles and runs: an unused `Result` is only a `#[must_use]` *warning* (the drill runner hides warnings). The error still happened; nobody looked. In real code, handle it, propagate it with `?`, or write `let _ = save();` to say on purpose that it does not matter.</details>
<details><summary>d04 · unwrap on user input</summary>Panics at run time: 70000 does not fit in a `u16`, so `parse` returns `Err(PosOverflow)` and `unwrap` panics. The compiler cannot know what the text will contain. Decide a policy: `unwrap_or(1024)`, or return the error to the caller.</details>
<details><summary>d05 · Matching a Result</summary>E0004: `match` must be exhaustive, and `Err(_)` is not covered. Add an `Err` arm, or use `unwrap_or`, `if let`, or `?`.</details>
<details><summary>d06 · A &str is not a boxed error</summary>E0308: `Err("empty input")` has an error of type `&str`, but the function promises `Box<dyn Error>`. `?` would convert automatically; a plain `Err(...)` does not. Write `Err("empty input".into())`.</details>
<details><summary>d07 · What makes a type an Error</summary>E0277: `Error` requires both `Debug` and `Display`. Derive `Debug`, then implement `Display` by hand with the user-facing message (`write!(f, "quota exceeded: {} used", self.used)`).</details>
<details><summary>d08 · Returning a borrowed error</summary>E0308: matching on `&Result` binds `e` as `&ParseIntError`, but the function returns an owned `ParseIntError`. Clone it (`Err(e.clone())`), or better, take the `Result` by value so you can move the error out. `result.clone()` would replace the whole match.</details>
<details><summary>d09 · ? with a From impl</summary>Compiles and runs. `?` calls `From::from`, and the `impl From<ParseIntError> for AppError` makes the conversion exist, so `?` wraps the parse error in `AppError::BadNumber`. Compare with d02.</details>
<details><summary>d10 · A boxed error that leaves its thread</summary>E0277: `thread::spawn` requires the closure's result to be `Send`, and `Box<dyn Error>` is not. The type `dyn Error` promises nothing about threads, whatever error is inside. Change the return type of `parse` to `Result<u32, Box<dyn Error + Send + Sync>>`; `?` converts a `ParseIntError` into that box in the same way. See lesson 09.</details>

## Mastery check: model answers

Answer each item of the README's mastery check in NOTES.md first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**Choose Result for expected failures and panic for a documented broken invariant.** Ask who can cause the failure. If the outside world can (a user, a file, the network), the program is still correct and the caller needs a value to act on, so return a `Result`. If only a programming mistake can cause it, a documented rule of the function was broken and the program can no longer trust its state, so panic with a message that names the rule. `require_capacity(0)` is a caller bug; an empty label typed by a user is data.

**Explain unwinding versus aborting and why catch_unwind is not general error handling.** By default a panic unwinds: the thread leaves each active function and runs the destructors of its locals, so files are closed and locks released, and then the thread ends. With `panic = "abort"` the process stops at once and no destructor runs. `catch_unwind` stops an unwinding panic at a boundary, such as a thread pool or a foreign-function interface. It is not error handling because it catches nothing under `abort`, it gives you an untyped payload instead of an error value, the state touched by the panicking code may be half-updated (which is what the `UnwindSafe` bound is about), and a panic means a bug, which should be fixed and not routed around.

**Propagate errors with ? and preserve contextual information and source chains.** `expr?` gives the `Ok` value or returns `Err(From::from(error))` from the enclosing function, so the function must return a compatible `Result` (or `Option`). Context is what the lower error does not know: which setting, which row, which file. Add it where it is known, by wrapping the lower error in your own error with `map_err`, or through a `From` impl when no extra data is needed. Keep the lower error as a value and return it from `source()`; do not paste its text into your `Display` text as well. A reporter can then walk `source()` and print every level once.

**Choose between a typed error enum, `Box<dyn Error>`, and `Box<dyn Error + Send + Sync + 'static>` for a given boundary, and say what each choice costs the caller.** An enum lists every failure in the signature, so callers can `match` and the compiler checks that they covered each case; it costs you the type, its `Display` and `source`, and it makes adding a variant a visible change. A boxed error accepts anything with `?` and suits the top of an application that mostly reports; the caller pays by having to guess types to downcast, and each error costs one allocation. Between the two boxes, prefer the `Send + Sync + 'static` form whenever you box: it costs nothing at run time and it can cross threads, while the plain form cannot be returned from `thread::spawn` and cannot be converted into the stricter one later.

**Explain what ignoring an error means and make main return a useful exit status.** Ignoring an error means the failure still happened and nobody will find out. Do it only as a visible decision: `let _ = ...` for best-effort work, or recovery that reports what was dropped, such as an error count. For the process, return from `main` instead of calling `process::exit`, so destructors run. `fn main() -> Result<(), E>` prints `Error: ` with the `Debug` text and exits with status 1. For a tool that others use, return `ExitCode`: print a `Display` message on standard error, use 0 for success and a small documented set of non-zero values for failures.

**Redo the final capstone after a break; invent a changed requirement and explain the tradeoff.** One example of a changed requirement: "report every bad row, not only the first". The return type becomes something like `Result<(usize, u16), Vec<ReportError>>`, the loop can no longer use `?` to stop early, and the function now allocates on the error path. The caller gains a complete list to fix in one pass and loses the cheap, allocation-free failure. Another: "skip bad rows and report how many were skipped", which is the recovery policy of lesson 07 and changes the meaning of success. For any change, say what the caller can now do that it could not do before, and what it costs.

</details>
