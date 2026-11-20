# Chapter 22 hints — Writing macros

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Try the first hint before opening the second. The implementation constraints in each task are part of the exercise. The sections follow the study order of the chapter guide, so lesson 08 comes before the capstone (lesson 07).

## l01 · Expansion and evaluation once

<details><summary>Hint 1</summary>

Bind the expression once inside the expansion rather than substituting it twice.

</details>

<details><summary>Hint 2</summary>

A block expression can contain let statements and return its final expression. Built-in macros expand where the exported expansion invokes them.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

A macro body is plain token substitution. If it says `$value` twice, the caller's expression is evaluated twice, side effects and all. Binding it once with `let value = $value;` inside a block makes the macro behave like a function call. That `value` is hygienic: it has the macro's syntax context, so it can neither see nor shadow a caller variable with the same spelling. Control flow is different. A `return` or `?` inside a macro argument is pasted into the caller's function, so it returns from the *caller*. A function argument is evaluated before the call and cannot do that. Built-ins such as `line!()` and `module_path!()` report the outermost invocation site.

</details>

## l02 · Repetition and separators

<details><summary>Hint 1</summary>

Repeat insertion statements for the map. Counting syntax should transform each fragment into unit, not execute it.

</details>

<details><summary>Hint 2</summary>

An array of repeated () values has a length; stringify! consumes tokens without evaluating their expressions.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`$( ... ),*` matches zero or more repetitions separated by commas, and `$(,)?` optionally accepts one trailing comma that adds nothing. The transcriber repeats a `$( ... )*` group once per match, and nested repetitions must be repeated at the same depth they were matched. An empty invocation expands to code with no inserts, so the map's key and value types come only from the caller's annotation. Without it, inference fails. Separators are tokens too: a token-tree counter sees `a, b, c` as five trees. `vec![elem; n]` evaluates `elem` once and clones it.

</details>

## l03 · Type and identifier fragments

<details><summary>Hint 1</summary>

Macro-generated items are normal Rust items after expansion; they still need valid implementations.

</details>

<details><summary>Hint 2</summary>

Use Self { value } and &self.value in the item expansion. The turbofish can contain the captured $target:ty.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`macro_rules!` only checks that the tokens fit each fragment: `ident` is one identifier, `ty` is any type syntax such as `Vec<u8>` or `std::num::NonZeroU8`, `expr` is one expression parsed as a single node, and `literal` also accepts `-5` and `true`. After expansion, the compiler checks the generated items like hand-written code: types, privacy, borrows, and trait bounds. An identifier supplied by the caller keeps the caller's context, so `let $name = 10;` is visible to the caller. A name the macro writes itself is not. Once captured, a fragment such as `$e:expr` is opaque when forwarded: another macro cannot match it as `ident` or against specific tokens like `null`. There is one exception: a forwarded `expr` capture that is a literal (also a negated one such as `-5`) still matches a `$l:literal` matcher. That is why `forward!(5)` says "literal" while `forward!(x)` says "expr". The `ident` fragment accepts more than names of variables: it also accepts keywords (`type`, `fn`) and raw identifiers, and `stringify!` keeps the `r#` prefix of a raw identifier.

</details>

## l04 · Recursive structured literals

<details><summary>Hint 1</summary>

Write a rule for each structural case, then delegate every nested value back to the macro.

</details>

<details><summary>Hint 2</summary>

A tt repetition handles bracketed/braced/grouped values. The final expr rule delegates leaf conversion to From rather than reimplementing type selection.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

The grammar is `null`, `[ tt, ... ]`, `{ "key": tt, ... }`, or any expression. Each container rule calls `data!` again on every element, so nesting is handled by recursion. Every element must be ONE token tree: a delimited group counts as one tree, but `-2` is two (`-` and `2`), so negative or compound values must be wrapped in parentheses. Rules are tried in order, and `null` also parses as an expression (a path), so the special cases must come before the catch-all `$other:expr`. Values are forwarded as `tt`, which stays transparent. A value forwarded as `expr` would no longer match `(null)`. This is a literal builder, not a JSON parser: there is no string escaping and no validation of keys.

