# Capstone: collections for programs without a heap

[All chapters](../../README.md) · [Specification](SPEC.md) · [Hints](HINTS.md) · [Rubric](RUBRIC.md)

You write a `#![no_std]` library with three data structures that keep their
memory inside themselves:

- `ArrayVec<T, N>`: a vector with room for at most `N` elements;
- `RingBuffer<T, N>`: a double-ended queue with room for at most `N`
  elements;
- `Arena<N>`: a bump allocator over `N` bytes that hands out references
  tied to the lifetime of the arena.

A microcontroller, a kernel before its heap exists, or an interrupt handler
cannot use `Vec` or `Box`. Types like these three are what such code uses
instead. Building them is a compact test of everything the course taught
about ownership: you manage uninitialised memory by hand, and the result
must be as safe to use as `Vec`.

```rust
use nostd_collections::{Arena, ArrayVec, RingBuffer};

let mut samples: ArrayVec<u16, 8> = ArrayVec::new();
samples.push(512);
samples.push(498);

let mut last_three: RingBuffer<u16, 3> = RingBuffer::new();
for sample in samples {
    last_three.push_overwrite(sample);
}

let arena: Arena<64> = Arena::new();
let label = arena.alloc_str("adc0").unwrap();
assert_eq!((last_three.len(), &*label), (2, "adc0"));
```

This is a **blank-file project**. There are no lesson files and no function
bodies to fill in one by one. You get a precise specification, a facade
that only declares the public names so that the tests compile, and
acceptance tests. The fields of every type, the invariants, the unsafe
code and its proofs are yours.

| File | What it is |
| --- | --- |
| [SPEC.md](SPEC.md) | The complete specification. Read all of it first. Its examples run as tests. |
| [HINTS.md](HINTS.md) | Design hints per milestone. No code. |
| [RUBRIC.md](RUBRIC.md) | How to judge your result, including what the tests cannot see. |
| `src/lib.rs` | The facade: every public item of SPEC.md with a `todo!()` body. |
| `src/spec_examples.rs` | One line that turns SPEC.md into documentation tests. Leave it alone. |
| `tests/m1_*.rs` to `tests/m6_*.rs` | The acceptance tests, one file per milestone. |
| `tests/common/mod.rs` | Helpers of the tests: the time limit, and elements that count their drops. |

The reference solution, with a `DESIGN.md` that explains it, is in
`solutions/capstones/nostd_collections/` of the course repository. Look at
it after you have finished, or when one problem has kept you stuck for a
long time.

## Prerequisites

| From | You use it for |
| --- | --- |
| [Chapter 4, Ownership and moves](../../exercises/chapter04/) and [Chapter 5, Borrowing with references](../../exercises/chapter05/) | who owns a value at each moment; why a reference must not outlive its owner |
| [Chapter 13, Everyday traits](../../exercises/chapter13/) | `Drop` and drop order, `Deref`, `Default`, `From`, `Clone` |
| [Chapter 15, Iterators and adapters](../../exercises/chapter15/) | writing iterators, `DoubleEndedIterator`, `ExactSizeIterator`, `IntoIterator` |
| [Chapter 16, Standard collections](../../exercises/chapter16/) | what `Vec` and `VecDeque` offer; they are the models that the tests compare with |
| [Chapter 23, Unsafe Rust](../../exercises/chapter23/) | raw pointers, `unsafe` blocks, writing `SAFETY` comments, a first look at Miri |
| Mastery module [M1 (`m1_types`)](../../mastery/m1_types/README.md) | variance, `PhantomData`, zero-sized types, const generics |
| Mastery module [M2 (`m2_unsafe`)](../../mastery/m2_unsafe/README.md) | `MaybeUninit`, building `Vec`, `IntoIter` and `Drain`; panic safety and leak safety; the aliasing rules; `Send` and `Sync`; Miri |
| Mastery module [M8 (`m8_api`)](../../mastery/m8_api/README.md) | error types, which traits a library type should implement |

Mastery module [M2](../../mastery/m2_unsafe/README.md) is the real prerequisite. If you have not built a vector
from raw memory there, do that first. This capstone takes away the heap
and the step-by-step guidance, and adds a second collection, an arena, and
the full list of edge cases.

Plan for 20 to 30 hours of work (an estimate).

## Install the tools

