# Mastery M2 concepts — Unsafe Rust by rebuilding the standard library

[Module guide](README.md) · [Hints](HINTS.md) · [Notes](NOTES.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every `rust` block in this file was compiled and
run with the course toolchain (Rust 1.99.0, edition 2024) and then run again
under Miri. Every compiler message was copied from that compiler, and every
Miri report from the nightly toolchain named in the [module guide](README.md).
Long messages are shortened: a line with `...` marks a cut, and the end of a
message is often left out. The numbers in a Miri report (`<565>`, `alloc208`)
name pointers and allocations; they can differ on your machine.

## What you will be able to do

After this module you can:

- state the invariant of an unsafe data structure, and check each `unsafe`
  block against it;
- allocate, grow and free memory with `Layout`, and handle zero-sized types;
- move values with `ptr::read`, `ptr::write` and `ptr::copy` without dropping
  a value twice or reading one that is not there;
- keep a structure valid when a destructor panics or when a guard is leaked;
- explain what a reference promises about aliasing, and read a Miri report;
- decide when a type may be `Send` or `Sync`, and write down why;
- build `Cell`, `RefCell`, `Box`, `Vec`, `IntoIter`, `Drain`, `Rc`, `Weak`, a
  linked queue, a spin lock and a bump arena from raw allocation.

## Before you start

Chapters 4 and 5 (ownership, moves, references),
chapter 9 (`Cell` and `RefCell` as a user), chapter 13 (`Drop`, `Deref`,
`Clone`), chapter 15 (the `Iterator` traits), chapter 19 (threads, `Send` and
`Sync` as a user) and all of chapter 23: raw pointers, `unsafe fn`, SAFETY
comments, `ptr::read` and `ptr::write`, `MaybeUninit`, and a first Miri run.
This module does not repeat chapter 23; it builds on it.

## Time

These are estimates for a careful first pass, including the Miri
lab and the notes: lessons 01 and 02 about 3 hours each, lesson 03 about 5,
lesson 04 about 4, lesson 05 about 5, lessons 06 and 07 about 4 each, the
capstone (lesson 08) about 8, the code review about 3. About 40 hours in all.

## Words used in every section

- An *invariant* of a type is a statement about its fields that is true
  between any two calls of its methods. Safe code cannot break it, because the
  fields are private. Each `unsafe` block is justified by pointing at it.
- A type is *sound* if no program that uses it without `unsafe` can cause
  undefined behaviour. One wrong method makes the whole type unsound.
- *Undefined behaviour* (UB) means that the language makes no promise about
  what the program does. It can print the right result today and a wrong one
  after the next compiler update.
- *Miri* is an interpreter for Rust that checks every memory access while it
  runs your program. It reports UB that actually happens on the path it runs.
  It does not prove that other paths are fine.

## Lesson 01 · UnsafeCell: building Cell and RefCell

**What is assumed.** You have used `Cell` and `RefCell` (chapter 9) and you
know that `&T` is a shared, read-only reference (chapter 5).

**The model.** A `&T` is a promise in two directions. You promise not to write
through it. The compiler in turn assumes that the bytes behind it do not
change while the reference is alive, and it uses that: it may read a value
once and reuse it, or decide that a branch can never be taken. So "I know
that nobody else is looking" is not enough to write through a `&T`. The write
itself is the undefined behaviour.

There is exactly one exception, and it is a type: `UnsafeCell<T>`. Bytes
inside an `UnsafeCell` are excluded from the promise. A `&UnsafeCell<T>` says
"this may change behind my back", and its method `get` returns a `*mut T`
that you are allowed to write through. Every type with *interior mutability*
(`Cell`, `RefCell`, `Mutex`, the atomics) contains an `UnsafeCell`. It has the
same size and layout as `T`, with two visible effects: it is never `Sync`,
and its content cannot lend a spare bit pattern to an enclosing `Option`
(prediction P1 of the lesson).

`UnsafeCell` removes one promise only. The other rules still hold for
whatever you build: at most one `&mut T` to the inside at a time, never
together with a `&T`, and no data race. The standard cells are two different
strategies for keeping those rules:

| Type | Strategy | Cost |
| --- | --- | --- |
| `Cell<T>` | Never give out a reference to the inside. Values only move in and out. | None at run time. |
| `RefCell<T>` | Give out references, and count them while the program runs. | A counter, and a panic when the rules are broken. |

Both strategies only work on one thread, which is why `UnsafeCell` (and so
everything built from it without extra work) is not `Sync`. Lesson 07 covers
the multi-threaded strategies.

**A worked example.** A slot that can be filled once. It is neither of the
two cells: it does give out `&T`, and it stays sound without a counter,
because of a different invariant: *once the slot is full, it is never written
again.*

```rust
use std::cell::UnsafeCell;

/// A slot that can be filled once, through a shared reference.
pub struct OnceSlot<T> {
    value: UnsafeCell<Option<T>>,
}

impl<T> OnceSlot<T> {
    pub const fn new() -> OnceSlot<T> {
        OnceSlot { value: UnsafeCell::new(None) }
    }

    /// The value, if the slot has been filled.
    pub fn get(&self) -> Option<&T> {
        // SAFETY: the pointer is valid, because it points into `self`. Shared
        // references to the inside may coexist. The only write is in `fill`,
        // and it happens only while the slot is empty, when no `&T` into the
        // slot can exist. `OnceSlot` is not `Sync`, so no other thread is
        // inside `fill` right now.
        unsafe { (*self.value.get()).as_ref() }
    }

    /// Fills the slot, or gives the value back if the slot is already full.
    pub fn fill(&self, value: T) -> Result<(), T> {
        if self.get().is_some() {
            return Err(value);
        }
        // SAFETY: the slot is empty, so `get` has never returned a reference
        // into it, and no reference to the inside exists. Overwriting `None`
        // drops nothing, so no code of `T` runs during the write.
        unsafe { *self.value.get() = Some(value) };
        Ok(())
    }
}

fn main() {
    let slot = OnceSlot::new();
    assert!(slot.get().is_none());
    slot.fill(String::from("first")).unwrap();
    let seen: &String = slot.get().unwrap();
    // `seen` is still alive here. `fill` must not write, and it does not.
    assert_eq!(slot.fill(String::from("second")), Err(String::from("second")));
    assert_eq!(seen, "first");
}
```

Read the two SAFETY comments again. Neither says "this works". Each names the
facts that make the operation legal: the pointer is valid, and no conflicting
reference can exist, *because of the invariant*. That is the form you will use
in every lesson. If you cannot finish the sentence "no conflicting reference
exists, because ...", the code is not ready.

**Mistakes people really make.**

*1. Writing through a pointer made from `&T`.* The compiler recognises the
direct form and rejects it:

```rust,compile_fail
fn bump(count: &u32) {
    let writable = count as *const u32 as *mut u32;
    unsafe { *writable += 1 };
}
```

```text
error: assigning to `&T` is undefined behavior, consider using an `UnsafeCell`
 --> src/main.rs:3:14
  |
2 |     let writable = count as *const u32 as *mut u32;
  |                    ------------------------------- casting happened here
3 |     unsafe { *writable += 1 };
  |              ^^^^^^^^^^^^^^
  |
  = note: `#[deny(invalid_reference_casting)]` on by default
```

This lint only sees the cast and the write in one function. Hide the cast in a
helper and the code compiles. It is still undefined behaviour; the Miri lab
below shows that version.

*2. Running code of `T` while you hold a pointer into the cell.* This is why
`Cell::get` needs `T: Copy`:

```rust,compile_fail,E0599
use std::cell::Cell;

let tags = Cell::new(vec![1u8, 2]);
let copy = tags.get();
```

```text
error[E0599]: the method `get` exists for struct `Cell<Vec<u8>>`, but its trait bounds were not satisfied
 --> src/main.rs:4:21
  |
4 |     let copy = tags.get();
  |                     ^^^
  ...
  = note: the following trait bounds were not satisfied:
          `Vec<u8>: Copy`
```

A `get` for `T: Clone` would call `clone(&self)` with a reference into the
cell, and a `Clone` impl is ordinary code: it could call `set` on the same
cell and free the value that the reference points to. A `Copy` type is copied
byte by byte, and no code runs. The same reasoning applies to destructors.
Task A of the lesson has a test for it: `set` must move the old value out of
the cell *first* and drop it *afterwards*, because the old value's destructor
may use the cell again.

*3. Breaking the `RefCell` rules at run time.* The compiler accepts this
program; the counter stops it:

```rust,should_panic
use std::cell::RefCell;

let log = RefCell::new(vec![1]);
let first = log.borrow();
log.borrow_mut().push(2);
println!("{first:?}");
```

```text
thread 'main' panicked at src/main.rs:5:9:
RefCell already borrowed
```

**What Miri says about the wrong version.** `examples/miri_l01_cell.rs` is a
counter without `UnsafeCell`. Its helper `writable` hides the cast from the
lint. One native debug build of it printed `hits: 2`. The release build of the
same file failed its own assertion with `left: 0, right: 2`: the optimizer had
used the promise that the field behind `&Counter` does not change. Miri:

```text
error: Undefined Behavior: attempting a write access using <565> at alloc208[0x0], but that tag only grants SharedReadOnly permission for this location
  --> examples/miri_l01_cell.rs:42:18
   |
42 |         unsafe { *hits += 1 };
   |                  ^^^^^^^^^^ this error occurs as part of an access at alloc208[0x0..0x4]
   |
help: <565> was created by a SharedReadOnly retag at offsets [0x0..0x4]
  --> examples/miri_l01_cell.rs:23:5
   |
23 |     std::ptr::from_ref(shared).cast_mut()
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^
```

Read it from the bottom: the pointer got its permission where it was created,
from a shared reference, and that permission is "read only". Casting to
`*mut` changed the type and nothing else. Lesson 06 explains "tag" and
"retag".

**When to use which.** Prefer, in this order: no interior mutability at all
(`&mut`); `Cell` for small `Copy` values and for values you can swap;
`RefCell` when you need references to the inside and can accept a run-time
check; `OnceCell` for "set once"; `Mutex`, `RwLock` and atomics across
threads. Write your own type on `UnsafeCell` only when none of these can
express the invariant you have.

**Read more.**
[`UnsafeCell`](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html) ·
[the `std::cell` module](https://doc.rust-lang.org/std/cell/index.html) ·
[Interior mutability in the Reference](https://doc.rust-lang.org/reference/interior-mutability.html) ·
[Behavior considered undefined](https://doc.rust-lang.org/reference/behavior-considered-undefined.html)

## Lesson 02 · Layout, alloc, dealloc and NonNull: building Box

**What is assumed.** Raw pointers, `ptr::read` and `ptr::write` from chapter
23, and `Drop` from chapter 13.

**The model.** The global allocator has a small interface, and all of it is
`unsafe`:

- `alloc(layout) -> *mut u8` returns a block of `layout.size()` bytes whose
  address is a multiple of `layout.align()`, or null if it cannot. The bytes
  are uninitialized.
- `dealloc(ptr, layout)` gives the block back. `layout` must be the layout the
  block was allocated with. The allocator does not remember it for you.
- `realloc(ptr, layout, new_size)` changes the size, perhaps by moving the
  block. Lesson 03 uses it.

A `Layout` is a size and an alignment. `Layout::new::<T>()` is the layout of
one `T`; `Layout::array::<T>(n)` is the layout of `n` of them and fails if the
size does not fit in `isize`.

Three rules of the contract cause most bugs:

1. **Never ask for zero bytes.** `alloc` with a zero-sized layout is
   undefined behaviour. Types such as `()` and `[u64; 0]` have size zero, so
   an owning pointer needs a second representation for them.
2. **Check for null.** On failure, call `handle_alloc_error(layout)`. It does
   not return.
3. **Free with the same layout.** A box of `T` never stores its layout; it
   recomputes `Layout::new::<T>()`, which is why a `Box<T>` is one pointer.

`NonNull<T>` is a `*mut T` that is never null. Two things make it the right
field for an owning pointer. `Option<NonNull<T>>` is the size of one pointer,
because `None` can use the null value. And `NonNull<T>` is *covariant* in
`T`, like `Box<T>` and `&T`, while `*mut T` is invariant: with a `*mut T`
field, a `MyBox<&'static str>` could not be passed where a `MyBox<&'a str>` is
expected. For a zero-sized `T`, `NonNull::dangling()` gives a pointer that is
non-null and aligned and points to no allocation. That is all a read or write
of zero bytes needs.

A `PhantomData<T>` field tells the compiler that the struct owns a `T`
although it only holds a pointer. Without it nothing in the fields says that
dropping the struct drops a `T`. (For the drop checker a `Drop` impl has the
same effect today, so the field is mostly a statement of intent, and the
convention in every owning pointer type.)

```text
stack                         heap
+--------------+              +-----------------------------+
| ptr: NonNull-+------------> | T (size_of::<T>() bytes,    |
+--------------+              |    aligned to align_of::<T>) |
  MyBox<T>                    +-----------------------------+
  one pointer                 allocated with Layout::new::<T>()
                              freed with the same layout
```

**A worked example.** `Box<[u8]>` is aligned to 1. Some uses (direct I/O,
page tables) need a buffer aligned to 4096. That needs a layout of your own,
and here the layout must be stored, because it is not derived from a type:

```rust
use std::alloc::{self, Layout};
use std::ptr::NonNull;

/// `len` zeroed bytes at an address that is a multiple of 4096.
pub struct PageBuf {
    ptr: NonNull<u8>,
    layout: Layout,
}

impl PageBuf {
    pub fn new(len: usize) -> PageBuf {
        assert!(len > 0, "PageBuf needs at least one byte");
        let layout = Layout::from_size_align(len, 4096).expect("length too large");
        // SAFETY: the layout has a non-zero size.
        let raw = unsafe { alloc::alloc_zeroed(layout) };
        let ptr = NonNull::new(raw).unwrap_or_else(|| alloc::handle_alloc_error(layout));
        PageBuf { ptr, layout }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: the block is `layout.size()` bytes long and was zeroed, and
        // every bit pattern is a valid `u8`. `self` owns the block, so it
        // lives as long as this borrow, and `&mut self` makes the slice unique.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.layout.size()) }
    }
}

impl Drop for PageBuf {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from the global allocator with exactly this
        // layout in `new`, and it is freed only here.
        unsafe { alloc::dealloc(self.ptr.as_ptr(), self.layout) };
    }
}

