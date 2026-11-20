# Chapter 6 hints — Expressions and control flow

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Try the first hint before opening the second. The implementation constraints in each task are part of the exercise.

When a task names a construct (for example "use a labeled block"), one test reads your source file and looks for that construct inside the body of the task's own function. If such a test fails although your results are right, move the construct into that function; a helper function elsewhere is not seen. Comments and text inside string literals are ignored.

## 01 · Expression blocks and declarations

<details><summary>Hint 1</summary>

A block returns its final expression when there is no terminating semicolon.

</details>
<details><summary>Hint 2</summary>

Shadow an input text binding with its byte length. Nested fn items cannot capture the surrounding local environment.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A block evaluates its statements in order and takes the value of its final expression; if the last item ends in a semicolon, the block's value is `()`. That is why `{ 3 }` is an `i32` and `{ 3; }` is unit, and why an `if` whose branches disagree on that semicolon fails to type-check. Shadowing with a new `let` creates a new binding, which may have a different type, while `mut` changes the value of the same binding and keeps its type. A shadow inside an inner block ends with that block. A nested `fn` item is visible throughout its enclosing block, even before its definition, but it cannot capture local variables; only closures can. A temporary value created in a block's tail expression is dropped at the end of the block *before* the block's local variables (edition 2024), so in P4 the order is the temporary first, then the locals in reverse order of declaration.

</details>

## 02 · if and match as values

<details><summary>Hint 1</summary>

Classify into disjoint conditions and make the whole branch construct the result.

</details>
<details><summary>Hint 2</summary>

Use if/else for a bounded comparison and a match with a wildcard for the remaining integer range.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`if` and `match` are expressions, so when their value is used every branch must produce a compatible type, and an `if` without `else` can only produce `()`. `match` must be exhaustive: the compiler proves that every possible value of the scrutinee hits some arm, which for integers usually means a final `_` or a range that reaches the end. Arms are tried from top to bottom and the first match wins, so a broad pattern placed early makes later arms unreachable (the compiler warns). The scrutinee is evaluated once, while an `if / else if` chain evaluates each condition separately.

</details>

## 03 · if let and let else

<details><summary>Hint 1</summary>

if let is a convenient one-pattern conditional; let else keeps successful bindings in the surrounding scope.

</details>
<details><summary>Hint 2</summary>

Use `if let Some(...)` with an `else` in task A. In task B, use one let else for the colon (`split_once(':')` gives an `Option`), then trim the key and reject an empty one with an ordinary `if`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A refutable pattern such as `Some(x)` can fail to match; an irrefutable one such as `(a, b)` cannot. `if let` runs its block only when the pattern matches, and its bindings exist only inside that block. `let else` binds into the surrounding scope, so the rest of the function can use the values; in exchange, its `else` block must diverge (`return`, `break`, `continue`, or panic), because there is no value to bind if the pattern fails. Use `if let` for an optional adjustment and `let else` to guard the remainder of a function. A temporary in the condition of an `if let` lives through the first block but is dropped before the `else` block runs (edition 2024). A temporary in the scrutinee of a `match` lives until the end of the whole `match`, so it is still alive inside every arm.

</details>

## 04 · for, while, break and continue

<details><summary>Hint 1</summary>

Two different things can happen to a value: it ends the scan, or it is skipped. Decide which test each one needs, and check that no value can pass both tests.

</details>
<details><summary>Hint 2</summary>

Use break at the sentinel and continue for negative values. Widen i16 before adding; use a while loop for repeated halving.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`for` drives an iterator until it is exhausted; `while` re-checks its condition before every iteration, so the condition runs one more time than the body. `continue` skips the rest of the current iteration, while `break` ends the loop. In the sentinel task the two tests `v == 0` and `v < 0` can never both be true, so they may come in either order. A wider filter such as `if v <= 0 { continue; }` is different: placed before the sentinel check it would swallow the zero, so the loop would never break at the sentinel and would add the values after it. The `for` loop ends because the slice runs out or `break` fires; the `while` loop ends because `n /= 2` makes `n` smaller on every pass until it is 0. `0..n` excludes `n`; `0..=n` includes it.

</details>

## 05 · Loop values and labeled blocks

<details><summary>Hint 1</summary>

An unconditional loop can return a value with break; a labeled block can return early.

