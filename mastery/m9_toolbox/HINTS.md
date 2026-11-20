# M9 hints

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Labs](LABS.md)

Try the first hint before opening the second. Open *Check your explanation* only after writing your own answer to the lesson's EXPLAIN prompt in [NOTES.md](NOTES.md).

For the fixture lessons, the fastest hint is often Cargo itself: `cd` into the fixture and run the command the failing test names. Its warnings usually say what to change.

## Lesson 01 · Cargo in depth

<details><summary>Hint 1</summary>

Start in the fixture: `cd mastery/m9_toolbox/fixtures/l01_ws && cargo metadata --no-deps --format-version 1 > /dev/null`. Cargo prints two warnings, and each names its fix (a resolver for the virtual workspace; a profile in the wrong manifest). Then work top-down in the root `Cargo.toml`: `[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints.rust]`. A member inherits one key at a time: `version.workspace = true`, `tinyjson = { workspace = true, optional = true }`, and a table `[lints]` with `workspace = true`.

</details>
<details><summary>Hint 2</summary>

Features: in `meter`, `[features] default = []` and `json = ["dep:tinyjson"]`. The `dep:` prefix is what removes the implicit feature `tinyjson`; then change the `#[cfg(feature = "tinyjson")]` in `src/lib.rs`. In `meter-cli`: `default = ["json"]`, `json = ["meter/json"]`, and gate the JSON code with `cfg!(feature = "json")` and `#[cfg(feature = "json")]` / `#[cfg(not(feature = "json"))]`. Windows: move `winconsole` under `[target.'cfg(windows)'.dependencies]`. Build script: print `cargo::rerun-if-env-changed=METER_FAST` and `cargo::rustc-check-cfg=cfg(has_fast_path)` on every run, and `cargo::rustc-cfg=has_fast_path` only when the variable is set. xtask: `members = ["crates/*", "xtask"]`, `default-members = ["crates/*"]`, and `.cargo/config.toml` with `[alias] xtask = "run --quiet --package xtask --"`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Cargo compiles each crate once per build. When several packages in that build ask for different features of the same dependency, it gets the union of all requests. `cargo build -p app` builds only what `app` needs, so the dependency has only `app`'s features; `cargo build --workspace` builds everything together, so the same dependency now also has the features other members asked for. Code that only works with a feature it never asked for compiles in the workspace build and breaks alone (or the other way round, if a feature removes something). That is why features must be additive, and why CI should also build each package on its own (`cargo check -p name`, or cargo-hack's `--each-feature`). Build dependencies and dev-dependencies are unified separately from normal ones with resolver 2 and 3.

</details>

## Lesson 02 · Toolchains, the editor, and generated code

<details><summary>Hint 1</summary>

`active_toolchain` is two early returns and one loop. Check `plus`, then `env`. Then start at `cwd` and repeat: is there an override for exactly this directory? Is there a file for exactly this directory? If neither, go to the parent. Compare whole directory strings with `==`; do not use `starts_with`, or `/work/app` will match `/work/app2`.

</details>
<details><summary>Hint 2</summary>

The parent of `/work/app` is the text before the last `/`: `dir.rfind('/')` gives its position. Two special cases: when that position is 0, the parent is `"/"` (not the empty string), and when the directory already is `"/"`, stop. For the files: the component rust-analyzer needs is `rust-src`; the three settings are `rust-analyzer.check.command`, `rust-analyzer.cargo.features` and `rust-analyzer.cargo.targetDir`. For the predictions, read `expanded.rs` line by line: which field does `partial_cmp` compare first? What is written after `impl<T:`? And remember that the printed `x + x` hides which `x` is which.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

rust-analyzer builds its own model of the project. It needs a workspace root it can hand to Cargo (`cargo metadata`), the sources of the standard library (`rust-src`) to resolve and show std items, and permission to run build scripts and procedural macros, since generated code is part of what it must analyse. It uses the toolchain rustup selects for the directory, so a `rust-toolchain.toml` makes the editor, the terminal and CI agree on one compiler and one set of lints; it also makes rustup install missing components for a new contributor. Without the file, "works on my machine" includes the compiler version. Team-wide settings (check with Clippy, analyse all features) belong in the repository; taste (inlay hints) does not.

