# Mastery M4 concepts — An async runtime from scratch, then a real service

[Module guide](README.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Read one section, then do its lesson. Every `rust` block on this page is compiled and run by the course checks (`cargo test --doc`), so you can paste any of them into a file and change it. The two blocks of lesson 09 need tokio; they are taken from `examples/l09_shutdown.rs`, which is compiled and run in the same way.

## What you will be able to do

Chapter 21 gave you one future, one waker and a `block_on` that parks a thread. A runtime is what you get when you ask three more questions. Who polls *many* futures, and in which order? Who calls the waker when the thing a future waits for is a clock or a socket, and not another task? And what happens to a future that is thrown away halfway?

After this module you will be able to:

- build a `Waker` from a data pointer and a table of four functions, and say who owns what in each of the four;
- write an executor with a run queue, `spawn`, `JoinHandle`, abort and an orderly shutdown;
- write a timer driver and a reactor, and explain why neither can lose a wakeup;
- store a future inside another type without a `Box`, and write down why the `unsafe` code that this needs is sound;
- write streams, `select` and a cancellation token, and judge whether an `async` function is safe to cancel;
- build a network service that bounds its queues, limits its connections and shuts down cleanly, first on your own runtime and then on tokio.

## Before you start

Chapters 19, 20, 21 and 23 of this course, and Chapter 18 for the socket lessons. Lesson 09 also assumes lesson E06 of the ecosystem track. The module guide lists what each one is needed for.

## Time

These are estimates for reading the section, doing the lesson, and writing your notes.

| Lesson | Estimate |
| --- | --- |
| 01 RawWaker and its vtable | 1 h 30 min |
| 02 Tasks, run queue, spawn, JoinHandle | 3 h |
| 03 A timer heap and sleep | 2 h 30 min |
| 04 A reactor over non-blocking sockets | 2 h 30 min |
| 05 Pin projection by hand | 2 h 30 min |
| 06 Streams, a channel, adapters | 3 h |
| 07 Select and cancellation safety | 2 h 30 min |
| Drills and C01 | 2 h |
| 08 Capstone: chat server | 4 to 6 h |
| 09 The same service on tokio | 3 h |

## Words used on every page

A *task* is a future that the executor owns and polls until it is finished. A *driver* is a thread that waits for something outside the program (time, a socket) and wakes tasks. The *lock rule* is introduced in lesson 03 and holds from then on: while you hold a lock, call no code that you did not write, which means no `wake` and no drop of a `Waker`.

## Lesson 01 · RawWaker and its vtable, by hand

**What is assumed.** You have implemented `Wake` for a type in an `Arc` (Chapter 21, lesson 02) and you know `Arc::into_raw` and `Arc::from_raw` (Chapter 23).

**The model.** A `Waker` is two machine words. The first is a *data pointer*: an untyped `*const ()` that says *whom* to wake. The second points to a *vtable*: a table of four functions that say *how*. The standard library does not know what the data pointer means. Only your four functions do.

```text
Waker
+-----------+      +------------------- RawWakerVTable --------------------+
| data  ----|--+   | clone(data) -> RawWaker   wake(data)                  |
| vtable ---|--|-> | wake_by_ref(data)         drop(data)                  |
+-----------+  |   +-------------------------------------------------------+
               +-> whatever your functions agree on: an Arc's pointee,
                   a static, or a plain number
```

Each `Waker` value *owns* one unit of whatever the data pointer stands for. With an `Arc` behind it, that unit is one strong reference. The contract of the four functions is easiest to remember as a table of what happens to that unit:

| Function | Called by | The caller's unit | Must do |
| --- | --- | --- | --- |
| `clone` | `Waker::clone` | stays with the caller | create one more unit for the returned `RawWaker` |
| `wake` | `Waker::wake(self)` | is handed over | notify, then release the unit |
| `wake_by_ref` | `Waker::wake_by_ref(&self)` | stays with the caller | notify only |
| `drop` | dropping a `Waker` | is handed over | release the unit, do not notify |

`Waker::wake` takes `self` and then *forgets* it: the library does not call your `drop` afterwards. So after `wake`, nobody but your `wake` function can release the unit.

Two more rules complete the contract. `Waker` is `Send + Sync`, so the four functions may be called from any thread, at the same time. And the `RawWaker` that your `clone` returns must keep the contract too: its table must fit its data pointer. Return the same data pointer and the same vtable. That is the simple way to be right, and it is what makes `will_wake` say `true` for a waker and its clone.

**The tools for an `Arc`.** Four operations move between an `Arc<T>` and a raw pointer. The function that builds the waker uses the first, and each vtable function uses one of the other three. `Arc::into_raw(arc)` gives a `*const T` and does *not* lower the count: the reference now belongs to whoever holds the pointer. `Arc::from_raw(ptr)` is the way back and takes that reference over; when the `Arc` it returns is dropped, the count goes down. `Arc::increment_strong_count(ptr)` adds a reference without creating a value. `&*ptr` borrows the `T` and leaves the count alone. The last three are `unsafe`: each needs a pointer that came from `into_raw` and a count that is still above zero.

**The safe way first.** `impl Wake for T` plus `Waker::from(Arc<T>)` writes exactly this vtable for you: `clone` raises the strong count, `wake` calls your `wake(self: Arc<Self>)`, `drop` lowers the count. You write the table by hand in three cases: there is no allocator (embedded code), the tasks live in a `static` and need no counting, or the reference count lives somewhere else, for example in a task header that the executor manages.

**A worked example: the data pointer is a number.** A small runtime with a fixed table of at most 64 tasks does not need a pointer at all. The "pointer" is the task's index, and waking sets one bit in a mask that the executor reads.

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{RawWaker, RawWakerVTable, Waker};

/// Bit `i` is set when task number `i` wants to be polled.
static READY: AtomicU64 = AtomicU64::new(0);

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, drop_waker);

/// A waker for task number `index`. The "pointer" is the number itself.
fn task_waker(index: usize) -> Waker {
    assert!(index < 64, "this toy runtime has 64 task slots");
    let data = std::ptr::without_provenance::<()>(index);
    // SAFETY: the functions in VTABLE never dereference `data`. They read it
    // back as a number below 64. Nothing is owned, so `clone` may copy the
    // word and `drop` has nothing to release. All they touch is an atomic
    // static, so any thread may call them at any time.
    unsafe { Waker::from_raw(RawWaker::new(data, &VTABLE)) }
}

