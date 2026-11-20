# Mastery M4 hints

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Notes](NOTES.md)

Try the first hint before opening the second. Open *Check your explanation* only after writing your own answer to the lesson's EXPLAIN prompt. The model solutions are in [`solutions/mastery/m4_async/`](../../solutions/mastery/m4_async/); look at them after your own attempt.

## l01 · RawWaker and its vtable, by hand

<details><summary>Hint 1</summary>

Task B is four conversions between `*const ()` and `Arc<Signal>`. `Arc::into_raw` gives the pointer and keeps the reference alive. `Arc::from_raw` takes one reference back; when the `Arc` it returns is dropped, the count goes down. `Arc::increment_strong_count` adds a reference without creating an `Arc` value. A plain `&*data.cast::<Signal>()` borrows without touching the count. Match each vtable function to the row of the table in CONCEPTS: does the caller's reference stay, or is it handed over?

</details>
<details><summary>Hint 2</summary>

`waker_by_hand`: `let data = Arc::into_raw(signal).cast::<()>();` then `unsafe { Waker::from_raw(RawWaker::new(data, &SIGNAL_VTABLE)) }`. `signal_clone`: `Arc::increment_strong_count(data.cast::<Signal>())`, then return `RawWaker::new(data, &SIGNAL_VTABLE)`. `signal_wake`: `let signal = Arc::from_raw(data.cast::<Signal>()); signal.notify();` and let `signal` drop. `signal_wake_by_ref`: `(&*data.cast::<Signal>()).notify()`. `signal_drop`: `drop(Arc::from_raw(..))`. Task C needs a second `static` vtable whose `clone` returns `RawWaker::new(data, &COUNTER_VTABLE)`, whose wake functions do `fetch_add(1, ..)` on `&*data.cast::<AtomicUsize>()`, and whose `drop` is empty. Each `unsafe` block needs its SAFETY comment: say where the pointer came from and who owns the reference.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Each waker owns one strong reference. `clone`: the caller keeps its reference, and the function creates one more for the new waker. `wake`: the caller hands its reference over; the function notifies and releases it, because `Waker::wake` forgets the waker and never calls `drop`. `wake_by_ref`: the caller keeps its reference; the function only notifies. `drop`: the caller hands its reference over; the function releases it without notifying. The vtable is a `static` because `Waker::will_wake` compares the addresses of the two vtables: a `static` is one object with one address, while each use of a `const` may get its own copy, so two wakers of the same task could compare as different.

</details>

## l02 · Tasks, a run queue, spawn and JoinHandle

<details><summary>Hint 1</summary>

Work in the order A, C, B, D and run the tests after each. `Task::schedule` is three steps: `swap` the `queued` flag and return if it was already set; upgrade the weak pointer; lock, push `Runnable::Task(Arc::clone(self))`, unlock, `notify_one`. `block_on` is a loop around one helper: lock the state, pop the front of the queue or wait on `work`, and remember to clear `root_queued` when you pop the root marker. In `spawn`, the box holds `async move { let output = future.await; completer.complete(output); }`.

</details>
<details><summary>Hint 2</summary>

`Task::run`: `self.queued.store(false, ..)`; `let Some(mut future) = lock(&self.future).take() else { return };` (the guard is a temporary, so the lock is released at the end of the statement); if not aborted, make the waker with `Waker::from(Arc::clone(&self))` and poll; on `Pending` put the future back and return; otherwise drop the future and remove `self.id` from `State::tasks`. `Completer::finish` changes the outcome only if it is `Running`, takes the waiter inside the lock, and wakes it after the lock. `Executor::drop`: under the lock set `closed` and `mem::take` the queue and the task table; after the lock, for each task take the future out and drop it. `JoinHandle::poll`: `mem::replace` the outcome with `Taken`, match on what was there, and put `Running` or `Cancelled` back in those two arms.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Strong arrows: executor and spawner to the shared state; the shared state's task table and its queue to each task; every waker to its task; the join handle and the task's future (through the completer) to the join state. Weak arrows: the task to the shared state, and the join handle to the task. A cycle of strong arrows would need a path from a task back to the shared state. The only such path goes through the task's *future*, which may own a spawner. `Executor::drop` empties the queue and the table and drops every future, so that path is cut, and what is left are wakers that point at tasks without a future. Because the task's own arrow to the shared state is weak, those leftover wakers keep nothing else alive.

