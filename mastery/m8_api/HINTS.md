# M8 hints

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Notes](NOTES.md)

Each lesson has two hints. Open the first one only after a real attempt, and the second one only if the first was not enough. *Check your explanation* is a model answer to the lesson's EXPLAIN prompt; read it after you have written your own. From lesson 04 on, several designs pass the tests. The hints describe one of them.

## 01 · Version numbers as promises

<details><summary>Hint 1</summary>

For task A, find the left-most non-zero number of the *requirement*. Everything up to and including that number must be equal in the candidate, and the candidate must not be older than the requirement. `Version` derives `Ord`, and the derived order compares `major`, then `minor`, then `patch`, which is exactly "older" and "newer". For task B, a `match` on `(major, minor, bump)` with the `0.0.z` case first keeps the three regimes apart.

For the predictions, read `fixtures/l01/app.rs` line by line and ask of each line: does this change touch something that this line uses? If no line is touched, the program gives `ok`, whatever the class of the change is. For the class, think of *any* correct program, not of `app.rs`.

</details>
<details><summary>Hint 2</summary>

Task A in three cases: if `requirement.major != 0`, the major numbers must be equal. Otherwise, if `requirement.minor != 0`, the candidate needs `major == 0` and the same minor number. Otherwise the candidate must equal the requirement. In all cases return `false` first when `candidate < requirement`.

Which line of `app.rs` meets which change: the struct expression `Item { name, qty }` meets S02 and S10. The `match` in `advice` meets S03. The pattern `Event::Taken { name, left }` meets S04. The two calls of `take` meet S05. `names.join(" ")` meets S07 (a method of slices, not of iterators) and S08 (the order). Using `level` twice meets S09. The trait method `count` of the program meets S12: an inherent method with the same name is found first. Nothing in the program meets S01, S06, S11 or S13.

</details>
<details><summary>The classes (after your own attempt)</summary>

Major, by the list: S02 (a public field when no private field exists), S03 (a variant without `non_exhaustive`), S04 (a field in a variant), S05 (arity), S07 (the return type changes, so the function no longer "supports the original type"), S10 (`non_exhaustive` added later), S13 (a public item removed). Major, by the definition, because the list has no entry: S08 (documented behaviour changes) and S09 (a trait implementation that users rely on is removed). Minor: S01 (a new public item), S06 (a function generalized so that it can still be instantiated to the original type), S11 (a private field when one already exists). Possibly-breaking: S12 (a new inherent item).

A note on S06. The list calls it minor and adds that generalizing "can potentially create type inference failures". Here is one: a caller who passed `&shared` with `shared: Rc<[Item]>` relied on the compiler turning `&Rc<[Item]>` into `&[Item]` for a parameter of exactly that type. A generic parameter gives the compiler no such target, and the call fails with `E0277`. The repair is a few characters (`&shared[..]`), which is why the list accepts the change. A second caller that breaks is one who stored the function in a variable of the type `fn(&[Item]) -> u32`: the generic function has to be instantiated for one lifetime, the pointer type promises every lifetime, and the compiler answers `E0308`. So "minor" on the list means "very unlikely to break idiomatic code", not "cannot break".

</details>
<details><summary>Check your explanation (after writing your own)</summary>

S13 removes a method that `app.rs` never calls, so the program still builds, and the change is major: some other correct program calls `is_empty`, and its author did nothing wrong. S12 adds an inherent method, and `app.rs` stops compiling because its own trait method of the same name is no longer the one that `shelf.count()` finds; the change is still only "possibly-breaking". The class is a rule about the kind of change, made so that authors and users can agree in advance. "Major" means: this kind of change can break code that used the library as intended, and the user has no reasonable defence. "Possibly-breaking" means: it can break code, the repair is small and local (name the trait: `Report::count(&shelf)`), and forbidding the change would stop libraries from growing at all. So "it broke somebody" is neither necessary nor sufficient for "major". The evidence from one program can prove that a change breaks; it can never prove that a change is safe.

</details>

## 02 · Traits, bounds and auto traits

