# Mastery M3 hints

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Notes](NOTES.md)

Each lesson has two hints. Open the first one only after a real attempt, and the second one only if the first was not enough. The rules in each task's doc comment and in each contract are part of the exercise. *Check your explanation* is a model answer to the lesson's EXPLAIN prompt; read it after you have written your own. Model answers to the mastery check are at the end.

Three things that apply to every lesson:

- Import from `crate::sync`. If a model test passes although you know that an ordering is wrong, look for a `use std::sync::atomic` in your code.
- The cell in `crate::sync` has `with(|p| ...)` for reading and `with_mut(|p| ...)` for writing. The closure gets a raw pointer. `p.cast::<T>()` turns a `*mut MaybeUninit<T>` into a `*mut T`, and `p.read()`, `p.write(v)` and `p.drop_in_place()` do the rest without creating a reference. To return a reference with the lifetime of `&self`, let the closure return the pointer and dereference it outside: `unsafe { &*self.value.with(|p| p) }`.
- When a model test fails, read the first panic message. "Causality violation" means that two accesses to the same memory are not ordered. "deadlock" means that every thread is blocked.

## 01 · Memory orderings and happens-before

<details><summary>Hint 1</summary>

A: write the loop as "read, decide, try to replace". `compare_exchange_weak(seen, seen + 1, ..)` succeeds only if nobody changed the counter since you read it; on failure it gives you the new value, and you decide again. Nothing else is shared, so ask yourself whether any ordering stronger than `Relaxed` has a partner to pair with. B: there are three steps in `set`: claim the cell, write the value, announce it. Which of the three must be visible to `get` before `get` may touch the cell? That step is the `Release` store, and the load in `get` is its `Acquire`. C: the function is three lines: store, fence, load.

</details>
<details><summary>Hint 2</summary>

A: `let mut used = self.used.load(Relaxed); loop { if used >= self.limit { return false; } match self.used.compare_exchange_weak(used, used + 1, Relaxed, Relaxed) { Ok(_) => return true, Err(now) => used = now } }`. B: claim with `compare_exchange(EMPTY, WRITING, Relaxed, Relaxed)` and return `Err(value)` if it fails. Then `self.value.with_mut(|p| unsafe { p.cast::<T>().write(value) })`, then `self.state.store(READY, Release)`. In `get`: `if self.state.load(Acquire) != READY { return None; }`, then build the reference from `self.value.with(|p| p.cast::<T>())`. `Drop`: if the state is `READY`, `drop_in_place`; a `Relaxed` load is enough there, because `&mut self` already means that nobody else is using the cell. C: `mine.store(true, Relaxed); fence(SeqCst); !theirs.load(Relaxed)`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A happens before B if both are in one thread and A comes first, or if A is a `Release` store and B an `Acquire` load that reads the stored value, or through spawn, join, a lock or an unpark, or through a chain of these. Two accesses to the same plain memory, one of them a write, must be ordered this way; otherwise the program has a data race. Task A needs no such order: the counter is the only shared memory, and a compare-exchange always works on the newest value, so `Relaxed` is enough. In task B the write of the value in `set` must happen before the read in `get`. The store of `READY` is `Release` and comes after the write; the load in `get` is `Acquire` and comes before the read; the chain write → store → load → read orders the two. The compare-exchange that claims the cell publishes nothing and can be `Relaxed`. In task C no chain exists: each thread stores to one flag and loads the other, and each load may read the initial value. The requirement "at least one thread sees the other's store" needs one order of all four operations, which is what `SeqCst` (or a `SeqCst` fence between store and load) provides.

</details>

## 02 · A spin lock with a guard

<details><summary>Hint 1</summary>

A: ask the compiler. In a scratch test, write `fn needs_sync<T: Sync>() {}` and call it with `SpinGuard<'static, Cell<u32>>`. It compiles with the starter's fields, and it should not. The guard hands out `&T` and `&mut T`; which built-in type does the same and already has the right `Send` and `Sync` rules? Tell the compiler that the guard "contains" one, with `PhantomData`. B: `lock` is a loop around one compare-exchange from `false` to `true`. The guard's `Drop` is one store. The two must form a `Release`/`Acquire` pair. `try_lock` is the same compare-exchange once, without a loop, and therefore the strong version.

