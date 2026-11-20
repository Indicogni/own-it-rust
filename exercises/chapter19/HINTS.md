# Chapter 19 hints — Threads and shared state

[Chapter guide](README.md) · [All hints](../../HINTS.md)

Try hint 1 before opening hint 2.

## l01 · Fork, join, and worker failures

<details><summary>Hint 1</summary>

Collect join handles before joining, so all workers can make progress. A chain such as `.map(spawn).map(join)` without a `collect` in between is lazy: it starts and joins one worker at a time. `measure` is a `fn` pointer, which is `Copy`, so a `move` closure can simply use it.

</details>

<details><summary>Hint 2</summary>

The join result is Result<Result<u32, &str>, panic payload>. Map only the outer error, then return the inner result.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

A `JoinHandle<T>`'s `join` returns `thread::Result<T>`: the outer `Err` carries a panic payload (`Box<dyn Any + Send>`), and the inner `T` is whatever the closure returned, so a worker that returns `Result<u32, E>` produces `Result<Result<u32, E>, payload>`. Mapping only the outer error keeps the worker's own `Ok`/`Err` intact and turns a panic into an application value. A `move` closure takes ownership of each captured value, so the String travels to the new thread and is dropped there; `spawn` demands `'static` because the caller might return before the worker finishes. Dropping a handle detaches the thread instead of cancelling it, so you join every handle you need results from, in the order you spawned them, to report results in input order.

</details>

## l02 · Borrowed parallel work

<details><summary>Hint 1</summary>

Split before spawning; the compiler can prove the halves do not overlap.

</details>

<details><summary>Hint 2</summary>

thread::scope allows non-static borrows. Use scoped handles for returned sums, and mutate each half directly in separate closures. Spawn both workers before you join either. If a closure needs `move` to take its half, but should only borrow the callback, make a reference first (`let widen = &widen;`) and let `move` copy the reference.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`thread::scope` guarantees that every thread spawned inside it is joined before `scope` returns, so those threads may borrow locals that outlive the scope; no `'static` bound and no `Arc` are needed. The borrow checker still applies across threads: two closures may share `&[i32]`, but two `&mut` borrows must not overlap, which `split_at_mut` proves by producing two disjoint halves. Because the joins happen at the end of the scope, any borrow taken by a worker has ended once `scope` returns and the data is usable again. Join a scoped handle yourself when you need the value that the worker returned, or when you want to handle its panic as a `Result`. Threads that you do not join are joined at the end of the scope, and `thread::scope` then panics ("a scoped thread panicked") if any of them panicked.

</details>

## l03 · Ownership and channel closure

<details><summary>Hint 1</summary>

An iterator over a receiver ends only after all sender handles have been dropped and buffered messages consumed.

</details>

<details><summary>Hint 2</summary>

Use an explicit channel::<String>. Move the receiver and `work` into the worker; the worker's loop over the receiver ends when the sender is dropped, so `drop(tx)` comes before `join`. In drain_now match TryRecvError variants without retrying Empty.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

Messages from one sender arrive in the order they were sent (FIFO), but messages from several senders interleave in whatever order the scheduler produced, so a test must not assume a global order unless the values carry an index. `send` on an unbounded `channel` never blocks; it only means the value was queued, not processed. A `sync_channel(n)` blocks the sender when n messages are waiting, which bounds memory and slows a fast producer to the consumer's pace (backpressure); capacity 0 is a rendezvous. `recv` and the receiver's iterator report disconnection only after every sender is dropped and the buffer is empty, so a forgotten sender clone keeps a consumer waiting forever.

</details>

## l04 · Send and Sync contracts

<details><summary>Hint 1</summary>

Send is about moving a value across threads; Sync is about sharing references across threads.

</details>

<details><summary>Hint 2</summary>

Arc cloning changes ownership counts, not Vec contents. Clone before spawning, start both readers, then join both before returning the original. Inside a reader, `read(&data)` works because `&Arc<Vec<u8>>` converts to `&[u8]` by deref coercion.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`Send` means a value may be moved to another thread; `Sync` means `&T` may be shared, and `T: Sync` exactly when `&T: Send`. They are auto traits: the compiler implements them for a type when all of its fields have them, so `Rc` (non-atomic count) and `RefCell` (non-atomic borrow flag) opt out, and anything containing them loses the trait too. `Arc<T>` is `Send` and `Sync` only when `T: Send + Sync`, because every clone gives another thread `&T`. `Mutex<T>` adds `Sync` (it serializes access), but it still requires `T: Send`, because the thread that locks it can move the value out; a `Mutex<Rc<_>>` would let two threads touch one non-atomic count.

