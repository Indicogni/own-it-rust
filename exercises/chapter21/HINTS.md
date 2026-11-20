# Chapter 21 hints — Futures by hand

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Each lesson has two hints. Open the first one only after a real attempt, and the second one only if the first was not enough. *Check your explanation* is a model answer to the lesson's EXPLAIN prompt; read it after you have written your own.

## l01 · Implement Future

<details><summary>Hint 1</summary>

Pin restricts moving the pointee; the smart-pointer handle itself can move.

</details>

<details><summary>Hint 2</summary>

get_mut is available because Once<T> is Unpin when T is Unpin. Take from Option rather than cloning the output.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`Output` is the type the future produces when it completes; `poll` returns `Poll<Output>`, either `Ready(value)` once or `Pending`. `poll` takes `self: Pin<&mut Self>` so the caller promises the future will not move again once polling has started, which is what lets compiler-generated futures hold references into their own state. `Once<T>` holds no such references, so it is `Unpin` whenever `T` is; that unlocks `Pin::get_mut`, giving an ordinary `&mut Self` and letting `Option::take` move the value out without cloning. A pinned `Box` keeps its contents at one heap address, so moving the `Pin<Box<_>>` handle is fine. Polling after `Ready` is a contract violation, and panicking with a clear message is the conventional response.

</details>

## l02 · Pending and wakeups

<details><summary>Hint 1</summary>

A waker is a scheduling notification; result storage belongs elsewhere.

</details>

<details><summary>Hint 2</summary>

Use cx.waker().wake_by_ref on the pending path only. Atomic fetch_add returns an old value that you can discard.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

A `Waker` only says "poll this task again"; a wake carries no result, and waking does not run `poll` itself, it asks the executor to schedule a poll later. Executors may *coalesce* wakes: several wakes before the next poll lead to one poll, so a wake is not a counter of events and the future must re-check its own state when polled. A future that returns `Pending` must have arranged a wake (registered the waker somewhere, or woken itself), or it will never be polled again. Self-waking is right only when the poll made bounded progress, as `Countdown` does; self-waking without progress turns the task into a busy loop that burns CPU and starves others.

</details>

## l03 · A completion cell

<details><summary>Hint 1</summary>

The state and waiter registration must be inspected atomically under one mutex. Never wait on that mutex for future completion.

</details>

<details><summary>Hint 2</summary>

Store a clone of the latest waker. Move the waker out while locked, then invoke it only after the lock guard is dropped.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

Completion first: `finish` locks, stores the value, finds no waiter, unlocks; the first `poll` locks, finds the value, and returns `Ready` without registering anything. Poll first: `poll` locks, finds no value, stores a clone of the current waker, unlocks, and returns `Pending`; later `finish` locks, stores the value, takes the waker, unlocks, and wakes, and the next poll finds the value. Because the "is there a value?" check and the waker registration happen under the same lock, `finish` cannot slip in between them, so no wake is lost. Each poll replaces the stored waker, since the executor may have moved the task. Waking after unlocking means a task woken on another thread does not immediately block on the lock you still hold.

</details>

## l04 · A parker block_on

<details><summary>Hint 1</summary>

thread::unpark stores a token if the thread has not parked yet. Always poll again after a park return.

</details>

<details><summary>Hint 2</summary>

Use pin! on the stack. A poll_fn closure can capture a yielded bool and an Option<T> that is taken only on the second poll.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`thread::unpark` sets a token for the thread; `park` consumes a token if one is present and returns immediately, otherwise it blocks. So a wake that arrives after the poll returned `Pending` but before `park` is not lost. The token is a flag, not a counter: two `unpark` calls before a `park` let only one `park` through, which is all an executor needs, because one more poll looks at everything that has changed. `park` may also return spuriously with no wake at all, which is why `block_on` simply polls again in a loop rather than assuming the future is ready. A `block_on` that passes `Waker::noop()` parks and is never unparked; one that spins or sleeps for a fixed time gives the right answer but polls when nothing has happened, and the lesson's test counts those polls. This executor runs exactly one future on the calling thread; a real runtime adds a task queue for spawned tasks, an I/O reactor, timers, work stealing across threads, cooperative budgeting, and cancellation.

