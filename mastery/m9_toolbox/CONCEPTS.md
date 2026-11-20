# M9 concepts: the professional toolbox

[Module guide](README.md) · [Labs](LABS.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Read one section, then do its lesson. The sections teach; the lessons make you do it.

## What you will be able to do

Knowing the language is half of working in Rust. The other half is the set of tools that every Rust project uses: Cargo beyond `cargo build`, the editor's language server, lints and formatting as team rules, test tooling, fuzzers and sanitizers, a debugger, rustdoc, supply-chain checks, and a CI pipeline that ties them together.

After this module you can:

- set up and repair a Cargo workspace, design features, and read what Cargo resolved;
- pin a toolchain and give an editor what it needs;
- write a lint and formatting policy that CI enforces without surprise breakage;
- organise tests, use snapshots, and judge a test suite by more than coverage;
- fuzz a parser, minimise a crash, and run sanitizers;
- go from a panic message to a root cause with backtraces and a debugger;
- document a crate the way crates.io and docs.rs expect;
- check dependencies for advisories, licences and duplicates, and keep an MSRV promise;
- write a CI workflow and prepare a release.

## Before you start

Chapters 1 to 22 of the course, and the [ecosystem track](../../ecosystem/README.md), above all its [tooling workday](../../ecosystem/TOOLING.md). That workday introduces `cargo doc`, `cargo tree`, `[lints]`, profiles and flamegraphs. This module builds on it and does not repeat it.

## Time

Reading and lesson together:

| Lesson | Estimate |
| --- | --- |
| 01 Cargo in depth | 3 h |
| 02 Toolchains, editor, generated code | 1.5 h |
| 03 Lints and formatting | 2 h |
| 04 Testing at scale | 3 h, plus 1.5 h of labs |
| 05 Fuzzing and sanitizers | 2.5 h, plus 1.5 h of labs |
| 06 Debugging | 2 h, plus 1 h of lab |
| 07 Documentation | 2 h |
| 08 Supply chain and policy | 2.5 h, plus 1 h of labs |
| 09 CI and release | 3 h |
| 10 Capstone | 5 to 8 h |

## Related mastery modules

Miri is taught in [M2](../m2_unsafe/README.md), loom in [M3](../m3_concurrency/README.md), trybuild in [M5](../m5_proc_macros/README.md), build scripts for C libraries in [M6](../m6_ffi/README.md), criterion and profiling in [M7](../m7_perf/README.md), and semantic versioning of your own API in [M8](../m8_api/README.md). [M10](../m10_contributing/README.md) applies these tools to someone else's codebase.

## Versions

Everything here was run with Rust 1.99.0 (Cargo 1.99.0, rustup 1.29.3) on macOS on Apple silicon; nightly commands used the course's dated nightly, `nightly-2026-11-20` (`rustc 1.102.0-nightly (6905905bb 2026-11-19)`). External tools change faster than the compiler. The README lists the exact version of each tool that the outputs below come from. The ideas are stable; flags may have moved by the time you read this, so check `--help`.

---

## Lesson 01 · Cargo in depth

**Assumed:** chapter 8 (crates, modules, a first workspace) and tooling-workday task 2 (`cargo tree`).

### The mental model

Cargo does three separate jobs, and most confusion comes from mixing them up.

1. **Resolve.** Read every manifest in the workspace, pick one version of each dependency, and write the result to `Cargo.lock`. The lock file covers every feature and every platform, also those you never build.
2. **Select features and units.** For the packages and the target you asked for, decide which features of each dependency are on. This is where *feature unification* happens.
3. **Build.** Run build scripts, then call `rustc` once per crate, with the settings of a *profile*.

A **workspace** is a set of packages that share one lock file and one `target/` directory. Its root manifest can hold values that members inherit:

```toml
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.package]
edition = "2024"
license = "MIT"

[workspace.dependencies]
serde = { version = "1.0.200", features = ["derive"] }

[workspace.lints.rust]
unsafe_code = "forbid"
```

A member opts in key by key: `edition.workspace = true`, `serde.workspace = true`, and `[lints] workspace = true`. Nothing is inherited unless the member asks.

### Features are additive, and they are unified

A feature is a named switch in `[features]`. It can turn on other features, optional dependencies (`dep:name`), and features of dependencies (`name/feature`). When two packages in one build ask for different features of the same crate, Cargo compiles that crate **once**, with the union of the requests. That is why a feature must only *add* things. A feature that removes an item or changes behaviour breaks whichever crate did not ask for it.

Look at what this package's build really enables:

```sh
cargo tree --manifest-path mastery/m9_toolbox/Cargo.toml -i serde_core
```

```text
serde_core v1.0.229
├── serde v1.0.229
│   └── serde_norway v0.9.42
│       └── m9_toolbox v0.0.0 (/path/to/mastery/m9_toolbox)
├── serde_json v1.0.154
│   └── m9_toolbox v0.0.0 (/path/to/mastery/m9_toolbox)
├── serde_spanned v1.1.3
│   └── toml v1.2.1+spec-1.1.0
│       └── m9_toolbox v0.0.0 (/path/to/mastery/m9_toolbox)
...
```

`-i` inverts the tree: it answers "who needs this crate?". Add `-e features` to see which crate asked for which feature. `cargo tree -d` lists crates that are present in two versions; for this package it prints `warning: nothing to print.`, which is the answer you want.

Resolver 2 and 3 keep three kinds of requests apart: features asked for by build dependencies, by dev-dependencies (unless you build tests), and by dependencies of other target platforms do not leak into your normal build. Resolver 3 is the default for edition 2024 and also prefers dependency versions that support your `rust-version`.

### Profiles and build scripts

A **profile** is a set of compiler settings with a name. `dev` and `release` exist always; you can add your own with `inherits`. Profiles are read from the workspace root only. See [task 5 of the tooling workday](../../ecosystem/TOOLING.md#5-release-profiles) for what each setting trades.

A **build script** (`build.rs`) is a program Cargo compiles and runs before your crate. It talks to Cargo by printing `cargo::` lines:

```rust,no_run
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    let target = std::env::var("TARGET").expect("Cargo sets TARGET for build scripts");
    println!("cargo::rustc-env=STAMP_TARGET={target}");
}
```

With that script, `println!("built for {}", env!("STAMP_TARGET"));` in `main` printed `built for aarch64-apple-darwin` here. A build script should say when it must run again (`rerun-if-changed`, `rerun-if-env-changed`). If it says nothing, Cargo reruns it when any file of the package changes, and never when an environment variable changes. If it sets a custom `cfg`, it must also declare it with `cargo::rustc-check-cfg`. Build scripts that compile C code are the subject of [M6](../m6_ffi/README.md).

### Taming compile times

Profiles, features and build scripts also decide how long you wait for a build. Build time grows with the number of crates in the dependency graph, with procedural macros and build scripts (they must be compiled and run before the crates that use them), and with settings such as `lto` and `codegen-units = 1`. Measure before you change anything: `cargo build --timings` writes an HTML report that shows which crates took longest and which ones waited for others. The course guide [Taming compile times](../../guides/COMPILE_TIMES.md) collects the measures that help, from trimming features and dependencies to the settings of the `dev` profile.

### Mistakes people really make

**A virtual workspace without a resolver.** A root manifest without `[package]` has no edition, so it cannot infer the resolver:

```text
warning: virtual workspace defaulting to `resolver = "1"` despite one or more workspace members being on edition 2024 which implies `resolver = "3"`
  |
  = note: to keep the current resolver, specify `workspace.resolver = "1"` in the workspace root's manifest
  = note: to use the edition 2024 resolver, specify `workspace.resolver = "3"` in the workspace root's manifest
```

**A profile in a member manifest.** It does nothing, and Cargo says so:

```text
warning: profiles for the non root package will be ignored, specify profiles at the workspace root:
```

**A custom cfg that nobody declared.** The compiler checks `cfg` names against the list Cargo gives it:

```text
warning: unexpected `cfg` condition name: `has_fast_path`
  --> crates/meter/src/lib.rs:20:10
   |
20 |     cfg!(has_fast_path)
   |          ^^^^^^^^^^^^^
   |
   = help: or consider adding `println!("cargo::rustc-check-cfg=cfg(has_fast_path)");` to the top of the `build.rs`
```

The same check catches a misspelled feature name in `#[cfg(feature = "...")]`.

### Patching, vendoring, offline builds

- `[patch.crates-io] itoa = { path = "itoa-patched" }` replaces a registry crate everywhere in the graph, for example to test a fix before it is released. After adding it, `cargo tree` showed `itoa v1.0.19 (/work/patchlab/itoa-patched)`. A patch must have the same crate name and a version the requirement accepts.
- `cargo vendor` copies all dependencies into `vendor/` and prints the `[source]` settings that make Cargo use them. Teams do this for builds without network access and for audits.
- `--offline` forbids network access; `--locked` forbids changing `Cargo.lock`; `--frozen` is both. An offline build can only use crates already in Cargo's cache. With an empty cache it fails with `error: no matching package named ... found`.

### An xtask

When a project needs automation (run all CI checks, build a release archive), a shell script ties it to one platform. An **xtask** is a normal binary crate in the workspace plus an alias in `.cargo/config.toml`:

```toml
[alias]
xtask = "run --quiet --package xtask --"
```

Now `cargo xtask ci` runs Rust code that every contributor can build. Keep the xtask out of `default-members`, so a plain `cargo build` skips it.

One trap: Cargo looks for `.cargo/config.toml` in the **current directory** and its parents, not next to the manifest. So the alias works when you run Cargo from inside the workspace, and `cargo xtask ci --manifest-path some/where/Cargo.toml` from outside fails with ``error: no such command: `xtask` ``. The same holds for every setting in that file, such as the linker in lesson 09.

### When to use which

| Need | Use |
| --- | --- |
| Same value in all members | `[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints]` |
| Optional functionality | a feature, off by default in libraries |
| Code for one platform | `[target.'cfg(windows)'.dependencies]` and `#[cfg(windows)]` |
| A value computed at build time | a build script with `cargo::rustc-env` |
| A fix in a dependency, today | `[patch]`, removed again when the fix is released |
| Why is this slow to build? | `cargo build --timings` (writes an HTML report under `target/cargo-timings/`) |
| What did Cargo resolve? | `cargo tree`, and `cargo metadata --format-version 1` for scripts |

**Read more:** [Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html) · [Features](https://doc.rust-lang.org/cargo/reference/features.html) · [Resolver versions](https://doc.rust-lang.org/cargo/reference/resolver.html#resolver-versions) · [Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) · [Build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html) · [Overriding dependencies](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html) · [cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html)

---

## Lesson 02 · Toolchains, the editor, and generated code

**Assumed:** chapter 22 (macros) and tooling-workday task 4 (debugging a macro without cargo-expand).

### rustup: which compiler runs?

`cargo` and `rustc` on your `PATH` are small **proxy** programs installed by rustup. Each time you run one, the proxy decides which installed toolchain to start. A **toolchain** is one release of the compiler with its tools, such as `1.99.0-aarch64-apple-darwin` or `nightly`. A **component** is an optional part of a toolchain: `rustfmt`, `clippy`, `rust-src`, `rust-analyzer`, `llvm-tools`, `miri`. A **target** is a platform you can compile for; each needs its own copy of the standard library (`rustup target add`).

The proxy decides in this order, and `rustup show active-toolchain` tells you the result and the reason:

```text
$ rustup show active-toolchain
1.96.0-aarch64-apple-darwin (overridden by '/work/outer/rust-toolchain.toml')
$ RUSTUP_TOOLCHAIN=nightly rustup show active-toolchain
nightly-aarch64-apple-darwin (overridden by environment variable RUSTUP_TOOLCHAIN)
$ RUSTUP_TOOLCHAIN=nightly cargo +1.99.0 --version
cargo 1.99.0 (5f94df478 2026-08-27)
```

1. `+name` on the command line.
2. The environment variable `RUSTUP_TOOLCHAIN`.
3. Walking up from the current directory: a directory override (`rustup override set`), or a `rust-toolchain.toml`, whichever is found in the nearest directory.
4. The default toolchain.

A repository should contain a `rust-toolchain.toml`. Then every contributor, the editor and CI use the same compiler, and rustup installs it on first use:

```toml
[toolchain]
channel = "1.99.0"
profile = "minimal"
components = ["rustfmt", "clippy"]
```

Pin an exact version. `channel = "stable"` means a different compiler every six weeks, with new lints, on a day you did not choose.

### rust-analyzer: what it needs

rust-analyzer is the language server behind completion, go-to-definition, inline errors and inlay hints (the grey type and parameter names the editor shows between your code). It is a separate program that understands a project by asking Cargo. It needs:

- a `Cargo.toml` it can find from the folder you opened (open the workspace root, not one file);
- the `rust-src` component, to navigate into the standard library;
- the right to run build scripts and procedural macros, because their output is part of your code.

Settings that are worth sharing in a team (names from rust-analyzer's configuration; your editor wraps them in its own file format):

| Setting | Effect |
| --- | --- |
| `rust-analyzer.check.command` = `"clippy"` | "Check on save" runs Clippy instead of `cargo check` |
| `rust-analyzer.cargo.features` = `"all"` | Code behind non-default features is analysed, not greyed out |
| `rust-analyzer.cargo.targetDir` = `true` | rust-analyzer builds in a subdirectory of `target/`, so it does not hold the build lock while you run `cargo build` in a terminal |
| `rust-analyzer.inlayHints.typeHints.enable` | Inferred types shown inline; personal taste |

The command "Expand macro recursively" shows what a macro call turns into, at the cursor.

### Reading generated code

`cargo expand` (an external tool) prints a whole crate after macro expansion. Derives are the most useful thing to read this way. For this source:

```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Money {
    cents: u8,
    units: u32,
}

// The derived order compares `cents` first, because it is declared first.
assert!(Money { cents: 5, units: 9 } < Money { cents: 10, units: 1 });
```

the expansion of `PartialOrd` contains, in order, a comparison of `cents` and then of `units`. Reading it settles the question "in which order are fields compared?" faster than any documentation.

Two cautions when you read expanded code:

- **Bounds.** A derive on `struct Wrapper<T>` generates `impl<T: Clone> Clone for Wrapper<T>`. The bound is on `T`, whatever the fields are. Sometimes that is stricter than necessary, and you write the impl by hand.
- **Hygiene is lost in the text.** A `macro_rules!` macro's own local variables cannot collide with yours, but the printed expansion shows both under the same name:

```rust
macro_rules! double_plus {
    ($e:expr) => {{
        let n = 1000;
        $e * 2 + n
    }};
}

let n = 4;
// Printed expansion: { let n = 1000; n * 2 + n }. Read as text, that is 3000.
assert_eq!(double_plus!(n), 1008);
```

Expanded output is for reading. It is not always code you could compile in place of the original.

### Mistakes people really make

- **Opening a subfolder.** rust-analyzer then finds no workspace, or the wrong one, and shows no types and no errors.
- **Missing `rust-src`.** Go-to-definition stops at the border of the standard library. The minimal profile does not install it; list it in `rust-toolchain.toml` if your team uses an editor.
- **A stale override.** `rustup override set` is stored in rustup's own settings on your machine, outside the repository. A year later it still applies to that directory and everything below it, and it silently beats the `rust-toolchain.toml` in the same directory and in every directory above. Only a toolchain file in a directory *below* the override is nearer and wins. `rustup override list` shows the overrides; `rustup override unset` removes the one for the current directory.

**Read more:** [rustup overrides](https://rust-lang.github.io/rustup/overrides.html) · [The toolchain file](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file) · [rust-analyzer configuration](https://rust-analyzer.github.io/book/configuration.html) · [Derive](https://doc.rust-lang.org/reference/attributes/derive.html) · [Macro hygiene](https://doc.rust-lang.org/reference/macros-by-example.html#hygiene)

---

## Lesson 03 · Lints and formatting as policy

**Assumed:** tooling-workday task 3, which introduces `[lints]` and `#[expect]`.

### The mental model

A **lint** is a check with a name and a **level**: `allow` (silent), `warn`, `deny` (an error), `forbid` (an error that nothing further down may relax). rustc has its own lints; Clippy adds several hundred more, sorted into **groups**: `correctness`, `suspicious`, `style`, `complexity`, `perf` (all on by default), and `pedantic`, `nursery`, `restriction`, `cargo` (off by default).

A level can be set in four places. The nearest one to the code wins:

```text
default level  <  manifest [lints] / command line  <  #![...] on the crate  <  #[...] on an item
```

`forbid` is the exception: once set, an `allow` further in is itself an error.

A team's policy lives in the manifest, so that the terminal, the editor and CI agree:

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"   # our module names are short on purpose
```

Cargo passes the table to the compiler as flags and ignores the order of the lines. `priority` decides the order instead: lower numbers are applied first, so a group needs a lower priority than the single lints that adjust it. Without it, `cargo build` says nothing and you cannot tell which line won. Clippy reports the clash (the lint `clippy::lint_groups_priority`):

```text
error: lint group `pedantic` has the same priority (0) as a lint
  --> Cargo.toml:10:1
   |
10 | pedantic = "warn"
   | ^^^^^^^^   ------ has an implicit priority of 0
11 | must_use_candidate = "allow"
   | ------------------ has the same priority as this lint
   |
   = note: the order of the lints in the table is ignored by Cargo
help: to have lints override the group set `pedantic` to a lower priority
   |
10 - pedantic = "warn"
10 + pedantic = { level = "warn", priority = -1 }
```

### Silencing one place: `expect`, with a reason

When a lint is right in general and wrong in one place, say so at that place:

```rust
#[expect(clippy::too_many_arguments, reason = "mirrors the C callback signature")]
fn on_event(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) -> u32 {
    [a, b, c, d, e, f, g, h].iter().map(|x| u32::from(*x)).sum()
}
assert_eq!(on_event(1, 1, 1, 1, 1, 1, 1, 1), 8);
```

`#[expect]` is `#[allow]` that checks itself. If the lint stops firing (someone refactored the function), the compiler tells you to remove the attribute:

```text
warning: this lint expectation is unfulfilled
  --> levels/src/lib.rs:18:10
   |
18 | #[expect(unused_variables, reason = "kept for the next release")]
   |          ^^^^^^^^^^^^^^^^
   |
   = note: kept for the next release
```

An `allow` stays forever and hides the next, different instance of the problem.

### Clippy configuration

Some lints take parameters. They go in `clippy.toml` next to the manifest:

```toml
msrv = "1.85"
disallowed-methods = [
    { path = "std::env::set_var", reason = "not thread-safe; pass configuration explicitly" },
]
```

`disallowed-methods`, `disallowed-types` and `disallowed-macros` let a team ban specific APIs with a reason that Clippy prints at every use. This turns "we agreed not to do that" into a check.

### Warnings as errors, without self-inflicted outages

You want CI to fail on warnings. You do not want the build to fail for users, or for yourself on the day a new compiler adds a lint. So:

- **Never** write `#![deny(warnings)]` in source code, and do not put `warnings = "deny"` in `[lints]`. Every new compiler version may add warnings. A commit that built yesterday then stops building for everyone who compiles it from a checkout with the newer compiler: contributors, packagers, your own CI, and projects that use the crate as a path dependency. (For crates that come from a registry or from a git dependency, Cargo passes `--cap-lints allow` to the compiler, so their users are spared. That is Cargo protecting them from you; do not rely on it.)
- **Do** make warnings errors in the CI command only: `cargo clippy --all-targets --locked -- -D warnings`.
- **Do** pin the toolchain (lesson 02). New lints then arrive in the pull request that raises the version, where someone is looking.

### Formatting

`rustfmt` removes formatting from code review. Its configuration is `rustfmt.toml`; keep it short. `cargo fmt --all --check` is the CI command: it changes nothing and fails if a file is not formatted.

Stable rustfmt accepts only stable options. With an unstable one it prints a warning and carries on without it:

```text
Warning: can't set `imports_granularity = Crate`, unstable features are only available in nightly channel.
```

The danger is quiet: teammates on nightly format differently from CI.

### Mistakes people really make

- **`forbid` plus a local `allow`.** Here the manifest forbids `unused_mut`, and one function tries to allow it:

  ```text
  error[E0453]: allow(unused_mut) incompatible with previous forbid
   --> forbidden/src/lib.rs:3:9
    |
  3 | #[allow(unused_mut)]
    |         ^^^^^^^^^^ overruled by previous forbid
    |
    = note: `forbid` lint level was set on command line (`-F unused_mut`)
  ```

  Use `deny` in the manifest if single items need an exception.
- **Turning on `restriction` as a group.** Its lints contradict each other on purpose. Pick single ones.
- **Fixing lints by changing behaviour.** A lint fix is a refactoring. Run the tests.

### When to use which

| Situation | Do |
| --- | --- |
| Rule for the whole workspace | `[workspace.lints]`, inherited by members |
| Group mostly right, a few lints wrong for you | group at `priority = -1`, single lints `allow` with a comment |
| One justified exception | `#[expect(lint, reason = "...")]` on the smallest item |
| An API the team must not call | `disallowed-methods` in `clippy.toml` |
| Warnings as errors | the CI command line, with a pinned toolchain |

**Read more:** [Lint levels](https://doc.rust-lang.org/rustc/lints/levels.html) · [The `[lints]` section](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section) · [Clippy configuration](https://doc.rust-lang.org/clippy/configuration.html) · [Clippy lint list](https://clippy.rust-lang.org/stable/index.html) · [`#[expect]`](https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-expect-attribute) · [Configuring rustfmt](https://rust-lang.github.io/rustfmt/)

---

## Lesson 04 · Testing at scale

**Assumed:** chapter 8 (unit and integration tests), [E05](../../ecosystem/README.md#e05--property-based-testing-with-proptest) (property tests and hand-made mutants).

### Where tests live, and what Cargo builds

| Kind | Place | Sees | Built as |
| --- | --- | --- | --- |
| Unit test | `#[cfg(test)] mod tests` in `src/` | private items | one executable per library or binary target |
| Integration test | each file directly in `tests/` | the public API | one executable **per file** |
| Doc test | code blocks in doc comments | the public API | compiled by rustdoc |
| Example | `examples/*.rs` | the public API | built by `cargo test`, run by `cargo run --example` |

Every file directly in `tests/` is its own crate. Shared helper code therefore goes into a subdirectory, `tests/common/mod.rs`, and each test file says `mod common;`. A helper file directly in `tests/` would become an extra, empty test executable. Many test files also mean many link steps; large projects use one file `tests/it/main.rs` with modules.

`cargo test --all-targets` does **not** run doc tests; plain `cargo test` does.

### Snapshot tests

Some output is large and meant for humans: a report, a diagnostic, generated code. Writing the expected text into an `assert_eq!` is painful, and nobody maintains it. A **snapshot test** stores the accepted output in a file and compares against it. With [insta](https://insta.rs):

```rust,ignore
#[test]
fn renders_the_report() {
    insta::assert_snapshot!(render_report(&sample()));
}
```

The first run writes `snapshots/<module>__<test>.snap.new`. You look at it and accept it; it becomes `.snap`, which you commit. Later, a change in the output fails the test with a diff:

```text
-old snapshot
+new results
────────────┬───────────────────────────────────────────────────────────────────
    5     5 │ total             155      2       5
    6       │-result: FAILED (2 failed)
          6 │+result: FAILED (2 of 157 failed)
────────────┴───────────────────────────────────────────────────────────────────
To update snapshots run `cargo insta review` or set `INSTA_UPDATE=always`
```

The discipline matters more than the tool: **a snapshot is only worth something if a person reviewed it.** Accepting without reading turns the test into "the output is whatever it is". Two rules follow. Keep snapshots small enough to read. Remove everything that changes between runs (timestamps, durations, temporary paths, hash-map order) before you snapshot; this is called **redaction**.

### Doc tests are a contract

An example in your documentation is a promise. rustdoc compiles and runs it, so the promise is checked:

- a plain block must compile and run without panicking;
- `should_panic` must panic; `no_run` must compile only;
- `compile_fail` must *not* compile. Always add the error code, as in `compile_fail,E0308`. Without it, the test also passes when the example fails for an unrelated reason such as a typo;
- lines that start with `# ` are compiled but hidden. An example that uses `?` ends with a hidden `# Ok::<(), MyError>(())`.

### Flaky tests

A flaky test fails sometimes, with no change in the code. Each one teaches the team to ignore red builds. The causes are few:

| Cause | Cure |
| --- | --- |
| Shared state: a fixed file name, a global, an environment variable | Give each test its own (`tempfile::tempdir()`), pass state as arguments |
| Order: hash-map iteration, directory listing, thread scheduling | Sort, or use `BTreeMap`; never assert on unspecified order |
| Time: `sleep`, `SystemTime::now()` inside the logic | Pass the clock in; use virtual time in async tests (E06) |
| The outside world: network, a port number, the machine's locale | A local fake; ask the OS for a free port (`127.0.0.1:0`) |

Passing the clock in looks like this:

```rust
use std::time::{Duration, SystemTime};

fn is_fresh(written: SystemTime, max_age: Duration, now: SystemTime) -> bool {
    now.duration_since(written).is_ok_and(|age| age < max_age)
}

let written = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
let later = written + Duration::from_secs(30);
assert!(is_fresh(written, Duration::from_secs(60), later));
assert!(!is_fresh(written, Duration::from_secs(10), later));
```

Tests in one executable run on parallel threads. [cargo-nextest](https://nexte.st) is a test runner that starts every test in its own process, which isolates global state, and it reports slow and flaky tests. It does not run doc tests; run `cargo test --doc` next to it.

### Coverage, and what it does not prove

**Line coverage** is the share of lines that ran during the tests. `cargo llvm-cov` measures it:

```text
    9|      5|pub fn shipping_cents(weight_g: u32, express: bool) -> u32 {
   10|      5|    let base = if weight_g <= 500 {
   11|      3|        300
   ...
   15|      1|        550 + (weight_g - 2000).div_ceil(1000) * 120
```

Coverage finds code that no test runs. That is all it can say. A line that ran is not a line that was *checked*: the tests may not look at its result, or may look at an input where a bug makes no difference.

**Mutation testing** asks the sharper question. It changes your code in small ways (`+` to `*`, `<=` to `>`, a body to `return 0`) and runs the tests against each *mutant*. A mutant that survives is a bug your tests would not notice. For the function above, with tests that reach 100% of the lines, `cargo mutants` reported:

```text
Found 12 mutants to test
MISSED   src/lib.rs:15:13: replace + with * in shipping_cents in 0s build + 0s test
MISSED   src/lib.rs:15:48: replace * with + in shipping_cents in 0s build + 0s test
MISSED   src/lib.rs:17:23: replace * with + in shipping_cents in 0s build + 0s test
...
```

The last line of the real output counts the missed and the caught mutants. Lab 4.4 asks you for that count, so it is not printed here.

Mutation testing is slow (one build and test run per mutant) and reports some mutants that cannot be killed because they do not change behaviour. Run it on the modules that matter, now and then, not on every commit.

### When to use which

| Question | Tool |
| --- | --- |
| Is this function right for these inputs? | example test |
| Is this property true for all inputs? | property test (E05), fuzzing (lesson 05) |
| Did this large output change? | snapshot test |
| Does the documented example still work? | doc test |
| Which code does no test run? | coverage |
| Would my tests notice a bug here? | mutation testing |

**Read more:** [Test organisation in Cargo](https://doc.rust-lang.org/cargo/guide/tests.html) · [`cargo test`](https://doc.rust-lang.org/cargo/commands/cargo-test.html) · [Documentation tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html) · [insta](https://insta.rs/docs/) · [cargo-nextest](https://nexte.st) · [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) · [cargo-mutants](https://mutants.rs)

---

## Lesson 05 · Fuzzing and sanitizers

**Assumed:** [E05](../../ecosystem/README.md#e05--property-based-testing-with-proptest) (generators, properties, shrinking) and chapter 23 (what undefined behaviour is).

### The mental model

A **fuzzer** runs one function millions of times with inputs it invents. A **coverage-guided** fuzzer (libFuzzer, used by `cargo fuzz`) watches which branches of your code each input reaches. An input that reaches new code is kept in the **corpus** and mutated further. So the fuzzer learns the input format by itself: it finds that a `%` leads somewhere new, then that two characters after it matter, and so on.

A **fuzz target** is the function it calls:

```rust,ignore
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = my_crate::parse(text);
    }
});
```

The fuzzer reports a **finding** when the target panics, aborts, overflows the stack, runs out of memory, or takes too long. Nothing else. A target that ignores the parser's result can therefore only find crashes. To find *wrong answers* you must turn them into panics, with the same properties you know from proptest: decode after encode gives the input back, the new implementation agrees with the old one, an accepted value satisfies its invariant.

### Structure-aware fuzzing

Random bytes are a good input for a parser of bytes. For code that takes structured values they are poor: almost every input is rejected early. The [arbitrary](https://docs.rs/arbitrary) crate turns the fuzzer's bytes into values of your type. `Unstructured` wraps the bytes and hands them out:

```rust,ignore
impl<'a> Arbitrary<'a> for Colour {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(match u.int_in_range(0..=2)? {
            0 => Colour::Named(*u.choose(&["red", "green", "blue"])?),
            1 => Colour::Grey(u.arbitrary()?),
            _ => Colour::Rgb(u.arbitrary()?, u.arbitrary()?, u.arbitrary()?),
        })
    }
}
```

Two properties of `Unstructured` shape how you write such code. When the bytes run out it returns zeros instead of failing, so short inputs give small values and the fuzzer can grow them step by step. And the same bytes always give the same value, so a finding can be replayed. Often `#[derive(Arbitrary)]` is enough; write the impl by hand when values must satisfy a rule (a size limit, a restricted alphabet).

### From a crash to a regression test

A session with the decoder of this lesson, on this machine:

```text
$ cargo +nightly-2026-11-20 fuzz run decode
thread '<unnamed>' panicked at src/l05_fuzz.rs:71:36:
end byte index 6 is not a char boundary; it is inside '\u{6dc}' (bytes 5..7 of string)
==ERROR: libFuzzer: deadly signal
...
Failing input:

	fuzz/artifacts/decode/crash-80cf75330e636cfa6ee7409d31352122e748481e

Output of `std::fmt::Debug`:

	[2, 0, 0, 37, 36, 219, 156, 8]

Minimize test case with:

	cargo fuzz tmin decode fuzz/artifacts/decode/crash-80cf75330e636cfa6ee7409d31352122e748481e
```

It took a few seconds. The crashing input is eight bytes with noise in it. `cargo fuzz tmin` removes what is not needed and ended with four bytes. Then the work is ordinary engineering:

1. Copy the minimised input into a normal `#[test]` that fails. This is the regression test; it runs in every `cargo test` from now on, without a fuzzer.
2. Fix the bug. The test passes.
3. Commit the test, and keep the corpus (`fuzz/corpus/`) somewhere, so the next session starts where this one stopped.

`cargo fuzz` needs a nightly toolchain, because the instrumentation flags are unstable. Your crate does not: the fuzz project is a separate package in `fuzz/`, outside the workspace.

### Sanitizers

A **sanitizer** is extra checking code that the compiler inserts into the program. It catches errors at the moment they happen, where a normal build would continue with corrupt memory. They matter for `unsafe` code and for C code you link.

| Sanitizer | Finds | Flag |
| --- | --- | --- |
| AddressSanitizer (ASan) | out-of-bounds access, use after free, double free | `-Zsanitizer=address` |
| ThreadSanitizer (TSan) | data races | `-Zsanitizer=thread` |
| LeakSanitizer | memory that is never freed | `-Zsanitizer=leak` |
| MemorySanitizer | reads of uninitialised memory (Linux) | `-Zsanitizer=memory` |

A program that reads one element past a `Vec` printed a number and exited with 0 in a normal build. With ASan:

```text
$ RUSTFLAGS=-Zsanitizer=address cargo +nightly-2026-11-20 run --target aarch64-apple-darwin --bin overread
==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x602000000980 ...
READ of size 4 at 0x602000000980 thread T0
    #0 0x000100b41514 in _RNvCs3BfjgHXtgqp_8overread9sum_first overread.rs:11
    #1 0x000100b410d0 in _RNvCs3BfjgHXtgqp_8overread4main overread.rs:19
...
0x602000000980 is located 0 bytes after 16-byte region [0x602000000970,0x602000000980)
```

The report names the bad access, and the allocation it ran past. Pass `--target` explicitly, so the flags apply to your program and not to build scripts. TSan must see every memory access, so the standard library has to be rebuilt with it (`-Zbuild-std`, which needs the `rust-src` component); without that, the compiler refuses with "mixing `-Zsanitizer` will cause an ABI mismatch". `cargo fuzz` builds with ASan by default.

### Fuzzer, sanitizer, Miri: which one?

| Tool | Strength | Limit |
| --- | --- | --- |
| Fuzzer | finds inputs you would never write | only sees what the target turns into a crash |
| Sanitizer | catches memory errors and races at native speed, also in C code | only on paths that actually run; nightly; not every platform |
| Miri ([M2](../m2_unsafe/README.md)) | checks Rust's aliasing rules exactly | slow; cannot run C code or most system calls |
| loom ([M3](../m3_concurrency/README.md)) | tries every interleaving of a small concurrent test | needs code written against its types |

They combine: a fuzzer supplies the inputs, a sanitizer watches each run.

### Mistakes people really make

- **A harness that swallows the result.** `let _ = parse(data);` finds panics only. Ask what must be true of the result, and assert it.
- **A generator that never reaches the interesting inputs.** If your `Arbitrary` impl only produces letters, the fuzzer never tries `%`. This is the lesson of E05's mutants again.
- **Fixing the crash and losing the input.** Without the regression test, the bug comes back with the next refactoring.

**Read more:** [The Rust Fuzz Book](https://rust-fuzz.github.io/book/) · [arbitrary](https://docs.rs/arbitrary) · [Sanitizers in the unstable book](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html) · [`str` indexing and char boundaries](https://doc.rust-lang.org/std/primitive.str.html#method.is_char_boundary)

---

## Lesson 06 · Debugging

**Assumed:** chapter 7 (panics and unwinding) and [E04](../../ecosystem/README.md#e04--tracing) (structured logging).

### The mental model

A failing program gives you a **symptom**: the place where it could not continue. The **cause** is the place where a wrong value was produced. They are often far apart. Debugging is walking backwards from one to the other, and each tool below answers one kind of question on that walk.

### The panic message and the backtrace

```text
thread 'main' (122946845) panicked at src/bin/panicker.rs:38:5:
depth 3 is over the limit 2
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Read it fully: thread name, file, line and column of the `panic!` (or of the standard-library check that failed), then the message. A process whose main thread panics exits with code 101.

A **backtrace** is the chain of function calls that were active. `RUST_BACKTRACE=1` prints it, without the runtime's own frames:

```text
stack backtrace:
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: panicker::inner
             at ./src/bin/panicker.rs:38:5
   3: panicker::middle
             at ./src/bin/panicker.rs:33:5
   4: panicker::outer
             at ./src/bin/panicker.rs:29:5
   5: panicker::main
             at ./src/bin/panicker.rs:25:5
```

Read from the top down to the first frame in your own code: that is where the symptom appeared. The frames below it say how the program got there. File names and line numbers come from debug information; a release build needs `debug = "line-tables-only"` to keep them (tooling-workday task 5). In your own code, `std::backtrace::Backtrace::capture()` gives the same information as a value, for example to store in an error.

### `dbg!`

`dbg!(expr)` prints the file, line, expression text and value to **stderr**, and returns the value, so you can wrap it around part of an expression without changing the program:

```rust
let total = dbg!(3 * 4) + 1;
assert_eq!(total, 13);
```

It is the fastest way to answer one question. It is not something to commit; the lint `clippy::dbg_macro` keeps it out (lesson 03).

### Panic hooks

Before a panic unwinds, the runtime calls the **panic hook**. The default hook prints the message you saw above. `std::panic::set_hook` replaces it, for the whole process. Programs use this to log panics in their own format, or to report them. Only the hook sees the location:

```rust
use std::sync::{Arc, Mutex};

let seen = Arc::new(Mutex::new(Vec::new()));
let sink = Arc::clone(&seen);
let previous = std::panic::take_hook();
std::panic::set_hook(Box::new(move |info| {
    let line = info.location().map_or(0, |location| location.line());
    let message = info.payload_as_str().unwrap_or("<no text>").to_owned();
    sink.lock().unwrap().push((line, message));
}));
let outcome = std::panic::catch_unwind(|| panic!("no stock"));
std::panic::set_hook(previous);

assert!(outcome.is_err());
assert_eq!(seen.lock().unwrap()[0].1, "no stock");
```

The hook is global state. In a test suite that runs on many threads, a hook that one test installs sees the panics of all the others. That is the difficulty of this lesson's task.

`catch_unwind` stops a panic at a boundary (a thread pool, a plugin call). It only works when panics unwind. With `panic = "abort"` in the profile, the hook still runs, and then the process ends.

### A debugger

A debugger stops the running program and lets you look at every variable in every active frame. `rust-lldb` and `rust-gdb` ship with the toolchain; they start LLDB or GDB with Rust's pretty-printers loaded, so a `Vec` shows its elements instead of three raw pointers. You need a debugger on your system (Xcode's command-line tools on macOS, `lldb` or `gdb` packages on Linux) and a debug build.

A session on a small test program, stopped at the panic:

```text
$ rust-lldb target/debug/stockroom
(lldb) breakpoint set --name rust_panic
(lldb) run
(lldb) bt
    frame #9:  ... <alloc::vec::Vec<stockroom::Item> as core::ops::index::IndexMut<usize>>::index_mut(self=size=4, index=4) at mod.rs:3943:9
    frame #10: ... <stockroom::Stockroom>::ship(self={items:size=4, index:size=5}, sku="hinge", quantity=20) at stockroom.rs:47:35
    frame #11: ... stockroom::main at stockroom.rs:78:19
(lldb) frame select 10
(lldb) frame variable position
(unsigned long) position = 4
```

`rust_panic` is a function the runtime calls for every panic, which makes it a convenient breakpoint. The useful commands are few: `bt` (backtrace), `frame select N`, `frame variable [name]`, `breakpoint set --file F --line N`, `next`, `step`, `continue`. In GDB: `bt`, `frame N`, `info locals`, `print name`, `break F:N`.

### Logging or debugging?

| Situation | Reach for |
| --- | --- |
| It fails on your machine, every time | backtrace, then `dbg!` or a debugger |
| You need to see the state of many variables at once | a debugger |
| It fails only in CI or production, or only sometimes | logging with context (E04 tracing); you cannot stop that program |
| Memory is corrupted | a sanitizer or Miri (lesson 05); the crash site is not the cause |
| It is slow, not wrong | a profiler ([M7](../m7_perf/README.md)) |

### Mistakes people really make

- **Patching the symptom.** An index is out of bounds, so you add a bounds check there. The wrong index is still being produced, and now it fails silently somewhere else. Ask: which value is wrong here, and who made it?
- **Debugging a release build.** Variables are "optimized out" and lines jump around. Use the dev profile.
- **Forgetting that the hook replaces the default output.** After `set_hook`, nothing prints the message or the backtrace unless your hook does, or calls the previous hook.

**Read more:** [`std::panic`](https://doc.rust-lang.org/std/panic/index.html) · [`set_hook`](https://doc.rust-lang.org/std/panic/fn.set_hook.html) · [`std::backtrace`](https://doc.rust-lang.org/std/backtrace/index.html) · [`dbg!`](https://doc.rust-lang.org/std/macro.dbg.html) · [The `panic` profile setting](https://doc.rust-lang.org/cargo/reference/profiles.html#panic)

---

## Lesson 07 · Documentation

**Assumed:** tooling-workday task 1 (`cargo doc`, a first doc test, one intra-doc link).

### The mental model

A crate is read by four kinds of reader, and each has a place:

| Reader | Question | Place |
| --- | --- | --- |
| Someone choosing a crate | What is this, and is it for me? | `README.md`, shown on crates.io and on the repository page |
| Someone starting to use it | How do I begin? | crate-level docs (`//!` in `lib.rs`), `examples/` |
| Someone calling one function | What exactly does it do? | the item's doc comment |
| Someone upgrading | What changed? | `CHANGELOG.md` |

rustdoc builds the reference from your doc comments. The standard sections of an item are a one-line summary, details, and then `# Examples`, `# Errors` (when it returns `Result`), `# Panics`, `# Safety` (for `unsafe fn`).

### One README, tested

The README and the crate-level documentation want the same content. Write it once:

```rust,ignore
#![doc = include_str!("../README.md")]
```

Now the README's Rust code blocks are doc tests. An example in the README that no longer compiles fails `cargo test`. Mark blocks that are not Rust (`sh`, `text`, `toml`), or rustdoc will try to compile them.

### Intra-doc links

Write the names of other items in brackets, and rustdoc links and checks them:

```rust
/// A temperature in degrees Celsius. Convert with [`Celsius::to_kelvin`];
/// the lowest value is [`ABSOLUTE_ZERO`].
pub struct Celsius(pub f64);

/// The coldest temperature there is.
pub const ABSOLUTE_ZERO: Celsius = Celsius(-273.15);

impl Celsius {
    /// The same temperature in kelvin.
    pub fn to_kelvin(&self) -> f64 {
        self.0 + 273.15
    }
}
assert_eq!(ABSOLUTE_ZERO.to_kelvin(), 0.0);
```

A name in back-ticks alone is only code font. In brackets it is a link that breaks the docs build when the item is renamed:

```text
error: unresolved link to `Slug::create`
 --> src/lib.rs:5:20
  |
5 | /// wrapper, and [`Slug::create`] for its constructor.
  |                    ^^^^^^^^^^^^ the struct `Slug` has no field or associated item named `create`
  |
  = note: `-D rustdoc::broken-intra-doc-links` implied by `-D warnings`
```

That error appears with `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`, which is the command for CI. To find undocumented items, turn on the rustc lint `missing_docs`.

### Features and docs.rs

rustdoc documents what is compiled. An item behind a feature that is off does not exist for it. docs.rs, which builds the documentation of every published crate, uses default features unless you say otherwise:

```toml
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

`--cfg docsrs` is a convention: a cfg that is only set on docs.rs, which builds with nightly. With it you can switch on an unstable rustdoc feature there and nowhere else:

```rust,ignore
#![cfg_attr(docsrs, feature(doc_cfg))]
```

With `doc_cfg` on, rustdoc adds a label such as "Available on crate feature `titles` only" to every item behind a `#[cfg]`. On the nightly used for this module the labels appear automatically once the feature is on; an explicit `#[doc(cfg(...))]` overrides the text. On stable the attribute line is skipped, because `docsrs` is not set. To preview what docs.rs will show:

```sh
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly-2026-11-20 doc --no-deps --all-features
```

### Examples directory

Each file in `examples/` is a small program that uses the crate from outside, like a user. `cargo run --example name` runs one; `cargo test` compiles them all, so they cannot rot. Use examples for anything longer than a doc test: a complete command-line program, a server, a typical workflow.

### The changelog

A changelog is written for people who upgrade. Git history is written for people who work on the code. They are not the same list. The widely used layout ("Keep a Changelog"):

```text
  ## [Unreleased]

  ## [1.2.0] - 2026-09-14

  ### Added
  - `Slug::as_str`.

  ### Fixed
  - Empty input no longer panics.
```

Newest release first; dates as `YYYY-MM-DD`; entries grouped as Added, Changed, Deprecated, Removed, Fixed, Security. The `[Unreleased]` section is where every pull request adds its line, so writing release notes is renaming a heading. Which change needs which version number is the subject of [M8](../m8_api/README.md).

### Mistakes people really make

- **A README example that is not a doc test.** It breaks with the first API change and nobody notices until a user files an issue.
- **`cargo doc` without `--no-deps` in CI.** It documents every dependency too, slowly, and fails on their warnings.
- **Documenting how instead of what.** The doc comment says what the function promises to its caller. How it does that belongs in a normal comment inside.

**Read more:** [The rustdoc book](https://doc.rust-lang.org/rustdoc/) · [Linking to items by name](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html) · [Documentation tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html) · [`missing_docs`](https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#missing-docs) · [Cargo's `examples`](https://doc.rust-lang.org/cargo/guide/project-layout.html) · [docs.rs build metadata](https://docs.rs/about/metadata) · [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

---

## Lesson 08 · Supply chain and policy

**Assumed:** lesson 01 (resolution and the lock file).

### The mental model

Your program is mostly other people's code. This package has five direct dependencies and about forty crates in its lock file. Each is written by someone you have not met, and each can publish a new version at any time. *Supply-chain* work answers four questions about that code, with tools instead of hope:

| Question | Tool |
| --- | --- |
| Exactly which code are we building? | `Cargo.lock`, `--locked` |
| Is any of it known to be vulnerable or abandoned? | `cargo audit`, `cargo deny check advisories` |
| May we legally ship it? Does it pull in things we banned? | `cargo deny check licenses bans sources` |
| Has anyone we trust read it? | `cargo vet` |

### Version requirements are ranges

`serde = "1.0.200"` does not mean "version 1.0.200". It is a **caret requirement**: any version from 1.0.200 up to, but not including, 2.0.0. Below 1.0 the rule shifts one place: `"0.8.5"` allows 0.8.5 up to, not including, 0.9.0. Other forms: `"~1.4"` (1.4.x only), `">=1.2, <1.9"`, and `"=1.4.2"` (exactly that).

The resolver picks the newest version that fits, once, and records it in `Cargo.lock`. From then on the lock file decides. It changes only when you ask (`cargo update`, or editing a manifest). So:

- **Commit `Cargo.lock`**, for applications and for libraries. For an application it defines what you ship. For a library it makes your own CI repeatable. It does not bind your library's users: their build makes its own lock file from your *requirements*.
- **Build with `--locked` in CI.** Without it, Cargo quietly updates a lock file that does not match the manifest, and CI tests something nobody reviewed.
- `cargo install some-tool` ignores the tool's lock file unless you pass `--locked`.

### Advisories

The [RustSec advisory database](https://rustsec.org) lists vulnerable, unsound, unmaintained and yanked crate versions. `cargo audit` compares your lock file with it:

```text
$ cargo audit
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 1550 security advisories (from ~/.cargo/advisory-db)
    Updating crates.io index
    Scanning Cargo.lock for vulnerabilities (43 crate dependencies)
```

No findings means: nothing *known* today. New advisories appear for old versions, so run it on a schedule, not only when the code changes.

### Policy with cargo-deny

`cargo deny` checks four things against a `deny.toml` in the repository: `advisories` (the same database), `licenses`, `bans` (unwanted crates, duplicate versions, `*` requirements) and `sources` (only registries and git hosts you named). A licence that is not on the allow list fails the build, and the report shows who pulled the crate in. This is the report for an allow list that contains only `Apache-2.0`:

```text
error[rejected]: failed to satisfy license requirements
   ┌─ ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winnow-1.0.5/Cargo.toml:42:12
   │
42 │ license = "MIT"
   │            ━━━
   │            │
   │            rejected: license is not explicitly allowed
   ├ MIT - MIT License:
   ├   - OSI approved
   ├   - FSF Free/Libre
   ├ winnow v1.0.5
     ├── toml v1.2.1+spec-1.1.0
     │   └── m9_toolbox v0.0.0
     └── toml_parser v1.1.5+spec-1.1.0
         └── toml v1.2.1+spec-1.1.0 (*)

licenses FAILED
```

Write the allow list from your organisation's decision, not from what happens to be in the graph today. An expression such as `MIT OR Apache-2.0` passes if one side is allowed. A real detail from the capstone: with `wildcards = "deny"`, path dependencies between your own crates count as wildcards unless you add `allow-wildcard-paths = true`.

`cargo vet` goes one step further: it records, per crate version, that somebody audited the code, and it can import the audit lists of organisations you trust. `cargo vet init` creates a `supply-chain/` directory that starts by exempting everything you already use; the work is shrinking that list.

### MSRV: the oldest compiler you promise to support

`rust-version = "1.85"` in the manifest is the **minimum supported Rust version**. Cargo refuses to build the crate with an older compiler:

```text
$ cargo +1.96.0 check --manifest-path mastery/m9_toolbox/Cargo.toml
error: rustc 1.96.0 is not supported by the following packages:
  m9_toolbox@0.0.0 requires rustc 1.99
```

(Cargo prints the second line once for every target of the package; it is shown once here.)

The field is a promise, and nothing checks it unless you do. Two checks, both needed:

- **Clippy** knows when each standard-library item became stable and compares with `rust-version`:

  ```text
  warning: current MSRV (Minimum Supported Rust Version) is `1.85.0` but this item is stable since `1.88.0`
   --> src/lib.rs:6:31
    |
  6 |     let (words, rest) = bytes.as_chunks::<4>();
    |                               ^^^^^^^^^^^^^^^^
    = note: `#[warn(clippy::incompatible_msrv)]` on by default
  ```

  It does not know about language features or about your dependencies' requirements.
- **A CI job** that installs exactly the MSRV toolchain and runs `cargo check --locked`. This is the real test.

Raising the MSRV shuts out users on older compilers. Treat it as a deliberate, announced change ([M8](../m8_api/README.md)).

### Honest lower bounds

If your manifest says `semver = "1.0.20"` but your code uses a function added in 1.0.25, your crate is broken for anyone whose lock file holds 1.0.20. Normal builds never notice, because they resolve to the newest version. Nightly Cargo can resolve to the *oldest* allowed versions instead:

```text
$ cargo +nightly-2026-11-20 update -Z minimal-versions
 Downgrading semver v1.0.28 -> v1.0.20 (available: v1.0.28)
```

Then build and test with that lock file. `-Z direct-minimal-versions` lowers only your direct dependencies, which avoids failures caused by other crates' sloppy bounds.

### When to use which

| Goal | Do |
| --- | --- |
| Repeatable builds | commit `Cargo.lock`; `--locked` in CI |
| Know about vulnerabilities | `cargo audit` or `cargo deny check advisories`, on a schedule |
| Licence and dependency rules | `deny.toml`, checked in CI |
| Keep the MSRV promise | `rust-version`, Clippy, and an MSRV job |
| Update dependencies | `cargo update` in its own pull request, read the changelogs, run everything. A bot such as Dependabot or Renovate can open that pull request for you; a person still reads it |

**Read more:** [Specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) · [Cargo.toml vs Cargo.lock](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html) · [The `rust-version` field](https://doc.rust-lang.org/cargo/reference/rust-version.html) · [RustSec](https://rustsec.org) · [cargo-deny](https://embarkstudios.github.io/cargo-deny/) · [cargo-vet](https://mozilla.github.io/cargo-vet/) · [`minimal-versions`](https://doc.rust-lang.org/cargo/reference/unstable.html#minimal-versions)

---

## Lesson 09 · CI and release

**Assumed:** lessons 02, 03, 04 and 08. This lesson assembles their commands.

### The mental model

**Continuous integration (CI)** runs your checks on a clean machine for every change. Its value is one sentence: *if CI is green, the change is safe to merge.* Two things destroy that value. A check that is missing, so green does not mean safe. And a check that fails for reasons outside the change, so red does not mean broken. Most of the craft is removing the second kind.

What can change without a commit? The compiler (`stable` moves), dependencies (no `--locked`), the runner image (`ubuntu-latest` moves), the actions you call (`@main` moves), the network, the clock. Pin each one, and CI becomes a function of the commit.

### A GitHub Actions workflow, piece by piece

A workflow is a YAML file in `.github/workflows/`. It has triggers (`on`), and **jobs** that run on fresh virtual machines (**runners**). A job is a list of **steps**; a step runs a shell command (`run`) or a packaged **action** (`uses`).

```yaml
name: CI
on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read          # the workflow's token can read the repository, nothing more

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true   # a newer push cancels the run of the older one

jobs:
  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-24.04, macos-15, windows-2025]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v7
      - run: rustup toolchain install 1.99.0 --profile minimal && rustup default 1.99.0
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --locked
```

- A **matrix** runs the job once per combination of its values. `fail-fast: false` lets the other combinations finish when one fails, so you learn whether a failure is specific to one platform.
- **Caching** saves `~/.cargo` and `target/` between runs, keyed by the lock file and the toolchain. Without it every run compiles all dependencies again.
- A `run:` line is given to bash on Linux and macOS, and to PowerShell on Windows. A line that must work on all three either says `shell: bash`, or avoids shell syntax: `${{ env.NAME }}` is filled in by GitHub before any shell sees the line, while `$NAME` means an environment variable only to bash.
- Give the workflow token the **least privilege** it needs. Pin actions to a version tag, or to a full commit hash when the action is from a small publisher. A major tag such as `v7` was current when this was written (`actions/checkout` 7.0.1, `Swatinem/rust-cache` 2.9.3, `EmbarkStudios/cargo-deny-action` 2.1.3); look up the current one when you write a workflow, because old majors stop working when the runner drops their runtime.
- The example names the toolchain in the workflow. If the repository has a `rust-toolchain.toml` (lesson 02), `rustup toolchain install` without a name installs exactly what that file asks for, and the version is written in one place for people and for CI. One trap follows from lesson 02: the file beats `rustup default`. A job that must use another compiler, such as the MSRV job, then has to say so on the command itself: `cargo +1.85.0 check --locked`.

The checks themselves are the commands you already know:

| Check | Command |
| --- | --- |
| Formatting | `cargo fmt --all --check` |
| Lints | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` |
| Tests | `cargo test --workspace --all-features --locked` |
| Docs | `cargo doc --workspace --no-deps --locked`, with `RUSTDOCFLAGS: -D warnings` |
| Policy | `cargo deny check`, with a `deny.toml` in the repository (without one, no licence is allowed and the check fails) |
| MSRV | on the `rust-version` toolchain: `cargo check --workspace --all-features --locked` |

Run fmt, Clippy and docs once, on Linux. Run tests on every platform you support. A contributor must be able to run each line locally; an xtask (lesson 01) that runs the same list keeps the two from drifting apart.

### Cross-compilation

A **target triple** such as `aarch64-unknown-linux-gnu` names the CPU, the vendor, the operating system and the C library. `rustc --print cfg --target <triple>` shows what `#[cfg]` sees for it. Compiling for another target needs two things: its standard library (`rustup target add`) and a **linker** for that platform. Rust code alone often cross-compiles with only the first. When it does not:

```text
error[E0463]: can't find crate for `std`
  |
  = note: the `x86_64-unknown-linux-musl` target may not be installed
  = help: consider downloading the target with `rustup target add x86_64-unknown-linux-musl`
```

The linker is named in `.cargo/config.toml`, per target. The tool [cross](https://github.com/cross-rs/cross) wraps Cargo and runs the build inside a container image that already has the right linker and C libraries; `cross build --target ...` then works like `cargo build`. For Linux binaries that must run everywhere, the `*-linux-musl` targets link the C library statically.

### Reproducible release builds

A build is **reproducible** when the same source gives the same binary, byte for byte, on another machine. That lets anyone verify that a published binary came from the published source. The inputs to control: the toolchain (pinned), the dependencies (`--locked`), the profile (written in the manifest, not on someone's command line), and absolute paths that end up inside the binary (panic messages of dependencies contain `~/.cargo/registry/...`). `RUSTFLAGS="--remap-path-prefix=$HOME=/home"` replaces such a prefix; a Cargo profile option for this, `trim-paths`, is still unstable in 1.99.

A typical release profile for a command-line tool:

```toml
[profile.release]
lto = "thin"
codegen-units = 1
strip = "symbols"
```

### Publishing a crate

`cargo package --list` shows the files that would be uploaded. By default, inside a git repository, that is every file in the package directory that git does not ignore: tracked files, new files you have not added yet, and hidden directories such as `.github/`. Outside a git repository Cargo takes every file except hidden ones. Use `include` (an allow list) or `exclude` in `[package]` to keep test data, notes and CI files out; an allow list is the safer of the two, because a new file is left out until you name it. crates.io limits a package to 10 MB. `cargo package` then builds the crate from the packed files alone, which catches a missing file. `cargo publish --dry-run` does everything except the upload:

```text
   Packaging indicogni-publish-lab v0.1.0 (/work/publab)
    Packaged 4 files, 1.1KiB (773B compressed)
   Verifying indicogni-publish-lab v0.1.0 (/work/publab)
   Uploading indicogni-publish-lab v0.1.0 (/work/publab)
warning: aborting upload due to dry run
```

A published version is **permanent**: it can never be changed or deleted. `cargo yank --version 1.2.3` marks a version so that new lock files will not choose it; projects that already locked it keep working. Yank a broken release and publish a fixed one. `publish = false` in the manifest protects a private crate from an accidental `cargo publish`.

Binary releases at a glance: a second workflow, triggered by a version tag, builds the release profile on a matrix of targets and attaches the archives to a release page. Tools such as cargo-dist generate that workflow. Sign or checksum what you upload.

### Mistakes people really make

- **`toolchain: stable` in CI.** Green on Monday, red on Tuesday, no commit in between: a new lint arrived.
- **No `--locked`.** CI resolves fresher dependencies than the lock file your users get.
- **`continue-on-error: true` on a check that fails too often.** The check is now decoration. Fix it or delete it.
- **`cargo test --all-targets` as the only test command.** Doc tests never run.

**Read more:** [Continuous integration in the Cargo book](https://doc.rust-lang.org/cargo/guide/continuous-integration.html) · [Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html) · [`cargo package`](https://doc.rust-lang.org/cargo/commands/cargo-package.html) · [`cargo yank`](https://doc.rust-lang.org/cargo/commands/cargo-yank.html) · [Platform support](https://doc.rust-lang.org/rustc/platform-support.html) · [Cargo configuration: `[target]`](https://doc.rust-lang.org/cargo/reference/config.html#target) · [GitHub Actions workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)

---

## Lesson 10 · Capstone

The capstone has no new ideas. It asks for all of the above at once, on a project in the state real projects are often found in: [capstone/TASK.md](capstone/TASK.md).

An order that works:

1. Make it a workspace and get `cargo build` green (lesson 01). Generate `Cargo.lock`.
2. Write tests for the behaviour in the task, and fix the parser until they pass. Start with the inputs that panic.
3. Turn the lints on and clear them (lesson 03). Format.
4. Split out the `json` feature (lesson 01), and check both configurations.
5. Add the snapshot test (lesson 04) and the fuzz target (lesson 05). If you have cargo-fuzz, run it for a minute.
6. Document, write the changelog (lesson 07).
7. Add `deny.toml` (lesson 08) and the workflow (lesson 09).

Fix behaviour before style. A lint clean-up on top of a parser you are about to rewrite is work done twice.