</details>
<details><summary>Hint 2</summary>

Use loop to grow a u32 until its square reaches the bound; u64 multiplication avoids overflow. Use a labeled block to reject a missing or empty half.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`loop` is the only loop that can produce a value with `break value`, because it is the only loop the compiler knows runs until a `break`; `for` and `while` might finish without one, so they have type `()`. Labels let `break` and `continue` target an outer loop, and a labeled block lets you `break 'label value` out of straight-line code. `return` leaves the whole function. Expressions like `return`, `break`, and `panic!` have the never type `!`, which coerces to any type, so a branch that returns can sit beside a branch that produces an integer. An unconditional `loop` whose only exits are `break n` initializes a plain `u32` without an `Option`.

</details>

## 06 · const and unsafe expression blocks

<details><summary>Hint 1</summary>

const evaluates expressions at compile time; unsafe acknowledges proof obligations and does not remove ordinary checks.

</details>
<details><summary>Hint 2</summary>

Write the array length as an inline const block: `[0; const { 3 * 4 }]`. Use checked indexing (`get`) for the safe task, and keep pointer dereferencing for the scratch experiment.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

An inline `const { ... }` block is evaluated by the compiler, so it can only use constant expressions, not runtime locals, and its result can appear in types such as array lengths. `unsafe` does not change what code does or make it faster; it only allows a few operations the compiler cannot check, such as dereferencing raw pointers or calling `unsafe fn`s. Each use comes with a proof obligation: the pointer is non-null, aligned, points to a live, initialized value, and does not break aliasing rules. These tasks need none of that, because `slice::get` checks bounds safely and returns `Option`.

</details>

## 07 · Operators, calls, fields and assignment

<details><summary>Hint 1</summary>

&& and || short-circuit. Parenthesize bit operations when combining them with comparisons.

</details>
<details><summary>Hint 2</summary>

Mask each nibble and shift the high half. In the second task put the counter increment on the right-hand side of &&.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Operands are evaluated left to right (except that an assignment evaluates its right side before the place on its left). `&&` and `||` short-circuit: the right side runs only when the left side does not already decide the result, which lets you guard side effects. `&&` binds tighter than `||`, and unary minus binds looser than a method call, so `-2i32.pow(2)` is `-(2.pow(2))`. Assignment is an expression of type `()`, so `a = b = c` does not chain as it does in C: it is a type error unless `a` itself has type `()`. Comparison operators do not associate, so `a < b < c` is a syntax error. Masks with `&` keep chosen bits, `|` combines them, and `<<` moves them; on a signed type, `>>` copies the sign bit.

</details>

## 08 · Casts and closure expressions

<details><summary>Hint 1</summary>

Numeric casts need explicit policy. A closure can carry the threshold in its environment.

</details>
<details><summary>Hint 2</summary>

Use as for intentionally truncated low bits. Return a move closure whose input type and result are fixed by impl Fn.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`as` never panics: integer-to-integer casts keep the low bits (so `300u16 as u8` is 44 and `-1i8 as u8` is 255), and float-to-integer casts truncate toward zero, saturate at the type's range, and turn NaN into 0. A closure captures each variable it uses in the least demanding way it can, by shared reference, mutable reference, or by value; `move` forces capture by value. Moving a `Copy` value such as an `i32` into a closure just copies it, so the closure is still `Fn` and can be called repeatedly. A closure becomes `FnOnce` only when its body *consumes* something it owns, which is different from capturing by move.

</details>

## 09 · Capstone: command expression engine

<details><summary>Hint 1</summary>

Parse line by line while preserving original one-based positions.

</details>
<details><summary>Hint 2</summary>

Trim first; skip blanks and # comments; match stop before parsing a signed number; use checked_add and map errors to the current line.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Each line is classified with expressions: `trim`, a `match` or `if` on the comment marker, and `parse` mapped to `Result<Option<i32>, ()>`. The loop uses `enumerate` to keep one-based line numbers, `continue` for skipped lines, `break` (or an early `return Ok(sum)`) at `stop`, and an early `return Err(line)` for both parse errors and `checked_add` overflow. Because the function returns as soon as it sees `stop`, later lines are never decoded, so garbage after `stop` cannot cause an error. Every path ends in a `Result`, and the borrowed lines are never copied, so the engine allocates nothing.

