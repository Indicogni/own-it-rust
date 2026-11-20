# Mastery M3 notes — Concurrency by building the primitives

Your workbook. Write answers in your own words, *before* checking the hints. When you come back to this module for review, answer the mastery questions at the bottom again from memory, then compare with what you wrote here.

In this module, also keep the invariant of each primitive here, in one or two sentences, and the interleaving behind each model failure you had, as a numbered list of steps. Those two things are what you will want to find again.

## l01 · Memory orderings and happens-before

**Predictions I got wrong, and why:**

- 

**The invariant, in my words:**

> 

**Explain:** Define happens-before. Then say, for each task, which pair of operations must be ordered by it, and which ordering arguments create that order.

> 

**Model failures I had, as interleavings:**

- 

**Edge case I added:**

- 

## l02 · A spin lock with a guard

**Predictions I got wrong, and why:**

- 

**The invariant, in my words:**

> 

**Explain:** State the invariant that connects `locked` and `value`. Use it to justify the two `unsafe` blocks in the guard, the orderings in `lock` and in the guard's `Drop`, and the bounds on `Sync` for the lock and for the guard.

> 

**Model failures I had, as interleavings:**

- 

**Edge case I added:**

- 

## l03 · A one-shot channel

**Predictions I got wrong, and why:**

- 

**The invariant, in my words:**

> 

**Explain:** For each value of `state`, say whether `message` is initialised and who may access it. Then explain why `recv` cannot miss a wake-up, and why the receiver is not `Send`.

> 

**Model failures I had, as interleavings:**

- 

**Edge case I added:**

- 

## l04 · `MyArc` and `MyWeak`

**Predictions I got wrong, and why:**

- 

**The invariant, in my words:**

> 

**Explain:** Write down the two counters and what each one counts. State when the value is dropped and when the allocation is freed. Then justify the ordering of every atomic operation, and the bounds on your `Send` and `Sync` impls.

> 

**Model failures I had, as interleavings:**

- 

**Edge case I added:**

- 

## l05 · A mutex that puts waiting threads to sleep

**Predictions I got wrong, and why:**

- 

**The invariant, in my words:**

> 

**Explain:** Describe your waiting protocol as two lists of steps, one for a thread that must wait and one for a thread that unlocks. Then show, for every way these steps can interleave, that a waiting thread is woken. Say which step makes spurious wake-ups harmless.

> 

**Model failures I had, as interleavings:**

- 

**Edge case I added:**

- 

## l06 · A bounded channel with `Mutex` and `Condvar`

**Predictions I got wrong, and why:**

- 

**The invariant, in my words:**

> 

**Explain:** Write the predicate of each of your waits (the condition under which the thread must keep sleeping). For each predicate, list every place in your code that can make it false, and show that each such place notifies the right condition variable.

> 

**Model failures I had, as interleavings:**

- 

**Edge case I added:**

- 

## l07 · A lock-free stack, and the ABA problem

**Predictions I got wrong, and why:**

- 

**The invariant, in my words:**

> 

**Explain:** State the rule that decides when your stack frees a node, and use it twice: once to show that `pop` never reads freed memory, and once to show that the compare-exchange in `pop` cannot succeed by mistake (no ABA). Then say what your rule costs.

> 

**Model failures I had, as interleavings:**

- 

**Edge case I added:**

- 

## l08 · Capstone: a wait-free ring buffer for one producer and one consumer

**Predictions I got wrong, and why:**

- 

**The invariant, in my words:**

> 

**Explain:** Write your invariants: which slots are initialised, expressed with the two indices, and who may touch which slot. Give the ordering of each of the four index operations, and for each `Acquire` name the `Release` it pairs with and the two accesses that this orders.

> 

**Model failures I had, as interleavings:**

- 

**Edge case I added:**

- 

## c01 · A teammate's lock-free slot pool

**My review, written before running the tests (for each problem: the interleaving or the input that shows it):**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**Define "data race" and "happens-before". Name four ways in which two operations in different threads become ordered.**

> 

**A thread stores a flag with `Relaxed` after writing a buffer; another thread loads the flag with `Acquire` and then reads the buffer. Is this correct? What exactly can go wrong, and which single change repairs it?**

> 

**Give a program for which `Release`/`Acquire` is not enough and `SeqCst` is. Say why.**

> 

**Why is `SpinLock<T>: Sync` for `T: Send`, but its guard `Sync` only for `T: Sync`?**

> 

**Why must the waker change the state before it calls `unpark`, and why must the waiter call `park` in a loop?**

> 

**Why does `Arc<T>` need `T: Send + Sync` to be `Send`? Give one type that shows the need for each bound.**

> 

**Which ordering does `Arc`'s decrement need, and what is the `Acquire` fence for? Why is `Relaxed` enough for the increment?**

> 

**In a mutex that parks its waiters: in which order must a waiting thread "register" and "try again", and in which order must the unlocking thread "free" and "look for waiters"? What happens if either pair is swapped?**

> 

**State the three-part rule for condition variables. Give a reason for the loop that has nothing to do with spurious wake-ups.**

> 

**Describe the ABA problem in a lock-free stack with a concrete interleaving. Name two ways to prevent it, and say which of them also prevents a use after free.**

> 

**In a single-producer, single-consumer ring buffer, which two accesses does the `Release` store of `head` order, and why can each side load its own index with `Relaxed`?**

> 

**What can loom check that Miri cannot, and what can Miri check that loom cannot? Name one thing that loom 0.7.3 gets wrong.**

>
