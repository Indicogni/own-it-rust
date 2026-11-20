# Reference `nostd_collections`: design notes

This is the model answer for the capstone in `capstones/nostd_collections/`.
The files in this directory mirror that directory: copy them over a fresh
copy of the capstone and every acceptance test passes.

It is one possible design. Its main rule is **safe code first**: unsafe
code appears only where a slot is read as a value, where several slots
move at once, and in the arena. The result is about 1400 lines with
comments, 37 `unsafe` blocks, no `unsafe fn`, and no `unsafe impl`.

## Reading order

| File | Role |
| --- | --- |
| `src/lib.rs` | The modules and the four re-exports. |
| `src/error.rs` | `CapacityError<T>`. |
| `src/array_vec.rs` | `ArrayVec`, its by-value iterator `IntoIter`, and `Drain`. |
| `src/ring.rs` | `RingBuffer` and its iterators `Iter`, `IterMut`, `IntoIter`. |
| `src/arena.rs` | `Arena`. |
| `src/spec_examples.rs` | Unchanged from the starter: runs SPEC.md as documentation tests. |

Each type starts with its invariant. Every `SAFETY` comment refers to it.

## Unsafe inventory

| File | `unsafe` blocks | What they do |
| --- | --- | --- |
| `array_vec.rs` | 21 | Read, borrow or drop slots that the invariant says are initialised (15); block moves in `try_insert`, `remove`, the `retain` guard and the `Drain` guard (4); drop a slice of elements in place (1, in `truncate`); turn a full storage into `[T; N]` (1). |
| `ring.rs` | 8 | Read or borrow a slot (4); turn the two slot ranges into slices of `T` (2); drop the two slices in place (2). |
| `arena.rs` | 8 | Offset the storage pointer (1); write a value and make the reference to it (2); copy a slice and make the reference (2); `str` from bytes that came from a `str` (1); write one element and make the slice in `alloc_slice_fill_with` (2). |
| `error.rs`, `lib.rs` | 0 | |

## Decisions

### 1. Storage is an array of slots: `[MaybeUninit<T>; N]`

Both collections store `[MaybeUninit<T>; N]` plus one or two `usize`
fields. The invariant says which slots are initialised.

Alternatives:

- `[Option<T>; N]` needs no unsafe code at all. It was rejected because a
  slot then costs more than a `T` for most types, and because the elements
  are not a `[T]` in memory, so `Deref<Target = [T]>` and `as_slices`
  cannot exist. SPEC 8.1 excludes it.
- `MaybeUninit<[T; N]>` (one uninitialised array) has the same layout. But
  every access then goes through a pointer cast and pointer arithmetic.
  With an array *of slots*, indexing is bounds-checked safe code, and so
  are `swap`, `split_at_mut` and `mem::replace` of the whole storage.
- A raw pointer plus `PhantomData<T>` is what a heap vector needs. Here it
  would only add work: variance, `Send`, `Sync` and the drop check would
  have to be argued by hand. With the array field, the compiler derives
  all of them correctly.

`new` uses `[const { MaybeUninit::uninit() }; N]`. The inline `const`
block makes the array expression legal for a `T` that is not `Copy`, and
legal in a `const fn`.

### 2. Safe code where it does the job

Several functions that are usually written with pointer reads are safe
code here:

| Function | How |
| --- | --- |
| `ArrayVec::try_push`, `RingBuffer::push_*` | `self.buf[i].write(value)`: indexing checks the bound, `write` does not drop the old content. |
| `ArrayVec::swap_remove` | `self.buf.swap(index, last)`, then one read. |
| `ArrayVec::retain`, moving a kept element down | `self.buf.swap(hole, slot)`. |
| `ArrayVec::into_iter` | `mem::take` the length and `mem::replace` the storage with a fresh uninitialised one. The emptied vector is then dropped normally. |
| `From<[T; N]>` | `array.map(MaybeUninit::new)`. |
| `RingBuffer::as_mut_slices` | `split_at_mut(head)` separates the two runs. |
| `RingBuffer` iterators | Two slice iterators; the by-value iterator owns the buffer and pops. |
| `Clone`, `Extend`, `FromIterator` | Built with `push` / `push_back`. |

The cost: a `swap` of two slots is three moves where a raw copy is one,
and the bounds checks stay in. For a container whose point is predictable
behaviour, that is an easy trade. The block moves in `insert`, `remove`,
and at the end of `retain` and `drain` remain `ptr::copy`: a loop of swaps
there would be slower and no easier to read.

### 3. Panic safety: order first, guards second

Three kinds of foreign code run inside the collections: closures
(`retain`), `Clone::clone`, and destructors. At each of these points the
type must be valid, because a panic unwinds straight out.