fn main() {
    let mut page = PageBuf::new(100);
    assert_eq!(page.as_mut_slice().as_ptr().addr() % 4096, 0);
    page.as_mut_slice()[99] = 7;
    assert_eq!(page.as_mut_slice().iter().map(|&b| u32::from(b)).sum::<u32>(), 7);
}
```

`PageBuf` holds bytes, so it has nothing to drop before it frees. Your `MyBox`
holds a `T`: its destructor must first run the destructor of the value
(`drop_in_place`) and then free the block.

**Mistakes people really make.**

*1. Moving the value out by dereferencing.* A move through a raw pointer is
refused, with good reason: the original would still be there.

```rust,compile_fail,E0507
use std::ptr::NonNull;

struct Slot<T> {
    ptr: NonNull<T>,
}

impl<T> Slot<T> {
    fn take(self) -> T {
        unsafe { *self.ptr.as_ptr() }
    }
}
```

```text
error[E0507]: cannot move out of a raw pointer
 --> src/lib.rs:9:18
  |
9 |         unsafe { *self.ptr.as_ptr() }
  |                  ^^^^^^^^^^^^^^^^^^ move occurs because value has type `T`, which does not implement the `Copy` trait
```

`ptr::read` is the answer, and it creates a new duty: `read` makes a bitwise
copy, so now there are two copies of one value and exactly one of them may be
used and dropped. The Miri lab is about getting that wrong.

*2. Freeing with another layout.* The native allocator often accepts it.
Miri does not:

```text
error: Undefined Behavior: incorrect layout on deallocation: alloc120048 has size 64 and alignment 64, but gave size 1 and alignment 1
```

*3. Forgetting to free, or to drop.* No undefined behaviour, but a leak, which
Miri reports at the end of the program as `error: memory leaked`. A leak that
is intended (`MyBox::leak`) must be taken back in a test, or Miri will fail
that test.

**What Miri says about the wrong version.** `examples/miri_l02_box.rs` has an
`into_inner` that reads the value out and returns. But `self` is dropped at
the end of the function, so the box's destructor drops the same value a
second time and frees the block. The caller holds a `String` whose buffer is
already gone:

```text
error: Undefined Behavior: constructing invalid value of type &[u8]: encountered a dangling reference (use-after-free)
     |
