# Chapter 22 labs: expansion, diagnostics, and procedural macros

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md)

Lab 4 is part of lesson 06 and is checked by tests. Labs 1 to 3 are **optional labs**: you build small projects by hand, and no test in this repository checks them. Do Labs 1 to 3 in a directory outside this repository, for example `~/scratch/ch22`, so that an experiment never breaks `cargo test --all-targets` here.

Labs 2 and 3 need three crates from crates.io, so they need network access once. They were built for this course on Rust 1.99.0 with syn 3.0.10, quote 1.0.47 and proc-macro2 1.0.107. Tested lessons on procedural macros, with supplied tests, are in the mastery track ([`mastery/m5_proc_macros`](../../mastery/m5_proc_macros/README.md)); the two labs here are a first contact.

## Lab 1 · Inspect expansions (optional)

Goal: see what your own macros expand to, and connect the expansion to the diagnostics that you get.

Minimal `Cargo.toml` (no dependencies):

```toml
[package]
name = "expand-lab"
version = "0.1.0"
edition = "2024"
```

1. Create a scratch crate (`cargo new --lib expand-lab`) and copy in one of your finished macros from this chapter, for example `ordered_map!` or `data!`.
   *Checkpoint:* `cargo test` in the scratch crate compiles your macro unchanged.
2. Choose how you will see expansions: the expansion view of your editor, or the `cargo expand` tool, installed by following its own instructions. If you have neither, write the expansion by hand and compile it beside the invocation.
   *Checkpoint:* for one invocation you can see (or have written) the code that it becomes.
3. Before you expand each case below, write your predicted expansion. Then compare it with the real one:
   - nested repetitions;
   - helper paths (`$crate::...` versus a bare name);
   - local bindings introduced by the macro;
   - `return` inside a macro argument;
   - `?` inside a macro argument;
   - one step of a tt-muncher from lesson 08.
   *Checkpoint:* you can see that `return` and `?` in an argument act on the *caller's* function, and that the macro's own locals keep their hygiene even though they print with ordinary names.
4. Run one small experiment with each built-in macro: `stringify!`, `concat!`, `env!` and `option_env!`, `include_str!` with a file that you create yourself, `cfg!`, and `dbg!`.
   *Checkpoint:* for each one you can say whether it does its work at compile time or at run time, and what happens when its input is missing (for example `env!` on a variable that is not set).
5. Break one invocation on purpose (a missing separator or a wrong fragment), record the diagnostic, then repair the macro or the invocation.
6. `trace_macros!` and `log_syntax!` are unstable and need a nightly toolchain. If you try them, do it in a separate project and write the toolchain in your notes.

Acceptance criteria:

- [ ] One invocation recorded with its predicted expansion.
- [ ] The actual expansion or diagnostic recorded next to the prediction.
- [ ] One corrected expansion that compiles.
- [ ] A written observation for each case in step 3 and each built-in in step 4.
- [ ] Any nightly-only experiment is labelled as such.

## Lab 2 · A derive macro in its own workspace (optional)

Goal: implement `#[derive(Summarize)]` and see why a procedural macro needs a crate of its own.

A procedural macro is a Rust function that the compiler runs while it compiles *another* crate. The function receives tokens and returns tokens. Because it runs inside the compiler, it must be compiled first, as a separate crate of the special kind `proc-macro`.

Minimal workspace:

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["summary-derive", "demo"]
resolver = "3"

# summary-derive/Cargo.toml
[package]
name = "summary-derive"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true

[dependencies]
syn = { version = "3", features = ["derive"] }  # add "full" only if you need it
quote = "1"
proc-macro2 = "1"

# demo/Cargo.toml
[package]
name = "demo"
version = "0.1.0"
edition = "2024"