</details>

## l05 · Readiness registration model

<details><summary>Hint 1</summary>

The ready flag stores the event, not the waker. This toy event has one waiter and is never reset.

</details>

<details><summary>Hint 2</summary>

Never leave a window between checking ready and installing the waker. Actual I/O also retries the operation and handles WouldBlock.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

Readiness means "an operation is now likely to make progress", not "your data has arrived": a readable socket may hold half a message, and readiness can be stale by the time you try, so the future always performs the real I/O and handles `WouldBlock` by re-registering. EOF is a successful read of 0 bytes, not an error. The ready flag is the stored event, so a signal that arrives before anyone polls is remembered. Checking the flag and installing the waker must happen under the same lock that the signaler uses to set the flag; otherwise the signal could land after the check but before registration, and the wake would go to nobody.

</details>

## l06 · Cooperative work budgets

<details><summary>Hint 1</summary>

A Future does not automatically make CPU work cooperative. Count elements processed in one poll.

</details>

<details><summary>Hint 2</summary>

Compute an end index bounded by both budget and slice length, and widen each element to u64 before adding. Wake only when work remains after consuming the budget.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

A future that runs a long loop inside one `poll` monopolizes its executor thread: runtimes cannot preempt code in the middle of `poll`, so other tasks on that thread wait. `BudgetSum` bounds the work per poll to `budget` elements and, if more remain, returns `Pending`. Here the self-wake is appropriate because this poll *did* make progress and the future is immediately ready to make more; it is asking to go to the back of the queue, not waiting for an external event. Poll counts (`ceil(len / budget)`, at least one) show the structure, not speed; timing depends on the executor and the machine.

</details>

## l07 · Capstone: pinned future adapter

<details><summary>Hint 1</summary>

Pin<Box<F>> is itself a movable handle. Moving the wrapper does not move F inside its heap allocation.

</details>

<details><summary>Hint 2</summary>

get_mut on the wrapper is safe because its fields can move; then call inner.as_mut().poll(cx). Do not take the tag until completion.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`WithTag` stores the inner future as `Pin<Box<F>>`: the future lives at a fixed heap address that never changes, however often the wrapper moves. `Pin<Box<F>>` is `Unpin` whatever `F` is, and `Option<String>` is `Unpin`, so the whole wrapper is `Unpin` and `Pin::get_mut` gives safe `&mut` access to its fields; `inner.as_mut()` then re-creates the `Pin<&mut F>` needed to poll the inner future in place. No `unsafe` projection is needed because the only pinned thing is behind its own pinned pointer. Construction (`tagged`) moves `F` into the box before any poll, which is fine because a future only starts holding self-references after its first poll; from then on it never moves. The tag is taken only on `Ready`, so it survives any number of `Pending` polls.

</details>

## R01 · Spaced review of Chapters 16, 19, and 20

<details><summary>Hint 1</summary>

A: one `Option<T>` per future records which ones are done; each poll skips the finished ones. B: share `Arc<Mutex<Shared<T>>>` between the future and the worker, like l03's completion cell, with a thread as the producer. C: a `VecDeque` keeps arrival order; search it by id before pushing.

</details>
<details><summary>Hint 2</summary>

A: after polling, if every slot is `Some`, take them all into a `Vec`. B: the worker computes, then in a block stores the result and `take()`s the waiter, and wakes after the block (one test wakes a probe that tries the lock, so a wake inside the block fails); `poll` takes the result or replaces the waiter. C: `iter_mut().find(|(i, _)| *i == id)` and `Waker::clone_from` update in place; `pop_front()?` then `wake()` consumes the waker.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one wakes a stale waker, one returns `Pending` without arranging any wake. Then look at what `Message` does with the String, what `publish` is holding when it wakes, and what `ChunkedSum` allocates on every poll.

</details>
<details><summary>Hint 2</summary>