</details>
<details><summary>Hint 2</summary>

A: add the field `_borrow: PhantomData<&'a mut T>`. `&mut T` is `Sync` exactly when `T: Sync`, and the guard inherits that. (Together with the `lock` field, the guard is then `Sync` for `T: Send + Sync`. Only `T: Sync` is necessary, and the standard library's `MutexGuard` asks for no more, with an `unsafe impl`. The stricter rule refuses only types that are `Sync` and not `Send`, which is safe.) B: `while self.locked.compare_exchange_weak(false, true, Acquire, Relaxed).is_err() { hint::spin_loop(); }`. `try_lock`: `compare_exchange(false, true, Acquire, Relaxed).ok().map(|_| guard)`. `deref`: `unsafe { &*self.lock.value.with(|p| p) }`; `deref_mut`: `unsafe { &mut *self.lock.value.with_mut(|p| p) }`. `Drop`: `self.lock.locked.store(false, Release)`. `into_inner`: `self.value.into_inner()`. If the model stops with "exceeded maximum number of branches", your loop does not call `hint::spin_loop()` from `crate::sync`, or it uses `swap`, which writes on every attempt.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Invariant: while `locked` is `false`, nobody accesses `value`; while it is `true`, exactly one guard exists and only it accesses `value`. A guard is created only after a compare-exchange from `false` to `true` succeeded, and that can succeed for one thread at a time, so two guards never exist together. Therefore `deref` may create `&T` (only this guard can reach the value, and `&self` rules out `deref_mut`), and `deref_mut` may create `&mut T` (`&mut self` makes it the only use of the only guard). The store in `Drop` is `Release` and the successful compare-exchange in `lock` is `Acquire`: everything the previous owner did to the value happens before everything the next owner does. With `Relaxed` on either side, the two owners' accesses would be concurrent, which is a data race even though they never overlap in time. `SpinLock<T>: Sync` needs `T: Send`, because the lock passes `&mut T` from thread to thread and never shows the value to two threads at once. The guard is `Sync` only for `T: Sync`, because `&SpinGuard<T>` gives `&T`.

</details>

## 03 · A one-shot channel

<details><summary>Hint 1</summary>

Write next to each state who may touch `message`. `EMPTY`: only the sender (to write). `READY`: only the receiver, or `Drop`. `TAKEN` and `CLOSED`: nobody. `send` is: write the message, store `READY`, unpark, in this order. `recv` is a loop: load the state; on `READY` leave the loop, on `CLOSED` return the error, otherwise park. After the loop, mark the message as taken and move it out. The sender's `Drop` runs after `send` too; it must close the channel only if it is still `EMPTY`. That is a job for a compare-exchange. `Shared::drop` must drop a message that is `READY` and nothing else.

</details>
<details><summary>Hint 2</summary>

`channel`: one `Arc::new(Shared { state: AtomicU8::new(EMPTY), message: UnsafeCell::new(MaybeUninit::uninit()) })`, cloned once; `receiver: thread::current()` in the sender; `PhantomData` in the receiver. `send`: `self.shared.message.with_mut(|p| unsafe { p.cast::<T>().write(value) })`, then `state.store(READY, Release)`, then `self.receiver.unpark()`. `Drop for Sender`: `if state.compare_exchange(EMPTY, CLOSED, Relaxed, Relaxed).is_ok() { self.receiver.unpark(); }`. `recv`: `loop { match state.load(Acquire) { READY => break, CLOSED => return Err(Disconnected), _ => thread::park() } }` (a loop, not an `if`: one test makes the first `park` return early on purpose), then `state.store(TAKEN, Relaxed)`, then `Ok(message.with(|p| unsafe { p.cast::<T>().read() }))`. `Drop for Shared`: `if state.load(Relaxed) == READY { drop_in_place }`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`EMPTY`: the message is not initialised; only the sender may touch it, to write it once. `READY`: it is initialised; only the receiver may read it, or `Drop` if no receiver ever does. `TAKEN`: the receiver moved it out, so the slot counts as empty again and nobody touches it. `CLOSED`: it was never written. The sender stores `READY` with `Release` after the write, and the receiver loads with `Acquire` before the read, which orders the two accesses. `recv` cannot miss a wake-up because the sender changes the state first and unparks second. Take the receiver's last check before it parks. If the check came after the store, it saw `READY`. If it came before, then the unpark comes after the check, so its token is available when `park` runs, or arrives while the thread sleeps. The loop makes early returns of `park` harmless. The sender remembers the thread that called `channel` and unparks that thread. If the receiver could move to another thread, `recv` would park there and the wake-up would go to the wrong thread. Making the receiver `!Send` turns that rule into a compile error.

</details>

## 04 · `MyArc` and `MyWeak`

<details><summary>Hint 1</summary>

Design the heap block first: two `AtomicUsize` and the value. Let all the `MyArc`s together own one weak reference, so the weak counter starts at 1; then "the block is freed when the weak counter reaches zero" is the only rule for freeing. Both handle types are a `NonNull` to the block (allocate with `Box`, keep the pointer with `Box::leak` or `Box::into_raw`, free with `Box::from_raw`). Wrap the value as `UnsafeCell<ManuallyDrop<T>>`: the last `MyArc` drops the value in place while `MyWeak`s may still read the counters, and freeing the block later must not drop the value again. Then go through the operations and ask for each: does this thread already have the right to use the block? If yes, the counter update needs no ordering.

</details>
<details><summary>Hint 2</summary>

`clone` and `downgrade`: `fetch_add(1, Relaxed)`, then abort if the old value was above `usize::MAX / 2`. `Drop for MyArc`: `if strong.fetch_sub(1, Release) != 1 { return; }`, then `fence(Acquire)`, then drop the value with `value.with_mut(|p| unsafe { ManuallyDrop::drop(&mut *p) })`, then give back the shared weak reference by constructing a `MyWeak` from the same pointer and dropping it. `Drop for MyWeak`: `if weak.fetch_sub(1, Release) != 1 { return; }`, `fence(Acquire)`, `drop(unsafe { Box::from_raw(self.ptr.as_ptr()) })`. `upgrade`: load `strong`; loop: if it is 0 return `None`; otherwise `compare_exchange_weak(n, n + 1, Relaxed, Relaxed)`, and on failure continue with the value it returned. `weak_count` is the weak counter minus one. All four `unsafe impl`s have the bound `T: Send + Sync`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`strong` counts the `MyArc`s. `weak` counts the `MyWeak`s, plus one that all the `MyArc`s own together. The value is dropped by the thread that takes `strong` from 1 to 0. The block is freed by the thread that takes `weak` from 1 to 0; the last `MyArc` gives back the shared weak reference after it has dropped the value, so a block without `MyWeak`s is freed at that moment. Increments are `Relaxed`: the thread already owns a handle, so the block cannot go away, and the increment has nothing to announce. Decrements are `Release`, and the thread that reaches zero does an `Acquire` fence: every use of the value (or of the block) through any other handle must happen before the drop (or the free), and those uses were on other threads. `upgrade` is a compare-exchange loop so that "not zero" and "add one" are one step; it can be `Relaxed`, because a non-zero count means that some `MyArc` keeps the value alive. `Send` and `Sync` both need `T: Sync`, because two handles on two threads give out `&T`, and `T: Send`, because the thread that drops the last handle drops the `T`, wherever the `T` was created.

</details>

## 05 · A mutex that puts waiting threads to sleep

<details><summary>Hint 1</summary>

Three fields are enough: an `AtomicBool`, the value in an `UnsafeCell`, and a `SpinLock<VecDeque<Thread>>` of waiters. Write a private `acquire(&self) -> bool` that does one strong compare-exchange; `try_lock` and both paths of `lock` use it. In the slow path the order is everything: put `thread::current()` into the queue, *then* try `acquire` again, and only if that fails, park. In the guard's `Drop`: store `false`, *then* look into the queue and unpark a waiter. Never hold the queue's lock while you park, and call `unpark` after you have released it. Decide when a thread leaves the queue; the simplest rule is "when it owns the mutex".

</details>
<details><summary>Hint 2</summary>

`lock`: `if self.acquire() { return guard; } let me = thread::current(); self.waiters.lock().push_back(me.clone()); while !self.acquire() { thread::park(); } self.waiters.lock().retain(|w| w.id() != me.id()); guard`. `Drop for MutexGuard`: `self.mutex.locked.store(false, Release); let next = self.mutex.waiters.lock().front().cloned(); if let Some(waiter) = next { waiter.unpark(); }`. `acquire` is `compare_exchange(false, true, Acquire, Relaxed).is_ok()`. The guard needs the same `PhantomData<&'a mut T>` as in lesson 02, and the mutex needs `unsafe impl<T: Send> Sync`. The fast path must come first and must not touch the queue: the test for "no allocation on a free mutex" depends on it.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A thread that must wait: (1) puts its handle into the queue, (2) tries once more to take the mutex, (3) parks if that failed, and after waking goes back to (2), (4) removes itself from the queue once it owns the mutex. A thread that unlocks: (1) stores `false`, (2) reads the front of the queue, (3) unparks that thread. Compare the waiter's step 1 with the unlocker's step 2; the queue's lock puts them in some order. If the waiter registered first, the unlocker finds a waiter and unparks the front one; that thread takes the mutex or loses it to another thread, and whoever owns the mutex next will run the unlock steps again, so every thread in the queue is reached in the end. If the unlocker looked first and found nothing, its store of `false` is before the waiter's step 1 and therefore before the waiter's step 2, so that attempt sees a free mutex, or a mutex that a third thread took in the meantime, and that thread's unlock comes later and will find the waiter in the queue. An unpark that arrives before the park leaves a token, so the gap between step 2 and step 3 is safe. The loop around `park` handles early returns and losing the race against a thread that takes the fast path.

</details>

## 06 · A bounded channel with `Mutex` and `Condvar`

<details><summary>Hint 1</summary>

Put everything that a waiting thread asks about into one struct behind one `Mutex`: the `VecDeque`, the number of senders, and whether the receiver is alive. Use two condition variables, one for "the receiver should look again" and one for "senders should look again". Write the two predicates as comments first. Then list every place that can make each predicate false: `send` and `try_send` (a message arrived), `recv` (a slot is free), `Drop for Sender` (the last sender is gone), `Drop for Receiver` (nobody will ever make room). Each needs a notification on the right condition variable. Decide for each whether one waiter or all waiters are concerned.

</details>
<details><summary>Hint 2</summary>

`send`: lock; `loop { if !state.receiver_alive { return Err(SendError(value)); } if state.queue.len() < capacity { break; } state = not_full.wait(state).unwrap(); }`; push; unlock; `not_empty.notify_one()`. `recv`: lock; `loop { if let Some(v) = state.queue.pop_front() { unlock; not_full.notify_one(); return Ok(v); } if state.senders == 0 { return Err(RecvError); } state = not_empty.wait(state).unwrap(); }`. Check the queue before the sender count, so that queued messages are delivered after the last sender is gone. `Clone` adds one to `senders`. `Drop for Sender` subtracts one and calls `not_empty.notify_all()` if it was the last. `Drop for Receiver` sets `receiver_alive = false`, takes the queue out with `std::mem::take`, unlocks, calls `not_full.notify_all()`, and only then drops the messages. Create the queue with `VecDeque::with_capacity(capacity)`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A sender keeps sleeping while "the receiver is alive and the queue is full". That becomes false when `recv` pops a message (one slot: `notify_one` on `not_full`) or when the receiver is dropped (every sender must give up: `notify_all` on `not_full`). The receiver keeps sleeping while "the queue is empty and at least one sender exists". That becomes false when `send` or `try_send` pushes a message (`notify_one` on `not_empty`) or when the last sender is dropped (`notify_all` on `not_empty`). Each predicate is checked in a loop with the mutex held, because a thread that was notified gets the mutex only later, and another thread can change the state in between: a second sender that was not waiting can take the slot that the notification was about. All changes to the state happen with the mutex held, so a waiter cannot check, miss a change, and then sleep. Notifying after unlocking is allowed, and it lets the woken thread take the mutex at once.

</details>

## 07 · A lock-free stack, and the ABA problem

<details><summary>Hint 1</summary>

Two atomic pointers: `head` for the stack and a second one for the nodes that `pop` removed. A node needs the value, the link to the node below it, and a link for the second list. Keep the value in a `MaybeUninit<T>`: `pop` moves it out, and `Drop` must not drop it again. `push`: allocate with `Box::into_raw`, then loop "read `head`, write it into the node's `next`, compare-exchange `head` from that to the node". `pop`: loop "read `head`; if null, return `None`; read its `next`; compare-exchange `head` from the node to `next`". After a successful exchange the node is yours: move the value out and push the node onto the second list. Now the orderings: which thread writes the fields of a node, which thread reads them, and which pair of atomic operations lies between?

</details>
<details><summary>Hint 2</summary>

`push`: `load(Relaxed)`, then `compare_exchange_weak(head, node, Release, Relaxed)`; write `(*node).next = head` before each attempt. `pop`: `load(Acquire)`; read `(*head).next`; `compare_exchange_weak(head, next, Relaxed, Acquire)`; on failure continue with the returned pointer (that is why the failure ordering is `Acquire`). Take the value with `(&raw const (*head).value).cast::<T>().read()`. The second list can use `Relaxed` everywhere: nobody looks inside those nodes before `Drop`. `Drop`: walk the `head` list with `Box::from_raw`, dropping each value with `assume_init`; then walk the second list with `Box::from_raw` and do not touch the values. Both impls are `unsafe impl<T: Send> Send`/`Sync for Stack<T>`. They are needed although the code compiles without them: a struct of `AtomicPtr`s is automatically `Send` and `Sync` for every `T`, and the explicit impls replace that with the right rule.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Rule: a node is allocated in `push` and freed only in `Drop`. `pop` unlinks a node and keeps it on a second list. First use: `pop` reads `head` and then reads the `next` field of that node. The node was published, so by the rule it is still allocated, however many other threads have popped it in the meantime; the read is of valid memory. Its `next` field is never written after publication, so the read does not race. Second use: a node enters the stack once and leaves it once, because a removed node is never pushed again and its address cannot be given to a new node while it is still allocated. So if the compare-exchange finds the same pointer in `head`, that node never left the stack, nothing below it changed, and the `next` that was read is still the node below. Without the rule, the address could come back with a different `next`, and the compare-exchange would install a node that is no longer in the stack. The cost is memory: the stack keeps every node it ever allocated until it is dropped.

</details>

## 08 · Capstone: a wait-free ring buffer

<details><summary>Hint 1</summary>

One shared struct in an `Arc`: a boxed slice of `UnsafeCell<MaybeUninit<T>>` and two `AtomicUsize`. Choose how to tell "full" from "empty" before you write anything; one unused slot is the easiest (allocate `capacity + 1` slots). Then write the invariant: which slots are initialised, in terms of the two indices. `push` reads both indices, writes one slot and stores one index. `pop` reads both indices, reads one slot and stores one index. For the orderings, go through the three questions in the concepts section. `Drop` belongs to the shared struct, not to the two ends.

</details>
<details><summary>Hint 2</summary>

`push`: `tail.load(Relaxed)`; compute the next position; if it equals `head.load(Acquire)`, return `Err(value)`; write the slot at `tail` with `with_mut(|p| unsafe { p.cast::<T>().write(value) })`; `tail.store(next, Release)`. `pop`: `head.load(Relaxed)`; if it equals `tail.load(Acquire)`, return `None`; read the slot at `head` with `with(|p| unsafe { p.cast::<T>().read() })`; `head.store(next, Release)`. `Drop for Ring`: walk from `head` to `tail` and `drop_in_place` each slot. One `unsafe impl<T: Send> Sync for Ring<T>`; the two ends then get `Send` from `Arc`. Build the slots with `(0..=capacity).map(|_| UnsafeCell::new(MaybeUninit::uninit())).collect()`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

With one unused slot: the initialised slots are `head, head + 1, ..., tail - 1`, counted around the ring; `head == tail` is empty and `tail + 1 == head` is full. Only the producer stores `tail` and writes the slot at `tail`. Only the consumer stores `head` and reads the slot at `head`. The producer loads `tail` with `Relaxed` and the consumer loads `head` with `Relaxed`: each is the only writer of that index, so it reads its own last store. The producer stores `tail` with `Release` after writing a slot, and the consumer loads `tail` with `Acquire` before reading it: the write of the slot happens before the read. The consumer stores `head` with `Release` after reading a slot, and the producer loads `head` with `Acquire` before it writes a slot that this value of `head` shows as free: the consumer's read of the slot happens before the producer's next write to it. Without that second pair, a slot would be read and overwritten with no order between the two, although the indices never allow them to overlap in time.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

Review each function with a different question. `new`: draw the list for three slots; where does the last slot point, and what does an empty pool look like? `release`: what happens to the slot if the compare-exchange fails? `acquire`: between its two loads and its compare-exchange, let another thread run any number of complete operations, and look for a sequence after which `head` has the old value but the list below it is different. Then look at the orderings: `release` writes a link and publishes the slot; `acquire` reads the link of a slot that somebody else published.

</details>
<details><summary>Hint 2</summary>

(1) `(i + 1) % slots` links the last slot back to slot 0, so the list is a ring and the pool never runs out; the last slot must link to `NIL`, and a pool with no slots must start with `head == NIL`. (2) `release` tries once and drops the slot if it loses the race; it needs the same retry loop as `acquire`. (3) ABA: thread A reads `head = 0` and `next[0] = 1`; thread B acquires 0, acquires 1 and releases 0; A's compare-exchange succeeds and makes slot 1 the head although B holds it. Keep a version number next to the slot number in `head` (for example the high half of an `AtomicU64`), add one on every successful `acquire`, and compare both. (4) With `Relaxed` everywhere, `acquire` may read an old value of `next[slot]`. Publish with `Release` in `release`, and load `head` with `Acquire` in `acquire`, also on the failure path of the compare-exchange.

</details>
<details><summary>Check your review (after writing your own)</summary>

Four bugs. First, `new` builds a ring instead of a list, and gives an empty pool a head of 0: after every slot is taken, `acquire` hands out slot 0 again, and `SlotPool::new(0).acquire()` indexes out of bounds. An ordinary test finds this one. Second, `release` gives up when its compare-exchange fails. With two threads that is a lost slot; under Miri it also happens with one thread, because a weak compare-exchange may fail for no reason. Third, `acquire` has an ABA bug: it compares only the slot number, and a slot can leave the list and come back with a different slot below it. The model test shows a slot handed to two holders. Fourth, every ordering is `Relaxed`. All the shared memory is atomic, so there is no data race, but `acquire` may combine a current `head` with an out-of-date link, which corrupts the list in the same way. The repair is a `Release` compare-exchange in `release` and an `Acquire` load of `head` in `acquire`. Smaller points: `next: next` and the `if ... { return true; } else { return false; }` are noise that clippy reports; `Vec` can be a boxed slice, because the length never changes; and the comment "it is only a pool" explains nothing and excuses a bug. A design question that the tests leave open: `release` trusts the caller. Returning a guard from `acquire` that releases the slot in its `Drop` would make a double release impossible.

</details>

## Drills

<details><summary>d01 · An array of counters</summary>

`E0277`. `[value; 4]` copies `value` into each element, and an atomic is not `Copy`: a copy would be a second, separate counter. Write `[const { AtomicUsize::new(0) }; 4]`, which evaluates the block once per element, or `std::array::from_fn(|_| AtomicUsize::new(0))`.

</details>
<details><summary>d02 · A load with Release</summary>

`error` (no code). `Release` describes what a store publishes; a load has nothing to release. The lint `invalid_atomic_ordering` is an error by default. The load that pairs with a `Release` store is `Acquire`.

</details>
<details><summary>d03 · Holding a lock with `let _`</summary>

`error` (no code). `let _ = ...` binds nothing, so the guard is dropped at the end of that statement and the lock is free on the next line. For lock guards the lint `let_underscore_lock` makes this an error. Use a name such as `_guard`, which lives until the end of the block.

</details>
<details><summary>d04 · A guard inside a `match` head</summary>

`panic`. A temporary in the head of a `match` lives until the end of the whole `match`, so the guard is still alive inside the arm, and `try_lock().unwrap()` fails with `WouldBlock`. Take the job out in its own statement (`let job = jobs.lock().unwrap().pop();`) and match on `job`.

</details>
<details><summary>d05 · A guard inside an `if let` head</summary>

`ok`. Since edition 2024 a temporary in the head of an `if let` is dropped before the `else` block starts, so the lock is free there. Inside the `if` block it is still held, and in a `match` it is held in every arm.

</details>
<details><summary>d06 · Handing a guard to a worker</summary>

`E0277`: `MutexGuard<'_, i32>` cannot be sent between threads safely. On some systems a mutex must be unlocked by the thread that locked it, so the guard is not `Send`. Share the mutex and lock it inside the worker.

</details>
<details><summary>d07 · A home-made lock shared by two threads</summary>

`E0277`: `UnsafeCell<u64>` cannot be shared between threads safely. The compiler cannot know that the flag protects the cell. Add `unsafe impl<T: Send> Sync for Lock<T> {}` with a SAFETY comment that says why `with` lets only one caller at a time reach the value. `T: Send` is the right bound, as in lesson 02.

</details>
<details><summary>d08 · `Arc::get_mut` with an observer</summary>

`panic`. `get_mut` returns `None` unless this `Arc` is the only pointer of either kind: a `Weak` could be upgraded on another thread while you hold `&mut`. Change the value before you create the `Weak`. `Arc::make_mut` is not a fix here: with a `Weak` around, it moves the value into a new block and the observer is left with nothing.

</details>
<details><summary>d09 · Unpark first, park second</summary>

`ok`. `unpark` makes the thread's token available, and `park` returns at once when the token is available. So a wake-up that comes before the sleep is not lost. The worker in the second half shows the full pattern: park in a loop that checks a flag, and set the flag before unparking.

</details>
<details><summary>d10 · The value that `fetch_add` returns</summary>

`panic`: the assertion fails with `[2, 3, 4]`. `fetch_add` returns the value from before the addition, and that old value is the caller's id. Remove the `+ 1`. Do not replace it by a second `load`: another thread could change the counter between the two operations.

</details>

## Mastery check: model answers

<details><summary>1 · Data race and happens-before</summary>

A data race is two accesses to the same memory location from different threads, at least one of them a write and at least one of them not atomic, with no happens-before relation between them. It is undefined behaviour. A happens before B if A is earlier in the same thread, or through a chain of such steps and of synchronizing pairs. Four pairs: spawning a thread and anything in it (and anything in a thread and the return of `join`); a `Release` store and an `Acquire` load that reads its value; the unlock of a mutex and a later lock; `unpark` and the return of the `park` that uses its token.

</details>
<details><summary>2 · A `Relaxed` flag</summary>

It is not correct. The reader gets the right flag value, because the store is atomic. But a `Relaxed` store releases nothing, so the write of the buffer and the read of the buffer are concurrent: a data race. In practice the reader can see the flag and a buffer that is not completely written, through compiler or processor reordering. Change the store to `Release`; together with the `Acquire` load this orders the write before the read.

</details>
<details><summary>3 · When `SeqCst` is needed</summary>

Store buffering: thread 1 stores to X and then loads Y; thread 2 stores to Y and then loads X; the requirement is that at least one of them sees the other's store. With `Release`/`Acquire` both may load the initial value: a load is ordered only after the store whose value it reads, and each reads the initial value, which no `Release` store wrote. `SeqCst` puts all four operations into one order that both threads respect. Whichever store is first in that order is seen by the other thread's load, which is later in that order.

</details>
<details><summary>4 · `Sync` for the lock and for the guard</summary>

A shared lock gives `&mut T` to one thread at a time, which is like moving the `T` between threads: `T: Send`. It never gives two threads access at once, so `T: Sync` is not needed. A shared guard is different: `&Guard` gives `&T`, so two threads with `&Guard` share the `T` itself, and that needs `T: Sync`.

</details>
<details><summary>5 · State first, loop around park</summary>

If the waker unparks first, the waiter can wake, see the old state, and park again; the later state change wakes nobody. With the state changed first, the waiter's last check either sees the new state, or is followed by the unpark, whose token makes the next `park` return. The loop is needed because `park` may return without an `unpark`, and because a token may be left over from earlier code on the same thread.

</details>
<details><summary>6 · Both bounds on `Arc`</summary>

`T: Sync`, because two `Arc`s on two threads give `&T` to both: `Arc<Cell<u32>>` would let two threads write one `Cell`. `T: Send`, because the last `Arc` to be dropped drops the `T`, on whatever thread it is: an `Arc<MutexGuard<..>>` could unlock a mutex from the wrong thread.

</details>
<details><summary>7 · The orderings of `Arc`</summary>

The decrement is `Release`: it announces that this thread is done with the value. The thread that reaches zero needs the announcements of all the others before it drops the value, which is the `Acquire` fence on the zero path (or an `AcqRel` decrement). Without the pair, the last use on one thread and the drop on another are concurrent. The increment can be `Relaxed`, because the thread already holds a handle: the value cannot be dropped during the increment, and nothing is published by it.

</details>
<details><summary>8 · The order of the steps in a parking mutex</summary>

Waiter: register in the queue first, then try again, then park. Unlocker: free the mutex first, then look for waiters. If the waiter tries first and registers second, an unlock can fall between the two: it sees an empty queue, and the waiter then registers and sleeps for ever. If the unlocker looks first and frees second, a thread can register after the look and fail its attempt before the free, with the same result.

</details>
<details><summary>9 · The rule for condition variables</summary>

Every wait has a predicate over the state behind the mutex; the predicate is checked in a loop with the mutex held; and every change that can make a predicate false notifies the matching condition variable (one waiter if one can use the change, all if all are concerned). The loop is needed even without spurious wake-ups: between the notification and the moment the woken thread gets the mutex, another thread can take the mutex and change the state back, for example by taking the slot that was just freed.

</details>
<details><summary>10 · ABA</summary>

Thread 1 reads `head = C` and `C.next = B`. Thread 2 pops C, pops B, and pushes a node that has the address of C again, with `next = A`. Thread 1's compare-exchange sees `C` and succeeds, making B the head, although B was removed. Prevention: never free or reuse a node while the structure is shared (or defer it with hazard pointers or epochs), which also prevents a use after free; or compare a version counter together with the pointer, which prevents ABA but not a use after free, so it fits structures whose nodes are never freed, such as slots in an array.

</details>
<details><summary>11 · The ring buffer's orderings</summary>

The consumer's `Release` store of `head` comes after it has read a slot. The producer's `Acquire` load of `head` comes before it writes a slot that this value shows as free. The pair orders the consumer's read of the slot before the producer's next write to the same slot. Each side is the only writer of its own index, so a load of that index reads the thread's own last store; there is no other thread's write to be ordered with.

</details>
<details><summary>12 · Loom and Miri</summary>

Loom tries every interleaving of a small scenario, and the stale values that weak orderings allow, so it finds lost wake-ups and orderings that are too weak even when the bad run is very unlikely. Miri runs one interleaving per run, but it checks every memory access: it finds reads of freed or uninitialised memory, leaks, and data races on memory that loom does not track. Loom 0.7.3 does not model `SeqCst` loads and stores precisely, so correct code that relies on them can fail a model; it also wakes a thread out of `join` when it is unparked, and it cannot follow more than two threads that spin on one lock.

</details>