fn clone(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

/// Serves as `wake` and as `wake_by_ref`: there is no unit to release.
fn wake(data: *const ()) {
    READY.fetch_or(1 << data.addr(), Ordering::SeqCst);
}

fn drop_waker(_data: *const ()) {}

fn main() {
    let three = task_waker(3);
    let five = task_waker(5);
    five.wake_by_ref();
    let copy = three.clone();
    copy.wake();
    drop(three);
    assert_eq!(READY.swap(0, Ordering::SeqCst), 0b10_1000);
    assert_eq!(size_of::<Waker>(), 2 * size_of::<usize>());
}
```

Three things to notice. The four functions are ordinary safe `fn` items; a safe function can be used where an `unsafe fn` pointer is expected. The one `unsafe` block is where you *promise* that the table and the data fit together, so that is where the argument goes. And the vtable is a `static`: it has one address for the whole program.

**Mistakes people really make.**

*A function with the wrong shape.* The compiler checks the signatures, not the meaning:

```rust,compile_fail,E0308
use std::task::RawWakerVTable;

unsafe fn clone(_data: *const ()) {}
unsafe fn noop(_data: *const ()) {}

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
```

```text
error[E0308]: mismatched types
  |
6 | static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
  |                                 ------------------- ^^^^^ expected fn pointer, found fn item
  |
  = note: expected fn pointer `unsafe fn(_) -> RawWaker`
                found fn item `unsafe fn(_) -> () {clone}`
```

*`wake` that does not release.* If `wake` only notifies (a common shortcut is to call `wake_by_ref` from it), every wake by value leaks one reference, and the task behind the waker is never freed. No test that only counts wakes notices. Miri does:

```text
error: memory leaked: alloc304 (Rust heap, size: 24, align: 8), allocated here:
    |
441 |           let x: Box<_> = Box::new(ArcInner {
```

*`wake_by_ref` that releases.* The opposite shortcut, one function that rebuilds the `Arc` with `from_raw` for both entries, takes a unit that the waker still owns. The second call then touches freed memory. This is undefined behaviour; the program may seem to work. Miri reports:

```text
error: Undefined Behavior: in-bounds pointer arithmetic failed: alloc303 has been freed, so this pointer is dangling
help: alloc303 was deallocated here:
   |
15 | }
   | ^
```

*A vtable in a `const`.* `Waker::will_wake` compares the data pointers and the *addresses* of the two vtables. A `const` has no address of its own: each use may get a separate copy, so two wakers for the same task can compare as different. Nothing breaks, but code that uses `will_wake` to avoid cloning (you will write such code from lesson 02 on) loses its shortcut. Use a `static`.

**When to use which.**

| Situation | Use |
| --- | --- |
| The wake target lives in an `Arc` | `impl Wake` and `Waker::from`. No `unsafe`. |
| A waker that does nothing (a test, a single poll) | `Waker::noop()` |
| No allocator, static tasks, or your own reference count | `RawWaker` by hand, with a SAFETY argument |

**Read more.**
[`std::task::RawWaker`](https://doc.rust-lang.org/std/task/struct.RawWaker.html) ·
[`RawWakerVTable`](https://doc.rust-lang.org/std/task/struct.RawWakerVTable.html) (the contract of each function) ·
[`Wake`](https://doc.rust-lang.org/std/task/trait.Wake.html) ·
[`Waker::will_wake`](https://doc.rust-lang.org/std/task/struct.Waker.html#method.will_wake) ·
[`Arc::increment_strong_count`](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.increment_strong_count)

## Lesson 02 · Tasks, a run queue, spawn and JoinHandle

**What is assumed.** Lesson 01, `block_on` from Chapter 21, and `Mutex` with `Condvar` from Chapter 19.

**The model.** An executor for many futures needs four things.

1. A **task**: a future in a box, so that futures of different types fit into one queue, plus a few flags that the executor keeps for it.
2. A **run queue**: the tasks that are ready to be polled.
3. A **waker per task** whose only job is: put this task on the run queue.
4. A **loop**: take a task from the queue, poll it once, repeat. When the queue is empty, sleep until something is pushed.

That is all an executor does. It never knows *why* a task is pending. A task that waits for a timer, a socket or another task has given its waker to whoever will know. The waker is the whole interface between the executor and the rest of the world, which is why lessons 03 and 04 can add timers and sockets without changing a line of the executor.

**A worked example: the smallest executor for several tasks.** The run queue is a channel, and each task holds a sender to it.

```rust
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

struct Task {
    future: Mutex<Option<BoxFuture>>,
    queue: Sender<Arc<Task>>,
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        // Waking means: put me on the run queue.
        let queue = self.queue.clone();
        let _ = queue.send(self);
    }
}

async fn yield_now() {
    let mut yielded = false;
    std::future::poll_fn(|cx| {
        if yielded {
            return Poll::Ready(());
        }
        yielded = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    })
    .await
}

fn main() {
    let (queue, ready) = mpsc::channel::<Arc<Task>>();
    let log = Arc::new(Mutex::new(Vec::new()));
    for name in ["a", "b"] {
        let log = Arc::clone(&log);
        let future: BoxFuture = Box::pin(async move {
            for step in 1..=2 {
                log.lock().unwrap().push(format!("{name}{step}"));
                yield_now().await;
            }
        });
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            queue: queue.clone(),
        });
        queue.send(task).unwrap();
    }
    // From here on only tasks and their wakers hold senders.
    drop(queue);

    // The executor loop. When every task is gone, every sender is gone, and
    // `recv` returns an error.
    while let Ok(task) = ready.recv() {
        let future = task.future.lock().unwrap().take();
        if let Some(mut future) = future {
            let waker = Waker::from(Arc::clone(&task));
            let mut cx = Context::from_waker(&waker);
            if future.as_mut().poll(&mut cx).is_pending() {
                *task.future.lock().unwrap() = Some(future);
            }
        }
    }
    assert_eq!(*log.lock().unwrap(), ["a1", "b1", "a2", "b2"]);
}
```

It runs, and it shows the shape. It also leaves out everything that makes the lesson: a task that is woken three times sits in the queue three times; nobody can wait for a task's result; there is no future that the caller itself waits for; and a task that is still pending when the program wants to stop is never dropped in an orderly way.

**Who owns whom.** Draw this before you write code, because every shutdown bug is an ownership bug.

```text
Executor ---strong---> Shared { queue, tasks, closed }
Spawner  ---strong---> Shared
Shared.tasks ---strong---> Task { future, queued, aborted }   (the owner of every live task)
Shared.queue ---strong---> Task                                (only while it is queued)
Waker    ---strong---> Task
Task     ----weak----> Shared
JoinHandle --strong--> Join state <--strong-- the task's future
JoinHandle ---weak---> Task
```

The table `tasks` is what makes shutdown possible: it reaches a task that is parked somewhere with only a waker pointing at it. `Executor::drop` empties the table and drops each future. That drops everything the futures own, including spawners and sockets, so no cycle of strong arrows survives. Wakers that still exist then point at empty tasks, and waking them does nothing.

**The invariants of the lesson's design.**

- *A task is in the queue at most once.* `Task::queued` is set when the task is pushed and cleared when the executor takes it out, *before* the poll. A wake during the poll therefore queues the task again; a wake while it is queued does nothing.
- *The check and the sleep are one critical section.* The executor looks at the queue and waits on the condition variable under one lock, and every push takes that lock. A push cannot fall into the gap between "the queue is empty" and "go to sleep". This is the same argument as for the completion cell of Chapter 21.
- *Only one thread polls.* `block_on` takes `&mut self`. The compiler therefore proves that two threads never run the tasks of one executor at the same time, and taking a future out of its slot for the poll is safe.
- *No lock is held during a poll or a drop of a future.* A future may wake, spawn, or drop a handle, and each of those needs a lock.

**Why `Send + 'static`.** The executor polls on one thread, but a `Waker` must be `Send + Sync`, and the waker is an `Arc<Task>`. So `Task`, and the future inside it, must be safe to reach from other threads: `Send`. And a task may live longer than the function that spawned it, so it may not borrow from that function: `'static`. The compiler reports both, each in its own way:

```rust,compile_fail
use std::future::Future;
use std::rc::Rc;

fn spawn<F: Future + Send + 'static>(_future: F) {}

fn main() {
    spawn(async {
        let name = Rc::new(String::from("task"));
        std::future::ready(()).await;
        println!("{name}");
    });
}
```

```text
error: future cannot be sent between threads safely
   |
   = help: within `{async block@src/main.rs:7:11: 7:16}`, the trait `Send` is not implemented for `Rc<String>`
note: future is not `Send` as this value is used across an await
   |
 8 |         let name = Rc::new(String::from("task"));
   |             ---- has type `Rc<String>` which is not `Send`
 9 |         std::future::ready(()).await;
   |                                ^^^^^ await occurs here, with `name` maybe used later
```

The fix is to end the `Rc`'s scope before the `.await` (an inner block), or to use an `Arc`. Calling `drop(name)` before the `.await` is not enough for this compiler: it decides by scope. Drill d03 lets you try it.

```rust,compile_fail,E0373
use std::future::Future;

fn spawn<F: Future + Send + 'static>(_future: F) {}

fn main() {
    let greeting = String::from("hello");
    spawn(async { println!("{greeting}") });
}
```

```text
error[E0373]: async block may outlive the current function, but it borrows `greeting`, which is owned by the current function
  |
7 |     spawn(async { println!("{greeting}") });
  |           ^^^^^              -------- `greeting` is borrowed here
  |           |
  |           may outlive borrowed value `greeting`
  |
help: to force the async block to take ownership of `greeting` (and any other referenced variables), use the `move` keyword
  |
7 |     spawn(async move { println!("{greeting}") });
  |                 ++++