The file `rust-toolchain.toml` pins the compiler, so `rustup` installs it
the first time you run `cargo` here. You also need a nightly toolchain with
Miri, because Miri exists only there. The course uses one dated nightly,
`nightly-2026-11-20`, because Miri's reports change between nightlies (see
[The nightly toolchain](../../README.md#the-nightly-toolchain)):

```text
rustup toolchain install nightly-2026-11-20 --component miri --component rust-src
cargo +nightly-2026-11-20 miri --version
```

Nothing else is needed. The crate has no dependencies, and the tests use
only the standard library. They run on Linux, macOS and Windows.

## How to work

1. Read [SPEC.md](SPEC.md) completely.
2. Run the tests once, to see the starting point:

   ```text
   cargo test --no-fail-fast
   ```

   Nearly every test fails with `not yet implemented`, at once and without
   hanging. Nine checks pass in the starter (four tests and five of the
   programs that must not compile); SPEC section 10 says which ones and
   why.
3. Work on one milestone at a time, in the order of SPEC section 10. For
   each milestone:
   - write down the invariant of the type before you write a function;
   - make the tests of the milestone pass in debug and in release;
   - run the same tests under Miri;
   - run Clippy.
4. Replace the facade as you go. `src/lib.rs` is a single file with
   placeholder types; a finished crate has modules, real fields and its own
   iterator types. Delete the placeholder iterator `Todo` when nothing uses
   it any more, and delete the line `#![allow(unused_variables)]`.
5. At the end, score yourself with [RUBRIC.md](RUBRIC.md) and collect the
   evidence that SPEC section 13 lists.

### Commands

Run these in this directory.

```text
cargo test --test m1_array_vec               # one milestone
cargo test --test m2_array_vec_ops retain    # the tests of one milestone whose name contains "retain"
cargo test --doc                             # the examples of SPEC.md, and the programs that must not compile
cargo test --no-fail-fast                    # everything, debug profile
cargo test --release --no-fail-fast          # everything, optimised
cargo +nightly-2026-11-20 miri test --test m1_array_vec # one milestone under Miri
cargo +nightly-2026-11-20 miri test                     # everything under Miri (less than a minute)
cargo clippy --all-targets                   # lints for the library and the tests
```

Useful while you hunt a problem under Miri:

```text
cargo +nightly-2026-11-20 miri test --test m5_arena earlier_references -- --nocapture
MIRIFLAGS="-Zmiri-tree-borrows" cargo +nightly-2026-11-20 miri test      # the second aliasing model
MIRIFLAGS="-Zmiri-many-seeds=0..8" cargo +nightly-2026-11-20 miri test --test m5_arena   # several memory layouts
```

On Windows PowerShell, set the variable first:
`$env:MIRIFLAGS = "-Zmiri-tree-borrows"`.

## When something goes wrong

| What you see | Likely cause |
| --- | --- |
| `N element(s) were dropped more than once` | A slot was treated as a value after its value had been moved out or dropped. Usual place: the length (or an index) is updated *after* a call that can panic, or not at all. |
| `N of M element(s) were never dropped (leaked)` | A `Drop` implementation is missing (the collection, or an iterator that owns elements), or it stops at the first destructor that panics. |
| `the test was still running after 20 s` | A loop in your code never ends: an iterator that never returns `None`, or an index that is not advanced. |
| The test program ends with a signal and prints no failure message. On macOS a double free looks like `(signal: 5, SIGTRAP: trace/breakpoint trap)`; on Linux expect `SIGABRT` or `SIGSEGV`; on Windows expect an access violation | Undefined behaviour with a type that owns heap memory, for example a `String` that was dropped twice. Run the same test under Miri. |
| Miri: `Undefined Behavior: reading memory at alloc51016[0x8..0x10], but memory is uninitialized at [0x8..0x10], and this operation requires initialized memory` | A slot that holds no value was read or dropped as a `T`. Compare the function with your invariant: which slots are initialised at this line? |
| Miri: `Undefined Behavior: attempting a read access using <150122> at alloc47805[0x0], but that tag does not exist in the borrow stack for this location` | An aliasing violation: a reference was used after something else claimed the same memory. In the arena, the usual cause is a `&mut` to the whole storage, made while references to earlier allocations are alive. |
| Miri: `memory leaked: alloc48770 (Rust heap, size: 3, align: 1), allocated here:` | An element was never dropped. Miri reports it for elements that own heap memory; the ledger tests report it for the others. |
| A `compile fail` block of SPEC section 7 is reported as failed | The program compiled although it must not. A signature lost a lifetime: the return type no longer borrows from `self`. |
| ``error[E0599]: the method `clone` exists for struct `nostd_collections::ring::Iter<'_, Unique>`, but its trait bounds were not satisfied`` while `m4_ring_iter` is compiled (the path of the type is the one in your crate) | Your iterator of `RingBuffer::iter` is `Clone` only when the element type is. `#[derive(Clone)]` does that. SPEC 5.5 asks for `Clone` without a bound on `T`. |
| `error: mutable borrow from immutable input(s)` from Clippy | The lint `clippy::mut_from_ref` on the allocation functions of the arena. SPEC 6.3 says what to do. |

The numbers in the Miri messages (`alloc51016`, `<150122>`) will be
different in your runs.

## How to assess your work

Green tests are necessary and not sufficient: they cannot see whether your
unsafe code is right for the right reason. Assess yourself in three steps.

1. **The machine checks.** All of these must succeed, and the three test
   runs three times in a row:

   ```text
   cargo test --no-fail-fast
   cargo test --release --no-fail-fast
   cargo +nightly-2026-11-20 miri test
   cargo clippy --all-targets
   ```

2. **The rubric.** Go through [RUBRIC.md](RUBRIC.md) with your code open.
   It asks for things you can point at: an invariant in a comment, a
   `SAFETY` comment that names a fact, the line where a length is lowered
   before a destructor runs.
3. **The comparison.** Read `DESIGN.md` and the code in
   `solutions/capstones/nostd_collections/`. For each place where your
   design differs, decide which one you prefer and why. A different design
   that passes Miri and that you can defend is a good result.