</details>

## Lesson 03 · Lints and formatting as policy

<details><summary>Hint 1</summary>

Add the `[lints.rust]` and `[lints.clippy]` tables first, delete `#![deny(warnings)]`, and run `cargo clippy --all-targets` in the fixture. Now you see the real list. Most findings come with a `help:` line that shows the fix. `rustfmt.toml`: remove the option that stable rustfmt warns about, set the three required ones, run `cargo fmt`.

</details>
<details><summary>Hint 2</summary>

`pedantic = { level = "warn", priority = -1 }`. `net_total` takes `&[Line]` and becomes an iterator chain with `i64::from(line.quantity)`. `tax` can use integer arithmetic: `cents * i64::from(percent) / 100`. The `#[allow(unused_variables)]` is stale: delete it. The `#[allow(clippy::too_many_arguments)]` is justified: make it `#[expect(clippy::too_many_arguments, reason = "...")]`. `must_use_candidate` fires on every function; either add `#[must_use]` or allow that one lint in the table with a comment. `clippy.toml`: `disallowed-methods = [{ path = "std::process::exit", reason = "..." }]`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`#![deny(warnings)]` makes every warning an error for everyone who compiles the crate itself, with whatever compiler they have. Compilers add warnings in every release. So a commit that was fine when it was written stops building later, for people who changed nothing: a contributor with a newer compiler, a packager, a project that uses the crate as a path dependency. (Cargo caps lints for crates that come from a registry or from git, so their users are spared. The workspace's own members, and everyone who builds from a checkout, are not.) "Warnings are errors" is a rule about what may be *merged*, so it belongs where merging is decided: in the CI command (`cargo clippy ... -- -D warnings`), together with a pinned toolchain so that the set of warnings only changes when the team raises the version on purpose.

</details>

## Lesson 04 · Testing at scale

<details><summary>Hint 1</summary>

`render_report`: sort a `Vec<&Suite>` by name, compute the width of the first column from the names plus `"suite"` and `"total"`, then one `format!` per row: `{name:<width$} {passed:>6} {failed:>6} {ignored:>7}`. `redact`: replace the root first (`str::replace`), then walk the characters; at a digit that does not follow a letter, digit or dot, try to read "digits, optional `.digits`, unit". `Scratch`: hold a `tempfile::TempDir`. `word_counts`: collect into a `BTreeMap`, then sort by count with a *stable* sort. `is_expired_at`: `now.duration_since(created)` returns `Err` when the clock went back.

</details>
<details><summary>Hint 2</summary>

In `redact`, check the two-letter units before `s`, and reject the match when the character after the unit is alphanumeric (`3steps`, `10seconds`). `µ` is one `char` but two bytes, so work on a `Vec<char>` or use `char_indices`. Doc tests (task C): `to_fahrenheit` is the real function name and 100 °C is 212 °F; the `?` example needs `# Ok::<(), ParseError>(())` as its last line; the mix-up example is a block opened with three back-ticks and `compile_fail,E0308`. `shipping_cases`: one case exactly on each boundary (500 g, 2000 g), one just above 2000 g that is not a whole kilogram, one with several kilograms, one express, one express and heavy.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Line coverage counts which lines ran while the tests ran. It says nothing about whether any assertion depended on what those lines computed. A test can run a line and not check its result, or check it only for an input where a wrong version gives the same answer (3000 g hides a rounding bug that 2001 g shows). A surviving mutant is direct evidence of that: the code was changed, the behaviour changed, and no test failed. So coverage answers "what did we not even run?", and mutation testing answers "would we notice?". Neither proves correctness: mutants are small syntactic changes, and some survivors are equivalent to the original.

</details>

## Lesson 05 · Fuzzing and sanitizers

<details><summary>Hint 1</summary>

The decoder's bug is in the line that takes `&text[i + 1..i + 3]`: a string slice must begin and end on character boundaries, and the two bytes after a `%` need not be whole characters. Work with `text.as_bytes()` and look at single bytes. The encoder's bug is one byte in the "leave as it is" list that the decoder treats specially. For `Arbitrary`: `u.int_in_range(0..=4)?` for the number of pairs, and `u.choose(&ALPHABET)?` for each character.

</details>
<details><summary>Hint 2</summary>

A hex digit: `char::from(byte).to_digit(16)`. Read both digits with `bytes.get(i + 1)` and `bytes.get(i + 2)`; if either is missing or not hex, return `Err(DecodeError::BadEscape(i))`. Finish with `String::from_utf8(out).map_err(|_| DecodeError::InvalidUtf8)`. `fuzz_roundtrip` is three lines: `format_query(query, codec.encode)`, `parse_query(&text, codec.decode)`, and `assert_eq!(parsed.as_ref(), Ok(query), ...)`. Predictions: `Unstructured` never fails for lack of bytes; it pads with zeros, and integers are read little-endian.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A coverage-guided fuzzer instruments the program, so it can see which branches each input takes. It keeps inputs that reach new branches and mutates them further, which lets it work its way into the input format without being told what it is. It reports only what the process does: a panic, an abort, a hang, running out of memory. A wrong result is none of those. `decode("+")` returning a space is perfectly calm. A round-trip check turns "wrong" into "panic": encode, decode, compare with the original, and assert. Now the same search that finds crashes finds logic errors, as long as the generator can produce the inputs that matter (here, a literal `+`).

</details>

## Lesson 06 · Debugging

<details><summary>Hint 1</summary>

`capture_panic` needs two things: `std::panic::catch_unwind(f)` to stop the panic, and a panic hook to learn its message and location. The hard part is that the hook is global. Do not install and remove a hook per call: two threads doing that at the same time undo each other. Install one hook once (`std::sync::Once`), and let it decide per thread what to do, using a `thread_local!`. For the stockroom: run it with `RUST_BACKTRACE=1`, find the frame in `ship`, and ask which value there is wrong and who computed it.

</details>
<details><summary>Hint 2</summary>

Keep a thread-local `RefCell<Option<Option<PanicReport>>>`: the outer `Option` says "this thread is inside `capture_panic`", the inner one holds the report. The hook: if the outer option is `None`, call the previous hook (saved with `take_hook` before `set_hook`) and return; otherwise build the report from `info.payload_as_str()`, `info.location()` and `std::thread::current().name()`, and store it. `capture_panic`: save the slot's old value and set it to `Some(None)`, run `catch_unwind(f)`, put the old value back (this makes nesting work), and return the report on `Err`. Stockroom: `index` maps each SKU to a *position* in `items`. `drop_discontinued` removes an item, so every later position moves down by one, and the map is not updated. Rebuild the index after the `retain`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The stockroom fails in `ship`, at `self.items[position]`, with an index that is out of bounds. Nothing is wrong on that line: for a correct index it does the right thing. The program went wrong earlier, in `drop_discontinued`, which removed an element from `items` and left `index` pointing at the old positions. Between the two points the damage was silent: the order for gears was taken from the hinges. A fix at the failure site (a bounds check, a saturating subtraction) would remove the panic and keep the wrong stock numbers. Walking back from the symptom means asking, at each step, "which value here is wrong, and where was it produced?", until you reach code that turns right inputs into wrong outputs.

</details>

## Lesson 07 · Documentation

<details><summary>Hint 1</summary>

Run `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` in the fixture and fix what it reports; then `cargo check --all-features` for `missing_docs`. An intra-doc link is the item's path in back-ticks inside square brackets: ``[`Slug`]``, ``[`Slug::new`]``. The crate documentation comes from the README through `#![doc = include_str!("../README.md")]` as the first line of `lib.rs`; the README then needs a fenced `rust` block that really compiles against the crate (`use slugs::{Slug, slugify};`).

</details>
<details><summary>Hint 2</summary>

docs.rs: a table `[package.metadata.docs.rs]` with `all-features = true` and `rustdoc-args = ["--cfg", "docsrs"]`, and `#![cfg_attr(docsrs, feature(doc_cfg))]` in `lib.rs`. The example is four lines: `fn main() { println!("{}", slugs::slugify("The Professional Toolbox")); }`. The changelog headings are exactly `## [Unreleased]`, `## [0.2.0] - 2026-10-05`, `## [0.1.0] - 2026-08-17`, with `### Added` and `### Fixed` groups and `- ` entries; the facts are in the starter README's "History" paragraph. Predictions: a block tagged `text` is not Rust, so it is not a test; rustdoc only documents what is public and compiled.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The README is read by someone deciding whether to use the crate, on crates.io or the repository page, before they have it: it must say what the crate does, show one example, and name features and requirements. The crate-level docs are read by someone starting to use it; with `include_str!` they are the same text, and its example is tested. An item's docs are read at the moment of calling that item, often in an editor tooltip, so the first line must stand alone and the rest states the contract: errors, panics, an example. An example program is read and copied by someone who wants a working starting point. The changelog is read by someone upgrading, who asks only "what changed for me, and do I have to do anything?".

</details>

## Lesson 08 · Supply chain and policy

<details><summary>Hint 1</summary>

`locked_packages`: `toml::from_str::<toml::Table>(lock)`, then the array under `"package"`; each entry has `name` and `version` strings. Parse versions with `semver::Version::parse` and sort the `Locked` values (the derive orders by name, then version). `duplicates` groups equal names. `vulnerable` and `resolve` both come down to `VersionReq::parse(text)?.matches(&version)`. For task E, run `cargo clippy` in `fixtures/l08_policy`: the warning names the method that is too new.

</details>
<details><summary>Hint 2</summary>

`duplicates`: walk the sorted list and push into the last group when the name is the same; keep the groups with more than one version. `vulnerable`: for each package and each advisory with the same package name, the package is safe if any `patched` or any `unaffected` requirement matches; skip requirements that do not parse (`filter_map(... .ok())`), so that a broken requirement can never mark a version as safe. `resolve`: filter the versions that match, then `.max()` or `.min()`. Task E: `as_chunks::<4>()` became stable in 1.88; `chunks_exact(4)` with `.remainder()` has been there since 1.31, and `<[u8; 4]>::try_from(word)` turns each chunk into an array. `deny.toml`: see the section "Policy with cargo-deny" in CONCEPTS for the four tables.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Cargo.lock` records the exact version and checksum of every crate in the graph. With it, two builds of the same commit use the same dependency code, and a new release of a dependency cannot change your build until you run `cargo update`. That protects against surprise breakage and against a registry entry being altered after the fact. It does not say that the locked code is good: a vulnerability found tomorrow is in your lock file today, which is why the lock is compared against the advisory database on a schedule. It also does not travel with a library to its users; their lock is resolved from your version *requirements*. The lock changes when a person decides to update, in a commit that can be reviewed, and CI enforces that with `--locked`.

</details>

## Lesson 09 · CI and release

<details><summary>Hint 1</summary>

Write the workflow from the top: `name`, `on` with `pull_request` and `push`, `concurrency`, `permissions`, an `env` entry that holds `1.99.0` once, then jobs. Each job starts with `actions/checkout@v7` (the current major tag when this was written), installs Rust with `rustup toolchain install "$RUST_VERSION" --profile minimal ...` and `rustup default`, and adds a cache action. The table of checks in CONCEPTS is the list of `run:` lines. For the package: `cargo package --list --allow-dirty` in the fixture shows what would be uploaded.

</details>
<details><summary>Hint 2</summary>

The test job: `strategy: { fail-fast: false, matrix: { os: [ubuntu-24.04, macos-15, windows-2025] } }` and `runs-on: ${{ matrix.os }}`. The docs step needs `env: RUSTDOCFLAGS: -D warnings` on the step. On the Windows runner the default shell is PowerShell, which does not read `"$RUST_VERSION"` as an environment variable: give the install step of the test job `shell: bash`, or write `${{ env.RUST_VERSION }}`. The MSRV job's key contains `msrv` and installs `1.85.0` (written out; the test counts how often `1.99.0` appears). If you would rather pin 1.99.0 in a `rust-toolchain.toml` in the fixture, the workflow names no version at all and runs `rustup toolchain install`; the MSRV job then needs `cargo +1.85.0 check --locked`, because the file beats `rustup default`. `deny.toml`: four short tables as in lesson 08; `[licenses] allow = ["MIT"]` is enough for a crate without dependencies. Package: `include = ["/src/**", "/README.md", "/LICENSE", "/CHANGELOG.md"]`, plus `description`, `readme`, `repository`. (An `exclude` list works too, but it must also name `.cargo/` and every file you add later.) Profile: `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`. Linker: a table `[target.aarch64-unknown-linux-gnu]` with `linker = "aarch64-linux-gnu-gcc"` in `.cargo/config.toml`. Predictions: the fixture's `[profile.release]` sets `overflow-checks = true`; and with `panic = "abort"` the *first* panic ends the process, including the one inside `catch_unwind`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A build is reproducible when the same commit gives the same output wherever and whenever it is built. Everything that feeds the build and is not in the commit must be fixed: the compiler (an exact toolchain version, here in one `env` entry), the dependencies (`Cargo.lock` plus `--locked`), the build settings (the profile in the manifest), the machine image (`ubuntu-24.04`, not `ubuntu-latest`), and the actions (version tags or commit hashes). What remains outside control after this lesson: the contents of a tag like `v7` can still move, the runner image receives patches, and absolute paths of the build machine can leak into the binary unless they are remapped. Caches must be keyed on the lock file and toolchain, so they speed the build up without changing its result.

</details>

## Lesson 10 · Capstone

<details><summary>Hint 1</summary>

Follow the order at the end of CONCEPTS. Lessons 01, 03, 08 and 09 gave you files you can adapt: the workspace root, the lint tables, `deny.toml`, the workflow. The new work is the parser. Split a line with `split_whitespace()` and match on a slice pattern `[date, category, amount]`; validate the date byte by byte (never slice a `&str` at fixed positions); parse the amount as "digits, optional dot, one or two digits" with `split_once('.')`, and use `checked_mul`/`checked_add` so a huge number is an error. Return `Result` everywhere, and let `main` map errors to exit codes with `std::process::ExitCode`.

</details>
<details><summary>Hint 2</summary>

Line numbers come from `text.lines().enumerate()`; implement `Display` for the error as `line {n}: {reason}`. For the report, a `BTreeMap<&str, u128>` gives sorted categories; compute both column widths before formatting. The sums are `u128` because three amounts that each fit in 64 bits can add up to more: `u128::from(entry.cents)`. JSON: write the string escaping yourself (`"` and `\`), behind `#[cfg(feature = "json")]`. The binary's `json` feature forwards with `json = ["tally-core/json"]`; check `cfg!(feature = "json")` before reading the file, so `--json` without the feature exits with 2. After adding `insta` under `[workspace.dependencies]` and `[dev-dependencies]`, run `cargo generate-lockfile` in `capstone/` and accept the first snapshot with `INSTA_UPDATE=always cargo test` or `cargo insta review`, after reading it. With `wildcards = "deny"` in `deny.toml`, add `allow-wildcard-paths = true` for the path dependency between your crates. The fuzz project is a copy of this package's `fuzz/` with one target. In the workflow you do not have to repeat the toolchain version: `rustup toolchain install` without a name installs what your `rust-toolchain.toml` pins.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

There is no single right order, but a defensible one is: tests for the behaviour first, because every later change (lints, refactoring, features) needs something that says "still correct"; then CI that runs them with a pinned toolchain and `--locked`, because checks that only run on one laptop decay; then the lint policy, because it stops new problems from entering while the old ones are cleaned up gradually. Documentation, the changelog and the dependency policy matter as soon as other people use or contribute to the project, and fuzzing matters as soon as the program reads input it does not control. What the capstone shows is that most of this is cheap once and expensive never: a few files that, afterwards, do the checking for you.

</details>

## Mastery check: model answers

<details><summary>1. Features across a workspace</summary>

`cargo build -p one-member` builds that member and its dependencies; the shared crate gets exactly the features this member asked for. `cargo build --workspace` builds all members in one go, and the shared crate is compiled once with the union of all requested features. Because any consumer can end up with features it never asked for, a feature may only add things. If enabling a feature removes an item or changes behaviour, some crate that did not ask for it breaks, and only in some build configurations.

</details>
<details><summary>2. A new lint arrives</summary>

For users nothing happens: Cargo builds dependencies with lints capped, and your source does not turn warnings into errors (`forbid(unsafe_code)` is about one lint you chose, not about "all warnings"). In your CI, what happens depends on the toolchain. If it is pinned, nothing changes until someone raises the version in a pull request, where the new lint shows up and is fixed. If CI uses `stable`, it turns red on release day with no commit to blame. The deciding facts are where `-D warnings` lives (the CI command, not the source) and whether the toolchain is pinned.

</details>
<details><summary>3. Bugs behind 100% coverage</summary>

First, the line ran but no assertion depends on its result (a test that calls the function and checks only that it returns, or uses `>` where it should use `==`). Second, the assertions are exact, but the chosen inputs do not distinguish the right code from a wrong version: a boundary that is never hit exactly, a round number that hides a rounding error, two conditions never tested together. Mutation testing (cargo-mutants, or hand-written mutants as in lesson 04 and E05) finds both, because both let a changed program pass. Property tests and fuzzing attack the second from the other side, by generating the inputs you did not think of.

</details>
<details><summary>4. A silent fuzzer</summary>

(1) The target turns the failures you care about into panics: it checks a property, not just "does not crash". (2) The inputs can reach the interesting code: coverage is growing, most inputs are not rejected at the first check, and for structured input the `Arbitrary` impl can produce the special cases. (3) The harness itself can fail: you have seen it report a seeded bug, or it found the old known bug. Also worth checking: it ran with a sanitizer if `unsafe` code is involved, and the corpus was kept, so the hour was not spent rediscovering the format.

</details>
<details><summary>5. An out-of-bounds panic</summary>

Read the whole message and get a backtrace (`RUST_BACKTRACE=1`): find the first frame in your code. There, look at the values: which one is wrong, the index or the length? Use `dbg!` or a debugger for that. Then find where the wrong value was produced, and repeat the question there, until you reach code that produces a wrong value from right ones. The mistake to avoid is patching the failure site (a bounds check, `get`, a default value), which removes the report and keeps the damage.

</details>
<details><summary>6. The lock file</summary>

It guarantees that the same commit builds with the same versions of all dependencies, with checksums, until someone deliberately updates. It does not guarantee that those versions are free of known problems, and it does not apply to users of a library, who resolve their own. Everyone should commit it: applications because it defines what ships, libraries because it makes their own CI repeatable. `--locked` makes Cargo fail when the lock file would need to change, instead of quietly changing it.

</details>
<details><summary>7. Breaking the MSRV promise</summary>

Your own code starts using a newer standard-library item or language feature. Clippy's `incompatible_msrv` catches library items; only a build with the MSRV toolchain catches everything. Or a dependency raises *its* `rust-version` in a release your requirement allows; again only an MSRV CI job (`cargo check --locked` on that toolchain) notices, and resolver 3 helps by preferring versions that fit your `rust-version`. Normal tests pass in both cases, because they run on a new compiler.

</details>
<details><summary>8. What changes without a commit</summary>

The compiler and its lints: pin an exact toolchain version. Dependencies: commit `Cargo.lock`, use `--locked`. The runner image: name a version (`ubuntu-24.04`). Actions: a version tag, or a commit hash. Tools installed in CI (cargo-deny, nextest): install a named version. The advisory database: this one *should* change, so run that check on a schedule and keep it apart from the checks that gate a pull request. Time, network and randomness in tests: remove them (lesson 04).

</details>