- **Order.** `truncate`, `clear` and the ring buffer's `clear` lower the
  length *before* they run destructors. If a destructor panics, the
  collection does not claim a dropped element. The elements are dropped
  with `ptr::drop_in_place` on a slice, which keeps dropping the remaining
  elements while it unwinds. A hand-written loop would stop at the panic
  and leak the rest.
- **Guards.** Where the repair is more than one assignment, a private
  struct with a `Drop` implementation does it, so the same code runs on
  normal exit and on unwinding:
  - `retain`: `Guard` closes the holes and sets the final length. The
    counters are raised *before* a rejected element is dropped, so a
    panicking destructor cannot make the guard count it as alive. A panic
    of the closure leaves the current element among the "not yet seen",
    which gives the state that SPEC 4.5 demands.
  - `Drain::drop`: `MoveTail` moves the tail down even if dropping the
    rest of the range panics.
  - `RingBuffer::clear`: `DropSlice` drops the second run even if a
    destructor in the first one panics.
- **Ordinary locals.** `Clone`, `Extend` and `FromIterator` build a
  normal `ArrayVec` with `push`. If something panics, unwinding drops that
  local. No guard is needed.

### 4. `drain` is safe to leak

`drain` sets the vector's length to the start of the range before it
returns. While the iterator lives, the vector claims only the elements in
front of the range; the iterator owns the range and is responsible for the
tail. If the iterator is leaked, the range and the tail are leaked with
it. That is the price for never having an element with two owners, and it
is what `Vec::drain` does.

`Drain` holds `&'a mut ArrayVec<T, N>`. That makes the borrow obvious, but
it also makes `Drain` invariant in `T`. The standard library's `Drain`
uses a `NonNull` and a slice iterator to stay covariant. For this crate
the simpler type was chosen; the difference would matter only to someone
who needs to shorten a lifetime inside a live `Drain`.

### 5. Ring buffer: front slot and length, and no division

The fields are `head` (the slot of the front) and `len`.

- "Front slot and back slot" needs one more trick, because an empty and a
  full buffer then have equal indices: a wasted slot, a flag, or indices
  that run to `2 * N`. With a length there is nothing to disambiguate.
- Capacities that are powers of two allow masking instead of wrapping.
  SPEC allows any `N`, so this was not an option.

All wrap-around arithmetic is in two private functions. `slot(offset)`
compares the offset with the room before the end of the storage and
subtracts; it never computes `head + offset` when that passes the end. So
it does not divide (which also covers `N == 0`), and it cannot overflow
for zero-sized elements, where `N` may be `usize::MAX`. `ranges()` returns
the two slot ranges of the contents; `as_slices`, `as_mut_slices`, the
iterators and `clear` all use it.

`Iter` is a pair of slice iterators and not a pair of indices. The slice
iterators already are double-ended, exact, and correct for zero-sized
types; `Iter` only decides which of the two to ask. `Chain` would do the
same but is not an `ExactSizeIterator`.

`clear` sets `head` back to 0. SPEC leaves this open; resetting makes a
cleared buffer behave like a new one, and it costs nothing.

`clone` produces a buffer that starts at slot 0. Cloning the slot layout
as well would need unsafe code and gives the caller nothing: equality and
iteration do not depend on the layout.

### 6. Arena

```text
struct Arena<N> {
    storage: Storage<N>,   // #[repr(C, align(16))] UnsafeCell<[MaybeUninit<u8>; N]>
    used: Cell<usize>,
}
```

- **`UnsafeCell`** is the only sound way to write through `&self`.
  **`MaybeUninit<u8>`** and not `u8`, because the bytes start
  uninitialised and because the padding inside stored values stays
  uninitialised; a `[u8; N]` would promise initialised bytes.
- **The alignment wrapper** makes offset 0 a multiple of 16, which SPEC
  6.2 requires. Without it, the padding before a `u64` would depend on
  where the caller put the arena.
- **One function, `reserve(Layout)`, does all pointer arithmetic.** It
  takes a raw pointer from the `UnsafeCell`, computes the padding from the
  real address (so alignments above 16 work as well), checks the bounds
  with checked additions, moves `used`, and returns a pointer derived from
  the storage pointer. It never creates a reference to the storage. That
  is the point Miri checks: a `&mut` to the whole array would invalidate
  every reference handed out before.
- **Zero-sized layouts** return early with a well-aligned address that is
  not null (the alignment itself, as an address without provenance). They
  do not read `used`.
- **`alloc_slice_fill_with` reserves first, then fills.** The closure can
  therefore allocate from the same arena. The slice reference is created
  only after the last element is written; if the closure panics, the room
  stays taken and no reference to it exists.
- **No destructors.** A bump allocator has no list of what it holds.
  Running destructors would also be unsound for values that refer to each
  other, unless the stored types are `'static`.
