# Chapter 8 hints — Modules, crates and Cargo

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Try the first hint before opening the second. The implementation constraints in each task are part of the exercise.

## 01 · Crates, editions and profiles

<details><summary>Hint 1</summary>

cfg! returns a compile-time Boolean. Package metadata is available through compile-time environment variables.

</details>
<details><summary>Hint 2</summary>

Use cfg!(debug_assertions) for the profile-sensitive task. Read env!("CARGO_PKG_NAME") and env!("CARGO_PKG_VERSION") rather than runtime environment variables.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A *package* is a directory with a `Cargo.toml`; it contains one or more *targets* (a library, binaries, examples, tests), and each target is compiled as its own *crate*. That is why this lesson's tests live in a crate named `chapter08` inside the package `own_it`, and the support library is the crate `practice`, named by `[lib]`. The installed toolchain (`rustc --version`, pinned here by `rust-toolchain.toml`) decides which compiler runs; `edition` selects language rules such as keywords and defaults; `rust-version` declares the oldest compiler the package promises to support. A *profile* (dev, release, test) chooses settings like optimization and `debug_assertions`, which `cfg!` can observe at compile time.

</details>

## 02 · Nested modules and private implementation

<details><summary>Hint 1</summary>

Items are private by default. Export the API while keeping the calculation helper private.

</details>
<details><summary>Hint 2</summary>

The public cost function can invoke a nested module function marked pub(super). Use widened u32 arithmetic.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Items are private to the module that declares them and visible to that module's descendants, so a child can use its ancestors' private items, but a parent cannot see into a child unless the child marks items `pub`, `pub(super)`, or `pub(crate)`. A module boundary should expose operations that keep its invariants true, such as "zero items are free", rather than raw pieces a caller could combine wrongly. Keeping `rules::fee` private means you can change the fee formula without breaking any caller. Moving a module into `tariff.rs` and `tariff/rules.rs` changes the files but not the paths, because `mod` declarations define the tree.

</details>

## 03 · Paths, imports and reexports

<details><summary>Hint 1</summary>

A facade can reexport a public type from a private implementation module.

</details>
<details><summary>Hint 2</summary>

Create Ticket through its inherent new constructor; name may be borrowed through an accessor while representation stays private.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`crate::` starts at the crate root, `self::` at the current module, and `super::` at the parent; a bare path is resolved from the current scope. `use` creates a new *name* for an existing item, not a new item: an alias and a `pub use` reexport both name the same type, which is why `type_name` still reports the defining module. Names in the standard prelude, such as `Option`, `Some`, and `drop`, are available everywhere, but a local item with the same name shadows them. A facade `pub use` lets callers write a short stable path while the implementation module stays private, and `pub struct` with private fields forces callers through constructors that enforce the invariants.

</details>

## 04 · Constants and statics

<details><summary>Hint 1</summary>

Constants are values evaluated at compile time; immutable statics have one storage location.

</details>
<details><summary>Hint 2</summary>

Compute a byte count from the constant. Borrow the static array directly instead of constructing an equal temporary.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A `const` is a value: the compiler pastes it into every place it is used, so different uses are not guaranteed to share an address. A `static` is one location in memory that lives for the whole program, so every borrow of it, from any function, points to the same bytes, and a static declared inside a function body is still shared by every call. Only `const fn`s can run in a const or static initializer. Because a static is shared by all threads, mutating one requires an atomic, a lock, or `unsafe` with a proof that no data race can occur; prefer the first two.

</details>

## 05 · Attributes, cfg and included files

<details><summary>Hint 1</summary>

Attributes can remove code before type checking. include_str! embeds UTF-8 content into the executable.

</details>
<details><summary>Hint 2</summary>

Use cfg! for the target family value. Split the included literal by lines and trim nonempty lines.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`#[cfg(...)]` removes an item or statement before type checking, so code behind a false predicate is never compiled at all. `cfg!(...)` is just a `bool` known at compile time; both branches of an `if cfg!(...)` must still compile, and the optimizer then drops the dead one. The two constant predicates are written `true` and `false`, as in `#[cfg(false)]`; older code spells them `all()` (always true) and `any()` (always false). Under Cargo, a predicate name or value that cannot exist, such as a misspelt `target_os` or a feature that `Cargo.toml` does not declare, produces the `unexpected_cfgs` warning instead of silently counting as false. `env!` and `include_str!` are evaluated when the crate is compiled, embedding the value or file contents in the binary; `include_str!` paths are relative to the source file. Build scripts can generate files into `OUT_DIR` and pass values to `env!` with `cargo::rustc-env`.

