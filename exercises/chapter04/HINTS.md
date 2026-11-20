# Chapter 4 hints — Ownership and moves

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Each lesson has two hints. Open the first one only after a real attempt, and the second one only if the first was not enough. The rules in each task's doc comment ("do not clone", "use `swap_remove`") are part of the exercise. *Check your explanation* is a model answer to the lesson's EXPLAIN prompt; read it after you have written your own.

## 01 · Ownership scopes

<details><summary>Hint 1</summary>

A `String` owns its text buffer, and a vector owns its elements. The last expression of a block, written without a semicolon, leaves the block and gets a new owner. It is not dropped.

</details>
<details><summary>Hint 2</summary>

For A, create the two Strings inside an inner `{ ... }` block and make the vector the last expression of that block. For B, the whole function body is the parameter's name: returning a `String` moves it and copies no text.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A `Vec<String>` is a tree of owners. The variable owns the vector. The vector owns a heap buffer that holds one `String` header per element. Each header owns its own text buffer. When the variable goes out of scope, Rust drops the vector: it drops the elements one after another, starting at index 0, and then frees the vector's buffer. Local variables are dropped at the end of their block, in the reverse of the order in which they were declared. A move transfers the root of the tree to a new owner and does not touch the heap: only the vector's header (pointer, capacity, length) is copied, and the old variable can no longer be used. So the tree has exactly one owner at every moment.

</details>

## 02 · Moves across assignments and fields

<details><summary>Hint 1</summary>

Ownership moves when you pass a value to a function, return it, assign it, or push it into a collection. None of these copies the text of a `String`.

</details>
<details><summary>Hint 2</summary>

For A, take the pair apart with a pattern, `let (first, second) = pair;`, and build the result from the two new variables. For B, `push` takes ownership of its argument; after the push, return the vector.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Moving one field out of a tuple or struct is a *partial move*. The moved field has no value afterwards. The other fields still belong to the tuple and can be used one by one. The tuple as a whole can no longer be used, moved or passed on, because a part of it is missing (`E0382`, "use of partially moved value"). A pattern such as `let (a, b) = pair;` moves every field into its own variable in one step, so nothing is left half-used. Moving the whole tuple, `let other = pair;`, keeps the fields together under a new owner. The pattern `_` does not bind anything, so it does not move that field.

</details>

## 03 · Moves and control flow

<details><summary>Hint 1</summary>

Look at every place where a variable is used. On each path that leads there, the variable must hold a value. A move takes the value away; an assignment gives the variable a value again.

</details>
<details><summary>Hint 2</summary>

For A, `if`/`else` is an expression, and each branch can produce one of the two Strings. For B, write a small helper with the signature `fn(String, &str) -> String` that appends and returns its String. In the loop, assign the helper's result back to `value`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The compiler checks every path through a function, without running it. A variable may be used only where it holds a value on all paths that reach that point. If one branch of an `if` moves the variable and the other does not, the variable is "maybe moved" after the `if`, and using it is an error. A loop body runs more than once, so a move in the body would find an empty variable in the next round; this is rejected unless the body assigns a new value before the round ends. At run time, hidden booleans called drop flags record which of these variables still own a value, so that each value is dropped exactly once. This is why the logs in P1 and P2 differ: the variable that was moved from is skipped when the function ends.

</details>

## 04 · Moving from indexed collections

<details><summary>Hint 1</summary>

Indexing (`values[index]`) cannot move a `String` out. Use a method of `Vec` that takes the element out and leaves the vector in a valid state.

</details>
<details><summary>Hint 2</summary>

`remove` and `swap_remove` panic when the index is too large. Compare `index` with `values.len()` first, and return `None` when the index is not smaller than the length.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A vector treats every slot below its length as holding a valid value, and it drops all of them when it is dropped. `let x = v[i];` would move a `String` out and leave a slot that the vector still believes to be valid, so the compiler rejects it (`E0507`). For a `Copy` type such as `i32` the same line copies and is fine. The methods that take an element out also repair the vector. `pop` takes the last element and shortens the vector: constant cost. `remove(i)` moves every later element one slot to the left: the order is kept, and the cost grows with the number of later elements. `swap_remove(i)` moves the last element into the gap: constant cost, but the order changes. All three return the element by value, so no clone is needed.