<details><summary>Hint 1</summary>

For each witness, ask: which *capability* does the change take away, and from whom? T1 takes "my `impl` block is complete" from implementors. T2 takes "the name `flush` on a transport means my method" from a user who gave the library's type a method of that name through a trait of their own. T4 takes "any transport is accepted" from a user whose transport is not `Send`. T6 and T8 take `Send` from a value. T7 takes `dyn Transport`. T3 takes nothing from implementors, because there are none outside the library; think about what is left. Then write the shortest program that uses exactly that capability, and make sure it has an assertion that passes with 1.0.0.

For task C, the type must allow a change through `&self` that is safe from several threads, and clones must share it.

</details>
<details><summary>Hint 2</summary>

W1: a struct of your own with `impl Transport` that has only `send`. W2: `trait Drain { fn flush(&mut self) -> usize; }` implemented for `courier::Memory`, with both traits in scope and a call `memory.flush()`; the error is "multiple applicable items". W3: a transport that holds an `Rc<RefCell<Vec<u8>>>`. W4: `std::thread::spawn(move || worker.record())` with a clone of the `Stats`. W5: `let t: Box<dyn Transport> = Box::new(Memory::default());`. W6: move the iterator from `courier::words(..)` into `std::thread::spawn` and count there; the text must be a string literal, so that the iterator is `'static`.

Task C: `hits: Arc<AtomicU64>`, `fetch_add(1, Ordering::Relaxed)` and `load(Ordering::Relaxed)`. `Relaxed` is enough because the number guards no other data.

</details>
<details><summary>The classes (after your own attempt)</summary>

T1 major: a required item breaks every implementation outside the library. T2 possibly-breaking: implementations stay valid, and a caller can get a clash of names (your witness 2). T3 possibly-breaking: the entry of the reference for a required method says "major" and names sealing as the remedy; with the seal there is no implementor outside to break, and what is left is the callers' risk of T2, a clash with a method `name` that a user gave `Plain` or `Line` through a trait of their own. T4 major: tightening a bound. T5 minor: loosening a bound; every type that was accepted is still accepted. T7 major: the trait stops being dyn compatible.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

T6 changes the type of a private field from `Arc<AtomicUsize>` to `Rc<Cell<usize>>`. `Send` and `Sync` are auto traits: the compiler implements them for a struct exactly when every field has them, and it looks at private fields too. `Rc` has neither, so `Stats` loses both, and `thread::spawn`, which requires `Send` of everything the closure owns, rejects the old program. T8 changes only a function body. The return type `impl Iterator` hides the concrete type from the user's code, but not from the compiler's auto-trait check: the answer to "is the returned value `Send`?" comes from the hidden type. The new hidden type is a `Box<dyn Iterator>` without `+ Send`, so the answer changes from yes to no. In both cases nothing in a signature changed, which is why a review of the diff does not find it. The defence is a test in the library itself that fails to compile when the promise is broken: a function `fn assert_send<T: Send>(_: &T) {}` (or its type-only form) applied to `Stats` and to the value that `words` returns. Then the release cannot be built by accident.

</details>

## 03 · Room to change

<details><summary>Hint 1</summary>

Go through the cases in `fixtures/l03/cases/` one by one; each names its task. A: the field of a tuple struct is private unless it says `pub`, and `std::num::NonZeroU32` is the type whose zero the compiler knows to be impossible. (A struct with one named private field is a newtype too; the cases accept both.) B: remove `pub` from the fields. C: one attribute on the enum, and the same attribute on the variant. D: a supertrait that lives in a private module; the module itself must not be `pub`, or users can name the supertrait and implement it. E: a struct that wraps the iterator of the slice and implements `Iterator` by forwarding `next`.

</details>
<details><summary>Hint 2</summary>