1790 |             &*core::intrinsics::aggregate_raw_ptr::<*const [T], _, _>(self.as_ptr(), self.len)
     |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Undefined Behavior occurred here
     ...
     = note: stack backtrace:
             ...
             2: main
                 at examples/miri_l02_box.rs:66:17: 66:32
```

The error is raised inside the standard library, where the `String` turns its
dead pointer into a slice. The line to look for is the first frame in your own
file: line 66, the first use of the returned `String`. The bug is earlier, in
`into_inner`. Miri shows where undefined behaviour *happens*, not where it was
caused; finding the cause is your work. A native run of this lab printed a
wrong first byte in one build and a different wrong byte in another.

**When to use which.** Use `Box<T>` whenever it does the job. Reach for
`alloc` directly when the layout is not the layout of a type you can name: an
unusual alignment, a header followed by a variable number of elements
(lesson 08), or one block that is resized in place (lesson 03).

**Read more.**
[The `std::alloc` module](https://doc.rust-lang.org/std/alloc/index.html) ·
[`GlobalAlloc`, with the full contract](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html) ·
[`Layout`](https://doc.rust-lang.org/std/alloc/struct.Layout.html) ·
[`NonNull`](https://doc.rust-lang.org/std/ptr/struct.NonNull.html) ·
[`PhantomData` in the Rustonomicon](https://doc.rust-lang.org/nomicon/phantom-data.html)

## Lesson 03 · A growable buffer: building Vec

**What is assumed.** Lesson 02: `Layout`, `alloc`, `dealloc`, `NonNull`, and
the dangling pointer for zero-sized types.

**The model.** A vector is three numbers: a pointer to a block, the capacity
`cap` (how many elements the block has room for) and the length `len` (how
many of them hold values).

```text
ptr
 |
 v
+-----+-----+-----+-----+-----+-----+-----+-----+
|  a  |  b  |  c  |  ?  |  ?  |  ?  |  ?  |  ?  |      cap = 8, len = 3
+-----+-----+-----+-----+-----+-----+-----+-----+
 <-- initialized ->  <------ uninitialized ---->
     owned by the vector
```

The whole design is the invariant in that picture: slots `0..len` hold values
that the vector owns, slots `len..cap` hold no value. Every method is a short
moment in which the picture is not true, between a state where it is true and
another state where it is true. Three operations move values across the
boundary, and each has one rule:

- `ptr.write(v)` fills an empty slot. It does not look at the old bytes.
  **An assignment `*ptr = v` is different:** it first drops the "old value" of
  the slot. For an empty slot that old value is garbage. Only assign to slots
  that hold a value.
- `ptr.read()` takes a value out as a bitwise copy. The bytes stay in the
  slot, but the slot must no longer count as a value, or the value will be
  dropped twice.
- `ptr::copy(src, dst, n)` moves `n` values, and the ranges may overlap (like
  C's `memmove`). `ptr::copy_nonoverlapping` requires that they do not.
  Afterwards the source slots hold stale duplicates, to be treated as empty.

**Growing.** When `len == cap`, the vector needs a larger block. Doubling the
capacity makes `n` pushes cost a number of byte copies proportional to `n`
in total; growing by one element makes it proportional to `n * n`. `realloc`
takes the old layout and the new size in bytes and keeps the old bytes. Two
checks belong here. `Layout::array::<T>(new_cap)` fails if the size exceeds
`isize::MAX`: panic with `capacity overflow`. And if `realloc` returns null,
the old block is still valid, so nothing may be changed before
`handle_alloc_error`.

**Zero-sized types.** A `MyVec<()>` must never allocate: the layout would have
size zero. Use the dangling pointer for good, and report a capacity of
`usize::MAX`, so that "is there room?" is always answered with yes. Pointer
arithmetic on a zero-sized type does not move the pointer, and reading or
writing zero bytes through a dangling, aligned pointer is allowed. Most of
your code then needs no special case, but the code that allocates and frees
does.

**Panics inside a method.** Destructors of elements are code that you do not
control, and they may panic. If `truncate` drops the tail and only then
lowers `len`, a panic in the middle leaves the vector claiming slots whose
values are already dropped, and the vector's own destructor then drops them
again. The rule: **first make the structure valid, then run foreign code.**
Lower `len`, then drop. Drop the tail with one call of `ptr::drop_in_place`
on a slice pointer (`*mut [T]`), not in a loop of your own. If one destructor
panics, a loop is abandoned and the elements after it leak; the slice drop
goes on with them while the panic unwinds. (A second panic at that point
aborts the program.) For the vector's destructor there is a second
question: who frees the block if an element's destructor panics? A local
value whose destructor frees it. Locals are dropped during unwinding too.
Lesson 04 uses this pattern, the *drop guard*, a lot.

**A worked example.** The three pointer operations on an array of four slots,
without an allocator, so that you can watch each slot:

```rust
use std::mem::MaybeUninit;
use std::ptr;

fn main() {
    // Room for four Strings. No slot holds a value yet.
    let mut slots: [MaybeUninit<String>; 4] = [const { MaybeUninit::uninit() }; 4];
    let base: *mut String = slots.as_mut_ptr().cast();
    let mut len = 0;
    // SAFETY: every offset below is less than 4, so it is inside `slots`.
    // The comments say which slots hold a value at each step: `write` is only
    // used on empty slots, `read` and `drop_in_place` only on full ones, and
    // each value is dropped exactly once. `slots` is not used through any
    // other path while `base` is.
    unsafe {
        base.write(String::from("a")); //            [a, _, _, _]
        base.add(1).write(String::from("c")); //     [a, c, _, _]
        len += 2;

        // Insert "b" at index 1. Move the tail up by one slot first.
        ptr::copy(base.add(1), base.add(2), len - 1); // [a, c', c, _]
        // Slot 1 is a stale duplicate of "c": the same buffer pointer as
        // slot 2. It counts as empty, so it is overwritten with `write`.
        base.add(1).write(String::from("b")); //     [a, b, c, _]
        len += 1;

        // Take the last value out. Its bytes stay behind, but `len` says that
        // the slot is empty again.
        let last = base.add(len - 1).read(); //      [a, b, c', _]
        len -= 1;
        assert_eq!(last, "c");

        assert_eq!(std::slice::from_raw_parts(base, len), ["a", "b"]);
        ptr::drop_in_place(ptr::slice_from_raw_parts_mut(base, len));
    }
}
```

If the stale duplicate in slot 1 had been "overwritten" with an assignment,
the assignment would have dropped it, and with it the buffer that slot 2
still uses.

**Mistakes people really make.**

*1. Assigning into an empty slot.* See the Miri lab below.

*2. `copy_nonoverlapping` for a shift.* `insert` and `remove` move a range by
one slot, so source and destination overlap. A debug build stops with the
first message below, and says itself that the check is optional. A release
build checks nothing. Miri gives the second message in every build:

```text
unsafe precondition(s) violated: ptr::copy_nonoverlapping requires that both pointer arguments are aligned and non-null and the specified memory ranges do not overlap

This indicates a bug in the program. This Undefined Behavior check is optional, and cannot be relied on for safety.
```

```text
error: Undefined Behavior: `copy_nonoverlapping` called on overlapping ranges
```

*3. Freeing with the layout of the wrong moment.* The block was allocated for
`cap` elements, not for `len`, and after a `realloc` its layout is the new
one. Miri reports `incorrect layout on deallocation`, as in lesson 02. The
native tests of this lesson check it as well, through an allocator that
records each block's layout.

*4. A pointer that covers too little.* `as_mut_ptr` must return the pointer
stored in the vector. If you create a slice of the `len` elements and return
*its* pointer, that pointer is only allowed to touch `len` elements, and code
that fills the spare capacity through it (lesson 04 does) is rejected by
Miri's default model. Lesson 06 explains why.

**What Miri says about the wrong version.** `examples/miri_l03_vec.rs` is a
fixed-capacity stack whose `push` assigns: `*slot = value`. Both native builds
of it printed the expected output. The fresh block happened to contain bytes
that looked like an empty `String`. Miri tracks which bytes were ever written:

```text
error: Undefined Behavior: reading memory at alloc304[0x8..0x10], but memory is uninitialized at [0x8..0x10], and this operation requires initialized memory
    ...
    = note: stack backtrace:
            ...
            6: std::ptr::drop_glue::<std::string::String> - shim(Some(std::string::String))
            7: FixedStack::<std::string::String>::push
                at examples/miri_l03_vec.rs:48:18: 48:52
            8: main
                at examples/miri_l03_vec.rs:72:5: 72:38