</details>

## 05 · Leaving a replacement behind

<details><summary>Hint 1</summary>

Through a `&mut` you can take a value out only if you put another value in at the same moment. The standard library has functions that do both in one step.

</details>
<details><summary>Hint 2</summary>

For A, `std::mem::replace` stores the value you pass and returns the old one. For B, `Option` has a method that leaves `None` in the place and returns what was there.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The rule is: a place that you reach through `&mut T` holds a valid `T` whenever anyone can look at it. The owner will use the value after your function returns and will drop it later, so you may not leave the place empty. `mem::replace` stores a value that you supply and returns the old one. `mem::take` does the same with the type's default value, for example an empty `String` that has no heap buffer. `Option::take` leaves `None`. In each case the caller finds a valid value in the place afterwards (the replacement, the default, or `None`), and the old value moves out with its heap buffer intact: nothing is copied and nothing is allocated.

</details>

## 06 · Copy and explicit cloning

<details><summary>Hint 1</summary>

`[u8; 4]` is a `Copy` type. Assigning it to a new variable copies it, and the old variable stays usable. A `String` is not `Copy`; a duplicate needs an explicit `.clone()`.

</details>
<details><summary>Hint 2</summary>

For A, assign the array to a `let mut` variable and change element 0 of that variable. For B, call `.clone()` on the input first, then call `push_str` on the clone.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A `Copy` type is made only of bytes that mean the same thing wherever they are stored: numbers, booleans, characters, shared references, and tuples or arrays of these. Assigning such a value copies the bytes, and both copies are complete and independent. A type must ask for `Copy` (with `#[derive(Clone, Copy)]`), and every field must be `Copy`, because `Copy` is a promise that users of the type rely on. A `String` cannot be `Copy`: a second copy of its header would point at the same heap buffer, and both would free it. `clone` is always written out and can be expensive. Cloning a `String` allocates a new buffer and copies the text into it, so the clone and the original own separate buffers, and a change to one does not show in the other.

</details>

## 07 · Rc and Arc

<details><summary>Hint 1</summary>

`Rc::clone` and `Arc::clone` make another pointer to the same allocation. `strong_count` reports how many such pointers exist at this moment.

</details>
<details><summary>Hint 2</summary>

For A, call `Rc::new` once and `Rc::clone` once. For B, clone the `Arc` into a new variable before `thread::spawn`, and write `move` in front of the closure so that the closure owns the clone. Build the tuple of three values inside the closure. `join()` returns a `Result`; take the tuple out of it with `expect`. Do not move `text` itself into the closure: the calling thread must still own it while the worker runs.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

An `Rc<T>` points at a heap block that holds the value and a strong count. `Rc::clone` adds one to the count and returns another pointer to the same block; the value is not copied. Dropping an `Rc` subtracts one, and when the count reaches zero the value is dropped. `Arc` works in the same way but changes its count with atomic operations. These are safe when several threads do it at once, and they cost a little more on every clone and drop; this is why `Rc` exists for single-threaded code and why the compiler refuses to send an `Rc` to another thread. Both types give read-only access, because other owners may be reading. If two values own each other through `Rc`, each keeps the other's count at one or more, so neither count reaches zero and both are never freed. A `Weak` pointer does not count as an owner and breaks such a cycle (Chapter 9).

</details>

## 08 · Capstone: owned dispatch queue

<details><summary>Hint 1</summary>

Treat the two kinds of data differently. Every job changes its own label, so every job owns its label. No job changes the configuration, so all jobs can share one.

</details>
<details><summary>Hint 2</summary>

In `dispatch`, move `config` into one `Rc`. Then go through the input vector by value (a `for` loop over `labels`, or `into_iter`), and for each label build a tuple from the label and a clone of the `Rc`. In `finish`, take the tuple apart with a pattern, bind the label as `mut`, and append to it with `push` and `push_str`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

After `dispatch` there are these allocations: the new vector's buffer, one text buffer per label, one `Rc` block, and the configuration's text buffer. Each job tuple owns its label alone; the label was moved out of the input vector, so its text buffer is the one the caller created. Each job also holds one strong pointer to the single `Rc` block, so the strong count equals the number of jobs. Sharing the configuration is safe because nobody changes it. The labels are changed, each one separately, and a shared value cannot be changed through `Rc`, so each label needs exactly one owner. `finish` consumes a job: it appends to the label it received and returns it, and the job's `Rc` is dropped at the end of `finish`, which lowers the count by one.

