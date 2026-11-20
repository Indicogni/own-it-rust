# Chapter 21 concepts — Futures by hand

[Chapter guide](README.md) · [Hints](HINTS.md)

Read one section, then do its lesson. Every code block on this page is compiled and run by the course checks, so you can paste any of them into [the scratch example](../../examples/ch21_scratch.rs) and change it.

## What you will be able to do

In Chapter 20 you wrote `async` functions and used `.await`. The compiler and a runtime did the rest. In this chapter you build that "rest" by hand, with the standard library only: the future, the waker, the shared state between a task and an event, and a small executor.

After this chapter you will be able to:

- implement the `Future` trait for your own types and poll any future by hand;
- say exactly what `Poll::Pending` promises, and arrange the wake that keeps the promise;
- build a future that is completed from another thread without losing a wakeup;
- write `block_on`, a one-future executor that sleeps while there is nothing to do;
- explain what a reactor does and why "ready" does not mean "your data has arrived";
- keep one task from starving the others by doing a bounded amount of work for each poll;
- explain what pinning is for, and wrap a future that must not move.

## Before you start

Traits with associated types (Chapter 11), `Deref` and `Drop` (Chapter 13), closures (Chapter 14), threads, `Arc`, `Mutex`, channels and atomics (Chapter 19), and `async`/`.await`, `Box::pin` and `pin!` as a user (Chapter 20).

## Time

These are estimates for reading the section, doing the lesson, and writing your notes.

| Lesson | Estimate |
| --- | --- |
| 01 Implement Future | 45 min |
| 02 Pending and wakeups | 45 min |
| 03 A completion cell | 60 min |
| 04 A parker block_on | 60 min |
| 05 Readiness registration model | 45 min |
| 06 Cooperative work budgets | 40 min |
| 07 Capstone | 60 min |
| R01, C01, and the drills | 2 h 30 min |

## Lesson 01 · Implement Future

**The model.** A *future* is a value that stands for a computation that may not be finished yet. It does nothing by itself. Somebody has to *poll* it: call its `poll` method to let it work a little.

The whole trait is this:

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

`Output` is the type of the final result. `poll` returns `Poll::Ready(output)` when the work is finished, and `Poll::Pending` when it is not. The caller of `poll` is called the *executor*.

```text
executor                               future
   │  poll(pinned self, cx)  ───────►  works a little
   │  ◄───────  Poll::Pending          "not finished; I have arranged a wake"
   │        ... later, after the wake ...
   │  poll(pinned self, cx)  ───────►  works a little more
   │  ◄───────  Poll::Ready(output)    finished; do not poll again
```

Three rules follow from this picture.

1. `poll` borrows the future; it does not consume it. The future keeps its own state between polls.
2. `poll` must return quickly. It never waits for anything.
3. After `Ready`, the future is finished. The trait does not say what a later poll does. It may panic, return `Pending` forever, or return something wrong. So never poll again, and when you write a future, panic with a clear message if it happens.

**The receiver.** `self: Pin<&mut Self>` is a mutable reference with one extra promise: the value behind it will not be moved to another address. Lesson 07 explains why some futures need this promise. For now you need two facts.

- Most types do not care whether they move. They implement the marker trait `Unpin` automatically. For these types `Pin` changes nothing: `Pin::new(&mut value)` creates the pin, `get_mut()` gives the plain `&mut` back, and you can read and write fields through the pin.
- `Pin` is a wrapper that the compiler checks. It adds no data: a `Pin<Box<T>>` is still one pointer. Moving that pointer does not move the `T` on the heap.

**Giving the output away.** `poll` only borrows the future, but `Ready` must hand over an owned output. A future that stores its output therefore keeps it in an `Option` and moves it out with `Option::take` (Chapter 4), which leaves `None` behind. The `None` is also how a later poll can see that the future is finished.

**Worked example.** `Area` is ready at its first poll. The example polls it by hand, and then lets `.await` do the same polling.

```rust
use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll, Waker};

struct Area {
    width: u32,
    height: u32,
}

impl Future for Area {
    type Output = u32;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        Poll::Ready(self.width * self.height)
    }
}

fn main() {
    // A Context carries a waker (lesson 02). One that does nothing is enough here.
    let mut cx = Context::from_waker(Waker::noop());

    // 1. Poll by hand. `Area` is `Unpin`, so `Pin::new` accepts `&mut area`.
    let mut area = Area { width: 3, height: 4 };
    assert_eq!(Pin::new(&mut area).poll(&mut cx), Poll::Ready(12));

    // 2. An async block is a future too. Its `.await` polls `Area` for you,
    //    but only when somebody polls the block.
    let mut block = pin!(async { Area { width: 2, height: 5 }.await + 1 });
    assert_eq!(block.as_mut().poll(&mut cx), Poll::Ready(11));
}
```

**Mistakes people make.**

Calling `poll` on the future itself:

```rust,compile_fail,E0599
use std::future::Future;
use std::task::{Context, Poll, Waker};

struct Area {
    width: u32,
    height: u32,
}

impl Future for Area {
    type Output = u32;
    fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        Poll::Ready(self.width * self.height)
    }
}

fn main() {
    let mut cx = Context::from_waker(Waker::noop());
    let mut area = Area { width: 3, height: 4 };
    assert_eq!(area.poll(&mut cx), Poll::Ready(12));
}
```

```text
error[E0599]: no method named `poll` found for struct `Area` in the current scope
   |
 4 | struct Area {
   | ----------- method `poll` not found for this struct
...
19 |     assert_eq!(area.poll(&mut cx), Poll::Ready(12));
   |                     ^^^^ method not found in `Area`
   |
help: consider pinning the expression with `std::pin::pin!()` and assigning that to a new binding
```

The method exists, but only for `Pin<&mut Area>`. Write `Pin::new(&mut area).poll(&mut cx)`, or pin with `pin!` as the help says.

Writing to a field of a generic future:

```rust,compile_fail,E0594
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Tally<T> {
    label: T,
    polls: u32,
}

impl<T> Future for Tally<T> {
    type Output = u32;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        self.polls += 1;
        Poll::Ready(self.polls)
    }
}
```

```text
error[E0594]: cannot assign to data in dereference of `Pin<&mut Tally<T>>`
   |
13 |         self.polls += 1;
   |         ^^^^^^^^^^^^^^^ cannot assign
   |
   = help: trait `DerefMut` is required to modify through a dereference, but it is not implemented for `Pin<&mut Tally<T>>`
```

A struct is `Unpin` only if all its fields are. The compiler knows nothing about `T`, so `Tally<T>` might be a type that must not move, and then `Pin` does not hand out `&mut` access. The fix is to say what you need: `impl<T: Unpin> Future for Tally<T>`.

**When to use which.** To poll an `Unpin` future that you own, use `Pin::new(&mut f)`. To poll any future in the current function, pin it to the stack with `pin!`. To store or return a pinned future, use `Box::pin`, which costs one allocation.

