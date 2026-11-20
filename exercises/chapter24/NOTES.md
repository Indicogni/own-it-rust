# Chapter 24 notes — Calling C

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · C representations

**Predictions I got wrong, and why:**

- 

**Explain:** Explain primitive aliases, field layout, padding, and why a raw struct byte dump is not a portable wire format.

> 

**Edge case I added:**

- 

## l02 · C strings and ownership

**Predictions I got wrong, and why:**

- 

**Explain:** Distinguish owned CString, borrowed CStr, raw pointer lifetime, and UTF-8 validation.

> 

**Edge case I added:**

- 

## l03 · ABI calls and out-parameters

**Predictions I got wrong, and why:**

- 

**Explain:** Explain success-dependent initialization and why a function-pointer ABI must match exactly.

> 

**Edge case I added:**

- 

## l04 · Callbacks and userdata

**Predictions I got wrong, and why:**

- 

**Explain:** Explain callback lifetime, pointer retention, and why this provided callback uses nonpanicking saturation. The extension lab handles arbitrary closure panics.

> 

**Edge case I added:**

- 

## l05 · Opaque handle ownership

**Predictions I got wrong, and why:**

- 

**Explain:** Specify the ownership transfer at construction and exactly-once destructor call. Explain why the mock uses Box consistently and a real library needs its own free function.

> 

**Edge case I added:**

- 

## l06 · Owner-bound foreign borrows

**Predictions I got wrong, and why:**

- 

**Explain:** Explain the lifetime of the returned borrowed object and the difference between it and an owned copy. Give a real example of a child object that must not outlive its parent.

> 

**Edge case I added:**

- 

## l08 · Calling the real C library

**Predictions I got wrong, and why:**

- 

**Explain:** Explain what an `unsafe extern "C"` block promises and who has to keep that promise. For each wrapper, say which precondition of the C function it checks or guarantees before the call.

> 

**Edge case I added:**

- 

## l07 · Capstone: checked foreign transaction

**Predictions I got wrong, and why:**

- 

**Explain:** Audit arguments, status, initialized output, owned error copying, and no unwinding across the ABI. Explain which obligations of a real library are not modelled here (see LABS.md).

> 

**Edge case I added:**

- 

## r01 · Spaced retrieval: Chapters 18, 22, and 23 inside Chapter 24

**Predictions I got wrong, and why:**

- 

**Edge case I added:**

- 

## c01 · A teammate's FFI wrapper pull request

**My review, written before running the tests:**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**How do `#[repr(C)]`, the C type aliases, `usize` for `size_t`, pointers and ABI declarations make two languages agree on the same bytes?**

> 

**Why do `CString` and `CStr` exist as two types, and what is the lifetime of the pointer that `as_ptr` returns?**

> 

**When is a foreign output parameter initialized, and what may you read after a failure?**

> 

**Which allocator frees each foreign object, and how does a wrapper make sure that cleanup happens exactly once, also on an early return?**

> 

**How does a borrowed foreign object keep the lifetime of its owner?**

> 

**How long must callback userdata live, and what happens when a panic reaches the edge of an `extern "C"` function?**

> 

**Who checks an `unsafe extern` block, and what does each of the words `unsafe extern`, `safe fn` and `unsafe { ... }` promise?**

> 

**Which facts about a C library must a safe wrapper find out and write down (initialization and shutdown, threads, error messages, versions, linking)?**

>