</details>

## R01 · Spaced review of Chapter 3

<details><summary>Hint 1</summary>

For A, borrow the vector to add up its elements, and return the same vector afterwards: the borrow has ended before the move. For B, a string is walked with `chars()`, never by byte index. For C, the position where the tail starts is the length minus `count`, and that subtraction must not go below zero.

</details>
<details><summary>Hint 2</summary>

For A, `try_fold` with `checked_add` gives an `Option<u16>`; convert each `u8` with `u16::from`. For B, `chars().take(n).collect()` builds the prefix. For C, `len().saturating_sub(count)` gives the split position, and `Vec::split_off` moves the tail into a new vector without cloning.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs. One function measures the wrong unit of length. One leaves a separator at the end. Apart from the bugs, every `.clone()` in this file is unnecessary.

</details>
<details><summary>Hint 2</summary>

`longest` can return `Option<&str>`, borrowed from a `&[String]` parameter; compare `chars().count()`, and take care that the earliest name wins a tie. `[String]::join` puts the separator only between items and allocates the result once. A stable sort with the key "is not urgent" (`sort_by_key`) reorders the jobs by moving the Strings.

</details>
<details><summary>Check your review</summary>

`longest` compares byte lengths, not numbers of `char`s, so "ééé" (6 bytes, 3 `char`s) beats "abcd". It also clones the whole vector and then clones each new best name; it can return a borrow and allocate nothing. `join_labels` leaves a trailing ", " and rebuilds the whole string for every label, so the amount of copying grows with the square of the input length. `urgent_first` clones the vector and then clones every job a second time, although the jobs only need to change places. Clippy reports the `&Vec<String>` parameter of `longest` from the start. It does not report `urgent_first`'s `&mut Vec<String>` while the body calls `clear` and `push`; once you reorder in place, it reports that parameter too. Take a slice (`&[String]`, `&mut [String]`) whenever the function does not change the length of the vector.

</details>

## Drills

Run `cargo drill ch04` after lesson 07. The compiler output tells you *what* happened; these notes say *why*. Read a note only after you have seen the verdict for that drill. Each title names the lesson that the drill belongs to.

<details><summary>d01 · Use after move (lesson 02)</summary>

`E0382`. `let saved = route;` moves the `String`, so `route` has no value afterwards. The KEEP line reads both names. Repair it with `route.clone()` (two text buffers) or with `let saved = &route;` (one buffer, and `saved` only borrows it).

</details>
<details><summary>d02 · Moving out of an index (lesson 04)</summary>

`E0507`. Indexing names a slot inside the vector, and moving the `String` out would leave that slot empty. In this drill the KEEP lines need both names to stay in the vector, so borrow the element (`&names[0]`) or clone it. In other programs, where the vector may get shorter, `remove` or `swap_remove` takes the element out; here they would make the `names.len()` check fail.

</details>
<details><summary>d03 · Reassigning a moved binding (lesson 03)</summary>

Compiles and runs. A variable whose value was moved away is empty, not gone. Assigning a new value makes it usable again. The variable must be declared with `mut` for this.

</details>
<details><summary>d04 · Moves inside a loop (lesson 03)</summary>

`E0382`. The first round moves `greeting` into the vector, and the second round would move an empty variable. Three elements need three Strings: clone inside the loop, or create each String inside the loop.

</details>
<details><summary>d05 · Partial moves (lesson 02)</summary>

Compiles and runs. Only `pair.0` was moved; `pair.1` still belongs to the tuple and can be read. Using `pair` as a whole after the move would be `E0382`.

</details>
<details><summary>d06 · Copy is opt-in and all-or-nothing (lesson 06)</summary>

`E0204`. A type can be `Copy` only if every field is `Copy`, and `String` is not. Remove `Copy` from the derive and keep `Clone`; the KEEP lines only need `clone`.

</details>
<details><summary>d07 · Shared ownership is shared, not mutable (lesson 07)</summary>