A: `pub struct SensorId(NonZeroU32);`, `NonZeroU32::new(raw).map(SensorId)`, `self.0.get()`. C: `#[non_exhaustive] pub enum Fault { Timeout, #[non_exhaustive] OutOfRange { limit: f64 } }`; inside the library your own `match` still needs no wildcard. D: `mod sealed { pub trait Sealed {} impl Sealed for super::Celsius {} impl Sealed for super::Fahrenheit {} }` and `pub trait Unit: sealed::Sealed { .. }`. E: `pub struct Readings<'a> { inner: std::slice::Iter<'a, Reading> }` with `type Item = &'a Reading;`, plus `#[derive(Debug, Clone)]`; forward `size_hint` as well.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Private fields take away the struct expression, direct field access and full patterns; the library gains the right to add, remove and retype fields, and a constructor that can check rules. `#[non_exhaustive]` on an enum takes away the `match` without a wildcard, and with it the compiler's help when the user forgets a variant; on a variant or struct it takes away construction and full patterns; the library gains new variants and fields in minor releases. A sealed trait takes away the user's own implementations, including test doubles; the library gains new required items. A newtype takes away every method and trait of the wrapped type that the library does not pass on; the library gains the right to replace the wrapped type. The price is too high when the type is complete by nature and users profit from full knowledge: `Ordering`, a point with `x` and `y`, the variants of `Option`. It is also too high when the trait exists precisely so that users can implement it, like `Transport` in lesson 02 or `Iterator` itself.

</details>

## 04 · An error type for a library

<details><summary>Hint 1</summary>

Start from the three readers in CONCEPTS: the caller needs `kind()` and `line()`, the person needs `Display`, the reporter needs `source()`. A struct `ParseError` with private fields can hold all of it: the kind, the line number, the name for the message, and an `Option<ParseIntError>`. `ErrorKind` is a plain enum with `#[non_exhaustive]` and derives. For `parse`, `str::lines().enumerate()` gives the line numbers (add 1), and `split_once(char::is_whitespace)` separates the name from the rest.

</details>
<details><summary>Hint 2</summary>

`impl std::error::Error for ParseError` with `fn source(&self) -> Option<&(dyn Error + 'static)>` that returns the stored `ParseIntError` as `&(dyn Error + 'static)`: `self.source.as_ref().map(|e| e as _)`. In `Display`, `match self.kind` and `write!` the three sentences; do not print the source there. In `parse`: skip a line when `line.trim()` is empty or starts with `#`; `let Some((name, limit)) = line.split_once(..) else { MissingLimit }`; then `limit.trim().parse::<u32>().map_err(..)` for `InvalidLimit`, keeping the error as the source; then look for the name among the quotas collected so far for `Duplicate`. A line such as `uploads   ` has nothing after the name once it is trimmed, so `split_once` finds no white space: `MissingLimit`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A public enum with one variant per failure makes every variant and every field inside it a promise. To carry more context later (a column, a file name), each variant would need a new field, which is a major change unless every variant was `#[non_exhaustive]` from the start; and a variant that holds `ParseIntError` makes the choice of number parser part of the interface. The struct keeps all of that private. Callers still branch, on `kind()`, and the kind enum is small, `Copy`, and `#[non_exhaustive]`. So in a minor release the struct can gain a `column()` method and a private field, the library can replace `str::parse` with a parser of its own (as long as `source()` still returns some error for `InvalidLimit`, callers who only print the chain notice nothing), and a new kind can be added. With the enum, each of those is visible in a public type. The enum is the better choice only when the failures are few and final, and hold nothing but plain data.

</details>

## 05 · Features that only add

<details><summary>Hint 1</summary>

There are three separate mistakes in `swatch`. The dependency `tinyjson` is optional and no feature mentions it, so Cargo makes a feature with the dependency's name. `no-names` removes something. `wide` changes the type of an existing item. And one `compile_error!` forbids a combination. Repair the manifest first (`cargo metadata` and the first test tell you when it is right), then the code. Build `fixtures/l05/apps` yourself with `cargo test --workspace` and read the first error.

</details>
<details><summary>Hint 2</summary>

