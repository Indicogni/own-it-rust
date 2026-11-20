# Mastery M7 concepts — Performance engineering

[Module guide](README.md) · [Labs](LABS.md) · [Hints](HINTS.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every `rust` block in this file was compiled and
run with the course toolchain (Rust 1.99.0, edition 2024), and every compiler
and clippy message was copied from that toolchain. Long messages are
shortened: a line with `...` marks a cut. Times that are quoted come from one
machine and are labelled as such.

## What you will be able to do

After this module you can:

- design a benchmark that measures what you mean to measure, and explain a
  result that is too good to be true;
- read a profile: tell self cost from total cost, and decide what to change
  first;
- find and remove heap allocations, and keep them out with a budget test;
- predict the size of a type, remove padding, use niches, and choose between
  an array of structs and a struct of arrays;
- order loops so that they use the cache well, and write loops that need no
  bounds checks;
- choose between generics, trait objects and enums with their costs in mind;
- buffer input and output correctly, including the error cases;
- tune a real program in a measured loop, with a differential test as a safety
  net.

## Before you start

Chapters 1 to 18 and chapter 23 of this course. In
particular: ownership and borrowing (4, 5), structs and enums (9, 10), traits
and generics (11), `Drop` and `Cow` (13), closures (14), iterators (15),
collections (16), strings (17), `Read` and `Write` (18), and how to read a
`// SAFETY:` comment (23). The ecosystem track's tooling lesson (E08) is a
gentle start for lessons 01 and 02. Mastery M1 (lesson 05, trait objects)
helps with lesson 06, and M2 (unsafe) helps with the last part of lesson 05,
but neither is required.

## Time

These are estimates for a learner who has finished the chapters
above. They include reading, predictions, code, the lab and your notes.

| Lesson | Estimate |
| --- | --- |
| 01 · Benchmark design | 2 hours |
| 02 · Reading a profile | 2.5 hours |
| 03 · Allocation budgets | 2 hours |
| 04 · Data layout | 2 hours |
| 05 · Traversal and bounds checks | 3 hours |
| 06 · Dispatch, inlining and code size | 2.5 hours |
| 07 · Buffering | 3 hours |
| 08 · Capstone | 4 to 6 hours |
| C01 code review, drills | 1.5 hours |

## How this module tests performance

No test here reads a clock. A test that
says "this must take less than 5 ms" fails on a slow or busy machine and
passes on a fast one, whatever the code does. Instead, every test counts
something that causes time to be spent: heap allocations, bytes requested,
clock readings, comparisons, cache misses in a model, bounds checks, virtual
calls, calls of `read` and `write`. A count is the same on every machine and
in every run. A limit on a count is called a **budget**. The benchmarks in
`benches/` and the labs in [LABS.md](LABS.md) then show what the counts are
worth in time on your machine.

## Words used in this module

- **Hot**: executed very often. A hot loop is where a program spends its time.
  **Cold** is the opposite: code that rarely runs, such as error handling.
- **Optimizer**: the part of the compiler that rewrites your program into a
  faster one with the same observable behavior. It runs in release builds.
- **Allocation**: a request to the allocator for a block of heap memory.
- **Cache**: a small, fast memory inside the processor that holds copies of
  recently used parts of main memory. It works in blocks called **cache
  lines**. A line is 64 bytes on most current processors and 128 bytes on
  Apple silicon; this module calculates with 64. Using a line that is not in
  the cache is a **cache miss**.
- **System call**: a request from a program to the operating system, for
  example to write to a file. It costs far more than a function call.
- **Monomorphization**: the compiler makes one copy of a generic function for
  every set of types it is used with.

## Lesson 01 · Benchmark design

**Assumed:** closures (chapter 14) and `Instant` from the ecosystem tooling
lesson.

### The mental model

A benchmark is an experiment. You fix an input, run a piece of code many
times, and summarize how long it took. Two things work against you: the
machine and the compiler.

**The machine is noisy.** Other programs run. The processor changes its speed
with temperature and load. The first calls are slow because code and data are
not in the cache yet. And the clock itself is an instrument with limits: it
costs time to read, and it has a resolution. A call that takes 5 ns cannot be
timed with a clock reading that costs 20 ns.

A harness (the program that runs a benchmark) deals with this in four steps:

1. **Warm up**: run the code for a while and throw the timings away.
2. **Batch**: time many calls with one clock reading before and one after,
   and divide. The cost and the resolution of the clock are then spread over
   the whole batch. The harness doubles the batch size until a batch is long
   enough to time; this is called calibration.
3. **Sample**: time many batches. Each one gives a sample.
4. **Summarize robustly**: noise only ever adds time, and sometimes it adds a
   lot. The mean of the samples is pulled up by a single interrupted batch.
   The median, the middle sample, is not. For the spread, the median of the
   distances from the median (the median absolute deviation, MAD) is robust
   in the same way.

Two results differ only if the gap between them is clearly larger than their
spread. A harness that prints "3% faster" without a spread is guessing.

**The compiler is clever.** The optimizer may change your program in any way
that keeps its observable behavior. Two of its changes ruin benchmarks:

- **Constant folding.** If the input is known when the program is compiled,
  the compiler can run the computation itself and store only the result.
- **Dead code elimination.** If nobody uses a result and the computation has
  no other effect, the compiler can remove the computation.

Both are exactly what you want in real code, and exactly what you do not want
in a benchmark, where the input is usually a constant and the result is
thrown away. `std::hint::black_box` is the tool against both. It is a
function that returns its argument, and it asks the compiler to treat the
value as if it could be read and changed in ways the compiler cannot see.
Pass the *input* through it, and the compiler must build code that works for
any input. Pass the *result* through it, and the result counts as used.

`black_box` is a hint. The documentation says "best effort": it works with
today's compilers, but the language does not guarantee it, and it must never
be used to keep a secret from the optimizer in security code.

### Worked example

This program times a function by hand: calibration, samples, median. It is
the skeleton of every harness. (The exercise builds the same parts as
functions that take the clock as a parameter, so that tests can check them.)

```rust
use std::hint::black_box;
use std::time::{Duration, Instant};

/// How many steps the "3n + 1" rule needs to bring `n` down to 1.
fn collatz_steps(mut n: u64) -> u32 {
    let mut steps = 0;
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        steps += 1;
    }
    steps
}

/// Times `batch` calls with two clock readings.
fn time_batch(batch: u32) -> Duration {
    let start = Instant::now();
    for _ in 0..batch {
        // Input and result both pass through `black_box`.
        black_box(collatz_steps(black_box(27)));
    }
    start.elapsed()
}

fn main() {
    // Calibration: double the batch until it is long enough to time.
    let mut batch = 1;
    while time_batch(batch) < Duration::from_micros(100) && batch < 1 << 20 {
        batch *= 2;
    }
    // Samples: nanoseconds per call, one sample per batch.
    let mut samples: Vec<u128> = (0..15)
        .map(|_| time_batch(batch).as_nanos() / u128::from(batch))
        .collect();
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    println!("{median} ns per call (batches of {batch})");
    assert_eq!(collatz_steps(27), 111);
}
```

The calibration loop also served as the warm-up. A real harness does more:
it estimates the spread, detects outliers, and compares with the last run.
With criterion, the same benchmark is a few lines. This block needs the
criterion crate, so it is not run as part of this file; the complete,
compiled version of this pattern is `benches/l01_black_box.rs`.

```rust,ignore
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn steps(c: &mut Criterion) {
    // `iter` times the closure in batches and passes its result through
    // `black_box`; you only have to hide the input.
    c.bench_function("collatz_27", |b| b.iter(|| collatz_steps(black_box(27))));
}

criterion_group!(benches, steps);
criterion_main!(benches);
```

When each call needs a fresh input that is expensive to build (a vector to
sort, for example), building it must not be timed. Criterion has
`iter_batched` for this: it builds all inputs first, times only the calls,
and drops the results after the second clock reading. Task B of the exercise
is that function, and in Lab 1 you write a benchmark that needs it.

### Mistakes people really make

**A result that is too good to be true.** A function that loops over 1,000
elements cannot run in a fifth of a nanosecond. On the author's machine, the
two benchmarks of Lab 1 that forget a `black_box` reported exactly that:

```text
l01_sum_of_squares/result_used       time:   [157.90 ns 159.69 ns 162.04 ns]
l01_sum_of_squares/result_discarded  time:   [230.07 ps 231.44 ps 232.84 ps]
```

Whenever a time is below about one nanosecond, or does not grow when the
input grows, suspect that the work was removed.

**The benchmark never runs.** A criterion benchmark has its own `main`. If
its `[[bench]]` entry in `Cargo.toml` lacks `harness = false`, Cargo builds
the file with the standard test harness, which finds no tests in it:

```text
running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**Timing the wrong build.** `cargo test` and `cargo run` build without
optimization unless you pass `--release`. A debug build can be ten or more
times slower, and the ratio between two implementations is different too.
Only time optimized builds.

**Timing the setup.** If the closure builds its input and then processes it,
you measure both. Move the setup out of the closure, or use `iter_batched`.

**A routine that uses up its input.** A benchmark that sorts the same vector
in every call sorts scrambled data once and sorted data ever after. The
optimizer removed nothing, so `black_box` cannot help; every call needs a
fresh input (`iter_batched`). Lab 1 measures the gap.

**Believing one run.** On a busy machine, the author's second run of an
unchanged benchmark was reported as 71% slower (the output is in LABS.md).
Measure twice.

### When to use which

| You want | Use |
| --- | --- |
| to compare two implementations of a small function | criterion (or divan) |
| a rough number while exploring | `Instant` around a loop, with `black_box`, in a release build |
| a number that is the same on every machine, for a test | a count, as in this module |
| to know *where* a whole program spends time | a profiler (lesson 02), not a benchmark |

### Read more

- [`std::hint::black_box`](https://doc.rust-lang.org/std/hint/fn.black_box.html)
- [`std::time::Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html)
- [The Cargo book: `cargo bench`](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) and [the `harness` field](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-harness-field)
- [The Cargo book: profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)