`E0596`. An `Rc` hands out read-only access, because other owners may be reading. One repair is `Rc::make_mut(&mut config)`, with `config` declared `mut`. When the value is shared, `make_mut` first gives `config` its own copy of the text and then lets you change that copy, so `other` still sees the old text. Chapter 9 shows another tool, `RefCell`, for changing a shared value in place.

</details>
<details><summary>d08 · Removing past the end (lesson 04)</summary>

Panics at run time. The value of an index is known only when the program runs, so the compiler cannot reject it. The vector has one element, so use index 0. In real code, compare the index with `queue.len()` first.

</details>
<details><summary>d09 · Rc across threads (lesson 07)</summary>

`E0277`. `Rc<Vec<i32>>` is not `Send`: its count is a plain number, and two threads that change it at the same time could corrupt it. Replace `Rc` with `std::sync::Arc`, which counts with atomic operations.

</details>

## Mastery check: model answers

Answer the questions in the [chapter guide](README.md#mastery-check) from memory first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**1. Stack and heap before and after a move, a clone, and a drop.**

```text
let a = String::from("tide");

  a: pointer ──► [ t | i | d | e ]        one header, one buffer
     capacity
     length 4

let b = a;                                 MOVE

  a: (no value)
  b: pointer ──► [ t | i | d | e ]        the same buffer; only the header was copied

let c = b.clone();                         CLONE

  b: pointer ──► [ t | i | d | e ]
  c: pointer ──► [ t | i | d | e ]        a new buffer with a copy of the bytes

drop(b);                                   DROP

  b: (no value)                            b's buffer is freed
  c: pointer ──► [ t | i | d | e ]        c is not affected
```

**2. Which operations leave the source without a value, and why can it get a new one?**
For a type that is not `Copy`: assigning the value to another variable, passing it to a function by value, returning it, and putting it into another value (a `push`, a tuple, an array). After each of these the compiler treats the source variable as empty and rejects any read of it. A variable is a place, not the value itself. The compiler tracks whether the place holds a value at each point, and an assignment fills it again. The variable must be declared with `mut` for that second assignment.

**3. Taking an owned value out without cloning and without leaving a hole.**
From a vector: `pop` (last element), `remove(i)` (keeps the order, moves the later elements), `swap_remove(i)` (constant cost, the last element fills the gap), or a `for` loop over the vector itself, which takes every element. From a place behind `&mut`: `mem::replace(place, new)`, `mem::take(place)` (leaves the default value) or, for an `Option`, `take()` (leaves `None`). Each of these puts something valid in the place before it gives you the old value.

**4. Why no `Copy` for a type with a `String` field, and `Rc::clone` versus cloning the value.**
`Copy` means that a bit-for-bit duplicate is a complete, independent value. A duplicate of a `String` header would be a second owner of the same heap buffer, and both would free it. So `String` is not `Copy`, and a type is allowed to be `Copy` only if all of its fields are (`E0204` otherwise). `Rc::clone(&a)` adds one to the strong count and returns another pointer to the same allocation: no allocation, no copy of the value. `(*a).clone()` clones the value inside: a new, separate `String` with its own buffer, and the count does not change.

**5. `Rc` or `Arc`, read-only access, and cycles.**
Use `Rc` when all owners live in one thread; its count is a plain number and cheap to change. Use `Arc` when owners live in different threads; its count is changed with atomic operations, which is safe between threads and a little slower. Neither lets you change the value, because the other owners may be reading it at the same time; changing a shared value needs an extra tool (`RefCell` in Chapter 9, `Mutex` in Chapter 19). If two values own each other through `Rc`, each holds the other's count at one or more. Neither count can reach zero, so neither value is dropped, even when nothing else in the program can reach them.

**6. A changed requirement for the capstone.**
One example: "jobs are finished on worker threads". Then the shared configuration must be an `Arc<String>`, because an `Rc` may not cross into another thread. The labels do not change: each job still owns its label, and an owned `String` can be moved to another thread as it is. The cost is an atomic operation for each clone and drop of the pointer. Another example: "the configuration can change while jobs are waiting". Then either every job gets its own clone (a snapshot that later changes do not reach), or the jobs share one value that can be changed, which needs the tools of Chapter 9 or Chapter 19.

</details>
