# Chapter 23 hints — Unsafe Rust

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Each lesson has two hints. Open the first one only after a real attempt, and the second one only if the first was not enough. The rules in each task's doc comment ("use `map_addr`", "without `Vec::pop`") are part of the exercise. *Check your explanation* is a model answer to the lesson's EXPLAIN prompt; read it after you have written your own.

## l01 · A safe invariant boundary

<details><summary>Hint 1</summary>

Unsafe code may rely on a fact about private data (an invariant) only if no safe caller can make that fact false. Look at which functions can change the bytes.

</details>

<details><summary>Hint 2</summary>

Validate with `is_ascii`, and give the same `Vec` back in `Err` when the check fails. Borrow with `std::str::from_utf8_unchecked` inside an `unsafe` block. Before the block, write a SAFETY comment that explains why every stored byte is ASCII.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

The private invariant is that every byte in `Ascii.0` is at most 0x7f. `new` establishes it by checking `is_ascii()` before constructing, and gives the caller's Vec back unchanged if the check fails. The field is private, so code outside the module cannot break it. `lowercase` keeps it because ASCII lowercasing maps ASCII to ASCII. The only unsafe operation is `str::from_utf8_unchecked` in `as_str`: its precondition, valid UTF-8, follows from the invariant because every ASCII sequence is valid UTF-8, including `\0`. `unsafe` does not switch off the borrow checker. It only permits a short list of extra operations (dereference a raw pointer, call an unsafe function, use a `static mut`, read a union field, implement an unsafe trait), so the returned `&str` still borrows `self` as usual.

</details>

## l02 · Raw pointer reads with contracts

<details><summary>Hint 1</summary>

An unsafe function cannot check that a pointer is valid; it can only say what the caller must guarantee. The safe caller gets a good pointer in the easiest way: it derives the pointer from a reference it already holds.

</details>

<details><summary>Hint 2</summary>

Check `index < values.len()` first. Then both `values.as_ptr().add(index)` and the call to `read_u32` are unsafe operations, and they can share one `unsafe` block. The SAFETY comment needs two parts: the offset stays inside the slice (for `add`), and the pointer is valid for a read (for `read_u32`). Reading a `u32` copies it; nothing changes owner.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`read_u32`'s caller must guarantee that the pointer is non-null, aligned for u32, and points to an initialized u32 inside a live allocation it may read from, with no conflicting writer during the read. Non-null alone proves nothing: a dangling pointer is non-null. `at` discharges every obligation. It checks `index < values.len()` first, so `values.as_ptr().add(index)` stays in bounds of the slice it came from. Slice elements are aligned and initialized. The shared borrow `&[u32]` keeps the allocation alive and excludes writers for the duration of the call. Creating a raw pointer is safe, and so is `wrapping_add`. `add` and `offset` are unsafe functions, because the result must stay inside the same allocation; going outside is undefined behavior even if nothing is read. Dereferencing is unsafe, because the pointee must be valid.

</details>

## l03 · Raw slice reconstruction

<details><summary>Hint 1</summary>

The safety argument is about *creating* the two `&mut` slices, not only about using them later. An empty region still needs a pointer that is non-null and aligned.

</details>

<details><summary>Hint 2</summary>

Read `len` and call `as_mut_ptr()` once, before you build either slice. The halves are `from_raw_parts_mut(p, mid)` and `from_raw_parts_mut(p.add(mid), len - mid)`. For the swap, `ptr::swap` accepts two equal pointers; `copy_nonoverlapping` does not.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

Bounds: `mid <= len` is checked, so `[0, mid)` and `[mid, len)` lie inside the slice. Provenance: both pointers derive from `values.as_mut_ptr()`, so they may access the whole slice. Alignment and initialization follow from the original `&mut [T]`. Lifetime: both outputs borrow from the input's unique borrow, which the signature ties to the result. Non-overlap: the two ranges are disjoint, so the two `&mut` slices never alias. Merely *creating* overlapping `&mut` references is already undefined behavior, even if neither is used. A zero-length slice still needs a non-null, aligned pointer, which `as_mut_ptr().add(len)` provides. `ptr::swap` explicitly allows both pointers to be equal; `copy_nonoverlapping` does not.

</details>

## l04 · A flagged borrow

<details><summary>Hint 1</summary>

A pointer is an address plus the permission to use one allocation (its provenance). Change only the address. Do not turn the pointer into an integer and build a new pointer from that integer.

</details>

<details><summary>Hint 2</summary>