**Read more.** [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) · [`Poll`](https://doc.rust-lang.org/std/task/enum.Poll.html) · [`Pin::new` and `get_mut`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.new) · [`Unpin`](https://doc.rust-lang.org/std/marker/trait.Unpin.html) · [await expressions in the Rust Reference](https://doc.rust-lang.org/reference/expressions/await-expr.html)

## Lesson 02 · Pending and wakeups

**The model.** An executor does not poll in a loop. That would use a whole processor core to ask "finished yet?" again and again. It polls once, and then it waits until somebody says that another poll is useful.

That somebody uses a *waker*. Each poll receives a `Context`, and `cx.waker()` is a handle to the task that is being polled. Calling `wake` on it means one thing: "poll this task again". So returning `Poll::Pending` is a promise:

> I have arranged that this waker will be called when I can make progress.

A future that returns `Pending` without arranging a wake is never polled again. There is no error and no warning. The task just stops.

A wake carries no result: it does not tell the task what happened. Waking also does not run `poll`. It only marks the task as "needs a poll". The executor polls later, at a time it chooses. Several wakes before that poll may lead to a single poll. This is called *coalescing*. A wake is therefore a hint and not a count of events, and the future must look at its own state again each time it is polled.

**Making a waker.** Implement the trait `Wake` for a type and put a value of that type in an `Arc`. `Waker::from(arc)` then builds the waker. Wakers are cloned, stored and sent to other threads, and every clone shares the same task through the `Arc`; this is why the trait requires an `Arc`.

`Wake` has two methods. `wake(self: Arc<Self>)` takes the `Arc` by value. `wake_by_ref(self: &Arc<Self>)` borrows it. You must write `wake`. The trait supplies a `wake_by_ref` that clones the `Arc` and calls `wake`; you can replace it when a cheaper version exists.

**Worked example.** This executor is as small as it can be. Its task is one flag, "poll me", and waking sets the flag. `run` polls only while the flag is set.

```rust
use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Wake, Waker};

struct PollMe(AtomicBool);

impl Wake for PollMe {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// Finishes at its third poll.
struct ThreeSteps {
    step: u8,
    forgets_to_wake: bool,
}

impl Future for ThreeSteps {
    type Output = &'static str;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<&'static str> {
        self.step += 1;
        if self.step == 3 {
            return Poll::Ready("done");
        }
        if !self.forgets_to_wake {
            cx.waker().wake_by_ref();
        }
        Poll::Pending
    }
}

/// Returns the output and the number of polls. The output is `None` if the
/// future is still pending and nobody has asked for another poll.
fn run(future: impl Future<Output = &'static str>) -> (Option<&'static str>, u32) {
    let task = Arc::new(PollMe(AtomicBool::new(true))); // the first poll is free
    let waker = Waker::from(Arc::clone(&task));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    let mut polls = 0;
    while task.0.swap(false, Ordering::SeqCst) {
        polls += 1;
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return (Some(output), polls);
        }
    }
    (None, polls)
}

fn main() {
    let polite = ThreeSteps { step: 0, forgets_to_wake: false };
    assert_eq!(run(polite), (Some("done"), 3));

    // Pending without a wake: the executor has no reason to poll again.
    let silent = ThreeSteps { step: 0, forgets_to_wake: true };
    assert_eq!(run(silent), (None, 1));
}
```

`ThreeSteps` wakes *itself*. That is correct here because each poll makes progress and the next step can start at once. The future is saying "give me another turn". A future that waits for something outside (a timer, a socket, another thread) must not wake itself. It stores the waker, and the outside event calls it. You build that in lesson 03.

**Mistakes people make.**

Returning `Pending` without a wake. The second half of the example shows the result: one poll, and then silence. With a real executor the program hangs.

Waking yourself without making progress. The future is then polled again and again and does nothing each time. This is a busy loop that looks like async code.

Calling `wake` twice on the same waker:

```rust,compile_fail,E0382
use std::sync::Arc;
use std::task::{Wake, Waker};

struct Bell;

impl Wake for Bell {
    fn wake(self: Arc<Self>) {
        println!("ring");
    }
}

fn main() {
    let waker = Waker::from(Arc::new(Bell));
    waker.wake();
    waker.wake();
}
```

```text
error[E0382]: use of moved value: `waker`
    |
 13 |     let waker = Waker::from(Arc::new(Bell));
    |         ----- move occurs because `waker` has type `Waker`, which does not implement the `Copy` trait
 14 |     waker.wake();
    |           ------ `waker` moved due to this method call
 15 |     waker.wake();
    |     ^^^^^ value used here after move
    |
note: `std::task::Waker::wake` takes ownership of the receiver `self`, which moves `waker`
```

`wake` consumes the waker. Use `waker.wake_by_ref()` when you want to keep it.

**When to use which.**

| You have | You want | Use |
| --- | --- | --- |
| `cx.waker()`, a `&Waker` lent for this poll | another turn now | `cx.waker().wake_by_ref()` |
| `cx.waker()` | to be woken later by an event | store `cx.waker().clone()` |
| a stored `Waker` that you take out | to wake it once | `waker.wake()` |

**Read more.** [`Waker`](https://doc.rust-lang.org/std/task/struct.Waker.html) · [`Wake`](https://doc.rust-lang.org/std/task/trait.Wake.html) · [`Context`](https://doc.rust-lang.org/std/task/struct.Context.html) · [`Waker::noop`](https://doc.rust-lang.org/std/task/struct.Waker.html#method.noop)

## Lesson 03 · A completion cell

**The model.** Most futures at the bottom of a program wait for an event that comes from somewhere else: another thread finishes a job, a timer runs out, bytes arrive. Two parties then share one piece of state.

```text
                    Arc<Mutex<Inner>>
 waiting side  ───► ┌───────────────────────┐ ◄───  completing side
 (the future,       │ the event: value,     │       (another thread, or
  polled by the     │   flag or counter     │        another task)
  executor)         │ waiter: Option<Waker> │
                    └───────────────────────┘
```

The waiting side polls: "has it happened? If not, here is my waker." The completing side records the event and calls the waker. Three rules make this correct.

**Rule 1: check and register under one lock.** If the check and the registration are two separate steps, the event can arrive between them:

```text
waiting side                         completing side
lock; "not yet"; unlock
                                     lock; record the event;
                                     no waker is stored; unlock
lock; store the waker; unlock
return Pending                       (nobody will ever call that waker)
```

This is a *lost wakeup*. With one lock around both steps, the completing side comes either before the check (then the poll sees the event) or after the registration (then it finds the waker).

**Rule 2: store the waker of every poll.** An executor may poll the same future with a different waker each time, for example after it has moved the task to another thread. Only the newest waker is certain to reach the task. Replace the stored one on every `Pending`.

**Rule 3: wake after unlocking.** Take the waker out of the shared state while you hold the lock, release the lock, and then call `wake`. The woken task may start at once on another thread, and its first action is to lock the same state. If you still hold the lock, it runs straight into a wall.

**Worked example.** A `Latch` becomes ready when `count_down` has been called a given number of times. The shared state is a counter instead of a value, but the three rules are the same.

```rust
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;

struct Inner {
    remaining: usize,
    waiter: Option<Waker>,
}

#[derive(Clone)]
struct Latch(Arc<Mutex<Inner>>);

impl Latch {
    fn new(count: usize) -> Latch {
        Latch(Arc::new(Mutex::new(Inner {
            remaining: count,
            waiter: None,
        })))
    }

    fn count_down(&self) {
        let waiter = {
            let mut inner = self.0.lock().unwrap();
            inner.remaining = inner.remaining.saturating_sub(1);
            if inner.remaining == 0 {
                inner.waiter.take()
            } else {
                None
            }
        }; // the guard is dropped here (rule 3)
        if let Some(waker) = waiter {
            waker.wake();
        }
    }
}

impl Future for Latch {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut inner = self.0.lock().unwrap(); // one guard for both steps (rule 1)
        if inner.remaining == 0 {
            return Poll::Ready(());
        }
        inner.waiter = Some(cx.waker().clone()); // always the newest (rule 2)
        Poll::Pending
    }
}

/// A waker that counts how often it is called.
struct Count(AtomicUsize);

impl Wake for Count {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn main() {
    let mut latch = Latch::new(2);
    let count = Arc::new(Count(AtomicUsize::new(0)));
    let waker = Waker::from(Arc::clone(&count));
    let mut cx = Context::from_waker(&waker);

    assert!(Pin::new(&mut latch).poll(&mut cx).is_pending());

    let workers: Vec<_> = (0..2)
        .map(|_| {
            let latch = latch.clone();
            thread::spawn(move || latch.count_down())
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }

    // Only the call that reached zero woke the task.
    assert_eq!(count.0.load(Ordering::SeqCst), 1);
    assert!(Pin::new(&mut latch).poll(&mut cx).is_ready());
}
```

**Mistakes people make.**

Storing the borrowed waker:

```rust,compile_fail,E0308
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

struct Inner {
    remaining: usize,
    waiter: Option<Waker>,
}

struct Latch(Arc<Mutex<Inner>>);

impl Future for Latch {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut inner = self.0.lock().unwrap();
        if inner.remaining == 0 {
            return Poll::Ready(());
        }
        inner.waiter = Some(cx.waker());
        Poll::Pending
    }
}
```

```text
error[E0308]: mismatched types
    |
 21 |         inner.waiter = Some(cx.waker());
    |                        ---- ^^^^^^^^^^ expected `Waker`, found `&Waker`
    |                        |
    |                        arguments to this enum variant are incorrect
...
help: consider using clone here
    |
 21 |         inner.waiter = Some(cx.waker().clone());
    |                                       ++++++++
```

`cx.waker()` is only lent to you for this poll. To keep it, clone it. For a waker that was made from an `Arc`, a clone only adds one to a reference count.

Locking twice on one thread. This compiles, and it never returns:

```rust,no_run
use std::sync::Mutex;

let state = Mutex::new((false, 0u8));
match state.lock().unwrap().0 {
    true => println!("ready"),
    false => state.lock().unwrap().1 = 1, // the first guard is still alive
}
```

The guard that `lock()` creates in the head of a `match` lives until the end of the whole `match`. The second `lock()` then waits for a lock that its own thread holds. The standard library says only that this call does not return: it may block forever or panic. Bind the guard to a variable once and use that variable for both steps, as `Latch::poll` does.

Keeping only the first waker (`if inner.waiter.is_none() { ... }`). It works in every test that polls with one waker, and it loses the task when the executor passes a new one.

**Read more.** [`Mutex::lock`](https://doc.rust-lang.org/std/sync/struct.Mutex.html#method.lock) · [`Option::take`](https://doc.rust-lang.org/std/option/enum.Option.html#method.take) · [`Waker::clone_from` and `will_wake`](https://doc.rust-lang.org/std/task/struct.Waker.html#method.will_wake) · [temporary scopes in the Rust Reference](https://doc.rust-lang.org/reference/destructors.html#temporary-scopes)

## Lesson 04 · A parker block_on

**The model.** You now have futures and wakers. The missing part is the caller: an *executor*. The simplest useful executor runs one future on the current thread and returns its output. Its loop has two steps:

1. Poll the future. If it is `Ready`, return the output.
2. If it is `Pending`, put the thread to sleep until the waker is called. Then go to step 1.

Step 2 has a trap. The wake can come from another thread at any moment, also *between* the poll and the moment the thread falls asleep. If that early wake is forgotten, the thread sleeps forever. So the sleeping mechanism must remember a wake that came too early.

**Worked example.** This executor uses a channel that holds at most one message. Waking puts a message in the channel. Sleeping is `recv`. A wake that comes early is simply a message that is already there.

```rust
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::Duration;

struct Notify(SyncSender<()>);

impl Wake for Notify {
    fn wake(self: Arc<Self>) {
        // A full channel means that a wake is already waiting. One is enough.
        let _ = self.0.try_send(());
    }
}

fn run<F: Future>(future: F) -> F::Output {
    let (tx, rx) = sync_channel(1);
    let waker = Waker::from(Arc::new(Notify(tx)));
    let mut cx = Context::from_waker(&waker);
    // The future stays in this stack frame, so it can be pinned here.
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
        // Sleep until a wake arrives. `waker` keeps the sender alive, so
        // `recv` cannot fail.
        rx.recv().expect("the waker is still alive");
    }
}

fn main() {
    assert_eq!(run(async { 6 * 7 }), 42);

    // A future that another thread completes 20 ms later.
    let done = Arc::new(AtomicBool::new(false));
    let mut started = false;
    let wait = std::future::poll_fn(|cx| {
        if done.load(Ordering::SeqCst) {
            return Poll::Ready("woken");
        }
        if !started {
            // Keeping only the first waker is safe here, because `run`
            // passes the same waker to every poll.
            started = true;
            let (done, waker) = (Arc::clone(&done), cx.waker().clone());
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(20));
                done.store(true, Ordering::SeqCst);
                waker.wake();
            });
        }
        Poll::Pending
    });
    assert_eq!(run(wait), "woken");
}
```

**Parking.** In the lesson you use a lighter mechanism than a channel. Every thread has a built-in *token*, which is either present or absent.

- `thread::park()` blocks the current thread until its token is present. Then it removes the token and returns.
- `some_thread.unpark()` makes that thread's token present. If the thread is parked, it continues. If it is not parked yet, its next `park()` returns at once.

To call `unpark` you need a handle to the thread. `thread::current()` returns such a handle (a `Thread`) for the thread that calls it. The handle can be stored and sent to other threads, so a thread can take its own handle and give it to whoever must wake it later.

The token plays the part of the one-message channel: it remembers one early wake. `park` has one more property that you must know: it may return although nobody called `unpark`. This is called a *spurious* wakeup. For this reason the code around `park` is always a loop that checks a condition:

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

let ready = Arc::new(AtomicBool::new(false));
let sleeper = {
    let ready = Arc::clone(&ready);
    thread::spawn(move || {
        while !ready.load(Ordering::SeqCst) {
            thread::park(); // after any return, check the condition again
        }
    })
};
ready.store(true, Ordering::SeqCst);
sleeper.thread().unpark();
sleeper.join().unwrap();
```

This program ends whether `unpark` runs before or after the `park`. In an executor the "condition" is the future itself: after `park` returns, poll again. If the future is still `Pending`, park again.

**Mistakes people make.**

Using `Waker::noop()` in an executor that sleeps. The future keeps its side of the contract and calls the waker, but that waker does nothing. The thread is never unparked and the program hangs. `Waker::noop()` is only for code that polls a fixed number of times, as tests do.

Polling in a loop without sleeping, or sleeping for a fixed time and trying again. Both give the right result. The first burns a processor core. The second adds a delay to every wake and still polls when nothing has happened. The tests of this lesson count polls to find both.

Pinning with `Pin::new` in a generic executor:

```rust,compile_fail,E0277
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

fn poll_once<F: Future>(mut future: F) -> Poll<F::Output> {
    let mut cx = Context::from_waker(Waker::noop());
    Pin::new(&mut future).poll(&mut cx)
}
```

```text
error[E0277]: `F` cannot be unpinned
     |
   7 |     Pin::new(&mut future).poll(&mut cx)
     |     -------- ^^^^^^^^^^^ the trait `Unpin` is not implemented for `F`
     |     |
     |     required by a bound introduced by this call
     |
     = note: consider using the `pin!` macro
             consider using `Box::pin` if you need to access the pinned value outside of the current scope
```

`Pin::new` works only for `Unpin` types, and an executor must accept every future. Write `let mut future = pin!(future);` and poll with `future.as_mut().poll(&mut cx)`.

**What this executor is not.** It runs one future. A real runtime also has a queue of many tasks, a way to spawn new ones, timers, a reactor for I/O (lesson 05), and several worker threads. You build those parts in the mastery track, in [`mastery/m4_async`](../../mastery/m4_async/).

**Read more.** [`thread::park`](https://doc.rust-lang.org/std/thread/fn.park.html) · [`Thread::unpark`](https://doc.rust-lang.org/std/thread/struct.Thread.html#method.unpark) · [`pin!`](https://doc.rust-lang.org/std/pin/macro.pin.html) · [`poll_fn`](https://doc.rust-lang.org/std/future/fn.poll_fn.html) · [`sync_channel`](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html)

## Lesson 05 · Readiness registration model

**The model.** A thread that calls `read` on an ordinary socket sleeps inside the operating system until bytes arrive. A future must not do that, because `poll` has to return quickly. Async I/O therefore uses sockets in *nonblocking* mode. A nonblocking `read` never sleeps. If no bytes are there, it returns an error of kind `ErrorKind::WouldBlock`, which means "not now, try again later".

"Later" needs somebody who knows when. That part of a runtime is the *reactor*. It asks the operating system to watch many sockets at once, sleeps until at least one of them has news, and wakes the tasks that wait for those sockets.

```text
 task (a future)             reactor                        operating system
 try to read: WouldBlock
 register my waker  ───────► table: socket 7 → waker
 return Pending              wait for events  ◄───────────  "socket 7 is readable"
                             take the waker, wake it
 polled again
 try to read: some bytes
 (or WouldBlock again: register again, return Pending)
```

The important word in the picture is *try*. The reactor reports *readiness*: "an operation on this socket will probably not block now". It does not deliver data, and it does not promise that the operation will succeed.

- Readiness can be out of date. Another reader may have taken the bytes first.
- Readiness says nothing about how much data is there. Four bytes of a hundred-byte message make a socket readable.

So an I/O future always performs the real operation, and it treats `WouldBlock` as "register and return `Pending`", every time.

The lesson reduces this to its core: one `ready` flag and one waiter. The registration rules are those of lesson 03: check the flag and store the waker under one lock, and wake after unlocking.

**Worked example.** `Pipe` behaves like a nonblocking source that lives in memory. `notify` plays the reactor. Its second call below is a wake with no data behind it, which a real reactor can also produce.

```rust
use std::collections::VecDeque;
use std::future::Future;
use std::io::{self, ErrorKind};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

#[derive(Default)]
struct Pipe {
    bytes: VecDeque<u8>,
    reader: Option<Waker>,
}

/// Like a nonblocking read: it never waits.
fn try_read(pipe: &mut Pipe) -> io::Result<u8> {
    pipe.bytes
        .pop_front()
        .ok_or_else(|| ErrorKind::WouldBlock.into())
}

/// The reactor's part: something may have changed, so wake the reader.
fn notify(shared: &Arc<Mutex<Pipe>>, byte: Option<u8>) {
    let reader = {
        let mut pipe = shared.lock().unwrap();
        pipe.bytes.extend(byte);
        pipe.reader.take()
    };
    if let Some(waker) = reader {
        waker.wake();
    }
}

struct ReadByte(Arc<Mutex<Pipe>>);

impl Future for ReadByte {
    type Output = io::Result<u8>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u8>> {
        let mut pipe = self.0.lock().unwrap();
        match try_read(&mut pipe) {
            // Not ready: register under the same lock as the attempt.
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                pipe.reader = Some(cx.waker().clone());
                Poll::Pending
            }
            // A byte, or a real error: both end the future.
            result => Poll::Ready(result),
        }
    }
}

struct Count(AtomicUsize);

impl Wake for Count {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn main() {
    let shared = Arc::new(Mutex::new(Pipe::default()));
    let mut read = ReadByte(Arc::clone(&shared));
    let wakes = Arc::new(Count(AtomicUsize::new(0)));
    let waker = Waker::from(Arc::clone(&wakes));
    let mut cx = Context::from_waker(&waker);

    assert!(Pin::new(&mut read).poll(&mut cx).is_pending());

    // Woken, but there is nothing to read. The future must cope with that.
    notify(&shared, None);
    assert_eq!(wakes.0.load(Ordering::SeqCst), 1);
    assert!(Pin::new(&mut read).poll(&mut cx).is_pending());

    // The poll above registered again, so this wake arrives too.
    notify(&shared, Some(7));
    assert_eq!(wakes.0.load(Ordering::SeqCst), 2);
    assert!(matches!(Pin::new(&mut read).poll(&mut cx), Poll::Ready(Ok(7))));
}
```

**Mistakes people make.**

Treating the wake as the data. Code that reads with `.expect("the reactor said it was ready")` after a wake panics on the first out-of-date wake. A wake means "try again", nothing more.

Expecting a whole message. One successful `read` returns the bytes that are there at that moment, and that can be fewer than you asked for. Code that needs exactly `n` bytes has to keep what it got and continue after the next wake. The predictions of this lesson let you measure the difference between `read` and `read_exact`.

Comparing an I/O error with an error kind:

```rust,compile_fail,E0369
use std::io::{self, ErrorKind, Read};

fn try_read(source: &mut impl Read, buf: &mut [u8]) -> io::Result<Option<usize>> {
    match source.read(buf) {
        Ok(n) => Ok(Some(n)),
        Err(e) if e == ErrorKind::WouldBlock => Ok(None),
        Err(e) => Err(e),
    }
}
```

```text
error[E0369]: binary operation `==` cannot be applied to type `std::io::Error`
   |
 6 |         Err(e) if e == ErrorKind::WouldBlock => Ok(None),
   |                   - ^^ --------------------- ErrorKind
   |                   |
   |                   std::io::Error
   |
note: `std::io::Error` does not implement `PartialEq<ErrorKind>`
```

An `io::Error` is more than its kind. Ask for the kind: `e.kind() == ErrorKind::WouldBlock`.

**When to use which.** The lesson's event is *level-like*: the flag stays set, and every later poll is `Ready`. This suits "the socket is open" or "shutdown was requested". The `Pipe` above is *consuming*: each byte is taken once, and the reader registers again for the next one. Most real I/O is of the second kind, and it needs the "try, and register again on `WouldBlock`" loop that you see in `ReadByte`.

**Read more.** [`ErrorKind::WouldBlock`](https://doc.rust-lang.org/std/io/enum.ErrorKind.html#variant.WouldBlock) · [`TcpStream::set_nonblocking`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.set_nonblocking) · [the `Read` trait](https://doc.rust-lang.org/std/io/trait.Read.html) and its [`read_exact`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact). Lab 2 in [LABS.md](LABS.md) builds a reactor over real sockets.

## Lesson 06 · Cooperative work budgets

**The model.** An operating system can stop a thread at any instruction and give the processor to another thread. An executor cannot do that with a task. A task runs from the start of one `poll` call to its `return`, and nothing can interrupt it in between. Tasks on one executor thread therefore take turns only if each of them gives up its turn. This is called *cooperative* scheduling.

A future that computes for one second inside a single `poll` keeps every other task on its thread waiting for that second: no timer fires, and no socket is served. The type `Future` does not protect you. The remedy is a *budget*: do a bounded amount of work for each poll, and when the budget is used up and work remains, stop and ask for another turn.

"Ask for another turn" is `cx.waker().wake_by_ref()` followed by `Poll::Pending`. Here the self-wake from lesson 02 is exactly right: the poll made progress, and the next piece of work can start at once. The task goes to the back of the queue instead of waiting for an outside event.

**Worked example.** Each `Worker` has some units of work and a budget. It writes its name to a log for every unit, so the log shows the order in which the work was done. `round_robin` polls every unfinished task in turn.

```rust
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

struct Worker<'a> {
    name: char,
    left: u32,
    budget: u32,
    log: &'a RefCell<String>,
}

impl Future for Worker<'_> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let units = self.left.min(self.budget);
        for _ in 0..units {
            self.log.borrow_mut().push(self.name); // one unit of work
        }
        self.left -= units;
        if self.left == 0 {
            return Poll::Ready(());
        }
        cx.waker().wake_by_ref(); // more work is waiting: ask for another turn
        Poll::Pending
    }
}

