# M9 labs

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md)

A lab is a task that needs a tool this package cannot ship. Every lab has the same shape: **Run** (the exact command), **Expect** (what you should see), **Check** (how you know you are done). All commands are run from the repository root unless a lab says otherwise.

The outputs shown were produced with the tool versions listed in the [README](README.md#tool-versions), on macOS on Apple silicon. Paths are shortened. Hexadecimal addresses, process numbers and timings will differ on your machine.

Four labs end with a recorded answer that a test checks:

```sh
cargo test --manifest-path mastery/m9_toolbox/Cargo.toml lab_ -- --ignored
```

## Setup

The lessons and their tests install nothing, never run git, and write only inside this package's `target/` directory and the system's temporary directory. The labs are different: they need tools, and installing a tool changes your machine outside this repository. Read this section before you install anything.

**What the commands below change, and how to undo it**

| Command | Where it writes | Undo |
| --- | --- | --- |
| `cargo install <tool>` | the binary goes to `~/.cargo/bin`, for your whole user account | `cargo uninstall <tool>` |
| `rustup toolchain install nightly-2026-11-20` (and `1.85.0` in lab 8.3) | a toolchain under `~/.rustup/toolchains` | `rustup toolchain uninstall nightly-2026-11-20` |
| `rustup component add ...`, `rustup target add ...` (lab 9.2) | inside the named toolchain under `~/.rustup` | `rustup component remove ...`, `rustup target remove ...` |
| `cargo audit`, `cargo deny` (labs 8.1, 8.2) | a copy of the advisory database under `~/.cargo/advisory-db` and `~/.cargo/advisory-dbs` | delete those directories |
| `cargo vet` (lab 8.5) | a cache in your user cache directory (`~/Library/Caches/cargo-vet` on macOS, `~/.cache/cargo-vet` on Linux) | delete that directory |

If you do not want the tools in `~/.cargo/bin`, give `cargo install` a directory of its own and put its `bin` on your `PATH` for the session:

```sh
cargo install --locked --root "$HOME/m9-tools" cargo-nextest@0.9.152
export PATH="$HOME/m9-tools/bin:$PATH"
```

The author's check of this module installed every tool that way. Everything else a lab writes stays in the repository, in places that the course's `.gitignore` files cover (`target/`, `fuzz/corpus/`, `fuzz/artifacts/`, `mutants.out/`, `*.snap.new`), or in a scratch directory that the lab tells you to create outside the repository.

Install only what you need for the lab you are doing. `--locked` builds the tool with the dependency versions its authors tested. `name@version` pins the tool itself.

```sh
cargo install --locked cargo-expand@1.0.127
cargo install --locked cargo-nextest@0.9.152
cargo install --locked cargo-insta@1.49.0
cargo install --locked cargo-llvm-cov@0.9.1
cargo install --locked cargo-mutants@27.1.0
cargo install --locked cargo-fuzz@0.13.2
cargo install --locked cargo-audit@0.23.0
cargo install --locked cargo-deny@0.22.0
cargo install --locked cargo-vet@0.10.3

# Labs 5.x, 7.1 and 8.4 need a nightly toolchain; lab 5.4 also needs its rust-src.
rustup toolchain install nightly-2026-11-20 --profile minimal --component rust-src
# Lab 4.3 needs LLVM's coverage tools for the pinned toolchain.
rustup component add llvm-tools --toolchain 1.99.0
```

This directory pins Rust 1.99.0, so nightly commands are written `cargo +nightly-2026-11-20 ...`.

---

## Lesson 01

### Lab 1.1 · What takes the time, and who needs this crate?

**Run**

```sh
cargo build --manifest-path mastery/m9_toolbox/Cargo.toml --timings
cargo tree  --manifest-path mastery/m9_toolbox/Cargo.toml --depth 1
cargo tree  --manifest-path mastery/m9_toolbox/Cargo.toml -e features -i arbitrary
```

**Expect** The build prints `Timing report saved to .../target/cargo-timings/cargo-timing-<time>.html`. Open the file: one bar per crate, in the order they were built. The last command prints:

```text
arbitrary v1.5.0
└── arbitrary feature "default"
    └── m9_toolbox v0.0.0 (/path/to/mastery/m9_toolbox)
        └── m9_toolbox feature "default" (command-line)
```

**Check** You can name the crate with the longest bar, and say which crates could not start before it finished. You can say which features of `arbitrary` this package enables, and find the line in `Cargo.toml` that decides it.

### Lab 1.2 · Patch and vendor a dependency (needs network access)

Work in a scratch directory outside the repository.

**Run**

```sh
cargo new patchlab && cd patchlab
cargo add semver@1.0.20 itoa@1
cargo tree
cargo vendor vendor
cp -R vendor/itoa itoa-patched
```

Add to `Cargo.toml`:

```toml
[patch.crates-io]
itoa = { path = "itoa-patched" }
```

```sh
cargo tree
```

**Expect** `cargo vendor` ends by printing the `[source.crates-io]` and `[source.vendored-sources]` settings that would make Cargo build from `vendor/`. After the patch, the tree shows the path:

```text
patchlab v0.1.0 (/work/patchlab)
├── itoa v1.0.19 (/work/patchlab/itoa-patched)
└── semver v1.0.28
```

**Check** Look at the `itoa` entry in `Cargo.lock`: it has no `source` line any more. Say why a patch must keep the crate's name and a version that the requirement `"1"` accepts.

---

## Lesson 02

### Lab 2.1 · Ask rustup why

**Run**

```sh
cd mastery/m9_toolbox
rustup show active-toolchain
RUSTUP_TOOLCHAIN=nightly rustup show active-toolchain
RUSTUP_TOOLCHAIN=nightly cargo +1.99.0 --version
```

**Expect** (your platform name will differ)

```text
1.99.0-aarch64-apple-darwin (overridden by '/path/to/mastery/m9_toolbox/rust-toolchain.toml')
nightly-aarch64-apple-darwin (overridden by environment variable RUSTUP_TOOLCHAIN)
cargo 1.99.0 (5f94df478 2026-08-27)
```

**Check** Each line agrees with what your `active_toolchain` function returns for the same situation. `rustup override list` prints `no overrides`, or you know why each entry is there.

### Lab 2.2 · cargo expand

**Run**

```sh
cargo expand --manifest-path mastery/m9_toolbox/fixtures/l02_expand/Cargo.toml --lib
```

**Expect** The same text as `fixtures/l02_expand/expanded.rs`, starting with `#![feature(prelude_import)]`.

**Check** Find the three things the predictions asked about: the order of comparisons in `partial_cmp`, the bound in `impl<T: ::core::clone::Clone>`, and the two `x` in `bonus_demo`. Notice what is missing from the output: the `macro_rules!` definition itself.

### Lab 2.3 · rust-analyzer in your editor

This lab was not run by the author in an editor. The setting names in lesson 02 were checked against rust-analyzer's source (crate `ra_ap_rust-analyzer` 0.0.358).

**Run** Open the folder `mastery/m9_toolbox` (the folder, not a file) in an editor with rust-analyzer. In `fixtures/l02_expand/src/lib.rs`, put the cursor on `with_bonus!(x)` and run the command "Expand macro recursively". Then go to the definition of `Rc` in the same file.

**Expect** A view with the expansion `{ let x = 100; x + x }`. Go-to-definition opens `rc.rs` from the standard library. If it does not, the `rust-src` component is missing: `rustup component add rust-src`.

**Check** Turn inlay type hints on and off, and find the setting that controls them. Decide which of your rust-analyzer settings belong in the repository and which are yours alone.

---

## Lesson 04

### Lab 4.1 · cargo-nextest

**Run**

```sh
cargo nextest run --manifest-path mastery/m9_toolbox/Cargo.toml l04_
```

**Expect** One line per test, each run in its own process, then a summary. With lesson 04 solved:

```text
        PASS [   0.009s] ( 1/14) m9_toolbox l04_testing::tests::a_report_edge_cases
        ...
        PASS [   0.373s] (14/14) m9_toolbox l04_testing::tests::p_predictions
────────────
     Summary [   0.373s] 14 tests run: 14 passed, 81 skipped
```

**Check** Find the slowest test of the lesson in the output and say why it is the slowest. Say what nextest does not run (`cargo test --doc` is still needed).

### Lab 4.2 · Review a snapshot change

**Run** In `src/l04_testing.rs`, temporarily change the last line of the report, for example to `result: FAILED (2 of 157 failed)`. Then:

```sh
cargo insta test --manifest-path mastery/m9_toolbox/Cargo.toml -- a_report_matches
cargo insta pending-snapshots --manifest-path mastery/m9_toolbox/Cargo.toml
```

**Expect**

```text
stored new snapshot mastery/m9_toolbox/src/snapshots/m9_toolbox__l04_testing__tests__report.snap.new
info: 1 snapshot to review
use `cargo insta review` to review snapshots
Pending snapshots:
  src/snapshots/m9_toolbox__l04_testing__tests__report.snap
```

`cargo insta review --manifest-path mastery/m9_toolbox/Cargo.toml` shows the diff and asks whether to accept, reject or skip.

**Check** Reject the change (`r` in the review, or `cargo insta reject --manifest-path mastery/m9_toolbox/Cargo.toml`), undo your edit, and confirm that no `.snap.new` file is left and `cargo test ... l04_` is green. In this module the snapshot is the specification, so rejecting is the right answer. In your own project you would accept a change you intended, and commit the `.snap` file with the code change.

### Lab 4.3 · Coverage

**Run**

```sh
cargo llvm-cov --manifest-path mastery/m9_toolbox/fixtures/l04_coverage/Cargo.toml
cargo llvm-cov --manifest-path mastery/m9_toolbox/fixtures/l04_coverage/Cargo.toml --text
```

**Expect** The table ends with `100.00%` in the Lines column. The `--text` report shows a hit count before every line:

```text
    9|      5|pub fn shipping_cents(weight_g: u32, express: bool) -> u32 {
   10|      5|    let base = if weight_g <= 500 {
   11|      3|        300
```

**Check** Every line of `shipping_cents` has a count above zero. Read the four tests in the fixture and decide, before the next lab, whether you trust them.

### Lab 4.4 · Mutation testing

**Run**

```sh
cargo mutants -d mastery/m9_toolbox/fixtures/l04_coverage
```

**Expect** A list of mutants marked `MISSED`, and a last line of the form `12 mutants tested in 4s: N missed, M caught`. The details are in `fixtures/l04_coverage/mutants.out/`.

**Check**

1. Record the number of missed mutants in the test `lab_l04_mutants` (`src/l04_testing.rs`) and run the lab tests.
2. Improve the tests in `fixtures/l04_coverage/src/lib.rs` until the same command reports `12 mutants tested ...: 12 caught`. Coverage was 100% before and is 100% after. Say what changed.

---

## Lesson 05

Run the fuzzing labs from the package directory, because `cargo fuzz` looks for `fuzz/` there:

```sh
cd mastery/m9_toolbox
```

### Lab 5.1 · Find and minimise a crash

Needs task A (`decode`) to compile; it fuzzes the shipped decoder `buggy::decode`.

**Run**

```sh
cargo +nightly-2026-11-20 fuzz run decode
```

**Expect** Within seconds, a panic and a saved input:

```text
thread '<unnamed>' panicked at src/l05_fuzz.rs:71:36:
end byte index 6 is not a char boundary; it is inside '\u{6dc}' (bytes 5..7 of string)
...
Failing input:

	fuzz/artifacts/decode/crash-80cf75330e636cfa6ee7409d31352122e748481e

Output of `std::fmt::Debug`:

	[2, 0, 0, 37, 36, 219, 156, 8]
```

Your bytes and file name will be different; the fuzzer starts from a random seed. Now minimise, with the path it printed:

```sh
cargo +nightly-2026-11-20 fuzz tmin decode fuzz/artifacts/decode/crash-<your hash>
```

```text
CRASH_MIN: failed to minimize beyond fuzz/artifacts/decode/minimized-from-... (4 bytes), exiting
Output of `std::fmt::Debug`:

	[37, 36, 219, 156]
```

**Check** Paste your minimised bytes into `lab_decode_crash()` in `src/l05_fuzz.rs` and run the lab tests. Say in one sentence what the four bytes have in common with every other crashing input. Then run `cargo +nightly-2026-11-20 fuzz run decode` again after you change the target to use `FIXED`: it should keep running (stop it with Ctrl-C, or pass `-- -max_total_time=30`).

### Lab 5.2 · Find a wrong answer

Needs tasks B and C (`Arbitrary for Query`, `fuzz_roundtrip`).

**Run**

```sh
cargo +nightly-2026-11-20 fuzz run roundtrip
```

**Expect**

```text
assertion `left == right` failed: round trip changed the query; encoded text was "+aa=&="
  left: Ok(Query { pairs: [(" aa", ""), ("", "")] })
 right: Ok(Query { pairs: [("+aa", ""), ("", "")] })
```

(The message is the one from your own `fuzz_roundtrip`.) The decode target can run for hours without finding this, because nothing crashes.

**Check** Put the smallest value that shows the bug into `lab_roundtrip_counterexample()` and run the lab tests.

### Lab 5.3 · AddressSanitizer

Replace the target triple with your own (`rustc -vV` prints it as `host`). ASan works on Linux and macOS (x86-64 and ARM64).

**Run**

```sh
cargo run --manifest-path mastery/m9_toolbox/fixtures/l05_sanitizers/Cargo.toml --bin overread
RUSTFLAGS=-Zsanitizer=address cargo +nightly-2026-11-20 run \
  --manifest-path mastery/m9_toolbox/fixtures/l05_sanitizers/Cargo.toml \
  --target aarch64-apple-darwin --bin overread
```

**Expect** The first command prints `total:` and some number, and exits with 0. The second stops the program:

```text
==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x602000000980 ...
READ of size 4 at 0x602000000980 thread T0
    #0 ... in ..._8overread9sum_first overread.rs:11
    #1 ... in ..._8overread4main overread.rs:19
0x602000000980 is located 0 bytes after 16-byte region [0x602000000970,0x602000000980)
```

**Check** From the report alone, name the line that reads out of bounds, the size of the buffer in bytes, and the line that allocated it. Fix the caller and run both commands again.

### Lab 5.4 · ThreadSanitizer

TSan must also instrument the standard library, so it is rebuilt (`-Zbuild-std`; this takes a minute).

**Run**

```sh
RUSTFLAGS=-Zsanitizer=thread cargo +nightly-2026-11-20 run -Zbuild-std \
  --manifest-path mastery/m9_toolbox/fixtures/l05_sanitizers/Cargo.toml \
  --target aarch64-apple-darwin --bin race
```

**Expect**

```text
WARNING: ThreadSanitizer: data race (pid=...)
  Read of size 8 at 0x000109e00710 by thread T2:
    #0 ..._4race4bump race.rs:17
  Previous write of size 8 at 0x000109e00710 by thread T1:
    #0 ..._4race4bump race.rs:17
  Location is heap block of size 8 at 0x000109e00710 allocated by main thread:
...
SUMMARY: ThreadSanitizer: data race race.rs:17 in ..._4race4bump
```

Without `-Zbuild-std` the compiler refuses: ``error: mixing `-Zsanitizer` will cause an ABI mismatch in crate `race` ``.

**Check** Replace the raw pointer with an `AtomicU64` (or a `Mutex<u64>`) and run again: no warning, and `total: 200000` every time. The type system would have forced this without `unsafe`; say which line of the program switched that protection off.

---

## Lesson 06

### Lab 6.1 · From the panic to the cause, in a debugger

Do this **before** you repair `src/bin/stockroom.rs`. If you have repaired it already, copy your version away and put the starter text back for the lab; do not use a git command that would also throw away your other work. On Linux, `rust-gdb` works the same way; the GDB commands are given in brackets.

**Run**

```sh
cargo build --manifest-path mastery/m9_toolbox/Cargo.toml --bin stockroom
rust-lldb mastery/m9_toolbox/target/debug/stockroom
```

```text
(lldb) breakpoint set --name rust_panic        [gdb: break rust_panic]
(lldb) run
(lldb) bt                                      [gdb: bt]
```

**Expect** The program stops inside the panic machinery. The backtrace has the runtime's frames first, then yours:

```text
    frame #9:  ... <alloc::vec::Vec<stockroom::Item> as core::ops::index::IndexMut<usize>>::index_mut(self=size=4, index=4) at mod.rs:3943:9
    frame #10: ... <stockroom::Stockroom>::ship(self={items:size=4, index:size=5}, sku="hinge", quantity=20) at stockroom.rs:47:35
    frame #11: ... stockroom::main at stockroom.rs:78:19
```

Select the frame of `ship` (its number may differ) and look around:

```text
(lldb) frame select 10                          [gdb: frame 10]
(lldb) frame variable sku quantity position     [gdb: info locals]
(lldb) frame variable self->items               [gdb: print self.items]
(lldb) frame variable self->index               [gdb: print self.index]
```

```text
(unsigned long) position = 4
(alloc::vec::Vec<stockroom::Item, alloc::alloc::Global>) self->items = size=4 {
  [0] = {sku:"anvil", quantity:2, discontinued:false}
  ...
}
```

**Check**

1. The backtrace line of `ship` already shows two numbers that do not fit together. Which?
2. Print `self->items[3]` and record its `quantity` in the test `lab_l06_what_the_debugger_shows` (`tests/l06_process.rs`). Run the lab tests.
3. That quantity is wrong, and no panic reported it. Which earlier order damaged it? Now you know where to repair.

---

## Lesson 07

### Lab 7.1 · See your documentation as docs.rs will

**Run**

```sh
cargo doc --manifest-path mastery/m9_toolbox/fixtures/l07_docs/Cargo.toml --no-deps --all-features --open
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly-2026-11-20 doc \
  --manifest-path mastery/m9_toolbox/fixtures/l07_docs/Cargo.toml --no-deps --all-features --open
```

**Expect** The first command opens the crate page, with your README as its text. The page of `title_case` looks like any other. After the second command (nightly, with the `docsrs` cfg), the same page carries the label "Available on **crate feature `titles`** only."

**Check** Click from `slugify` to `Slug` and to `Slug::new`: the links work. Remove `--all-features` and reload: `title_case` is gone. That is what docs.rs would show without `[package.metadata.docs.rs]`.

---

## Lesson 08

### Lab 8.1 · cargo-audit

**Run**

```sh
cargo audit --file mastery/m9_toolbox/Cargo.lock
```

**Expect**

```text
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 1550 security advisories (from ~/.cargo/advisory-db)
    Scanning mastery/m9_toolbox/Cargo.lock for vulnerabilities (43 crate dependencies)
```

(plus a line `Updating crates.io index`) and exit code 0. The number of advisories grows over time. If a finding appears for one of the pinned crates, that is the tool working: read the advisory, then see what its "Solution" line says.

**Check** Explain what your task B function `vulnerable` does that this tool also does, and one thing the tool does that yours does not.

### Lab 8.2 · cargo-deny with your policy

**Run**

```sh
cargo deny --manifest-path mastery/m9_toolbox/Cargo.toml \
  --config mastery/m9_toolbox/fixtures/l08_policy/deny.toml check
```

**Expect** `advisories ok, bans ok, licenses ok, sources ok`, possibly with warnings.

**Check** Remove `MIT` from your allow list and run `... check licenses`. The report names every MIT-only crate in the graph and the path from this package to it. Put `MIT` back.

### Lab 8.3 · The real MSRV test

**Run** With the starter version of `fixtures/l08_policy/src/lib.rs` (before task E):

```sh
rustup toolchain install 1.85.0 --profile minimal
cargo +1.85.0 check --manifest-path mastery/m9_toolbox/fixtures/l08_policy/Cargo.toml
```

**Expect** A compile error at the call of `as_chunks`: on 1.85 that method is not stable yet.

```text
error[E0658]: use of unstable library feature `slice_as_chunks`
 --> src/lib.rs:6:31
  |
6 |     let (words, rest) = bytes.as_chunks::<4>();
  |                               ^^^^^^^^^
  |
  = note: see issue #74985 <https://github.com/rust-lang/rust/issues/74985> for more information
```

**Check** After task E the same command succeeds. This command, in a CI job, is the test of the `rust-version` promise. Remove the toolchain again with `rustup toolchain uninstall 1.85.0` if you do not need it.

### Lab 8.4 · Honest lower bounds (needs network access)

In the scratch project of lab 1.2:

**Run**

```sh
cargo +nightly-2026-11-20 update -Z minimal-versions
cargo tree
```

**Expect**

```text
 Downgrading semver v1.0.28 -> v1.0.20 (available: v1.0.28)
```

and the tree shows `semver v1.0.20`.

**Check** `cargo test` still passes with that lock file. If it did not, the requirement in `Cargo.toml` would be lower than what the code needs. Run `cargo update` to return to the newest versions.

### Lab 8.5 · cargo-vet at a glance

In the same scratch project:

**Run**

```sh
cargo vet init
cargo vet
```

**Expect** A new directory `supply-chain/` with `audits.toml`, `config.toml` and `imports.lock`, and the line `Vetting Succeeded (2 exempted)`.

**Check** Open `supply-chain/config.toml` and find the two exemptions. They mean "used, not audited". Say what would have to happen for the number to become 0.

---

## Lesson 09

### Lab 9.1 · Publish without publishing (needs network access)

In a scratch directory outside the repository. Pick a crate name of your own.

**Run**

```sh
cargo new --lib my-publish-lab && cd my-publish-lab
cargo package --list --allow-dirty
cargo publish --dry-run --allow-dirty
```

**Expect** A warning that the manifest has no description, licence or repository, then:

```text
   Packaging my-publish-lab v0.1.0 (/work/my-publish-lab)
    Packaged 4 files, ...
   Verifying my-publish-lab v0.1.0 (/work/my-publish-lab)
   Uploading my-publish-lab v0.1.0 (/work/my-publish-lab)
warning: aborting upload due to dry run
```

**Check** Add `description`, `license` and `repository` (an `https://example.com/...` address is enough for the dry run) until the warning is gone; after the first two it still asks for `documentation`, `homepage` or `repository`. Say what `--allow-dirty` switched off, and why a real release should not need it.

### Lab 9.2 · Another target

**Run**

```sh
rustc --print cfg --target x86_64-unknown-linux-musl
cargo build --manifest-path mastery/m9_toolbox/fixtures/l09_ship/Cargo.toml --target x86_64-unknown-linux-musl
```

**Expect** The first command lists the target's cfg values, among them `target_env="musl"`. The second fails unless you have installed that target:

```text
error[E0463]: can't find crate for `std`
  |
  = note: the `x86_64-unknown-linux-musl` target may not be installed
  = help: consider downloading the target with `rustup target add x86_64-unknown-linux-musl`
```

**Check** After `rustup target add x86_64-unknown-linux-musl` (it adds the target to the pinned 1.99.0 toolchain; `rustup target remove` takes it away again), the library compiles. Whether the binary links depends on your machine having a linker for that target. On macOS it does not: the build stops with ``error: linking with `cc` failed`` and `ld: unknown options: --as-needed -Bstatic ...`, because the Mac's linker cannot write Linux programs. That gap is what `cross` fills: `cross build --target x86_64-unknown-linux-musl` runs the build in a container that has one. (`cross` needs Docker or Podman, and was not run by the author.)