Manifest: `default = ["names"]`, `names = []`, `json = ["dep:tinyjson"]`, `wide = []`. Code: remove the alias `Channel` and give `Color` plain `u8` fields in every configuration. Put `#[cfg(feature = "names")]` on `named` and `#[cfg(feature = "json")]` on `Color::to_json`. Add `#[cfg(feature = "wide")] pub struct Color16 { pub r: u16, pub g: u16, pub b: u16 }` with the same derives, an `impl` block with `new` and `hex` (`{:04x}` three times), and `impl From<Color> for Color16` that multiplies each channel by 257 after `u16::from`. Inside the `Color16` block, a `to_json` with `#[cfg(feature = "json")]` exists only when both features are on; `fixtures/l05/both` is the user that needs it. Delete the `compile_error!`. The version: removing a feature is a major change, and for a library at 0.3.0 the incompatible number is the second one, so the release is 0.4.0.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The implicit feature `tinyjson` breaks a user on the day the author replaces that crate: `features = ["tinyjson"]` in the user's manifest names a feature that no longer exists, and Cargo refuses to resolve. `no-names` breaks a program that calls `Color::named` when a second crate in the same build turns the feature on: the function is gone for both. `wide` breaks a program that stores `color.r` in a `u8`, or that relies on a colour being three bytes, when another crate turns the feature on: the field is suddenly a `u16`. The `compile_error!` breaks a program whose dependencies ask for `wide` and for JSON between them. The author never sees any of this, because `cargo test` in the library's own directory builds one configuration, the default one, with one user, the library's own tests. Each mistake needs a second crate in the build, with other wishes. The tests that find them are a matrix over the feature combinations and a workspace build of several users together.

</details>

## 06 · Documentation as interface

<details><summary>Hint 1</summary>

Work in the order of the contract. `cargo check` with `RUSTFLAGS="-D missing-docs"` lists what has no comment; remember the variants and the field `max`. Then write the examples. For each function, read its body in `imp.rs` and write down the edges: for `slug`, several separators in a row and separators at the ends; for `truncate`, a cut that lands just behind a dash, and `max == 0`; for `Tag::parse`, each of the three errors. An example needs one assertion per edge. For `?` in an example, end it with a hidden line that gives it a return type.

</details>
<details><summary>Hint 2</summary>

The five mutants and an assertion that catches each: `assert_eq!(slug("  Rust --  2024  "), "rust-2024")` catches both `slug` mutants (a run in the middle, separators at the end). `assert_eq!(truncate("hello-world", 6), "hello")` catches the dash mutant. A separate block that starts with ```` ```should_panic ```` and calls `slugger::truncate("hello-world", 0);` catches the zero mutant. `assert_eq!(Tag::parse("Rust"), Err(TagError::InvalidChar('R')))` catches the upper-case mutant. End the examples that use `?` with `# Ok::<(), slugger::TagError>(())`. The example program is `for title in std::env::args().skip(1) { println!("{}", slugger::slug(&title)); }`, with a branch for no arguments.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

In the model solution, one assertion with a run of separators in the middle and at both ends catches the two `slug` mutants; the cut behind the dash catches `truncate_keeps_dash`; the `should_panic` block catches `truncate_accepts_zero`; the refused upper-case letter catches `tag_allows_upper`. A typical first draft has `assert_eq!(slug("Hello World"), "hello-world")`, and that passes against both wrong versions of `slug`: it shows the function and specifies almost nothing. An example that shows says "this is how you call it". An example that specifies says "and this is what must be true", and it chooses its inputs at the places where an implementation could plausibly go wrong: the empty input, the boundary, the repeated character, the refused value. Those are also the places a user asks about. So good examples and good tests are found by the same question: for each sentence of the documentation, which assertion fails if the sentence is false?

</details>

## 07 · Deprecation, minimum Rust version, editions

<details><summary>Hint 1</summary>

Task A: write the new function with the old body, and turn the old function into one line that calls the new one. The attribute is `#[deprecated(since = "...", note = "...")]`. The test `a_the_library_itself_has_no_warnings` fails as long as `total_elapsed_ms` calls the old method. Task B: run `cargo clippy -- -D clippy::incompatible_msrv` in `fixtures/l07/oldtimer/`. It names four items and the version in which each became stable. Each has an older way to say the same.