## Lesson 02 · Reading a profile

**Assumed:** lesson 01; `Drop` (chapter 13); `Cell` and `RefCell` (chapter 9).

### The mental model

A benchmark tells you how long something takes. A **profile** tells you where
the time goes inside it. There are two ways to get one.

A **sampling profiler** interrupts the program many times per second and
notes the call stack each time: which function is running, who called it, who
called that one, and so on. A function that appears in 30% of the samples
uses about 30% of the time. Nothing in the program has to change, the cost is
small, and everything is visible, including the standard library and the
operating system. The price is that the result is statistical.

An **instrumenting profiler** is code that you add: "a region starts here",
"a region ends here", "this cost so much". It is exact, but it sees only what
you marked, and its own work changes what it measures. If the marking code
itself is slow, small regions look more expensive than they are. This is
called the observer effect, and it is why task A of the exercise demands that
charging a cost does not allocate.

Both kinds report two numbers per function, and you must not confuse them:

- **Self cost**: cost spent in the function's own code.
- **Total cost**: self cost plus the cost of everything it called.

```text
main (self 2)
 └─ load (self 5)
     └─ parse (self 3, called 10 times)

            self   total
main          2      37      <- total: everything happens "inside" main
load          5      35
parse        30      30      <- self: this is where the work is
```

Sort by total cost to find the *region* of the program that matters. Sort by
self cost to find the *code* to change. A function with a large total and a
small self cost is not slow; it calls something that is.

Profilers often write their result as **folded stacks**: one line per call
path, with the cost that was spent while exactly that path was running.

```text
main 2
main;load 5
main;load;parse 30
```

A flamegraph is a drawing of these lines. Each box is a function on a path.
Its width is its total cost, and the part of its top edge that has nothing
above it is its self cost. The horizontal order is alphabetical, not time.

**How much can a change gain?** If a part takes 30% of the time and you make
it ten times faster, the run takes 70% + 3% = 73% of the old time. If you
made that part free, the run would still take 70%. This limit is Amdahl's
law. It is the reason to profile first: a brilliant change to code that takes
2% of the time is worth at most 2%.

### Worked example: a guard that marks a region

Regions nest, and a region must end on every way out of a function: at the
last line, at an early `return`, at `?`, and when a panic unwinds. In Rust a
value with a `Drop` impl does this. Such a value is called a guard. This
example logs nested regions with indentation; the exercise's `Span` uses the
same technique to keep a call path.

```rust
use std::cell::{Cell, RefCell};

struct Tracer {
    depth: Cell<usize>,
    lines: RefCell<Vec<String>>,
}

/// Ends its region when it is dropped.
#[must_use = "the region ends when this guard is dropped"]
struct Region<'t> {
    tracer: &'t Tracer,
}

impl Tracer {
    fn region(&self, name: &str) -> Region<'_> {
        let depth = self.depth.get();
        self.lines
            .borrow_mut()
            .push(format!("{}{name}", "  ".repeat(depth)));
        self.depth.set(depth + 1);
        Region { tracer: self }
    }
}

impl Drop for Region<'_> {
    fn drop(&mut self) {
        self.tracer.depth.set(self.tracer.depth.get() - 1);
    }
}

fn parse(tracer: &Tracer, text: &str) -> Option<u32> {
    let _region = tracer.region("parse");
    let number = text.parse().ok()?; // may leave early; the guard still drops
    let _inner = tracer.region("validate");
    Some(number)
}

fn main() {
    let tracer = Tracer {
        depth: Cell::new(0),
        lines: RefCell::new(Vec::new()),
    };
    let _main = tracer.region("main");
    assert_eq!(parse(&tracer, "x"), None);
    assert_eq!(parse(&tracer, "7"), Some(7));
    assert_eq!(
        *tracer.lines.borrow(),
        ["main", "  parse", "  parse", "    validate"]
    );
}
```

After the failed `parse("x")`, the depth was back at 1: the second `parse`
line is indented like the first.

### Mistakes people really make

**The guard that lives for no time.** `let _ = guard;` does not keep the
value: the pattern `_` binds nothing, so the guard is dropped at the end of
that statement. `let _name = guard;` keeps it until the end of the block.

```rust
struct Guard<'a>(&'a std::cell::RefCell<Vec<&'static str>>, &'static str);
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push(self.1);
    }
}

let log = std::cell::RefCell::new(Vec::new());
{
    let _ = Guard(&log, "closed a"); // dropped right here
    log.borrow_mut().push("work a");
    let _b = Guard(&log, "closed b"); // dropped at the end of the block
    log.borrow_mut().push("work b");
}
assert_eq!(*log.borrow(), ["closed a", "work a", "work b", "closed b"]);
```

Calling the function without binding the result is the same mistake. A
`#[must_use]` attribute on the guard type makes the compiler say so:

```text
warning: unused `Region` that must be used
 --> region.rs:9:5
  |
9 |     region("load");
  |     ^^^^^^^^^^^^^^
  |
  = note: the region ends when this guard is dropped
  = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
  |
9 |     let _ = region("load");
  |     +++++++
```

Do not follow the help here. `let _ = region("load");` silences the warning
and keeps the bug. Give the guard a name: `let _region = region("load");`.

**A `RefCell` borrowed twice.** A profiler is called from everywhere, also
while it is in the middle of its own work. If a method holds a `borrow()` of
a `RefCell` and calls something that needs `borrow_mut()` of the same cell,
the program panics:

```rust,should_panic
use std::cell::RefCell;

struct Log {
    lines: RefCell<Vec<String>>,
}

impl Log {
    fn add(&self, line: &str) {
        self.lines.borrow_mut().push(line.to_owned());
    }
    fn mark_empty(&self) {
        let lines = self.lines.borrow(); // shared borrow, held during the loop
        for line in lines.iter() {
            if line.is_empty() {
                self.add("(empty)"); // needs a mutable borrow: panics
            }
        }
    }
}

let log = Log { lines: RefCell::new(vec![String::new()]) };
log.mark_empty();
```

```text
thread 'main' panicked at refcell.rs:4:44:
RefCell already borrowed
```

Keep each borrow as short as possible: take what you need out of the cell,
end the borrow, then call other code.

**Profiling a debug build, or a build without names.** A profile of an
unoptimized build shows costs that the optimizer would have removed. A
profile of a release build without debug information shows addresses instead
of names. Use an optimized build with debug information; this package has a
`profiling` profile for that (`inherits = "release"`, `debug = true`).

**Missing functions.** The optimizer pastes small functions into their
callers (inlining, lesson 06). An inlined function has no frame of its own. A
good profiler uses the debug information to show it anyway; a simple one
charges its cost to the caller. When a function you expect is missing, look
at its caller.

### When to use which

Use a sampling profiler first: it needs no changes and it cannot miss what
you did not think of. Use counted instrumentation when you need exact,
repeatable numbers for one known region, for example to put a budget into a
test, as this lesson does in task B.

### Read more