</details>

## l05 · Mutex invariants and poisoning

<details><summary>Hint 1</summary>

Keep a guard only while modifying the invariant: call `amount()` first and store the result in a local, then lock and add. PoisonError can return the guard for deliberate recovery.

</details>

<details><summary>Hint 2</summary>

Use into_inner on the poison error, overwrite the entire protected state, then clear_poison. Joining workers establishes completion before reading.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

The invariant is "the counter equals the number of increments that have completed", and it holds whenever the mutex is unlocked. A `MutexGuard` is the only way to reach the data, and the lock is released when the guard is dropped, so keep guards short and never hold one across slow work or a join. If a thread panics while holding the guard, the mutex is poisoned: later `lock` calls return `Err`, because the invariant might be half-updated, but the data is still there (`into_inner` on the error gives it back) and poisoning proves nothing about corruption. Deadlock comes from two threads taking two locks in opposite orders, or one thread locking a std `Mutex` it already holds (it is not reentrant); a single global acquisition order prevents the first. One more rule about guard lifetimes: a temporary guard in a `match` scrutinee is held through every arm, while in edition 2024 a temporary guard in an `if let` condition is released before the `else` block runs (older editions held it there too).

</details>

## l06 · RwLock and condition predicates

<details><summary>Hint 1</summary>

The mutex-protected Option is the state; the Condvar only helps wait efficiently. Two tests publish before anyone waits. A solution that treats the notification as the data waits forever there, and the test stops it after a time limit with a message. Other tests start the consumer first, so the wake path is exercised too, and one of them wakes the consumer while the slot is still empty.

</details>

<details><summary>Hint 2</summary>

while guard.is_none() { guard = cv.wait(guard).unwrap(); }. After the loop, take the value. For the snapshot, `read()` (not `write()`) lets other readers in while you clone.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

A `Condvar` is a wake-up mechanism, not storage: a `notify` with no waiter is lost, so the real state lives in the mutex-protected value and the waiter checks a predicate. `wait` atomically releases the mutex and blocks, then reacquires it before returning, which lets the producer take the lock and change the state. Waits may return spuriously, or after another consumer already took the value, so the check is a `while` loop (or `wait_while`), never an `if`. An `RwLock` lets many readers in at once but a writer needs exclusive access; you cannot upgrade a read guard, so drop it and take a write lock, then re-check.

</details>

## l07 · Atomic counters and local state

<details><summary>Hint 1</summary>

try_update (called fetch_update on older toolchains) accepts a checked update function and returns the old value on success.

</details>

<details><summary>Hint 2</summary>

`counter` is a `&'static LocalKey<Cell<u32>>`. Such a reference is `Copy` and `'static`, so every spawned `move` closure can use it. `counter.get()` and `counter.set(..)` act on the copy that belongs to the thread that calls them, so call them inside the worker. A thread-local is not shared mutation of one counter.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

Atomicity means a read-modify-write such as `fetch_add` happens as one indivisible step, so no increment is lost and every reserved ID is unique. Ordering is a separate question: `Relaxed` orders nothing except that one variable, so it is right for counters and ID allocation but cannot publish other data; that needs `Release` on the store and `Acquire` on the load, or a lock. Atomic arithmetic wraps on overflow (even in debug builds), which is why `reserve` uses a checked update loop (`fetch_update`/`try_update`). A `thread_local!` gives each thread its own independent value, so nothing is shared at all. A `static mut` counter would need an invariant nobody can enforce ("only one thread touches this"), which is why reading or writing it is `unsafe`.

</details>

## l09 · Retry loops, orderings, and one-time initialization

<details><summary>Hint 1</summary>

A: one call does it. `compare_exchange(0, id, ..)` returns `Ok(0)` when it wrote and `Err(owner)` when it did not; you only have to change the `Ok` value. B: keep the value you last saw in a `mut` variable. Each turn of the loop asks `next`, tries `compare_exchange_weak`, and on `Err(actual)` stores `actual` in that variable. C and D: ask of each of the four operations, "does another thread rely on this operation to see something *else*?".