</details>
<details><summary>Hint 2</summary>

Task A: `#[deprecated(since = "1.4.0", note = "use `format_duration` instead")] pub fn fmt_secs(secs: u64) -> String { format_duration(secs) }`, and the same pattern for the method with the note ``use `elapsed_ms` instead``. Task B: `LazyLock` (1.80) becomes a function with a `static UNITS: OnceLock<..>` inside and `get_or_init`. `Result::inspect_err` (1.76) becomes `if total.is_err() { .. }` before returning `total`. `Option::is_none_or` (1.82) becomes a `match` on the option, or `map_or(true, ..)`. `is_sorted` (1.82) becomes `laps.windows(2).all(|pair| pair[0] <= pair[1])`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The lint knows the version in which each item of the standard library became stable, and compares it with `rust-version`. It knows nothing else. Code can pass it and still fail on Rust 1.74 in at least three ways. It can use newer *syntax* or language rules: a `c"..."` literal (Rust 1.77), an inline `const { .. }` block (1.79), `&raw const` (1.82), a newer edition in the manifest. It can depend on a crate that needs a newer compiler. And a manifest key can be newer than the old Cargo. To promise a `rust-version` honestly, a project builds and tests with exactly that compiler in CI on every change (`cargo +1.74 test`), including with the oldest versions of its dependencies that its manifest allows, and it states its policy for raising the number. The lint is still useful: it is fast and it runs on every machine without a second toolchain, so it catches the common case before CI does.

</details>

## C01 · An API review

<details><summary>Hint 1</summary>

Use the six steps of the CONCEPTS section. Names first: which names would a user of `Vec` or `HashSet` guess, and which of the names here differ? Then each signature: what must the caller own or build before the call, and what does the `bool` at the call site tell a reader? Then traits: try to write `println!("{set:?}")`, `set.clone()`, `set == other`, `for tag in &set`, `"a, b".parse::<TagSet>()`. For the two bugs, look at what `add_tag` compares before it stores, and at what `as_string` writes after the last tag.

</details>
<details><summary>Hint 2</summary>

The reviewed interface, as the cases use it: `TagSet::new()`, `Default`, `Debug`, `Clone`, `PartialEq`, `Eq`; `len`, `is_empty`; `insert(&mut self, tag: &str) -> Result<bool, TagError>`; `contains(&self, tag: &str) -> bool`; `iter(&self)` yielding `&str`, and `IntoIterator for &TagSet`; `Display`; `into_vec(self)`; `union(&self, other: &TagSet) -> TagSet`; `FromStr`; a private `tags` field. `TagError` is a small `#[non_exhaustive]` enum with `Display` and `Error`. For `insert`: reject the empty tag and any white space, lower the case *first*, then `binary_search`; `Err(position)` is where to `Vec::insert`, which keeps the vector sorted without sorting.

</details>
<details><summary>Check your review (after writing your own)</summary>

Findings, by guideline. **C-STRUCT-PRIVATE**: `pub tags` lets any user break "sorted, lower case, no duplicates", and freezes the representation. **C-CTOR**: `create` should be `new`, with `Default`. **C-COMMON-TRAITS** and **C-DEBUG**: no `Debug`, `Clone`, `PartialEq`, `Eq`, `Default`, `Display`. **C-GETTER**: `get_len` should be `len`, and `is_empty` is then expected (clippy: `len_without_is_empty`). **C-GENERIC** / **C-CALLER-CONTROL**: `&String` parameters force the caller to own a `String`; `&str` accepts both (clippy: `ptr_arg`). **C-CUSTOM-TYPE**: `add_tag(&tag, true)` does not say what `true` means, and validation that can be switched off is no validation (**C-VALIDATE**). **C-GOOD-ERR**: `add_tag` returns `false` for "refused" and for "already there", and `load` reports failures as `String`s; a refusal should be an error type. **C-NO-OUT**: `load` writes its results into `errors`; it should return them, and reading from text is `FromStr`. **C-CONV**: `to_vec(self)` consumes, so `into_vec`; `as_string` allocates, so it is not an `as_`; text for people is `Display`. **C-ITER**: `tags_iter` should be `iter`, and `&TagSet` should be iterable; the return type `std::slice::Iter<'_, String>` exposes the representation (**C-NEWTYPE-HIDE**). **C-METHOD**: `merge(a, b)` has a clear receiver; make it `a.union(&b)`. Also `has_tag` is `contains` everywhere else in Rust.

