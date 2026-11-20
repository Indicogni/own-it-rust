# Chapter 23 labs: Miri, audits and deeper implementations

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md)

Lab 1 (Miri) belongs to the chapter: do it when your lessons pass. Labs 2 to 5 are optional. No test in this repository checks them; each one has its own acceptance list.

All experiments use valid allocations and initialized values under stated contracts. The only place where you run undefined behavior on purpose is under Miri in Lab 1, step 5, where the interpreter stops at the first violation. Keep every other invalid-pointer, use-after-free, aliasing or double-drop scenario on paper.

## Lab 1 · Run your unsafe code under Miri

Goal: check the paths that your tests execute for undefined behavior, and learn to read a Miri report.

A passing test does not show that unsafe code is sound. Miri is an interpreter for Rust programs that checks every memory access while it runs your tests. It reports out-of-bounds and misaligned accesses, use after free, reads of uninitialized memory, invalid values, violations of the aliasing rules for references, and memory leaks.

Miri is part of the nightly toolchain only. The course uses one dated nightly, `nightly-2026-11-20`, the same one its CI uses: Miri's messages, and now and then its verdicts, change between nightlies, and a fixed date makes your output match the reports quoted here (see [The nightly toolchain](../../README.md#the-nightly-toolchain) in the course README). The course toolchain stays pinned to Rust 1.99.0; `+nightly-2026-11-20` selects the nightly for one command and changes nothing else.

1. Install the dated nightly with Miri (once; this needs a network connection):

   ```sh
   rustup toolchain install nightly-2026-11-20 --component miri --component rust-src
   ```

2. Run this chapter's tests under Miri, from the repository root:

   ```sh
   cargo +nightly-2026-11-20 miri test --test chapter23
   ```

   Name the test target. A bare `cargo +nightly-2026-11-20 miri test` would interpret every chapter, including tests that use files and threads, and would take very long. The first run also builds a copy of the standard library for Miri. One lesson at a time works too:

   ```sh
   cargo +nightly-2026-11-20 miri test --test chapter23 l06_
   ```

   *Checkpoint:* with all lessons finished, the run ends like an ordinary test run. The model solutions give this result (checked with `miri 0.1.0 (6905905bb7 2026-11-19)`; a newer nightly may print different details):

   ```text
   test result: ok. 74 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   ```

   Unfinished lessons fail at `todo!()` and `guess()` exactly as they do without Miri. That is not a Miri report.

3. Learn the shape of a report. The reports below are real and shortened. This is what Miri printed for a version of lesson 09 that used `read()` where `read_unaligned()` is required. The ordinary tests passed in debug and in release on the development machine; only the technique test and Miri objected.

   ```text
   test l09_cells_layout::tests::c_reads_at_any_alignment ... error: Undefined Behavior: accessing memory based on pointer with alignment 1, but alignment 4 is required
      --> exercises/chapter23/l09_cells_layout.rs:147:19
       |
   147 |     Some(unsafe { bytes.as_ptr().add(offset).cast::<T>().read() })
       |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Undefined Behavior occurred here
       |
       = help: this indicates a bug in the program: it performed an invalid operation, and caused Undefined Behavior
       = note: stack backtrace:
               0: l09_cells_layout::read_at::<u32>
                   at exercises/chapter23/l09_cells_layout.rs:147:19: 147:64
               1: l09_cells_layout::tests::c_reads_at_any_alignment
                   at exercises/chapter23/l09_cells_layout.rs:341:13: 341:41
   ```

   Read it in this order: the first line names the rule that was broken; the arrow shows the expression that broke it; the backtrace shows which test reached it. Miri stops at the first violation, so fix it and run again.

   Two more reports from wrong solutions of this chapter, so that you recognize them. A gap move with `copy_nonoverlapping` (lesson 07):

   ```text
   error: Undefined Behavior: `copy_nonoverlapping` called on overlapping ranges
   ```

   A `pop` that forgets to decrement `len` (lesson 08), so that one `String` is dropped twice. The type in the message depends on the value that was dropped twice; here it is the text buffer of the `String`:

   ```text
   error: Undefined Behavior: constructing invalid value of type &mut [u8]: encountered a dangling reference (use-after-free)
   ```

