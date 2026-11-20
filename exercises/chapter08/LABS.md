# Chapter 8 optional labs

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md)

These labs are **optional** and are not checked by any test. Each one is a short experiment in which you break something on purpose, read what the compiler or Cargo says, and repair it. Write what you observe in [NOTES.md](NOTES.md).

The checked lab of this chapter is a different thing: the workspace in [`label_lab/`](label_lab/README.md). Do that one in any case.

Labs 2 to 4 use [the scratch example](../../examples/ch08_scratch.rs): `cargo run --example ch08_scratch`. Leave the file in a compiling state when you finish. Lab 1 needs a package of its own.

## Lab 1 · Inspect a package from the outside

Goal: find every term of lesson 01 in the output of real tools. About 20 minutes.

Work in a temporary directory **outside** the course repository, for example a new folder in your operating system's temporary directory. Outside the repository the pinned toolchain does not apply; your default toolchain is used.

1. Run `cargo new --lib inspect-me` and change into the new directory.
2. Run `cargo metadata --no-deps --format-version 1`. The output is one long line of JSON. Find the package name, the `edition`, and the list of `targets`.
   *Checkpoint:* the package is called `inspect-me`, and its library target is called `inspect_me`.
3. Add a file `src/main.rs` with an empty `fn main() {}` and a file `tests/smoke.rs` with one `#[test]` function. Run step 2 again.
   *Checkpoint:* three targets, of the kinds `lib`, `bin` and `test`.
4. Run `rustc --version`, `cargo --version` and `rustup show active-toolchain`. Then add `rust-version = "1.120"` to `[package]` and run `cargo check`.
   *Checkpoint:* Cargo refuses, with a message of this form:

   ```text
   error: rustc 1.99.0 is not supported by the following package:
     inspect-me@0.1.0 requires rustc 1.120
   ```

   Set `rust-version` to the version you really have and check again.
5. Run `cargo check`, `cargo build`, `cargo test` and `cargo build --release`. After each one, look into `target/`.
   *Checkpoint:* you can say which commands write to `target/debug` and which to `target/release`, and which command produces no program at all.

Acceptance criteria:

- [ ] You can point to the package, each target, the edition and the `rust-version` in the metadata output.
- [ ] You can explain in one sentence each: installed toolchain, edition, `rust-version`.
- [ ] You deleted the temporary package.

## Lab 2 · Privacy experiments in the scratch file

Goal: see each privacy rule of lessons 02 and 03 as a compiler error. About 20 minutes.

1. In the scratch file, write a module `tariff` with a private child module `rules` that has one function, and a public function in `tariff` that calls it.
2. Call the function of `rules` from `main`.
   *Checkpoint:* error E0603. Decide which is the right repair: `pub`, `pub(crate)`, or no change to the module and a different call.
3. Add a struct `Ticket` with a private field and a constructor to `tariff`. In `main`, build one with a struct literal.
   *Checkpoint:* error E0451. Repair the caller, not the struct.
4. Change the field to `pub(super)`, then to `pub(crate)`. For each, predict whether `main` can read the field, then try.
5. Write `std::option::Option::Some(1)` in full, then shorten it step by step to `Some(1)`. Then define your own `fn drop<T>(_: T) -> &'static str` in the file and call `drop(5)`.
   *Checkpoint:* you can name the list that made the short forms work, and say why your `drop` won.

Acceptance criteria:

- [ ] Each error code above is recorded in your notes with the line that caused it.
- [ ] For one of them you wrote down a wrong repair that would also compile, and why it is worse.
- [ ] The scratch file compiles again.

## Lab 3 · Const and static in the scratch file

Goal: observe the difference between a value and a place. About 15 minutes.

1. Define `const LIMITS: [u32; 3]` and `static TABLE: [u32; 3]` with the same contents. Print `&TABLE as *const _` twice, from two different functions.
   *Checkpoint:* the same address both times.
2. Do the same with `&LIMITS`. Whatever you see, write down why you must not rely on it.
3. Write a function `fn double(n: u32) -> u32` and use it in the initialiser of a const.
   *Checkpoint:* error E0015. Repair it with `const fn`.
4. Declare `static mut COUNTER: u32 = 0;` and try to add 1 to it in `main` without `unsafe`. Read the error and its note about threads. Then replace it with an `AtomicU32` and `fetch_add`.

Acceptance criteria:

- [ ] You can state the difference between `const` and `static` in terms of addresses.
- [ ] You can say why the atomic version needs no `unsafe`.
- [ ] The scratch file compiles again.

## Lab 4 · cfg in the scratch file

Goal: see what conditional compilation removes, and when. About 15 minutes.

1. Write a function with a body that has a type error, for example one that returns text where a number is declared. Put `#[cfg(false)]` on it.
   *Checkpoint:* the program compiles. Now break the *syntax* inside the function (delete a closing brace).
   *Checkpoint:* it no longer compiles: removed code must still be valid syntax.
2. Write `if cfg!(false) { let n: u8 = "text"; }` in `main`.
   *Checkpoint:* a type error, although the branch can never run.
3. Print `cfg!(debug_assertions)` and run the example twice: `cargo run --example ch08_scratch` and `cargo run --release --example ch08_scratch`.
4. Print `option_env!("CH08_LAB_MODE")`. Run once normally and once with the variable set for the build (`CH08_LAB_MODE=on cargo run --example ch08_scratch` in a Unix shell; in PowerShell set `$env:CH08_LAB_MODE = "on"` first).
   *Checkpoint:* the value was fixed when the example was *compiled*. Run the compiled program directly from `target/debug/examples/` without the variable and see that it still prints the value.

Acceptance criteria:

- [ ] You can explain the difference between `#[cfg(false)]` and `if cfg!(false)` with your own two experiments.
- [ ] You can say at which moment `option_env!` reads the variable.
- [ ] The scratch file compiles again.