/// Polls each unfinished task in turn. It does not look at wakes, which is
/// only correct because these tasks always wake themselves.
fn round_robin(mut tasks: Vec<Pin<Box<dyn Future<Output = ()> + '_>>>) {
    let mut cx = Context::from_waker(Waker::noop());
    while !tasks.is_empty() {
        tasks.retain_mut(|task| task.as_mut().poll(&mut cx).is_pending());
    }
}

fn main() {
    let log = RefCell::new(String::new());

    // Both tasks stop after 2 units. The short task `b` is finished early.
    round_robin(vec![
        Box::pin(Worker { name: 'a', left: 6, budget: 2, log: &log }),
        Box::pin(Worker { name: 'b', left: 2, budget: 2, log: &log }),
    ]);
    assert_eq!(*log.borrow(), "aabbaaaa");

    // Task `a` has no limit now. `b` has to wait for all of it.
    log.borrow_mut().clear();
    round_robin(vec![
        Box::pin(Worker { name: 'a', left: 6, budget: u32::MAX, log: &log }),
        Box::pin(Worker { name: 'b', left: 2, budget: 2, log: &log }),
    ]);
    assert_eq!(*log.borrow(), "aaaaaabb");
}
```

The total work is the same in both runs. The budget changes how long `b` waits: two units in the first run, six in the second.

**Mistakes people make.**

Stopping at the budget without a wake. The future returns `Pending`, nothing will ever wake it, and it is never polled again. A test loop that polls blindly hides this bug. A real executor shows it as a hang.

Taking a chunk that goes past the end. With 5 elements, position 4 and budget 2, the slice `values[4..6]` does not exist:

```text
range end index 6 out of range for slice of length 5
```

That is a panic at run time. Limit the end of each chunk to the length, for example with `min`.

Adding in a type that is too small. A sum of `u32` values can be larger than `u32::MAX`. In a debug build the addition panics with `attempt to add with overflow`. In a release build it wraps around and gives a wrong sum. Convert each value to the wider type *before* you add it, with `u64::from(value)`.

Reading one field and writing another in a loop, through the pin:

```rust,compile_fail,E0502
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Longest {
    words: Vec<String>,
    next: usize,
    best: usize,
}