</details>

## l05 · Traits handle macro leaf types

<details><summary>Hint 1</summary>

A declarative macro operates on syntax; ordinary trait resolution knows the types.

</details>

<details><summary>Hint 2</summary>

Use a repeated call to one generic helper. The bool Field implementation intentionally differs from Display to expose accidental formatting shortcuts.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

A macro sees only syntax. It cannot know whether `value` is an i32 or a &str, so it should not guess from spelling. Instead, every leaf becomes `render(value)`, and ordinary trait resolution picks the `Field` impl from the leaf's type. The macro handles the shape (repetition, commas, order) and the traits handle the behavior, so adding a type means adding an impl, not a macro rule. An unsupported leaf such as `Vec<u8>` fails with a trait-bound error (E0277) pointing at the expansion, which is clearer than a macro mismatch. Integer literals are inferred from the available impls: with only `Field for i32`, `12` becomes an i32.

</details>

## l06 · Exports and hygiene

<details><summary>Hint 1</summary>

The helper must be reachable through a crate-relative path after expansion in the caller.

</details>

<details><summary>Hint 2</summary>

Use `$crate::l06_exports::format_event`, because each chapter is its own integration-test crate and l06_exports is the defining module. `crate::l06_exports::format_event` passes every run-time test in this file, but it means "the crate of the caller": the export lab (`cargo test --manifest-path exercises/chapter22/export_lab/Cargo.toml`) then fails to compile.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

Names that the macro writes itself, such as local variables, are hygienic: they cannot capture or be captured by the caller's locals, and the caller's expression still sees the caller's `count`. Item paths such as function names are NOT hygienic in `macro_rules!`: a plain `format_event(...)` is looked up where the macro is *invoked*, so the caller's own helper, or a missing import, changes the result. `$crate` expands to the defining crate under whatever name the caller gave it, including a renamed dependency, so `$crate::l06_exports::format_event` always reaches the real helper. The path must also be public all the way down: a pub function inside a private module is still unreachable from another crate. `#[macro_export]` places the macro at the crate root, so it is invoked as `crate::ch22_event!` or `dependency::ch22_event!`.

</details>

## l08 · Patterns, items, and token munchers

<details><summary>Hint 1</summary>

A: one `match` on `$value` does everything; decide what the arm after the caller's arms must be. B: try the direct expansion `$( $(#[$attr])* $item )*` and read the error. The list of attributes and the list of items are two separate repetitions, and a transcriber cannot combine them freely. C: write by hand the constants that `{ A, B = 5, C }` must become. What does each line need to know from the line before it?

</details>

<details><summary>Hint 2</summary>

A: `match $value { $( $pattern $(if $guard)? => Some($result), )+ _ => None }`. B: capture the whole bracket as one token tree (`$attrs:tt`), and hand it with one item at a time to an internal rule `(@one [$(#[$attr:meta])*] $item:item)`. Put that rule first. C: keep the entry rule and replace its `$( pub const ... = 0; )*` line with `numbered!(@step $ty, 0; $($name $(= $value)?,)*);`. Then write three `@step` rules: nothing left; `$name:ident = $value:expr, $($rest:tt)*`; and `$name:ident, $($rest:tt)*`. Each of the last two writes one constant and calls `@step` again with `$name + 1` as the next value.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