`ptr.map_addr(|a| a | 1)` sets the tag and keeps the provenance; `ptr.map_addr(|a| a & !1)` clears it. Clear the tag before you dereference. `PhantomData<&'a u32>` tells the borrow checker that the struct holds a borrow for `'a`.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

A raw pointer carries no lifetime, so `Flagged` stores `PhantomData<&'a u32>`. To the borrow checker the struct holds a `&'a u32`, so it cannot outlive the borrowed value, and it costs zero bytes. u32 is 4-aligned, so the two low address bits of a valid pointer are always zero, and one of them can hold the flag. `map_addr` changes only the address bits and keeps the pointer's provenance, so after masking the tag you have the *same* pointer again, valid to dereference. A round trip through `usize` with `as` also works, but in a weaker way: `ptr as usize` *exposes* the provenance, and `usize as *const T` asks for "some exposed provenance that fits this address". That is permitted. The cost is that the compiler and Miri can no longer follow which allocation the pointer belongs to, so some optimizations are lost and Miri may miss bugs. `map_addr` keeps the provenance intact. It is the strict-provenance way, and the one to prefer.

</details>

## l05 · Layout, unsafe traits, unions

<details><summary>Hint 1</summary>

An unsafe trait puts a promise on everyone who writes `unsafe impl`. Generic unsafe code may then rely on that promise for every `T` with the bound.

</details>

<details><summary>Hint 2</summary>

For A, cast the reference to `*const u8` and build a shared slice of `size_of::<T>()` bytes with `slice::from_raw_parts`. For B, ask the compiler: `size_of`, `align_of`, `offset_of!`. For C, create `Word { float: value }` and read `.integer` in an unsafe block; reading a union field is always unsafe, even when your pair of fields is harmless.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

The `Plain` contract says the type is Copy, has no padding, and every bit pattern of its size is a valid value. Generic unsafe code such as `bytes` relies on it to read the value as initialized bytes. Implementing it is an `unsafe impl` because the compiler cannot check the promise. bool would break it: only 0 and 1 are valid, so treating arbitrary bytes as a bool is undefined behavior. The same invalid patterns are why `Option<bool>` fits in one byte. Padding bytes are uninitialized, so a struct with padding cannot be viewed as `&[u8]` either. Reading a union field reinterprets the stored bits without validating them, so it is only sound when every pattern is valid for the field you read, as with u32. Layout facts such as size, alignment, and offset should be measured with `size_of`, `align_of`, and `offset_of!`, never assumed.

</details>

## l06 · Moving values and initialization

<details><summary>Hint 1</summary>

`ptr::read` copies the bits of a value and, by convention, transfers ownership to the copy. The compiler does not know that. You must make sure the old place is never treated as an owner again.

</details>

<details><summary>Hint 2</summary>

