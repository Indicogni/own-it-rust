# Mastery M2 hints

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Notes](NOTES.md) · [All chapters](../../README.md)

Each lesson has two hints. Open the first one only after a real attempt, and the second one only if the first was not enough. *Check your explanation* is a model answer to the lesson's EXPLAIN prompt; read it after you have written your own. *Miri lab* says what was wrong in the lab program and how the repair works; read it after you have seen Miri's report yourself.

A hint in this module never replaces the SAFETY comment. If your code passes the tests and you cannot say why each `unsafe` block is sound, the lesson is not finished.

## 01 · UnsafeCell: building Cell and RefCell

<details><summary>Hint 1</summary>

`UnsafeCell::get(&self)` returns a `*mut T` that you may write through. For `MyCell`, never turn that pointer into a reference that outlives one expression, and never let code of `T` run while you use it. The functions in `std::ptr` that only move bytes are your tools: `ptr::replace`, `ptr::swap`, and a plain read for `Copy` types. For `MyRefCell`, all the `unsafe` is in the three `deref` functions; the borrow rules live in `try_borrow`, `try_borrow_mut` and the two destructors, which are safe code on a `MyCell<isize>`.

</details>
<details><summary>Hint 2</summary>

`replace` is `unsafe { std::ptr::replace(self.value.get(), value) }`. Build `set` and `take` from `replace`: `set` is `drop(self.replace(value))`, so the old value is dropped when it is an ordinary local and the cell already holds the new one. `swap` is `ptr::swap` on the two `get()` pointers; it also works when both are the same cell, which `mem::swap` on two `&mut` would not. In `try_borrow`, read `state`, return `None` if it is negative or equal to `isize::MAX`, otherwise add one and return `Some(Ref { cell: self })`. `Ref::drop` subtracts one; `RefMut::drop` sets 0. `borrow` is `self.try_borrow().expect("already mutably borrowed")`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`MyCell` is sound because no reference to its inside ever exists while a `&self` method runs. No such method returns one, each method touches the inside only through byte moves that run no code of `T`, and the type is not `Sync`, so no second thread is inside a method. With no reference to conflict with, a write through `&self` breaks no promise. `MyRefCell` does hand out references, so it counts them: `state` is the number of live `Ref` guards, or -1 for one `RefMut`. A `&T` exists only while a `Ref` is alive and a `&mut T` only while the `RefMut` is, and `try_borrow*` refuses any combination that the borrow rules forbid. `set` must drop the old value after the new one is in place, because the old value's destructor is foreign code: it may call `set` on the same cell. If the old value were dropped in place, that nested call would write to a slot whose value is in the middle of being destroyed, and the value would be dropped a second time.

</details>
<details><summary>Miri lab</summary>

The SAFETY comment in `record` lists true facts (non-null, aligned, initialized, one thread) and leaves out the one that matters: the pointer was made from a `&u32`, so it only has permission to read. The repair is to store the count in a `Cell<u32>`. The `unsafe` block disappears, and so does the helper `writable`, which should not exist in any program.

</details>

## 02 · Layout, alloc, dealloc and NonNull: building Box

<details><summary>Hint 1</summary>

Compute `Layout::new::<T>()` once in `new`. If its size is 0, use `NonNull::dangling()` and do not call the allocator. Otherwise call `alloc`, turn the result into a `NonNull` and call `handle_alloc_error(layout)` if it was null. Store the value with `ptr.write(value)` in both cases. The destructor does the reverse in two steps: `drop_in_place`, then `dealloc` with the same layout, skipped for size 0. Three methods must *not* run that destructor: `into_inner`, `into_raw` and `leak`.

</details>
<details><summary>Hint 2</summary>

Write one private helper `unsafe fn free<T>(ptr: NonNull<T>)` that deallocates with `Layout::new::<T>()` unless the size is 0, and use it from `drop` and `into_inner`. `into_inner`: `let this = ManuallyDrop::new(self); let value = unsafe { this.ptr.read() }; unsafe { free(this.ptr) }; value`. `into_raw`: `ManuallyDrop::new(self).ptr.as_ptr()`. `from_raw`: `NonNull::new_unchecked(ptr)` in a new `MyBox`. `leak`: `unsafe { &mut *self.into_raw() }`. `deref` and `deref_mut` are `self.ptr.as_ref()` and `self.ptr.as_mut()`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The invariant has three facts. (1) `ptr` is non-null, aligned and points to an initialized `T`: `new` establishes it with `alloc` or `dangling` followed by `write`, and `deref`, `deref_mut`, `into_inner` and `drop` rely on it. (2) For a sized `T` the block came from the global allocator with `Layout::new::<T>()` and this box is its only owner: `new` establishes it, and `drop` and `into_inner` rely on it when they call `dealloc` with that layout. (3) For a zero-sized `T` there is no block: `new` establishes it by never calling `alloc`, and the freeing code relies on it by skipping `dealloc`. `into_inner`, `into_raw` and `leak` wrap `self` in `ManuallyDrop` (or call `mem::forget`), so that `Drop for MyBox` does not run at the end of the method. `into_inner` then does half of the destructor's work by hand: it frees the block but does not drop the value, which now belongs to the caller.

</details>
<details><summary>Miri lab</summary>