The two bugs. `add_tag` checks `self.tags.contains(tag)` with the spelling that was passed in, and then stores the lower-case form; so `rust` followed by `Rust` is stored twice. `as_string` writes the separator after every tag, also after the last. Smaller points: the vector is sorted again after every insertion, and `as_string` builds a new `String` in every step.

</details>

## 08 · Capstone: a release

<details><summary>Hint 1</summary>

Read `user_of_1_0/src/lib.rs` first and list what it does: a struct expression and a full pattern for `Config`, a `match` on `Error` without a wildcard, an `impl Clock` with one method, function pointers with the exact signatures, a trait of its own with a method `total_ms` on `Config`. Each line forbids one "obvious" change. Then take the requests one at a time, and run `cargo test --package user_of_1_0` in `fixtures/l08/users/` after each. `user_of_next/src/lib.rs` shows every new item in use.

</details>
<details><summary>Hint 2</summary>

`Policy { retries, base_ms, factor, cap_ms: Option<u64> }` with private fields and `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`. `with_factor` and `with_cap_ms` take `self` and return a changed copy (`Policy { factor, ..self }`). `delays` returns a struct `Delays { next, remaining, factor, cap_ms }` whose `next` gives `min(next, cap)` and then sets `next = next.saturating_mul(factor)`. `total_ms` is a fold with `u64::saturating_add`. `impl TryFrom<Config> for Policy` calls `Policy::new`. `PolicyError { kind }` plus `#[non_exhaustive] enum PolicyErrorKind`. `total_ms(config)` gets the old body, and `get_total_ms` becomes a deprecated one-line wrapper; change the call inside `deadline` too. `elapsed_since` gets a body: `self.now_ms().saturating_sub(start_ms)`. Do not add `total_ms` as a method of `Config`: the first user has a trait method of that name. The version is 1.1.0.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Request 1 could not be fulfilled by adding `factor` and `cap_ms` to `Config`: in 1.0.0 all its fields are public and the type is not `#[non_exhaustive]`, so users build it with struct expressions and take it apart with full patterns. 1.0.0 should have given it private fields, a constructor and getters; then the two settings would be two new methods. Request 2 could not add variants to `Error`: it is an exhaustive public enum, and the user's `match` has no wildcard. 1.0.0 should have marked it `#[non_exhaustive]`, or used a struct with a kind. Request 3 could not rename: a public name is permanent within a major version; the cost of the wrong name is a deprecation that lasts until 2.0.0. A review against the guidelines before 1.0.0 would have found it. Request 4 could not add a required method to `Clock`: users implement the trait. A provided method works here, with the small risk of a name clash; had the method needed data from the implementor, only a new trait or a major release would do. Request 5 is the record keeping that makes the other four safe to receive. The pattern in all of them: the freedom that 1.0.0 gave to users is exactly the freedom that 1.1.0 no longer has.

</details>

## Drills

<details><summary>01 · A trait gets a new method</summary>

`E0046`. `Plugin` is implemented by users. A method without a body makes every existing `impl` block incomplete. The compatible change is a provided method: give `priority` the body `{ 5 }`. Implementations that want another priority override it.

</details>
<details><summary>02 · A struct gets a public field</summary>

`E0063`. A struct expression must name every field, so a new public field breaks every user who builds the struct that way; that is why the library moved to 2.0.0. The user's repair that also survives the next field is functional update syntax: `Size { width: 3, height: 4, ..Default::default() }`. It works because all fields are public and the type has a `Default`. The library could have prevented the problem in 1.0.0 with a private field and a constructor, or with `#[non_exhaustive]`.

