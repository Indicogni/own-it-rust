# Taming compile times

[Course overview](../README.md) · [All guides](README.md)

Rust compiles slowly compared with Go or C, and a slow edit-compile-test loop makes learning harder. This page shows where the time goes and what you can change. Every command below was run, and every output is real, on one machine: an Apple silicon laptop with 15 cores, macOS, Rust 1.99.0. Your numbers will differ; the ratios are what matter. Two sections describe things that this machine could not run (they are marked).

The course itself is small enough that you rarely wait long. These habits matter more in the ecosystem track, the mastery modules with dependencies (M5, M9, M11) and the capstones, and in any real project afterwards.

## Measure first

Before you change anything, find out what is slow. Cargo can draw a timeline of a build:

```text
$ cargo build --timings
   ...
      Timing report saved to target/cargo-timings/cargo-timing-20261120T173021459Z-eab9109460269fa4.html
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.23s
```

The HTML report shows, for every crate, when it started, how long it took, and what it waited for. This is the same build, a small program with one `#[derive(Serialize)]`, read from the report's data:

```text
crate                              start    took
serde_core                         0.29s    0.77s
proc-macro2                        0.37s    0.18s
quote                              0.52s    0.09s
syn                                0.59s    0.60s
serde_derive                       1.19s    0.71s
serde                              1.90s    0.24s
derive (the program itself)        2.14s    0.09s
```

The program itself took 0.09 seconds. It could not start until 2.14 seconds, because it waited for a chain of crates: `proc-macro2`, `quote`, `syn`, `serde_derive`, then `serde`. Finding such a chain, the *critical path*, is the main use of `--timings`.

## `cargo check` while you write

`cargo check` runs the parser, the type checker and the borrow checker, and stops before code generation. Most compiler errors come from those stages, so `check` tells you almost everything that `build` does, in less time. On a generated library of 20 modules and 2,400 methods (10,820 lines):

```text
$ cargo check            # from a clean target directory
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.34s
$ cargo build            # code generation as well
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.96s
```

Editors run `cargo check` (through rust-analyzer) on every save for this reason. Run `cargo test` or `cargo build` when you need a program that runs. In this course, `cargo clippy` also does the work of `check`.

## Incremental compilation

In the `dev` profile Cargo compiles *incrementally*: it saves the compiler's intermediate results and reuses those that an edit did not change. After a one-line edit in one module of the same library:

```text
$ cargo check                          # after editing one function
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.39s
$ cargo build                          # after editing it back
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.42s
$ CARGO_INCREMENTAL=0 cargo build      # the same edit, incremental compilation off
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.76s
```

Incremental compilation is on by default for `dev` and off for `release`, where it would cost run-time speed. Keep it on for development. Note that it works inside one crate; a change to a crate makes every crate that depends on it compile again.

## Debug information

A debug build writes debug information, which takes time and disk space. If you rarely use a debugger, keep only line tables (enough for file and line numbers in panics and backtraces):

```toml
# Cargo.toml
[profile.dev]
debug = "line-tables-only"
```

On the same library, two clean builds of each setting:

```text
debug = true               2.17s  2.26s   23M libdemo.rlib
debug = "line-tables-only" 2.01s  1.98s   19M libdemo.rlib
debug = false              1.89s  1.90s   14M libdemo.rlib
```

You can try a setting without editing `Cargo.toml`: `CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo build`.

## Split a large crate into a workspace

The crate is the unit of compilation, of parallelism and of incremental reuse. A large crate compiles on fewer cores, and a change to one module re-checks the whole crate. Moving stable code into its own crate of a workspace helps both. Here the generated library became a crate `model`, used by a small binary crate `app`:

```text
$ cargo build                    # clean
   Compiling model v0.1.0
   Compiling app v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.20s
$ cargo build                    # after editing app/src/main.rs
   Compiling app v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
$ cargo build                    # after editing a module of model
   Compiling model v0.1.0
   Compiling app v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.60s
```

Split along lines that change at different speeds: a stable domain model, a slow-to-compile dependency wrapper, a binary that you edit all day. Do not split into dozens of tiny crates; each crate has a fixed cost, and a deep chain of crates is a critical path.

## Generics and monomorphisation

A generic function is compiled once for every concrete type it is used with. This is called *monomorphisation*, and it is why generic code is fast. It is also why it can be slow to compile: the compiler and LLVM see one copy per type. This program uses one generic function with twelve integer and character types (`one.rs` is the same program with only the first call):