</details>

## l03 · A timer heap and sleep

<details><summary>Hint 1</summary>

Decide on the data structure before you write a method. With a `BinaryHeap` you need three things: the heap of `Reverse((deadline, id))`, a map from id to waker for the live timers, and a counter for ids. Every method that removes something must leave the heap in a state where `peek` gives a *live* entry, or `next_deadline(&self)` cannot be right. With a `BTreeMap<(Instant, u64), Waker>` you also need a way from an id to its key. For the driver thread, write the loop as: take what is due; if anything was due, unlock, wake, lock again and start over; otherwise wait, with a timeout if there is a next deadline and without one if there is none.

</details>
<details><summary>Hint 2</summary>

`Sleep::poll`: first `if Instant::now() >= deadline` then cancel your own timer and return `Ready`. Otherwise lock. If you have an id and `waker_mut(id)` gives a slot: when `!slot.will_wake(cx.waker())`, `mem::replace` it with a clone and keep the old waker in a local. If you have no live timer: remember `next_deadline()`, insert, and `notify_all` on the condition variable when your deadline is earlier than what was there (or nothing was there). Then drop the guard, then drop the old waker, then return `Pending`. `Drop for Sleep`: `let waker = lock(..).heap.cancel(id);` on one line, so the guard is gone before `waker` is dropped. `Drop for TimerDriver`: set `shutdown`, take the whole heap out, unlock, `notify_all`, drop the heap, join the thread.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The rule: while the timer lock is held, call no code that you did not write, so no `wake` and no drop of a `Waker`. A program that deadlocks without it: a task's waker, when woken, runs code that starts a new timer on the same driver (a retry, or an executor that re-arms a watchdog on every wake). If the driver thread calls `wake` under its lock, that code asks for the same lock on the same thread and waits for itself. `Sleep` reads the clock because a poll is not proof of anything: the task may have been woken by another future, or the executor may poll spuriously. Only the comparison of the clock with the deadline says that the time has passed. The same check makes the registration race harmless: a deadline that passes between the check and the insert is found by the driver at once.

</details>

## l04 · A reactor over non-blocking sockets

<details><summary>Hint 1</summary>

The reactor's state is a list of wakers and a shutdown flag behind a mutex, plus a condition variable. `Condvar::wait_while` is the idle wait ("while the list is empty and there is no shutdown"), and `wait_timeout_while` with `TICK` is the tick ("while there is no shutdown"). After the tick, move the list out of the state, release the lock, and wake. `poll_io` is the loop from CONCEPTS: `match op()` with one arm for `WouldBlock`, one for `Interrupted`, and one for everything else. Every socket method is then one line that passes a closure to `poll_io`.

</details>
<details><summary>Hint 2</summary>