A repetition handles every element in the same way and has no memory, so it cannot number elements or treat `NAME` and `NAME = 5` differently. A tt-muncher is a recursive macro that takes one element from the front of a token list, emits code for it, and calls itself with the rest. What the next step must know travels in the call as extra arguments: here the type and the expression for the next value. This state is an accumulator. Rules that the caller is not meant to use start with a marker such as `@step`. They stand before the entry rule because rules are tried from the top, and a fragment that starts to parse and then fails (an `item`, an `expr`) is a hard error, not a reason to try the next rule. A captured `item` (also `expr`, `ty`, `pat`, `vis`) is closed: you can place it in the output, with attributes in front, but no later rule can look inside it. So capture an item whole when you only pass it on, and match its parts (`$vis:vis struct $name:ident ...`) when the expansion needs a part. Each recursive call counts against the recursion limit (128 by default), so a muncher suits lists of some dozens of elements, not thousands.

</details>

## l07 · Capstone: structured events

<details><summary>Hint 1</summary>

Separate parsing from constructing the ordinary Event type. Identifier fragments become names via stringify!.

</details>

<details><summary>Hint 2</summary>

Bind kind before repeating fields. A BTreeMap provides deterministic comparison and last-insert-wins behavior.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

The grammar is `kind_expr ; (ident : expr),* ,?`. The kind is bound first, then each value is evaluated once, in source order, and inserted into a BTreeMap. Repeated names overwrite earlier ones, so the last value wins and the map's order is deterministic. Field names come from `stringify!($field)`. The `ident` fragment accepts any identifier, including keywords such as `type` and raw identifiers such as `r#type` (which becomes the field name "r#type"); it rejects literals, paths and `_`. So the macro checks the shape of a field name, not that it is a sensible name. Values use `Display` (`to_string`), so the API requires `Display` and a missing impl is an ordinary E0277. Parsing stays in the macro and construction in the plain function `make_event`, which takes owned inputs and moves them in without copying. If the macro were exported, it would call `$crate::...::make_event`. A fallback rule such as `($($t:tt)*) => { compile_error!("expected: kind; name: value, ...") }` turns a mismatch into a readable message.

</details>

## R01 · Spaced review of Chapters 15, 19, and 21

<details><summary>Hint 1</summary>

Each macro is a thin repetition around something you already know: polling a pinned future with a `Context`, spawning inside `std::thread::scope`, and `Iterator::chain`. Write the code for two arguments by hand first, then turn the repeated part into `$( ... )*`.

</details>
<details><summary>Hint 2</summary>

A: build `Context::from_waker(Waker::noop())` once, then `vec![$({ let mut f = pin!($fut); f.as_mut().poll(&mut cx) }),*]`. B: inside `thread::scope(|s| ...)`, make an array of `s.spawn(|| $task)` handles first, so that every thread is running, and only then join them in order. Joining each handle right after its spawn runs the tasks one after another, and the barrier test then reports that the macro is blocked. C: the base case is `IntoIterator::into_iter($only)`; the recursive case is `into_iter($first).chain(chain_all!($($rest),+))`.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs. In one, an argument appears twice in an expansion. In the other, a helper's name is resolved in the wrong place. Then look for every String that is built only to be copied or thrown away, and for an `unwrap()` inside a function that already returns a `Result`.

</details>
<details><summary>Hint 2</summary>

Bind both arguments once: `match ($a, $b) { (a, b) => if a >= b { a } else { b } }`. Call the helper as `$crate::c01_code_review::tag_name` and build the Vec in one step with `vec![...]`. `format!("#{tag}")` needs no `to_string`. In `fmt`, `write!(f, ...)` writes straight into the formatter and returns its `fmt::Result`.

</details>
<details><summary>Check your review</summary>

`max_of!` pastes `$a` and `$b` into both the comparison and the result, so the winning argument runs twice. Side effects repeat, and the returned value can differ from the one that was compared. `std::cmp::max` would not work as a fix: it returns the second argument on a tie and needs `Ord`, which f64 lacks. `ch22_tags!` calls `tag_name` by its bare name, so the call is resolved where the macro is invoked: any module with its own `tag_name` silently gets different output, and a module without one fails to compile. The repair is `$crate::c01_code_review::tag_name`. A path that starts with plain `crate::` would work inside this crate, but the macro is exported, and in another crate `crate::` means that other crate. It also grows a Vec from empty, clones each finished String, and calls `to_string()` inside `format!` (clippy: `to_string_in_format_args`). The Display impl formats into a temporary String, then `unwrap()`s the write, so a failing writer causes a panic instead of an `Err` returned to the caller.