Uninitialized memory occurred at alloc304[0x8..0x10], in this allocation:
alloc304 (Rust heap, size: 48, align: 8) {
    0x00 │ __ __ __ __ __ __ __ __ __ __ __ __ __ __ __ __ │ ░░░░░░░░░░░░░░░░
    ...
}
```

Frame 6 is the proof: `push` called the destructor of a `String`. Nothing in
the source says "drop"; the assignment operator did it.

**When to use which.** `Vec<T>` is right almost always. Its unsafe
functions, `set_len`, `as_mut_ptr`, `spare_capacity_mut` and
`from_raw_parts`, let you do most low-level work without owning the
allocation logic. Write your own buffer when the layout differs from "one
block of `T`": a header in front, several arrays in one block, a ring.

**Read more.**
[Implementing `Vec` in the Rustonomicon](https://doc.rust-lang.org/nomicon/vec/vec.html) ·
[Safety rules of `std::ptr`](https://doc.rust-lang.org/std/ptr/index.html#safety) ·
[`ptr::copy`](https://doc.rust-lang.org/std/ptr/fn.copy.html) ·
[`Vec::from_raw_parts`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.from_raw_parts) ·
[`MaybeUninit`](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html)

## Lesson 04 · IntoIter and Drain: panic safety and leak safety

**What is assumed.** Your `MyVec` from lesson 03, the `Iterator` and
`DoubleEndedIterator` traits (chapter 15), and `catch_unwind` as a tool to
observe a panic.

**The model.** Two facts about safe Rust decide how these two iterators must
be written.

*Any code you call may panic.* A panic unwinds the stack: it leaves each
function early and drops its local variables on the way. Code that is "in
the middle" of changing a structure is abandoned in the middle. The structure
must be valid at every point where foreign code runs: an element's
destructor, a closure, `clone`, a comparison. This is *panic safety*
(the Rustonomicon says "exception safety"). The minimum is: no undefined
behaviour, whatever state is left. Better: nothing leaks.

*Any value may be leaked.* `std::mem::forget(value)` is a safe function. It
takes the value and never runs its destructor. A reference cycle of `Rc`
does the same by accident. So **no type may rely on its destructor for
soundness.** A destructor may restore order; it must not be the only thing
between the user and undefined behaviour. This is *leak safety*.

An owning iterator (`IntoIter`) is the simpler case. It takes over the block
of the vector and moves values out from both ends. It owns the values that
are left, so its destructor drops those and frees the block. If it is leaked,
everything leaks, and nothing dangles.

`Drain` borrows the vector mutably and removes a range. While it runs, the
vector is in pieces:

```text
before:     [ a  b | c  d  e | f  g ]        len = 7, drain(2..5)
during:     [ a  b | .  d  e | f  g ]        "c" was already taken out
after drop: [ a  b | f  g ]                  len = 4
```

If the vector kept `len = 7` during the drain and the `Drain` were leaked, the
vector would still claim the slot of `c`, whose value now belongs to someone
else: a double drop, in safe code. The standard solution is to set
`len = 2` *when the drain is created*. From that moment the vector only claims
`a` and `b`, which is true whatever happens next. The `Drain` owns the rest;
its destructor moves the tail down and sets `len = 4`. If the `Drain` is
leaked, the vector simply stays at `[a b]`: the range and the tail are lost.
Paying with a larger leak to keep soundness is called *leak amplification*.

**The drop guard.** `Drain`'s destructor must drop the elements that were not
taken and then move the tail. Those destructors can panic, and then the tail
must still be moved. Put the step that must happen into the destructor of a
small local struct. The compiler drops locals on the normal path and during
unwinding, so the step happens either way.

**A worked example.** Apply a function to every element of a `Vec` in place,
without allocating. While `f` runs, one slot is empty, and `f` may panic:

```rust
use std::ptr;

/// Replaces each element by `f(element)`. If `f` panics, the vector keeps
/// the elements that were already replaced, and the others are dropped.
fn map_in_place<T>(vec: &mut Vec<T>, mut f: impl FnMut(T) -> T) {
    /// Owns the elements while the function runs, and makes the vector valid
    /// again when it goes out of scope, also during unwinding.
    struct Repair<'a, T> {
        vec: &'a mut Vec<T>,
        /// Slots `0..done` hold new values.
        done: usize,
        len: usize,
    }

    impl<T> Drop for Repair<'_, T> {
        fn drop(&mut self) {
            // SAFETY: slots `0..done` hold values, so `done` is a valid
            // length. If `done < len`, `f` panicked: slot `done` is empty
            // (its value was moved into `f`), and the slots after it still
            // hold old values that nobody else owns. They are dropped here.
            unsafe {
                if self.done < self.len {
                    let rest = self.vec.as_mut_ptr().add(self.done + 1);
                    ptr::drop_in_place(ptr::slice_from_raw_parts_mut(
                        rest,
                        self.len - self.done - 1,
                    ));
                }
                self.vec.set_len(self.done);
            }
        }
    }

    let len = vec.len();
    // Leak safety and panic safety in one step: the vector claims nothing
    // while its slots are being emptied and refilled.
    // SAFETY: 0 is a valid length for any vector.
    unsafe { vec.set_len(0) };
    let mut repair = Repair { vec, done: 0, len };
    while repair.done < repair.len {
        // SAFETY: `done < len`, so the slot is inside the block and holds an
        // old value. `read` empties it; `write` fills it again. If `f`
        // panics in between, `repair` knows that the slot is empty.
        unsafe {
            let slot = repair.vec.as_mut_ptr().add(repair.done);
            let new = f(slot.read());
            slot.write(new);
        }
        repair.done += 1;
    }
}

fn main() {
    let mut words = vec![String::from("a"), String::from("b"), String::from("c")];
    map_in_place(&mut words, |word| word + "!");
    assert_eq!(words, ["a!", "b!", "c!"]);

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        map_in_place(&mut words, |word| {
            assert!(word != "b!", "refusing b");
            word.to_uppercase()
        });
    }));
    assert!(outcome.is_err());
    assert_eq!(words, ["A!"], "the replaced prefix is kept; nothing is dropped twice");
}
```

There is one destructor for both outcomes. The normal path is the special
case `done == len`.

**Mistakes people really make.**

*1. Using the vector while a `Drain` is alive.* The borrow checker protects
the "in pieces" state, as long as your `Drain` really holds the `&mut`. The
same rule for the standard iterator:

```rust,compile_fail,E0499
let mut words = vec![String::from("a"), String::from("b")];
let mut each = words.iter_mut();
let first = each.next();
words.clear();
println!("{first:?}");
```

```text
error[E0499]: cannot borrow `words` as mutable more than once at a time
 --> src/main.rs:5:5
  |
3 |     let mut each = words.iter_mut();
  |                    ----- first mutable borrow occurs here
4 |     let first = each.next();
5 |     words.clear();
  |     ^^^^^ second mutable borrow occurs here
6 |     println!("{first:?}");
  |                ----- first borrow later used here
```

If your iterator stores a raw pointer instead of a reference, add a
`PhantomData<&'a mut MyVec<T>>` field, or this protection is gone.

*2. Restoring the vector only on the normal path.* The tests of the lesson
contain an element whose destructor panics. Without a guard the tail is lost.

*3. Dropping the remaining elements one by one in a loop.* If the third
destructor panics, the loop is abandoned and the fourth element leaks.
`ptr::drop_in_place` on a slice (`*mut [T]`) continues with the remaining
elements while the panic unwinds.

**What Miri says about the wrong version.** `examples/miri_l04_drain.rs` is a
front-drain over a `Vec` that leaves `len` alone and trusts its destructor to
tidy up. `main` takes one element and then forgets the iterator, all in safe
code. The program prints that the vector "now holds 3 element(s)", and then:

```text
error: Undefined Behavior: constructing invalid value of type &mut [u8]: encountered a dangling reference (use-after-free)
    ...
    = note: stack backtrace:
            ...
            4: std::ptr::drop_glue::<std::string::String> - shim(Some(std::string::String))
            ...
            8: <std::vec::Vec<std::string::String> as std::ops::Drop>::drop
            ...
            11: main
                at examples/miri_l04_drain.rs:90:5: 90:16
```

The vector dropped a `String` that `main` had already received and dropped.
A native run of this lab crashed at that point, without a message. After the
repair Miri
reports something else: `error: memory leaked`, twice, for the two strings
that the forgotten iterator still owned. That is the expected result. A leak
is safe, and it is what leak amplification costs. Run the lab with
`MIRIFLAGS=-Zmiri-ignore-leaks` to confirm that nothing else is wrong.

**When to use which.** `drain(..)` when you need the removed elements by
value; `truncate`, `clear` or `retain` when you only want them gone;
`mem::take(&mut vec)` when you want the whole vector and can leave an empty
one. When you write a guard type of your own, ask the two questions of this
lesson before anything else: what if this is leaked, and what if the code
between creation and drop panics?

**Read more.**
[Exception safety in the Rustonomicon](https://doc.rust-lang.org/nomicon/exception-safety.html) ·
[Leaking in the Rustonomicon](https://doc.rust-lang.org/nomicon/leaking.html) ·
[`mem::forget`](https://doc.rust-lang.org/std/mem/fn.forget.html) ·
[`Vec::drain`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.drain) ·
[`ptr::drop_in_place`](https://doc.rust-lang.org/std/ptr/fn.drop_in_place.html)

## Lesson 05 · Shared ownership: building Rc and Weak

**What is assumed.** `Rc` and `Weak` as a user (chapters 4 and 9), `Cell`
(lesson 01), an owning pointer with `NonNull` (lesson 02).

**The model.** An `Rc<T>` is a pointer to one heap allocation that holds two
counters and the value. Cloning an `Rc` adds one to the *strong* count and
copies the pointer; dropping subtracts one. A `Weak<T>` points to the same
allocation and keeps a *weak* count.

The interesting part is that the allocation has two lifetimes that end at
different moments:

- the **value** lives as long as the strong count is above zero;
- the **allocation** must live as long as *any* pointer to it exists, strong
  or weak, because a `Weak` reads the counters to learn that the value is
  gone.

| Strong | Weak pointers | Value | Allocation |
| --- | --- | --- | --- |
| 2 | 0 | alive | alive |
| 1 | 2 | alive | alive |
| 0 | 2 | dropped | alive, counters only |
| 0 | 0 | dropped | freed |

You can watch the first three rows with the standard types:

```rust
use std::rc::Rc;

fn main() {
    let first = Rc::new(String::from("shared"));
    let second = Rc::clone(&first);
    let weak = Rc::downgrade(&first);
    assert_eq!((Rc::strong_count(&first), Rc::weak_count(&first)), (2, 1));

    drop(first);
    assert!(weak.upgrade().is_some_and(|owner| *owner == "shared"));

    drop(second); // The String is dropped here. The allocation stays.
    assert!(weak.upgrade().is_none());
} // `weak` is dropped here, and the allocation is freed.
```

**One count too few.** With "free the allocation when both counts are zero"
there is a trap. Suppose the value itself contains a `Weak` to its own
allocation, as a tree node with a parent link can. When the last `Rc` goes,
the strong count becomes 0 and the value's destructor runs. That destructor
drops the `Weak`: now both counts are 0, and the `Weak` frees the allocation,
while the destructor of the value inside it is still running, and while
`Rc::drop` is about to read the counters again.

The standard library avoids this with a convention: **all strong pointers
together hold one weak reference.** The weak counter starts at 1. The last
`Rc` drops the value first and gives up that shared weak reference
afterwards. The allocation is freed by whoever brings the weak counter to 0.
With this convention there is exactly one rule for freeing, and no moment
at which two owners both think they are the last.

**Three more rules that soundness needs.**

- *`get_mut` must look at both counts.* It returns `&mut T` from the only
  `Rc`. A `Weak` could be upgraded to a second `Rc` while that `&mut T` is
  alive, so "strong count is 1" is not enough.
- *Counts must not wrap around.* `mem::forget(rc.clone())` is safe and adds
  one to the count without ever subtracting it. In a loop, and the optimizer
  may turn that loop into one addition, the count can pass `usize::MAX` and
  come back to a small number. Then a few drops free the value while other
  pointers still exist. Check the addition, and panic or abort.
- *Borrow fields, never the whole allocation.* The next paragraphs explain it.

**References to one field.** While the last `Rc` drops the value, the value's
destructor holds `&mut T`. If the destructor uses a `Weak` to the same
allocation, and that `Weak` creates a reference to the *whole* allocation, the
new reference covers the bytes of `T` too, and so it overlaps with the
`&mut T`. With a raw pointer you can borrow one field and leave the rest
alone:

```rust
use std::cell::Cell;

struct Record {
    visits: Cell<u32>,
    payload: String,
}

fn main() {
    let record = Box::into_raw(Box::new(Record {
        visits: Cell::new(0),
        payload: String::from("data"),
    }));
    // SAFETY: `record` came from `Box::into_raw` and is turned back into a
    // box exactly once, at the end. Until then nothing else uses the
    // allocation. Each reference below covers one field only, the two fields
    // do not overlap, and both references end before the allocation is freed.
    unsafe {
        let visits: &Cell<u32> = &(*record).visits;
        let payload: &mut String = &mut (*record).payload;
        payload.push('!');
        visits.set(visits.get() + 1);
        assert_eq!((visits.get(), payload.as_str()), (1, "data!"));
        drop(Box::from_raw(record));
    }
}
```

`(*record).visits` is a *place*: it names the field without creating a
reference to the struct around it. `&(*record).visits` borrows that field
only. `&raw const (*record).visits` goes one step further and gives a raw
pointer without creating any reference, which you need when the field may be
uninitialized.

**Mistakes people really make.**

*1. Freeing at "strong count is zero".* The Miri lab below.

*2. A reference to the whole allocation in `Weak::upgrade`.* The tests of the
lesson contain a value whose destructor upgrades a `Weak` to itself. Miri's
default model rejects the overlap, in this lesson's reference solution with
one line changed:

```text
error: Undefined Behavior: not granting access to tag <414429> because that would remove [Unique for <414624>] which is strongly protected (i.e. it is in use as a function argument)
```

The "function argument" is the `&mut T` of the running destructor. Tree
Borrows, Miri's second model, accepts this particular program. Code that only
one of the two models accepts is not code to ship: write the version that
both accept.

*3. Dropping the value but not releasing the allocation when the destructor
panics.* Create the guard that gives up the shared weak reference *before*
you run the value's destructor. Then it runs during unwinding as well.

**What Miri says about the wrong version.** `examples/miri_l05_rc.rs` frees
the allocation as soon as the last owner is gone. A weak pointer then asks
whether the value is alive:

```text
error: Undefined Behavior: constructing invalid value of type &Shared<std::string::String>: encountered a dangling reference (use-after-free)
    ...
    = note: stack backtrace:
            ...
            1: Weak::<std::string::String>::is_alive
                at examples/miri_l05_rc.rs:82:18: 82:33
            2: main
                at examples/miri_l05_rc.rs:103:56: 103:71
```

One native build of this lab printed `false`, the "right" answer, read from
freed memory. Another build panicked with `attempt to subtract with overflow`
in the weak pointer's destructor.

**When to use which.** `Rc` when ownership really is shared and the graph has
no cycles of strong pointers; `Weak` for the back edges (parent links,
observers, caches); `Arc` across threads (module M3 builds it); an arena
(lesson 08) or indices into a `Vec` when all nodes die together anyway.
That is often simpler and faster than counting.

**Read more.**
[The `std::rc` module](https://doc.rust-lang.org/std/rc/index.html) ·
[`Rc::get_mut`](https://doc.rust-lang.org/std/rc/struct.Rc.html#method.get_mut) ·
[`Weak::upgrade`](https://doc.rust-lang.org/std/rc/struct.Weak.html#method.upgrade) ·
[Raw borrow operators `&raw const` and `&raw mut`](https://doc.rust-lang.org/reference/expressions/operator-expr.html#raw-borrow-operators) ·
[Implementing `Arc` in the Rustonomicon](https://doc.rust-lang.org/nomicon/arc-mutex/arc.html)

## Lesson 06 · The aliasing rules, as Miri enforces them

**What is assumed.** All earlier lessons. You have seen the words "tag" and
"retag" in Miri reports; this section explains them.

**The model.** Two pointers *alias* when they point to overlapping memory.
The compiler optimizes on the assumption that references follow strict rules
about aliasing:

- while a `&mut T` is in use, nothing else reads or writes its memory;
- while a `&T` is in use, nothing writes its memory, except inside an
  `UnsafeCell`.

The borrow checker enforces these rules for references. Raw pointers are not
checked, but **they are not exempt**: a raw pointer is always made from
something, and it inherits what that source was allowed to do. This inherited
permission is part of the pointer's *provenance*. Two pointers with the same
address can have different provenance, and using the one with too little is
undefined behaviour.

Rust does not have a finished specification of these rules yet. Miri
implements two candidate models, and this module uses both:

- **Stacked Borrows** is Miri's default. In most cases it is the stricter one.
- **Tree Borrows** (`MIRIFLAGS=-Zmiri-tree-borrows`) accepts more programs,
  but not a superset: task M has a specimen that only Stacked Borrows accepts.

Each report says that the rules "are still experimental". The practical
reading: code that both models accept is what you should write. Code that
both reject is wrong. Code that only one of them accepts is in dispute, and
a careful author rewrites it.

**How Stacked Borrows sees it.** Every reference and every raw pointer
carries a *tag*. Each byte of memory has a stack of tags that may currently
access it. Creating a reference or pointer from another one (a *retag*)
pushes a new tag on top, with a permission: `Unique` for `&mut`,
`SharedReadOnly` for `&`, `SharedReadWrite` for a raw pointer made from a
`&mut` or pointing into an `UnsafeCell`. Using a tag to write removes
everything above it in the stack: "I am using the older pointer now, so
everything that was derived from it is over." Using a tag that is no longer
in the stack is the error.

**Worked example 1.** `demo1` in `examples/aliasing.rs`:

```rust,no_run
fn demo1() -> i32 {
    let mut x = 1;
    let r = &mut x;
    let p = r as *mut i32;
    unsafe { *p += 1 }; // fine: p is on top of the stack
    *r += 1; //            uses r: everything above r is removed, p included
    unsafe { *p += 1 }; // p is no longer in the stack
    x
}
```

```text
error: Undefined Behavior: attempting a read access using <1745> at alloc715[0x0], but that tag does not exist in the borrow stack for this location
  --> examples/aliasing.rs:34:14
   |
34 |     unsafe { *p += 1 };
   |              ^^^^^^^ this error occurs as part of an access at alloc715[0x0..0x4]
   |
help: <1745> was created by a SharedReadWrite retag at offsets [0x0..0x4]
  --> examples/aliasing.rs:31:13
   |
31 |     let p = r as *mut i32;
   |             ^
help: <1745> was later invalidated at offsets [0x0..0x4] by a write access
  --> examples/aliasing.rs:33:5
   |
33 |     *r += 1;
   |     ^^^^^^^
```

A report has three parts, and you should read all three: the access that
failed, where the pointer was *created*, and where it was *invalidated*. The
bug is almost always at the third place. Tree Borrows accepts `demo1`: there,
a raw pointer cast from a reference shares the reference's tag, so `r` and
`p` are the same thing to it. The models disagree, so treat `demo1` as wrong.

**Worked example 2.** `demo2` takes a pointer to an array and then writes to
the array directly:

```rust,no_run
fn demo2() -> i32 {
    let mut v = [1, 2, 3];
    let p = v.as_mut_ptr();
    v[0] = 10; //          a write through the owner, not through p
    unsafe { *p += 1 };
    v[0]
}
```

Both models reject it. Tree Borrows describes the same event in its own
words:

```text
error: Undefined Behavior: read access through <1733> at alloc720[0x0] is forbidden
  --> examples/aliasing.rs:43:14
   |
43 |     unsafe { *p += 1 };
   |              ^^^^^^^ Undefined Behavior occurred here
   |
   = help: the accessed tag <1733> has state Disabled which forbids this child read access
help: the accessed tag <1733> was created here, in the initial state Reserved
  --> examples/aliasing.rs:41:13
   |
41 |     let p = v.as_mut_ptr();
   |             ^^^^^^^^^^^^^^
help: the accessed tag <1733> later transitioned to Disabled due to a foreign write access at offsets [0x0..0x4]
  --> examples/aliasing.rs:42:5
   |
42 |     v[0] = 10;
   |     ^^^^^^^^^
```

In Tree Borrows the tags of one allocation form a tree: a pointer is the
child of the pointer it was made from. Each tag has a state for each byte.
A mutable reference starts as `Reserved`. A write through a tag, or through
one of its descendants, is a *child access*. A write through any other tag is
a *foreign access*, and a foreign write sets a tag to `Disabled` for good.

**Rules of thumb that both models accept.**

1. Make one raw pointer from the owner, derive every other pointer from that
   one, and do not touch the owner again until you are done with all of them.
2. To reach several elements, take the pointer to the *whole* buffer
   (`as_mut_ptr`) and use `add`. A pointer made from `&mut v[0]` may only
   touch element 0 under Stacked Borrows.
3. Read what you need from the owner (`len`, for example) *before* you make
   the raw pointer.
4. Once a value is reached through raw pointers, do not also keep a `Box` or
   a `&mut` to it. Moving a `Box` or passing a `&mut` to a function claims
   exclusive access again. This is the Miri lab.
5. A reference must be non-null, aligned and point into a live allocation
   that is large enough for its type at the moment it is created, even if it
   is never read. Miri checks exactly that. The Reference also asks that the
   memory holds a valid value; that part is still under discussion and Miri
   does not check it, so treat it as required. A raw pointer needs none of
   this until it is used.

**A worked example of the pattern.** Reversing a slice with two raw cursors.
The length is read first; after `as_mut_ptr`, `slice` is not used again:

```rust
fn reverse<T>(slice: &mut [T]) {
    let len = slice.len();
    let base = slice.as_mut_ptr();
    for i in 0..len / 2 {
        // SAFETY: `i < len / 2`, so `i` and `len - 1 - i` are in bounds and
        // different. `base` was made from the whole slice, so it may read
        // and write every element, and `slice` is not used while `base` is.
        unsafe { std::ptr::swap(base.add(i), base.add(len - 1 - i)) };
    }
}

fn main() {
    let mut words = [String::from("a"), String::from("b"), String::from("c")];
    reverse(&mut words);
    assert_eq!(words, ["c", "b", "a"]);
}
```

**What Miri says about the wrong version.** `examples/miri_l06_queue.rs` is a
linked queue. Its nodes are owned by `Box`es, chained from `head`, and `tail`
is a raw pointer to the last node. Every native test of the lesson's task B
passes with this design. Under Stacked Borrows the second `push_back` fails:

```text
error: Undefined Behavior: trying to retag from <767> for Unique permission at alloc389[0x18], but that tag does not exist in the borrow stack for this location
    ...
help: <767> was created by a SharedReadWrite retag at offsets [0x0..0x20]
   --> examples/miri_l06_queue.rs:47:33
    |
 47 |         let raw: *mut Node<T> = &mut *node;
    |                                 ^^^^^^^^^^
help: <767> was later invalidated at offsets [0x0..0x20] by a Unique retag
   --> examples/miri_l06_queue.rs:49:30
    |
 49 |             self.head = Some(node);
    |                              ^^^^
    = note: stack backtrace:
            ...
            1: Queue::<std::string::String>::push_back
                at examples/miri_l06_queue.rs:54:22: 54:39
```

Line 49 is the surprise: *moving* the `Box` into `self.head`. A `Box` claims
to be the only way to its value, like a `&mut`, and Miri applies that claim
again when the box is moved. The raw pointer taken two lines earlier is over.
Tree Borrows accepts the pushes and stops later, in `pop_front`, with
`reborrow through <752> ... is forbidden`: the write through `tail` at line 54
was a foreign write for the box in `head`. Both models reject the design, at
different lines. The repair is task B: nodes that are reached only through
raw pointers, from `Box::into_raw` (or `Box::leak`) until `Box::from_raw`.

**When to use which.** Stay with references and indices whenever you can:
`split_at_mut`, `get_disjoint_mut`, `iter_mut` and `chunks_mut` cover most
"two mutable parts of one buffer" needs with no `unsafe`. Use raw pointers
for structures that the borrow checker cannot describe, such as nodes with
several incoming pointers, and then use *only* raw pointers inside, with
references created for a short time at the edges.

**Read more.**
[Behavior considered undefined](https://doc.rust-lang.org/reference/behavior-considered-undefined.html) ·
[Provenance in the `std::ptr` docs](https://doc.rust-lang.org/std/ptr/index.html#provenance) ·
[Aliasing in the Rustonomicon](https://doc.rust-lang.org/nomicon/aliasing.html) ·
[`slice::get_disjoint_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.get_disjoint_mut) ·
[Miri's README, with all flags](https://github.com/rust-lang/miri#readme)

## Lesson 07 · Send and Sync: writing the proof

**What is assumed.** Threads, `Send` and `Sync` as a user (chapter 19), and
`UnsafeCell` (lesson 01). Memory orderings are the subject of module M3; here
the two orderings you need are given.

**The model.** Two marker traits describe what may cross a thread boundary:

- `T: Send` means: a value of type `T` may be *moved* to another thread. That
  thread may then use it and drop it.
- `T: Sync` means: a `&T` may be *used from several threads at the same
  time*. The definition is exactly that `&T` is `Send`.

Both are *auto traits*. The compiler implements them for a struct when all of
its fields have them. Raw pointers and `UnsafeCell` are the two field types
that switch them off: `*const T`, `*mut T` and `NonNull<T>` are neither `Send`
nor `Sync`, and `UnsafeCell<T>` is not `Sync`. This is a deliberate nudge. A
type that contains a raw pointer stays on its thread until its author says
otherwise, with an `unsafe impl`, and takes responsibility for the claim.

The claim is a statement about every safe program, so the proof has to
consider every method. A method for writing it:

1. **For `Send`:** list what the receiving thread can do with the value,
   including dropping it. Does any of that touch something the old thread
   can still reach? If the type owns its data and shares nothing, sending it
   sends the `T`s inside, and the bound is `T: Send`.
2. **For `Sync`:** list what a thread can do with only a `&Self`. If all it
   can get is `&T`, the bound is `T: Sync`. If it can get `&mut T` or a `T` by
   value (a lock does that), the bound is `T: Send`, and you must show that
   two threads never get that access at the same time.
3. **Name a counterexample for a missing bound.** The usual test types:
   `Rc<u8>` is neither `Send` nor `Sync`. `Cell<u8>` is `Send` but not `Sync`.
   `MutexGuard<'static, u8>` is `Sync` but not `Send`.

**A worked example.** A hand-written iterator over shared references, like
`slice::Iter`. It holds a raw pointer, so the compiler makes it neither
`Send` nor `Sync`:

```rust
use std::marker::PhantomData;

/// Yields `&'a T` for each element of a slice.
struct Cursor<'a, T> {
    next: *const T,
    left: usize,
    _borrow: PhantomData<&'a [T]>,
}

// SAFETY: a `Cursor<'a, T>` is a `&'a [T]` in another shape. It never writes,
// moves or drops a `T`; all it gives out is `&'a T`. Sending a cursor to
// another thread therefore gives that thread shared references to elements
// that the first thread can still read through the slice. That is sound
// exactly when `T: Sync`. `T: Send` is not needed.
unsafe impl<T: Sync> Send for Cursor<'_, T> {}

// SAFETY: a `&Cursor` gives nothing but a read of the two fields (`next`
// needs `&mut self`). Even through a clone it could give no more than `&T`,
// so `T: Sync` is enough.
unsafe impl<T: Sync> Sync for Cursor<'_, T> {}

impl<'a, T> Cursor<'a, T> {
    fn new(slice: &'a [T]) -> Cursor<'a, T> {
        Cursor { next: slice.as_ptr(), left: slice.len(), _borrow: PhantomData }
    }
}

impl<'a, T> Iterator for Cursor<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        // SAFETY: `left` was above zero, so `next` points to an element of
        // the slice, which is borrowed for `'a` and so alive and unchanged.
        // Moving one element forward stays inside the slice or just past it.
        unsafe {
            let item = &*self.next;
            self.next = self.next.add(1);
            Some(item)
        }
    }
}

fn main() {
    let numbers = [1u64, 2, 3, 4];
    let cursor = Cursor::new(&numbers);
    let total = std::thread::scope(|scope| scope.spawn(move || cursor.sum::<u64>()).join().unwrap());
    assert_eq!(total, 10);
}
```

The bounds match what the compiler derives for `&'a [T]`. That is a good
check: if your type is "morally" a reference, a box or a lock, its bounds
should equal those of the standard type.

**Mistakes people really make.**

*1. No bound at all.* `unsafe impl<T> Send for Wrapper<T> {}` "fixes" the
compiler error and makes `Wrapper<Rc<u8>>` sendable. The code review lesson
has this one.

*2. The wrong bound on a lock.* A lock gives `&mut T` to whichever thread
holds it, so a `T` created on one thread can be changed, replaced and dropped
on another: `Sync` for the lock needs `T: Send`. It does not need `T: Sync`,
because no two threads look at the `T` together.

*3. Trusting the derived impl of a guard.* A guard that contains
`&'a Lock<T>` is `Sync` whenever `Lock<T>` is `Sync`, that is for `T: Send`.
But a shared guard gives `&T`, which needs `T: Sync`. With `T = Cell<u8>` the
derived answer is wrong. An explicit `unsafe impl<T: Sync> Sync for Guard`
*replaces* the derived impl, so it can narrow it.

*4. Sharing without any of this.* The compiler's message when you try:

```rust,compile_fail,E0277
use std::cell::UnsafeCell;

struct Tally {
    count: UnsafeCell<u64>,
}

let tally = Tally { count: UnsafeCell::new(0) };
std::thread::scope(|scope| {
    scope.spawn(|| unsafe { *tally.count.get() += 1 });
});
```

```text
error[E0277]: `UnsafeCell<u64>` cannot be shared between threads safely
  --> src/main.rs:10:21
   |
10 |         scope.spawn(|| unsafe { *tally.count.get() += 1 });
   |               ----- ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `UnsafeCell<u64>` cannot be shared between threads safely
   |               |
   |               required by a bound introduced by this call
   |
   = help: the trait `Sync` is not implemented for `UnsafeCell<u64>`
   = note: required for `&UnsafeCell<u64>` to implement `Send`
```

The last line is the definition from the top of this section, applied.

**What Miri says about the wrong version.** `examples/miri_l07_sync.rs`
silences that error with `unsafe impl Sync` and a comment that sounds
reasonable ("a u64 has no invalid values"). Two threads then run
`count += 1` on the same memory. Both native builds printed `total: 200`.

```text
error: Undefined Behavior: Data race detected between (1) non-atomic write on thread `unnamed-1` and (2) non-atomic read on thread `unnamed-2` at alloc208
  --> examples/miri_l07_sync.rs:36:18
   |
36 |         unsafe { *self.count.get() += 1 };
   |                  ^^^^^^^^^^^^^^^^^^^^^^ (2) just happened here
   |
help: and (1) occurred earlier here
  --> examples/miri_l07_sync.rs:36:18
   |
36 |         unsafe { *self.count.get() += 1 };
   |                  ^^^^^^^^^^^^^^^^^^^^^^
```

A data race is undefined behaviour by itself. It does not matter which values
are written. Miri runs the threads in one of many possible orders; for
thread code, also run with `MIRIFLAGS=-Zmiri-many-seeds=0..16` to try
sixteen of them.

**When to use which.** Prefer fields that already have the right auto traits
(`AtomicU64`, `Mutex<T>`, `Arc<T>`) and write no `unsafe impl` at all. Write
one when the type contains a raw pointer or an `UnsafeCell` and you can
complete the proof. To make a type *less* thread-capable than its fields,
add a `PhantomData<*const ()>` field (neither `Send` nor `Sync`) or
`PhantomData<Cell<()>>` (`Send`, not `Sync`).

**Read more.**
[`Send` and `Sync` in the Rustonomicon](https://doc.rust-lang.org/nomicon/send-and-sync.html) ·
[`Send`](https://doc.rust-lang.org/std/marker/trait.Send.html) ·
[`Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html) ·
[Auto traits in the Reference](https://doc.rust-lang.org/reference/special-types-and-traits.html#auto-traits) ·
[Data races in the Rustonomicon](https://doc.rust-lang.org/nomicon/races.html)

## Lesson 08 · Capstone: a bump arena

**What is assumed.** Everything before. This section gives the ideas; the
design is yours.

**The model.** A *bump allocator* owns a few large blocks, called chunks. To
allocate, it rounds a cursor up to the alignment that the value needs, hands
out the bytes from there, and moves the cursor past them. Nothing is freed
one by one. When the arena goes away, the chunks are freed, and every value
with them.

```text
chunk 1 (full)                      chunk 2 (current)
+----+--+--------+-----+------+     +--------+----+-----------------+
| u8 |//|  u64   | u16 |//////|     |  u64   | u8 |      free       |
+----+--+--------+-----+------+     +--------+----+-----------------+
      ^ padding for alignment                      ^ cursor          ^ end
```

Why it is worth having: allocation is a few additions; values that die
together are freed with one call; and the borrow checker ties every reference
to the arena, so values can point at each other freely (`&'a Node<'a>`),
which is painful with `Box` and `Rc`.

**Why `&self -> &mut T` is sound here.** The signature
`fn alloc<T>(&self, value: T) -> &mut T` looks like the thing lesson 01
forbids. It is sound because of one fact: each call returns bytes that no
earlier call returned and no later call will return. The cursor only moves
forward. So every `&mut T` is unique by construction. The arena must also
never move a value: growing means *adding* a chunk, never reallocating one.

**Rounding up.** For an alignment `a` (always a power of two), the next
multiple of `a` at or after `n` is `(n + a - 1) & !(a - 1)`, or
`n.next_multiple_of(a)`. Round the *address*, not the offset, unless you know
the alignment of the chunk itself. And keep provenance: compute how many bytes
to skip as an integer, then move the pointer with `add`. Do not turn the
rounded integer back into a pointer.

**Two design decisions.** The contract of the capstone makes them for you;
you should be able to defend them.

*`alloc` never runs destructors.* The arena is not generic, so when it is
dropped it only has bytes: it does not know that the bytes at offset 24 are a
`String`. It could remember (the next decision does), but there is a deeper
problem. Values in an arena may hold references to each other. If the arena
ran their destructors, a destructor could follow such a reference to a value
that was destroyed a moment earlier. The compiler normally prevents this with
the drop checker, but it cannot see references hidden in an untyped block.

*`alloc_with_drop` requires `T: 'static`.* A `'static` type contains no
reference that can expire: whatever it borrows lives until the program ends.
A reference into an arena that will be dropped or reset is not of that kind,
so such a value cannot point at a neighbour in the arena, and its destructor
can run at any time. The bound is the safety argument, and the compiler
enforces it at every call:

```rust,compile_fail,E0597
fn keep<T: 'static>(value: T) -> T {
    value
}

let name = String::from("ada");
let kept = keep(&name);
println!("{kept}");
```

```text
error[E0597]: `name` does not live long enough
 --> src/main.rs:6:21
  |
5 |     let name = String::from("ada");
  |         ---- binding `name` declared here
6 |     let kept = keep(&name);
  |                -----^^^^^-
  |                |    |
  |                |    borrowed value does not live long enough
  |                argument requires that `name` is borrowed for `'static`
```

A safety argument of this kind lives in a signature, so no test that runs
and no Miri run can check it: the wrong version has no bad execution in the
test suite, it only *accepts* a bad program. The header of the capstone
therefore ends with three small programs for `cargo test --doc`: one that
must compile, and two that the compiler must reject (a borrowed value for
`alloc_with_drop`, and `reset` while a reference into the arena is in use).

To run a destructor later without knowing the type, store a pointer to the
value next to a function pointer that does know it:
`unsafe fn drop_value<T>(p: *mut u8)` casts and calls `drop_in_place`, and
`drop_value::<T>` for a concrete `T` is an ordinary `unsafe fn(*mut u8)`.
`std::mem::needs_drop::<T>()` tells you when there is nothing to record.

**A worked example.** One block that holds a header and a run of values
behind it, laid out by hand. It uses the same arithmetic as an arena, for one
allocation:

```rust
use std::alloc::{self, Layout};

fn main() {
    let count = 3usize;
    let header = Layout::new::<u32>();
    let values = Layout::array::<u64>(count).unwrap();
    // `extend` puts `values` behind `header`, at the next offset that is
    // aligned for it, and returns the layout of both and that offset.
    let (block, offset) = header.extend(values).unwrap();
    assert_eq!((offset, block.size(), block.align()), (8, 32, 8));

    // SAFETY: the layout has a non-zero size. `base` is checked for null.
    // The block starts at a multiple of 8, so the u32 at offset 0 and the
    // u64 values from offset 8 on are aligned and inside the 32 bytes. Each
    // location is written before it is read. The block is freed once, with
    // the layout it was allocated with.
    unsafe {
        let base = alloc::alloc(block);
        if base.is_null() {
            alloc::handle_alloc_error(block);
        }
        base.cast::<u32>().write(3);
        let first = base.add(offset).cast::<u64>();
        for i in 0..count {
            first.add(i).write(10 * i as u64);
        }
        assert_eq!(base.cast::<u32>().read(), 3);
        assert_eq!(std::slice::from_raw_parts(first, count), [0, 10, 20]);
        alloc::dealloc(base, block);
    }
}
```

Four bytes of padding sit between the `u32` and the first `u64`. `extend`
computed them; your arena computes them from the cursor.

**Mistakes people really make.**

*1. Forgetting alignment.* The Miri lab below.

*2. One growing buffer.* Keeping the bytes in a `Vec<u8>` and letting it grow
moves every value that was handed out. All earlier references dangle. The
native tests often still pass, because freed memory tends to keep its
contents for a while. Miri reports the first read through an old reference
as a use-after-free.

*3. Giving the arena a `'static` reference to hide a lifetime.* If you feel
the need for `transmute` to extend a lifetime, the design is wrong: the
lifetime of every returned reference is the lifetime of the `&self` borrow,
and that is exactly right.

*4. Making the arena `Send`.* `alloc_with_drop` accepts any `'static` type,
including `Rc<u8>`. An arena that is sent to another thread and dropped there
runs that `Rc`'s destructor on the wrong thread. Either the arena stays on
its thread, or `alloc_with_drop` needs `T: Send`.

**What Miri says about the wrong version.** `examples/miri_l08_arena.rs`
bumps an offset without rounding. After a `u8` at offset 0, a `u64` is
written at offset 1:

```text
error: Undefined Behavior: accessing memory based on pointer with alignment 1, but alignment 8 is required
  --> examples/miri_l08_arena.rs:62:13
   |
62 |             slot.write(value);
   |             ^^^^^^^^^^^^^^^^^ Undefined Behavior occurred here
```

This is one of the few kinds of undefined behaviour that a native *debug*
build can also catch: the debug build of this lab stopped with
`misaligned pointer dereference: address must be a multiple of 0x8`. The
release build printed the expected result. Do not rely on the debug check:
it covers a small part of what Miri checks.

**When to use which.** An arena fits when many values share one lifetime: the
nodes of a syntax tree, the objects of one request, the scratch data of one
frame. It does not fit when values must be freed one by one, or when most of
them own resources that need destructors. For production code, look at an
existing crate before writing your own; you now know what to check in its
source.

**Read more.**
[`Layout::extend`](https://doc.rust-lang.org/std/alloc/struct.Layout.html#method.extend) ·
[`mem::needs_drop`](https://doc.rust-lang.org/std/mem/fn.needs_drop.html) ·
[The drop check in the Rustonomicon](https://doc.rust-lang.org/nomicon/dropck.html) ·
[Type layout in the Reference](https://doc.rust-lang.org/reference/type-layout.html) ·
[Function pointer types](https://doc.rust-lang.org/reference/types/function-pointer.html)