</details>
<details><summary>03 · Using a sealed trait from outside</summary>

`ok`. Sealing forbids one thing: implementing the trait outside its home. Naming it, using it as a bound, calling its methods and holding it as `dyn Unit` all work, because none of them needs to name the private supertrait.

</details>
<details><summary>04 · A method asks for more than it uses</summary>

`E0277`. `get` only hands out a reference; nothing in its body clones. The bound `V: Clone` still applies to every caller, and `Connection` is not `Clone`. Tightening a bound is a major change, so do not add bounds "for later". The repair is to remove the `where` clause.

</details>
<details><summary>05 · A private field changes its type</summary>

`E0277`. `Send` is an auto trait: `Counter` has it only when every field has it, and `Rc<Cell<u64>>` does not. `thread::spawn` needs `Send` for what the closure owns. The "internal clean-up" removed a trait implementation from a public type. Go back to `Arc<AtomicU64>` with `fetch_add` and `load`, and add a compile-time check that `Counter: Send + Sync` to the library's tests.

</details>
<details><summary>06 · A provided method meets a user's method</summary>

`E0034`. After the release, two traits in scope offer `describe` for `Square`, and method syntax cannot choose. The official list calls a new provided method "possibly-breaking" for this reason. The repair is on the user's side and is one line: name the trait, `Describe::describe(&square)`.

</details>
<details><summary>07 · A convenient method for a trait that is used as `dyn`</summary>

`E0038`. A method with a type parameter cannot be called through a vtable, so with it the trait is no longer dyn compatible, and the user's `Box<dyn Sink>` is rejected. `where Self: Sized` on the new method leaves it out of the trait object: callers with a concrete type keep the method, and `dyn Sink` works as before.

</details>
<details><summary>08 · A deprecation in a strict project</summary>

`error` (no code). `#![deny(deprecated)]` turns the warning into an error; a lint that is denied has no error code. The repair is the one the note asks for: call `word_count`. The drill shows why a deprecation, although a minor change, is not free for everyone, and why denying warnings belongs in CI and not in the source of a published crate.

</details>
<details><summary>09 · A new inherent method with a familiar name</summary>

`panic`. Method lookup tries inherent methods before trait methods. So `stock.count()` now calls the library's new `count` (3 boxes) and no longer the user's trait method (36 pieces). Both return `u32`, so nothing fails to compile: the assertion fails at run time. This is the silent form of "possibly-breaking". The repair names the trait: `Pieces::count(&stock)`.

</details>
<details><summary>10 · What `impl Trait` borrows in edition 2024</summary>

`E0597`. In edition 2024, a return type `impl Trait` may use every generic parameter in scope, including the lifetime of `text`. The caller must therefore assume that the iterator borrows from `text`, although the body copies the lengths into a vector first. `+ use<>` says that the hidden type uses no generic parameter, and the caller may drop `text`. The captured parameters are part of the signature: decide them on purpose for every public function that returns `impl Trait`.

</details>

## Mastery check: model answers

<details><summary>1 · Requirements and next versions</summary>

`dep = "0.4.1"` allows every version from 0.4.1 up to, and not including, 0.5.0: before 1.0.0 the minor number is the incompatible one. After a bug fix: 0.4.2. After a new function: also 0.4.2, because 0.5.0 would tell Cargo that the release is incompatible, and no user would receive it automatically. After a removed function: 0.5.0.

</details>
<details><summary>2 · Sorting a diff</summary>

Go through the diff item by item and ask who can be on the other side. Removed or renamed items, changed signatures, new fields where users can build the struct, new variants where users can match exhaustively, new required trait items where users can implement the trait, tightened bounds, and lost trait implementations (including auto traits) are major. New items, loosened bounds, and new private fields next to existing private fields are minor. New inherent methods and new provided trait methods are possibly-breaking. For each major change, the breaking program uses exactly the capability that went away: the struct expression, the `match` without a wildcard, the `impl` block, the call with the old argument list, the `thread::spawn`.

</details>
<details><summary>3 · A private field that breaks a user</summary>