</details>

## R01 · Spaced review of Chapters 3–5

<details><summary>Hint 1</summary>

Give the result the lifetime of the lines, not of the key. `Option::take` moves a value out of a slot through `&mut` and leaves `None`. Widen before you add.

</details>
<details><summary>Hint 2</summary>

`let Some((k, v)) = line.split_once('=') else { continue };` then compare `k.trim()`. In B, `for slot in slots.iter_mut()` with `match slot.take() { Some(m) => out.push(m), None => break }`. In C, sum with `i64::from(v)`, divide by `values.len() as i64`, and cast the mean back with `as i32` (it always fits).

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one range forgets its last value, and one cast silently throws bits away. The first function is correct but allocates and keeps looping after it has its answer.

</details>
<details><summary>Hint 2</summary>

Change the parameter to `&[String]` and iterate with `iter().position(|l| l.trim() == "end").map(|i| i + 1)`; `position` stops at the first match. Write `classify` as one `match code { 100..=199 => "info", ... }`. Parse each entry as `u32` and combine with `checked_add`, so both invalid text and overflow become `None`.

</details>
<details><summary>Check your review</summary>

`end_line` allocates a String per line just to compare it (`trim()` already gives a `&str`), compares `Option` to `None` with `==`, uses a manual index loop, keeps scanning after the first match, ends with a needless `return`, and takes `&Vec<String>` where `&[String]` will do. `classify` uses a mutable flag set by four separate `if`s; a `match` with inclusive ranges says the same thing as one expression, and would have made the `< 599` bug (599 is classified as invalid) obvious. `total_size` accumulates in `u64` and then casts with `as u32`, which wraps silently instead of reporting overflow, and a huge `u64` entry can still overflow the accumulator itself. Clippy also flags the manual range checks and the `i = i + 1` style assignments.

</details>

## Drills

Run `cargo drill ch06`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · An if used as a value</summary>E0317. Without `else`, an `if` evaluates to `()` when its condition is false, so it cannot also produce an integer. Add `else { 0 }` (or whatever the other case should be).</details>
<details><summary>d02 · The semicolon that changes a type</summary>E0308. `twice;` is a statement, so the function body evaluates to `()` instead of `i32`. Remove the semicolon so `twice` is the tail expression.</details>
<details><summary>d03 · Matching on an integer</summary>E0004: the arms cover 0..=99, but a `u8` can be 100..=255. Add a final `_ => "invalid"` arm. The compiler proves exhaustiveness; it does not trust you.</details>
<details><summary>d04 · let else must leave</summary>E0308: the `else` block of `let else` must diverge (have type `!`), because there is nothing to bind `n` to. Write `return 0;` inside it, or use `unwrap_or(0)` if a fallback value is what you want.</details>
<details><summary>d05 · break with a value</summary>E0571: only `loop` can `break` with a value. A `for` loop can end without a `break`, so it always has type `()`. Use `return r` to leave the function, or `readings.iter().copied().find(|&r| r > 10).unwrap_or(0)`.</details>
<details><summary>d06 · Chained comparisons</summary>A parse error with no error code (answer `error`): Rust refuses `a < b < c`, because comparing a `bool` with `high` is almost never what you mean. Write `low < value && value < high`, or `(low + 1..high).contains(&value)`.</details>
<details><summary>d07 · Unsigned subtraction</summary>Panics in a debug build with "attempt to subtract with overflow": `2 - 3` does not fit in a `u8`. The compiler cannot reject it because `used` comes from run-time parsing. Use `saturating_sub` (stop at 0) or `checked_sub` (get an `Option`). In release builds the default is to wrap, so make the policy explicit.</details>
<details><summary>d08 · Casts never panic</summary>Compiles and runs. `as` between integer types keeps the low bits: 300 becomes 44 and -1 becomes `u32::MAX`. Use `u8::try_from(big)` when a value that does not fit should be an error.</details>
<details><summary>d09 · Initialized on every path?</summary>E0381: when `n == 0` neither branch assigns `label`, so it may be uninitialized at the `format!`. Add `else { label = "zero"; }`, or better, write `let label = if n > 0 { ... } else if n < 0 { ... } else { "zero" };`.</details>
<details><summary>d10 · A closure that borrows mutably</summary>E0502: `inc` holds a mutable borrow of `count` from its creation until its last use, so reading `count` in between is a conflict. Move the `println!` after the last call to `inc()`, so the closure's borrow has ended.</details>
<details><summary>d11 · Temporaries that hold a borrow</summary>E0499, and only in `next_job`. `queue.drain(..)` returns a `Drain` value that no variable names: a temporary. It borrows the queue mutably, and its destructor uses that borrow, so the borrow lasts until the temporary is dropped. A temporary in the scrutinee of a `match` lives until the end of the whole `match`, so `queue.push(7)` in the `None` arm is a second mutable borrow. `flushed` is accepted: in edition 2024 a temporary in a tail expression is dropped before the block's locals, so the `Drain` is gone before `batch` is destroyed. Two fixes work. Write an `if let`: `if let Some(job) = queue.drain(..).next() { job } else { queue.push(7); 7 }`, because the temporary of an `if let` is dropped before the `else` block runs. Or end the temporary in a statement of its own: `let first = queue.drain(..).next();` and then `match first { ... }`. The compiler's own suggestion (store the whole `match` in a local) does not help here: the temporary would still be alive inside the arms.</details>