`poll_read`: `self.reactor.poll_io(cx, || (&self.inner).read(buf))`; the `&` matters, because `Read` is implemented for `&TcpStream`. `poll_accept`: `ready!(self.reactor.poll_io(cx, || self.inner.accept()))`, then wrap the stream with `AsyncStream::new`, which calls `set_nonblocking(true)`. The async methods are `std::future::poll_fn(|cx| self.poll_read(cx, buf)).await`. `write_all`: `while !buf.is_empty()`, await one `poll_write`, return `ErrorKind::WriteZero.into()` for `0`, otherwise `buf = &buf[written..]`. `Drop for Reactor` has the shape of `Drop for TimerDriver`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The executor polls the task. The read future calls `poll_io`, which calls `read` on the non-blocking socket and gets `WouldBlock`. `poll_io` clones the waker into the reactor's list (now two clones exist: the executor's and the reactor's) and returns `Pending`; the executor goes on to other tasks or sleeps. The reactor thread, which was idle, is notified, waits one tick, takes the list, releases its lock and calls `wake`. That consumes the reactor's clone and puts the task on the run queue. The executor thread polls the task again, `read` returns the bytes, and the future is `Ready`. With epoll, kqueue or IOCP the reactor thread sleeps *inside the system call* and gets back the list of sockets that became ready, so it wakes only the tasks that can make progress. Its cost is proportional to the number of ready sockets, not to the number of waiting tasks, and there is no tick of delay.

</details>

## l05 · Pin projection by hand

<details><summary>Hint 1</summary>

Every projection has the same shape: `unsafe { let this = self.get_unchecked_mut(); .. }`, then `Pin::new_unchecked(&mut this.field)` for a structurally pinned field and `&mut this.field` for a plain one. For a single field `self.map_unchecked_mut(|this| &mut this.field)` is shorter. Write the SAFETY comment as the four duties, one line each. In `Fuse::poll` you need no more `unsafe` after the projection: look up `Option::as_pin_mut` and `Pin::set`. In `Chain::poll`, `match` on `self.as_mut().get_unchecked_mut()` and re-pin the `future` of the current variant.

</details>
<details><summary>Hint 2</summary>

`Fuse::poll`: `let mut inner = self.inner();` then `let Some(future) = inner.as_mut().as_pin_mut() else { return Poll::Pending };`, `let output = ready!(future.poll(cx));`, `inner.set(None);`, `Poll::Ready(output)`. `Chain::poll` is a `loop`: in `First { future, next }` poll the re-pinned future with `ready!`, take the closure with `next.take()`, build the second future, and `self.set(Chain::Second { future: second })`; do not return, so the loop polls the new state in the same call. In `Second { future }` poll, then `self.set(Chain::Done)` and return the output. In `Done`, panic with a message that contains `polled after`, as the contract asks. The borrow of `future` and `next` ends before `self.set`, because you do not use them afterwards.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`Fuse<F>` has one field, `inner: Option<F>`, and it is structurally pinned as a whole. (1) Unpin: there is no manual impl, so `Fuse<F>` is `Unpin` exactly when `Option<F>` is; adding `impl<F> Unpin for Fuse<F> {}` would break this, because safe code could then call `Pin::get_mut` and move the future. (2) Drop: `Fuse` has no `Drop` impl; a `Drop` that did `let f = self.inner.take();` would move a pinned future. (3) No moves: the option is reached only through `Pin<&mut Option<F>>`, and it is emptied with `Pin::set`, which drops the future in place; replacing that with `Option::take` moves the future out before dropping it. (4) Layout: the struct is not `repr(packed)`; adding that attribute could misalign the field, and a misaligned field is moved to be dropped.

</details>

## l06 · A Stream trait, a channel, and adapters

<details><summary>Hint 1</summary>

The channel is one struct behind `Arc<Mutex<..>>`: the queue, the capacity, the number of senders, a flag "receiver alive", and the receiver's waker. Write `poll_next` first as three checks in this order: an item, the end, register. Then decide for each of `try_send`, `Clone for Sender`, `Drop for Sender` and `Drop for Receiver` what it changes and whether it must wake. For the adapters, write one private struct that holds a stream and some state, with one projection, and implement `Stream` for it once per kind of state. `merge` needs its own struct with two pinned streams. `iter` is the other way round: its stream pins nothing, so when the compiler says that the iterator "cannot be unpinned", the answer is `impl<I> Unpin for YourStream<I> {}` and not a `Box` (CONCEPTS says why that impl is sound here).

</details>
<details><summary>Hint 2</summary>