```rust
use std::collections::BTreeMap;
use std::fmt::Debug;

// Counts the values and returns them as text, most frequent first.
fn summarise<T: Ord + Clone + Debug>(items: &[T]) -> String {
    let mut counts: BTreeMap<T, usize> = BTreeMap::new();
    for item in items {
        *counts.entry(item.clone()).or_default() += 1;
    }
    let mut pairs: Vec<(T, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    format!("{pairs:?}")
}

fn main() {
    println!("{}", summarise(&[u8::default(); 3]));
    println!("{}", summarise(&[u16::default(); 3]));
    println!("{}", summarise(&[u32::default(); 3]));
    println!("{}", summarise(&[u64::default(); 3]));
    println!("{}", summarise(&[i8::default(); 3]));
    println!("{}", summarise(&[i16::default(); 3]));
    println!("{}", summarise(&[i32::default(); 3]));
    println!("{}", summarise(&[i64::default(); 3]));
    println!("{}", summarise(&[usize::default(); 3]));
    println!("{}", summarise(&[isize::default(); 3]));
    println!("{}", summarise(&[char::default(); 3]));
    println!("{}", summarise(&[bool::default(); 3]));
}
```

Counting the functions in the LLVM IR that `rustc` produces, then timing an optimised build:

```text
$ rustc --edition 2024 --emit=llvm-ir -o one.ll one.rs; grep -c '^define' one.ll
218
$ rustc --edition 2024 --emit=llvm-ir -o twelve.ll twelve.rs; grep -c '^define' twelve.ll
1779
$ /usr/bin/time -p rustc --edition 2024 -C opt-level=3 -o one one.rs
real 0.18
$ /usr/bin/time -p rustc --edition 2024 -C opt-level=3 -o twelve twelve.rs
real 0.46
```

Eleven more types turned 218 functions into 1,779, because `BTreeMap`, `Vec`, sorting and formatting were all instantiated again for each type. In real code the same happens with large generic functions that are called with many types, and with deep stacks of generic adapters.

When a generic function is large, keep only a thin generic layer and move the work into a non-generic function. The standard library does this in many places:

```rust
use std::path::Path;

// Generic: compiled once per caller's type, but it is only one line.
pub fn line_count<P: AsRef<Path>>(path: P) -> std::io::Result<usize> {
    line_count_inner(path.as_ref())
}

// Not generic: compiled once, however many types the callers use.
fn line_count_inner(path: &Path) -> std::io::Result<usize> {
    let text = std::fs::read_to_string(path)?;
    Ok(text.lines().count())
}

fn main() {
    let missing = line_count("no/such/file.txt");
    assert!(missing.is_err());
}
```

A trait object (`&dyn Trait`) is the other way to compile code once: it trades a small run-time cost (an indirect call) for less code. Chapter 11 and mastery module M7 cover the trade-off.

## Procedural macros

A derive such as `#[derive(Serialize)]` is a procedural macro: a separate crate that must be compiled, with its own dependencies (usually `syn`, `quote` and `proc-macro2`), before any crate that uses it. The same program as above, once with `#[derive(Serialize)]` and once with the same `Serialize` implementation written by hand:

```text
with #[derive(Serialize)]             Finished ... in 2.28s / 2.30s
hand-written impl, no derive feature  Finished ... in 1.39s / 1.32s
```

That is about one second on a clean build, almost all of it the macro's dependency chain. You pay it once per clean build and once per change to the macro crates, not on every edit, so derives are usually worth it. Be careful with crates that pull in several different procedural macros, and check with `cargo tree -e features` which features enable them. Mastery module M5 shows what a procedural macro does.

## A shared compilation cache: sccache