- [LABS.md, Lab 2](LABS.md#lab-2--record-and-read-a-real-profile): record a real profile of this module's slow program
- [`Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html) and [`RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html)
- [The Cargo book: the `debug` profile setting](https://doc.rust-lang.org/cargo/reference/profiles.html#debug)
- [The rustc book: `force-frame-pointers`](https://doc.rust-lang.org/rustc/codegen-options/index.html#force-frame-pointers), which helps profilers walk the stack

## Lesson 03 · Allocation budgets

**Assumed:** lesson 02; `Vec`, `String` and `HashMap` (chapters 16 and 17);
`Cow` (chapter 13).

### The mental model

The heap is managed by an **allocator**. `Vec`, `String`, `Box`, `HashMap`
and their relatives ask it for a block when they need one and give the block
back when they are dropped. One request is cheap, some tens of nanoseconds,
but it is not free: the allocator searches its lists, sometimes takes a lock,
sometimes asks the operating system for more memory. A fresh block is also
memory that is not in the cache yet. A loop that allocates once per item can
easily spend more time in the allocator than in its own work; the profile in
Lab 2 shows a real program where that share is 40%.

There are four ways to remove an allocation. In order of preference:

1. **Borrow instead of owning.** A `&str` into text that already exists
   costs nothing. Own a value only when it must outlive what it was made
   from.
2. **Own only when needed.** `Cow<'a, str>` is either a borrowed `&'a str` or
   an owned `String`. A function that usually returns its input unchanged
   can return `Cow::Borrowed` for free and pay only in the rare case.
3. **Know the size in advance.** A `Vec` or `String` that grows step by step
   allocates again and again: when it is full, it asks for a block twice as
   large. Unless the allocator can extend the old block where it lies, that
   means copying everything to the new block and freeing the old one.
   `with_capacity(n)` asks once.
4. **Reuse a buffer.** `clear()` removes the elements and keeps the block.
   One buffer that is cleared and refilled a million times is allocated
   once.

### Worked example: counting, and what growth costs

How do the tests of this module count allocations? Rust lets a program
replace the allocator. A replacement that counts and then calls the system
allocator is a few lines. This is a complete program; the module's own
version in `src/support.rs` counts per thread and also tracks bytes and the
peak.

```rust
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

struct Counting;

// SAFETY: each method passes its arguments unchanged to `System` and returns
// what `System` returns, so `System` upholds the `GlobalAlloc` contract for
// us. The extra work only adds to two atomic counters: it does not allocate,
// cannot panic, and does not touch the memory that is handed out.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        CALLS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        // SAFETY: the caller upholds the contract of `alloc` for `layout`.
        unsafe { System.alloc(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        CALLS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(new_size, Ordering::Relaxed);
        // SAFETY: the caller guarantees that `ptr` came from this allocator
        // with `layout`; every block of this allocator is a block of `System`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: as for `realloc`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// (allocator calls, bytes requested) made by `f`.
fn measure(f: impl FnOnce()) -> (usize, usize) {
    let before = (CALLS.load(Ordering::Relaxed), BYTES.load(Ordering::Relaxed));
    f();
    (
        CALLS.load(Ordering::Relaxed) - before.0,
        BYTES.load(Ordering::Relaxed) - before.1,
    )
}

fn main() {
    // Growing step by step: the block is replaced by one twice as large.
    let grown = measure(|| {
        let mut text = String::new();
        for _ in 0..1_000 {
            text.push('x');
        }
        std::hint::black_box(text);
    });
    // Asking once for the size that is needed.
    let sized = measure(|| {
        let mut text = String::with_capacity(1_000);
        for _ in 0..1_000 {
            text.push('x');
        }
        std::hint::black_box(text);
    });
    // Blocks of 8, 16, 32, ... 1,024 bytes: 8 requests, 2,040 bytes in all.
    assert_eq!(grown, (8, 2_040));
    assert_eq!(sized, (1, 1_000));
}
```

Growth by doubling keeps the *number* of allocations small (it grows with the
logarithm of the length), but the bytes requested add up to about twice the
final size, and a step may have to copy everything. The starting size of 8 bytes
and the factor of two are choices of the current standard library, not
promises. What is promised: `with_capacity(n)` allocates room for at least
`n` elements, and no push reallocates while the length is below the
capacity.

**Looking up before owning.** A map with owned keys can be asked with a
borrowed key: `HashMap<String, V>::get` takes a `&str`. So "is it there?"
never needs a `String`. And when the map does not outlive the text it was
built from, its keys need not be owned at all:

```rust
use std::collections::HashMap;

/// For each name, the position at which it was first seen.
fn first_seen<'a>(names: &[&'a str]) -> HashMap<&'a str, usize> {
    let mut seen = HashMap::new();
    for (position, &name) in names.iter().enumerate() {
        // The key is a `&'a str`: a pointer and a length, no heap memory.
        seen.entry(name).or_insert(position);
    }
    seen
}

let seen = first_seen(&["ada", "grace", "ada", "linus"]);
assert_eq!(seen["ada"], 0);
assert_eq!(seen["linus"], 3);
```

`entry` is the right tool here because making the key is free. With owned
keys, `map.entry(name.to_owned())` builds a `String` on every call, also when
the key is already present; look the key up first in that case.

**Owning only when needed.** A function that adds a trailing slash to a path
unless it has one:

```rust
use std::borrow::Cow;

fn with_trailing_slash(path: &str) -> Cow<'_, str> {
    if path.ends_with('/') {
        Cow::Borrowed(path)
    } else {
        Cow::Owned(format!("{path}/"))
    }
}

let unchanged = with_trailing_slash("/var/log/");
assert!(matches!(unchanged, Cow::Borrowed(_)));
assert_eq!(with_trailing_slash("/var/log"), "/var/log/");
```

A `Cow<str>` derefs to `&str`, so most callers do not care which case they
got.

### Mistakes people really make

**Keeping a reference into a buffer that may move.** Growth frees the old
block. A reference to an element would point into freed memory, so the
borrow checker forbids pushing while such a reference lives:

```rust,compile_fail,E0502
let mut scores = vec![10, 20, 30];
let first = &scores[0];
scores.push(40);
println!("{first}");
```

```text
error[E0502]: cannot borrow `scores` as mutable because it is also borrowed as immutable
 --> push.rs:4:5
  |
3 |     let first = &scores[0];
  |                  ------ immutable borrow occurs here
4 |     scores.push(40);
  |     ^^^^^^^^^^^^^^^ mutable borrow occurs here
5 |     println!("{first}");
  |                ----- immutable borrow later used here
```

Copy the value out (`let first = scores[0];`), or keep an index instead of a
reference.

**Returning two different types where a `Cow` is expected.** A `String` and a
`&str` are not a `Cow` until you wrap them:

```rust,compile_fail,E0308
use std::borrow::Cow;

fn trim_end_dots(input: &str) -> Cow<'_, str> {
    if input.ends_with('.') {
        input.trim_end_matches('.').to_owned()
    } else {
        input
    }
}
```

```text
error[E0308]: mismatched types
 --> cow.rs:4:9
  |
2 | fn trim_end_dots(input: &str) -> Cow<'_, str> {
  |                                  ------------ expected `Cow<'_, str>` because of return type
3 |     if input.ends_with('.') {
4 |         input.trim_end_matches('.').to_owned()
  |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `Cow<'_, str>`, found `String`
...
error[E0308]: mismatched types
 --> cow.rs:6:9
  |
...
6 |         input
  |         ^^^^^ expected `Cow<'_, str>`, found `&str`
  |
...
help: try wrapping the expression in `std::borrow::Cow::Borrowed`
```

Wrap the two results in `Cow::Owned(..)` and `Cow::Borrowed(..)`. (Here even
the first branch could borrow: `trim_end_matches` returns a part of the
input.)

**Emptying a buffer the wrong way.** `buffer.clear()` keeps the block.
`buffer = String::new()` and `std::mem::take(&mut buffer)` leave a buffer
without a block, and the next push allocates again.

**Collecting only to loop.** `let parts: Vec<&str> = line.split(',').collect();`
followed by one `for` loop over `parts` allocates a vector that the loop did
not need. Loop over the iterator.

**Allocations that the source does not show.** `to_string`, `format!`,
`to_owned`, `join`, `collect` and the `clone` of a collection all allocate,
which is easy to see once you look for it. Some calls are harder to suspect.
`slice::sort` is a stable sort, and a stable sort needs scratch memory: for
a long slice it allocates a buffer on every call. `sort_unstable` sorts in
place and never allocates. With the pinned standard library, sorting `u32`
values, the stable sort starts to allocate at 1,025 elements; that limit is
an implementation detail, and the rule to remember is "stable may allocate,
unstable does not". Use `sort_unstable` whenever equal elements cannot be
told apart, as with plain numbers.

### When to use which

| Situation | Tool |
| --- | --- |
| the result is a part of the input | return a borrow (`&str`, `&[T]`) |
| the result is usually the input, sometimes changed | `Cow` |
| you know or can compute the final size | `with_capacity`, `reserve` |
| the same kind of temporary in every loop round | one buffer, `clear()` each round |
| the caller already has a buffer | take `&mut String` or `&mut Vec<T>` and append |
| many small values that die together | an arena crate, after a profile demands it |

Do not remove an allocation that runs once per request next to a network
call. Remove the ones a profile shows, then guard them with a budget test.

### Read more

- [`Vec`: capacity and reallocation](https://doc.rust-lang.org/std/vec/struct.Vec.html#capacity-and-reallocation) and [its guarantees](https://doc.rust-lang.org/std/vec/struct.Vec.html#guarantees)
- [`Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html)
- [`HashMap::entry`](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.entry) and [`HashMap::get`](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.get), which accepts a borrowed form of the key
- [`GlobalAlloc`](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html)