`try_send`: inside a block, lock; return `Err(Closed(value))` if the receiver is gone, `Err(Full(value))` if `queue.len() >= capacity`; push; `state.waiter.take()`. After the block, wake what you took. `Drop for Sender`: decrease the count, and take the waiter only if the count reached zero. `Drop for Receiver`: clear the flag and `mem::take` the queue and the waiter out of the lock before they drop. `filter` is `loop { match ready!(stream.as_mut().poll_next(cx)) { Some(item) if keep(&item) => return .., Some(_) => {}, None => return .. } }`. `merge`: read and flip the preference, poll the preferred side unless it is done, then the other side unless it is done; `Ready(None)` only when both are done; otherwise `Pending`. A helper `fn poll_side(stream, done: &mut bool, cx) -> Option<Item>` keeps the two orders short.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The receiver is woken in two cases. A value is queued: `try_send` pushes and takes the waker under the lock, and `poll_next` checks the queue and stores the waker under the same lock, so either the poll sees the value or the send sees the waker. The last sender is dropped: `Drop for Sender` decreases the count and takes the waker under the lock, and `poll_next` checks the count under the lock, with the same argument. Nothing else can end the wait, so nothing else needs to wake. (Dropping the receiver wakes nobody; it ends the wait from the other side.) `merge` must remember an ended side because the contract of `poll_next` forbids polling after `None`: a stream may panic, return `None` again, or return stale items. Without the flags `merge` would also never know that *both* sides have ended.

</details>

## l07 · Select and cancellation safety

<details><summary>Hint 1</summary>

`select` can be an `async fn`: pin both arguments with `pin!` and await a `std::future::poll_fn` that polls them in order. The token is a mutex around a flag and a collection of wakers. Each wait future needs to find *its own* waker again, to replace it on a later poll and to remove it in `Drop`, so give each registration a key (a counter and a `BTreeMap`, for example). For the line reader, list the fields first: every byte that was received and not yet returned must be in a field, and so must "the source has ended". Then write a plain `fn poll_line(&mut self, cx) -> Poll<..>` and make `next_line` a one-line wrapper around it.

</details>
<details><summary>Hint 2</summary>

`poll_line` is a loop with four steps. (1) Look for `b'\n'` in the part of the buffer you have not scanned yet. (2) If the line so far (up to the newline, or the whole buffer) is longer than `max_len`, remember the failure and return `Err(TooLong)`. (3) If there is a newline, `drain(..=end)` the line, drop the `\n` and a `\r` before it, and return it. (4) If the source has ended, return the rest as a last line, or `Ok(None)`; otherwise `ready!(Pin::new(&mut self.source).poll_next(cx))` and append the chunk, or set the "ended" flag. The wait future of the token: `cancelled()` returns `WaitCancelled { shared: Arc::clone(..), key: None }`; its `poll` returns `Ready` if the flag is set, otherwise inserts or replaces its waker; its `Drop` removes the entry and drops the waker after the lock.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A method is cancellation safe if dropping its future at any await point loses nothing and repeats nothing: calling the method again continues as if the first call had not happened. In practice the future must hold no state between awaits that exists nowhere else. `next_line` is safe: all received bytes are in the reader, and a chunk moves from the source into the buffer within one poll. `Receiver::next` is safe: it takes an item only in the poll that returns it. `AsyncStream::read` is safe: a read either returns bytes in that poll or has read nothing. `AsyncStream::write_all` is not: if it is dropped after a partial write, some bytes are on the wire and the caller does not know how many, so the stream is in an unknown state. That is acceptable only when the connection is closed afterwards.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

Review each piece with the tables of this module. For the waker: fill in the ownership table of lesson 01 for the four functions as they are written, and try to write the SAFETY comment for each block. For the latch: which waker is stored after two polls with different wakers, where is `wake` called relative to the lock, and when is the stored waker released? For `tracked`: which lines run when the future is dropped at the `.await`?