</details>

## 06 · Tests and executable documentation

<details><summary>Hint 1</summary>

Public APIs should make their normalization rules precise enough for examples and tests.

</details>
<details><summary>Hint 2</summary>

trim and split_whitespace cover leading/trailing and repeated Unicode whitespace. Test the public contract rather than internal steps.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Unit tests sit inside the crate, usually in a `#[cfg(test)]` module, so they can test private items. Integration tests in `tests/` are separate crates that can only use the public API, exactly like a real consumer, so they prove what the public contract is. Doc-tests are the examples in `///` comments of a library; `cargo test` compiles and runs them, so documentation that drifts from the code fails the build, and a `compile_fail` example can show that something is rejected. Assertions like `assert_eq!` report both values, which makes failures easier to diagnose than a bare `assert!`.

</details>

## 07 · Dependency requirements and lockfiles

<details><summary>Hint 1</summary>

These tasks model only the numeric stable-version caret intervals explicitly stated, not full Cargo semver parsing.

</details>
<details><summary>Hint 2</summary>

For a nonzero major caret interval, keep the same major and compare minor/patch lexicographically. For 0.minor with nonzero minor, keep both major and minor.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A requirement such as `"1.2.3"` is a *caret* range: any version that should be compatible according to SemVer, here `>=1.2.3, <2.0.0`. Before 1.0 the leftmost non-zero component acts as the major version, so `"0.2.3"` means `>=0.2.3, <0.3.0`. Cargo resolves each requirement to one concrete version and records the result in `Cargo.lock`, one lockfile per workspace, so every build uses the same versions until you run `cargo update`. Path dependencies point at a local directory, which is ideal for offline experiments and for crates developed together. Versions must be compared as numbers, not as strings, because "1.10.0" sorts before "1.9.9" as text.

</details>

## 08 · Capstone: library facade and workspace

<details><summary>Hint 1</summary>

Keep validation private, reexport the public function, and let command-line concerns call the library.

</details>
<details><summary>Hint 2</summary>

The facade accepts text, validates and formats it; a batch function propagates the first indexed failure.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The library exposes a small facade, `tag` (and a batch helper), while validation stays private, so its rules can change without breaking callers. A CLI binary would be a thin layer that reads arguments, calls the library, and maps errors to exit codes; keeping it out of the library makes the logic testable and reusable. An external integration test proves the privacy: it can call `tag`, but it cannot name `labels::validate`. Before publishing, `cargo package --list` shows exactly which files ship in the `.crate` archive; `Cargo.lock` is used for binaries and ignored by consumers of a library. Every `pub` item is part of the compatibility promise, so expose as little as possible.

</details>

## R01 · Spaced review of Chapters 4, 6 and 7

<details><summary>Hint 1</summary>

`?` calls `From::from` on the error, so once the `From` impl exists, `text.trim().parse::<u16>()?` produces a `ConfigError`. For B, find where the digits end. For C, ownership can move out of the vector without copying any text.

</details>
<details><summary>Hint 2</summary>

A2: `let port: u16 = text.trim().parse()?;` then reject zero. B: `let split = text.find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len());`, parse `&text[..split]` (empty fails), and `match &text[split..] { "" => Some(n), unit => { let (_, m) = UNITS.iter().find(|(u, _)| *u == unit)?; n.checked_mul(*m) } }`. C: `Vec::extract_if(.., |m| m.starts_with(prefix)).collect()`, or `mem::take` the vector and `partition` it.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one comparison lets the bin hold one item too many, and one counter can hand out the same id twice. A public field and a cloning getter make the module boundary useless. Clippy points at two more places that say something simple in a long way.

</details>
<details><summary>Hint 2</summary>

Make `items` private and return `&[String]` from `items()`; the tests call only methods. The capacity rule is written twice, with two different operators: let `add` ask `is_full()`. `is_full` itself is one expression, `self.items.len() >= CAPACITY`. `position` can take `&str` and become `self.items.iter().position(|item| item == name)`. Make the read and the write of the counter one atomic step: return `NEXT_ID.fetch_add(1, Ordering::Relaxed) + 1`.