impl Future for Longest {
    type Output = usize;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<usize> {
        let end = (self.next + 2).min(self.words.len());
        for word in &self.words[self.next..end] {
            self.best = self.best.max(word.len());
        }
        self.next = end;
        if self.next == self.words.len() {
            return Poll::Ready(self.best);
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
```

```text
error[E0502]: cannot borrow `self` as mutable because it is also borrowed as immutable
   |
15 |           for word in &self.words[self.next..end] {
   |           -           ---------------------------
   |           |           ||
   |           |           |immutable borrow occurs here
   |  _________|           immutable borrow later used here
   | |
16 | |             self.best = self.best.max(word.len());
   | |             ^^^^ mutable borrow occurs here
17 | |         }
   | |_________- this for loop borrows `self` immutably, preventing mutation within its body
```

With a plain `&mut Longest` this loop is fine, because the compiler sees two different fields. Through a `Pin`, every field access goes through `Deref` or `DerefMut` on the *whole* pin, so the two borrows collide. For an `Unpin` type, turn the pin into a plain reference once, at the top of `poll`: `let this = &mut *self;`. Then use `this.words` and `this.best`. Another way is to compute the result of the chunk first and store it afterwards.

**When to use which.** Use a budget and a self-wake for work that can be cut into small pieces. Give work that cannot be cut, or that calls blocking functions, to another thread and wait for it with a completion cell (lesson 03; task B of the review lesson does this). A budget tells you how the work is divided. It tells you nothing about speed: the number of polls is not a time measurement.

**Read more.** [`Waker::wake_by_ref`](https://doc.rust-lang.org/std/task/struct.Waker.html#method.wake_by_ref) · [`usize::min`](https://doc.rust-lang.org/std/primitive.usize.html#method.min) and [`saturating_add`](https://doc.rust-lang.org/std/primitive.usize.html#method.saturating_add) · [`u64::from`](https://doc.rust-lang.org/std/convert/trait.From.html) · [`Pin` and `DerefMut`](https://doc.rust-lang.org/std/pin/struct.Pin.html#impl-DerefMut-for-Pin%3CPtr%3E)

## Lesson 07 · Capstone: pinned future adapter

**The model.** So far every future that you wrote was `Unpin`, and `Pin` did not get in your way. The capstone wraps a future of *any* type, also one that must not move. This section explains why such futures exist and which tool the capstone uses for them.

**Why some futures must not move.** The compiler turns an `async` block into a struct that holds every local variable that is alive across an `.await`. One of those variables can be a reference to another one:

```text
the state of the future, while it waits at the .await
┌─────────────────────────────┐
│ text: String                │ ◄─┐
│ view: &String  ─────────────┼───┘  a pointer into the same struct
└─────────────────────────────┘
```

If this struct were copied to a new address, `view` would still point to the old one. So the future has two stages in its life. Before the first poll it has not started, it holds no such reference, and it can move freely: you can return it from a function or put it in a box. From the first poll on it must stay where it is. `Pin<&mut Self>` in the signature of `poll` is how the caller makes that promise.

The compiler does not look inside the block to see whether such a reference really exists. *Every* async block and every future of an `async fn` is `!Unpin` (not `Unpin`):

```rust,compile_fail,E0277
fn needs_unpin<T: Unpin>(_: &T) {}

fn main() {
    let plain = async { 1 };
    needs_unpin(&plain);
}
```

```text
error[E0277]: `{async block@src/main.rs:4:17: 4:22}` cannot be unpinned
  |
5 |     needs_unpin(&plain);
  |     ----------- ^^^^^^ the trait `Unpin` is not implemented for `{async block@src/main.rs:4:17: 4:22}`
  |     |
  |     required by a bound introduced by this call
  |
  = note: consider using the `pin!` macro
          consider using `Box::pin` if you need to access the pinned value outside of the current scope
```

**The tool: a pinned box.** `Box::pin(future)` moves the future to the heap, once, before its first poll, and gives you a `Pin<Box<F>>`. Three properties make this the easy way to hold a future of unknown type.

```rust
use std::future::Future;
use std::task::{Context, Poll, Waker};

fn is_unpin<T: Unpin>(_: &T) -> bool {
    true
}

fn main() {
    let boxed = Box::pin(async {
        let text = String::from("kept across the await");
        let view = &text; // a reference into the future's own state
        std::future::ready(()).await;
        view.len()
    });

    // 1. The pinned box is `Unpin`, although the future inside it is not.
    assert!(is_unpin(&boxed));

    // 2. Moving the box moves a pointer. The future keeps its address.
    let address = &*boxed as *const _ as usize;
    let mut moved = boxed;
    assert_eq!(&*moved as *const _ as usize, address);

    // 3. `as_mut` turns `&mut Pin<Box<F>>` into the `Pin<&mut F>` that `poll` needs.
    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(moved.as_mut().poll(&mut cx), Poll::Ready(21));
}
```

**The approach for the capstone.** Your wrapper has two fields: the inner future in a pinned box, and a value of its own that it gives back together with the inner output. Think about these points before you write code.

- Is the wrapper `Unpin`? A struct is `Unpin` when all its fields are. Use property 1 for the first field. The answer decides whether `get_mut()` is available in `poll`.
- How do you poll the inner future without moving it and without allocating again? Property 3.
- When may the wrapper's own value leave? The wrapper can be polled many times before the inner future is ready, and it must still have the value at the end. In lesson 01 you moved a value out of a field exactly once; the same tool works here.
- What does `Pending` from the inner future mean for the wrapper? The inner future was polled with *your* `cx`, so it has already arranged the wake. The wrapper only passes `Pending` on.

**Mistakes people make.**

Asking for `F: Unpin` so that the code compiles. The wrapper then rejects every async block, and the first test fails to build. The box is what makes the bound unnecessary.

Taking the wrapper's value out before the inner future is ready. The first `Pending` then loses it.

Creating a new box in `poll`. The future must be pinned once, when the wrapper is built. The tests count allocations to check this.

**When to use which.** A pinned box costs one allocation, and it lets you write the wrapper without any `unsafe` code. Library code that wants to avoid the allocation stores the inner future directly in the wrapper struct and creates a `Pin<&mut F>` to that field. This is called *pin projection*. It needs `unsafe` code with a careful proof, or a crate that generates the code for you. You write it both ways in the mastery track, in [`mastery/m4_async`](../../mastery/m4_async/).

**Read more.** [the `std::pin` module](https://doc.rust-lang.org/std/pin/index.html) · [`Box::pin`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.pin) · [`Pin::as_mut`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.as_mut) · [`Unpin`](https://doc.rust-lang.org/std/marker/trait.Unpin.html) · [`PhantomPinned`](https://doc.rust-lang.org/std/marker/struct.PhantomPinned.html)
