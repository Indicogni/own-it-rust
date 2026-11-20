# Capstone "nostd_collections": rubric

Green tests show that your crate behaves correctly in the cases the tests
run. They do not show that your unsafe code is sound for the cases they do
not run, or that somebody else could maintain it. This rubric covers the
rest. Use it to review your own work, or give it to a reviewer together
with your code and your `EVIDENCE.md` (SPEC section 13).

## How to score

Each criterion gets 0 to 3 points.

| Points | Meaning |
| --- | --- |
| 0 | Missing, or wrong in a way that matters. |
| 1 | Present, but with a gap that you can name. |
| 2 | Solid. A reviewer finds only small things. |
| 3 | You could defend every line, and you have evidence. |

There are 12 criteria, so 36 points.

| Total | Result |
| --- | --- |
| 30 to 36, and no 0 anywhere | Mastery. This is work you can show. |
| 22 to 29, and no 0 in criteria 1 to 5 | Pass. Note what you would fix next. |
| below 22, or a 0 in criteria 1 to 5 | Not yet. Fix the lowest criteria first. |

Criteria 1 to 5 are about correctness and soundness. A library with
unsafe code that is unsound is not finished, however good the rest is.

## The criteria

### 1. Correctness: the acceptance tests pass

| Points | What a reviewer sees |
| --- | --- |
| 0 | Fewer than four milestones pass. |
| 1 | M1 to M5 pass in the debug profile. |
| 2 | `cargo test` and `cargo test --release` pass completely, including the documentation tests of SPEC.md. |
| 3 | The same, three runs in a row in each profile, with the summary lines in `EVIDENCE.md`. The module `spec_examples` is still in place, so the programs of SPEC section 7 are really checked. |

### 2. Correctness: Miri is clean

| Points | What a reviewer sees |
| --- | --- |
| 0 | Miri was not run, or it reports undefined behaviour. |
| 1 | Miri is clean for some test files; others were not run or report leaks. |
| 2 | `cargo +nightly-2026-11-20 miri test` is clean for all test files and the documentation tests; the output and the Miri version are in `EVIDENCE.md`. |
| 3 | The same, also with `-Zmiri-tree-borrows`, and with several seeds for the arena tests (`-Zmiri-many-seeds`). Your own tests (criterion 11) run under Miri too. |

### 3. Design: invariants are written down and kept

Open each type and look for its invariant.

| Points | What a reviewer sees |
| --- | --- |
| 0 | No invariant is stated anywhere. |
| 1 | An invariant is stated for some types, or it is vague ("the buffer is valid"). |
| 2 | Every type that contains `MaybeUninit` or `UnsafeCell`, *including the iterator types*, has an invariant that says exactly which slots or bytes are initialised and owned, in terms of its fields. |
| 3 | The same, and for any function you can show the lines between which the invariant does not hold, and why nothing can observe the type there (no call into code of the caller, no destructor, no panic). |

### 4. Unsafe hygiene: small, placed and justified

Count your `unsafe` blocks and read each `SAFETY` comment as a stranger
would.

| Points | What a reviewer sees |
| --- | --- |
| 0 | `unsafe` blocks without comments, or comments that state no fact ("this is safe"). `unsafe_op_in_unsafe_fn` was switched off. |
| 1 | Every block has a comment, but some comments describe *what* the code does and not *why it is sound*. Or `unsafe` is used where safe code does the same (indexing, swapping two slots, splitting a slice). |
| 2 | Every comment names the facts it relies on and points to the invariant. Every `unsafe fn` (if you have any) has a `# Safety` section. Each unsafe block contains one operation, or a few that share one argument. |
| 3 | The same, and unsafe code appears only where it is needed: reading a slot as a `T`, moving several slots, and the arena. The `RingBuffer` iterators, `Clone`, `Extend`, `PartialEq` and `Debug` contain none. There is no `unsafe impl Send` or `Sync`, and no `transmute` between types of different layout. The inventory in `EVIDENCE.md` matches the code. |

For orientation: the reference has 37 `unsafe` blocks in about 1400
lines of library code (comments included), no `unsafe fn` and no
`unsafe impl`.

### 5. Panic safety and leak safety

The tests check the cases of SPEC 4.3, 4.5, 4.6, 4.8, 5.4, 5.6 and 6.4.
This criterion is about the reasoning.

| Points | What a reviewer sees |
| --- | --- |
| 0 | A panic in a closure, a `clone` or a destructor can lead to a double drop or to a read of an uninitialised slot. |
| 1 | No double drop, but elements leak after a panic where the specification forbids it, or a function repairs its state in a `catch_unwind`-like way that does not work in `no_std`. |
| 2 | Every function that runs foreign code (closures, `Clone`, `Drop`, iterators of the caller) keeps the type valid at that moment, by ordering its steps or with a guard whose `Drop` repairs the state. `Drain` is safe to leak. |
| 3 | The same, and for each such function a comment says what happens on a panic. `EVIDENCE.md` lists the places for three functions (SPEC 13, item 5), and you have a test of your own for at least one panic path that the given tests do not cover (for example a destructor that panics inside `retain` or inside the drop of a `Drain`). |

### 6. Design: the soundness argument of the arena

| Points | What a reviewer sees |
| --- | --- |
| 0 | `alloc` creates a reference to the whole storage, or its returned lifetime is not tied to the borrow of the arena. |
| 1 | The code is sound (Miri is clean), but nothing says why. |
| 2 | The documentation of the type answers four questions: why two returned references never overlap; why a reference cannot outlive the bytes; why `reset` cannot run while a reference is alive; why two threads cannot allocate at once. |
| 3 | The same, and each answer points to a field type or a signature that enforces it. All pointer arithmetic is in one private function, uses checked arithmetic, and keeps the provenance of the storage pointer (no pointer is built from an integer). You can say what would break if the storage were `[u8; N]` and not `[MaybeUninit<u8>; N]`. |