</details>
<details><summary>Check your review</summary>

`Bin::add` checks `len() > CAPACITY`, so a fifth item fits. The same rule already exists, correctly, in `is_full`; a rule that is written twice will sooner or later be written two ways, so `add` should call `is_full`. Because `items` is `pub`, any caller can push past the limit anyway, and the type cannot promise its invariant. Make the field private; the module boundary is what makes the invariant enforceable. No test can check this part, because a test that touched a private field would not compile. Check it yourself as step 5 of the lesson header says.

`items()` clones the whole vector on every call where a borrowed `&[String]` is enough. `next_id` loads and then stores as two separate steps: two threads can load the same value and both return it (a lost update). Each operation is atomic, so this is not undefined behavior, but the sequence is not atomic. `fetch_add` makes read-modify-write a single step. A `static mut` counter would have been worse: a real data race, which is undefined behavior.

Clippy reports four warnings in the starter. `needless_return` (twice) and `needless_bool` are about `is_full`: an `if` that returns `true` or `false` is just its condition. `manual_find` is about the index loop in `position`. Clippy's own replacement there, a `find` over a range of indexes, compiles but still indexes by hand; `iter().position(..)` says what is meant. Treat a lint as a pointer to a smell, and its suggestion as one option. Taking `&str` instead of `&String` lets callers pass a literal without building a `String`.

</details>

## Drills

Run `cargo drill ch08`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · Private by default</summary>E0603: `fee` has no `pub`, so it is private to `billing` and its children; `main` is in the parent. Write `pub fn fee` (or `pub(crate)`). Privacy is checked by module, not by file.</details>
<details><summary>d02 · A pub struct with a private field</summary>E0451: a struct literal must set every field, and `owner` is private outside `tickets`. That is the point of private fields: callers must go through `Ticket::new`, which trims the owner. Use the constructor.</details>
<details><summary>d03 · A path that does not exist</summary>E0432: `Circle` lives in `shapes::round`, not `shapes`. Write `use shapes::round::Circle;`, or add `pub use round::Circle;` inside `shapes` to give it a shorter public path.</details>
<details><summary>d04 · A const needs a const fn</summary>E0015: a const initializer is evaluated by the compiler, which can only call `const fn`s. Mark `kib` as `const fn`; the multiplication is allowed in const context.</details>
<details><summary>d05 · A static must be shareable</summary>E0277: every thread can reach a `static`, so its type must be `Sync`, and `Cell` is not. For a per-thread counter, use `thread_local! { static HITS: Cell<u32> = const { Cell::new(0) }; }` (the `KEEP` line still works, because `LocalKey<Cell<T>>` has `get` and `set`). For one shared counter, use `AtomicU32`.</details>
<details><summary>d06 · cfg removes items</summary>E0425: the `fast` feature is not enabled, so `#[cfg(feature = "fast")]` removed `helper` before name resolution, and nothing called `helper` exists. Remove the attribute, enable the feature (in Cargo, `--features fast`), or provide a `#[cfg(not(feature = "fast"))]` version. In a Cargo package you would also get the warning ``unexpected `cfg` condition value: `fast` `` unless `Cargo.toml` declares a feature of that name; `cargo drill` compiles the file directly with `rustc`, which has no list of features to compare with.</details>
<details><summary>d07 · What cfg hides is not type-checked</summary>Compiles and runs; there is nothing to fix. Code behind `#[cfg(false)]` is removed before type checking, so the ill-typed `broken` is never compiled. The code must still *parse*, though. `if cfg!(false) { ... }` would be different: both branches are type-checked. Older code writes the same switch as `#[cfg(any())]`.</details>
<details><summary>d08 · Re-exporting a crate-private item</summary>E0364: `pub use` cannot make an item more visible than it is declared; `start` is only `pub(crate)`. Make it `pub fn start`, or keep it crate-private and write `pub(crate) use engine::start;`.</details>
<details><summary>d09 · Two globs, one name</summary>E0659: both globs bring a `parse` into scope, and neither wins, so the call is ambiguous. Import the one you mean explicitly (`use toml::parse;`), because an explicit import shadows glob imports, or call `toml::parse` with its path.</details>
<details><summary>d10 · An environment variable read at compile time</summary>Panics at run time: `option_env!` is evaluated while *compiling*, and the variable was not set then, so it produced `None` and `expect` panics when the program runs. Use `.unwrap_or("default")`. `env!` would instead fail the build, and `std::env::var` reads the variable at run time.</details>
<details><summary>d11 · Reaching past the constructor</summary>E0616: `items` has no `pub`, so only code inside `inventory` can touch it. Replace the line with `bin.add("nut");`, the one function that checks the capacity. If the field were public, the push would compile and nothing would stop a third and a fourth item. This is the proof that no test in C01 can give you: the private field turns a broken invariant into a compile error.</details>

