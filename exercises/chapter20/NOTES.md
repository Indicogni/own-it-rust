# Chapter 20 notes — Writing async code

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · Lazy futures and await

**Predictions I got wrong, and why:**

- 

**Explain:** Trace future creation, first polling, and await sequencing.

> 

**Edge case I added:**

- 

## l02 · Borrowed and owned futures

**Predictions I got wrong, and why:**

- 

**Explain:** Explain the difference between moving a reference and moving ownership of its referent.

> 

**Edge case I added:**

- 

## l03 · Async behavior in traits

**Predictions I got wrong, and why:**

- 

**Explain:** Explain static dispatch for async trait methods and when explicit boxed futures are useful.

> 

**Edge case I added:**

- 

## l04 · Lending async callbacks

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why the callback bound is AsyncFnMut and what happens when the caller awaits each call before the next.

> 

**Edge case I added:**

- 

## l05 · Cancellation by dropping

**Predictions I got wrong, and why:**

- 

**Explain:** Distinguish captured resources from resources created after polling. Explain what cancellation does to local guards.

> 

**Edge case I added:**

- 

## l06 · Polling a race

**Predictions I got wrong, and why:**

- 

**Explain:** Explain how losing futures are dropped, and why this small model does not cover tokio::select! syntax, fairness, task aborts, or timers.

> 

**Edge case I added:**

- 

## l07 · Capstone: asynchronous graph traversal

**Predictions I got wrong, and why:**

- 

**Explain:** Describe the sequential core honestly: what does `async` add here, and what does it not add? Then sketch in words how a version with at most three lookups in flight would differ (the optional LABS.md Lab 1 builds it).

> 

**Edge case I added:**

- 

## r01 · Spaced retrieval: Chapters 13, 17–19 inside Chapter 20

**Predictions I got wrong, and why:**

- 

**Edge case I added:**

- 

## c01 · A teammate's pull request

**My review, written before running the tests:**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**Why is calling an async function different from executing its body?**

> 

**What may a future borrow, and what must it own? What do `Send` and `'static` ask of a task that is spawned on a runtime?**

> 

**How does a trait offer an async method, and what does each form (generic bound, `+ Send`, boxed future) cost?**

> 

**How do AsyncFn, AsyncFnMut, and AsyncFnOnce express lending asynchronous callbacks?**

> 

**What happens to a future's state when it is dropped, and what follows from that for the code you write around an await?**

> 

**What must a hand-written combinator such as a race do on each poll?**

> 

**Is the capstone traversal concurrent? What would a concurrent version need, and what evidence would show that it bounds its work and reports every error?**

> 

**After the optional labs: which work may block an executor thread, and what belongs in spawn_blocking?**

> 

**After the optional labs: what state is lost when a future is dropped in a select! branch?**

> 

**After the optional labs: how do abort, cooperative shutdown signals, and cancellation tokens differ?**

>