```

**Two more mistakes, which the compiler does not see.**

*Dropping a future under the queue lock.* It looks harmless until a destructor inside the future spawns a cleanup task or wakes someone. Then the thread locks the same mutex twice and stops for ever. Move the future out of the critical section first, then drop it.

*Aborting by dropping the future directly.* `abort` may be called from any thread, also from inside the task itself. Dropping the future there would run its destructors on the wrong thread, or while it is being polled. Set a flag and schedule the task; the executor thread drops the future when the task's turn comes.

**When to use which.**

| You want | Use |
| --- | --- |
| The result of a task | `handle.await` |
| A task that runs on its own | drop the handle (the task is *detached*) |
| To stop a task | `handle.abort()`, then `handle.await` to see `Cancelled` |
| To stop all tasks | drop the executor |

**Read more.**
[`std::task::Wake`](https://doc.rust-lang.org/std/task/trait.Wake.html) (its example is a `block_on`) ·
[`Condvar`](https://doc.rust-lang.org/std/sync/struct.Condvar.html) ·
[`Weak`](https://doc.rust-lang.org/std/sync/struct.Weak.html) ·
[`Send` and `Sync`](https://doc.rust-lang.org/nomicon/send-and-sync.html) in the Rustonomicon ·
[E0373](https://doc.rust-lang.org/error_codes/E0373.html)

## Lesson 03 · A timer heap and sleep

**What is assumed.** Lesson 02. `BinaryHeap` and `Condvar::wait_timeout` from the standard library.

**The model.** `sleep(d).await` cannot block the thread, because other tasks need it. So the future does three things: it works out its *deadline*, it hands `(deadline, waker)` to a **timer driver**, and it returns `Pending`. The driver is a thread with a set of such entries. It sleeps until the earliest deadline, takes every entry that is due, and wakes the wakers. The woken task polls its `Sleep` again, which looks at the clock and returns `Ready`.

The data structure must answer "which deadline is next?" quickly, so it is a *priority queue*. Three operations matter: insert (every sleep), remove the earliest (every time a timer fires), and cancel (every time a sleep is dropped early, which in a server is most of them: a timeout that does not fire is cancelled).

**A worked example: timers in an ordered map.** A `BTreeMap` keyed by `(deadline, sequence number)` is a priority queue that can also remove any entry directly.

```rust
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

fn main() {
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);

    // The sequence number makes equal deadlines different keys, and keeps
    // them in insertion order.
    let mut timers: BTreeMap<(Instant, u64), &str> = BTreeMap::new();
    timers.insert((at(300), 0), "retry upload");
    timers.insert((at(100), 1), "blink cursor");
    timers.insert((at(100), 2), "poll sensor");
    timers.insert((at(200), 3), "request timeout");

    // Cancel: with the key in hand it is one removal, and nothing stays behind.
    assert_eq!(timers.remove(&(at(200), 3)), Some("request timeout"));

    // How long may the driver sleep?
    let (&(next, _), _) = timers.first_key_value().unwrap();
    assert_eq!(next, at(100));

    // Fire everything that is due at `now`.
    let now = at(150);
    let mut fired = Vec::new();
    while let Some(entry) = timers.first_entry() {
        if entry.key().0 > now {
            break;
        }
        fired.push(entry.remove());
    }
    assert_eq!(fired, ["blink cursor", "poll sensor"]);
    assert_eq!(timers.len(), 1);
}
```

The lesson asks for the same contract and leaves the structure to you. The other classic choice is a `BinaryHeap`, which is smaller and faster for insert and pop, but cannot remove from the middle. A heap therefore needs *lazy deletion*: `cancel` only marks the timer as dead (for example by removing its waker from a side table), and the dead entry is skipped when it reaches the top.

```text
heap:  (100, id 1) (100, id 2) (200, id 3) (300, id 0)      ordered by (deadline, id)
live:  { 0: waker, 1: waker, 2: waker }                     id 3 was cancelled
        an id that is in the heap and not in `live` is dead: skip it
```

Lazy deletion has a cost that you should be able to name: dead entries stay in memory until their deadline comes. A server that sets a 30-second timeout on every request and cancels nearly all of them carries 30 seconds of dead entries. The usual answers are to rebuild the heap when dead entries are the majority, or to use a structure with real removal. Production runtimes use a *timer wheel*: an array of buckets indexed by time, where insert and cancel are both constant time.

**The lock rule.** The driver thread and every `Sleep` share the timer state behind a mutex. The rule that keeps this safe is short:

> While the lock is held, call no code that you did not write. No `wake`. No drop of a `Waker`.

A waker is foreign code: its `wake` and its `drop` are whatever the executor's author wrote. A `wake` may push to a queue, but it may also run a callback that starts a new timer, and that needs your lock. A `drop` may free the last reference to a task, whose destructor drops another `Sleep`, which cancels its timer, which needs your lock. In both cases one thread waits for itself.

```text
driver thread                         what goes wrong
-------------                         ---------------
lock(timers)
  take due wakers
  waker.wake()   ------------------>  the woken code calls timers.sleep(..)
                                        lock(timers)      <- same thread, same lock: stuck
```

So the driver *moves* the due wakers out of the lock and wakes them afterwards. `Sleep` does the same with a waker it replaces or cancels: take it out, release the lock, then let it drop. The lesson's API is shaped for this: `take_due` and `cancel` hand wakers back instead of dropping them.

What about `clone`? It is foreign code too: the third function of the vtable. The rule names only `wake` and `drop` because those two are where real work happens. A wake runs a scheduler, and a drop may free a whole task with everything it owns. A clone only adds a reference, and a waker whose clone called back into a timer or a channel would break every runtime there is. So the usual practice, also in the model solutions, is to clone the waker of `cx` under the lock, where the comparison with the stored waker happens. If you want the rule without an exception, clone before you lock, and drop the clone after the lock when it was not needed.

**Why `Sleep` reads the clock.** A wake is a hint, not a fact. A task can be polled for many reasons: another future in the same task became ready, or an executor polled once more to be safe. If `Sleep` returned `Ready` because "I was woken", it could complete early. So `poll` compares `Instant::now()` with the deadline every time, and trusts nothing else.

This check also closes the one race that timers have. A `Sleep` reads the clock, sees that the deadline is still in the future, and registers; between those two steps the deadline may pass. The entry is then already due when it is inserted. The insert tells the driver that the earliest deadline has changed, the driver looks, finds it due, and wakes. The wake is late by microseconds, and never lost.

**Mistakes people really make.**

*A heap of wakers.* The first idea is `BinaryHeap<(Instant, Waker)>`. A heap must compare its elements, and a waker cannot be compared:

```rust,compile_fail,E0277
use std::collections::BinaryHeap;
use std::task::Waker;
use std::time::Instant;

fn main() {
    let mut timers = BinaryHeap::new();
    timers.push((Instant::now(), Waker::noop().clone()));
}
```

```text
error[E0277]: the trait bound `Waker: Ord` is not satisfied
  |
7 |     timers.push((Instant::now(), Waker::noop().clone()));
  |            ^^^^ the trait `Ord` is not implemented for `Waker`
  |
  = note: required for `(Instant, Waker)` to implement `Ord`
```

Order the entries by `(deadline, id)` and keep the wakers elsewhere. The id also decides between equal deadlines, so the order is the same on every run.

*The latest timer first.* `BinaryHeap` is a max-heap: `pop` returns the largest element. A driver built on it fires the timer that is furthest away. Wrap the entries in `std::cmp::Reverse`.

*A deadline that cannot be computed.* "Never" is tempting to write as `Instant::now() + Duration::MAX`:

```rust,should_panic
use std::time::{Duration, Instant};

fn main() {
    let never = Instant::now() + Duration::MAX;
    println!("{never:?}");
}
```

```text
thread 'main' (126266716) panicked at src/main.rs:4:17:
overflow when adding duration to instant
```

(The number after the thread's name is its id; it differs from run to run.)

Use `Instant::checked_add` when a duration comes from outside, and model "no deadline" as `Option<Instant>`.

**When to use which.**

| Structure | Insert | Next deadline | Cancel | Fits |
| --- | --- | --- | --- | --- |
| `BinaryHeap` with lazy deletion | O(log n) | O(1) | O(1), entry stays | few cancellations |
| `BTreeMap<(deadline, id), _>` | O(log n) | O(log n) | O(log n), entry gone | many cancellations, simple code |
| Timer wheel | O(1) | O(1) amortised | O(1) | production runtimes |

And the driver: a thread of its own, as here, is the simplest design. Real runtimes have no timer thread. The executor's "sleep while the queue is empty" simply gets a time limit, the next deadline. The data structure and the rules are the same.

**Read more.**
[`BinaryHeap`](https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html) (its page shows a min-heap) ·
[`Reverse`](https://doc.rust-lang.org/std/cmp/struct.Reverse.html) ·
[`BTreeMap::first_entry`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html#method.first_entry) ·
[`Condvar::wait_timeout`](https://doc.rust-lang.org/std/sync/struct.Condvar.html#method.wait_timeout) ·
[`Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html) (what "monotonic" promises)

## Lesson 04 · A reactor over non-blocking sockets

**What is assumed.** Lesson 03, and `TcpListener` and `TcpStream` on loopback from Chapter 18. Chapter 21, lesson 05, modelled readiness with a flag; here the sockets are real.

**The model.** A socket in *non-blocking mode* never makes a thread wait. A `read` that has no data returns at once with an error of kind `WouldBlock`. That error is not a failure. It means: not now. The question of asynchronous I/O is what happens next.