## Lesson 04 · Data layout

**Assumed:** lesson 03; structs and enums (chapters 9 and 10); `Box`
(chapter 4); `NonZero` integer types are explained here.

### The mental model

Every type has a **size** (how many bytes a value takes) and an **alignment**
(its address must be a multiple of this number). On the 64-bit targets of
this course, a `u32` has size 4 and alignment 4; a `u64` has 8 and 8; a `u8`
has 1 and 1. Processors read aligned
values fastest, and some cannot read unaligned ones at all.

A struct's alignment is the largest alignment of its fields. Each field must
start at a multiple of its own alignment, so the compiler may have to leave
unused bytes between fields. These bytes are called **padding**. The size of
the struct is rounded up to a multiple of its alignment, so that structs can
stand side by side in an array; that can add padding at the end.

How much padding there is depends on the order of the fields, and on who
chooses that order:

- With the default representation (`repr(Rust)`), **the compiler chooses**.
  The order is not specified, and today's compiler sorts fields so that
  padding is small. You get a good layout without thinking, and you must not
  assume any particular order.
- With `#[repr(C)]`, **you choose**: fields lie in the order in which you
  wrote them, by the rules that C compilers use. This is for data that
  another language or a file format also reads. A careless order costs
  space.

```text
#[repr(C)] struct Wide { a: u8, b: u32, c: u8 }       size 12, alignment 4

offset   0    1    2    3    4    5    6    7    8    9   10   11
       +----+----+----+----+----+----+----+----+----+----+----+----+
       | a  | padding      | b                 | c  | padding      |
       +----+----+----+----+----+----+----+----+----+----+----+----+

#[repr(C)] struct Tight { b: u32, a: u8, c: u8 }      size 8, alignment 4

offset   0    1    2    3    4    5    6    7
       +----+----+----+----+----+----+----+----+
       | b                 | a  | c  | padding |
       +----+----+----+----+----+----+----+----+
```

The rule for `repr(C)`: order the fields from the largest alignment to the
smallest, and no padding is needed between them.

```rust
use std::mem::{align_of, offset_of, size_of};

#[repr(C)]
struct Wide {
    a: u8,
    b: u32,
    c: u8,
}

#[repr(C)]
struct Tight {
    b: u32,
    a: u8,
    c: u8,
}

/// The same fields as `Wide`, in the default representation.
struct Plain {
    a: u8,
    b: u32,
    c: u8,
}

assert_eq!((size_of::<Wide>(), align_of::<Wide>()), (12, 4));
assert_eq!(offset_of!(Wide, b), 4);
assert_eq!(offset_of!(Wide, c), 8);
assert_eq!((size_of::<Tight>(), align_of::<Tight>()), (8, 4));
// Not a promise of the language, but what this compiler does:
assert_eq!(size_of::<Plain>(), 8);
```

### Niches: an `Option` for free

An enum must remember which variant it holds. Usually that takes extra bytes,
a **tag**. But some types have bit patterns that are never valid values: a
reference and a `Box` are never null, a `bool` is only ever 0 or 1, and a
`NonZeroU32` is never 0. Such an unused pattern is called a **niche**. The
compiler stores `None` in it, and the `Option` costs nothing.

```rust
use std::num::NonZeroU32;
use std::mem::size_of;

assert_eq!(size_of::<Option<&u8>>(), size_of::<&u8>());
assert_eq!(size_of::<Option<Box<u64>>>(), size_of::<Box<u64>>());
assert_eq!(size_of::<Option<NonZeroU32>>(), 4);
// Every bit pattern of a `u32` is a valid `u32`: no niche, so a tag is
// needed, and the size is rounded up to the alignment of 4.
assert_eq!(size_of::<Option<u32>>(), 8);

// A common trick: 0 is never a valid id, so store ids as NonZeroU32.
let id = NonZeroU32::new(17).expect("17 is not zero");
assert_eq!(id.get(), 17);
assert_eq!(NonZeroU32::new(0), None);
```

The first three equalities are guaranteed by the documentation of `Option`.

### An enum is as large as its largest variant

Every value of an enum type has the same size, so it must have room for the
largest variant. One large, rare variant makes every value large:

```rust
use std::mem::size_of;

enum Event {
    Tick,
    Key(u8),
    Snapshot([u64; 64]), // 512 bytes, and rare
}

enum SlimEvent {
    Tick,
    Key(u8),
    Snapshot(Box<[u64; 64]>), // 8 bytes here, 512 on the heap
}

assert_eq!(size_of::<Event>(), 520);
assert_eq!(size_of::<SlimEvent>(), 16);
```

A queue of a million `Event`s takes 520 MB; of `SlimEvent`s, 16 MB plus the
snapshots that really exist. Clippy points at such enums:

```text
warning: large size difference between variants
 --> src/lib.rs:3:1
  |
3 | / pub enum Event {
4 | |     Tick,
5 | |     Key(u8),
  | |     ------- the second-largest variant contains at least 1 bytes
6 | |     Snapshot([u64; 64]),
  | |     ------------------- the largest variant contains at least 512 bytes
7 | | }
  | |_^ the entire enum is at least 520 bytes
  |
...
help: consider boxing the large fields or introducing indirection in some other way to reduce the total size of the enum
```

The price of the `Box` is an allocation and a pointer to follow, paid only by
the rare variant.

### Arrays of structs and structs of arrays

A `Vec<Point>` stores whole points one after another. This is an **array of
structs** (AoS). A loop that needs one field of every point still drags all
the other fields through the cache:

```text
array of structs:   | x y label.. | x y label.. | x y label.. | ...
                      ^             ^             ^
                      a loop over `x` uses 4 bytes of every 32

struct of arrays:   xs:     | x | x | x | x | x | x | x | x | ...
                    ys:     | y | y | y | y | y | y | y | y | ...
                    labels: | label.. | label.. | label.. | ...
                      a loop over `x` uses every byte it loads
```

A **struct of arrays** (SoA) keeps one vector per field. Row `i` is element
`i` of every vector.

```rust
struct Points {
    // Invariant: all three vectors have the same length.
    xs: Vec<f32>,
    ys: Vec<f32>,
    labels: Vec<String>,
}

impl Points {
    fn push(&mut self, x: f32, y: f32, label: &str) {
        self.xs.push(x);
        self.ys.push(y);
        self.labels.push(label.to_owned());
    }

    /// Reads only the two coordinate columns.
    fn centre(&self) -> Option<(f32, f32)> {
        let n = self.xs.len();
        if n == 0 {
            return None;
        }
        let sum_x: f32 = self.xs.iter().sum();
        let sum_y: f32 = self.ys.iter().sum();
        Some((sum_x / n as f32, sum_y / n as f32))
    }
}

let mut points = Points { xs: Vec::new(), ys: Vec::new(), labels: Vec::new() };
points.push(0.0, 2.0, "a");
points.push(4.0, 6.0, "b");
assert_eq!(points.centre(), Some((2.0, 4.0)));
assert_eq!(points.labels[1], "b");
```

The invariant in the comment is the cost of this layout: nothing in the type
system keeps the columns in step. Keep the fields private and let one `push`
method add to all of them.

### Mistakes people really make

**Assuming the default order.** Code that reads a `repr(Rust)` struct as
bytes, or assumes that the first field is at offset 0, is wrong even when it
works today. Ask with `offset_of!`, or use `repr(C)`.