[dependencies]
summary-derive = { path = "../summary-derive" }
```

Commit the `Cargo.lock` that Cargo writes, so that the lab keeps building with the versions you started with.

1. Create the workspace with the two crates above: `summary-derive` (`proc-macro = true`) and `demo`.
   *Checkpoint:* `cargo build` succeeds with a derive function that returns an empty `TokenStream`.
2. In `demo/src/lib.rs`, define the trait and re-export the derive, so that users need only one import:

   ```rust
   pub use summary_derive::Summarize;

   pub trait Summarize {
       fn summarize(&self) -> Vec<(&'static str, String)>;
   }
   ```

   A derive macro and a trait may share one name, because macros and traits live in different namespaces.
3. Implement `#[derive(Summarize)]` for structs with named fields. Generate one `(field_name, value)` pair per field, in declaration order, with each value formatted by `Display`. Put the checks in `demo/tests/derive.rs` (see step 6 for the reason).
   *Checkpoint:* `#[derive(Summarize)] struct Job { id: u32, name: String }` returns `[("id", "7"), ("name", "build")]` for `Job { id: 7, name: "build".into() }`.
4. Reject enums, unions and tuple structs with `syn::Error::new_spanned(...).to_compile_error()` instead of a panic.
   *Checkpoint:* a derive on an enum gives your own error message with the enum underlined, not "proc-macro derive panicked".
5. Keep generics and an existing `where` clause (`Generics::split_for_impl` gives the three pieces of the `impl` header; `Generics::make_where_clause` lets you add bounds). Add a `Display` bound for each field type.
   *Checkpoint:* a struct with a generic field `T`, a borrowed field `&'a str` and its own `where` clause derives cleanly.
6. Decide how the generated code names the trait, and write the decision down. The simple choice is the absolute path `::demo::Summarize`. It has two limits that you should see for yourself:
   - `::demo` resolves in other crates and in `demo/tests/`, but **not inside the `demo` crate itself**. A derive used in `demo/src/lib.rs` fails with ``error[E0433]: cannot find `demo` in the list of imported crates``. The repair is one line in `demo/src/lib.rs`: `extern crate self as demo;`.
   - A user who renames the dependency in their `Cargo.toml` breaks the path. A procedural macro has no `$crate`. A helper attribute that names the path (for example `#[summarize(crate = "renamed")]`) is the usual way out; implementing it is optional.

   *Checkpoint:* the derive works from `mod inner { ... }` in a test file, and you have seen the `E0433` error and its repair.
7. Record three compile failures and their diagnostics in your notes: a derive on an enum, a derive on a tuple struct, and a field whose type has no `Display` (this one is an ordinary `E0277` trait-bound error that the compiler reports for your generated code). Add each failing case to a test file for a moment, copy the message, and remove it again. The `trybuild` crate can turn such cases into permanent tests; this course did not check that tool, so treat it as further exploration.
8. Write up: compare this derive with the `macro_rules!` item generator in lesson 03 (`define_cell!`). What can the derive do that `define_cell!` cannot? What is the `TokenStream` boundary, and why does the derive need its own crate?

Acceptance criteria (each case has a compile or run check, or a recorded diagnostic):

- [ ] A struct with named fields, and one with zero fields.
- [ ] A generic field, a borrowed field with a lifetime, and an existing `where` clause.
- [ ] A field type without `Display` (recorded `E0277`).
- [ ] A rejected enum and a rejected tuple struct, with your own error message at the right place.
- [ ] The trait-path decision is written down, including what happens inside `demo` itself and with a renamed dependency.
- [ ] The written comparison with lesson 03.

## Lab 3 · An attribute macro (optional)

Goal: parse the arguments of an attribute, report errors at the right place, and change a function without losing anything that the caller wrote.

Add a third crate to the Lab 2 workspace (and to `members`):

```toml
# audit-attr/Cargo.toml
[package]
name = "audit-attr"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true

[dependencies]
syn = { version = "3", features = ["full"] }
quote = "1"
proc-macro2 = "1"
```

1. Add `#[audit_name("...")]` for functions. Parse the attribute's argument as a string literal (`syn::LitStr`) and the item as `syn::ItemFn`.
   *Checkpoint:* `#[audit_name("load")] fn load() {}` compiles, and the label `"load"` reaches your generated code.
2. Reject a non-string argument, a second argument, and a missing argument. Report each error at the most exact place you can.
   *Checkpoint:* `#[audit_name(7)]` reports "expected string literal" under `7`, and `#[audit_name("a", "b")]` reports its error under the comma. With no argument at all there is no token to point at, so the whole attribute is underlined.
3. Generate a body that passes the label to a callback that the caller's function receives as a parameter, instead of writing to a global logger.
4. Keep the function's visibility, generics, `async`, return type and other attributes (doc comments, `#[inline]`, `#[must_use]`). The safe way is to keep the parsed `ItemFn` whole, replace only its `block` field (for example with `syn::parse_quote!`), and emit the whole function again with `quote!(#function)`. If you take the struct apart field by field, a field that you forget is silently dropped.
   *Checkpoint:* `pub async fn fetch<T: Clone>(x: T) -> T` keeps its signature exactly.

Acceptance criteria:

- [ ] Two working cases.
- [ ] Three rejected cases, each with the error at the most exact place.
- [ ] No attribute on the original function is dropped.
- [ ] A written explanation of how hygiene differs between procedural and declarative macros.

## Lab 4 · A real caller in another crate (part of lesson 06)

Goal: prove that `ch22_event!` works from a crate that renames its dependency.

The workspace is supplied: [export_lab](export_lab/README.md) has a library and a consumer. It needs no crates from the registry. The library's only dev-dependency is this repository's own `practice` helper crate.

1. Finish lesson 06 (both `todo!()`s and its `p_predictions`).
2. From the repository root, run `cargo test --manifest-path exercises/chapter22/export_lab/Cargo.toml`.
   *Checkpoint:* the consumer test `exported_macro_resolves_public_definition_path` passes, although the consumer renames the library and defines its own `format_event`.

Two optional experiments, each followed by a restore:

3. Public reachability: remove `pub` from `pub mod l06_exports` in `export_lab/library/src/lib.rs` and record the privacy error that the consumer gets.
4. `$crate`: replace `$crate::l06_exports::format_event` in lesson 06 first with a bare `format_event`, then with `crate::l06_exports::format_event`. For each, record what the consumer does: one reaches the wrong helper, the other does not compile.

Acceptance criteria:

- [ ] The consumer test passes with the renamed dependency.
- [ ] (Optional) The privacy error from step 3 is recorded and explained.
- [ ] (Optional) Both results from step 4 are recorded and explained.

The solved lesson in [solutions/exercises/chapter22/](../../solutions/exercises/chapter22/) passes this lab unchanged, because the library includes `l06_exports.rs` by path.

## References

- [Macros by example](https://doc.rust-lang.org/reference/macros-by-example.html) and [procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html) in the Rust Reference.
- [The `proc_macro` crate](https://doc.rust-lang.org/proc_macro/index.html) in the standard library documentation.
- The API documentation of [syn](https://docs.rs/syn/3) and [quote](https://docs.rs/quote/1).