`into_inner` reads the value out and returns it, but `self` is still an ordinary local, so `Drop for MiniBox` runs at the closing brace: it drops the value a second time (freeing the `String`'s buffer) and frees the block. The SAFETY comment says that nobody else can use the box "any more", which is true and beside the point: the box's own destructor is the second user. The repair wraps `self` in `ManuallyDrop` and frees the block by hand.

</details>

## 03 · A growable buffer: building Vec

<details><summary>Hint 1</summary>

Fields: `ptr: NonNull<T>`, `cap: usize`, `len: usize`, `PhantomData<T>`. For a zero-sized `T` keep `ptr` dangling and set `cap` to `usize::MAX` in `new`; then `push`, `pop`, `insert` and `remove` need no special case, and only the code that allocates and frees must check the size of `T`. Write one private method that moves the elements to a block of a given capacity (first `alloc`, later `realloc`), and build `with_capacity` and `reserve` on it. Decide for each method at which line the picture "slots `0..len` are full, the rest is empty" stops being true and at which line it is true again; no call that can panic may stand between those lines.

</details>
<details><summary>Hint 2</summary>

`grow_to(new_cap)`: `Layout::array::<T>(new_cap)` or panic with `capacity overflow`; if `cap == 0` call `alloc(new_layout)`, else `realloc(ptr.cast(), Layout::array::<T>(cap).unwrap(), new_layout.size())`; null means `handle_alloc_error(new_layout)`. `reserve(n)`: `needed = len.checked_add(n)` or panic; if `needed > cap`, grow to `max(needed, cap * 2, 4)`. `push`: `reserve(1)`, `ptr.add(len).write(value)`, `len += 1`. `insert`: assert, `reserve(1)`, then `slot.copy_to(slot.add(1), len - index)` and `slot.write(value)`. `remove`: `slot.read()`, then `slot.add(1).copy_to(slot, len - index - 1)`. `truncate`: build `ptr::slice_from_raw_parts_mut(ptr.add(new_len), len - new_len)`, set `len = new_len`, then `drop_in_place` the slice. `Drop`: create a local guard whose destructor frees the block, then call `clear()`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Invariant: `ptr` is non-null and aligned; for a zero-sized `T` there is no block and `cap` is `usize::MAX`; otherwise `cap == 0` means no block and `cap > 0` means a block from the global allocator with `Layout::array::<T>(cap)` that the vector alone owns; `len <= cap`, slots `0..len` hold values owned by the vector and slots `len..cap` hold none. `push` breaks it for a moment after `write` (slot `len` is full but not counted) until `len += 1`; nothing runs in between. `insert` breaks it between the `copy`, which leaves two bitwise copies of one element, and the `write` plus `len += 1`; again nothing foreign runs in between, and `reserve`, which can panic, is called before. `remove` breaks it between `read` and `len -= 1`: slot `index` is empty, then the last slot holds a stale duplicate; only pointer operations run there. `truncate` is the one method that runs foreign code, the destructors of the tail. It lowers `len` first, so during those destructors the invariant already holds for the shorter vector, and a panic can at worst leak elements.

</details>
<details><summary>Miri lab</summary>

`*slot = value` is an assignment, and an assignment drops the old value of the place first. The slot above `len` has no old value, so the destructor of a `String` runs on whatever bytes the allocator returned. The SAFETY comment proves that the slot is in bounds and aligned; it says nothing about what the slot contains, which is exactly the missing fact. The repair is `slot.write(value)`.

</details>

## 04 · IntoIter and Drain: panic safety and leak safety

<details><summary>Hint 1</summary>

`IntoIter`: take the vector apart with `into_raw_parts` and keep the pointer, the capacity and two indices, `front` and `back`; the values in `front..back` are yours. `Drain`: keep the `&'a mut MyVec<T>`, the two indices of the range that is left, and where the tail starts and how long it is. Call `set_len(start)` *in `drain`*, before any element is moved. Both destructors run foreign code (the destructors of the remaining elements) and both have one step that must happen afterwards whatever happens: freeing the block, and moving the tail back. Put that step into the destructor of a local value.

</details>
<details><summary>Hint 2</summary>

`IntoIter::drop`: first `let _block = unsafe { MyVec::from_raw_parts(self.buf, 0, self.cap) };`, an empty vector whose destructor frees the block with the right layout, also during unwinding. Then `drop_in_place` the slice `front..back`. `Drain::drop`: define a local `struct RestoreTail<'r, 'a, T>(&'r mut Drain<'a, T>)` whose `Drop` does `base.add(tail_start).copy_to(base.add(start), tail_len)` and `set_len(start + tail_len)`, where `start` is the vector's current length and `base` is `vec.as_mut_ptr()`. Create it, then `drop_in_place` the slice `front..back` through the guard's reference. `next` in both types: check `front == back`, advance the index, then `read` the slot. For `Drain`, read through `self.vec.as_mut_ptr()`, which is valid for the whole capacity.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

(a) While the drain is alive, the vector's length is the start of the range, so the vector owns only the elements before the range. The `Drain` owns the elements of the range that it has not returned yet, and the tail. The borrow checker keeps everyone else away from the vector, because the `Drain` holds the `&mut`. (b) After the drain is dropped, the remaining elements of the range have been dropped, the tail has been moved down to the start of the range, and the length is `start + tail_len`: the vector is contiguous again. (c) After `mem::forget(drain)` nothing more happens: the vector keeps the length `start`. That is safe because the vector claims only slots that are still full and still its own; the slots it no longer claims are never read or dropped by it. The cost is a leak that is larger than strictly necessary: the unreturned part of the range and the whole tail are never dropped, although only one element may have been taken. This is leak amplification, and it is the price of not depending on a destructor for soundness.

</details>
<details><summary>Miri lab</summary>

`TakeFront` leaves the vector's length alone and repairs the vector in its destructor. `mem::forget` is safe, so that destructor may never run, and then the vector still claims a slot whose value was moved out: the `String` "a" is dropped once by `main` and once by the vector. The SAFETY comment in `next` ends with "its destructor removes the slot", which is a hope about code that may not run. The repair sets the length to 0 in `take_front` and stores the old length in the iterator. After the repair Miri reports two leaked strings for the forgotten iterator, which is correct and safe.

</details>

## 05 · Shared ownership: building Rc and Weak

<details><summary>Hint 1</summary>

Put two `Cell<usize>` counters and the value into one private struct, allocate it with `Box::new`, and keep a `NonNull` to it in both `MyRc` and `MyWeak`. Let the weak counter start at 1: all `MyRc` pointers together hold one weak reference, and the allocation is freed by whoever brings the weak counter to 0. That gives one rule for freeing and makes the hard cases (a `MyWeak` inside the value, a destructor that panics) fall out without special code. Never create a reference to the whole inner struct; borrow one field at a time through the raw pointer, `&(*ptr).strong`.

</details>
<details><summary>Hint 2</summary>

`MyRc::drop`: subtract one from `strong`; if it is now 0, first create `let _weak = MyWeak { ptr: self.ptr };` (it releases the shared weak reference at the end of the function, also on a panic), then `ptr::drop_in_place(&raw mut (*ptr).value)`. `MyWeak::drop`: subtract one from `weak`; if it is now 0, `alloc::dealloc(ptr.cast(), Layout::new::<Inner<T>>())`, which is valid for a block that came from `Box::new`. Do not rebuild a `Box` there: its destructor would drop the value again. `upgrade`: `None` if `strong` is 0, else add one and build a `MyRc`. `get_mut`: `Some` only if `strong == 1 && weak == 1`. `try_unwrap`: if `strong == 1`, wrap `this` in `ManuallyDrop`, set `strong` to 0, create the `_weak` guard, and `ptr::read` the value. `weak_count` is the counter minus 1. Every addition goes through `checked_add(1).expect(..)`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Invariant, for as long as the allocation exists: `strong` is the number of `MyRc` pointers; `weak` is the number of `MyWeak` pointers plus one while `strong > 0`; the value is initialized exactly while `strong > 0`; the allocation is freed exactly when `weak` reaches 0. Since the `MyRc` pointers hold one weak reference together, `weak` cannot reach 0 while a `MyRc` exists, so both kinds of pointer can always read the counters. `get_mut` returns `&mut T`, which must be the only way to the value for as long as it lives. Another `MyRc` would be a second way now. A `MyWeak` would be a second way a moment later, through `upgrade`, so `get_mut` must refuse while any `MyWeak` exists. A counter that wraps around would break the first line of the invariant: after `usize::MAX` leaked clones and one more, `strong` would be small again while many pointers exist, and a few drops would destroy the value under them. `mem::forget(rc.clone())` in a loop is safe code, and an optimizer may turn the loop into one addition, so this is reachable.

</details>
<details><summary>Miri lab</summary>

`Strong::drop` frees the allocation when the last owner goes, although `Weak` pointers still read the counters in it. The SAFETY comment in `is_alive` says that reading plain integers "cannot go wrong": true for the type, false for the memory, which is gone. The repair starts the weak counter at 1, lets the last `Strong` drop only the value and then give up the shared weak reference, and frees the allocation in `Weak::drop` when the counter reaches 0.

</details>

## 06 · The aliasing rules, as Miri enforces them

<details><summary>Hint 1</summary>

Task A: check the three failure cases first. Then call `as_mut_ptr()` once and do not use `slice` again in the function. Both element pointers come from that one pointer with `add`. Task B: decide who owns a node. If a `Box` owns it, every move of that `Box` claims the node exclusively again, and a raw `tail` pointer to the same node loses its permission. So let no `Box` exist between method calls: turn each new node into a raw pointer at once, and turn it back into a `Box` only in `pop_front`, when the node leaves the list.

</details>
<details><summary>Hint 2</summary>

Task A: `let base = slice.as_mut_ptr(); Some(unsafe { (&mut *base.add(i), &mut *base.add(j)) })`. Task B: `struct Node<T> { value: T, next: Option<NonNull<Node<T>>> }`, and in the queue `head` and `tail` of type `Option<NonNull<Node<T>>>`, `len`, and `PhantomData<Box<Node<T>>>`. `push_back`: `let node = NonNull::from(Box::leak(Box::new(Node { value, next: None })));` then either `(*tail.as_ptr()).next = Some(node)` or `self.head = Some(node)`, and `self.tail = Some(node)`. `pop_front`: `let node = unsafe { Box::from_raw(head.as_ptr()) }; self.head = node.next;` and if the head is now `None`, clear `tail` as well. `peek_front` is `self.head.map(|n| unsafe { &(*n.as_ptr()).value })`. `Drop` is `while self.pop_front().is_some() {}`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A raw pointer gets its permission from the reference or pointer it was made from, and it keeps that permission only while its source is left alone. Under Stacked Borrows, three things end it: a write through the source or through anything older than the source; a new `&mut` to the same memory that is not derived from the raw pointer (creating it counts, even if it is never used); and a use of the source as a `&mut` function argument or a moved `Box`, which claims exclusive access for the whole call. A shared reference is weaker still: a pointer made from `&T` can never write, except into an `UnsafeCell`. Specimen `c` is judged differently by the two models. It takes a pointer to element 0 with `&mut v[0]` and writes element 1 through it. Stacked Borrows gives the pointer permission for the four bytes of element 0 only and rejects the write. Tree Borrows lets a pointer reach the rest of the allocation that its parent could reach, as long as nothing conflicts, and accepts it. A careful author does not rely on the more generous model: `v.as_mut_ptr().add(1)` has permission for the whole array under both, costs nothing, and states the intent.

</details>
<details><summary>Task M: the verdicts explained (after you have recorded yours)</summary>

- `a`: both reject. A pointer made from `&x` is read-only (`SharedReadOnly`; Tree Borrows says `Frozen`).
- `b`: both accept. Copying a raw pointer does not create a new permission; `p` and `q` are the same pointer twice.
- `c`: Stacked Borrows rejects, Tree Borrows accepts. See *Check your explanation*.
- `d`: both accept. The value is inside an `UnsafeCell` (a `Cell` contains one), so a shared reference does not make it read-only. This is what `Cell::as_ptr` is for.
- `e`: Stacked Borrows accepts, Tree Borrows rejects. After `p` has written, reading `x` directly is a *foreign read* for `p`; Tree Borrows then sets `p` to `Frozen`, and the next write through `p` is refused. Stacked Borrows only removes `Unique` items above the owner on a read and keeps the raw pointer. This is the rare case where the default model is the more generous one.
- `f`: both reject. `exclusive` is a `&mut` function argument. For the whole call nothing else may write its memory; `alias` does. The report calls the argument "protected".
- `g`: both reject, and not because of aliasing. A reference must point into a live allocation that is large enough for its type at the moment it is created, even if it is never read; `&*p.add(3)` points past the array. The raw pointer `p.add(3)` alone is fine.
- `h`: Stacked Borrows rejects, Tree Borrows accepts. The second `v.as_mut_ptr()` borrows the whole array mutably again, which under Stacked Borrows removes `first` from the stack. Under Tree Borrows creating the second pointer is not a write, and the two pointers then write to different bytes; Tree Borrows tracks permissions byte by byte, so neither disturbs the other.

</details>
<details><summary>Miri lab</summary>

The SAFETY comment argues that the node is *alive* and *does not move*. Both are true. What it leaves out is permission: `raw` was derived from `node`, and `node` is a `Box`. Moving the box into `self.head` (or into the previous node's `next`) claims the node exclusively again, as passing a `&mut` would. Stacked Borrows removes `raw`'s tag at that move and reports the next use of `tail`. Tree Borrows lets the write through `tail` happen and then finds the box in `head` disabled when `pop_front` uses it. The repair is the design of task B.

</details>

## 07 · Send and Sync: writing the proof

<details><summary>Hint 1</summary>

Write the two types first and let the compiler tell you what it derives: `Spin<T>` with an `AtomicBool` and an `UnsafeCell<T>` is `Send` when `T: Send` (right) and never `Sync` (so one `unsafe impl` is needed). A guard that holds `&'a Spin<T>` is `Send` and `Sync` exactly when `Spin<T>` is `Sync`. Compare that with the table in the contract: one column is right, one is wrong. An explicit impl of an auto trait for a type replaces the derived one, so you can narrow it. For part B, each of your containers owns its elements and gives `&T` through `&self`; compare with `Box<T>` and `Vec<T>`.

</details>
<details><summary>Hint 2</summary>

`unsafe impl<T: Send> Sync for Spin<T> {}` and `unsafe impl<T: Sync> Sync for SpinGuard<'_, T> {}`; nothing else is needed for part A. `try_lock` is `self.locked.compare_exchange(false, true, Acquire, Relaxed).ok().map(|_| SpinGuard { lock: self })`, and `lock` loops over `try_lock` with `std::hint::spin_loop()`. The guard's `deref` is `unsafe { &*self.lock.value.get() }`, `deref_mut` the same with `&mut`, and `drop` is `self.lock.locked.store(false, Release)`. Part B: `unsafe impl<T: Send> Send` and `unsafe impl<T: Sync> Sync` for `MyBox<T>`, `MyVec<T>` and `IntoIter<T>`. A `Drain` that holds `&mut MyVec<T>` needs nothing. Do not store `&'a mut T` in the guard; see *Check your explanation*.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Spin<T>: Sync` when `T: Send`. Proof: threads that share `&Spin<T>` can each get a guard and through it `&mut T`, but the flag is `true` from a successful `compare_exchange` until the guard's destructor stores `false`, so at most one guard exists, and `Acquire` on taking and `Release` on releasing make the previous holder's writes visible to the next. The threads therefore take turns with exclusive access, which is what `T: Send` permits. Without the bound, `Spin<Rc<u8>>` would let a second thread clone the `Rc` with a count that is not atomic. `T: Sync` is not needed, because no two threads hold `&T` together; that is why `Spin<Cell<u8>>` is fine. `SpinGuard<'_, T>: Sync` when `T: Sync`. Proof: a `&SpinGuard` gives only `&T`, so sharing a guard shares a `&T`. Without the explicit impl the compiler would derive `Sync` for `T: Send`, and two threads could write through `&Cell<u8>` at once. `MyBox<T>`, `MyVec<T>` and `IntoIter<T>` own their elements and share nothing: `Send` when `T: Send` (otherwise `MyBox<Rc<u8>>` would move an `Rc` to another thread), `Sync` when `T: Sync` (otherwise two threads could use `&Cell<u8>` through `&MyBox<Cell<u8>>`). The cells stay as they are: `MyCell::set` writes through `&self` with no synchronization at all, so two threads with `&MyCell<u8>` would be a data race for every `T`.

A design that looks tidy and is wrong: a guard with the fields `&'a AtomicBool` and `&'a mut T`. Its derived auto traits are exactly the table, with no `unsafe impl`. But when the guard is passed to a function by value (`drop(guard)` is enough), the `&mut T` inside is a function argument, and it must stay exclusive until that function returns. The guard's destructor releases the lock before that, and another thread writes the value. Miri, on such a guard:

```text
error: Undefined Behavior: not granting access to tag <11169> because that would remove [Unique for <11074>] which is strongly protected (i.e. it is in use as a function argument)
help: <11074> is this argument
   |
26 |                     drop(guard);
   |                     ^^^^^^^^^^^
```

Keep a reference to the lock in the guard and create the `&mut T` anew in each `deref_mut`.

</details>
<details><summary>Miri lab</summary>

"A u64 has no invalid values" is true and does not help: a data race is undefined behaviour whatever is written, and `count += 1` is a read followed by a write, so updates can be lost even on a machine where nothing else goes wrong. The `unsafe impl Sync` is a promise that `&Hits` may be used from several threads at once, and `record` breaks it. The repair uses an `AtomicU64`, which is `Sync` by itself, so the `unsafe impl` and both `unsafe` blocks disappear.

</details>

## 08 · Capstone: a bump arena

<details><summary>Hint 1</summary>

State: a cursor and an end pointer for the current chunk, in `Cell`s; the list of chunks, each with the `Layout` it was allocated with, so that it can be freed; and a list of pending destructors. One private function does all the pointer arithmetic: given a `Layout`, round the cursor's *address* up to the alignment, check that the value fits before the end, and move the cursor. If it does not fit, add a chunk (at least twice the size of the last one, at least as large and as aligned as the request) and try once more. Everything public is built on that function plus `ptr::write` or `copy_nonoverlapping`. Handle size 0 before you get there, with `NonNull::dangling()`.

</details>
<details><summary>Hint 2</summary>

`bump(layout)`: `let addr = cursor.addr(); let padding = addr.checked_next_multiple_of(layout.align())? - addr; let needed = padding.checked_add(layout.size())?; if needed > end.addr() - addr { return None }`, then `start = cursor.add(padding)` and `cursor = start.add(layout.size())`. Keeping the arithmetic on integers and the movement on the pointer preserves provenance. A chunk type with its own `Drop` that calls `dealloc` frees the memory even when a destructor panics. For `alloc_with_drop`, push `(pointer as *mut u8, drop_value::<T> as unsafe fn(*mut u8))` when `needs_drop::<T>()`. To run the destructors in reverse and continue after a panic: pop an entry, create a local guard whose `Drop` calls the same function on the rest of the list, call the entry's function, then `mem::forget` the guard. `reset`: run the destructors, keep only the last (largest) chunk, and point the cursor at its start.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Invariant (one possible form): the chunk list owns every block taken from the global allocator, and a block is freed only by `reset` or by dropping the arena, both of which need exclusive access, so no reference handed out through `&self` is alive then. The cursor and the end point into the last chunk, and the bytes between them belong to no value. Every value handed out lies below a cursor position inside one chunk, and no two values share a byte. The pending list has one entry for each value from `alloc_with_drop` whose type needs dropping, and such a value is initialized and `'static`.

`alloc` never runs destructors because values in an arena may refer to each other, and the arena cannot know in which order, or whether, their destructors could run without one of them following a reference to a value that is already destroyed. The drop checker, which normally forbids such programs, cannot see references inside an untyped block. `alloc_with_drop` is sound because `T: 'static` rules out every reference into the arena. Without the bound, this safe program would read freed memory:

```rust,ignore
struct Shout<'a>(&'a String);

impl Drop for Shout<'_> {
    fn drop(&mut self) {
        println!("{}", self.0); // the String was destroyed a moment ago
    }
}

let arena = Arena::new();
let text = arena.alloc_with_drop(String::from("freed first"));
arena.alloc_with_drop(Shout(text));
// The arena is dropped here. Whatever order it chooses for the destructors,
// a second pair of values with the references the other way round would be
// wrong in that order.
```

With the bound, that program does not compile (`E0597`: `arena` does not live long enough, because the argument requires that `arena` is borrowed for `'static`).

No test that runs can tell you that you wrote the bound, and Miri cannot either: without it every supplied test still passes. That is why the lesson header has the three compiler checks for `cargo test --doc`. The second is this argument as a program. The third is the same idea for `reset`: with `&self` instead of `&mut self`, a reference handed out earlier could be used after its bytes were given to another value.

</details>
<details><summary>Miri lab</summary>

The SAFETY comment establishes that the bytes are inside the block and not handed out before. It never mentions alignment, and `start` is simply the number of bytes used so far: after one `u8` it is 1, and a `u64` must not start there. The repair rounds `start` up to `align_of::<T>()`, which is enough here because the block itself is aligned to 8 and the function refuses types that need more.

</details>

## C01 · Code review: a teammate's ring buffer

<details><summary>Hint 1</summary>

Do what the header says before anything else: write the SAFETY comment for each `unsafe` block as if you were the author. For `alloc(layout)`: which layouts does `alloc` forbid, and which capacities and element types produce one? For the reference created in `get`: what must be true of a slot before a `&T` to it may exist, and does `index < self.cap` establish that? For the two `unsafe impl`s: for which `T` is the statement false? Then look at the destructor and count what it gives back.

</details>
<details><summary>Hint 2</summary>

The four soundness holes: `get` (it checks the capacity, not the length, so it hands out slots that hold no value); the two `unsafe impl`s without bounds (`Ring<Rc<u8>>` is `Send`); `new` with a capacity of 0 or a zero-sized `T` (it asks the allocator for zero bytes, which is undefined behaviour); and `new` again, because it never checks the result of `alloc` for null (a capacity that the allocator cannot serve gives a ring whose `push` writes through a null pointer). All four are reachable from safe code, which is what makes them soundness holes and not ordinary bugs. The bug that is not a soundness hole is `Drop`: it frees the block and forgets the values in it, which is a leak, and a leak is safe. The smells: `*mut T` where `NonNull<T>` would give `Option<Ring<T>>` a niche and make the ring covariant, `iter` that builds a `Vec`, no invariant, no SAFETY comments.

</details>
<details><summary>Check your review (after writing your own)</summary>

**Hole 1, `get`.** After `push`, `pop`, the slot is empty again, and a new ring has only empty slots, but `get(0)` returns `Some` for both because `0 < cap`. A caller in safe code then reads a value that was never written or was already dropped. A test of your own that Miri flags in the starting code:

```rust,ignore
#[test]
fn exploit_get_hands_out_a_slot_without_a_value() {
    let mut ring = Ring::new(2);
    ring.push(String::from("gone")).unwrap();
    drop(ring.pop());
    let stale: &String = ring.get(0).expect("the hole: an empty slot is handed out");
    assert_eq!(stale.as_str(), "gone");
}
```

```text
error: Undefined Behavior: reading memory at alloc90572[0x20..0x28], but memory is uninitialized at [0x20..0x28], and this operation requires initialized memory
```

The supplied test for this hole only calls `is_none()`. It never reads through the reference, so even Miri sees no undefined behaviour in it: the *type* is unsound, but that run is clean. This is the difference between "unsound" and "this execution has UB", and the reason why Miri alone does not prove soundness. Fix: compare with `self.len`.

**Hole 2, the `unsafe impl`s.** `unsafe impl<T> Send for Ring<T>` claims that every ring may move to another thread. A ring of `Rc<u8>` then carries an `Rc` across:

```rust,ignore
#[test]
fn exploit_an_rc_is_used_on_two_threads() {
    let shared = Rc::new(0u8);
    let mut ring = Ring::new(1);
    ring.push(Rc::clone(&shared)).unwrap();
    let worker = std::thread::spawn(move || {
        for _ in 0..50 {
            drop(ring.get(0).map(Rc::clone));
        }
    });
    for _ in 0..50 {
        drop(Rc::clone(&shared));
    }
    worker.join().unwrap();
}
```

```text
error: Undefined Behavior: Data race detected between (1) non-atomic write on thread `c01_code_review::tests::exploit_an_rc_is_used_on_two_threads` and (2) non-atomic read on thread `unnamed-2` at alloc88714
```

Fix: `unsafe impl<T: Send> Send` and `unsafe impl<T: Sync> Sync`, each with its proof. After the fix this test no longer compiles, which is the best outcome an exploit can have; delete it.

**Hole 3, `new` asks for zero bytes.** For `cap == 0` or a zero-sized `T` the layout has size 0, and `Ring::<u8>::new(0)` is safe code. The system allocator usually tolerates the request, which is why this looks like a harmless bug; the contract of `alloc` calls it undefined behaviour, and another allocator may do anything. Here the supplied test `nothing_to_store_means_nothing_to_allocate` is already the exploit. Miri on the starting code, in that test:

```text
error: Undefined Behavior: creating allocation with size 0
  --> src/c01_code_review.rs:46:28
   |
46 |         let buf = unsafe { alloc(layout) } as *mut T;
   |                            ^^^^^^^^^^^^^ Undefined Behavior occurred here
```

The native test catches it too, because the test allocator counts requests for zero bytes. Fix: `NonNull::dangling()` when the layout's size is 0, and no `dealloc` for such a ring.

**Hole 4, `new` does not check for null.** `alloc` returns null when it cannot serve the request, and the starting code stores that null pointer as the buffer. The safe program is two lines:

```rust,ignore
let mut ring = Ring::<u8>::new(isize::MAX as usize);
ring.push(1).unwrap(); // writes one byte through a null pointer
```

No supplied test shows this hole, and Miri does not show it either: Miri stops at the allocation with `resource exhaustion` instead of returning null. On the machine used to check this module, a native debug build of that program died with a segmentation fault in `push`. The release build ran to the end without a message: the optimizer may remove an allocation whose contents are never observed, and an allocation that was removed cannot fail. You find this hole by reading: the SAFETY comment for the `write` in `push` needs "`buf` is not null", and nothing establishes it. Fix: `NonNull::new(raw)` and `handle_alloc_error(layout)` for null. `handle_alloc_error` aborts the process, which is why no test can check the fixed behaviour from inside.

**The bug, `Drop`.** It frees the block and never drops the values that are still in the ring. That is a leak, not undefined behaviour: `mem::forget` does the same in safe code. It is still wrong, and the supplied test `clear_and_drop_release_every_value_once` shows it. Fix: call `self.clear()` first.

**Smells.** `buf: *mut T` makes `Option<Ring<T>>` one word larger and the ring invariant in `T`; `NonNull<T>` plus `PhantomData<T>` fixes both. `iter` allocates a `Vec` for something that is a simple `impl Iterator<Item = &T>`. `Layout::array(..).unwrap()` hides the meaning of the failure ("capacity overflow"). The comment above the `unsafe impl`s describes the compiler error that the lines silence, not why they are true. And there is no invariant written down, which is the root of all of the above: with "slots `(head + i) % cap` for `i` in `0..len` hold values, all others hold none" in front of you, every one of these mistakes is hard to make.

</details>

## Drills

Run `cargo drill m2` at any time after lesson 04. Three drills look ahead: d09 to lesson 05, d05 to lesson 07 and d07 to lesson 08. You can predict them with what you know from the chapters, and come back to them after those lessons. The compiler output tells you *what* happened; these notes say *why*. Read a note only after you have seen the verdict for that drill.

<details><summary>d01 · Reading a Cell that holds a String (lesson 01)</summary>

`E0599`. `Cell::get` exists only for `T: Copy`. Copying a `String` needs `clone`, which is code that could reenter the cell while a reference into it exists. Take the value out instead: `replace` (or `take`) moves the old value out and puts another in, in one step.

</details>
<details><summary>d02 · An UnsafeCell in a static (lesson 01, lesson 07)</summary>

`E0277`: `UnsafeCell<u32>` cannot be shared between threads safely. Every thread can reach a `static`, so its type must be `Sync`. `unsafe impl Sync for Slot` would make the program compile and leave a data race waiting for the second thread. An `AtomicU32` with `fetch_add` is `Sync` and needs no `unsafe`.

</details>
<details><summary>d03 · An unsafe operation inside an unsafe fn (lesson 02)</summary>

Compiles and runs. Since edition 2024 the body of an `unsafe fn` is not an `unsafe` block, but the missing block is a warning (`unsafe_op_in_unsafe_fn`), not an error, and drills are built with warnings off. This module's package turns the warning into an error, so that every unsafe operation gets its own block and SAFETY comment.

</details>
<details><summary>d04 · Taking a field out of a type with a destructor (lesson 02)</summary>

`E0509`. A value whose type implements `Drop` must be whole when the destructor runs, so a field cannot be moved out of it. In safe code, leave a replacement behind (`mem::take`, `Option::take`). An owning pointer such as `MyBox` has no replacement to leave: it reads the value out with `ptr::read` and switches its own destructor off with `ManuallyDrop`.

</details>
<details><summary>d05 · A Send wrapper around a raw pointer (lesson 07)</summary>

`E0277`: `*mut u32` cannot be sent between threads safely, although the wrapper has `unsafe impl Send`. Since edition 2021 a closure captures only the places it uses, and `ptr.0` is the bare pointer. `let ptr = ptr;` inside the closure uses, and so captures, the whole wrapper. A method on the wrapper has the same effect. Better still for this program: `thread::scope` and a plain `&mut`, with no `unsafe`.

</details>
<details><summary>d06 · Using a vector while it is being drained (lesson 04)</summary>

`E0499`. The `Drain` holds the `&mut Vec` until it is dropped. Between `drain` and that drop the vector's length is shortened and the tail is not yet moved back; the borrow is what keeps everyone else out of that state. Finish with the drain, then push.

</details>
<details><summary>d07 · A compile-time check inside a generic function (lesson 08)</summary>

`E0080`: evaluation panicked. A `const { .. }` block in a generic function is evaluated once for each concrete type, when code is generated. `park::<String>` fails the assertion, so the build fails, with a note that names the instantiation. `cargo check` generates no code and reports nothing. Use this pattern to reject types at compile time when no trait bound can express the condition.

</details>
<details><summary>d08 · A thousand values of size zero (lessons 02 and 03)</summary>

Compiles and runs, and asks the allocator for nothing. A `Vec<()>` only counts; its capacity is `usize::MAX`. A `Box<()>` holds a dangling pointer that is non-null and aligned.

</details>
<details><summary>d09 · Mutating through the only Rc (lesson 05)</summary>

Panics: `get_mut` returns `None`, and `unwrap` panics. One `Rc` is not enough when a `Weak` exists, because the `Weak` could be upgraded while the `&mut String` is alive. Mutate first and create the observer afterwards, or put the value in a `RefCell`.

</details>
<details><summary>d10 · A counter in a const (lesson 01)</summary>

Panics: the assertion compares 0 with 2. A `const` is a value that is pasted in at each use, not a place. Every mention of `HITS` creates a new `Cell`, so `record` changes a temporary and the final `get` reads another one. State that must be shared needs one place: a `static` (with a `Sync` type), or `thread_local!` for a `Cell`.

</details>

## Mastery check: model answers

<details><summary>1 · What `&T` promises, and what UnsafeCell changes</summary>

A `&T` promises that the bytes behind it are not written while the reference is alive, by anyone, and the compiler optimizes on that. Bytes inside an `UnsafeCell` are excluded from the promise: `&UnsafeCell<T>` allows writes through the pointer from `get`. The other rules stay: one `&mut` at a time, no `&` together with a `&mut`, no data race. `Cell::get` needs `Copy` because any other way of producing a `T` from a `&T` runs code of `T` (`clone`) while a reference into the cell exists, and that code could change the cell.

</details>
<details><summary>2 · The allocator's rules, and zero-sized types</summary>

Never request zero bytes; check the result for null and call `handle_alloc_error`; free (and reallocate) with exactly the layout the block has. A box computes `Layout::new::<T>()` both times. A vector computes `Layout::array::<T>(cap)` from its current capacity. For a zero-sized `T` both use `NonNull::dangling()`, never call the allocator, and still run destructors; the vector reports a capacity of `usize::MAX`.

</details>
<details><summary>3 · write, read, assignment, copy</summary>

Fill an empty slot with `write`: an assignment would first drop the garbage in the slot. Take a value out with `read`, and make sure the slot no longer counts as a value, or it is dropped twice. Shift a range by one slot with `copy`, which allows overlap; `copy_nonoverlapping` on overlapping ranges is undefined behaviour. After a shift one slot holds a stale duplicate: overwrite it with `write`, never with an assignment.

</details>
<details><summary>4 · Panic safety, leak safety, drop guards</summary>

Panic safety: at every point where foreign code can run (a destructor, a closure, `clone`), the structure must be in a state that is valid if the function is abandoned there. Leak safety: the structure must be valid even if a guard's destructor never runs, because `mem::forget` is safe. A drop guard is a local value whose destructor performs the step that must happen on both the normal and the unwinding path. Leak amplification: `Drain` shortens the vector when it is created, so a leaked `Drain` leaks the range and the tail, which is more than necessary, in exchange for never exposing a slot whose value was moved out.

</details>
<details><summary>5 · The two counters of Rc</summary>

`strong` counts owners; `weak` counts weak pointers plus one shared weak reference held by all owners together. The value is dropped when `strong` reaches 0; the allocation is freed when `weak` reaches 0. The shared weak reference keeps the allocation alive while the value's destructor runs, even if that destructor drops the last `Weak`. `get_mut` requires that no `Weak` exists, because a `Weak` could be upgraded while the `&mut T` lives. Counts must be checked for overflow, because leaked clones are safe and a wrapped count would free a value that still has owners.

</details>
<details><summary>6 · Reading a Miri report; the two models</summary>

A borrow report has three parts: the access that failed, where the pointer was created (with which permission), and where it lost that permission. The bug is usually at the third place. Stacked Borrows keeps a stack of permitted tags for each byte and removes the tags above the one that is used. Tree Borrows keeps a tree of tags with a state per byte and reacts to accesses by unrelated tags. Tree Borrows accepts more programs in most cases, but neither model contains the other: specimen `e` of lesson 06 is accepted by Stacked Borrows only. Write code that both accept: one raw pointer from the owner, everything else derived from it, the owner untouched meanwhile, and no `Box` or `&mut` to memory that raw pointers also reach, because moving a `Box` or passing a `&mut` claims exclusive access again.

</details>
<details><summary>7 · Bounds for a lock and its guard</summary>

Lock: `Send` when `T: Send` (moving the lock moves the value); `Sync` when `T: Send` (threads take turns with exclusive access; `Rc<u8>` is the counterexample for a missing bound, and `Cell<u8>` shows that `T: Sync` is not needed). Guard: `Sync` when `T: Sync` (a shared guard gives `&T`; `Cell<u8>` is the counterexample for the derived `T: Send`); `Send` when `T: Send` for a lock that any thread may release.

</details>
<details><summary>8 · The arena's three arguments</summary>

`&self -> &mut T` is sound because the cursor only moves forward, so every call returns bytes that no other call returns, and chunks are never moved or freed while the arena is borrowed. `alloc` runs no destructors because values may refer to each other and the arena cannot order their destruction safely; the compiler cannot check references inside an untyped block. `alloc_with_drop` requires `'static` because a `'static` value holds no reference into the arena, so its destructor cannot reach a dead neighbour.

</details>
<details><summary>9 · What Miri shows, and what it does not</summary>

Miri finds undefined behaviour that happens on the paths your tests actually run, with the inputs they use, in the thread interleavings it happens to choose, under a model of the aliasing rules that is still experimental. It does not show that a type is sound: an unsound function that no test calls in the bad way passes. It does not run foreign code, and it is slow. It supports a proof; it does not replace the invariant and the SAFETY comments.

</details>
<details><summary>10 · The capstone again, with a changed requirement</summary>

For "make the arena `Send`": the values stored with `alloc` are never touched again, so they are no obstacle. The values stored with `alloc_with_drop` are dropped wherever the arena is dropped, so that function would need `T: Send + 'static`. Then `unsafe impl Send for Arena` has a proof: the arena owns its chunks, shares nothing, and every destructor it may run belongs to a `Send` type. It still must not be `Sync`: two threads would move one cursor.

</details>