In `poll`, `take()` the value and always store the current waker. In `publish`, set the value and `take()` the waiter inside a block, then call `wake()` after the guard is gone. In `ChunkedSum`, sum the slice `items[next..end]` directly and call `cx.waker().wake_by_ref()` before returning `Pending`. If the compiler then reports E0502, you are reading `self.items` and writing `self.total` in the same loop through the `Pin`: compute the sum of the chunk into a local variable first, then add it.

</details>
<details><summary>Check your review</summary>

`Message::poll` registers a waker only the first time, so after the executor polls with a new waker the task is never woken (a lost wakeup that looks like a hang in production). It clones the message instead of moving it out, which doubles the allocation and leaves the payload in the shared state. `publish` calls `wake_by_ref` while holding the lock, so a task woken on another thread immediately blocks on that lock, and it leaves the waker registered. `ChunkedSum` returns `Pending` without waking, so a real executor never polls it again; it only "works" under a loop that polls blindly. It also copies the remaining items into a new `Vec` on every poll, which is quadratic work over the whole sum. Clippy reports only the index loop (`needless_range_loop`). Its suggestion, `remaining.iter().take(count)`, is tidier and keeps the copy, so a clean clippy run is not the end of a review. Iterating over `&self.items[self.next..end]` while adding to `self.total` does not compile (E0502), because each field access borrows the whole `Pin`; `let chunk: u64 = self.items[self.next..end].iter().sum();` followed by `self.total += chunk;` avoids both the copy and the conflict.

</details>

## Drills

Run `cargo drill ch21`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · Polling without a pin</summary>E0599: `poll` takes `self: Pin<&mut Self>`, so it is not callable on a plain value or `&mut`. `Ready<T>` is `Unpin`, so `Pin::new(&mut job).poll(&mut cx)` works; for any future, `std::pin::pin!(job)` or `Box::pin(job)` does.</details>
<details><summary>d02 · Pin::new on an async block</summary>E0277: `Pin::new` only accepts `Unpin` targets, and no async block is `Unpin`. The compiler treats every async block as a value that may hold references into itself and does not check whether it really does. (This one does: `view` borrows `text` across the await. But `Pin::new(&mut async { 24 })` is rejected in the same way.) Pin it where it will stay: `let mut job = pin!(async { .. });` or `Box::pin(async { .. })`, then `job.as_mut().poll(&mut cx)`.</details>
<details><summary>d03 · poll with the wrong receiver</summary>E0053: the trait's method takes `self: Pin<&mut Self>`, and an impl must match. Write `fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>)`; because `Countdown` is `Unpin`, `self.0` reads and writes still work through the pin.</details>
<details><summary>d04 · A Future without an Output</summary>E0046: `Future` has an associated type, and every impl must name it. Add `type Output = u32;` and the `Poll<u32>` return type then matches.</details>
<details><summary>d05 · Storing the context's waker</summary>E0308: `cx.waker()` returns `&Waker`, borrowed from the `Context` for this poll only. To keep it, store an owned clone: `Some(cx.waker().clone())` (or `clone_from` into an existing one).</details>
<details><summary>d06 · Moving a pinned box</summary>Compiles and runs. Moving a `Pin<Box<_>>` moves only the pointer; the future stays at the same heap address, which is exactly the guarantee pinning needs.</details>
<details><summary>d07 · Polling Ready twice</summary>Panics at run time: "`Ready` polled after completion". `Ready` hands out its value once. After a future returns `Poll::Ready`, the contract says not to poll it again. What happens if you do is up to the future: `std::future::Ready` and the futures that the compiler builds from async blocks panic, while others may return `Pending` forever or give a wrong value, so never rely on it. Stop polling after `Ready`.</details>
<details><summary>d08 · Waking while the state is borrowed</summary>Panics at run time: "RefCell already mutably borrowed". The "wake" runs the task, which borrows the state that `complete` still holds mutably. It is the single-threaded version of waking under a mutex. Take the callback out inside a block, let the borrow end, then call it.</details>
<details><summary>d09 · A Waker from a plain struct</summary>E0277: `Waker` implements `From<Arc<W>>` for `W: Wake`, not `From<W>`. Wakers are cloned and moved between threads, so they share the task through an `Arc`: `let flag = Arc::new(Flag(..)); let waker = Waker::from(Arc::clone(&flag));`.</details>
<details><summary>d10 · Mutating through Pin<&mut Self></summary>E0596: `self` is a `Pin<&mut Self>` binding, and mutating through it requires the binding itself to be `mut` (it calls `DerefMut` on the pin). Write `mut self: Pin<&mut Self>`; that works here because `Retries` is `Unpin`.</details>