[sccache](https://github.com/mozilla/sccache) wraps `rustc` and stores compiled crates in a cache, so a clean build, a second checkout or a CI job can reuse them. Install it (`cargo install sccache --locked`), then tell Cargo to use it:

```text
$ export RUSTC_WRAPPER=sccache
$ cargo build                  # empty cache
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.07s
$ rm -rf target && cargo build # a clean build with a filled cache
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.75s
$ sccache --show-stats
Compile requests                     40
Compile requests executed            18
Cache hits                            8
Cache misses                          8
...
```

This was sccache 0.19.0 on the serde program, whose clean build takes about 2.3 seconds without sccache. The first build was slower (the cache had to be filled), the second faster. sccache does not cache crates that are compiled incrementally, which is why your own workspace crates still compile; it helps most with dependencies. It is most useful in CI and when you often build several copies of the same project. You can also set it permanently in `~/.cargo/config.toml` under `[build]` as `rustc-wrapper = "sccache"`.

## The linker

Linking joins all compiled crates into one executable. For a large program with debug information it can take a noticeable part of every incremental build, because it runs after every change.

**Not run on this machine (Linux only).** Since Rust 1.90, the `x86_64-unknown-linux-gnu` target links with LLD, the LLVM linker shipped with Rust, instead of the system's default linker. The 1.99 release notes list no change to this. LLD is usually much faster than GNU ld. You can turn it off, for example to compare, with a flag that is stable only on that target:

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-Clinker-features=-lld"]
```

On other targets the flag is rejected; this is what Rust 1.99 prints on macOS:

```text
$ rustc -Clinker-features=-lld e.rs -o e
error: `-C linker-features=-lld` is unstable on the `aarch64-apple-darwin` target. The `-Z unstable-options` flag must also be passed to use it on this target
```

On other Linux targets the system linker is still the default. There you can choose a faster linker through the C compiler driver, for example `rustflags = ["-C", "link-arg=-fuse-ld=lld"]` (or `mold`, if it is installed). On macOS the system linker is already fast. On Windows with MSVC, `rust-lld` can be selected with `linker = "rust-lld"` in the `[target.x86_64-pc-windows-msvc]` table. Measure before and after; a different linker rarely changes `cargo check`, only `build` and `test`.

## Looking inside the compiler

**Nightly only.** Two unstable flags show where the compiler itself spends its time. They need a nightly toolchain; these runs used the course's dated nightly, `nightly-2026-11-20` (`rustc 1.102.0-nightly`, see [The nightly toolchain](../README.md#the-nightly-toolchain)). You do not need them for this course.

`-Ztime-passes` prints the time of each compiler stage. The largest entries for a clean, non-incremental build of the generated library:

```text
$ CARGO_INCREMENTAL=0 cargo +nightly-2026-11-20 rustc --lib -- -Ztime-passes
time:   1.496; rss:   27MB ->  375MB ( +348MB)	total
time:   0.410; rss:  155MB ->  251MB (  +96MB)	type_check_crate
time:   0.317; rss:  251MB ->  335MB (  +84MB)	MIR_checking
time:   0.271; rss:  417MB ->  507MB (  +91MB)	codegen_crate
time:   0.270; rss:  417MB ->  507MB (  +90MB)	codegen_to_LLVM_IR
time:   0.240; rss:  341MB ->  417MB (  +76MB)	generate_crate_metadata
time:   0.238; rss:  436MB ->  434MB (   -2MB)	LLVM_passes
time:   0.178; rss:  344MB ->  399MB (  +55MB)	monomorphization_collector_graph_walk
```

`-Zself-profile` records much more detail into a file, which the `summarize` and `crox` tools of the [measureme](https://github.com/rust-lang/measureme) project turn into a table or a Chrome trace:

```text
$ cargo +nightly-2026-11-20 rustc --lib -- -Zself-profile=profiles
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.77s
$ ls profiles
demo-0024225.mm_profdata
```

## A checklist

1. Use `cargo check` (or your editor) while you write, and `cargo test` when you need to run.
2. Leave incremental compilation on in the `dev` profile.
3. Run `cargo build --timings` once and look at the critical path.
4. Consider `debug = "line-tables-only"` if you seldom use a debugger.
5. Split a large crate along lines that change at different speeds.
6. Keep large generic functions thin; move the body into a non-generic function.
7. Know which procedural macros your dependencies pull in.
8. Use sccache in CI or across several checkouts.
9. On Linux, check which linker you use before you replace it.

## Read more

- [The Cargo book: build timings](https://doc.rust-lang.org/cargo/reference/timings.html)
- [The Cargo book: profiles](https://doc.rust-lang.org/cargo/reference/profiles.html), including `debug` and `incremental`
- [The Cargo book: workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [The rustc book: codegen options](https://doc.rust-lang.org/rustc/codegen-options/index.html), including `linker` and `linker-features`
- [The unstable book: self-profile](https://doc.rust-lang.org/unstable-book/compiler-flags/self-profile.html)