## Lab · label_lab

Hints for [the checked lab](label_lab/README.md). Open one step at a time.

<details><summary>The three predictions before step 1</summary>

1. Three crates of your own: the binary `label_cli`, the binary `label_check`, and the test crate `cli`. (`cargo test` also compiles each binary a second time as a test harness; these are the two "Running unittests" lines with 0 tests.) The two binaries cannot share a private function, because they are separate crates. Shared code belongs in a library that both use.
2. `tests/public_api.rs` is a separate crate, so it names the library like any other user: `label_core::...`. The unit tests are a child module inside the library, so they reach their parent with `super::...`.
3. Only the unit tests. That is why the tests for `validate::clean` stand in `src/lib.rs`.

</details>
<details><summary>Step 1 · Make the library work</summary>

`str::trim` returns a sub-slice of its input, so `clean` needs no allocation. Count characters with `text.chars().count()`; `text.len()` counts bytes, and `é` has two. In `Label::new`, `validate::clean(raw)?` stops at an error and otherwise gives the cleaned `&str`, which you turn into the owned field. `tag` is `Label::new(raw)` followed by `Label::tag`; `?` or `Result::map` joins them.

</details>
<details><summary>Step 2 · Give the crate a shape</summary>

Cut everything *between* the braces of `mod validate { ... }` and paste it into `label_core/src/validate.rs`. The new file starts with the `use crate::{LabelError, MAX_LEN};` line and contains no `mod validate` line of its own. In `lib.rs`, what remains is `mod validate;`. Do the same for `model`. Paths inside the moved code do not change, because the module tree did not change.

Visibility: `mod validate;` and `mod model;` without `pub`. Inside `model`, the types and methods stay `pub`; the `pub use model::{Label, LabelError};` line in `lib.rs` decides that they are public, and where. For `clean`, `pub(crate)` is the narrowest form that the unit tests in `lib.rs` can still use (`pub(super)` means the same thing here, because the parent is the crate root). Remove `pub` from the field `text`.

Until the modules and the field are private, the three `compile_fail` examples report:

```text
---- label_core/src/lib.rs - (line 21) stdout ----
Test compiled successfully, but it's marked `compile_fail`.
```

</details>
<details><summary>Step 3 · Document by example</summary>

Inside the `///` comment of `tag`, add a line with three backticks, then the example lines, then three backticks again; every line starts with `/// `. A doc-test is a separate crate, so the example needs `use label_core::tag;` (or full paths). Two assertions are enough: one valid label and one rejected one. The model for the layout is the example at the top of `lib.rs`.

</details>
<details><summary>Step 4 · Two binaries</summary>

Collect the arguments first: `std::env::args().skip(1).collect::<Vec<String>>()`; the first element of `args()` is the program's own name. Return `ExitCode::SUCCESS`, `ExitCode::FAILURE` (status 1) or `ExitCode::from(2)` from `main`. Write diagnostics with `eprintln!`.

"Print nothing when any label is invalid" means: convert all labels into a `Vec<String>` first, and print only after the loop. `enumerate()` counts from 0, the contract counts from 1. For `label_check`, `Iterator::all` over the arguments is the whole program.

Before `default-run` is set, Cargo answers:

```text
error: `cargo run` could not determine which binary to run. Use the `--bin` option to specify a binary, or the `default-run` manifest key.
available binaries: label_check, label_cli
```

</details>
<details><summary>Step 5 · Share settings through the workspace</summary>

The root manifest gains three tables:

