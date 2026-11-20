# M7 labs — real tools, real numbers

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

The tests of this module count things, so that they give the same answer on every machine. The labs are the other half: here you use the tools that performance work is done with, on your machine, and you look at times. No test depends on a lab. Do each lab when its lesson tells you to, and write what you saw in [NOTES.md](NOTES.md).

Every command was run with the course toolchain (Rust 1.99.0) on one machine: an Apple silicon laptop with macOS. The numbers that are quoted come from that machine and are labelled as such. Yours will differ; the *pattern* should not. Where a tool could not be checked on that machine, the text says so.

Run the commands from the root of the course repository.

| Lab | With lesson | Tool | Needs an install |
| --- | --- | --- | --- |
| [Lab 1](#lab-1--a-benchmark-that-measures-nothing) | 01 | criterion, `rustc --emit=llvm-ir` | no |
| [Lab 2](#lab-2--record-and-read-a-real-profile) | 02 | samply or cargo-flamegraph | yes |
| [Lab 4](#lab-4--an-array-of-structs-against-a-struct-of-arrays) | 04 | criterion | no |
| [Lab 5](#lab-5--traversal-order-in-time-and-bounds-checks-in-the-compilers-output) | 05 | criterion, `rustc --emit=llvm-ir` | no |
| [Lab 6](#lab-6--how-many-copies-of-a-generic-function) | 06 | `rustc --emit=llvm-ir`; optionally cargo-llvm-lines | no |
| [Lab 8](#lab-8--the-capstone-in-numbers) | 08 | criterion, the usage example, a profiler | no |

## Before you measure anything

A time that you measure is the time of your code plus everything else the machine did in that moment. Three habits remove most of the trouble:

1. **Close what you can.** Browsers, builds and video calls use the processor and its caches. On a laptop, plug in the power supply.
2. **Measure the optimized build.** `cargo bench` does that for you. `cargo run` and `cargo test` do not, unless you add `--release`.
3. **Run it twice before you believe it.** If two runs of the same code differ by 5%, a change of 5% means nothing.

To see what an unquiet machine does, here is a real result. The author ran the same benchmark twice, without changing a line, while other programs were busy on the same machine. Criterion compared the second run with the first:

```text
l01_digit_sum/opaque_input
                        time:   [15.016 ns 15.430 ns 15.941 ns]
                        change: [+67.274% +71.125% +76.369%] (p = 0.00 < 0.05)
                        Performance has regressed.
```

Nothing had regressed. The statistics are correct about the samples; the samples were taken on a machine that was doing something else.

**How to read a criterion line.** `time: [low estimate high]` is a confidence interval for the time of one call: criterion is 95% sure that the true mean lies between `low` and `high`, and `estimate` is its best guess. On a second run, `change:` is the same kind of interval for the difference from the last run, and the sentence below it says whether that difference is larger than the noise. `ps`, `ns`, `µs`, `ms` are picoseconds, nanoseconds, microseconds, milliseconds: each is 1,000 times the one before.

Useful options go after `--`:

```sh
# Shorter runs while you explore (less exact).
cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l01_black_box -- --warm-up-time 1 --measurement-time 2
# Only benchmarks whose name contains a text.
cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l01_black_box -- digit_sum
# Keep a named reference point, and compare with it later.
cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l08_capstone -- --save-baseline before
cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l08_capstone -- --baseline before
```

Criterion stores its data in the `criterion/` directory of the package's target directory: `mastery/m7_perf/target/criterion/`, unless you have set `CARGO_TARGET_DIR` or `build.target-dir`. Delete that directory to start again. The same holds for the paths `mastery/m7_perf/target/...` in Labs 2 and 8.

## Lab 1 · A benchmark that measures nothing

**Goal:** see with your own eyes what the optimizer does to a careless benchmark, and what `std::hint::black_box` changes.

**Predict first.** Open [`benches/l01_black_box.rs`](benches/l01_black_box.rs). It has two groups with two benchmarks each. Before you run anything, write in NOTES.md for each group: how many times faster or slower do you expect the second benchmark to be?

**Run.**

```sh
cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l01_black_box
```

On the author's machine:

```text
l01_digit_sum/opaque_input        time:   [8.8680 ns 8.8906 ns 8.9207 ns]
l01_digit_sum/constant_input      time:   [220.01 ps 220.86 ps 221.89 ps]
l01_sum_of_squares/result_used    time:   [157.90 ns 159.69 ns 162.04 ns]
l01_sum_of_squares/result_discarded
                                  time:   [230.07 ps 231.44 ps 232.84 ps]
```

(The lines are shortened: criterion prints each name on its own line.)

A fifth of a nanosecond is less than one clock cycle of the processor. No loop over 19 digits, and no loop over 1,000 numbers, runs in that time. In both "fast" benchmarks the work is gone, for two different reasons:

- `constant_input`: the compiler can see the input, so it ran the loop while compiling. The program contains the answer, not the loop.
- `result_discarded`: nobody uses the sum, and the loop does nothing else, so the compiler removed the loop.

**Look at the proof.** You do not have to trust the timing. [`labs/fold.rs`](labs/fold.rs) holds the same `digit_sum` and two callers. Ask the compiler for its intermediate code, which is readable text:

```sh
rustc --edition 2024 -O --crate-type=lib --emit=llvm-ir -o fold.ll mastery/m7_perf/labs/fold.rs
```

Open `fold.ll` and find the function whose name contains `known_input`. Its whole body is:

```text
  ret i32 90
```

It returns 90, the sum of the digits, and computes nothing. The function whose name contains `opaque_input` still has the loop: look for `udiv` (the division by 10). Delete `fold.ll` afterwards.

The two callers carry `#[inline(never)]`. Without it you would not find them in the file: in an optimized build, this compiler does not emit a small library function that calls nothing as a function of its own. It keeps the body ready to be pasted into callers in other crates, as if you had written `#[inline]`. Lesson 06 explains these attributes.

**Change it yourself.** In the benchmark file, wrap `&values` of `result_discarded` in nothing at all (remove its `black_box`). Does the time change? Then make the closure return the sum again, and remove only the `black_box` around the input of `result_used`. What do you measure, and why is the answer different from `constant_input`? (The values come from a `Vec` that is built when the program runs.) Restore the file afterwards.

**Write a benchmark yourself.** So far you have only run benchmarks. Add a third group, `l01_sort`, to `benches/l01_black_box.rs`. It measures `sort_unstable` on 10,000 `u32` values in a scrambled order (for example `(0..10_000u32).map(|i| i * 7_919 % 10_000)`), in two ways:

1. `same_vector_every_time`: make one vector before the benchmark and sort that same vector inside `b.iter`.
2. `fresh_copy_every_time`: give every call a new scrambled copy with [`Bencher::iter_batched`](https://docs.rs/criterion/0.8.2/criterion/struct.Bencher.html#method.iter_batched). Its first closure makes the input and is not timed; its second closure takes the input, sorts it and returns it, so that dropping the vector is not timed either. The third argument is `BatchSize::SmallInput`. This is the function you built as task B of lesson 01.

Add your function to the `criterion_group!` line and run only the new group: `cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l01_black_box -- l01_sort`. Predict the ratio of the two times before you look.

<details><summary>A model answer, and what the author measured</summary>

```rust
// Add `BatchSize` to the `use criterion::{...}` line at the top.
fn fresh_input(c: &mut Criterion) {
    // The numbers 0 to 9,999 in a scrambled order.
    let scrambled: Vec<u32> = (0..10_000u32).map(|i| i * 7_919 % 10_000).collect();
    let mut group = c.benchmark_group("l01_sort");

    // Wrong: the first call sorts `data`. Every later call sorts sorted data.
    let mut data = scrambled.clone();
    group.bench_function("same_vector_every_time", |b| {
        b.iter(|| black_box(&mut data).sort_unstable())
    });

    // Right: every call gets a fresh scrambled copy. Making the copy and
    // dropping the sorted vector are outside the timed region.
    group.bench_function("fresh_copy_every_time", |b| {
        b.iter_batched(
            || scrambled.clone(),
            |mut values| {
                values.sort_unstable();
                values
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, known_input, unused_result, fresh_input);
```

```text
l01_sort/same_vector_every_time   time:   [2.8591 µs 3.0295 µs 3.2623 µs]
l01_sort/fresh_copy_every_time    time:   [60.436 µs 63.618 µs 67.632 µs]
```

The first benchmark is twenty times "faster", and it measures something else: after its first call the vector is sorted, and the standard library's sort notices sorted input and stops after one pass. Nothing was removed by the optimizer here, and `black_box` does not help. The benchmark destroyed its own input. Whenever the routine changes its input, each call needs a fresh one.

</details>

**Write in NOTES.md:** in one sentence each, what `black_box` on an input prevents, and what `black_box` on a result prevents; and the two times of your own benchmark, with the reason for the gap.

## Lab 2 · Record and read a real profile

**Goal:** find out where a real program spends its time, without changing its code. The program is the slow baseline of the capstone, so what you learn here is the first step of lesson 08.

**Predict first.** Read `baseline::summarize` in [`src/l08_capstone.rs`](src/l08_capstone.rs). It reads the text, splits it into lines, splits lines into fields, looks up the service, sorts the latencies and formats a report. Write in NOTES.md which of these steps you expect to take the most time, and roughly what share.

**Install a sampling profiler.** Either one is enough.

- [samply](https://github.com/mstange/samply) works on macOS, Linux and Windows and shows the result in the Firefox Profiler in your browser:

  ```sh
  cargo install samply --locked
  ```

  The author used version 0.13.1 on macOS, where it needs no special rights. On Linux, samply reads the kernel's performance counters and may ask you to lower `kernel.perf_event_paranoid`; it prints the exact command. On Windows it needs an administrator terminal. These two were not checked on the author's machine.

- [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph) draws a flamegraph as an SVG file:

  ```sh
  cargo install flamegraph --locked
  ```

  It uses `perf` on Linux and `dtrace` on macOS, which needs `sudo` there. It was not checked on the author's machine.

**Build with debug information.** The `profiling` profile in `Cargo.toml` is the release profile plus full debug information, so that the profiler can name functions and lines:

```sh
cargo build --profile profiling --manifest-path mastery/m7_perf/Cargo.toml --example profile_target
```

**Record.**

```sh
# samply: opens the browser when the program has ended
samply record mastery/m7_perf/target/profiling/examples/profile_target baseline

# or cargo-flamegraph: writes flamegraph.svg
cargo flamegraph --profile profiling --manifest-path mastery/m7_perf/Cargo.toml --example profile_target -- baseline
```

On Windows the program is `profile_target.exe`. The program summarizes a generated log of 4 MB one hundred times, which takes a few seconds.

**Read.** In the Firefox Profiler, the *Call Tree* tab shows total time ("Running Time") and self time ("Self") per function; the *Flame Graph* tab shows the same data as a picture. Switch on "Invert call stack" to list the functions by self time. In a flamegraph, the width of a box is the total time of that function, and the part of a box that has nothing above it is its self time.

Write in NOTES.md:

1. the three entries with the largest self time, with their shares;
2. the function of *this package* with the largest total time below `baseline::summarize`;
3. the share of the sort that `baseline::top_k` does.

For comparison, this is what the author's recording showed (2,632 samples; your shares will differ by a few points, and the names of the allocator's functions depend on the operating system):

| Share of samples | Where |
| --- | --- |
| about 40% self | inside the system allocator (`malloc` and `free`) |
| about 20% self, 40% total | the closure that turns the fields of a line into `String`s |
| about 46% total | collecting those `String`s into the `Vec` of fields |
| about 17% total | collecting the lines into a `Vec<String>` |
| about 7% self | `memchr`, which searches for the line breaks |
| about 2% self | sorting the latencies |

Two things to notice. The largest single cost is not in any function that the program's author wrote: it is the allocator, called from many places. A profile sorted by self time shows this at once; reading the source does not. And the sort, which looks expensive in the source, hardly matters.

**Then:** do tasks A and B of lesson 02. There you build a profiler that counts work instead of sampling time, and you will see the same two views, self and total, in exact numbers.

## Lab 4 · An array of structs against a struct of arrays

**Goal:** see what the layout of lesson 04, task D, is worth in time. Do this after task D; before that the benchmark stops at `todo!()`.

```sh
cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l04_layout
```

Both benchmarks add up `price_cents * quantity` over the open ones of 500,000 orders. The array of structs is 24 MB; the three columns that the sum reads are 4.5 MB together.

On the author's machine, with the model solution:

```text
l04_open_value/array_of_structs   time:   [259.16 µs 270.02 µs 285.66 µs]
l04_open_value/struct_of_arrays   time:   [194.29 µs 195.63 µs 197.12 µs]
```

A gain of about a quarter, and far less than the factor of five in bytes. **Write in NOTES.md** why the two factors differ. (Think about what the processor does when it notices that a program reads memory in a straight line, and about which part of the work has nothing to do with memory.) Then change `ORDERS` in the benchmark to 5,000, so that everything fits into the innermost cache, and compare again.

## Lab 5 · Traversal order in time, and bounds checks in the compiler's output

**Part 1: traversal.** Do this after tasks A and B of lesson 05.

```sh
cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l05_traversal
```

On the author's machine, with the model solution, for a grid of 2,048 by 2,048 `u32` (16 MiB):

```text
l05_column_sums/column_by_column  time:   [9.4003 ms 10.773 ms 11.804 ms]
l05_column_sums/yours             time:   [1.4563 ms 1.7824 ms 2.3478 ms]
l05_transpose/plain_loops         time:   [17.852 ms 18.193 ms 18.715 ms]
l05_transpose/yours               time:   [6.1216 ms 6.9680 ms 8.4178 ms]
```

The same additions in another order: six times faster. The same moves in blocks: more than twice as fast. **Write in NOTES.md** how these factors compare with the miss counts of the cache model in the tests (16 times fewer misses for the sums, 8 times fewer for the transpose), and name two things that a real processor has and the model has not.

**Part 2: bounds checks.** [`labs/bounds.rs`](labs/bounds.rs) holds three functions that do the same work: they add two slices element by element and store the sums. Before you compile, read them and predict for each: does the optimized code still contain a call of the bounds-check panic?

```sh
rustc --edition 2024 -O --crate-type=lib --emit=llvm-ir -o bounds.ll mastery/m7_perf/labs/bounds.rs
```

Open `bounds.ll`. Each function starts at a line that begins with `define` and contains its name, and ends at a line that is only `}`. In front of every call, the compiler writes a comment line with the readable name of the function that is called. Inside each of the three functions, count the comment lines for these two:

- `; call core::panicking::panic_bounds_check`: the panic of an index that is out of range (`a[i]`);
- `; call core::slice::index::slice_index_fail`: the panic of a range that is out of range (`&b[..n]`).

(The line after such a comment is the call instruction itself, with the name in its mangled form. Count each pair once. The two names also appear once more at the end of the file, in lines that begin with `declare`; those are not calls.)

With the course toolchain the author found the following, the same for the macOS, Linux (`x86_64-unknown-linux-gnu`) and Windows (`x86_64-pc-windows-msvc`) targets:

| Function | Calls of `panic_bounds_check` | Calls of `slice_index_fail` |
| --- | --- | --- |
| `add_indexed` | 2 | 0 |
| `add_resliced` | 0 | 2, both in front of the loop |
| `add_zipped` | 0 | 0 |

`add_indexed` must keep its checks: the loop stores a result before it tries the next index, so a panic in the middle would leave visible results behind, and the compiler may not move the panic to the front. (In a loop that only adds numbers up, this compiler does move the check out of the loop by itself. That is the kind of proof you cannot rely on: it depends on details.) `add_resliced` pays two checks once. `add_zipped` has nothing that can panic, and a different meaning: a slice that is too short silently gives fewer results.

**Part 3: what the remaining checks cost.** Do not stop at "two checks are left". Look inside `add_indexed` for a block with the label `vector.body`. It loads, adds and stores values of the type `<2 x i64>`: two numbers per instruction. The compiler has split the loop. In front of it, two calls of `llvm.umin` work out how many rounds are in range for all three slices. Those rounds run in `vector.body` without any check. Only the rounds that are left go through the loop with the two panics. All three functions have a `vector.body` (the same on the three targets named above). So in this function the checks cost a few instructions per call, not one comparison per element.

This is the usual state of affairs with a current compiler: a bounds check in a simple loop over slices is nearly free. The checks that stay in every round are the ones in complicated loops: an index that comes from data, several slices whose lengths the compiler cannot relate, a call that was not inlined. That is why the rule is to measure before you rewrite a loop for its checks, and why the first reason to prefer `zip` or a sub-slice is that the code says what it means.

If you want to see machine code instead of the compiler's intermediate code, replace `--emit=llvm-ir -o bounds.ll` by `--emit=asm -o bounds.s`. The panics are then the targets of `bl` instructions (ARM) or `call` instructions (x86), and the additions of two numbers at a time are `add.2d` (ARM) or `paddq` (x86). Delete `bounds.ll` and `bounds.s` afterwards.

**Write in NOTES.md:** for which of the three would you add a length check of your own, and where? What did you expect the checks in `add_indexed` to cost before part 3, and what do they cost?

## Lab 6 · How many copies of a generic function?

**Goal:** see monomorphization in the compiler's output. [`labs/mono.rs`](labs/mono.rs) holds the same function three times: generic, with a trait object, and as a thin generic shell around a core that is not generic. Each one is called with six types. Predict how many copies of each body the compiled code contains.

```sh
rustc --edition 2024 -O --crate-type=lib --emit=llvm-ir -C codegen-units=1 -o mono.ll mastery/m7_perf/labs/mono.rs
```

Every function in `mono.ll` starts with a line that begins with `define`. Count the `define` lines that contain `framed_generic`, `framed_dyn` and `framed_shell`. On Linux and macOS:

```sh
grep -c '^define.*framed_generic' mono.ll
grep -c '^define.*framed_dyn' mono.ll
grep -c '^define.*framed_shell' mono.ll
```

In PowerShell, use `(Select-String -Path mono.ll -Pattern '^define.*framed_generic').Count`.

With the course toolchain: 6, 1 and 1. The generic function exists six times. The trait-object version exists once. The shell version also has one `define`: the core. The six shells are so small that the compiler pasted them into their callers. Delete `mono.ll` afterwards.

**Optional, with an install.** [cargo-llvm-lines](https://github.com/dtolnay/cargo-llvm-lines) counts lines of intermediate code per generic function over a whole package, and [cargo-bloat](https://github.com/RazrFalcon/cargo-bloat) lists the largest functions of a finished program. Neither was checked on the author's machine:

```sh
cargo install cargo-llvm-lines cargo-bloat --locked
cargo llvm-lines --manifest-path mastery/m7_perf/Cargo.toml --lib --release | head -20
cargo bloat --manifest-path mastery/m7_perf/Cargo.toml --release --example profile_target -n 20
```

**Write in NOTES.md:** a function of 40 lines that is generic over a `Display` value and called with 30 types. Which of the three shapes do you choose if it is called once per request, and which if it is called in the innermost loop?

## Lab 8 · The capstone in numbers

Use this lab while you work on lesson 08, not after it. The order matters: measure, then change.

**1. Counts.** The usage example prints what the baseline and your version ask of the heap:

```sh
cargo run --release --manifest-path mastery/m7_perf/Cargo.toml --example usage
```

Before you have changed anything, both rows are equal. With the model solution:

```text
version       lines  allocations    bytes asked   peak bytes
baseline      10000        59980        2881274      1091689
yours         10000           21          18569        18531
baseline      40000       239395       11520129      4364015
yours         40000           21          18569        18531
```

Look at how each column grows when the log is four times longer.

**2. Time.** Save the starting point once, then compare after every change:

```sh
cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l08_capstone -- --save-baseline start
# ... change one thing, run the tests ...
cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l08_capstone -- --baseline start
```

On the author's machine, with the model solution:

```text
l08_summarize/baseline  time:   [15.340 ms 17.559 ms 20.332 ms]
                        thrpt:  [47.738 MiB/s 55.278 MiB/s 63.276 MiB/s]
l08_summarize/yours     time:   [3.8265 ms 3.9836 ms 4.1877 ms]
                        thrpt:  [231.78 MiB/s 243.66 MiB/s 253.66 MiB/s]
l08_top_k/baseline      time:   [971.24 µs 1.1460 ms 1.3546 ms]
l08_top_k/yours         time:   [107.19 µs 108.14 µs 109.15 µs]
```

`thrpt` is throughput: bytes of log per second.

**3. Profile.** Record your own version the way you recorded the baseline in Lab 2:

```sh
cargo build --profile profiling --manifest-path mastery/m7_perf/Cargo.toml --example profile_target
samply record mastery/m7_perf/target/profiling/examples/profile_target yours
```

When no single entry stands out any more, you are done.

**4. Optional: build settings.** Everything so far changed the source. The build has settings too, and they are measured in the same way. Add these lines to `mastery/m7_perf/Cargo.toml`, run the comparison again, and remove them afterwards:

```toml
[profile.bench]
lto = "fat"          # optimize across crate boundaries, the standard library included
codegen-units = 1    # one unit per crate: slower to build, more to inline
```

On the author's machine, which was busy with other work during these runs, `l08_top_k/yours` went from about 130 µs to about 51 µs and `l08_summarize/yours` from about 4.8 ms to between 3.0 ms and 3.6 ms. The price is build time: with these settings the linker step optimizes the whole program again. Do not carry these numbers over to another program: the gain depends on how much of the hot code crosses a crate boundary. Measure. [The Cargo book](https://doc.rust-lang.org/cargo/reference/profiles.html) lists the settings.

**5. Report.** Write a short performance report in NOTES.md. A report that somebody else can check has these parts:

- what was measured: the input, the command, the machine;
- the numbers before and after, counts and times, with the spread of the times;
- each change, in the order you made it, with the gain it gave;
- what you tried that did not help;
- what you left alone, and why.