### 7. Correctness at the edges

| Points | What a reviewer sees |
| --- | --- |
| 0 | `N == 0` or a zero-sized element type panics, divides by zero, or is refused. |
| 1 | The tests for these cases pass, through special cases spread over many functions. |
| 2 | Zero-sized types and `N == 0` need at most one special case per type (the arena has one for size 0; the ring buffer has one in `push_overwrite`). No arithmetic can overflow, and you can say why for each addition. |
| 3 | The same, and you tested more than the given tests do: a 32-bit target (for example Miri with `--target i686-unknown-linux-gnu`), an element type with alignment 32 or more in both collections, and a `static` or `const` built with `new`. |

### 8. Idiom: the API feels like the standard library

| Points | What a reviewer sees |
| --- | --- |
| 0 | Signatures differ from SPEC.md, or public items have no documentation. |
| 1 | The specified API is there, with documentation that repeats the function names. |
| 2 | Every public item has documentation that states panics (`# Panics`) and, where they exist, the leak or panic guarantees. Every public type implements `Debug`, also the iterator types. Panicking functions carry `#[track_caller]`, so the message points to the caller's line. Iterators implement `FusedIterator` and give an exact `size_hint`. |
| 3 | The same, and the crate enables `missing_docs` and `missing_debug_implementations` as warnings and has none. Names of extra items follow the standard library (`Drain`, `IntoIter`, `Iter`, `IterMut`). `CapacityError` has no bounds it does not need. Nothing public exposes a field or a `MaybeUninit`. |

### 9. Design: the structure of the crate

| Points | What a reviewer sees |
| --- | --- |
| 0 | Everything is still in the one facade file, with `Todo` or `#![allow(unused_variables)]` left in it. |
| 1 | The code is split into files, but logic is duplicated: the wrap-around computation appears in several functions, or the shared and the mutable slice view compute their ranges separately. |
| 2 | One module per type. Each type has one or two private helpers that hold its arithmetic (slot of an index; ranges of the contents; reserve a layout), and every other function goes through them. Higher-level functions are written with lower-level ones (`push` with `try_push`, `clear` with `truncate`, `Clone` with `push`). |
| 3 | The same, and a reader can check each unsafe block by reading only the invariant and the function it is in. The crate root is a short list of modules and re-exports. `Cargo.toml` has no dependencies and no features that are not used. |

### 10. Performance: no hidden costs

The capstone has no speed target, but a fixed-capacity collection is
chosen for predictable cost. Check your code against this table.

| Operation | Expected cost |
| --- | --- |
| `ArrayVec`: `push`, `pop`, `swap_remove`, `len` | constant |
| `ArrayVec`: `insert`, `remove` | one block move of the elements behind the index |
| `ArrayVec`: `retain` | one pass; each kept element is moved at most once |
| `ArrayVec`: `drain` | the range, plus one block move of the tail |
| `RingBuffer`: every push, pop, `get`, `as_slices` | constant, without a division |
| iterators: `next`, `next_back`, `len` | constant |
| `Arena`: `alloc` | constant; `alloc_slice_fill_with` is linear in `len` |
| `new` of all three types | no initialisation of the storage |

| Points | What a reviewer sees |
| --- | --- |
| 0 | An operation is in a worse class than the table says (for example, `pop_front` moves every element), or elements are cloned where they could be moved. |
| 1 | The classes are right, but there is avoidable work: the storage is zero-filled in `new`, a slot costs more than `size_of::<T>()`, or the ring buffer divides on every access. |
| 2 | The code meets the table, and the size test of SPEC 8.1 passes. Elements are moved with block copies, not one by one in a loop, where a block move is possible. |
| 3 | The same, and you measured or inspected something and wrote it into `EVIDENCE.md`: a benchmark of your `ArrayVec` against `Vec::with_capacity` and your `RingBuffer` against `VecDeque` (mastery module M7 shows how), or the optimised machine code of `push` for a small element type. |

### 11. Tests of your own

The acceptance tests are given. This criterion is about what *you* added.

| Points | What a reviewer sees |
| --- | --- |
| 0 | No tests of your own. |
| 1 | A few unit tests that repeat what the acceptance tests check. |
| 2 | Unit tests for your private helpers (the slot arithmetic with `N` in `{0, 1, 2, usize::MAX}`, the range resolution of `drain`, the placement function of the arena), with messages that explain a failure. |
| 3 | The same, plus tests for paths that the acceptance tests reach only once or not at all, each run under Miri: a destructor that panics in every function that drops, a `Drain` used from both ends and then leaked, nested calls into the arena from a fill closure that itself panics. Or a property test that compares with `Vec` and `VecDeque` and shrinks a failure (mastery module M9). |

### 12. Evidence that it is really `no_std`

| Points | What a reviewer sees |
| --- | --- |
| 0 | The crate uses `std` or `alloc`. |
| 1 | `#![no_std]` is there and the no-heap test passes, but the crate was never built for a target without an operating system. |
| 2 | `cargo build --target aarch64-unknown-none` succeeds (or another bare-metal target), in debug and release, and the commands and their output are in `EVIDENCE.md`. |
| 3 | The same for a 32-bit target as well, and the crate is used by a real `#![no_std]` program: the kernel capstone, firmware, or a small `no_main` binary of your own that puts an `ArrayVec` into a `static`. |