- *Try again in a loop* burns a whole CPU core while nothing happens.
- *Sleep, then try again* wastes less, but every task still pays for every tick.
- *Ask someone to tell you when to try again*, and return `Pending`. That someone is the **reactor**.

Every asynchronous I/O operation in every Rust runtime has the same body, and the lesson calls it `poll_io`:

```text
poll_io(cx, op):
    loop:
        match op():                          the non-blocking system call
            WouldBlock  => register cx.waker() with the reactor; return Pending
            Interrupted => continue          a signal arrived during the call; try again
            anything else => return Ready(it)    success or a real error
```

`accept`, `read` and `write` are thin wrappers around it. The futures themselves have no state: the socket's state is in the operating system, and every poll simply asks again.

**What a reactor is, and which one you build.** A production reactor asks the operating system: here are 10 000 sockets, tell me which of them became readable or writable. The system calls for that are `epoll` on Linux, `kqueue` on macOS and the BSDs, and IOCP on Windows. The reactor thread sleeps inside that call and wakes exactly the tasks whose sockets are ready.

The standard library has no portable interface to those calls. So the reactor of this lesson *polls*: it cannot know which socket is ready, and therefore wakes every registered task one tick (one millisecond) after it registered. Each woken task tries its operation again; most find `WouldBlock` again and register again.

```text
task                         reactor thread                    kernel
----                         --------------                    ------
read() -> WouldBlock
register(waker); Pending
                             wait while no waker is registered
                             wait one TICK
                             take all wakers, unlock, wake
polled again:
read() -> WouldBlock                                           (still nothing)
register(waker); Pending
                             ... one TICK ...                  data arrives
                             take all wakers, unlock, wake
polled again:
read() -> Ok(n)
```

This costs work in proportion to the number of *waiting* tasks, every tick, where a real reactor costs in proportion to the number of *ready* sockets. It also adds up to a millisecond of delay. What it shares with a real reactor is everything that matters for the code above it: the `poll_io` contract, the registration, the waker, the lock rule. Replacing the tick with `epoll` changes the inside of the reactor thread and nothing else.

**Why no wakeup is lost.** Look at the gap between `op()` returning `WouldBlock` and `register`. Data can arrive exactly there. With this reactor the argument is one sentence: every registered waker is woken at the next tick, whatever happened in between, and the task then reads the data. With a real reactor the same gap exists, and it is closed by asking the kernel in a way that still reports data that was already there (a *level-triggered* registration), or by trying the operation once more after registering.

**A worked example: `WouldBlock`, and "try again later" as a loop.** This program uses UDP, where one `send_to` is one `recv_from`, so that only the non-blocking part is new.

```rust
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

fn main() -> std::io::Result<()> {
    let receiver = UdpSocket::bind("127.0.0.1:0")?;
    let sender = UdpSocket::bind("127.0.0.1:0")?;
    receiver.set_nonblocking(true)?;

    let mut buf = [0u8; 16];
    // Nothing was sent. A blocking socket would wait here; this one answers
    // at once, with an error that means "not now".
    let early = receiver.recv_from(&mut buf).unwrap_err();
    assert_eq!(early.kind(), ErrorKind::WouldBlock);

    sender.send_to(b"ping", receiver.local_addr()?)?;

    // "Try again later", written as a loop. A task does one attempt per
    // poll, and a reactor decides when "later" is.
    let give_up = Instant::now() + Duration::from_secs(5);
    let (len, from) = loop {
        match receiver.recv_from(&mut buf) {
            Ok(received) => break received,
            Err(error) if error.kind() == ErrorKind::WouldBlock && Instant::now() < give_up => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(error),
        }
    };
    assert_eq!(&buf[..len], b"ping");
    assert_eq!(from, sender.local_addr()?);
    Ok(())
}
```

**Facts about TCP that the wrappers must respect.**

- *A write may be partial.* `write` returns how many bytes the kernel accepted, which can be fewer than you offered when its buffer is nearly full. `write_all` is a loop over `write`. A `write` that returns `Ok(0)` for a non-empty buffer will never make progress; report it as `ErrorKind::WriteZero`.
- *A read of zero bytes is the end.* `Ok(0)` for a non-empty buffer means the peer closed its sending side. It is not `WouldBlock`, and it stays `Ok(0)` on every later read.
- *Ready does not mean a message.* TCP is a stream of bytes. One `read` may return half a line or three lines. Lesson 07 builds the line reader.
- *The mode of an accepted socket depends on the system.* Linux does not copy the non-blocking mode of the listener to the socket that `accept` returns; macOS and the BSDs do. Code that forgets `set_nonblocking(true)` on the accepted stream therefore works on a Mac and blocks the whole executor on Linux. Set the mode yourself, always.

**Mistakes people really make.**

*Treating `WouldBlock` as an error.* The first `unwrap` on a non-blocking socket ends the program:

```text
called `Result::unwrap()` on an `Err` value: Os { code: 35, kind: WouldBlock, message: "Resource temporarily unavailable" }
```

(The number and the text come from the operating system; this is macOS. Match on `error.kind()`, never on the number.)

*Waking yourself.* `cx.waker().wake_by_ref()` before `Pending` makes every test pass that only checks the result. It is the busy loop from the top of this section, hidden inside an executor. The lesson's tests look at which thread made the wake.

*A blocking call inside a task.* One call that waits, such as a `read` on a socket that is still in blocking mode, a `thread::sleep`, or a long computation, stops every task of a single-threaded executor. Nothing fails; the server just stops answering. This is the most common production problem in async code, and it has no compiler error.

**When to use which.**

| Design | Good for | Cost |
| --- | --- | --- |
| One thread per connection, blocking calls | tens of connections, simple code | a stack per connection; thousands become expensive |
| A polling reactor (this lesson) | learning the contract; very small programs | work per waiting task per tick; up to one tick of delay |
| A reactor on epoll, kqueue or IOCP (mio, tokio) | real servers | platform code, which those crates maintain for you |

**Read more.**
[`TcpStream::set_nonblocking`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.set_nonblocking) ·
[`io::ErrorKind`](https://doc.rust-lang.org/std/io/enum.ErrorKind.html) (`WouldBlock`, `Interrupted`, `WriteZero`) ·
[`Read for &TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#impl-Read-for-%26TcpStream) (why reading needs only `&self`) ·
[`Write::write_all`](https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all)

## Lesson 05 · Pin projection by hand

**What is assumed.** Chapter 21's capstone, where you kept a future in a `Pin<Box<F>>`, and Chapter 23's habit of writing a SAFETY comment for every `unsafe` block. This lesson does not need lessons 03 and 04.

**The model.** `Pin<&mut T>` is a promise about a *place*: the `T` behind this pointer will stay at this address until it is dropped, and its destructor will run before its memory is used for anything else. Futures made by `async` rely on that, because they may contain pointers into themselves. For a type that does not care about its address there is the marker trait `Unpin`, and for `T: Unpin` a `Pin<&mut T>` is no stronger than a `&mut T`.

The promise becomes work when you write a future that *contains* another future as a field. Your `poll` receives `Pin<&mut Self>`. To poll the inner future you need `Pin<&mut Inner>`. Going from the pinned whole to a pinned part is called a **projection**. The compiler cannot do it for you:

```rust,compile_fail,E0599
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Wrapper<F> {
    inner: F,
}

impl<F: Future> Future for Wrapper<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        self.inner.poll(cx)
    }
}
```

```text
error[E0599]: no method named `poll` found for type parameter `F` in the current scope
   |
 9 | impl<F: Future> Future for Wrapper<F> {
   |      - method `poll` not found for this type parameter
...
12 |         self.inner.poll(cx)
   |                    ^^^^ method not found in `F`
   |
help: consider pinning the expression
```

`self.inner` is a plain `F`, reached through `Deref`. `poll` exists only on `Pin<&mut F>`. (Do not follow the compiler's suggestion here. `pin!(self.inner)` would have to move the field out of the pinned struct, and the next error, E0507, says that it cannot.) The safe ways around it each give something up:

```rust,compile_fail,E0277
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Wrapper<F> {
    inner: F,
}

impl<F: Future> Future for Wrapper<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let this = self.get_mut();
        Pin::new(&mut this.inner).poll(cx)
    }
}
```

```text
error[E0277]: `F` cannot be unpinned
   |
12 |         let this = self.get_mut();
   |                         ^^^^^^^ within `Wrapper<F>`, the trait `Unpin` is not implemented for `F`
   |
   = note: consider using the `pin!` macro
           consider using `Box::pin` if you need to access the pinned value outside of the current scope