</details>
<details><summary>Hint 2</summary>

`tally_wake` must rebuild the `Arc` with `from_raw` so that the reference is released; borrowing is right only for `wake_by_ref`. `Latch::open`: take the waiter out inside a block, and call `wake()` on it after the block. `LatchWait::poll`: replace the stored waker unless `will_wake` says it is the same one. `tracked`: a struct that holds `&AtomicUsize` and decreases it in `Drop`, created right after the increase; then `future.await` without a `Box`.

</details>
<details><summary>Check your review (after writing your own)</summary>

Three bugs. (1) `tally_wake` only counts; it never releases the reference that a consumed waker owned, so every wake by value leaks one reference and the `Tally` is never freed. (2) `LatchWait::poll` keeps the *first* waker; a future that is polled again by another task, or inside a select, is woken through a stale waker and its real task sleeps on. (3) `tracked` decreases the gauge after the `.await`; a cancelled or panicking future never gets there, so the gauge only drifts upward. Smells: `open` calls `wake` while it holds the lock (a waker that looks at the latch deadlocks), and it wakes by reference and keeps the waker, so the waker lives as long as the latch and a second `open` wakes again; `tracked` boxes a future that could be awaited in place; the `unsafe` blocks have no SAFETY comments (clippy says so); and clippy flags the `== true` and the `return`.

</details>

## l08 · Capstone: a chat server on your own runtime

<details><summary>Hint 1</summary>

Two kinds of task: one accept loop, and one task per connection that does both the reading and the writing for its client. Shared between them: the limits, a timer handle, the stop token, and the room, which is a `Mutex` around a map from name to the sending end of that client's queue. A connection task waits with nested `select`s for: the stop token; its next line, wrapped in a `Timeout` whose deadline is a variable outside the loop; and the next item of its queue. Put the membership into a value whose `Drop` removes the name from the room and queues the `left` line for the others.

</details>
<details><summary>Hint 2</summary>

To feed your `LineReader`, write a small stream over `&AsyncStream` whose `poll_next` calls `poll_read` into a stack buffer of a kilobyte or more (CONCEPTS says why not less) and returns the bytes as a `Vec<u8>`, or `None` at the end or on an error. To deliver a line to "everyone else", copy the senders out of the room under the lock, release the lock, and `try_send` to each; when a `try_send` fails, remove that member and deliver its `left` line in the same way. Give every membership a number of its own, and let a task remove its name from the room only when the entry still carries its number: a removed client's task ends later, and by then the name may belong to somebody else. Write to the socket only through a helper that wraps `write_all` in a `Timeout`. Queue the answer to `/who` in the client's own queue, like a line from somebody else; written directly, it could overtake lines that were queued before it (rule 8). In the select, poll the stop token first: a connection that sees the stop sends the closing line and drops what is still in its queue (rule 9). After each handled line, `yield_now().await`. In `run`: start the reactor and the timer driver, create the executor, `block_on` the accept loop, and after it returns drop the executor *first*. In the accept loop, after the stop signal: drop the listener, then await the join handles inside a `Timeout` of `grace`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

In the model solution a line from `ann` to a `bob` who does not read passes these places. (1) The kernel's receive buffer of `ann`'s socket, bounded by the kernel. (2) The line reader's buffer in `ann`'s task, bounded by `max_line` plus one read. (3) `bob`'s queue, bounded by `outbox`: this is the only place where the server holds lines for a client. (4) The `write_all` in `bob`'s task, which holds one line. (5) The kernel's send buffer of `bob`'s socket and the receive buffer on `bob`'s machine, bounded by the kernel. When (5) is full, (4) is pending, so `bob`'s task stops taking lines from (3). When (3) is full and a new line is due, `try_send` fails, the server removes `bob` from the room, drops his queue's sender and tells the others. `bob`'s task ends when its write times out, or earlier if the write completes and it finds its queue closed.

</details>

## l09 · The same service on tokio