**Removing padding with `repr(packed)`.** Packing sets the alignment to 1 and
removes all padding. Fields may then lie at addresses that are not multiples
of their alignment, and a reference to such a field is not allowed to exist:

```rust,compile_fail,E0793
#[repr(C, packed)]
struct Header {
    kind: u8,
    length: u32,
}

let header = Header { kind: 1, length: 500 };
let length = &header.length;
println!("{} {length}", header.kind);
```

```text
error[E0793]: reference to field of packed struct is unaligned
 --> packed.rs:8:18
  |
8 |     let length = &header.length;
  |                  ^^^^^^^^^^^^^^
  |
  = note: this struct is 1-byte aligned, but the type of this field may require higher alignment
  = note: creating a misaligned reference is undefined behavior (even if that reference is never dereferenced)
  = help: copy the field contents to a local variable, or replace the reference with a raw pointer and use `read_unaligned`/`write_unaligned` (loads and stores via `*p` must be properly aligned even when using raw pointers)
```

Copy the field (`let length = header.length;`). Use `packed` only for a file
or wire format that demands it; reordering fields is the right way to save
space.

**Measuring the wrong size.** `size_of::<Vec<u8>>()` is 24 on a 64-bit
target, whatever the vector holds: pointer, capacity, length. The elements
are on the heap. `size_of` tells you what a value costs *inline*.

### When to use which

| Situation | Choice |
| --- | --- |
| an ordinary Rust type | the default representation; the compiler orders the fields |
| a type shared with C or a file format | `repr(C)`, fields ordered by alignment, largest first |
| "no value" for an id, an index or a pointer | a niche type (`NonZero`, a reference, a `Box`) inside `Option` |
| an enum with one large, rare variant | `Box` that variant |
| loops that read all fields of one record | array of structs |
| loops that read few fields of many records | struct of arrays |

### Read more