</details>

## Drills

Run `cargo drill ch22`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · Define before use</summary>error (no error code): "cannot find macro". `macro_rules!` names have *textual* scope: a macro is visible only after its definition in the file, unlike functions. Move the definition above `main` (or export it with `#[macro_export]` and use a path).</details>
<details><summary>d02 · A macro cannot see the caller's locals</summary>E0425. The `tax` written in the macro body has the macro's syntax context and is looked up where the macro is *defined*, where no `tax` exists. One fix is to define the macro inside `main` after `let tax`, so `tax` is in scope at the definition. A cleaner API passes it in: `add_tax!(100, tax)`.</details>
<details><summary>d03 · Evaluating an argument twice</summary>Panics. `square!(next())` expands to `next() * next()`, which is 1 × 2 with `calls == 2`. Bind once: `{ let v = $x; v * v }`.</details>
<details><summary>d04 · Grouping of expression fragments</summary>Compiles and runs. An `expr` fragment is substituted as one parsed node, as if parenthesized, so `double!(1 + 2)` is `(1 + 2) * 2`. C preprocessor macros paste text and would give 5. Rust macros do not.</details>
<details><summary>d05 · A trailing comma</summary>error (no error code): "unexpected end of macro invocation". After the final comma the matcher expects another `expr`. Add `$(,)?` after the repetition to accept an optional trailing comma.</details>
<details><summary>d06 · Exported macros and helper paths</summary>E0425. `label` is resolved at the invocation site (`main`), where it is not in scope. Use `$crate::util::label($n)` and make `label` `pub`: the expansion runs in the caller's module, so the helper must be reachable from there.</details>
<details><summary>d07 · The expansion is still type-checked</summary>E0308. The macro produces `vec![1, "two", 3.5]`, and a Vec has one element type. Macros don't bypass type checking. Convert each leaf, for example `vec![$($x.to_string()),*]`, or use a trait as in l05.</details>
<details><summary>d08 · Locals declared by a macro</summary>E0425. The `x` written inside the macro is hygienic: the caller's `x` is a different name even though it is spelled the same. Let the caller supply the identifier: `($name:ident) => { let $name = 1; }` and `make_x!(x)`.</details>
<details><summary>d09 · Where #[macro_export] puts a macro</summary>E0433. `#[macro_export]` always places the macro at the crate root, whichever module defines it. Call `crate::area!(2, 3)`. A bare `area!(2, 3)` also works in `main`: the exported macro is an item of the crate root, and `main` is in the crate root, so the name is found by path lookup, even above the definition. It is not found through textual scope, because textual scope ends with `mod shapes` unless the module has `#[macro_use]`.</details>
<details><summary>d10 · Leaves need the trait the expansion uses</summary>E0277: `Vec<i32>` does not implement `Display`. The macro accepted the syntax, and the trait check failed after expansion. Use `{:?}` (both leaves implement Debug) or define a trait for supported leaves.</details>

## Mastery check: model answers