Auto traits (`Send`, `Sync`, `Unpin`, `UnwindSafe`) are implemented for a struct from its fields, private ones included. Replacing an `Arc` field by an `Rc` removes `Send` and `Sync` from the public type, and a user who moves the value to another thread stops compiling. The size of the type and whether it has a destructor follow from private fields too. The defence is a compile-time assertion in the library: `const _: fn() = || { fn check<T: Send + Sync>() {} check::<MyType>(); };` for every public type that should have the traits, and the same for the values behind `impl Trait` return types.

</details>
<details><summary>4 · The four tools</summary>

Private fields: the user cannot build or take apart the struct; you can change its fields. `#[non_exhaustive]`: the user must write a wildcard arm, and cannot build or fully match the struct or variant; you can add variants and fields. A sealed trait: the user cannot implement it; you can add required items. A newtype: the user cannot see or use the type inside; you can replace it. A type for none of them: something complete by its nature whose fields are its meaning, such as a 2D point `{ x, y }` or an RGB colour, or an enum such as `Ordering`.

</details>
<details><summary>5 · An error type for a parser</summary>

A struct with private fields: a kind, the position, and an optional source. `kind()` returns a small `Copy` enum that is `#[non_exhaustive]`, so callers branch with a wildcard arm and a new failure is a minor change. `Display` gives one short lower-case sentence about this level only. `Error::source` returns the wrapped error, so a reporter that walks the chain prints each cause once. The type is `Send + Sync + 'static` (checked by a test), so it converts into `Box<dyn Error + Send + Sync>` with `?`.

</details>
<details><summary>6 · Feature unification</summary>

Two programs in one workspace depend on the same library; one asks for feature `f`. A build of both compiles the library once, with `f` on, for both. So the program that did not ask for `f` sees the library with `f`. If `f` changes the type of an existing item (say a field from `u8` to `u16`), that program's code, written against the type without `f`, no longer compiles, although neither its source nor its manifest changed. Therefore a feature may only add items; then every program sees at least what it asked for.

</details>
<details><summary>7 · Documenting a fallible function</summary>

A first sentence that says what the function does in the caller's words. A paragraph for what the signature does not show: units, edge cases. A section `# Errors` that lists each error and its condition. An example that calls the function as a user would, uses `?`, ends with a hidden `# Ok::<(), ErrorType>(())`, and asserts the result at the edge the text mentions, for example the boundary value that is still accepted and the first value that is refused. If the comparison in the code were `<` instead of `<=`, that example would fail.

</details>
<details><summary>8 · Retiring a name</summary>

Release n (minor): add the new function; turn the old one into a wrapper marked `#[deprecated(since = "n", note = "use `new_name` instead")]`; move the library's own calls; say so in the changelog. Keep both through the whole major version. Release of the next major version: remove the old name. The first release can still break users whose build turns warnings into errors (`#![deny(warnings)]` in their source, or `-D warnings` in their CI), and it can clash with a name of the user's if the new function is a method.

</details>
<details><summary>9 · `rust-version`</summary>

It promises that the package builds with that version of Rust and every later one; Cargo refuses older compilers with a clear message, and the resolver (version 3, the default from edition 2024 on) prefers releases whose `rust-version` is not newer than the one the user's own package states, or than the user's compiler when it states none. Check one: clippy's `incompatible_msrv` lint, which compares the items of the standard library that you use with the field. Check two: building and testing with exactly that toolchain in CI. Only the second sees new syntax, new language rules, manifest keys, and the requirements of dependencies.

</details>
<details><summary>10 · A 2.0.0 of `pacer`</summary>

One reasonable plan: remove `get_total_ms` (deprecated since 1.1.0); remove `Config`, `Error`, the free functions `delays`, `total_ms` and `deadline`, after deprecating them in a late 1.x release in favour of `Policy`; give `deadline` a home as a method of `Policy`; keep `Clock` and decide whether to seal it. The changelog has a section `## 2.0.0` with `### Removed` (each item and what to use instead), `### Changed`, and a short migration note with before-and-after code, because a major release hands work to every user.

</details>