</details>

<details><summary>Hint 2</summary>

B: `let mut current = cell.load(Relaxed); loop { let Some(new) = next(current) else { return Err(current) }; match cell.compare_exchange_weak(current, new, Relaxed, Relaxed) { Ok(previous) => return Ok(previous), Err(actual) => current = actual } }`. C and D: the reader uses `value` only because of what it read in `ready`. So the store to `ready` is `Release` and the load of `ready` is `Acquire`. The store and the load of `value` may both be `Relaxed`, as long as the store comes before the flag is set and the load comes after the flag was seen.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

Between a `load` and a later `store` any other thread may change the value, and the `store` then overwrites that change. `compare_exchange` writes only if the value is still the one you expect, so a change by another thread makes the exchange fail instead of being lost. The failure returns the value that is there now; the loop must continue from that value, because repeating the exchange with the old expectation can never succeed. The weak form may also fail for no reason, which costs one more turn and nothing else. For the flag: with `Relaxed` on both sides, seeing `ready == true` says nothing about `value`; the processor or the compiler may make the two stores visible to another thread in either order, so the reader could still load the initial 0. A `Release` store makes everything the writer did before it visible to any thread whose `Acquire` load reads that stored `true`. This is called a happens-before relation: the store to `value` happens before the `Release` store, which happens before the `Acquire` load, which happens before the load of `value`. So `value` itself needs no ordering. `SeqCst` on the flag would also be correct; it promises more than is needed. No test can observe the difference by running the code, which is why the lesson's `Flag` type records the ordering and the test checks the record. `OnceLock` and `LazyLock` solve the common case "write once, then read forever" without any of this reasoning: the first caller runs the initializer, callers that arrive in the meantime wait, and everyone gets a reference to the same value.

</details>

## l08 · Capstone: bounded parallel pipeline

<details><summary>Hint 1</summary>

A bounded sender can block until the receiver drains. Drop the original sender before collecting to EOF. `input.chunks(chunk_size)` already panics for a chunk size of 0. For task B, remember from Chapter 15 that an iterator of `Result<T, E>` can be collected into a `Result<Vec<T>, E>`, and that this stops at the first `Err`.

</details>

<details><summary>Hint 2</summary>

Spawn workers, consume rx inside the scope, then join workers. Tag values with input indices to reconstruct order without guessing a schedule.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

Each producer owns a clone of the `SyncSender` and calls `work` on the elements of its own chunk; the original is dropped right after spawning, so the receiver's iterator ends exactly when the last producer finishes and drops its clone. With capacity 2, a producer blocks once two values wait, so the collecting loop must run while the producers are alive; joining first would deadlock as soon as the buffer filled. The output order across producers is unspecified, so each value carries its input index and sorting by that index *is* the contract: it restores a meaning the data already had. Sorting output to make a flaky test pass, without an index that defines the order, would hide a bug instead. In `collect_results`, returning at the first `Err` drops the receiver, and producers see that as a failed `send`, which is why they must handle send errors instead of unwrapping.

</details>

## R01 · Spaced review of Chapters 14–18

<details><summary>Hint 1</summary>

A: `BufReader::new(source)` implements `BufRead` for any `Read`, and `lines()` yields `io::Result<String>`, so `?` inside the worker closure turns the first bad line into that source's error. The bounds `R: Read + Send + 'static` are exactly what lets a source move into a spawned thread. B: workers borrow the chunks and a shared `Mutex<BTreeMap>` through `thread::scope`; `chunk.as_ref()` gives the `&str`, and it must be called inside the worker. C: a `Box<dyn FnOnce() -> T + Send>` is itself `FnOnce() -> T + Send + 'static`.

</details>
<details><summary>Hint 2</summary>

A: spawn every worker first, collect the handles, then join them in order. B: count into a local `BTreeMap<&str, usize>` with `entry(..).or_default()`, then lock once and add each count into the shared map, allocating a String only for new keys if you like. C: `jobs.into_iter().map(thread::spawn).collect::<Vec<_>>()` starts them all; joining in order gives input order.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one reverses the results, one leaves stale cache entries behind. Then look at how many threads `map_parallel` starts, what `join().unwrap()` does when `f` panics, and what `refresh` holds while it calls `fetch`.

</details>
<details><summary>Hint 2</summary>