4. Check lesson 04 with strict provenance. By default Miri accepts a pointer that was rebuilt from an integer and only prints a warning. This flag turns that into an error, so it tells you whether your `Flagged` keeps its provenance:

   ```sh
   MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly-2026-11-20 miri test --test chapter23 l04_
   ```

   In PowerShell, set the variable first: `$env:MIRIFLAGS="-Zmiri-strict-provenance"`. A solution that tags the pointer through `as usize` and back is stopped with:

   ```text
   error: unsupported operation: integer-to-pointer casts and `ptr::with_exposed_provenance` are not supported with `-Zmiri-strict-provenance`
   ```

   *Checkpoint:* your lesson 04 passes with the flag.

5. Break one thing on purpose, once. In your finished lesson 07, change `ptr::copy` to `ptr::copy_nonoverlapping`, run `cargo +nightly-2026-11-20 miri test --test chapter23 l07_`, read the report, and change it back. Then note in NOTES.md what the ordinary debug run and the ordinary release run did with the same change.

6. Write down what Miri did *not* check:

   - code that no test executes, and inputs that no test uses;
   - other element types: a generic function was interpreted only for the types in the tests;
   - layout assumptions that happen to hold, such as lesson 09 task B without `repr(transparent)`;
   - other targets (pointer width, endianness).

   For each item, say which sentence of your SAFETY comments covers it.

Acceptance criteria:

- [ ] The nightly and Miri versions and the result of `cargo +nightly-2026-11-20 miri test --test chapter23` are recorded in NOTES.md.
- [ ] Lesson 04 passes with `-Zmiri-strict-provenance`.
- [ ] One deliberate violation was run, read and reverted.
- [ ] The list of unchecked cases is written, each with the argument that covers it.

If you cannot install nightly-2026-11-20, the chapter still works: the technique tests and your written arguments remain, and the mastery track module on unsafe Rust comes back to Miri.

## Lab 2 · Safety proof and UB boundary

Goal: write the safety argument for every unsafe block in your finished lessons.

1. For every unsafe block in l01 to l09, list its obligations under these headings: allocation/provenance, in-bounds range, alignment, initialization/validity, aliasing, lifetime, and ownership.
   *Checkpoint:* each heading has an entry for each block, even when it is "not applicable, because ...".
2. Mark which obligations follow from a caller's contract (an `unsafe fn`'s `# Safety` section) and which from a private invariant of a safe type.
3. Compare the three places a contract can live: an `unsafe fn` (caller obligations), an `unsafe trait` (implementor obligations), and a safe API that contains unsafe code (implementation obligations). Use one example of each from this chapter.
4. Distinguish panics and compiler errors from undefined behavior. Explain why debug assertions cannot make a missing safety contract sound.
5. Add a `// SAFETY:` comment at every unsafe call site. Audit the unsafe operations inside every `unsafe fn` as well: Rust 2024 warns by default (`unsafe_op_in_unsafe_fn`) when an unsafe operation inside an `unsafe fn` is not wrapped in its own `unsafe {}` block. Treat that warning as an error in your own code.
   *Checkpoint:* `cargo clippy --test chapter23 -- -W clippy::undocumented_unsafe_blocks` reports nothing for your finished chapter. The model solutions pass this check.

Acceptance criteria:

- [ ] Every unsafe block has a written obligation list under all seven headings.
- [ ] Each obligation is attributed to a caller contract or to a private invariant.
- [ ] The unsafe fn / unsafe trait / safe-API comparison is written, with examples.
- [ ] The panic versus UB distinction and the debug-assertion argument are written.
- [ ] SAFETY comments everywhere, and explicit unsafe blocks inside unsafe fns.

## Labs 3 to 5: an isolated package

Labs 3 to 5 build a package outside this repository. One minimal manifest serves all of them (standard library only):

```toml
[package]
name = "unsafe-lab"
version = "0.1.0"
edition = "2024"

[dependencies]
```

Run its tests under Miri as well: `cargo +nightly-2026-11-20 miri test` inside that package.

## Lab 3 · Extend the gap buffer to non-Copy values