<details><summary>Hint 1</summary>

Keep the shape of your capstone and change three things. The queue per client is a `tokio::sync::mpsc::channel(limits.outbox)`, and delivering is `sender.send(line).await`. A connection runs two futures side by side with `tokio::join!`: one reads lines and delivers them, the other takes lines from the queue and writes them with `timeout(limits.patience, ..)`. The accept loop takes a permit from an `Arc<Semaphore>` with `acquire_owned()` before each `accept`, moves the permit into the spawned task, and keeps the tasks in a `JoinSet`.

</details>
<details><summary>Hint 2</summary>

Ending a connection: the reading side returns the farewell line (or none), then leaves the room, which drops the senders of its own queue; the writing side then sees `recv()` return `None` after the queued lines and ends too; after the `join!`, write the farewell and call `shutdown()` on the write half. Put the welcome line into the client's new queue with `try_send` *before* you insert the queue into the room: then no other task can get a line in front of it, on any runtime. A stuck client: when a write times out, the writing side calls `close()` on its receiver (every waiting `send` returns at once) and cancels a per-connection `CancellationToken` that the reading side selects on. Shutdown in `serve`: break out of the accept loop, drop the listener, `timeout(limits.grace, async { while set.join_next().await.is_some() {} })`, then `let aborted = set.len(); set.shutdown().await;`. Do not send `left` lines when the shutdown token is cancelled. A `Drop` guard for the membership still frees the name when a task is aborted. One wait needs an exit of its own: a reading side that waits for room in another client's queue must also stop when the shutdown begins, or a healthy client whose line waits behind a stuck one is aborted without its closing line. Wrap each delivery in `tokio::select! { biased; () = delivery => .., () = shutdown.cancelled() => .. }`; the line that waited is given up.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Sequence: (1) the token is cancelled; (2) the accept loop sees it, stops, and drops the listener, so no new client gets in; (3) each connection's reading side sees the token, stops reading, and leaves the room; its writing side sends what is still queued and ends; the connection writes `* server closing`, shuts its write half down and finishes; (4) `serve` joins the tasks, but for at most `grace`; (5) tasks that are still running are aborted, which drops their futures and their sockets; (6) `serve` reports. In the model solution a `select!` can cancel five awaits. Four of them are cancellation safe: `shutdown.cancelled()` and the per-connection token's `cancelled()`, which hold nothing; `acquire_owned()`, which only gives up its place in the queue; `listener.accept()`, which the `Listener` trait requires to be cancellation safe; and `lines.next_line()` inside `timeout_at`, which keeps its bytes in the `Lines` value. The fifth is not, and is cancelled on purpose: a delivery (`send` to the queues) that still waits for room when the shutdown begins. A cancelled `send` loses its line, and here that is the contract: once the shutdown has begun only queued lines are promised, and without this exit a healthy client that waits behind a stuck one could not be told goodbye. In normal operation no `send` is ever cancelled. Writing to the socket is never inside a `select!`, because a cancelled `write_all` leaves half a line on the wire; a write ends early only through its timeout, and then the connection is closed.

</details>

## Drills

