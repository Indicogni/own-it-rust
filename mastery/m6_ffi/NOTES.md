# Mastery M6 notes — FFI across a real language boundary

Your workbook. Write answers in your own words, *before* checking the hints. When `cargo progress` says this module is due for review, answer the mastery questions at the bottom again from memory, then compare with what you wrote here.

## l01 · The C library again

**Predictions I got wrong, and why:**

- 

**Explain:** For each of the three C functions, list every precondition that the C standard gives it. For each one, say whether your wrapper checks it, guarantees it by construction (through a type), or passes it on to its caller.

> 

**Edge case I added:**

- 

## l02 · Building C from Cargo

**Predictions I got wrong, and why:**

- 

**Explain:** Follow one `cargo build` of the lab from start to finish. Say in which order Cargo runs the two build scripts and compiles the two crates, which lines of your build script reach rustc and which reach the build script of `checker`, and what would happen if a second package in the graph also said `links = "fletch"`.

> 

**Edge case I added:**

- 

## l03 · Bindings by hand

**Predictions I got wrong, and why:**

- 

**Explain:** Suppose a new version of kvtab adds a field `size_t removes;` in the middle of `kvtab_stats`, and you forget to update your bindings. Which of the checks in this lesson would notice, which would not, and what would `measure` return if none of them existed?

> 

**What Miri reported, and what I changed:**

- 

**Edge case I added:**

- 

## l04 · A safe wrapper

**Predictions I got wrong, and why:**

- 

**Explain:** Write down the invariant of your `Table` in one sentence. Then go through every method and say whether it relies on the invariant, could break it, or both. Finally explain why `put` takes `&mut self` although the compiler would also accept `&self`.

> 

**What Miri reported, and what I changed:**

- 

**Edge case I added:**

- 

## l05 · Callbacks

**Predictions I got wrong, and why:**

- 

**Explain:** For each of your two trampolines, say what `userdata` points to, who owns that memory, how long it lives, and why no second reference to it can exist while the trampoline runs. Then say what happens, step by step, when the closure panics: in `for_each`, and in an observer.

> 

**What Miri reported, and what I changed:**

- 

**Edge case I added:**

- 

## l06 · Strings and buffers

**Predictions I got wrong, and why:**

- 

**Explain:** For every pointer that crosses the boundary in this lesson, in either direction, state three things: who allocated the memory, who frees it, and until when the other side may use it. Then explain why the slice that `peek` returns cannot outlive the next `put`, and which line of code enforces it.

> 

**What Miri reported, and what I changed:**

- 

**Edge case I added:**

- 

## c01 · A teammate's wrapper around kvtab

**My review, written before running the tests:**

- 

**Edge case I added:**

- 

## l07 · Exporting a Rust library to C

**Predictions I got wrong, and why:**

- 

**Explain:** `m6_tally_top` returns a string that Rust allocated, and the C caller must give it back to `m6_tally_string_free`. Explain what would go wrong if the caller passed it to C's `free` instead, and name two other designs for returning a string to C that avoid a second "free" function.

> 

**What Miri reported, and what I changed:**

- 

**Edge case I added:**

- 

## l08 · Capstone: a Rust library with a C interface, built on a C library

**Predictions I got wrong, and why:**

- 

**Explain:** Write the review that you would want from a colleague. For each exported function: which promises does the header ask of the C caller, which of them can your code check, and what happens when an unchecked one is broken? Then: which single change to the header would break existing C programs without any compiler noticing, and how does `m6_stock_table` limit what you may change inside the library later?

> 

**What Miri reported, and what I changed:**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**What exactly does an `unsafe extern "C"` block promise, and to whom? Name three independent ways to check a hand-written binding.**

> 

**Describe what happens between `cargo build` and the moment rustc compiles your crate, for a package whose build script uses `cc`. What do `links` and `cargo::metadata` do, and why must a build script name its C inputs?**

> 

**Why does a binding translate a C `enum` to integer constants and a C function pointer to an `Option`? How is an opaque struct declared, and what does each part of the declaration prevent?**

> 

**State the invariant of a type that owns a C handle. How do `Drop`, `into_raw` and `from_raw` relate to it? What evidence do you need before you write `unsafe impl Send`, and what more for `Sync`?**

> 

**How does a Rust closure reach a C callback? Compare the callback that runs during one call with the one that C keeps: where the closure lives, who frees it, which bounds it needs. What are the three ways to deal with a panic in it?**

> 

**Which three questions must be answered for every pointer that crosses the boundary? Why is `fn peek(&self) -> Option<&[u8]>` a sound signature for "valid until the next put", and what would make it unsound?**

> 

**What makes a Rust function callable from C, and what does each crate type produce? Who checks that the header matches the Rust code? Which later changes break existing C programs silently, and which are safe?**

> 

**What must a guard at an exported function do, what can still go wrong inside it, and how does the C caller find out what happened?**

> 

**Redo the capstone from its contract in a later session, without looking at your first solution. Then change one requirement and say which parts of the header, the Rust code and the C test program must change with it.**

> 