```toml
[workspace.package]
version = "0.3.0"
edition = "2024"
rust-version = "1.99"
license = "MIT"

[workspace.dependencies]
label_core = { path = "label_core", version = "0.3" }
label_slug = { path = "label_slug", version = "0.3" }

[workspace.lints.rust]
unsafe_code = "forbid"
```

Each member then says `version.workspace = true` (and the same for the other three keys) in `[package]`, and has a `[lints]` table with `workspace = true`. Paths in `[workspace.dependencies]` are relative to the root, so `"label_core"`, not `"../label_core"`. With the lint in force, an `unsafe` block is a hard error:

```text
error: usage of an `unsafe` block
  --> label_cli/src/bin/label_check.rs:14:17
   |
14 |     let first = unsafe { *[1u8].as_ptr() };
   |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: requested on the command line with `-F unsafe-code`
```

The note shows how the table works: Cargo turns `[lints]` into compiler flags (`-F` is "forbid") for every crate of the package.

</details>
<details><summary>Step 6 · A feature with an optional dependency</summary>

In `label_core/Cargo.toml`: `label_slug = { workspace = true, optional = true }` under `[dependencies]`, and `slug = ["dep:label_slug"]` under `[features]`. `optional` is written by the crate that uses the dependency; it cannot be inherited. In `label_cli/Cargo.toml`: `slug = ["label_core/slug"]`.

The prediction: no, `if cfg!(feature = "slug") { label_core::slug(raw) }` does not compile in a default build. `cfg!` is only a `bool`; the call is still compiled, and the function is not there:

```text
error[E0425]: cannot find function `slug` in crate `label_core`
  --> label_cli/src/main.rs:64:25
   |
64 |             label_core::slug(raw)
   |                         ^^^^ not found in `label_core`
   |
note: found an item that was configured out
  --> label_core/src/lib.rs:70:8
   |
69 | #[cfg(feature = "slug")]
   |       ---------------- the item is gated behind the `slug` feature
70 | pub fn slug(raw: &str) -> Result<String, LabelError> {
   |        ^^^^
```

Code that names an item which exists only with a feature must itself be removed with `#[cfg(feature = "slug")]`. A tidy way: write a small private function twice in `main.rs`, once under `#[cfg(feature = "slug")]` that calls `label_core::slug`, and once under `#[cfg(not(feature = "slug"))]`. The rest of `main` then needs no conditions, and `cfg!(feature = "slug")` is enough for the "built without the slug feature" message, because that branch names nothing that could be missing.

The second prediction: with `cargo test --features label_cli/slug`, the tests of `label_core` run *with* `slug`; you see `with_slug::...` names in the `public_api` output. In one Cargo command each package is built once, with the union of all features that were requested for it. `label_cli/slug` requests `label_core/slug`, so the one build of `label_core`, and its tests, have the feature. This is feature unification, and it is the reason a feature must only add things: some other crate in the build may have switched it on.

</details>
<details><summary>Step 7 · A build script</summary>

In `build.rs`, read the variable when the script runs, `std::env::var("OPT_LEVEL")`, and print `println!("cargo::rustc-env=LABEL_CORE_OPT_LEVEL={level}");`. In `lib.rs`, the body of `opt_level` is `env!("LABEL_CORE_OPT_LEVEL")`: read when the crate is compiled, and a `&'static str`.

The prediction: `env!` without the line in `build.rs` stops the build:

```text
error: environment variable `LABEL_CORE_OPT_LEVEL` not defined at compile time
  --> label_core/src/lib.rs:91:5
   |
91 |     env!("LABEL_CORE_OPT_LEVEL")
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: use `std::env::var("LABEL_CORE_OPT_LEVEL")` to read the variable at run time
```

Do not follow the help text here: at run time the variable does not exist either. Only the build script can provide it.

</details>
<details><summary>Step 8 · What you should have seen</summary>

The version requirement, with its hidden caret made visible:

```text
error: failed to select a version for the requirement `label_core = "^0.4"`
candidate versions found which didn't match: 0.3.0
```

The package list of `label_slug` in a Git checkout:

```text
.cargo_vcs_info.json
Cargo.lock
Cargo.toml
Cargo.toml.orig
src/lib.rs
```

`Cargo.toml` in the archive is a rewritten manifest in which every inherited key has its real value (`version = "0.3.0"`, and the lint as a `[lints.rust]` table); your own file is kept as `Cargo.toml.orig`. Without `--allow-dirty`, Cargo refuses to package files that differ from the last commit, and names them. The `target/`, `tests/` of other packages and the workspace root are not in the list: a package archive contains one package.

