# Chapter 8 lab · A workspace you finish by hand

[Chapter guide](../README.md) · [Concepts](../CONCEPTS.md#lab--a-workspace-with-a-feature) · [Hints](../HINTS.md#lab--label_lab)

The numbered lessons of this chapter run inside one test crate. That is enough to practise module rules, but not the things that only exist between crates: a library with a public surface, binaries that use it, integration tests, doc-tests, a Cargo feature, an optional dependency, a build script and shared workspace settings. This lab is a small workspace in which you do all of that, and every step has a check.

The lab is a separate workspace with its own `Cargo.lock`. It uses only the standard library and needs no network. Nothing here is ever published.

Estimated time: 2 to 3 hours.

## What you are given

```text
label_lab/
├── Cargo.toml              workspace root (no package of its own)
├── label_core/             library: validated labels            <- most of your work
│   ├── build.rs            build script                         <- step 7
│   ├── src/lib.rs          everything in one file, too public   <- steps 1, 2, 3, 6, 7
│   └── tests/public_api.rs integration test (do not edit)
├── label_slug/             small finished library; becomes an optional dependency
├── label_cli/              two binaries that use label_core     <- step 4
│   ├── src/main.rs         binary `label_cli`
│   ├── src/bin/label_check.rs  binary `label_check`
│   └── tests/cli.rs        runs both binaries as processes (do not edit)
└── lab_check/              checks files and manifests (do not edit)
```

Three kinds of tests check your work:

- **Behaviour**: unit tests in `label_core/src/lib.rs`, the integration tests in `tests/`, and the doc-tests in the `//!` and `///` comments.
- **Public surface**: three `compile_fail` doc-tests at the top of `label_core/src/lib.rs`. Each shows code that a user of the library must *not* be able to write.
- **Shape**: `lab_check` reads your source files and manifests. Where code lives and what a manifest says cannot be seen from behaviour, so these tests look directly.

Test names start with the step they belong to, for example `step4_cli_prints_one_tag_per_label`.

## Running the checks

Work from this directory:

```sh
cd exercises/chapter08/label_lab
cargo test                      # everything, default features
cargo test step1                # only tests whose name contains "step1"
cargo test -p label_core        # one package: unit, integration and doc-tests
cargo test -p label_core --doc  # only the doc-tests
cargo test -p lab_check         # only the shape checks
cargo test --all-features       # everything again, with every feature on
```

`cargo test` stops after the first test binary that fails. Add `--no-fail-fast` to see every failure in one run.

In the starter, the workspace compiles and almost every test fails: at `todo!()`, at a `compile_fail` example that still compiles, or at a shape check. That is the expected starting point.

Before you start, **predict** (write your answers in [NOTES.md](../NOTES.md)):

1. How many crates does `cargo test` compile for the package `label_cli`? Count its two binaries, its test file, and think about whether `src/main.rs` and `src/bin/label_check.rs` can share a private function.
2. `label_core/tests/public_api.rs` contains `use label_core::...`. The unit tests in `src/lib.rs` contain `use super::...`. Why the difference?
3. Which of the two can call a private function of the library?

## Step 1 · Make the library work

In `label_core/src/lib.rs`, implement the four tasks marked `1A` to `1D` from their doc comments. Then delete the `#![allow(unused_variables, unused_imports)]` line near the top; the compiler should have nothing to warn about.

Check: `cargo test -p label_core step1` passes (unit and integration tests), and the first doc-test, the one at the top of `lib.rs`, passes in `cargo test -p label_core --doc`.

## Step 2 · Give the crate a shape

The library works, but it is one file and far too open. Change three things. The public paths `label_core::Label`, `label_core::LabelError`, `label_core::tag` and `label_core::MAX_LEN` must keep working: `tests/public_api.rs` uses them and you may not edit it.

1. Move the body of `mod validate` to a new file `label_core/src/validate.rs`, and the body of `mod model` to `label_core/src/model.rs`. In `lib.rs` only the declarations `mod validate;` and `mod model;` remain.
2. Make both modules private. Other crates reach the two types only through the `pub use` line in `lib.rs`. The unit tests at the bottom of `lib.rs` must still be able to call `validate::clean`; choose the narrowest visibility that allows it.
3. Make the field of `Label` private.

Check: `cargo test -p label_core --doc` now passes all three `compile_fail` examples, `cargo test -p lab_check step2` passes, and step 1's tests still pass.

Then run `cargo doc -p label_core --no-deps --open` (or without `--open`, and read the path it prints). Look at what a user of the crate sees: `validate` and `model` are gone from the page, and `Label` and `LabelError` appear at the crate root.

## Step 3 · Document by example

Add an example to the doc comment of `pub fn tag`: a fenced code block with at least one assertion. Write it the way a caller would use the crate, starting with `use label_core::...`, because every doc-test is compiled as a separate crate.

Check: `cargo test -p label_core --doc` lists one more test, named `label_core/src/lib.rs - tag (line N)`, and `cargo test -p lab_check step3` passes.

Then break the example on purpose (change the expected value), run the doc-tests, read the failure, and repair it. This is the proof that documentation examples are really executed.

## Step 4 · Two binaries

Implement `label_cli/src/main.rs` and `label_cli/src/bin/label_check.rs` from the contracts at the top of each file. In `main.rs`, follow the part of the contract that is marked step 4; the part marked step 6 comes later.

Also give the package a default binary: add `default-run = "label_cli"` to `[package]` in `label_cli/Cargo.toml`. Before you do, run `cargo run -p label_cli -- "Bay 7"` once and read the error.

Check: `cargo test -p label_cli` and `cargo test -p lab_check step4` pass. Try the programs yourself:

```sh
cargo run -q -p label_cli -- " North Dock " "Bay 7"
cargo run -q -p label_cli --bin label_check -- "Bay 7" "" ; echo "exit status: $?"
```

(In PowerShell, print the status with `$LASTEXITCODE` instead of `$?`.)

## Step 5 · Share settings through the workspace

All three packages repeat `version`, `edition`, `rust-version` and `license`. Move them to one place.

1. In the root `Cargo.toml`, add a `[workspace.package]` table with those four keys (`0.3.0`, `2024`, `1.99`, `MIT`). In `label_core`, `label_slug` and `label_cli`, replace each of the four lines with `key.workspace = true`, for example `version.workspace = true`. Leave `lab_check` alone.
2. In the root `Cargo.toml`, add `[workspace.dependencies]` with `label_core = { path = "label_core", version = "0.3" }` and the same for `label_slug`. In `label_cli`, depend on the library with `label_core.workspace = true`.
3. In the root `Cargo.toml`, add `[workspace.lints.rust]` with `unsafe_code = "forbid"`. In each of the three packages, add a `[lints]` table with `workspace = true`.

Check: `cargo test -p lab_check step5` passes and everything else still passes. Then add `let first = unsafe { *[1u8].as_ptr() };` as the first line of `main` in `label_check.rs`, run `cargo check`, read the error, and remove the line.

## Step 6 · A feature with an optional dependency

`label_core::slug` should exist only for users who ask for it, and only they should pay for compiling `label_slug`.

1. In `label_core/Cargo.toml`, add `label_slug` under `[dependencies]` as an *optional* dependency taken from the workspace, and change the feature to `slug = ["dep:label_slug"]`.
2. Implement tasks `6A` and `6B` in `label_core/src/lib.rs`.
3. In `label_cli/Cargo.toml`, forward the feature: `slug = ["label_core/slug"]`. Then implement the `--slug` part of the contract in `main.rs`.

**Predict before part 3**: `label_core::slug` does not exist in a default build. Will `if cfg!(feature = "slug") { label_core::slug(raw) } else { .. }` compile in a default build? Try it, read the error, then solve it with `#[cfg(...)]`.

Check, both ways:

```sh
cargo test                                  # default: the `with_slug` tests are not even compiled
cargo test -p label_core --features slug    # the library with the feature
cargo test -p label_cli --features slug     # the binary forwards the feature
cargo test --all-features                   # everything, every feature
cargo test -p lab_check step6
cargo tree -p label_cli                     # label_slug is absent ...
cargo tree -p label_cli --features slug     # ... and now present
```

**Predict, then run**: `cargo test --features label_cli/slug` asks for the feature of the *binary* package only. Do the tests of `label_core` run with or without `slug`? Look for `with_slug::` test names in the `public_api` output. Compare your explanation with the hints.

## Step 7 · A build script

Cargo tells a build script things that the crate itself cannot see. One of them is the optimisation level, in the environment variable `OPT_LEVEL`.

1. In `label_core/build.rs`, read `OPT_LEVEL` and print a line `cargo::rustc-env=LABEL_CORE_OPT_LEVEL=<value>`.
2. Implement `opt_level()` in `label_core/src/lib.rs` (the task marked `Step 7`) with `env!`.

Check: `cargo test step7` and `cargo test --release step7` pass.

**Predict**: what happens if you do part 2 without part 1? Try it and read the error.

## Step 8 · Observe (optional, nothing to hand in)

These experiments change files on purpose. Undo each one afterwards.

1. **A version requirement is checked even for a path dependency.** In the root `Cargo.toml`, change the requirement for `label_core` to `version = "0.4"` and run `cargo check`. Read the error, then restore `"0.3"`.
2. **One lockfile.** Open `Cargo.lock` and find the four packages. There is one lockfile for the whole workspace, at its root. Compare it with the version from before step 6 (`git diff Cargo.lock`): the optional dependency is recorded even when the feature is off.
3. **What would be uploaded.** Run `cargo package --list -p label_slug --allow-dirty`. Then run `cargo package -p label_slug --allow-dirty` and open the `Cargo.toml` inside `target/package/label_slug-0.3.0/`. Find the keys you inherited in step 5. Do **not** run `cargo publish`; every package here has `publish = false`, so Cargo would refuse anyway.
4. **The whole toolbox.** Run `cargo fmt --all --check`, `cargo clippy --all-targets --all-features` and `cargo doc --no-deps`. A finished lab produces no warnings.

## Finished when

- [ ] `cargo test` passes.
- [ ] `cargo test --all-features` passes.
- [ ] `cargo test --release --all-features` passes.
- [ ] `cargo clippy --all-targets --all-features` prints no warnings.
- [ ] You can explain each of the three `compile_fail` examples: which rule makes it fail?

A model solution is in `solutions/exercises/chapter08/label_lab/` in the course repository. Compare with it after an honest attempt, not before.