Return `Ok(vec![])` early for empty input, because `chunks(0)` panics. Otherwise split with `items.chunks(items.len().div_ceil(workers.max(1)))`, which gives at most `workers` chunks, and give each chunk one scoped worker that returns a `Vec<R>`. Join **every** handle before you look at any result: collect all the join results into a `Vec` first, and only then turn a panic into `Err("worker panicked")`. An early `?` on the first failed join leaves the other handles unjoined, and if one of those workers also panicked, `thread::scope` itself panics. In `refresh`, build the new map first (calling `fetch` with no lock held), then take the write lock only to swap it in.

</details>
<details><summary>Check your review</summary>

`map_parallel` pops handles from the end, so results come back reversed. It also ignores `workers` and spawns one thread per item, which is unbounded for large inputs, and `join().unwrap()` turns a worker panic into a panic in the caller instead of the documented `Err`. The repair has its own trap: a scoped thread that panicked and was never joined makes `thread::scope` panic when the scope ends, so every handle must be joined before the function returns its `Err`. `refresh` loops over indexes where an iterator would do (the clippy warning), holds the write lock for the whole loop, so every reader waits behind slow fetches, and because it only inserts, keys that are no longer wanted stay in the cache. The fix computes everything without the lock and replaces the map in one short critical section, which also makes the update atomic from a reader's point of view.

</details>

## Drills

Run `cargo drill ch19`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · Spawning a borrowing closure</summary>E0373. `thread::spawn` requires a `'static` closure, because the new thread may outlive `main`'s stack frame; the closure only borrows `words`. Add `move` so the vector moves into the thread, or use `thread::scope` to borrow it safely.</details>
<details><summary>d02 · Arc does not make RefCell thread-safe</summary>E0277: `RefCell<Vec<&str>>` is not `Sync`, so `Arc<RefCell<_>>` is not `Send`. Its borrow flag is a plain integer that two threads could update at once. Use `Arc<Mutex<Vec<&str>>>` and `lock()` instead of `borrow_mut()`.</details>
<details><summary>d03 · A Mutex moved into the first worker</summary>E0382: the first `move` closure takes ownership of the Mutex, so the second cannot use it. Share it with `Arc<Mutex<_>>` and move one `Arc` clone into each worker, or borrow it from both workers inside `thread::scope`.</details>
<details><summary>d04 · Two scoped workers, one mutable slice</summary>E0499: both closures capture `pixels` mutably, and the compiler does not reason about which index ranges they touch. `let (left, right) = pixels.split_at_mut(3);` produces two disjoint `&mut` slices, one per worker.</details>
<details><summary>d05 · Mutating through a non-mut Mutex</summary>Compiles and runs. `lock()` takes `&self`; the Mutex provides interior mutability, and the guard hands out `&mut` to one thread at a time. Each guard here is a temporary dropped at the end of its statement, so the next `lock()` does not deadlock.</details>
<details><summary>d06 · Returning a reference out of a guard</summary>E0515: the reference points into data reached through the guard, and the guard (and with it the lock) is dropped when the function returns. Return an owned value (`.clone()`), return the `MutexGuard` itself, or take a closure that runs while the lock is held.</details>
<details><summary>d07 · A Cell in a static</summary>E0277: every thread can reach a `static`, so its type must be `Sync`, and `Cell` is not. Use `AtomicU32` (`fetch_add(1, Relaxed) + 1`) for a process-wide counter, or `thread_local!` if each thread should count separately.</details>
<details><summary>d08 · Condvar::wait takes the guard</summary>E0382: `wait` consumes the guard (it must unlock the mutex while sleeping) and returns a new one when it wakes. Reassign it: make the binding `mut` and write `guard = changed.wait(guard).unwrap();`, or use `wait_while`.</details>
<details><summary>d09 · Reading after a worker panicked</summary>Panics at run time: the worker panicked while holding the guard, so the mutex is poisoned and `lock().unwrap()` panics on the `PoisonError`. The data is intact (it contains the pushed 2). Recover deliberately with `lock().unwrap_or_else(PoisonError::into_inner)` when the invariant can be re-checked.</details>
<details><summary>d10 · Sending after the receiver is gone</summary>Panics at run time: the worker received one value and returned, dropping the receiver, so the second `send` returns `Err(SendError(2))` and `unwrap` panics. A failed send is how a producer learns that the consumer stopped; handle it (`if tx.send(2).is_err() { /* stop producing */ }`).</details>