For A, call `set_len(len - 1)` first and then `ptr::read` the slot at the old last index. For B, `MaybeUninit::write` initializes a slot, and `assume_init` is justified only after every slot was written.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`ptr::read` copies the bits and transfers ownership, but the compiler still thinks the source is initialized. The record of what is initialized (here the Vec's length) must change, so that the old location is no longer treated as an owner. In `raw_pop`, `set_len(len - 1)` excludes the slot before or immediately after the read, with nothing in between that could panic. If a panic unwound between the read and the length update, the Vec would drop the slot again: a double drop. Other mistakes leak instead (a missed drop), which is safe but wrong. `MaybeUninit` never drops its contents, so `assume_init` (or `assume_init_read`) is justified only once every slot has been written, and the result then becomes the sole owner.

</details>

## l07 · A fixed-capacity byte gap buffer

<details><summary>Hint 1</summary>

The cursor is `start`. To move the cursor left, the bytes just before the gap must go to the end of the gap. To move it right, the bytes just after the gap must go to the start of the gap.

</details>

<details><summary>Hint 2</summary>

Left, with `d = start - position`: copy `position..start` to `end - d`, then subtract `d` from both `start` and `end`. Right, with `d = position - start`: copy `end..end + d` to `start`, then add `d` to both. Use `ptr::copy(source, destination, count)`. The two ranges can overlap, so `copy_nonoverlapping` is wrong here. Only a debug build (or Miri) notices that mistake; a release build runs it silently. That is one more reason why the written range proof matters.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

The buffer holds `storage.len()` bytes. The text is `storage[..start]` followed by `storage[end..]`, the gap is `[start, end)`, and the cursor is `start`. The invariant is `0 <= start <= end <= storage.len()`, and the logical length is `storage.len() - (end - start)`. Moving the cursor left by d copies `storage[pos..start]` to `end - d`. Moving right copies `storage[end..end + d]` to `start`. In either case the source and destination can overlap, so the copy must be `ptr::copy` (memmove), not `copy_nonoverlapping`. Insertion writes at `start` when `start < end`; `start == end` means the buffer is full. Every byte stays initialized (the gap holds stale but valid u8s), so no ownership is at stake. A generic `Gap<T>` would need `MaybeUninit` and careful drops.

</details>

## l09 · Interior mutability and layout attributes

<details><summary>Hint 1</summary>

A: `UnsafeCell::get(&self)` returns a `*mut T` to the contents. That raw pointer is the only legal way to write behind a shared reference. B: a pointer to `u32` can be cast to a pointer to a type that is *guaranteed* to have the same layout. C: a raw pointer may be misaligned as long as you never dereference it normally; `&raw const place` makes such a pointer without making a reference.

</details>

<details><summary>Hint 2</summary>

A: `replace` is `unsafe { std::ptr::replace(self.value.get(), value) }`; `set` is `drop(self.replace(value))`; `get` is `unsafe { *self.value.get() }`. B: put `#[repr(transparent)]` on `UserId`, then `slice::from_raw_parts(raw.as_ptr().cast::<UserId>(), raw.len())`. C: `&raw const wire.count`, and for `read_at`: `offset.checked_add(size_of::<T>())?`, compare with `bytes.len()`, then `bytes.as_ptr().add(offset).cast::<T>().read_unaligned()`.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

A shared reference `&T` promises that the value does not change while the reference lives, and the compiler optimizes with that promise. `UnsafeCell<T>` is the one type that removes the promise for its contents: `get` hands out a `*mut T` from `&self`. `MyCell` is sound because of two facts. First, it never gives out a reference to the contents; every method copies or moves a whole `T` during the call, so no reference can observe a change. Second, it is `!Sync` (automatically, because `UnsafeCell` is), so two threads can never hold `&MyCell` at once; with `unsafe impl Sync`, two threads could call `set` at the same moment, which is a data race. `set` uses `replace` and drops the old value afterwards, because a destructor is user code and may call the cell again; at that point the cell already holds the new value and no pointer into it is in use. `repr(transparent)` guarantees that `UserId` has the size, alignment and ABI of its single field, so a `*const u32` may be cast to `*const UserId`; with the default layout the cast would only *happen* to work. `repr(packed)` removes padding and with it the alignment of the fields. So a reference to a field must never exist (`E0793`); you copy the field by value, or take a raw pointer with `&raw const` and use `read_unaligned`.

</details>

## l08 · Capstone: initialized-prefix storage

<details><summary>Hint 1</summary>

`MaybeUninit` does not drop its inner `T` automatically. The length is the only record of which slots hold a value, so it must decide both the raw reads and the cleanup.

</details>

<details><summary>Hint 2</summary>

Use `write` for push, `assume_init_read` for pop after shrinking `len`, and `slice::from_raw_parts` for a shared view of only the prefix. No operation that can panic belongs after a value was moved out and before `len` stops counting its old slot.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

The invariant is `len <= N`, with exactly `data[..len]` initialized and owned by the container. `push` checks capacity, writes the slot, then increments `len`, with no user code or panicking operation in between. A failed push returns the value untouched. `pop` decrements `len` *first*, so the slot is no longer counted, then `assume_init_read`s it. The caller owns the result, and Drop will not touch it again. `as_slice` builds `&[T]` from `data.as_ptr().cast()` and `len`, and never covers unused slots, even if they still hold old bits. The supplied Drop stops counting each element before it drops it. A panicking destructor can therefore leak the rest, which is safe, but never cause a double drop.

</details>

## R01 · Spaced review of Chapters 17, 21, and 22

<details><summary>Hint 1</summary>

A: bind `$slice` and then `$index` to locals before anything else, exactly as in Chapter 22's evaluate-once macros. B: a Waker is a data pointer plus a vtable, and the data pointer here is an `Arc` turned into a raw pointer. C: a byte below 0x80 never appears inside a multi-byte UTF-8 sequence.

</details>
<details><summary>Hint 2</summary>

A: `if index < slice.len() { Some(unsafe { slice.get_unchecked(index) }) } else { None }`. B: `Arc::into_raw(wakes).cast()` and `Waker::from_raw(RawWaker::new(data, &VTABLE))`. Clone calls `Arc::increment_strong_count`, `wake` rebuilds the Arc with `Arc::from_raw` (and drops it), and `wake_by_ref` only borrows `&*data.cast::<AtomicUsize>()`. C: `text.bytes().position(|b| b == sep)?`, then `get_unchecked(..i)` and `get_unchecked(i + 1..)`.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one returns the right elements in the wrong order, and one accepts coordinates that are outside the grid. Then, for every `unsafe`, ask what it buys compared with a safe std method, and what happens if anything between a `ptr::read` and `set_len` panics.

</details>
<details><summary>Hint 2</summary>

`labels.split_off(labels.len().saturating_sub(n))` moves the tail in order with one allocation. In `get`, reject `col >= cols` and use `checked_mul`/`checked_add` before a plain `get(i)`. `copy_prefix` has no caller obligations, so it should be a safe fn using `copy_from_slice`.

</details>
<details><summary>Check your review</summary>

`take_last` reads elements back to front, so the output is reversed. It also grows `out` from empty, and `push` can allocate or panic after some elements have been `ptr::read` but before `set_len`. A panic there would leave each moved String owned twice, a double drop. `split_off` does the same work safely. `Grid::get` checks only the flat index, so `(0, 3)` silently returns cell `(1, 0)`. `row * cols` can overflow: that panics in debug and wraps in release, where a huge row such as `usize::MAX / 3 + 1` lands back inside the grid. The `get_unchecked` saves nothing over `get`. `copy_prefix` is marked `unsafe fn` although callers have nothing to uphold. Its body calls an unsafe function without an `unsafe {}` block, which Rust 2024 warns about (`unsafe_op_in_unsafe_fn`), and there is no SAFETY comment. `copy_from_slice` is the safe equivalent.

</details>

## Drills

Run `cargo drill ch23`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · Dereferencing a raw pointer</summary>

`E0133`. Creating `p` is safe; reading through it is one of the operations `unsafe` unlocks, because the compiler cannot prove `p` is valid. Wrap only the read: `let y = unsafe { *p };` with a SAFETY comment saying `p` comes from a live reference.

</details>
<details><summary>d02 · Making pointers is safe</summary>

Compiles and runs. `wrapping_add` and comparisons never touch memory, so they are safe even far outside the array. Dereferencing needs `unsafe`, and so does `add`, which requires that the result stays inside the allocation. An address is not permission to read.

</details>
<details><summary>d03 · Writing through *const</summary>

`E0594`. A `*const i32` does not permit writes, even inside `unsafe`. Declare `let p: *mut i32 = &mut x;`. `unsafe` grants extra operations; it does not change what a type allows.

</details>
<details><summary>d04 · Implementing an unsafe trait</summary>

`E0200`. An `unsafe trait` puts a promise on implementors, so the impl must say `unsafe impl Plain for u32 {}`, ideally with a SAFETY comment explaining why every bit pattern is a valid u32.

</details>
<details><summary>d05 · unsafe impl of a safe trait</summary>

`E0199`. `Clone` has no safety contract, so there is nothing to promise. Remove `unsafe`. Marking code unsafe without a contract only hides real unsafety.

</details>
<details><summary>d06 · Raw pointers across threads</summary>

`E0277`: `*const i32` is not `Send`. The compiler has no idea who owns the pointee or how long it lives. Move the owner instead: `thread::spawn(move || *data)`. Wrapping the pointer in a type with `unsafe impl Send` would need a real proof.

</details>
<details><summary>d07 · transmute between sizes</summary>

`E0512`: f32 is 4 bytes and u64 is 8, and `transmute` requires equal sizes. Use the safe API: `1.5_f32.to_bits() as u64`.

</details>
<details><summary>d08 · A mutable static</summary>

`E0133`: any access to a `static mut` is unsafe, because another thread could be using it at the same moment (a data race). `unsafe { HITS += 1 }` compiles, but the idiomatic fix is `static HITS: AtomicU32`, with `fetch_add` and `load`, and no unsafe at all. Edition 2024 goes one step further for references: `&HITS` and `&mut HITS` on a `static mut` are rejected even inside `unsafe` (the lint `static_mut_refs` is an error by default: "creating a shared reference to mutable static"), because such a reference is undefined behavior as soon as anything else writes the static. If you must keep a `static mut`, read and write it by value or through `&raw const` / `&raw mut` pointers.

</details>
<details><summary>d09 · Checked layouts</summary>

Panics: `Layout::from_size_align` returns `Err` because an alignment must be a power of two, and `unwrap` panics. Use 4 (or `Layout::new::<[u8; 12]>()`). Safe constructors check layout rules at run time; unchecked ones make them your obligation.

</details>
<details><summary>d10 · Moving out through a raw pointer</summary>

`E0507`. `*p` is a place you may read, but moving the String out would leave `name` and `copy` both owning one buffer. Clone it (`unsafe { (*p).clone() }`), or use `ptr::read` only when the original is then forgotten.

</details>

## Mastery check: model answers

Answer the questions in the [chapter guide](README.md#mastery-check) from memory first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**1. What stays your responsibility inside an unsafe block?**
Everything the compiler normally proves for you about the extra operations: that a raw pointer you dereference is non-null, aligned, inside a live allocation and points to an initialized, valid value; that no `&mut` aliases another reference; that no two threads race on the same memory; that values are dropped exactly once; and that every unsafe function you call gets what its `# Safety` section demands. The borrow checker and the type checker keep working on all ordinary code inside the block.

**2. An unsafe function's contract versus a safe abstraction's invariant.**
An `unsafe fn` cannot defend itself, so it states preconditions, and each *caller* must prove them at each call site (the SAFETY comment). A safe abstraction contains unsafe code but exposes only safe functions. It must be impossible to cause undefined behavior through its public API, whatever a safe caller does. It achieves that with an invariant on private data (for example "every byte is ASCII") that all its functions establish and keep. The proof obligation moves from every caller to the one module.

**3. What makes a read through a raw pointer valid?**
The pointer must have provenance for the memory (it was derived from a pointer or reference to that allocation), the bytes must lie inside the allocation and the allocation must still be alive, the address must be aligned for the type (unless you use `read_unaligned`), the bytes must be an initialized, valid value of the type, and no `&mut` or writer in another thread may conflict with the read. If the read moves a non-`Copy` value, ownership must also be accounted for.

**4. When must a raw move update the record of ownership?**
Always, and with no panic possible in between. `ptr::read` and `assume_init_read` make a second copy of the bits; from then on exactly one of the two places may be treated as the owner. The container must stop counting the old slot (`set_len`, `len -= 1`) before any code that can panic or call user code runs. Otherwise unwinding drops the slot a second time.

**5. How do size, alignment, padding and valid bit patterns limit reinterpreting memory?**
The target type must not be larger than the memory you have. The address must be aligned for it, or you must use `read_unaligned`. Padding bytes are uninitialized, so a type with padding cannot be read as bytes. Not every bit pattern is a valid value: `bool` allows two, `char` excludes surrogates and values above 0x10FFFF, references must be non-null and aligned, and enums allow only their discriminants. Producing an invalid value is undefined behavior at once. The default layout of a struct is not specified; only `repr(C)`, `repr(transparent)` and friends give guarantees.

**6. How does a gap buffer keep its two regions disjoint?**
By the invariant `0 <= start <= end <= capacity`. The text is `[0, start)` followed by `[end, capacity)`; the gap `[start, end)` is free space. Insertion fills the gap from the left, deletion widens it to the right, and moving the cursor copies bytes across the gap and shifts `start` and `end` by the same distance, so the two regions never overlap and the gap keeps its size.

**7. What must stay true if a user-provided operation panics?**
The container's invariant must hold at every point where user code can run (a callback, a `Clone`, a `Drop`, an iterator). Then unwinding finds a valid container: its `Drop` drops exactly the initialized elements. Leaking a value during a panic is acceptable; dropping one twice or reading an uninitialized slot is not.

**8. What do tests and Miri show, and what still needs a written proof?**
Tests show that the executed inputs gave the expected results on this machine and build. Miri additionally reports undefined behavior on the paths that the tests execute: out-of-bounds and misaligned access, use after free, reads of uninitialized memory, violations of the aliasing model, some data races. Neither covers inputs or types that the tests never use, other targets, or layout assumptions that happen to hold. A claim of the form "for every `T` and every sequence of calls" needs the written argument from the invariant.

**9. Why is `UnsafeCell` needed, why is a cell `!Sync`, and what do `repr(transparent)` and `repr(packed)` change?**
Writing through anything derived from a plain `&T` is undefined behavior; `UnsafeCell<T>` is the only exception, so every type with interior mutability (`Cell`, `RefCell`, `Mutex`, atomics) contains one. A cell without a lock allows unsynchronized writes through `&self`, so sharing `&Cell` between threads would be a data race; `!Sync` forbids it. `repr(transparent)` guarantees that a one-field wrapper has the layout and ABI of that field, which makes pointer casts between the two sound. `repr(packed)` removes padding and lowers the alignment to 1, so fields may sit at misaligned addresses: copy them by value or use `&raw const` with `read_unaligned`, and never take a reference to them.

</details>