Answer the questions in the [chapter guide](README.md#mastery-check) from memory first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**1. A macro call and a function call.**
A function receives values: each argument is evaluated once, before the call, in the caller. A macro receives code. The compiler puts that code wherever the transcriber says `$name`, so an argument that is written twice in the transcriber runs twice, and one that is not written at all never runs. A well-behaved macro binds each argument to a local variable once and uses the variable afterwards. Because the argument's code ends up inside the caller's function, a `return`, `break` or `?` in it acts on the caller. An `expr` argument stays one expression, so there is no precedence problem as in a text-replacing macro.

**2. Fragments.**
`expr` for an expression, `ty` for a type, `ident` for a name (it also accepts keywords and raw identifiers), `tt` for one token tree, `pat` for a pattern (including `A | B`; `pat_param` excludes the top-level `|`), `item` for a whole item with its attributes and visibility, `vis` for a visibility that may be empty, and `literal` for a literal with an optional minus sign. Only `tt`, `ident` and `lifetime` captures stay open: another macro can match their tokens again. The others are closed after capture: they can be placed in the output or matched again as the same kind, but their tokens cannot be compared with specific tokens. One exception: a closed `expr` that is a literal still matches `literal`.

**3. Empty repetitions and trailing commas.**
`$( ... ),*` matches zero or more elements, so an empty call is accepted; `+` needs at least one. In an empty expansion there may be nothing that fixes a type, and the caller must then write it (`let m: BTreeMap<u8, u8> = ordered_map!();`). A separator stands only between elements, so `a, b,` is not matched by `$($x:expr),*`. Add `$(,)?` after the repetition to accept one optional trailing comma. It adds no element.

**4. Recursion and traits.**
The macro handles what the caller wrote: one rule per syntactic form, and a recursive call for every nested part, taken as `tt` so that the inner call can look at its tokens. Specific rules stand before the general one, because the first match wins. What a value *is* cannot be seen by a macro, so the rule for a leaf wraps the expression in a call of a trait method or a generic function (`Data::from(x)`, `render(x)`), and the compiler picks the implementation by type. A new type needs a new `impl`, not a new rule, and an unsupported type gives an ordinary `E0277` error.

**5. `$crate`.**
A function name in a transcriber is looked up where the macro is called. A bare name finds whatever the calling module has under that name, or nothing. A path that starts with `crate::` means the crate of the *caller*. `$crate` means the crate that defines the macro, under whatever name the caller's crate knows it, so `$crate::module::helper` reaches the same function from every call. Inside the defining crate, `crate` and `$crate` are the same crate, so every test there passes with either. The difference appears only when a second crate calls the macro; the export lab is such a crate. The helper and every module on its path must be `pub`.

**6. Hygiene.**
Hygiene covers local variables (and loop labels) and `$crate`. A local that the macro declares is invisible to the caller's code, and the caller's locals are invisible to code written in the macro body, even with the same spelling. A name that the caller passes in as an `ident` or inside a pattern stays the caller's, so `let $name = ...;` makes a variable that the caller can use. Hygiene does not cover items: functions, types, constants and modules named or defined in a transcriber are ordinary names, resolved at the place of the call. That is why a generated function can be called by the caller, and why helpers need `$crate`.

**7. Token munchers and internal rules.**
A repetition treats all elements alike and keeps no state. A token muncher is needed when elements have different shapes, when an element's meaning depends on earlier ones (numbering, "previous value plus one"), or when the output is in another order. Each step matches one element at the front and calls the macro again with the rest; what the next step must know travels in the call as an accumulator. Internal rules (marked with `@word`) hold these steps, and they also let a macro pass a group of tokens as one `tt` to a helper rule, for example to combine one list with each element of another. Put internal rules first: rules are tried from the top, and an entry rule that starts with a fragment such as `item` or `expr` can fail with a hard error on the marker instead of letting the next rule try. The costs are compile time, harder error messages, and the recursion limit (128 steps by default).

**8. A procedural macro.**
`macro_rules!` matches tokens by shape and cannot compute with them: it cannot read the fields of a struct that it received as an `item`, build new identifiers, or report an error at a chosen token. A procedural macro is a function that receives the token stream and returns a new one, so it can do all of that. It is the tool for `#[derive(...)]`, for attributes on items, and for syntax that needs a real parser. The price: it must live in a separate crate with `proc-macro = true`, because the compiler has to build it before it can run it; it usually brings parser crates as dependencies; it compiles more slowly; and it has no `$crate`, so naming the support crate needs care. Use `macro_rules!` while it is enough.

</details>