- **`Send` and `!Sync` are derived.** `Cell` and `UnsafeCell` are `Send`
  and not `Sync`, so the arena is too. Moving an arena to another thread
  is fine even if it holds bytes of a value that is not `Send`: nobody can
  reach those bytes, because the move required that no reference was
  alive.

The soundness argument for `&mut T` from `&self` is in the documentation
of the type.

Alternatives:

- *Bump downwards.* Starting at the end and rounding the address down
  makes the alignment step one mask operation. It was not used because
  SPEC fixes ascending offsets, which are easier to explain and to test.
- *Borrow the buffer* (`Arena<'buf>` over `&'buf mut [MaybeUninit<u8>]`).
  This is more flexible on a microcontroller, where the memory is often a
  `static` region, and moving the arena handle would be allowed while
  references are alive. It adds a lifetime parameter to every user.
- *A typed arena* (`Arena<T, N>`) has no alignment arithmetic and can run
  destructors. It cannot mix types.
- *Clippy's `mut_from_ref`* is expected on the four allocation functions,
  with a reason. `#[expect]` is used and not `#[allow]`, so the attribute
  reports itself if it ever becomes unnecessary.

### 7. One error type

`CapacityError<T>` serves the vector, the ring buffer and the arena. It
returns the value, like `std::sync::mpsc::SendError`. `Debug` is written
by hand without a bound on `T`, so that `unwrap` works for every element
type; it does not print the value. Returning the bare value
(`Result<(), T>`) would be smaller, but the error could then not
implement `Error` or be used with `?` and `Box<dyn Error>`.

### 8. Which functions panic

`ArrayVec::push` and `insert` panic when the vector is full, like
indexing out of bounds: for a vector, running out of room is usually a
sizing bug, and `try_push` exists for the other case. `RingBuffer` has
only fallible pushes, because a full queue is normal operation.
All panicking functions carry `#[track_caller]`, so the message names the
caller's line.

## What I would look at next

- `ArrayVec::into_iter` and `clone` move the whole storage (`N` slots),
  whatever the length is. For a large `N` and a short vector, copying only
  the initialised prefix would be faster and would need unsafe code.
- More functions could be `const fn` (`try_push` for example), which would
  allow building a table at compile time.
- `Drain` could be made covariant (decision 4).
- `Extend` could look at `size_hint` and panic before it consumes an
  iterator that cannot fit. The current behaviour (push until full) is
  what SPEC 4.8 fixes.

## Evidence

Measured on macOS (aarch64) with rustc 1.99.0 and, for Miri,
`miri 0.1.0 (6905905bb7 2026-11-19)` on rustc 1.102.0-nightly.

| Check | Result |
| --- | --- |
| `cargo test`, three runs | 125 integration tests and 19 documentation tests pass each time |
| `cargo test --release`, three runs | the same |
| `cargo clippy --all-targets -- -D warnings` | no warnings |
| `cargo +nightly-2026-11-20 miri test` | clean (Stacked Borrows, the default) |
| `MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly-2026-11-20 miri test` | clean |
| `cargo +nightly-2026-11-20 miri test --target i686-unknown-linux-gnu` | clean (32-bit; `u64` has alignment 4 there) |
| `MIRIFLAGS=-Zmiri-many-seeds=0..8 cargo +nightly-2026-11-20 miri test --test m5_arena` | clean for all eight seeds |
| `cargo +nightly-2026-11-20 test --doc` | the seven programs of SPEC section 7 fail with the expected error codes |
| `cargo build --target aarch64-unknown-none`, debug and release | builds |

Twenty-six wrong variants of this code were also run against the tests,
one at a time. Each one fails at least one acceptance test:

- `ArrayVec`: the length lowered after the destructors; a `Drop` that
  drops element by element and stops at a panic; a `retain` guard that
  forgets the unseen elements; a `retain` that drops the rejected elements
  at the end; a drain that keeps the old length while it lives; a drain
  whose inclusive end wraps at `usize::MAX`; a `clone` that sets the
  length at the end; a by-value iterator without `Drop`; `try_insert` that
  checks the capacity before the index; a flag per slot.
- `RingBuffer`: slot arithmetic with `%`; drop in slot order; `clear`
  without the guard for the second run; a `Drop` that pops in a loop and
  stops at a panic; a front slot that goes back to 0 when the buffer
  becomes empty; a `push_front` that treats an empty buffer like
  `push_back`; equality that compares the two slices pair-wise; an
  iterator that never ends (the time limit of the tests reports it after
  20 seconds); an iterator that is `Clone` only for `T: Clone`.
- `Arena`: a fill function that reserves after filling; an unchecked
  multiplication; storage without the alignment; padding computed from the
  offset for alignments above 16; zero-sized values placed like others;
  `unsafe impl Sync`; `alloc` with a lifetime that the caller chooses.

One more variant passes every normal test and is found only by Miri: an
arena that creates `&mut` to its whole storage.
