# E08 · Tooling workday

[Track guide](README.md) · [Concepts](CONCEPTS.md#lesson-e08--tooling-workday) · [Lesson file](src/e08_tooling.rs) · [Hints](HINTS.md#e08--tooling-workday)

This is a normal day of Rust work that has nothing to do with writing features. You document an API, find out why the build pulls in a crate, tune lints, debug a macro, pick a release profile, time code honestly, and read a profile. Every task uses tools that ship with Cargo. Optional tools are marked with their install commands. Nothing here requires them.

Run all commands from the repository root. Code tasks A–D are in [`src/e08_tooling.rs`](src/e08_tooling.rs). Check them with:

```sh
cargo test --manifest-path ecosystem/Cargo.toml e08_
```

Write your answers to the questions in your own notes, then compare with [HINTS.md](HINTS.md#e08--tooling-workday).

## 1. Documentation and doc tests

```sh
cargo doc --manifest-path ecosystem/Cargo.toml --no-deps --open
RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path ecosystem/Cargo.toml --no-deps
cargo test --manifest-path ecosystem/Cargo.toml --doc
```

- Implement `parse_size` (task A). Then add an `# Examples` section to its doc comment with a doc test that calls it from outside the crate (`use ecosystem::e08_tooling::parse_size;`) and shows one success and one error.
- The second command fails on a broken intra-doc link in the same doc comment. Fix the link.
- Read the generated page for `e08_tooling`. Notice that `double!` appears at the crate root, not in its module (`#[macro_export]`).

Done when:

- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc ... --no-deps` succeeds.
- [ ] `cargo test --doc` lists your new doc test as passed. Its line starts with `test src/e08_tooling.rs - e08_tooling::parse_size`. (The other doc tests in the list are the code blocks of [CONCEPTS.md](CONCEPTS.md).)

Doc tests are compiled as separate crates that link against the library, so they only see the public API. That makes them the cheapest check that your examples still compile. `cargo test --all-targets` does **not** run doc tests; CI usually runs `cargo test --doc` separately.

## 2. Dependency graph and features

```sh
cargo tree --manifest-path ecosystem/Cargo.toml --depth 1
cargo tree --manifest-path ecosystem/Cargo.toml --duplicates
cargo tree --manifest-path ecosystem/Cargo.toml -e features -i tokio
cargo tree --manifest-path ecosystem/Cargo.toml -e features,normal -i tokio
cargo tree --manifest-path ecosystem/Cargo.toml -i regex-syntax
```

Answer:

The third and fourth commands print a long tree, because many crates depend on tokio. Read the lines that say `tokio feature "..."`, and the lines directly below each one, which name who asked for it.

1. `--duplicates` shows one crate in two major versions. Which crate, and which dependency pulls in each version? What does it cost you?
2. Which tokio features does this package ask for in `[dependencies]`, and which extra feature appears for `cargo test`? Where does it come from?
3. Who needs `regex-syntax`? Does it end up in the `jobq` binary?
4. In `Cargo.toml`, `tracing-subscriber` has `default-features = false`. Which features do we enable, and what would the defaults have added?
5. Besides this package, three crates ask for tokio's `net` feature. Which ones? How many copies of tokio does Cargo build for all of them, and with which features?

Done when:

- [ ] You can answer all five from the command output alone.

## 3. Lints configured in the manifest

`Cargo.toml` has a `[lints]` section, so every `cargo clippy` run (yours, your editor's, and CI's) uses the same rules:

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
dbg_macro = "warn"
```

```sh
cargo clippy --manifest-path ecosystem/Cargo.toml --all-targets
cargo clippy --manifest-path ecosystem/Cargo.toml --all-targets -- -D warnings
```

- Add `pedantic = { level = "warn", priority = -1 }` under `[lints.clippy]` and run clippy. (The group needs `priority = -1` so individual lint settings override it.) Expect several dozen new warnings: the group also fires on the supplied code and on the tests, not only on what you wrote. Read three of them. Do not try to fix them all.
- Now decide the end state. Remove the `pedantic` line again. In its place, add one pedantic lint that you agree with by name at `"warn"`, for example `cast_possible_truncation = "warn"`. Add one lint that you disagree with by name at `"allow"`, with a comment explaining why. This is how most teams use pedantic: as a menu to choose from, not as a group to switch on.
- Put a `dbg!(...)` somewhere in your E08 code and watch `dbg_macro` catch it. Remove it.
- Find one place where a lint is right in general but wrong here, and silence it locally with `#[expect(clippy::lint_name, reason = "...")]`. Unlike `allow`, `expect` warns once the lint no longer fires. If you chose `cast_possible_truncation`, there is such a place in E05: the `NoSplit` mutant casts with `n as u8` on purpose, because wrapping counts above 255 is the bug it models. Put the attribute directly above that expression.
- Try adding `unsafe { }` anywhere: `forbid` cannot be overridden by an inner `allow`.

Done when:

- [ ] `cargo clippy --all-targets -- -D warnings` passes with your final configuration (the two named lints, without the whole `pedantic` group). That flag is what CI typically runs.

## 4. Debugging a macro without cargo-expand

`double!(1 + 2)` should be 6 but is 5 (task B). Before you fix it, *look* at the expansion with stable-Rust techniques:

- **`stringify!`:** add a debugging arm that returns the generated tokens as text, for example `(@show $($e:tt)*) => { stringify!($($e)* * 2) };`. Put it **first**, above the existing arm: arms are tried in order, and the existing arm matches any tokens, `@show` included, so an arm after it is never reached. Then `println!("{}", double!(@show 1 + 2));` in a test or scratch program.
- **`compile_error!`:** make the arm expand to `compile_error!(stringify!(...))`. The compiler prints the tokens as an error message, at the call site.
- **Your editor:** rust-analyzer's "Expand macro recursively" command shows the full expansion, including derive and attribute macros.
- **Optional tools:** `cargo-expand` prints the whole crate after expansion. It uses the compiler's unstable `-Zunpretty=expanded` option, but current versions (checked with 1.0.127) enable it on the pinned stable toolchain themselves, so no nightly is needed:

  ```sh
  cargo install cargo-expand
  cargo expand --manifest-path ecosystem/Cargo.toml --lib e08_tooling
  ```

  The package has a library and a binary, so `--lib` is needed to say which one to expand.

Then fix the matcher. An `$e:expr` fragment is parsed once, as a whole expression, and stays one unit when it is substituted. A `tt` repetition is pasted as loose tokens.

One trap when you check your fix: `stringify!($e * 2)` with an `$e:expr` fragment still prints `1 + 2 * 2`. The fragment is one unit for the compiler, but that grouping has no token of its own, so the text looks like the bug. Trust the test, or `cargo expand`, which prints the parentheses.

For derive and attribute macros (`#[derive(Deserialize)]`, `#[tracing::instrument]`), the generated code is usually large. Read the crate's documentation of the attribute first. Expand only when behavior surprises you.

Done when:

- [ ] `b_double_respects_precedence` passes, and you can explain P1 in `p_predictions`.

## 5. Release profiles

Read `[profile.release]` and `[profile.profiling]` at the bottom of `Cargo.toml`, then:

```sh
cargo build --manifest-path ecosystem/Cargo.toml --bin jobq
cargo build --manifest-path ecosystem/Cargo.toml --release --bin jobq
ls -l ecosystem/target/debug/jobq ecosystem/target/release/jobq
cargo test --manifest-path ecosystem/Cargo.toml --release e08_tooling::tests::p_predictions
```

For each setting, write down what it trades. Try one at a time and compare build time and binary size:

| Setting | Try | Trade-off to note |
| --- | --- | --- |
| `opt-level` | `3` (release default), `"s"`, `"z"` | speed vs size |
| `debug` | `"line-tables-only"` (set here), `true`, `false` | profiler frame names vs size |
| `lto` | `"thin"`, `true` | runtime speed vs link time |
| `codegen-units` | `1` | speed vs parallel compile time |
| `panic` | `"abort"` | size and speed vs `catch_unwind` and per-task panic isolation (E06) in the shipped binary. `cargo test` always builds tests with unwinding, so the tests here will not show the difference. |
| `overflow-checks` | `true` | safety vs a little speed. Some teams enable it in release. |
| `strip` | `"symbols"` | size vs readable backtraces |

`cargo build --profile profiling` uses the custom profile: release speed plus full debug info for the profiler in task 7.

Done when:

- [ ] P2 passes in both profiles, and you can explain why it holds in both.
- [ ] You can say which two settings you would change for a CLI that ships to users, and why.

## 6. Timing without `cargo bench`

Implement `summarize` and `measure` (task C), then run the comparison example in both profiles:

```sh
cargo run --manifest-path ecosystem/Cargo.toml --release --example e08_bench
cargo run --manifest-path ecosystem/Cargo.toml --example e08_bench
```

`Instant` caveats to check in the output:

- **Profile:** debug numbers can be 10–50× slower, with different winners. Benchmark only `--release`.
- **Dead code:** without `black_box`, the optimizer may delete work whose result is unused, or hoist it out of the loop. That is how `x * 2` "takes 0 ns".
- **Warm-up:** first runs pay for page faults, cache misses, and lazy initialization.
- **Noise:** frequency scaling, other processes, and thermal limits. Report min and median over many samples, not one run or a mean distorted by outliers.
- **Resolution:** `Instant` has finite resolution and a cost per call. Time a batch of iterations when one call is very short.

For real benchmarks, use a harness as a dev-dependency in a scratch project: [criterion](https://crates.io/crates/criterion) (statistics, regression reports) or [divan](https://crates.io/crates/divan) (simple, fast). To time whole CLI runs, use `cargo install --locked hyperfine`.

Done when:

- [ ] The `c_` tests pass, and the release run shows a clear winner. You can explain *why* it wins, which is a complexity argument, not a microbenchmark one.

Going further: [M7, lesson 01 · Benchmark design](../mastery/m7_perf/CONCEPTS.md#lesson-01--benchmark-design) treats a benchmark as an experiment: warm-up, noise, the clock's limits and an optimizer that deletes work. Its [Lab 1](../mastery/m7_perf/LABS.md#lab-1--a-benchmark-that-measures-nothing) runs a real harness.

## 7. Reading a flamegraph

A sampling profiler records the call stack many times per second. A flamegraph draws those samples. The x-axis is **not** time: each box's *width* is the number of samples whose stack contains that frame (inclusive). Boxes stack upward by caller → callee. A box's *self* time is the part of its width with nothing on top. The text format behind it is "folded stacks", one `a;b;c COUNT` line per distinct stack. That format is what `PROFILE` in the lesson holds.

Implement task D, then answer from `PROFILE`:

1. Which frame is widest after `main`? What fraction of all samples is `log`?
2. Which frame has the most *self* samples? Why does it beat `parse_json`, even though `parse_json` has the biggest single self-time line?
3. If you could make one call free, would you pick `format` or `parse_json`? Compare the most samples each could save: self time shows where the CPU is, inclusive time shows what a call costs including its callees.

Optional: profile real code. Pick one tool:

```sh
# Linux needs `perf` (Debian/Ubuntu: sudo apt install linux-tools-common linux-tools-$(uname -r)).
# macOS uses dtrace and needs sudo.
cargo install flamegraph
cargo flamegraph --manifest-path ecosystem/Cargo.toml --profile profiling --example e08_bench

# Or the Firefox Profiler UI, Linux and macOS:
cargo install --locked samply
cargo build --manifest-path ecosystem/Cargo.toml --profile profiling --example e08_bench
samply record ecosystem/target/profiling/examples/e08_bench
```

Done when:

- [ ] The `d_` tests and P3 pass, and you have answered the three questions.
- [ ] (Optional) In a real flamegraph of `e08_bench`, you can find the frames for `join_with_format`'s copying.

Going further: [M7, lesson 02 · Reading a profile](../mastery/m7_perf/CONCEPTS.md#lesson-02--reading-a-profile) explains sampling profilers and what a profile can and cannot tell you. Its [Lab 2](../mastery/m7_perf/LABS.md#lab-2--record-and-read-a-real-profile) records and reads a real profile.

## More optional tools

| Tool | Install | Use it for |
| --- | --- | --- |
| cargo-mutants | `cargo install --locked cargo-mutants` | Automated version of E05's mutant tests: does any test fail when a line of your code changes? |
| cargo-nextest | `cargo install --locked cargo-nextest` | Faster test runner, one process per test (avoids E07's tracing cache race by design) |
| cargo-deny | `cargo install --locked cargo-deny` | Licenses, advisories, and duplicate versions in CI |
| cargo-bloat | `cargo install cargo-bloat` | What makes the binary big |
| cargo-outdated | `cargo install --locked cargo-outdated` | Which pinned versions are behind |