help: consider further restricting type parameter `F` with trait `Unpin`
```

Adding `F: Unpin` compiles, and then the wrapper refuses every `async` block. Storing `Pin<Box<F>>` compiles for every `F`, and costs an allocation for each wrapper. The third way is to write the projection yourself, with `unsafe`, and to carry the proof.

**Structural pinning.** For each field you decide once, and write down:

- The field is **structurally pinned**: when the struct is pinned, the field is pinned too. You may create `Pin<&mut Field>` from `Pin<&mut Struct>`. You may *never* create `&mut Field` that safe code could use to move the field.
- The field is **not pinned**: it is plain data. You may create `&mut Field`. You may *never* create `Pin<&mut Field>`.

A field cannot be both. A wrapped future must be structurally pinned, because polling needs the pin. A counter, a flag or a closure is not pinned.

**The four duties.** The standard library's `pin` module lists what the author of a type with structurally pinned fields must guarantee. This module groups that list into four duties, and a SAFETY comment for a projection argues each one.

1. **Unpin.** The struct must not be `Unpin` unless every structurally pinned field is. The automatic rule gives exactly that. The duty is: *write no `impl Unpin` of your own* (it is a safe impl to write, and it would let anyone move your "pinned" fields with `Pin::get_mut`).
2. **Drop.** A destructor receives `&mut self`, also for a value that was pinned. If you write `Drop`, it must treat the pinned fields as pinned and not move them. The simple way to hold the duty: *no `Drop` impl*.
3. **No moves, and the drop guarantee.** No code of yours moves a pinned field out of the struct while it is pinned: no `mem::replace`, no `mem::swap`, no `Option::take` on it. And you never make its memory reusable without running its destructor: no `mem::forget` of it, no overwriting through a raw pointer. Replacing the value with `Pin::set` is allowed, because it drops the old value where it stands.
4. **Layout.** The struct is not `#[repr(packed)]`. Fields of a packed struct may be misaligned, and the compiler then moves a field to drop it.

**A worked example: `Map`.** A future that applies a function to the output of another future. The inner future is structurally pinned. The function is not: it is taken out by value at the end.

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// Applies `f` to the output of `future`.
struct Map<Fut, F> {
    /// Structurally pinned.
    future: Fut,
    /// Not pinned.
    f: Option<F>,
}

impl<Fut, F> Map<Fut, F> {
    fn project(self: Pin<&mut Self>) -> (Pin<&mut Fut>, &mut Option<F>) {
        // SAFETY: `future` is structurally pinned, `f` is not.
        // 1. Unpin: there is no manual `Unpin` impl, so `Map` is `Unpin`
        //    only when `Fut` (and `F`) are.
        // 2. Drop: `Map` has no `Drop` impl.
        // 3. No moves: `future` is reached only through the `Pin<&mut Fut>`
        //    returned here. Nothing moves it out, overwrites it or forgets
        //    it; it is dropped in place together with the `Map`.
        // 4. Layout: `Map` is not `repr(packed)`.
        // `f` is never given out pinned, so handing out `&mut Option<F>`
        // and moving the `F` out of it breaks no promise.
        unsafe {
            let this = self.get_unchecked_mut();
            (Pin::new_unchecked(&mut this.future), &mut this.f)
        }
    }
}

impl<Fut, F, T> Future for Map<Fut, F>
where
    Fut: Future,
    F: FnOnce(Fut::Output) -> T,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let (future, f) = self.project();
        let output = std::task::ready!(future.poll(cx));
        let f = f.take().expect("a Map was polled after it returned its output");
        Poll::Ready(f(output))
    }
}

fn main() {
    let mut cx = Context::from_waker(Waker::noop());
    let name = String::from("ada");
    let job = Map {
        future: async { 3 },
        f: Some(move |times: usize| name.repeat(times)),
    };
    // An async block is not Unpin, so this `Map` is not Unpin either.
    let mut job = std::pin::pin!(job);
    assert_eq!(job.as_mut().poll(&mut cx), Poll::Ready(String::from("adaadaada")));
}
```

`get_unchecked_mut` is `unsafe` because the `&mut Self` it returns *could* be used to move everything. The block keeps that reference private, uses it for two field accesses, and gives out only what each field allows. That is the pattern for every projection.

**Changing a pinned value: `Pin::set`.** Two tasks of the lesson replace a pinned value: an `Option<F>` becomes `None` when the future is done, and a state machine goes from one state to the next. The tool is `Pin::set(&mut self, value)`. It assigns through the pin, and an assignment drops the old value in place first. Compare the wrong tool:

```text
inner.set(None)                    the future is dropped at its pinned address   correct
this.inner.take()                  the future is MOVED out, then dropped         breaks duty 3
*this = State::Second(..)          through a raw `&mut`: fine in effect, but you
                                   needed `get_unchecked_mut` to write it;
                                   `self.set(State::Second(..))` needs no unsafe
```

`Option::as_pin_mut` is the matching read access: it turns `Pin<&mut Option<F>>` into `Option<Pin<&mut F>>` without `unsafe`.

**Mistakes people really make.** The two compiler errors above are the ones you see. The dangerous mistakes compile:

- `Option::take`, `mem::replace` or `mem::swap` on a structurally pinned field. The lesson's tests use a future that records its address and notices this.
- `impl<F> Unpin for Wrapper<F> {}` "to make the error go away". It does, and it makes the projection unsound. (The impl is only wrong for a type that pins a field. A type that never hands out a `Pin` to any of its fields may implement `Unpin` for all its type parameters; lesson 06 has such a type.)
- A `Drop` impl that takes a pinned field out to "finish it properly".
- Returning `Pending` after a state change without polling the new state. This one is a lost wakeup, not a pinning bug: the new future was never polled, so nobody holds the waker. Loop inside `poll` until something really is pending.

**When to use which.**

| You have | Use |
| --- | --- |
| One future to keep, and an allocation is fine | `Pin<Box<F>>` (Chapter 21) |
| Locals of an `async fn` or block | nothing: the compiler projects for you. `pin!` gives a `Pin<&mut _>` to a local. |
| A named future or stream type with inline fields, in a library | a projection. In production code the `pin-project-lite` crate writes it, with the same four duties checked by a macro. Write it by hand here, once, to know what the macro does. |
| A type that must not move for its own reasons | a `PhantomPinned` field, which makes it `!Unpin` |

**Read more.**
[The `std::pin` module](https://doc.rust-lang.org/std/pin/index.html), sections "Projections and Structural Pinning" and "`Drop` guarantee" ·
[`Pin::map_unchecked_mut`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.map_unchecked_mut) ·
[`Pin::set`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.set) ·
[`Option::as_pin_mut`](https://doc.rust-lang.org/std/option/enum.Option.html#method.as_pin_mut) ·
[`PhantomPinned`](https://doc.rust-lang.org/std/marker/struct.PhantomPinned.html)

## Lesson 06 · A Stream trait, a channel, and adapters

**What is assumed.** Lesson 05 (the adapters need a projection) and iterators with adapters from Chapter 15.

**The model.** A future produces one value. A **stream** produces many, over time. It is to `Iterator` what `Future` is to a plain function call:

| | one value | many values |
| --- | --- | --- |
| now | a function call | `Iterator::next(&mut self) -> Option<Item>` |
| later | `Future::poll -> Poll<Output>` | `Stream::poll_next -> Poll<Option<Item>>` |

`poll_next` has three answers, and each carries a promise:

- `Ready(Some(item))`: here is the next item. Poll again for more.
- `Ready(None)`: the stream has ended. Do not poll it again; what happens then is the stream's business.
- `Pending`: nothing now. The waker in `cx` has been stored and will be woken when polling is worth it.

The standard library has no stable stream trait yet (the unstable one is called `AsyncIterator`), so the lesson defines its own, with the same shape as the one in the `futures` crates.

**A worked example: a source and an adapter, the easy way.** An adapter that demands `S: Unpin` needs no projection, because `Pin<&mut Self>` then gives `&mut Self`.

```rust
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

trait Stream {
    type Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
}

/// 3, 2, 1, end. It is pending once before each answer, like a slow source.
struct Countdown {
    left: u32,
    ready: bool,
}

