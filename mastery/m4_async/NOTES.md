# Mastery M4 notes — An async runtime from scratch, then a real service

Your workbook. Write answers in your own words, *before* checking the hints. A week after you finish the module, and again a month later, answer the mastery questions at the bottom from memory, then compare with what you wrote here.

## l01 · RawWaker and its vtable, by hand

**Predictions I got wrong, and why:**

- 

**Explain:** Write the contract of the four vtable functions as a table: who owns the reference before the call, and who owns it after. Then say why the vtable in this lesson is a `static` and not a `const`.

> 

**Edge case I added:**

- 

**What Miri said (optional lab), and what I changed:**

- 

## l02 · Tasks, a run queue, spawn and JoinHandle

**Predictions I got wrong, and why:**

- 

**Explain:** Draw who owns whom: executor, spawner, shared state, run queue, task, waker, join handle. Mark each arrow strong or weak, and show why no cycle of strong arrows can survive `Executor::drop`.

> 

**Edge case I added:**

- 

## l03 · A timer heap and sleep

**Predictions I got wrong, and why:**

- 

**Explain:** State the lock rule of this lesson in one sentence, then give one concrete program that deadlocks when the driver breaks it. Say why `Sleep` reads the clock in `poll` although the driver already compared the times.

> 

**Edge case I added:**

- 

## l04 · A reactor over non-blocking sockets

**Predictions I got wrong, and why:**

- 

**Explain:** Describe one round trip of `stream.read(&mut buf).await` when no data is there yet: every call, every thread, and where each waker clone lives. Then say what a reactor built on epoll, kqueue or IOCP does differently in the step "decide whom to wake", and what that saves.

> 

**Edge case I added:**

- 

## l05 · Pin projection by hand

**Predictions I got wrong, and why:**

- 

**Explain:** For `Fuse<F>`, write the complete safety argument: which fields are structurally pinned, and why each of the four duties holds. Then name one line that would break each duty.

> 

**Edge case I added:**

- 

**What Miri said (optional lab), and what I changed:**

- 

## l06 · A Stream trait, a channel, and adapters

**Predictions I got wrong, and why:**

- 

**Explain:** List every way in which the receiving task of your channel can be woken, and for each one say why the wake cannot be lost. Then explain why `merge` must remember that one side has ended.

> 

**Edge case I added:**

- 

**What Miri said (optional lab), and what I changed:**

- 

## l07 · Select and cancellation safety

**Predictions I got wrong, and why:**

- 

**Explain:** Define "cancellation safe" for a method such as `next_line` in two sentences. Then classify each of these and say why: your `next_line`, `Receiver::next` from lesson 06, `AsyncStream::read` and `AsyncStream::write_all` from lesson 04.

> 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## c01 · A teammate's runtime helpers

**My review, written before running the tests:**

- 

**The SAFETY comments I could not write, and what that told me:**

- 

**Edge case I added:**

- 

## l08 · Capstone: a chat server on your own runtime

**Before coding.** Which tasks exist while three clients are connected? What does each task own? Where can a task wait, and what wakes it there? What must `Drop` clean up when a task is cancelled?

> 

**My select for the main loop of a connection, and for each branch: is it cancellation safe, and why?**

> 

**Explain:** Pick one client that stops reading while others keep talking. Follow one message to that client through every queue and buffer between the sender's socket and the slow client's socket. Say where it waits, what bounds each queue, and what your server does when a bound is hit.

> 

**Experiment: closing a socket that still has unread input.** What did the client see, on which system?

> 

**Edge case I added:**

- 

## l09 · The same service on tokio

**Predictions I got wrong, and why:**

- 

**Explain:** Describe graceful shutdown as a sequence: what stops first, what each connection still does, what bounds the wait, and what happens to a connection that does not finish in time. Then say which of your `.await` points a `select!` may cancel. For each one: is it safe to cancel, and if it is not, what is lost and why is that right at this point?

> 

**Where can one of my connections wait for room when the shutdown begins? What ends that wait, and what happens to the line that waited?**

> 

**What changed between my lesson 08 server and this one, and why:**

> 

**Edge case I added:**

- 

## Mastery check (answer from memory)

**A `Waker` is cloned on thread A and woken by value on thread B. List what each of the four vtable functions must do to the reference count, and say what goes wrong if `wake` forgets its part and if `wake_by_ref` does too much.**

> 

**A task returns `Pending` and is never polled again. Name five different places in a runtime where the wakeup can have been lost, one per lesson from 02 to 06, and the rule that prevents each.**

> 

**Why does `Spawner::spawn` demand `Send + 'static`, although your executor polls every task on one thread?**

> 

**State the lock rule of this module, and give one deadlock that breaking it causes.**

> 

**What exactly does `Pin<&mut T>` promise, and which four duties do you take on when you project it to a field? Which of them does `Option::take` on a pinned field break?**

> 

**Your reactor wakes every waiting task once per millisecond. What does a reactor on epoll or kqueue do instead, and what stays the same for the code that calls `poll_io`?**

> 

**Define cancellation safety. Then say why `next_line` of lesson 07 has it and `write_all` of lesson 04 does not, and what the capstone does about the second one.**

> 

**A client stops reading. Describe what happens in lesson 08 and in lesson 09, and give one reason to prefer each policy.**

> 

**In which order must a runtime drop its executor, its timer driver and its reactor, and why?**

> 

**Lesson 09 tests most behaviour with in-memory pipes and a paused clock. What can such a test prove that a test over real sockets cannot, and what can it not prove?**

> 