Run `cargo drill m4`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · Polling a field through a pin</summary>E0599: through `Pin<&mut Self>` the field `self.inner` is a plain `F`, and `poll` exists only on `Pin<&mut F>`. The compiler's suggestion to `pin!` the expression does not work: it would have to move the field out of a pinned struct (E0507). Project instead: <code>let inner = unsafe { self.map_unchecked_mut(|this| &mut this.inner) };</code> with a SAFETY comment for the four duties, then <code>inner.poll(cx)</code>.</details>
<details><summary>d02 · Pinning a value that must not move</summary>E0277: `Pin::new` requires `Unpin`, because after it the caller still owns the value and could move it. `PhantomPinned` makes `Anchor` `!Unpin`. `std::pin::pin!(anchor)` takes the value over and gives a `Pin<&mut Anchor>` that nothing can move any more.</details>
<details><summary>d03 · A task that holds an Rc across an await</summary>A compile error without a code: "future cannot be sent between threads safely". Everything that is alive at an `.await` is stored in the future, and an `Rc` is not `Send`. Put the `Rc` into an inner block that ends before the `.await`, or use an `Arc`. Calling `drop(name)` before the `.await` does not help with this compiler: it decides by the variable's scope.</details>
<details><summary>d04 · A task that borrows from its creator</summary>E0373: a spawned task may run after the function that created it has returned, so its future must be `'static` and may not borrow `greeting`. `async move` puts the string inside the future.</details>
<details><summary>d05 · A heap of deadlines and wakers</summary>E0277: a heap must compare its elements, and `Waker` has no order. Order the entries by `(deadline, id)` and keep the wakers in a map from id to waker. The id also makes equal deadlines come out in insertion order.</details>
<details><summary>d06 · Which timer fires first</summary>Panics: the assertion fails with `[30, 20, 10]`. `BinaryHeap` is a max-heap, so `pop` returns the latest deadline. Wrap the entries in `std::cmp::Reverse` and unwrap them when you pop.</details>
<details><summary>d07 · Safe functions in a waker vtable</summary>Compiles and runs. A safe `fn` can be used where an `unsafe fn` pointer is expected: it is callable under any conditions, so it is also callable under the caller's extra promises. The one `unsafe` block is `Waker::from_raw`, where you promise that the table and the data pointer fit together.</details>
<details><summary>d08 · A future that outlives the handle it came from</summary>E0505: in edition 2024 a returned `impl Trait` is assumed to capture every lifetime in scope, here the one of `&self`, although this future owns its own `Arc`. So `token` counts as borrowed while `check` is alive, and `drop(token)` is refused. Write <code>-> impl Future&lt;Output = bool&gt; + use&lt;&gt;</code> to say that nothing is captured.</details>
<details><summary>d09 · A read that is cancelled halfway</summary>Panics: the second call returns "llo". The first call held "he" in a local variable, and that variable died with the future. Keep the partial line in the `Source`, behind `&mut self`, and take it out only when the line is complete.</details>
<details><summary>d10 · Pending, and nobody to call back</summary>Panics: "never woken". `poll` returned `Pending` without leaving the waker anywhere, so `open` had nobody to wake, and an executor would never poll this task again. Store `cx.waker().clone()` in the door before returning `Pending`.</details>

## Mastery check: model answers

<details><summary>1 · The four functions and the reference count</summary>

`clone` adds one reference for the new waker and returns the same data pointer and the same vtable. `wake` notifies and then releases the reference of the waker that was consumed; the standard library does not call `drop` after `wake`. `wake_by_ref` notifies and changes nothing. `drop` releases the reference and does not notify. All four must be safe to call from any thread. If `wake` forgets to release, every wake by value leaks a reference and the task is never freed. If `wake_by_ref` releases, the waker's own reference is gone while the waker still exists; a later use is a use after free.

</details>
<details><summary>2 · Five places to lose a wakeup</summary>

Lesson 02: the executor checks "queue empty" and goes to sleep in two separate steps; a push between them is missed. Rule: check and wait under the one lock that every push takes. Lesson 03: a timer with an earlier deadline is added while the driver sleeps until a later one. Rule: tell the driver when the earliest deadline changes; and `Sleep` reads the clock itself. Lesson 04: data arrives between the failed `read` and the registration. Rule: the reactor wakes every registered waker (or, with a real reactor, readiness is level-triggered or the operation is retried after registering). Lesson 05: a state machine changes state and returns `Pending` without polling the new future. Rule: return `Pending` only when something that was polled in this call returned `Pending`. Lesson 06: `filter` returns `Pending` after rejecting an item, or the last sender is dropped without waking the receiver. Same rule, and: every event that ends a wait must wake.