## Mastery check: model answers

Answer the questions in the [chapter guide](README.md#mastery-check) from memory first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**1. What does `Poll::Pending` promise, and why must `poll` not block?**
`Pending` says two things: "I am not finished", and "I have arranged that the waker from this poll's `Context` will be called when I can make progress". The second part is the promise. A future that breaks it is never polled again, and nothing reports the mistake. `poll` must return quickly because the executor thread that calls it has other tasks to run. While one `poll` sleeps in a blocking call or computes for a long time, no other task on that thread runs, no timer fires and no I/O event is handled. Waiting is done by returning `Pending`, never by blocking.

**2. How does a wake lead to another poll without carrying the result?**
A `Waker` is a handle to a task. Calling it tells the executor "this task is worth polling again", and nothing else. It does not run `poll`; the executor does that later, and several wakes before that poll may become a single poll. The result travels another way: the completing side writes it into state that it shares with the future (a field behind a mutex, a queue, a flag), and then wakes. When the future is polled again, it looks at that state and finds the result. This is why a future must check its state on every poll and must not treat "I was woken" as "my data is here".

**3. Where must registration and completion be synchronized?**
The future's check ("has it happened?") and its registration ("here is my waker") must be one atomic step with respect to the completing side's step ("record the event and take the waker"). In this chapter that means both sides do their step while holding the same mutex. If the check and the registration were separate, the event could be recorded between them: the completing side finds no waker, the future then stores one, and nobody ever calls it. The actual call of `wake` belongs after the lock is released, so that the woken task does not block on the lock at once.

**4. How does `unpark` avoid losing a wake that comes before `park`?**
Each thread has a token. `unpark` makes the token present, whether the thread is parked or not. `park` returns at once if the token is present, and removes it. So when the wake arrives after the poll returned `Pending` but before the executor thread has parked, the token waits, and the next `park` does not sleep. The token is a flag and not a counter, and `park` may also return without any `unpark`. Both facts are harmless for an executor that polls again after every return from `park` and parks again if the future is still pending.

**5. Who does what: executor, reactor, future?**
The *future* holds the state of one computation and moves it forward a little on each poll. On `Pending` it is responsible for arranging its own wake, by handing its waker to whoever knows when progress is possible. The *executor* owns the tasks, decides which one to poll next, creates the waker for each task, and sleeps when no task has been woken. The *reactor* (also called the I/O driver) knows about the outside world: it asks the operating system which sockets are ready, keeps the table from event sources to wakers, and calls those wakers. A timer driver does the same for deadlines. The waker is the only connection that a future needs to either of them.

**6. Why does pinning matter once polling has begun, and what does `Unpin` allow?**
A future made from an async block stores its local variables in itself, and after its first poll one of them may be a reference to another. If the future were moved, that reference would point to the old address. Before the first poll no such reference exists, so a new future can be moved freely, for example into a `Box`. `poll` takes `Pin<&mut Self>` so that the caller promises not to move the future from then on. `Unpin` marks the types for which a move is always harmless, which is almost every ordinary type, including `Pin<Box<F>>` itself. For them `Pin::new` and `get_mut` are available and the pin has no effect. Async blocks and the futures of `async fn` are never `Unpin`; you pin them with `pin!` on the stack or with `Box::pin` on the heap.

**7. How does a future cooperate?**
It limits the work of one poll: a fixed number of items, or of steps. When the limit is reached and work remains, it saves its position, calls `cx.waker().wake_by_ref()` and returns `Pending`. The self-wake is correct here because progress was made and the future can continue at once; it only gives the other tasks a turn first. The number of polls for `n` items and budget `b` is `n / b` rounded up, and at least one. Work that cannot be cut into pieces, or that calls blocking functions, should run on another thread, and the task should wait for its result as a future.

</details>