## Mastery check

Answer each question of the [mastery check](README.md#mastery-check) in NOTES.md first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**How semicolons and tail expressions change a block type.** A block's value is its last expression when that expression has no semicolon. A semicolon turns the expression into a statement and throws the value away, so the block's value becomes `()`. The same rule gives a function its return value, which is why a stray semicolon on the last line produces "expected `i32`, found `()`".

**Choosing match, if let and let else.** Use `match` when several cases need their own code; the compiler then checks that no case is missing. Use `if let` when one pattern matters and everything else shares one fallback (or needs nothing); its variables exist only inside its block. Use `let else` when a failed pattern means "leave now" (`return`, `break`, `continue`, panic) and the rest of the function needs the variables.

**When temporaries are dropped.** A temporary in the tail expression of a block is dropped at the end of the block, before the block's local variables. A temporary in the condition of an `if let` lives through the first block and is dropped before the `else` block. A temporary in the scrutinee of a `match` lives until the end of the whole `match`. A temporary in an ordinary `let` or expression statement is dropped at the end of that statement (unless the `let` stores a reference to it, which keeps it alive as long as the variable). The first two rules are those of edition 2024.

**return, labeled break, continue, loop values and the never type.** `continue` starts the next pass of the innermost loop, or of the labeled loop. `break` leaves the innermost loop or the labeled loop or block; in a `loop` or a labeled block it can carry a value, which becomes the value of that expression. `for` and `while` have type `()` because they can end without a `break`. `return` leaves the whole function. All of these have the never type `!`: they produce no value, so they are accepted where any type is expected, for example in one arm of a `match` whose other arms give an integer.

**const evaluation and the duty at an unsafe block.** A `const` item or an inline `const { }` block is computed by the compiler, so it may only use other constants and `const fn` calls, never a runtime local; its result can be used in a type, such as an array length. An `unsafe` block does not change what code does and does not disable the borrow checker. It permits a few operations the compiler cannot verify, such as reading through a raw pointer. The author must make sure the operation's conditions hold (for a pointer read: not null, aligned, pointing to a live initialized value, no conflict with existing references) and should state them in a `// SAFETY:` comment.

**Precedence, short-circuiting, cast truncation and closure capture.** Method calls bind tighter than unary minus; arithmetic binds tighter than shifts; `&`, `^`, `|` bind tighter than comparisons; `&&` binds tighter than `||`. `&&` and `||` evaluate the right side only when the left side does not decide the result. An integer `as` cast to a narrower type keeps the low bits; a float `as` integer cast drops the fraction, stops at the ends of the range and turns NaN into 0; no cast panics. A closure stores what it uses from its surroundings: a shared reference if it only reads, a mutable reference if it changes the variable, the value itself if it gives it away or if the closure is marked `move`.

**Redoing the capstone with a changed requirement.** There is no single answer. One example: add a `reset` command that sets the sum back to 0. It is a command, like `stop`, so it should be recognised before the number parser runs, and it must not change line numbering. The tradeoff is where to recognise it: inside `decode` (then `decode` needs a richer return type than `Option<i32>`) or inside `execute` (simpler types, but command knowledge is spread over two functions).

</details>