Goal: a `Gap<T>` that owns arbitrary values, built from the byte version in l07.

1. In the isolated package, define `Gap<T>` backed by allocated `MaybeUninit<T>` slots. The initialized regions are `[0, start)` and `[end, capacity)`. Keep it fixed-capacity for now.
   *Checkpoint:* `Gap::<String>::with_capacity(4)` constructs and drops without touching any slot.
2. Implement cursor movement with an overlap-aware `ptr::copy`.
3. Implement insertion. A rejected insertion (buffer full) returns its input to the caller.
4. Implement deletion, which returns ownership with `ptr::read`.
5. Implement iteration over both initialized regions. Never create a `&[T]` that spans the gap.
6. Implement `Drop` so it drops exactly the live regions, once each.
   *Checkpoint:* a per-element Drop counter shows each inserted value dropped exactly once, whether it was deleted, returned, or still inside at drop time.
7. Verify that no user code, panicking indexing, or allocation can run after a move but before the ownership ranges are corrected. Annotate each move site.
8. Only after a complete ownership ledger, add growth.
9. Explain why the copied representation bits that remain inside the gap are not a second live `T`.

Acceptance criteria:

- [ ] A sequence of edits matches a `Vec<String>` model after every step.
- [ ] Empty and full capacity.
- [ ] Cursor at both endpoints.
- [ ] A zero-sized type with Drop.
- [ ] A rejected insertion keeps its input.
- [ ] Per-element Drop counters prove exactly-once drops.
- [ ] The tests pass under Miri, with no leak report.
- [ ] Written explanation of stale bits in the gap, and the move-site audit from step 7.

## Lab 4 · Panic safety under user callbacks

Goal: keep the container valid when user code panics in the middle of an operation.

1. Add `try_extend` to your l08 `Slots` (or to `Gap<T>`). Test it with an iterator that panics after yielding two elements.
2. Catch the panic outside the container with `std::panic::catch_unwind`.
   *Checkpoint:* after the panic, the existing prefix is valid, the two inserted values are in the container, and the remaining iterator-owned values follow their own ownership rules.
3. Make this hold by construction: use a guard, or update the initialized length immediately after each successful write.
4. Only after identifying what happens if the predicate panics, add a `retain` (filter) operation.
5. For each operation, state whether it gives the basic or the strong exception guarantee.
6. Consider a panic in a destructor during unwinding. It may leak the remaining elements, but it must never cause a double drop or an invalid read. Do not put TODOs or assertions in Drop scaffolding.

Acceptance criteria:

- [ ] A `catch_unwind` test shows the prefix valid and each successfully inserted value dropped exactly once.
- [ ] `retain` is tested with a panicking predicate, and its behavior is written down.
- [ ] Basic/strong guarantee stated for every operation.
- [ ] Destructor-panic behavior explained: leaks allowed, double drops never.

## Lab 5 · Tagged-pointer transfer exercise

Goal: two tag bits, with a written alignment proof.

1. Extend the l04 flagged borrow to two flag bits, only for an explicitly aligned owned type (`#[repr(align(4))]` or greater).
2. Preserve provenance with `map_addr` and lifetimes with `PhantomData`. Mask *all* tag bits before every access.
   *Checkpoint:* all four flag combinations round-trip, and the masked pointer equals the original address.
3. Write the alignment proof: why the two low bits of every valid pointer to your type are zero.
4. Discuss strict versus exposed provenance. Explain why printing an address differs from reconstructing an arbitrary usable pointer.

Acceptance criteria:

- [ ] A compile-fail check showing the lifetime cannot escape (keep the error; do not use `'static`).
- [ ] All flag combinations tested, also under Miri with `-Zmiri-strict-provenance`.
- [ ] Address identity tested.
- [ ] Written alignment proof and provenance discussion.

References: [The Rustonomicon](https://doc.rust-lang.org/nomicon/), [std::ptr and provenance](https://doc.rust-lang.org/std/ptr/index.html), [MaybeUninit](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html), [Behavior considered undefined](https://doc.rust-lang.org/reference/behavior-considered-undefined.html), and [Miri](https://github.com/rust-lang/miri).