- [The Rust Reference: type layout](https://doc.rust-lang.org/reference/type-layout.html)
- [`offset_of!`](https://doc.rust-lang.org/std/mem/macro.offset_of.html), [`size_of`](https://doc.rust-lang.org/std/mem/fn.size_of.html), [`align_of`](https://doc.rust-lang.org/std/mem/fn.align_of.html)
- [`Option`: representation](https://doc.rust-lang.org/std/option/index.html#representation), the list of types for which `Option<T>` has the size of `T`
- [The Rustonomicon: `repr(Rust)`](https://doc.rust-lang.org/nomicon/repr-rust.html) and [other representations](https://doc.rust-lang.org/nomicon/other-reprs.html)

## Lesson 05 · Traversal and bounds checks

**Assumed:** lesson 04; iterators and slices (chapters 15 and 16). The last
part reads one `unsafe` block; chapter 23 is enough for it.

### The mental model: the cache

Main memory is slow compared with the processor: fetching a value from it
takes as long as a hundred or more simple instructions. Processors therefore
keep copies of recently used memory in **caches**, small and fast. The
innermost cache of a current processor holds some tens of kilobytes.

Three facts about caches decide how fast a loop over data runs:

1. A cache works in **lines**, here taken as 64 bytes (the size on most
   x86-64 and ARM processors; Apple silicon uses 128, which changes the
   numbers below and not the reasoning). Reading one byte loads its whole
   line. The other 63 bytes then cost almost nothing.
2. A cache is small. When it is full, loading a line evicts another one,
   usually one that was not used for a while.
3. The processor notices when a program walks through memory in a straight
   line, and fetches the next lines before they are needed (prefetching).

So the cheapest way to use memory is to walk through it in address order and
to use everything you load. A loop that jumps around pays one miss per
access; a loop that walks straight pays one miss per 64 bytes, and
prefetching hides most of those too.

A two-dimensional grid that is stored row after row (**row-major order**)
makes this concrete:

```text
memory:   | row 0 ............ | row 1 ............ | row 2 ............ |

row by row:        ---------------->---------------->---------------->
                   one long straight walk

column by column:  ^                    ^                    ^
                   |____________________|____________________|___ ...
                   every step jumps a whole row ahead
```

For a grid whose rows are longer than a cache line, the column walk touches a
new line on every step. If the grid is larger than the cache, each of those
lines has been evicted again before the walk comes back for the next column.

**The model in the exercise.** The tests cannot observe your machine's
cache, so the lesson supplies a model: `Lru` holds the most recently used
lines (64 of them in most tests) and counts a miss whenever a line is not
among them. Real caches are
more complicated. They have several levels of different sizes. A line cannot
be stored anywhere but only in one of a few places that its address selects,
so lines can evict each other even when the cache is not full. And real
processors prefetch. The model ignores all of that and still captures the
main effect: whether your loop reuses a line while it is in the cache.

### Worked example: one pass instead of two

Finding the smallest and the largest value with two iterator chains walks
the data twice. If the data does not fit in the cache, the second pass
misses on every line again.

```rust
fn range_two_passes(values: &[u32]) -> Option<(u32, u32)> {
    let low = values.iter().copied().min()?;
    let high = values.iter().copied().max()?;
    Some((low, high))
}

fn range_one_pass(values: &[u32]) -> Option<(u32, u32)> {
    let (&first, rest) = values.split_first()?;
    Some(
        rest.iter()
            .fold((first, first), |(low, high), &v| (low.min(v), high.max(v))),
    )
}

let values = [7, 3, 9, 4];
assert_eq!(range_two_passes(&values), Some((3, 9)));
assert_eq!(range_one_pass(&values), Some((3, 9)));
assert_eq!(range_one_pass(&[]), None);
```

For a million `u32` values (4 MB, 62,500 lines) the first version loads
125,000 lines and the second 62,500. Combining loops like this is called
loop fusion.

**When no single order is good.** Some tasks walk two structures in
different directions at once. To transpose a grid, you read along a row of
the source and write down a column of the target; whichever order you pick,
one of the two is jumping. The answer is to split the work into small square
**blocks** (or tiles). Inside a block of 16 by 16 cells, the rows of the
source and the rows of the target that the block touches are a few dozen
lines, which all stay in the cache until the block is done. Then you move to
the next block. Every element is still read once and written once; only the
order has changed. Task B of the exercise is this.

### The mental model: bounds checks

`values[i]` is two operations: compare `i` with the length, and panic if it
is not smaller; then load the element. One comparison is cheap, and the
processor predicts its outcome well. What can cost more is indirect: a
possible panic in the middle of a loop limits how the optimizer may rewrite
the loop, for example to process several elements per instruction
(**vectorization**), because everything that happened before the panic must
stay visible.

The optimizer removes a check when it can prove that the index is in range.
For a simple loop it can do more: compute in front of the loop how many
rounds are surely in range, run those rounds without checks, and keep the
checks only for the rest. Lab 5 shows this compiler doing exactly that.
Sometimes these proofs are easy, and sometimes they fail for reasons that
are hard to see from the source: an index that comes from data, a call that
was not inlined, a length that changes in the loop. You can write the loop
so that no proof is needed:

- **Iterate instead of indexing.** An iterator goes from the first element
  to the last and stops. There is no index that could be wrong.
- **Check once, in front of the loop.** `let head = &values[..n];` is one
  check. After it, `head` has exactly `n` elements, and a loop over `0..n`
  that indexes `head` is known to be in range.

```rust
/// `values[i] *= factors[i]` for every position that both slices have.
fn scale_zip(values: &mut [f32], factors: &[f32]) {
    for (value, factor) in values.iter_mut().zip(factors) {
        *value *= factor;
    }
}

/// The same for the first `n` positions. Panics if a slice is shorter.
fn scale_first(values: &mut [f32], factors: &[f32], n: usize) {
    // Two checks here; none are needed in the loop.
    let (values, factors) = (&mut values[..n], &factors[..n]);
    for i in 0..n {
        values[i] *= factors[i];
    }
}

let mut values = [1.0, 2.0, 3.0];
scale_zip(&mut values, &[2.0, 2.0]);
assert_eq!(values, [2.0, 4.0, 3.0]);
scale_first(&mut values, &[0.5, 0.5, 0.5], 3);
assert_eq!(values, [1.0, 2.0, 1.5]);
```

Notice that the two functions mean different things. `zip` stops at the
shorter slice without a word. The sub-slice version panics when a slice is
too short. If a caller that passes slices of different lengths has a bug,
`zip` hides it. Choose on purpose.

### The last resort: `get_unchecked`

Some indices come from data, and no rewriting makes them provably in range:
a table of positions, for example. `slice::get_unchecked` skips the check.
It is an `unsafe` method: an index that is out of range is undefined
behavior. You may use it only where you can prove the index is in range, and
the proof belongs in a `// SAFETY:` comment.

The sound way is to check all indices once, and to let a type remember that
the check happened:

```rust
mod gather {
    /// Indices that are all known to be below `bound`.
    pub struct Checked<'a> {
        // Invariant: every element of `indices` is less than `bound`.
        // The fields are private, and no method changes them, so the
        // invariant holds for as long as a value of this type exists.
        indices: &'a [u32],
        bound: usize,
    }

    impl<'a> Checked<'a> {
        /// Checks every index once. `None` if one is out of range.
        pub fn new(indices: &'a [u32], bound: usize) -> Option<Checked<'a>> {
            indices
                .iter()
                .all(|&index| (index as usize) < bound)
                .then_some(Checked { indices, bound })
        }
    }

    /// The sum of `values[i]` over the checked indices.
    pub fn sum_at(values: &[u64], indices: &Checked<'_>) -> u64 {
        assert!(
            values.len() >= indices.bound,
            "the indices were checked against a larger length"
        );
        let mut sum = 0u64;
        for &index in indices.indices {
            // SAFETY: `index < indices.bound` by the invariant of `Checked`,
            // and `indices.bound <= values.len()` by the assertion above.
            // `values` is a shared borrow, so its length cannot change
            // during the loop. Hence `index` is in range for `values`.
            sum = sum.wrapping_add(unsafe { *values.get_unchecked(index as usize) });
        }
        sum
    }
}

let values = [10, 20, 30, 40];
let indices = gather::Checked::new(&[3, 0, 3], values.len()).expect("all below 4");
assert_eq!(gather::sum_at(&values, &indices), 90);
assert!(gather::Checked::new(&[4], values.len()).is_none());
```

Every step of the argument is checked by code that cannot be skipped: the
constructor checks the indices, and the function checks that the slice is
long enough. Do this only after a benchmark has shown that the checks cost
something. Most of the time they do not.

### Mistakes people really make

**The off-by-one that the check exists for.**

```rust,should_panic
let levels = vec![3, 1, 4];
let mut total = 0;
for i in 0..=levels.len() {
    total += levels[i];
}
println!("{total}");
```

```text
thread 'main' panicked at idx.rs:5:24:
index out of bounds: the len is 3 but the index is 3
```

**Skipping the check without `unsafe`.**

```rust,compile_fail,E0133
fn first(values: &[u32]) -> u32 {
    *values.get_unchecked(0)
}
```

```text
error[E0133]: call to unsafe function `core::slice::<impl [T]>::get_unchecked` is unsafe and requires unsafe block
 --> unchecked.rs:2:6
  |
2 |     *values.get_unchecked(0)
  |      ^^^^^^^^^^^^^^^^^^^^^^^ call to unsafe function
```

The compiler is asking for a proof, not for the keyword. Here no proof
exists: the slice may be empty. Use `values.first()`.

**Replacing an index loop with `zip` and losing a panic.** As above: the new
loop is shorter and has no checks, and it no longer notices slices of
different lengths. Add an `assert_eq!` on the lengths in front of it if equal
lengths are part of the contract.

### When to use which

| Situation | Write |
| --- | --- |
| walk one slice | `for x in slice` or iterator adapters |
| walk two slices in step | `zip`, with a length assertion if lengths must match |
| neighbours, fixed-size groups | `windows(n)`, `chunks_exact(n)` (and handle its remainder) |
| indices are needed, range known up front | take a sub-slice first, then index it |
| indices come from data | checked indexing; `get_unchecked` only with a proof and a benchmark |
| data larger than the cache, two walks in different directions | blocks |

### Read more

- [Slice methods](https://doc.rust-lang.org/std/primitive.slice.html): [`chunks_exact`](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks_exact), [`windows`](https://doc.rust-lang.org/std/primitive.slice.html#method.windows), [`split_at`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_at), [`get_unchecked`](https://doc.rust-lang.org/std/primitive.slice.html#method.get_unchecked)
- [`Iterator::zip`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.zip)
- [The Rust Reference: array and slice indexing](https://doc.rust-lang.org/reference/expressions/array-expr.html#array-and-slice-indexing-expressions)
- [LABS.md, Lab 5](LABS.md#lab-5--traversal-order-in-time-and-bounds-checks-in-the-compilers-output): find the checks in the compiler's output

## Lesson 06 · Dispatch, inlining and code size

**Assumed:** lesson 05; traits and generics (chapter 11); closures
(chapter 14).

### The mental model

When code calls a method of a trait, the program must find the function to
run. This step is called **dispatch**, and Rust has two kinds.

**Static dispatch: generics.** For `fn total<S: Shape>(shapes: &[S])`, the
compiler makes a separate copy of `total` for every type `S` that the program
uses. In each copy, `S` is one concrete type, and each call of a `Shape`
method is a direct call of a known function. The optimizer can then
**inline** the call: paste the body of the called function into the caller,
which removes the call and lets it simplify both together. The price is in
the copies: more code to compile, a larger program, and more machine code
competing for the instruction cache.

**Dynamic dispatch: trait objects.** A `&dyn Shape` is two pointers: one to
the value and one to a table of function pointers for the value's type, the
**vtable**. A method call loads the function's address from the table and
jumps to it. There is one copy of the calling code for all types. The price
is in the call: the processor has to predict where an indirect call goes, and
above all the optimizer cannot inline through it, because it does not know
which function will run.

```rust
use std::mem::size_of;

trait Shape {
    fn area(&self) -> f64;
}

struct Square(f64);
struct Circle(f64);

impl Shape for Square {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
}
impl Shape for Circle {
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.0 * self.0
    }
}

/// One copy of this function per type `S`. All elements have that one type.
fn total_static<S: Shape>(shapes: &[S]) -> f64 {
    shapes.iter().map(|shape| shape.area()).sum()
}

/// One copy for every type. The elements may be of different types.
fn total_dynamic(shapes: &[&dyn Shape]) -> f64 {
    shapes.iter().map(|shape| shape.area()).sum()
}

assert_eq!(total_static(&[Square(1.0), Square(2.0)]), 5.0);
assert_eq!(total_dynamic(&[&Square(2.0), &Circle(0.0)]), 4.0);
assert_eq!(size_of::<&Square>(), 8);
assert_eq!(size_of::<&dyn Shape>(), 16); // value pointer + vtable pointer
```

**A third way: an enum.** If you own the full list of types and it will not
grow, an enum with a `match` is a closed form of dispatch. The values lie
inline (no `Box` per element), the `match` is an ordinary branch, and each
arm can be inlined.

```rust
#[derive(Clone, Copy)]
enum Shape {
    Square(f64),
    Circle(f64),
}

impl Shape {
    fn area(self) -> f64 {
        match self {
            Shape::Square(side) => side * side,
            Shape::Circle(radius) => std::f64::consts::PI * radius * radius,
        }
    }
}

let shapes = [Shape::Square(2.0), Shape::Circle(0.0)];
assert_eq!(shapes.iter().map(|shape| shape.area()).sum::<f64>(), 4.0);
assert_eq!(std::mem::size_of::<Shape>(), 16); // 8 bytes of data + tag, aligned
```

### Two techniques that give you most of both

**One virtual call per batch.** The cost of dynamic dispatch is per call. If
a trait has a method that handles a whole slice, the dynamic call happens
once, and the loop over the elements runs inside the implementing type, where
every call is direct again. The I/O traits of the standard library are
designed this way: `Read::read` and `Write::write` move a whole slice of
bytes per call, so a `&mut dyn Write` costs one indirect call per buffer,
not one per byte. (`Iterator` is the opposite case: through a
`&mut dyn Iterator`, every `next` is an indirect call.) Task A of the
exercise builds a batch method for a small trait.

**A thin generic shell around a non-generic core.** A function is often
generic only to be convenient: it accepts anything that can be seen as a
`&str` or a `&Path`. The body does not care about the type after the first
line. Put the body into an inner function that is not generic:

```rust
/// Callers may pass `&str`, `String`, `&String`, `Box<str>`, `Cow<str>`...
fn shout<S: AsRef<str>>(text: S) -> String {
    // Not generic: compiled once, however many types `shout` is used with.
    fn inner(text: &str) -> String {
        let mut loud = text.to_uppercase();
        loud.push('!');
        loud
    }
    // The only code that is copied per type: one conversion and one call.
    inner(text.as_ref())
}

assert_eq!(shout("hi"), "HI!");
assert_eq!(shout(String::from("there")), "THERE!");
```

The standard library does this in many places, for example in
`std::fs::read`. Lab 6 counts the copies with and without the shell.

### Closures

Every closure expression has its own type, which nobody can write down. The
type is a struct that holds the captured variables. A closure that captures
nothing has size zero. A function that takes `impl Fn(u64) -> u64` is
generic over that type: it gets a copy per closure, and the call is direct
and can be inlined. `Box<dyn Fn(u64) -> u64>` is the dynamic form: 16 bytes
for the two pointers, an indirect call, and one allocation for the captured
variables (none when the closure captures nothing, because a value of size
zero needs no heap memory).

```rust
use std::mem::size_of_val;

let step = 3u64;
let add_step = move |x: u64| x + step; // captures one u64
let double = |x: u64| x * 2; // captures nothing
let boxed: Box<dyn Fn(u64) -> u64> = Box::new(add_step);
let pointer: fn(u64) -> u64 = double; // only possible without captures

assert_eq!(size_of_val(&add_step), 8);
assert_eq!(size_of_val(&double), 0);
assert_eq!(size_of_val(&boxed), 16);
assert_eq!(size_of_val(&pointer), 8);
assert_eq!((add_step(1), double(4), boxed(1), pointer(4)), (4, 8, 4, 8));
```

### Inlining and its attributes

Inlining is the optimization that makes small functions, iterators and
closures free. The compiler decides by itself, and inside one crate it
decides well. Four attributes let you interfere:

- `#[inline]` makes the body available to other crates and nudges the
  compiler toward inlining. A function that is not generic is otherwise
  compiled once, in its own crate, and callers in other crates can only call
  it, unless link-time optimization is switched on. (Generic functions are
  always available, because their copies are made in the crate that uses
  them. In optimized builds this compiler also treats very small functions
  that call nothing as if they had `#[inline]`; Lab 5 runs into that.)
- `#[inline(always)]` insists. It is rarely right: the compiler's own
  judgment about size and speed is usually better than a guess.
- `#[inline(never)]` forbids. Use it to keep a function visible in a
  profile, or to keep a large rare path out of a small hot function.
- `#[cold]` says "this is rarely called". The compiler then arranges the
  caller so that the common path runs straight through.

```rust
#[derive(Debug, PartialEq)]
struct BadDigit(String);

/// Hot: called for every byte of the input.
fn digit(byte: u8) -> Result<u8, BadDigit> {
    if byte.is_ascii_digit() {
        Ok(byte - b'0')
    } else {
        Err(bad_digit(byte))
    }
}

/// Rare, and comparatively large because of the formatting. Kept out of
/// `digit`, so that `digit` stays small enough to be inlined everywhere.
#[cold]
#[inline(never)]
fn bad_digit(byte: u8) -> BadDigit {
    BadDigit(format!("byte {byte:#04x} is not a digit"))
}

assert_eq!(digit(b'7'), Ok(7));
assert_eq!(digit(b'x'), Err(BadDigit("byte 0x78 is not a digit".to_owned())));
```

### Mistakes people really make

**A trait that cannot be a trait object.** A vtable has one entry per
method, with one fixed signature. A method that returns `Self` or has a type
parameter has no single entry:

```rust,compile_fail,E0038
trait Shape {
    fn area(&self) -> f64;
    fn scaled(&self, factor: f64) -> Self;
}

fn total(shapes: &[Box<dyn Shape>]) -> f64 {
    shapes.iter().map(|shape| shape.area()).sum()
}
```

```text
error[E0038]: the trait `Shape` is not dyn compatible
 --> dynsafe.rs:6:24
  |
6 | fn total(shapes: &[Box<dyn Shape>]) -> f64 {
  |                        ^^^^^^^^^ `Shape` is not dyn compatible
  |
note: for a trait to be dyn compatible it needs to allow building a vtable
...
3 |     fn scaled(&self, factor: f64) -> Self;
  |                                      ^^^^ ...because method `scaled` references the `Self` type in its return type
  = help: consider moving `scaled` to another trait
```

Move the method to another trait, or add `where Self: Sized` to it, which
excludes it from the vtable.

**Two closures in one collection.** A `Vec` has one element type, and two
closures never have the same type:

```rust,compile_fail,E0308
let offset = 10;
let steps = vec![|x: u64| x + offset, |x: u64| x * offset];
println!("{}", steps[0](1));
```

```text
error[E0308]: mismatched types
 --> closures.rs:3:43
  |
3 |     let steps = vec![|x: u64| x + offset, |x: u64| x * offset];
  |                      --------             ^^^^^^^^^^^^^^^^^^^ expected closure, found a different closure
  |                      |
  |                      the expected closure
  |
  = note: no two closures, even if identical, have the same type
  = help: consider boxing your closure and/or using it as a trait object
```

This is where `Box<dyn Fn(u64) -> u64>` is the right tool: a collection of
different behaviors. If the behaviors are a closed set, an enum avoids the
boxes.

**`#[inline(always)]` everywhere.** It makes the program larger and the
build slower, and it can make the program slower too. Add an inlining
attribute only after a profile or the compiler's output has shown a call
that should not be there.

### When to use which

| Situation | Choice |
| --- | --- |
| small function, called in a hot loop | generic (`impl Trait`, `T: Trait`) |
| large function, convenient generic parameter | generic shell, non-generic core |
| large function, many types, not hot | `&dyn Trait` |
| a collection of values of different types, open set | `Box<dyn Trait>` |
| a collection of values of different kinds, closed set | enum |
| `dyn` is required, but the work is per element | a batch method on the trait |

### Read more

- [The Rust Reference: trait objects](https://doc.rust-lang.org/reference/types/trait-object.html) and [dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)
- [The Rust Reference: the `inline` and `cold` attributes](https://doc.rust-lang.org/reference/attributes/codegen.html)
- [The Rust Reference: closure types](https://doc.rust-lang.org/reference/types/closure.html)
- [LABS.md, Lab 6](LABS.md#lab-6--how-many-copies-of-a-generic-function): count the copies of a generic function
- Mastery M1, lesson 05, takes trait objects apart in detail.

## Lesson 07 · Buffering

**Assumed:** lesson 03; `Read`, `Write` and `BufRead` (chapter 18); `Drop`
(chapter 13).

### The mental model

A file, a pipe, a socket and the terminal belong to the operating system.
Every `read` and `write` on them is a **system call**: the program stops, the
processor switches to the kernel, the kernel does the work and switches back.
That costs hundreds of nanoseconds or more, however few bytes are moved. A
program that writes a million small pieces with a million system calls spends
nearly all of its time on the switching.

A **buffer** is a block of memory between your code and the system call.

- `BufWriter<W>` collects what you write. When its buffer (8 KiB by default)
  is full, it passes the whole buffer to `W` with one `write`.
- `BufReader<R>` asks `R` for a large piece (8 KiB) with one `read` and hands
  it to you in the small pieces you ask for.

`write!` and `writeln!` call `write` on their target several times: at least
once for every piece of literal text and for every formatted value. Without
a buffer, each of those is a system call.

Buffering changes *when* things happen, and that has three consequences.

**1. Data is not written until the buffer is flushed.** `flush()` passes on
what is buffered and then flushes the inner writer. If another program or a
person is waiting for your output, you decide when to flush.

**2. Errors arrive late.** `writeln!` into a `BufWriter` usually just copies
into memory and returns `Ok`. A full disk or a closed pipe shows up when the
buffer is passed on: at some later write, or at the flush.

**3. Dropping is not flushing.** A `BufWriter` writes what is left when it
is dropped, but `drop` cannot return an error, so a failure there is
ignored. A function that returns `Ok(())` and lets its `BufWriter` fall out
of scope can lose data and report success. Always end with `flush()?`.

### Worked example: the lost error

`Flaky` accepts everything except the third `write` call it receives. The
function writes seven lines of five bytes through a buffer of 16 bytes, and
flushes at the end only if it is told to.

```rust
use std::io::{self, BufWriter, Write};

/// A writer that fails on its third `write` call.
#[derive(Default)]
struct Flaky {
    calls: usize,
    accepted: usize,
}

impl Write for Flaky {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.calls == 3 {
            return Err(io::Error::other("disk full"));
        }
        self.accepted += data.len();
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn seven_lines(out: &mut Flaky, flush: bool) -> io::Result<()> {
    let mut out = BufWriter::with_capacity(16, out);
    for _ in 0..7 {
        // Three lines fit into the buffer. The fourth and the seventh do
        // not, so each of them first sends 15 buffered bytes to `Flaky`.
        out.write_all(b"line\n")?;
    }
    // Five bytes are still in the buffer here.
    if flush { out.flush() } else { Ok(()) }
}

let mut flaky = Flaky::default();
let result = seven_lines(&mut flaky, false);
assert!(result.is_ok(), "the failure happened in `drop`, which cannot report it");
assert_eq!(flaky.accepted, 30, "only 30 of the 35 bytes arrived");

let mut flaky = Flaky::default();
let result = seven_lines(&mut flaky, true);
assert!(result.is_err(), "the flush reports the failure");
```

The first call is the dangerous one: the function returned `Ok`, and five
bytes were lost.

### Reading: one line buffer, not one `String` per line

`BufRead::lines()` is convenient and allocates a new `String` for every
line. `read_line` appends to a `String` that you own, so one buffer can serve
the whole input. It returns the number of bytes read, and 0 at the end.

```rust
use std::io::{self, BufRead, BufReader};

/// The length in bytes of the longest line, without its line ending.
fn longest_line(input: impl io::Read) -> io::Result<usize> {
    let mut reader = BufReader::new(input);
    let mut line = String::new();
    let mut longest = 0;
    loop {
        line.clear(); // `read_line` appends; without this the lines pile up
        if reader.read_line(&mut line)? == 0 {
            return Ok(longest);
        }
        longest = longest.max(line.trim_end_matches(['\n', '\r']).len());
    }
}

assert_eq!(longest_line("ab\nabcd\r\nabc".as_bytes()).unwrap(), 4);
assert_eq!(longest_line("".as_bytes()).unwrap(), 0);
```

This also keeps memory small: the program holds one buffer and one line,
however large the input is. `read_to_string` holds the whole input.

### What a writer may answer

To build a buffer yourself (task C), you need the exact contract of
`Write::write`. A call `write(data)` may:

- return `Ok(n)` with `0 < n <= data.len()`: it accepted the first `n` bytes.
  `n` may be less than the length; this is a **short write**, and the caller
  has to offer the rest again;
- return `Ok(0)` for a non-empty `data`: this usually means that the writer
  can take no more, now or later;
- return an error of kind `Interrupted`: nothing was written and the call
  should simply be repeated;
- return another error.

`write_all` is the loop that handles the first three cases, and it is what
you normally call. Its weakness is the fourth case: when it returns an
error, you do not know how many bytes were accepted before. Code that must
continue after an error, without losing or repeating bytes, has to call
`write` itself and count. Calling `write` once and ignoring the count is a
bug that clippy refuses outright:

```text
error: written amount is not handled
  --> src/lib.rs:19:5
   |
19 |     out.write(message)?;
   |     ^^^^^^^^^^^^^^^^^^^
   |
   = help: use `Write::write_all` instead, or handle partial writes
   = note: `#[deny(clippy::unused_io_amount)]` on by default
```

### Mistakes people really make

**Reading byte by byte without a buffer.** `Read::bytes()` calls `read` for
every single byte. On a file, that is a system call per byte. Clippy:

```text
warning: calling .bytes() is very inefficient when data is not in memory
  --> src/lib.rs:11:17
   |
11 |     for byte in file.bytes() {
   |                 ^^^^^^^^^^^^
   |
   = help: consider using `BufReader`
```

**Looking at the target too early.** While a `BufWriter` holds `&mut out`,
you cannot look at `out`, and the bytes are not all there yet anyway. Flush,
then drop the writer (or call `into_inner`), then look. Drill 04 is this
mistake.

**Writing again while unwinding.** A buffer type that writes in `drop` also
writes when it is dropped because of a panic. If the panic came out of the
inner writer in the middle of a write, some bytes may then be offered a
second time. `std::io::BufWriter` keeps a flag for exactly this case and
skips the write in `drop`. The exercise does not ask for it; know that it
exists before you ship a buffer type of your own.

**Two buffers in a row, or a buffer around memory.** `BufWriter` around a
`Vec<u8>` or around another `BufWriter` only adds a copy. Buffer the thing
that makes system calls, once.

**Forgetting that `stdout` is line-buffered.** Every `println!` locks
standard output, and Rust's standard output passes each completed line on at
once. For a lot of output, lock once and buffer yourself:
`let mut out = BufWriter::new(io::stdout().lock());`, and flush at the end.

### When to use which

| Situation | Choice |
| --- | --- |
| many small writes to a file, pipe or socket | `BufWriter`, then `flush()?` |
| reading lines or small pieces | `BufReader` with `read_line` into a reused `String` |
| a small file that you need in memory anyway | `std::fs::read_to_string` |
| a protocol where the other side waits for each message | buffer, and flush after each message |
| copying from a reader to a writer | `std::io::copy` |

### Read more

- [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html) and [`BufReader`](https://doc.rust-lang.org/std/io/struct.BufReader.html)
- [`Write::write`](https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write), [`write_all`](https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all) and [`flush`](https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.flush)
- [`BufRead::read_line`](https://doc.rust-lang.org/std/io/trait.BufRead.html#method.read_line)
- [`LineWriter`](https://doc.rust-lang.org/std/io/struct.LineWriter.html), the buffer behind standard output

## Lesson 08 · Capstone

**Assumed:** lessons 01 to 07.

The capstone gives you a program that is correct and slow, and budgets that
your version must meet. The lessons so far each practised one technique. The
capstone practises the method that connects them.

### The loop

1. **Fix the behavior first.** Before you change anything, make sure you can
   tell whether the program still does the same thing. Here the baseline
   stays in the file, and the tests compare your output with its output on
   generated inputs, byte for byte. This is a **differential test**: you do
   not write down what the right answer is, you compare two programs. It
   finds the mistakes that tuning produces: the last line without a line
   break, the empty input, the field that is almost a number.
2. **Measure.** Counts from the usage example, times from the benchmark.
   Write the numbers down; you will want them for the report.
3. **Find the largest cost.** Read the profile (Lab 2 recorded one for this
   very program). Do not start with what looks ugly in the source. Start
   with the largest self cost.
4. **Form one hypothesis and change one thing.** "The fields are collected
   into a `Vec<String>` per line; reading them from the iterator removes
   five allocations per line."
5. **Run the tests, then measure again.** If the number did not move, undo
   the change: it made the code different, not better.
6. **Repeat** until the budgets are met and the profile has no single peak.

### What to look for in this program

Every lesson has a counterpart in the baseline. You do not need all of
these to find them; the profile will point at most of them.

| Lesson | Question to ask of the baseline |
| --- | --- |
| 03 | What is allocated once per line, and what once per field? Which of these values could be borrowed from the line? |
| 03, 07 | Does it need the whole text in memory? The lines? Every request? |
| 04 | What is the smallest state per service from which the report can be written? |
| 07 | How many `read` and `write` calls reach the outside? What happens to an error at the end? |
| 02, 05 | How much work does "the five largest" need? Is sorting everything that work? |

One part of the baseline looks slow and is not: the linear search through
the list of services. There are twelve services, and comparing a few short
strings is cheap next to five allocations. Measure before you spend effort
there.

### Keeping only the best few

`top_k` asks for the `k` largest of `n` items, with `k` small. Sorting
everything costs about `n * log2(n)` comparisons and keeps all `n` items in
memory. A better way keeps only the best `k` seen so far, in a structure
that can tell you its *smallest* member at once. Most new items are smaller
than that member and are rejected with one comparison. `BinaryHeap` is the
standard library's structure for "give me the largest quickly";
`std::cmp::Reverse` turns it into "give me the smallest".

Be careful with the input order when you count comparisons. The standard
library's sort is adaptive: on input that is already sorted it needs only
`n - 1` comparisons. A benchmark or a test on sorted input would tell you
that sorting everything is cheap. Lesson 01 said that a benchmark fixes an
input; which input you fix decides what the result means. Prediction P3 of
the capstone shows the numbers.

### When to stop

Stop when the budgets are met, the profile is flat, and the next idea would
make the code harder to read than the last one. Then write the report (the
template is in LABS.md, Lab 8). A measured, explained speed-up of four times
is worth more to a team than an unexplained one of five.

Further steps exist, and they are outside this module: link-time
optimization and other profile settings, profile-guided optimization, a
different allocator, and parallelism (chapter 19 and mastery M3).

### Read more

- [`BinaryHeap`](https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html) and [`Reverse`](https://doc.rust-lang.org/std/cmp/struct.Reverse.html)
- [`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html), a map that keeps its keys sorted
- [The Cargo book: profile settings](https://doc.rust-lang.org/cargo/reference/profiles.html) (`opt-level`, `lto`, `codegen-units`, `panic`)
- [The rustc book: profile-guided optimization](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
