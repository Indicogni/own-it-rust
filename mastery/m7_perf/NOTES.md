# Mastery M7 notes — Performance engineering

Your workbook. Write answers in your own words, *before* checking the hints. Performance work is easy to forget and easy to misremember, so write numbers down with the command and the machine that produced them. When you come back to this module for review, answer the mastery questions at the bottom again from memory, then compare with what you wrote here.

## l01 · Benchmark design: what a harness does, and what the optimizer undoes

**Predictions I got wrong, and why:**

- 

**Explain:** Why does a harness time a whole batch of calls with two clock readings instead of timing every call? What does `black_box` promise, what does it not promise, and where does a benchmark need it? Why is the median a better summary of timings than the mean?

> 

**Lab 1 (see LABS.md).** Machine and date:

- My prediction for each group, and what I measured:

- What `black_box` on an input prevents, and what it prevents on a result:

- The benchmark I wrote (`l01_sort`): my predicted ratio, the two times, and the reason for the gap:

**Edge case I added:**

- 

## l02 · Reading a profile: self cost, total cost, and counted work

**Predictions I got wrong, and why:**

- 

**Task B: the frame I will remove first, and why (written after reading the printed profile, before coding):**

> 

**Explain:** What is the difference between the self cost and the total cost of a frame, and which one do you sort by to find code to change? Why must `charge` be cheap, and what would happen to a profile if it were not? Before B, which frame did you expect to be the most expensive, and which one was it?

> 

**Lab 2 (see LABS.md).** Machine and date:

- The step of `baseline::summarize` that I expected to cost most, and its share:

- The three largest self costs in my recording:

- The share of the sort in `baseline::top_k`:

**Edge case I added:**

- 

## l03 · Allocation budgets: capacity, reuse, borrowing, and lazy ownership

**Predictions I got wrong, and why:**

- 

**Explain:** For each task, say where the naive version allocates and which of these removed it: knowing the size in advance, reusing a buffer, borrowing instead of owning, or owning only when needed. When is an allocation not worth removing?

> 

**Edge case I added:**

- 

## l04 · Data layout: size, alignment, niches, and arrays of structs

**Predictions I got wrong, and why:**

- 

**Explain:** Why does the order of fields change the size of a `repr(C)` struct, and why does it not change the size of an ordinary Rust struct? What is a niche, and which of your types have one? When is a struct of arrays better than an array of structs, and when is it worse?

> 

**Lab 4 (see LABS.md).** Machine and date:

- Times for the two layouts, and why the gain is smaller than the factor in bytes:

- The same with 5,000 orders:

**Edge case I added:**

- 

## l05 · Traversal order, cache misses, and bounds checks

**Predictions I got wrong, and why:**

- 

**Explain:** Why does the tiled transpose miss less often than the plain one, although both read and write every element exactly once? What does the `Lru` model leave out that a real cache has? Why is a check before the loop better than a check in every iteration, and why is no check at all (an iterator) better still?

> 

**Lab 5 (see LABS.md).** Machine and date:

- Times for the two traversal orders, next to the miss counts of the model:

- Bounds-check calls I predicted and found in `add_indexed`, `add_resliced`, `add_zipped`:

- Where I would add a length check of my own:

- What I expected the checks in `add_indexed` to cost, and what part 3 showed:

**Edge case I added:**

- 

## l06 · Static and dynamic dispatch, inlining, and code size

**Predictions I got wrong, and why:**

- 

**Explain:** What does a call through `dyn Trait` cost that a generic call does not, and what does a generic function cost that a `dyn` call does not? Why does one virtual call per batch remove most of the first cost? Why is `Each<F>` with a closure that captures nothing zero bytes large?

> 

**Lab 6 (see LABS.md).** Machine and date:

- Copies I predicted and found for `framed_generic`, `framed_dyn`, `framed_shell`:

- The shape I would choose for a function called once per request, and for one in the innermost loop:

**Edge case I added:**

- 

## l07 · Buffering: fewer, larger reads and writes

**Predictions I got wrong, and why:**

- 

**Explain:** Why must `write_table` call `flush` itself although `BufWriter` flushes when it is dropped? In `Batcher`, why can you not use `write_all` to empty the buffer, and what must be true of the buffer after a failed write? Why does `sum_lines` reuse one `String`?

> 

**Edge case I added:**

- 

## l08 · Capstone: tune a slow log summary until it meets its budgets

**The three changes I expect to matter most, in order, written before measuring:**

1. 
2. 
3. 

**Predictions I got wrong, and why:**

- 

**Explain:** Which change gave the largest gain in allocations, which in peak memory, and which in time on your machine? Was the order what you had predicted? Name one thing in the baseline that looks slow and that you left alone, and say why.

> 

**Lab 8 (see LABS.md).** Machine and date:

- Counts before and after (allocations, bytes, peak):

- Times before and after, with their spread:

- (Optional) Times with `lto = "fat"` and `codegen-units = 1`:

**Performance report:**

- What was measured (input, command, machine):
- Each change, in order, with the gain it gave:
- What I tried that did not help:
- What I left alone, and why:

**Edge case I added:**

- 

## c01 · A pull request titled "make it faster"

**My review, written before running the tests:**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**A benchmark of a function that sums 10,000 numbers reports 0.3 ns. Give two possible causes and say how you would confirm each.**

> 

**Why does a harness time a batch of calls with two clock readings, and how does it choose the batch size?**

> 

**A profile shows `render` with a total cost of 70% and a self cost of 3%, and `escape` with a total and self cost of 55%. What do you look at first, and what is the most that a faster `escape` can gain?**

> 

**Name four ways to remove a heap allocation, and one situation for each.**

> 

**A `repr(C)` struct has the fields `u8`, `u64`, `u16`, `u32`, `u8` in this order. What is its size, and what is the smallest size you can reach by reordering?**

> 

**Why is `Option<Box<T>>` as large as `Box<T>`, and `Option<u64>` larger than `u64`?**

> 

**When is a struct of arrays faster than an array of structs, when is it slower, and what does it cost in code?**

> 

**A loop reads a large grid column by column. Why can it be much slower than a loop that reads the same grid row by row?**

> 

**Three ways to add two slices element by element: an index loop, an index loop after taking sub-slices, and `zip`. How do they differ in bounds checks, and how do they differ in meaning?**

> 

**What does a call through `dyn Trait` cost, what does a generic function cost, and how can a trait be designed so that a `dyn` call is cheap per element?**

> 

**A function writes through a `BufWriter` and returns `Ok(())` without flushing. What can go wrong, and why does `Drop` not save you?**

> 

**Why can a buffered writer not use `write_all` to empty its buffer if it must be able to continue after an error?**

> 

**You are asked to make a program faster. Describe the steps you take before you change the first line, and how you know when to stop.**

>