</details>

## Mastery check · model answers

Answer each question in [NOTES.md](NOTES.md) first. Then compare.

<details><summary>Model answers (open after writing your own)</summary>

**Package versus crate versus module; the crates of one package.** A package is a directory with a `Cargo.toml`: the unit that has a name and a version and that Cargo builds. A crate is what the compiler compiles in one run: a root file and all modules below it. A module is a named, nested namespace inside a crate with its own privacy boundary. A package with `src/lib.rs`, `src/main.rs` and `tests/api.rs` gives three crates: the library, the binary and the test crate. The last two use the library as a dependency. `cargo test` compiles the library and the binary once more each, with their unit tests enabled.

**Controlling visibility and proving it.** Items are private to their module and its descendants unless marked `pub`, `pub(crate)` or `pub(super)`. Keep the defining modules private and reexport only chosen items with `pub use`; keep fields private so that values can be created and changed only through functions that keep the type's rules. Only code outside the crate can prove that something is hidden: an integration test shows what *can* be used, and a `compile_fail` doc-test (or a drill) shows what *cannot*. A unit test proves nothing here, because it is inside the boundary.

**Splitting modules into files.** Replace the block `mod name { ... }` with the declaration `mod name;` and move the body to `name.rs` next to the declaring file (or `name/mod.rs`); a child of `name` goes to `name/child.rs`. The module tree is defined by the `mod` declarations, not by the files, so every path and every visibility stays the same and no caller changes.

**Const versus static; `#[cfg]` versus `cfg!`.** A `const` is a value that the compiler copies into each place of use; it has no fixed address, and "changing" it changes a temporary. A `static` is one place in memory for the whole run of the program, with a `'static` address; its type must be `Sync`, and it changes only through an atomic, a lock or `unsafe`. `#[cfg(p)]` removes the following item or statement before names and types are checked, so the code may refer to things that do not exist in this build. `cfg!(p)` is a `bool`; an `if` on it keeps both branches in the program for type checking, and the optimiser removes the dead one.

**Three kinds of tests and what each can see.** Unit tests are modules inside the crate, compiled only for `cargo test`; they see private items. Integration tests are files in `tests/`, each a separate crate that depends on the library; they see exactly the public API. Doc-tests are the code blocks in the library's documentation comments, also compiled outside the library; they see the public API and keep the documentation honest. The last two need a library target.

**Requirement, lockfile, one lockfile per workspace.** A requirement such as `"1.2.3"` is a range, here from 1.2.3 to below 2.0.0 (before 1.0 the range is narrower: `"0.2.3"` stops below 0.3.0). Cargo chooses one version per dependency that satisfies all requirements and records the choice in `Cargo.lock`, so every later build uses the same versions until `cargo update` or a changed requirement. A workspace is resolved as a whole, so that all members use the same version of each shared dependency and can be built together into one `target/` directory; that needs one lockfile, at the workspace root.

**A feature with an optional dependency; why features only add.** Mark the dependency `optional = true`, declare `name = ["dep:the_dependency"]` under `[features]`, and put `#[cfg(feature = "name")]` on the code that uses it. Users enable it with `--features name` or in their manifest. In one build Cargo compiles a package once, with the union of the features that all its users asked for. So a crate that did not ask for a feature can still get it. If a feature removed or changed something, that crate would break through no fault of its own; a feature may only add.

**Build scripts.** `build.rs` in the package root is a program that Cargo compiles and runs on the build machine before it compiles the package's crates. It can generate source files into `OUT_DIR`, build foreign code, and give instructions to Cargo by printing `cargo::` lines. `cargo::rustc-env=NAME=VALUE` makes `env!("NAME")` available in the crate; `cargo::rerun-if-changed=PATH` limits when the script runs again.

**Redoing the capstone.** There is no single answer. A good changed requirement forces a design decision, for example "report *all* invalid names, not only the first": the return type becomes `Result<Vec<String>, Vec<usize>>`, the early exit disappears, and you must decide whether valid tags are still built when some name is invalid. The tradeoff to name: early exit does the least work and gives the simplest type; collecting all errors serves a user who wants to repair every problem in one round.

</details>