impl Stream for Countdown {
    type Item = u32;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<u32>> {
        if !self.ready {
            self.ready = true;
            // Nothing outside will wake this stream, so it asks for the
            // next poll itself. Pending without a wake would be a hang.
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.ready = false;
        if self.left == 0 {
            return Poll::Ready(None);
        }
        self.left -= 1;
        Poll::Ready(Some(self.left + 1))
    }
}

/// Numbers the items of a stream.
struct Enumerate<S> {
    stream: S,
    next: usize,
}

impl<S: Stream + Unpin> Stream for Enumerate<S> {
    type Item = (usize, S::Item);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // `Enumerate<S>` is Unpin because `S` is, so `&mut` access is allowed.
        let this = &mut *self;
        match Pin::new(&mut this.stream).poll_next(cx) {
            Poll::Ready(Some(item)) => {
                let index = this.next;
                this.next += 1;
                Poll::Ready(Some((index, item)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn main() {
    let mut stream = Enumerate {
        stream: Countdown { left: 3, ready: false },
        next: 0,
    };
    let mut cx = Context::from_waker(Waker::noop());
    let (mut items, mut pending) = (Vec::new(), 0);
    loop {
        match Pin::new(&mut stream).poll_next(&mut cx) {
            Poll::Ready(Some(item)) => items.push(item),
            Poll::Ready(None) => break,
            Poll::Pending => pending += 1,
        }
    }
    assert_eq!(items, [(0, 3), (1, 2), (2, 1)]);
    assert_eq!(pending, 4, "once before each of the three items and once before the end");
}
```

The lesson's adapters must also work for streams that are not `Unpin`, so they store the stream inline and project to it, as in lesson 05. One trick keeps the `unsafe` small: every adapter with one inner stream is "a stream plus some plain state". Write that pair, and its projection, once.

The opposite case is in the lesson too. `iter` turns an iterator into a stream and promises `Unpin` for *every* iterator, also for one that is not `Unpin` itself. A struct that holds the iterator gets `Unpin` automatically only when the iterator has it, so the compiler refuses the return type. Here a manual `impl<I> Unpin for YourStream<I> {}` is the right answer, and it is safe code: the stream only ever calls `next` through `&mut`, it never creates a `Pin` to the iterator, and so nobody was promised that the iterator stays in place. Duty 1 of lesson 05 forbids the impl for a type with a structurally pinned field. This type has none.

**The rule for adapters: pass `Pending` on, never invent it.** An adapter returns `Pending` only in one case: the inner stream returned `Pending` in *this* call. Then the inner stream has stored the waker, and the promise of `Pending` is kept. `filter` is where people break the rule. When it rejects an item it must ask the inner stream again in the same call. If it returned `Pending` there, nobody would hold the waker: the inner stream was ready, not waiting.

**A channel is a stream with a queue inside.** The receiver's `poll_next` is the completion cell of Chapter 21 with a queue in place of the single slot:

```text
poll_next (under the lock):                    try_send (under the lock):
    item in queue?    -> Ready(Some(item))         receiver gone? -> Err(Closed(value))
    no sender left?   -> Ready(None)               queue full?    -> Err(Full(value))
    else store waker  -> Pending                   push; take the waker
                                               (after the lock) wake it
```

Two events can end the receiver's wait, and each must wake it: a value is queued, or the *last* sender is dropped (the stream has just ended, and the receiver must find out). Nothing else ends the wait: a sender that is cloned, or dropped while others remain, changes nothing for the receiver. The check and the registration in `poll_next` happen under one lock, and both events take that lock, so neither can slip into the gap.

**Bounded, and what "full" means.** A queue without a limit hides a problem until memory runs out: a producer that is faster than its consumer. A bounded queue makes the problem visible, and then somebody has to decide. There are three honest answers:

| When the queue is full | Name | Where |
| --- | --- | --- |
| hand the value back to the sender, at once | `try_send` | this lesson; the capstone removes a client that is too slow |
| make the sender wait until there is room | backpressure | lesson 09 (`send().await` on a tokio channel) |
| drop the oldest or the newest value | lossy | metrics, live video; say so in the API |

A function that is not `async` cannot wait, so `try_send` must return the value: the caller may want to try again, send it elsewhere, or count it as dropped.

**`merge` and fairness.** `merge(a, b)` polls two streams and gives whichever has an item. If it always asked `a` first, a busy `a` would starve `b`. The lesson's rule is simple and can be tested: the side that is asked first changes on every call. `merge` must also remember that one side has ended, because polling a stream after `None` is not allowed.

**Mistakes people really make.**

*Calling `next()` on a stream that is not `Unpin`.* `next` borrows the stream as `&mut S` and must pin it itself, so it needs `S: Unpin`:

```text
error[E0277]: `PhantomPinned` cannot be unpinned
   |
37 |     let _first = next(&mut ticks);
   |                  ---- ^^^^^^^^^^ within `Ticks`, the trait `Unpin` is not implemented for `PhantomPinned`
   |
   = note: consider using the `pin!` macro
```

The fix is the note: `let mut ticks = pin!(ticks);`. A `Pin<&mut S>` is a stream too, and it is always `Unpin`.

*Forgetting the last sender.* A receiver that waits while the last sender is dropped sleeps for ever unless `Drop for Sender` wakes it. The symptom is a program that does everything right and then does not exit.

*Dropping queued values under the lock.* When the receiver is dropped, the queued values go with it. Their destructors are foreign code (lesson 03): move the queue out of the lock first.

**Read more.**
[`AsyncIterator`](https://doc.rust-lang.org/std/async_iter/trait.AsyncIterator.html) (the unstable trait in the standard library, and why it looks like this) ·
[`std::sync::mpsc`](https://doc.rust-lang.org/std/sync/mpsc/index.html) (`sync_channel` is the blocking relative of this lesson's channel) ·
[`Iterator::fuse`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fuse) ·
[`std::task::ready!`](https://doc.rust-lang.org/std/task/macro.ready.html)

## Lesson 07 · Select and cancellation safety

**What is assumed.** Lessons 05 and 06, and Chapter 20, lessons 05 and 06, where you first cancelled a future by dropping it and raced two futures.

**The model.** `select(a, b)` polls two futures and finishes when the first one does. The other one is *dropped*. Dropping a future is the only way to cancel it in Rust, and it is not a request: the future gets no chance to finish its current step. Its destructor runs, the destructors of its local variables run, and everything those variables held is gone.

So the question for every future that may lose a select is: **what does it hold at each `.await` that exists nowhere else?** If the answer is "nothing", the future is *cancellation safe*: you may drop it at any await point and call the function again later, and nothing was lost or done twice. If the answer is "the first half of a message", "an item I took from a queue", or "a counter I increased and have not decreased", it is not.

Cancellation safety is a property of one function, and it belongs in its documentation. tokio's documentation states it for the methods that are used in `select!`; lesson 09 depends on those statements.

**A worked example: taking two items.** Both functions return two items from an inbox. The first takes one, waits, and takes the other. The second waits until both are there.

```rust
use std::collections::VecDeque;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// A queue that this example fills by hand.
struct Inbox {
    items: VecDeque<u32>,
}

impl Inbox {
    async fn next(&mut self) -> u32 {
        std::future::poll_fn(|_cx| match self.items.pop_front() {
            Some(item) => Poll::Ready(item),
            // A real queue stores the waker here. This example polls by hand.
            None => Poll::Pending,
        })
        .await
    }

    /// NOT cancellation safe: at the second await, `first` exists only in
    /// this future.
    async fn pair_fragile(&mut self) -> (u32, u32) {
        let first = self.next().await;
        let second = self.next().await;
        (first, second)
    }

    /// Cancellation safe: nothing is taken until both items are there, and
    /// then both are taken without an await in between.
    async fn pair(&mut self) -> (u32, u32) {
        std::future::poll_fn(|_cx| {
            if self.items.len() < 2 {
                return Poll::Pending;
            }
            let first = self.items.pop_front().unwrap();
            let second = self.items.pop_front().unwrap();
            Poll::Ready((first, second))
        })
        .await
    }
}

fn main() {
    let mut cx = Context::from_waker(Waker::noop());

    let mut inbox = Inbox { items: VecDeque::from([1]) };
    {
        let mut call = pin!(inbox.pair_fragile());
        assert!(call.as_mut().poll(&mut cx).is_pending());
        // `call` is dropped here, as the losing side of a select would be.
    }
    assert!(inbox.items.is_empty(), "the 1 died with the future that held it");

    let mut inbox = Inbox { items: VecDeque::from([1]) };
    {
        let mut call = pin!(inbox.pair());
        assert!(call.as_mut().poll(&mut cx).is_pending());
    }
    assert_eq!(inbox.items, [1], "nothing was taken");
    inbox.items.push_back(2);
    let mut call = pin!(inbox.pair());
    assert_eq!(call.as_mut().poll(&mut cx), Poll::Ready((1, 2)));
}
```

**Three ways to make an operation safe to cancel.**

1. **Keep the state in the owner, not in the future.** The line reader of this lesson has a buffer. If the buffer is a field of the reader, a cancelled `next_line` leaves the bytes where the next call finds them. If it is a local variable of `next_line`, they are dropped. The rule: between two awaits, everything that must survive lives behind `&mut self`.
2. **Do not cancel: keep the future.** Pin the future outside the loop and pass a *reference* to `select`. `Pin<&mut F>` implements `Future`, and dropping the reference does not drop `F`. The work of the losing round is kept, and the next round continues it.
3. **Clean up in `Drop`.** Code after an `.await` does not run when the future is cancelled. A destructor does. Whatever must be undone (a counter, a membership, a lock, a temporary file) belongs in a guard value whose `Drop` undoes it.

```rust
use std::cell::Cell;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// Counts the requests in flight. The guard is the only code that
/// decreases the counter, so every path decreases it.
struct InFlight<'a>(&'a Cell<u32>);

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}

async fn handle(counter: &Cell<u32>) {
    counter.set(counter.get() + 1);
    let _guard = InFlight(counter);
    std::future::pending::<()>().await; // a request that never finishes
}

fn main() {
    let counter = Cell::new(0);
    {
        let mut request = pin!(handle(&counter));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(request.as_mut().poll(&mut cx).is_pending());
        assert_eq!(counter.get(), 1);
    }
    assert_eq!(counter.get(), 0, "cancelled, and still counted down");
}
```

**Select in a loop.** Most selects are in a loop: wait for a line *or* a message *or* shutdown, handle it, repeat. Each round creates its futures anew, so each round cancels the losers. Go through the branches one by one:

- Is this branch cancellation safe? If not, use way 2 above or restructure.
- Does a branch own a deadline or a counter that should *not* restart every round (an idle timeout that only the client's own lines may reset)? Then the deadline is a variable outside the loop, and the branch only reads it.
- Does a branch register something (a waker in a token)? Then dropping it must remove the registration, or the loop leaks one entry per round. The lesson's `CancelToken` is tested for exactly this.

**Mistakes people really make.**

*Passing `&mut future` for a future that is not `Unpin`.* The idea of way 2 is right, but a plain `&mut` to an `async` block is not a future:

```rust,compile_fail,E0277
use std::future::Future;

async fn first<A: Future, B: Future>(_a: A, _b: B) {}

fn main() {
    let mut slow = async { 5 };
    let _race = first(&mut slow, async {});
}
```

```text
error[E0277]: `&mut {async block@src/main.rs:6:20: 6:25}` is not a future
  |
7 |     let _race = first(&mut slow, async {});
  |                 -----      ^^^^ `&mut {async block@src/main.rs:6:20: 6:25}` is not a future
  |
  = help: the trait `Unpin` is not implemented for `{async block@src/main.rs:6:20: 6:25}`
  = note: required for `&mut {async block@src/main.rs:6:20: 6:25}` to implement `Future`
```

Pin it first: `let mut slow = pin!(slow);` and pass `slow.as_mut()`.

*A returned future that borrows too much.* In edition 2024 the type behind a returned `impl Future` may use every lifetime that is in scope. The compiler therefore treats the future as a borrow of every reference among the arguments, `&self` included, whatever the body really does. A wait future of a token should own its share instead, so that it can be spawned and can outlive the handle it came from. Say so with `+ use<>`; drill d08 shows the error you get without it.

*Racing a write.* `write_all` is not cancellation safe: if it is dropped halfway, part of the buffer is on the wire and nobody knows how much. Race it only when losing the race also ends the connection, as a timeout does. (A write that must survive a lost race keeps its position in the owner, by way 1: a loop over `poll_write` that advances a counter outside the future.)

**When to use which.**

| You want | Use |
| --- | --- |
| The first of several results; the rest no longer matter | `select` |
| All results | join (Chapter 20), or spawn and await the handles |
| A long operation with regular interruptions | pin it outside, `select(op.as_mut(), tick)` in a loop |
| To stop many tasks at once | a cancellation token that each task selects on |
| A fixed priority between branches | a biased select such as this lesson's (the first argument wins ties) |
| No branch to starve | alternate the order, as `merge` does |

**Read more.**
[`impl Future for Pin<P>`](https://doc.rust-lang.org/std/pin/struct.Pin.html#impl-Future-for-Pin%3CP%3E) and [`for &mut F`](https://doc.rust-lang.org/std/future/trait.Future.html#impl-Future-for-%26mut+F) ·
[`std::pin::pin!`](https://doc.rust-lang.org/std/pin/macro.pin.html) ·
[`Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html) ·
[Precise capturing with `use<..>`](https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html) in the Edition Guide

## Lesson 08 · Capstone: a chat server on your own runtime

**The problem in plain words.** Clients connect over TCP and send lines. The first line is a name. Every later line goes to all other clients. The server tells the others when somebody joins or leaves, removes clients that are idle or too slow, and shuts down when asked. The lesson file states this as nine numbered rules; the tests are written against the rules and against nothing else.

There is no code to read first. You decide which tasks exist, what they own, and how they talk to each other. Everything you need is in your lessons 02 to 07.

**Questions to answer before you write code.** Write the answers in NOTES.md.

- *Tasks.* What does the accept loop do, and what does one connection task do? Does a connection need one task or two?
- *The room.* Where is the list of connected clients? Which tasks touch it? It is shared state on a single thread, but your futures must be `Send`: what does that leave you? How long is its lock held, and is anything woken or awaited while it is held?
- *Sending to a client.* A connection task is the only one that writes to its socket. How does a line from another client reach it? What bounds that path (rule 6)? The answer to `/who` is produced by the client's own task: which way must it take so that it cannot overtake a line that was produced earlier (rule 8)?
- *Waiting.* In its main loop a connection waits for four things at once: a line from its client, a line for its client, the idle deadline, and the stop signal. Write the select. Which branches are cancellation safe? Where does the idle deadline live so that only the client's own lines move it? Which branch must win when two are ready in the same poll, so that `* server closing` is the last line a client gets (rule 9)?
- *Leaving.* A client must leave the room on every path out of its task: end of input, an error, a limit, the stop signal, and the task being dropped with the executor (rule 4). Which construct runs on all of them? And one case that no test reaches: rule 6 removes a slow client from the room while its task is still alive, stuck in a write. Another client takes the free name. Then the old task ends and leaves. What must it find in the room so that it does not remove the newcomer, and does not announce `left` a second time?
- *Fairness.* A client sends 500 lines in one packet. Your line reader returns them one after another without ever being pending. What does the executor do during that time, and what must the task do (rule 7)?
- *Stopping.* What does `run` wait for, what bounds the wait, and in which order are the executor, the timer driver and the reactor dropped?

**Three facts that are easy to learn the hard way.**

*An accept error is not a reason to stop.* `accept` can fail for one connection (the client went away) or for a while (too many open files). Go on with the loop; after an error, pause briefly so that the loop does not spin.

*A write can wait for ever.* A client that does not read fills the kernel's buffers, and from then on `write_all` is pending. A connection task that awaits it without a time limit never notices the stop signal or the idle deadline again.

*Closing a socket that still has unread input may destroy your last message.* If the server closes a connection while bytes from the client are still unread, many systems send a reset instead of a normal close, and on some of them (Windows is one) the client may then never see the farewell line that was sent just before. The tests avoid this case as long as your task reads at least a kilobyte at a time: the longest input that a test leaves behind a refused line is 1000 bytes in one write, and one read of that size takes it all. A production server shuts down its writing side, then reads and discards input for a short time, then closes. Add it to your notes as an experiment.

**What the tests check.** The protocol, line by line, with blocking clients that time out after ten seconds; order under load; that one client's burst does not starve another; that a client which never reads is removed and does not delay shutdown; that limits end connections, and that lines from other clients do not move the idle deadline; and that `run` returns with the right summary. One test reads the source of the lesson file: the server may not use `std::thread`, because the point is that one thread serves everyone.

**After the tests pass.** Connect with a real client (`nc 127.0.0.1 PORT` on most systems) from two terminals. Then look for what the tests cannot see: read your `run` function and say, for every value, when it is dropped and on which thread.

**Read more.**
[`TcpStream::shutdown`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.shutdown) ·
[`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html) (sorted names for `/who`) ·
[`Arc<str>`](https://doc.rust-lang.org/std/sync/struct.Arc.html#impl-From%3CString%3E-for-Arc%3Cstr%3E) (one allocation for a line that goes to many clients)

## Lesson 09 · The same service on tokio

**What is assumed.** Lesson 08, and lesson E06 of the ecosystem track (`JoinSet`, bounded channels, `select!`, paused time).

**The model: you know every part already.**

| You built | tokio's version | What it adds |
| --- | --- | --- |
| `Executor`, `Spawner`, `JoinHandle` (02) | the runtime, `tokio::spawn`, `JoinHandle`, `JoinSet` | several worker threads that steal work from each other; panics are caught and reported through the handle |
| `TimerDriver`, `Sleep` (03) | `tokio::time::sleep`, `timeout`, `Instant` | a timer wheel inside the runtime, no extra thread; a clock that tests can pause |
| `Reactor`, `AsyncStream` (04) | `tokio::net`, built on mio | epoll, kqueue or IOCP: only ready sockets are woken |
| projections (05) | `pin-project-lite`, used inside tokio | the four duties, checked by a macro |
| `channel`, `try_send` (06) | `tokio::sync::mpsc` | `send().await`: a sender can wait for room |
| `select`, `CancelToken` (07) | `tokio::select!`, `tokio_util::sync::CancellationToken` | any number of branches; child tokens |
| `yield_now` after each line (08) | cooperative budgets | tokio's own resources make a task yield after a number of operations |

So this lesson is not about tokio's API. It is about three policies that the capstone handled in the simplest way, or not at all.

**Backpressure.** In the capstone a full queue removed the client. That protects the server, and it is harsh: a client on a slow link loses its connection during a burst. The alternative is to make the *sender* wait: `outbox.send(line).await` completes when there is room. While a connection waits there it reads nothing from its own client. The kernel's receive buffer for that socket fills. TCP then tells the client's machine that there is no room (the *receive window* shrinks to zero), the client's send buffer fills in turn, and the client's own `write` waits. The pressure travels back, hop by hop, to whoever produces too much. Every queue on the path is bounded, so memory is bounded, and nothing is dropped.

Backpressure has two traps.

*It can deadlock.* Two clients talk at the same time. Each connection wants to put a line into the other's queue, both queues are full, and each connection is the one that should be emptying its own queue. If a connection is one loop that either reads or writes, both wait for ever. The cure is to let the two directions of a connection make progress independently: two futures in one task, run side by side with `tokio::join!`, or two tasks. Then a connection that waits for room elsewhere still empties its own queue.

*It needs a limit.* A client that never reads would hold its senders for ever. So a write to a client gets a time limit (`patience`). When it expires, the client is treated as stuck: its queue is closed, which makes every sender that waits for room continue at once, and its connection ends.

*It needs an exit for shutdown.* A connection that waits for room in the queue of a stuck client does nothing else on its reading side. If that wait could only end with `patience`, a shutdown would find a healthy client that cannot be told goodbye within `grace`, and would abort it together with the stuck one. So the wait for room is the one wait in this service that the shutdown signal must be able to end. The line that waited is given up; the contract promises only lines that are already queued.

**Limits.** Each connection costs memory: buffers, a task, a queue. A server without a connection limit can be brought down by connections alone. A `Semaphore` with `max_connections` permits is the tool, and the order of two lines is the whole design: take the permit *before* you accept. Connections beyond the limit then wait in the listener's queue inside the kernel, which is itself bounded. The permit moves into the connection's task and is returned when the task ends, in whatever way it ends.

**Graceful shutdown.** A deployment replaces servers many times a day. Each time the old process must stop without cutting clients off in the middle of a line. The sequence is always the same:

```text
1. A signal arrives        -> cancel the token
2. Stop accepting          -> drop the listener (new clients go to the new process)
3. Tell every connection   -> each one sees the token, finishes what is queued,
                              says goodbye and closes
4. Wait, with a limit      -> join all tasks, but at most `grace`
5. Abort what is left      -> a stuck connection must not keep the process alive
6. Report                  -> how many finished, how many were aborted
```

A worked example with a different service: it answers each line in upper case, and says `BYE` when it shuts down. This is the connection handler and the server loop of `examples/l09_shutdown.rs`.

```rust,ignore
/// One connection: answer every line in upper case until the client leaves
/// or the server shuts down, then say goodbye. Returns the number of answers.
async fn shout(stream: TcpStream, shutdown: CancellationToken) -> std::io::Result<u64> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let mut answered = 0;
    loop {
        // tokio documents `next_line` as cancellation safe, so it may lose
        // this race in any round.
        let line = tokio::select! {
            () = shutdown.cancelled() => break,
            line = lines.next_line() => line?,
        };
        let Some(line) = line else { break };
        let answer = format!("{}\n", line.to_uppercase());
        writer.write_all(answer.as_bytes()).await?;
        answered += 1;
    }
    writer.write_all(b"BYE\n").await?;
    writer.shutdown().await?;
    Ok(answered)
}
```

```rust,ignore
/// Accepts connections until `shutdown` is cancelled, then gives the open
/// connections `grace` to finish. Returns the total number of answers.
async fn serve(listener: TcpListener, shutdown: CancellationToken, grace: Duration) -> u64 {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            () = shutdown.cancelled() => break,
            accepted = listener.accept() => {
                if let Ok((stream, _peer)) = accepted {
                    connections.spawn(shout(stream, shutdown.clone()));
                }
            }
        }
    }
    // Step 2: nobody can connect any more.
    drop(listener);
    // Steps 3 and 4: the connections have seen the token; wait for them.
    let mut answered = 0;
    let drain = async {
        while let Some(joined) = connections.join_next().await {
            if let Ok(Ok(count)) = joined {
                answered += count;
            }
        }
    };
    if timeout(grace, drain).await.is_err() {
        // Step 5.
        connections.shutdown().await;
    }
    answered
}
```

**Testing a service.** Lesson 08 could only test over real sockets, in real time. That is slow for anything that involves waiting, and it cannot test "nothing happens for a minute" at all. This lesson's `serve` is written against a small `Listener` trait, so the tests can hand it in-memory pipes (`tokio::io::duplex`) and run with a paused clock. A paused clock jumps forward whenever every task is waiting. A sleep of a minute then costs no time, and elapsed times can be compared with `assert_eq!`. A stuck test fails at once, because the clock runs to the test's own time limit.

What such a test proves is the logic: order, limits, timeouts, shutdown. What it cannot prove is that the code works with real sockets, where a write can be partial and a close can be a reset. So one test uses loopback TCP, and that is enough because the socket-facing code is thin.

**Mistakes people really make.**

*A `std` lock held across an await in a spawned task.* The guard is not `Send`, so the future is not, and `tokio::spawn` refuses it. The error is the "future cannot be sent between threads safely" of lesson 02, with a `MutexGuard` in it. The fix is almost never an async mutex. It is to end the critical section before the await, as the room in this module does.

*`send` inside `select!`.* If the branch `tx.send(value)` loses, the value that was moved into the future is dropped. tokio's documentation says so for `send`. Either await the send outside the select, or use `reserve().await` in the select and send afterwards. The model solution awaits its sends outside every select with one exception, which it marks: a send that still waits for room when the shutdown begins may lose, because that line is given up on purpose.

*Paused time with real sockets.* The clock jumps when all tasks are waiting, and a task that waits for a real packet counts as waiting. Timers then fire before the packet arrives. Use paused time with in-memory I/O only.

*Counting on `Drop` for work that must wait.* `Drop` cannot await. A connection that must tell others "I left" does so on its normal path, with an `.await`; the guard in `Drop` only does what needs no waiting.

**When to use which.**

| Question | Answer |
| --- | --- |
| A slow client | first backpressure, then a time limit, then removal |
| A queue between tasks | bounded, always; choose the size from memory per client, not from hope |
| Shared state for short updates | `std::sync::Mutex`, never held across an await |
| Many tasks to wait for | `JoinSet`; it also aborts them when it is dropped |
| Shutdown | a `CancellationToken`, a grace period, then abort |

**Read more.**
[tokio: graceful shutdown](https://tokio.rs/tokio/topics/shutdown) ·
[`tokio::select!`](https://docs.rs/tokio/1.54.1/tokio/macro.select.html) (section "Cancellation safety") ·
[`tokio::sync::mpsc::Sender::send`](https://docs.rs/tokio/1.54.1/tokio/sync/mpsc/struct.Sender.html#method.send) ·
[`tokio::sync::Semaphore`](https://docs.rs/tokio/1.54.1/tokio/sync/struct.Semaphore.html) ·
[`tokio::time::pause`](https://docs.rs/tokio/1.54.1/tokio/time/fn.pause.html) ·
[`CancellationToken`](https://docs.rs/tokio-util/0.7.20/tokio_util/sync/struct.CancellationToken.html)