</details>
<details><summary>3 · Send + 'static on one thread</summary>

`Send`: a waker is `Send + Sync` and may be cloned, woken and dropped on any thread. The waker is an `Arc` of the task, and the task holds the future, so the future can be reached, and dropped, from other threads. `'static`: the task outlives the call to `spawn` and may outlive the function that called it, so it cannot borrow from that function's stack. A runtime that wants `!Send` futures needs a separate design that keeps the future on its thread, such as tokio's `LocalSet`.

</details>
<details><summary>4 · The lock rule</summary>

While you hold a lock, call no code that you did not write: no `wake`, no drop of a `Waker` (or of a user's value). Example: a channel wakes its receiver inside `try_send` while holding the channel lock; the waker belongs to an executor that polls the receiving task at once on the same thread; that poll calls `poll_next`, which asks for the channel lock, and the thread waits for itself.

</details>
<details><summary>5 · Pin and the four duties</summary>

`Pin<&mut T>` promises that the `T` stays at its address until it is dropped, and that it is dropped before its memory is reused, unless `T: Unpin`. Projecting to a structurally pinned field obliges you to: write no `Unpin` impl of your own; have no `Drop` impl that moves the field; never move the field out, overwrite it without dropping, or forget it; and not make the struct `repr(packed)`. `Option::take` breaks the third: it moves the pinned value out of its place.

</details>
<details><summary>6 · Polling reactor and real reactor</summary>

A reactor on epoll or kqueue registers each socket with the kernel and sleeps in one system call that returns the sockets that became ready; it wakes only their tasks. The cost per wake-up of the reactor is proportional to the number of ready sockets, and there is no tick. For the caller nothing changes: `poll_io` still tries the non-blocking call, registers the waker on `WouldBlock`, returns `Pending`, and is polled again later, possibly to find `WouldBlock` once more.

</details>
<details><summary>7 · Cancellation safety</summary>

A method is cancellation safe if its future can be dropped at any await point without losing or repeating anything, so that a later call continues correctly. `next_line` keeps every received byte in the reader, not in the future. `write_all` holds the position in the buffer in the future, and the bytes already written cannot be taken back. The capstone never cancels a `write_all` and then uses the connection again: a write runs under a timeout, and when the timeout wins the connection is closed.

</details>
<details><summary>8 · A client stops reading</summary>

Lesson 08: its task's write becomes pending, its queue fills, and the next line for it fails with `Full`; the server removes it from the room and closes the connection. Simple, memory is bounded, healthy clients never wait; but a client that is slow for a moment is cut off. Lesson 09: senders wait for room in its queue and stop reading their own clients, so the pressure moves back to the producers and nothing is lost; if the client takes no line for `patience` it is removed. Kinder to slow clients and still bounded; but one slow client delays the others for up to `patience`, and the design must avoid a deadlock between two full queues.

</details>
<details><summary>9 · Drop order</summary>

The executor first, then the drivers. Dropping the executor drops every task's future, and with it every `Sleep`, every socket wrapper and every handle to a driver. A `Sleep` cancels its timer when it is dropped, and the sockets are closed. What the drivers may still hold are wakers of tasks that no longer have a future; waking or dropping those does nothing. After that the drivers can stop and join their threads. In the other order, tasks would for a moment hold futures whose driver is gone; any such future that was still polled would wait for ever.

</details>
<details><summary>10 · In-memory tests with a paused clock</summary>

They can prove logic that depends on time and order exactly and at once: that an idle client is removed after 60 seconds and not after 59, that a third connection is not served while two are open, that shutdown waits for `grace` and no longer, that a full queue stops the sender. A test over real sockets cannot check "nothing happens for a minute" without waiting a minute, and its timing varies. In-memory tests cannot prove that the code handles real sockets: partial writes, resets, the mode of accepted sockets, the behaviour of kernel buffers. One test over loopback TCP covers the thin layer that touches them.

</details>