## Mastery check: model answers

Write your own answers in NOTES.md first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**1. When can scoped threads borrow data that spawn cannot?**
Always when the data lives longer than the scope. `thread::scope` joins every thread that was spawned in it before it returns, so those threads cannot outlive any variable that was declared before the scope began. They may borrow such variables: shared (`&T`, if `T: Sync`) from several threads, or mutably from one thread, or mutably in disjoint parts made with `split_at_mut` or `chunks_mut`. `thread::spawn` makes no promise about when the thread ends, so its closure must be `'static`: it must own its data, often through `move` and `Arc`.

**2. How do join errors differ from a worker Result error?**
`join` returns `Err(payload)` only when the thread panicked; the payload is the panic value, a `Box<dyn Any + Send>`. A worker that *returns* `Err(e)` finished normally, and `join` gives `Ok(Err(e))`. The outer result answers "did the thread run to its end?", the inner one "did the work succeed?". Map the outer error to an application error if the caller was promised one, and pass the inner result on unchanged.

**3. Which values must be Send, and when does Arc<T> require Sync?**
Everything that a spawned closure captures, and the value it returns, must be `Send`, because they move to or from the other thread. The same holds for a value sent through a channel. A captured `&T` is `Send` only if `T: Sync`. `Arc<T>` is `Send` (and `Sync`) only if `T: Send + Sync`: after you send one clone, two threads hold a pointer to the same `T`, so they share it through `&T`, which needs `Sync`; and the last clone to be dropped destroys `T` on whichever thread that is, which needs `Send`.

**4. How do channel closure, bounded backpressure, and a forgotten sender affect termination?**
A receiving loop ends when the queue is empty and every `Sender` has been dropped. A sender that is forgotten (usually the original, in the thread that receives) keeps the channel open, and the loop waits forever. A bounded channel makes `send` wait while the queue is full, so producers need a consumer that is receiving at the same time; joining the producers before draining is a deadlock. In the other direction, a dropped receiver makes `send` fail, and that failure is how producers learn that they should stop.

**5. What invariant is a mutex protecting; why does a Condvar wait in a loop?**
The invariant is a statement about the protected value that is true whenever the lock is free, for example "the counter equals the number of completed increments" or "the two balances add up to 100". A thread may break it only while it holds the guard. A `Condvar` wait sits in a loop because waking up proves nothing: the wake-up may be spurious, another thread may have consumed the state first, and a notification sent before the wait began was never stored. Only the state under the mutex is information, so it is checked before the first wait and again after every wake-up.

**6. Which guarantees does relaxed atomic arithmetic provide, and which does it not provide?**
It guarantees that each operation is indivisible: `fetch_add` loses no update, and every thread sees values that were really stored, in an order that is consistent for that one atomic. It does not guarantee anything about other memory: seeing a new value of a `Relaxed` atomic does not mean that other writes of the same thread are visible. It also does not check for overflow; atomic integers wrap. And two separate atomics are not updated together.

**7. How can you design concurrent tests around completion signals rather than sleeps?**
Make the event that you wait for send a signal, and wait for the signal with a time limit. A worker sends on a channel when it has started or finished, and the test calls `recv_timeout`; a `Barrier` makes several threads reach a point together; a thread ID or a counter recorded inside a callback shows where and how often the work ran; `try_lock` or `try_read` shows whether a lock is held at a given moment. A correct program then passes at any speed, and a deadlock becomes a failed test with a message. A sleep only makes a failure less likely, and makes every run slower.

**8. What does a retry loop do after a failed compare-exchange, and which pair of orderings lets a flag announce other data?**
It takes the value that the failed exchange returned, computes the new value again from it, and tries again; nothing was written by the failed attempt. A flag announces other data when its store uses `Release` and its load uses `Acquire`: a reader that sees the flag then also sees everything the writer wrote before setting it.

**The capstone tradeoff.** One example: the channel capacity. A capacity of 2 keeps at most two results in memory, however large the input is, but producers spend much of their time blocked, and each `send` wakes the collector. A larger capacity lets producers run ahead and reduces the switching between threads, at the cost of memory. An unbounded channel removes the deadlock risk of "join before drain" and also removes the limit on memory. A case that the supplied tests do not exercise: a `work` function that panics in one producer. Decide what your pipeline should do then, and test it.

</details>
