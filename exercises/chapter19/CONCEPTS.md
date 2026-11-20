# Chapter 19 concepts — Threads and shared state

[Chapter guide](README.md) · [Hints](HINTS.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every code block in this file was compiled and
run with the course toolchain (Rust 1.99.0, edition 2024), and every compiler
message was copied from that compiler. Long messages are shortened: a line with
`...` marks a cut in the middle, and the end of a message is often left out.

## What you will be able to do

After this chapter you can:

- start threads, wait for them, and tell a worker that failed from a worker
  that panicked;
- let threads borrow local data safely with `thread::scope`, and split a slice
  so that two threads can change it at the same time;
- move values between threads through channels, and explain exactly when a
  receiving loop ends;
- read the `Send` and `Sync` errors of the compiler and repair their cause;
- protect shared data with `Mutex` and `RwLock`, wait for a condition with
  `Condvar`, and recover from a poisoned lock;
- count and publish with atomics, write a compare-and-swap loop, choose a
  memory ordering, and initialize a value exactly once.

## Before you start

Ownership, moves and `Arc` (Chapter 4), references
and `split_at_mut` (Chapter 5), `Result` and `?` (Chapter 7), `RefCell` and
interior mutability (Chapter 9), traits and trait bounds (Chapters 11 and 13),
closures and the `Fn` traits (Chapter 14), and iterators (Chapter 15). The
review lesson also uses maps (Chapter 16) and buffered readers (Chapter 18).

## Time

These are estimates for a learner who meets threads in Rust for the
first time. They include reading the section, the predictions, the code, and
your notes.

| Lesson | Estimate |
| --- | --- |
| 01 · Fork, join, and worker failures | 45 minutes |
| 02 · Borrowed parallel work | 50 minutes |
| 03 · Ownership and channel closure | 50 minutes |
| 04 · Send and Sync contracts | 40 minutes |
| 05 · Mutex invariants and poisoning | 60 minutes |
| 06 · RwLock and condition predicates | 60 minutes |
| 07 · Atomic counters and local state | 50 minutes |
| 09 · Retry loops, orderings, and one-time initialization | 80 minutes |
| 08 · Capstone | 60 minutes |
| R01 review, C01 code review, drills | 2 hours |

## Words used in this chapter

- **Thread**: one line of execution inside a program, with its own call stack.
  All threads of a program share the same heap and the same statics.
- **Main thread**: the thread that runs `main`. When it returns, the program
  ends, even if other threads are still running.
- **Worker**: a thread that you start to do one piece of work.
- **Join**: to wait until a thread has finished, and to collect its result.
- **Data race**: two threads use the same memory at the same time, at least
  one of them writes, and nothing orders the two accesses. In Rust a data race
  is undefined behavior, and safe code cannot contain one. The compiler errors
  in this chapter are how it keeps that promise.
- **Race condition**: a wider word. The result of a correct-looking program
  depends on timing. Safe Rust can still have race conditions and deadlocks.
- **Deadlock**: threads wait for each other, and none of them can continue.

## Lesson 01 · Fork, join, and worker failures

### The idea

`std::thread::spawn(f)` starts a new thread of the operating system and runs
the closure `f` on it. The new thread has its own stack. It runs at the same
time as the thread that started it, on another processor core if one is free.
You cannot know which of the two threads runs first or faster.

The bounds on `spawn` tell you what the closure may contain:

```text
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
```

`'static` means that the closure borrows nothing from the function that calls
`spawn`. The new thread may still be running after that function has returned
and its local variables are gone. So the closure must own what it uses, and in
practice you write `move` in front of it. `Send` is the subject of lesson 04;
for now, read it as "may be handed to another thread".

`spawn` returns a `JoinHandle<T>`. Its method `join` waits until the thread has
finished. It returns `Ok(value)` with the value that the closure returned, or
`Err(payload)` if the thread panicked. A panic in a worker does not stop the
other threads. It ends that one thread, and `join` is how you find out.

### Worked example

The main thread gives a string to a worker, which parses it.

```rust
use std::thread;

fn main() {
    let raw = String::from(" 8080 ");
    // `move` gives `raw` to the worker. The main thread cannot use it again.
    let parser = thread::spawn(move || raw.trim().parse::<u16>());

    // The main thread is free to do other work here, at the same time.

    match parser.join() {
        Ok(Ok(port)) => assert_eq!(port, 8080),
        Ok(Err(parse_error)) => panic!("the text was not a port: {parse_error}"),
        Err(_payload) => panic!("the parser thread panicked"),
    }
}
```

```text
main thread     spawn ─────────── (other work) ─────────── join ── match
                  │                                          ▲
                  │ owns `raw` now                           │ Result<Result<u16, ParseIntError>, payload>
worker            └──► trim, parse ── returns ───────────────┘
```

Look at the type that `join` returns here. The closure itself returns a
`Result`, and `join` wraps it in a second one. The outer `Result` answers
"did the thread finish normally?". The inner one answers "did the work
succeed?". These are different failures. A parse error is an ordinary value
that the worker chose to return. A panic means the worker never reached its
`return`.

The payload of a panic has the type `Box<dyn Any + Send>`. Usually it is the
panic message, stored either as a `&'static str` or as a `String`, depending
on how the message was built. `payload.downcast_ref::<&str>()` and
`payload.downcast_ref::<String>()` let you test for each.

**Start all, then join all.** With several workers, keep every handle. First
start all the workers and put their handles into a `Vec`. Then, in a second
pass, join the handles in the same order. The order of the handles is the
order of your inputs, so the results come back in input order, even if the
last worker finished first. If you call `join` right after each `spawn`, the
next worker does not start until the previous one has finished, and nothing
runs in parallel.

**Dropping a handle.** A `JoinHandle` that is dropped without `join` leaves
the thread running on its own. The thread is now *detached*: nobody can wait
for it or read its result.

### Mistakes people make

**The closure borrows a local variable.**

```rust,compile_fail,E0373
use std::thread;

fn main() {
    let readings = vec![21.5, 22.0, 19.8];
    let worker = thread::spawn(|| readings.iter().sum::<f64>());
    println!("{}", worker.join().unwrap());
}
```

```text
error[E0373]: closure may outlive the current function, but it borrows `readings`, which is owned by the current function
 --> src/main.rs:5:32
  |
5 |     let worker = thread::spawn(|| readings.iter().sum::<f64>());
  |                                ^^ -------- `readings` is borrowed here
  |                                |
  |                                may outlive borrowed value `readings`
  |
note: function requires argument type to outlive `'static`
...
help: to force the closure to take ownership of `readings` (and any other referenced variables), use the `move` keyword
```

Fix: write `move ||`. If the main thread also needs the data afterwards, give
the worker its own `Arc` clone (Chapter 4), or use a scoped thread (lesson 02).

**Returning from `main` without joining.** The program ends when `main`
returns. Workers that are still running are stopped in the middle of their
work, without any message. Join every worker whose work matters.

**Calling `unwrap` on `join` without thinking.** `handle.join().unwrap()`
panics in the parent when the worker panicked. That is fine when a worker
panic is a bug that should stop the program. It is wrong when the caller was
promised an error value; then map the `Err` to your error type.

### When to use which

- Use `thread::spawn` when the thread must live longer than the function that
  starts it, for example a background task. The closure must own its data.
- Use `thread::scope` (lesson 02) when the workers finish before the function
  returns. They may then borrow.
- An operating-system thread costs a stack and some time to create. Tens of
  threads are fine. For thousands of small tasks, use a thread pool (optional
  lab 1) or asynchronous tasks (Chapter 20).

### Read more

- [`std::thread`](https://doc.rust-lang.org/std/thread/index.html)
- [`thread::spawn`](https://doc.rust-lang.org/std/thread/fn.spawn.html)
- [`JoinHandle`](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html)
- [`thread::Builder`](https://doc.rust-lang.org/std/thread/struct.Builder.html), for named threads

## Lesson 02 · Borrowed parallel work

### The idea

`thread::spawn` forces the closure to own everything, because nobody promises
that the thread ends in time. `thread::scope` makes that promise:

```text
thread::scope(|s| {
    s.spawn(|| ...);     // may borrow local variables
    s.spawn(|| ...);
});                      // returns only after every thread spawned on `s` has finished
```

Every thread that you start through `s` is joined before `scope` returns. So a
scoped thread can never outlive the variables that existed before the scope
began, and the compiler lets it borrow them. No `'static`, no `Arc`.

The borrowing rules do not change because threads are involved. Any number of
threads may share `&T`. Only one thread may hold `&mut T`, and nobody else may
read at that time. To let two threads write, give each one a *different* part
of the data. `split_at_mut` and `chunks_mut` cut a slice into parts that cannot
overlap, and each part is its own `&mut`.

### Worked example

Two workers read the same slice. Three workers each write one row of a grid.

```rust
use std::thread;

fn main() {
    let temperatures = [18.5_f64, 21.0, 19.2, 24.8, 22.1];

    let (highest, warm_days) = thread::scope(|s| {
        // Both closures borrow `temperatures`. Shared borrows may overlap.
        let highest = s.spawn(|| temperatures.iter().copied().fold(f64::MIN, f64::max));
        let warm_days = s.spawn(|| temperatures.iter().filter(|t| **t > 20.0).count());
        (highest.join().unwrap(), warm_days.join().unwrap())
    });
    assert_eq!((highest, warm_days), (24.8, 3));

    let mut grid = vec![0_u8; 12]; // 3 rows of 4 cells
    thread::scope(|s| {
        for (row, cells) in grid.chunks_mut(4).enumerate() {
            // `move` gives this worker its own `row` and its own `&mut [u8]`.
            s.spawn(move || cells.fill(row as u8));
        }
    }); // all three workers are joined here
    assert_eq!(grid, [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2]);
}
```

```text
grid:   [ 0 0 0 0 | 1 1 1 1 | 2 2 2 2 ]
          └─ &mut ─┘ └─ &mut ─┘ └─ &mut ─┘     three borrows, no overlap
           worker 0   worker 1   worker 2
```

`s.spawn` returns a `ScopedJoinHandle`. Call `join` on it when you need the
value that the worker returned, or when you want to handle a worker panic
yourself as an `Err`. Workers that you do not join are joined by the scope.
If one of those panicked, `thread::scope` itself panics after all workers have
finished:

```rust,should_panic
use std::thread;

thread::scope(|s| {
    s.spawn(|| panic!("worker failed"));
}); // panics here with "a scoped thread panicked"
```

### Mistakes people make

**Two closures capture the same collection mutably.**

```rust,compile_fail,E0499
use std::thread;

fn main() {
    let mut tally = vec![0_u32; 4];
    thread::scope(|s| {
        s.spawn(|| tally[0] += 1);
        s.spawn(|| tally[3] += 1);
    });
}
```

```text
error[E0499]: cannot borrow `tally` as mutable more than once at a time
   --> src/main.rs:7:17
    |
  5 |     thread::scope(|s| {
    |                    - has type `&'1 Scope<'1, '_>`
  6 |         s.spawn(|| tally[0] += 1);
    |         -------------------------
    |         |       |  |
    |         |       |  first borrow occurs due to use of `tally` in closure
    |         |       first mutable borrow occurs here
    |         argument requires that `tally` is borrowed for `'1`
  7 |         s.spawn(|| tally[3] += 1);
    |                 ^^ ----- second borrow occurs due to use of `tally` in closure
    |                 |
    |                 second mutable borrow occurs here
```

The compiler does not look at the indexes. Each closure borrows the whole
vector. Fix: split first (`let (left, right) = tally.split_at_mut(2);`) and
let each closure use one part.

**A closure borrows a variable of the loop that spawns it.**

```rust,compile_fail,E0373
use std::thread;

fn main() {
    let names = ["ada", "lin", "sol"];
    thread::scope(|s| {
        for i in 0..names.len() {
            s.spawn(|| println!("{i}: {}", names[i]));
        }
    });
}
```

```text
error[E0373]: closure may outlive the current function, but it borrows `i`, which is owned by the current function
 --> src/main.rs:7:21
  |
...
7 |             s.spawn(|| println!("{i}: {}", names[i]));
  |                     ^^                           - `i` is borrowed here
  |                     |
  |                     may outlive borrowed value `i`
...
help: to force the closure to take ownership of `i` (and any other referenced variables), use the `move` keyword
```

`i` lives for one turn of the loop, which is shorter than the scope. Fix:
`move ||`. But `move` moves *every* captured variable. If the closure should
still only borrow something large, make a reference first and let `move` copy
the reference: `let names = &names;` before the loop.

**Joining inside the scope too early.** `s.spawn(...).join()` followed by a
second `s.spawn(...).join()` runs the two workers one after the other. Start
both, then join.

### When to use which

- `thread::scope`: parallel work on data that the calling function owns or
  borrows, finished before the function returns. This is the usual choice for
  "split the work, wait, combine".
- `thread::spawn` with `Arc`: the thread outlives the caller, or its lifetime
  is not tied to one function call. `Arc::clone` makes a second owner of the
  same allocation; it does not copy the data.

### Read more

- [`thread::scope`](https://doc.rust-lang.org/std/thread/fn.scope.html)
- [`slice::split_at_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_at_mut) and [`chunks_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks_mut)
- [`ScopedJoinHandle`](https://doc.rust-lang.org/std/thread/struct.ScopedJoinHandle.html)

## Lesson 03 · Ownership and channel closure

### The idea

A **channel** is a queue with two ends. `std::sync::mpsc::channel()` returns a
`Sender<T>` and a `Receiver<T>`. "mpsc" stands for "multiple producer, single
consumer": you may clone the sender as often as you like, but there is exactly
one receiver.

- `tx.send(value)` *moves* the value into the queue. The sending thread no
  longer owns it. This is why channels fit Rust so well: a value is owned by
  the sender, then by the queue, then by the receiver, never by two at once.
- `rx.recv()` waits for the next value. It returns `Err` only when the queue
  is empty **and** every sender has been dropped. Then no value can ever
  arrive, and the channel is *closed* (the documentation says *disconnected*).
- A `for value in rx` loop calls `recv` again and again and ends when the
  channel is closed.

A channel closes because of ownership, not because of a call. There is no
`close` method. The receiver learns that the work is over when the last
`Sender` is dropped.

### Worked example

Two stations send readings. The main thread adds them up.

```rust
use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel::<(u8, u32)>();

    for station in 1..=2_u8 {
        let tx = tx.clone(); // one sender per producer
        thread::spawn(move || {
            for reading in [10, 20, 30] {
                tx.send((station, reading)).unwrap();
            }
        }); // the producer's sender is dropped when its closure ends
    }
    drop(tx); // the original sender: without this line the loop below never ends

    let mut total = 0;
    for (_station, reading) in rx {
        total += reading;
    }
    assert_eq!(total, 120);
}
```

```text
station 1 ── tx clone ──┐
                        ├──► [ queue ] ──► rx ── main thread: for ... in rx
station 2 ── tx clone ──┘
main      ── tx (original): dropped by hand, or the queue never closes
```

The values of one station arrive in the order in which that station sent
them. The values of different stations are mixed in an order that changes
from run to run. That is why the example adds numbers and does not compare a
list.

**Bounded channels.** `channel()` has no size limit: `send` never waits, and a
fast producer can fill memory. `mpsc::sync_channel(n)` holds at most `n`
waiting values. When it is full, `send` blocks until the receiver has taken
one. This is called *backpressure*: a slow consumer slows the producers down.
With `n == 0`, every `send` waits until a `recv` takes the value directly.

**Not waiting.** `try_recv` returns at once. Its error says why there is no
value: `TryRecvError::Empty` (nothing yet, senders exist) or
`TryRecvError::Disconnected` (closed). `recv_timeout` waits for a limited
time. On the sending side, `send` fails when the receiver is gone, and the
error gives the value back to you: `Err(SendError(value))`.

### Mistakes people make

**A sender that nobody dropped.** The program compiles and then waits
forever. There is no message. The receiving loop is waiting for a value that
no thread will send, because one `Sender` is still alive somewhere. Count the
senders: the original, every clone, and every clone that a closure captured.
The usual culprit is the original in the thread that also receives.

**Sharing the receiver.**

```rust,compile_fail,E0277
use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel::<u32>();
    thread::scope(|s| {
        s.spawn(|| rx.recv());
        s.spawn(|| rx.recv());
    });
    drop(tx);
}
```

```text
error[E0277]: `std::sync::mpsc::Receiver<u32>` cannot be shared between threads safely
   --> src/main.rs:7:17
    |
  7 |         s.spawn(|| rx.recv());
    |           ----- ^^^^^^^^^^^^ `std::sync::mpsc::Receiver<u32>` cannot be shared between threads safely
    |           |
    |           required by a bound introduced by this call
    |
    = help: the trait `Sync` is not implemented for `std::sync::mpsc::Receiver<u32>`
    = note: required for `&std::sync::mpsc::Receiver<u32>` to implement `Send`
```

A `Receiver` may be *moved* to another thread, but two threads may not use it
through `&`. Fix: keep one receiving thread. If several workers must take jobs
from one queue, wrap the receiver in `Arc<Mutex<...>>` (optional lab 2).

**`send(...).unwrap()` when the receiver may stop early.**

```text
thread 'main' (123001234) panicked at src/main.rs:6:16:
called `Result::unwrap()` on an `Err` value: SendError { .. }
```

A failed `send` is the normal way for a producer to learn that nobody is
listening any more. Handle it: stop producing and return.

### When to use which

- `channel()`: producers are slow or few, and the consumer keeps up.
- `sync_channel(n)`: producers can be faster than the consumer, and memory
  must stay bounded. Start with a small `n`.
- A channel moves *ownership* of each value to one consumer. When several
  threads must look at or change the *same* value, use a lock (lesson 05).

### Read more

- [`std::sync::mpsc`](https://doc.rust-lang.org/std/sync/mpsc/index.html)
- [`Receiver::recv`](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.recv) and [`try_recv`](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.try_recv)
- [`sync_channel`](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html)

## Lesson 04 · Send and Sync contracts

### The idea

Two marker traits carry all of Rust's thread safety. A marker trait has no
methods. It only states a fact about a type.

- `T: Send` means: a value of type `T` may be **moved** to another thread.
- `T: Sync` means: a `&T` may be **shared** with another thread. In other
  words, `T` is `Sync` exactly when `&T` is `Send`.

`thread::spawn`, `Scope::spawn` and the channel types ask for these traits in
their bounds. That is the whole mechanism. The standard library contains no
special rule for threads; the trait bounds do the work.

Both are **auto traits**. You do not implement them. The compiler gives them
to your type when every field has them, and takes them away when one field
does not.

```rust
use std::rc::Rc;
use std::sync::Arc;

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

#[allow(dead_code)]
struct Job {
    id: u32,
    payload: Arc<[u8]>,
}

#[allow(dead_code)]
struct LocalJob {
    id: u32,
    payload: Rc<[u8]>, // one field without Send is enough
}

fn main() {
    assert_send::<Job>();
    assert_sync::<Job>();
    // assert_send::<LocalJob>();   // error[E0277], shown below
}
```

A function such as `assert_send` is a cheap test. It does nothing when it
runs. It only compiles if the type has the trait.

### The common types

| Type | `Send` | `Sync` | Reason |
| --- | --- | --- | --- |
| `u32`, `String`, `Vec<T>` | yes | yes | (for `Vec<T>`: when `T` is) |
| `&T` | when `T: Sync` | when `T: Sync` | a shared reference is a way to share `T` |
| `Rc<T>` | no | no | its counter is a plain number |
| `Arc<T>` | when `T: Send + Sync` | when `T: Send + Sync` | every clone gives another thread a `&T` |
| `Cell<T>`, `RefCell<T>` | when `T: Send` | no | they change data through `&` without any locking |
| `Mutex<T>` | when `T: Send` | when `T: Send` | it lets one thread at a time reach `T` |
| `RwLock<T>` | when `T: Send` | when `T: Send + Sync` | several readers hold `&T` at once |
| `MutexGuard<'_, T>` | no | when `T: Sync` | a lock must be released by the thread that took it |
| `mpsc::Sender<T>` | when `T: Send` | when `T: Send` | |
| `mpsc::Receiver<T>` | when `T: Send` | no | one consumer |
| `AtomicUsize` and the other atomics | yes | yes | |
| raw pointers `*const T`, `*mut T` | no | no | the compiler knows nothing about them |

Two rows deserve a second look.

`Arc<T>` needs `T: Sync` even to be *sent*. Sending one clone of an `Arc` to
another thread leaves the other clone here, so both threads now reach the same
`T` through `&T`. That is sharing.

`Mutex<T>` *adds* `Sync`: a `Mutex<Cell<u8>>` is `Sync` although `Cell<u8>` is
not, because the lock lets only one thread touch the cell at a time. But
`Mutex<T>` cannot add `Send`. Whoever holds the lock can take the value out
(for example with `std::mem::replace`) and so move it to their own thread.

### Mistakes people make

**A type with an `Rc` inside goes to another thread.**

```rust,compile_fail,E0277
use std::rc::Rc;
use std::thread;

fn main() {
    let name = Rc::new(String::from("sensor-4"));
    let copy = Rc::clone(&name);
    thread::spawn(move || println!("{copy}")).join().unwrap();
}
```

```text
error[E0277]: `Rc<String>` cannot be sent between threads safely
   --> src/main.rs:7:19
    |
  7 |     thread::spawn(move || println!("{copy}")).join().unwrap();
    |     ------------- -------^^^^^^^^^^^^^^^^^^^
    |     |             |
    |     |             `Rc<String>` cannot be sent between threads safely
    |     |             within this `{closure@src/main.rs:7:19: 7:26}`
    |     required by a bound introduced by this call
    |
    = help: within `{closure@src/main.rs:7:19: 7:26}`, the trait `Send` is not implemented for `Rc<String>`
```

Read such a message from the bottom of its `help` and `note` lines upwards.
They name the chain: the closure is not `Send` because it contains an
`Rc<String>`. Fix: `Arc`.

**A `RefCell` is shared between threads.**

```rust,compile_fail,E0277
use std::cell::RefCell;
use std::thread;

fn main() {
    let notes = RefCell::new(Vec::<String>::new());
    thread::scope(|s| {
        s.spawn(|| notes.borrow_mut().push("a".into()));
    });
}
```

```text
error[E0277]: `RefCell<Vec<String>>` cannot be shared between threads safely
...
    = help: the trait `Sync` is not implemented for `RefCell<Vec<String>>`
    = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` instead
    = note: required for `&RefCell<Vec<String>>` to implement `Send`
```

The closure captures `&notes`. A reference is `Send` only if its target is
`Sync`, and `RefCell` is not: its borrow counter is a plain number. Fix:
`Mutex` (or `RwLock`), which do the same job with a real lock.

**A lock guard is moved to another thread.** `MutexGuard` is not `Send`. For
the guard of a `Mutex<Vec<u32>>`, `s.spawn(move || drop(guard))` fails with:

```text
error[E0277]: `std::sync::MutexGuard<'_, Vec<u32>>` cannot be sent between threads safely
```

Lock inside the thread that needs the data.

### When to use which

Do not write `unsafe impl Send` or `unsafe impl Sync` to make an error go
away. The error describes a real danger. Replace the field that lacks the
trait: `Rc` with `Arc`, `Cell` with an atomic, `RefCell` with `Mutex`. Writing
these impls for your own pointer types, and proving them correct, is part of
the mastery track ([`mastery/m2_unsafe`](../../mastery/m2_unsafe/README.md)).

### Read more

- [`Send`](https://doc.rust-lang.org/std/marker/trait.Send.html) and [`Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html)
- [Send and Sync in the Rustonomicon](https://doc.rust-lang.org/nomicon/send-and-sync.html)
- [Auto traits in the Rust Reference](https://doc.rust-lang.org/reference/special-types-and-traits.html#auto-traits)

## Lesson 05 · Mutex invariants and poisoning

### The idea

A **mutex** ("mutual exclusion") lets one thread at a time use a value. In
many languages a mutex is a separate object that you must remember to lock.
In Rust the mutex *owns* the value: `Mutex<T>` contains the `T`, and the only
way to reach it is to lock.

- `m.lock()` waits until the lock is free, takes it, and returns a
  `MutexGuard`.
- The guard behaves like `&mut T`. Read and write the value through it.
- When the guard is dropped, the lock is released. There is no `unlock` call.

`lock` takes `&self`, so a `Mutex` that several threads share through `&` or
`Arc` still gives each of them `&mut T`, one after the other. This is interior
mutability (Chapter 9) made safe for threads.

Think about what the lock protects as an **invariant**: a statement about the
value that is true whenever nobody holds the lock. While you hold the guard
you may break the invariant for a moment. You must restore it before you
release the lock.

### Worked example

Four workers move money from one account to another. The invariant is: the
two balances add up to 100.

```rust
use std::sync::Mutex;
use std::thread;

struct Accounts {
    checking: i64,
    savings: i64,
}

fn main() {
    let bank = Mutex::new(Accounts { checking: 100, savings: 0 });

    thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                for _ in 0..25 {
                    let mut accounts = bank.lock().unwrap();
                    accounts.checking -= 1;
                    // Here the sum is 99. No other thread can see that,
                    // because this thread holds the lock.
                    accounts.savings += 1;
                } // the guard is dropped here, once per turn of the loop
            });
        }
    });

    let accounts = bank.into_inner().unwrap(); // no lock needed: we own it again
    assert_eq!((accounts.checking, accounts.savings), (0, 100));
}
```

Two separate `Mutex<i64>` values could not keep this invariant. Between
unlocking the first and locking the second, another thread could read a sum
of 99. Put values that must change together behind *one* lock.

### How long does a guard live?

The lock is held exactly as long as the guard exists, so you must know when
guards are dropped.

- A guard in a `let` lives to the end of the block, or until `drop(guard)`.
- A guard that is only a temporary inside a statement, as in
  `*m.lock().unwrap() += 1;`, is dropped at the end of that statement.
- A temporary guard in the scrutinee of a `match` (the expression after the
  word `match`) lives until the end of the whole `match`, through all its
  arms.
- A temporary guard in the condition of an `if let` lives through the `if`
  block. In edition 2024, which this course uses, it is dropped before the
  `else` block begins. In older editions it lived through the `else` block
  too, and locking again there was a deadlock.

Keep the guarded region short. Do slow work (computing, input and output,
waiting for another thread) before you lock or after you unlock.

### Poisoning

If a thread panics while it holds a guard, the value may be half-changed. The
mutex remembers this: it is **poisoned**. From then on `lock()` returns `Err`.
The data is not lost and not rolled back. The error contains the guard, so
you can still reach the value:

```rust
use std::sync::Mutex;

let stock = Mutex::new(vec![3_u32]);
let result = std::panic::catch_unwind(|| {
    let mut guard = stock.lock().unwrap();
    guard.push(4);
    panic!("scanner failed"); // the guard is dropped while panicking
});
assert!(result.is_err());
assert!(stock.is_poisoned());

// Decide deliberately: look at the value and repair it if needed.
let guard = stock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
assert_eq!(*guard, [3, 4]);
```

`lock().unwrap()` is the common choice and means: "if another thread panicked
while it changed this, I stop as well". Recover with `into_inner` only when
you can check or rebuild the invariant. `Mutex::clear_poison` removes the
mark once you have done so.

### Mistakes people make

**The guard binding is not `mut`.**

```rust,compile_fail,E0596
use std::sync::Mutex;

fn main() {
    let queue = Mutex::new(vec![1_u32]);
    let guard = queue.lock().unwrap();
    guard.push(2);
}
```

```text
error[E0596]: cannot borrow `guard` as mutable, as it is not declared as mutable
 --> src/main.rs:6:5
  |
6 |     guard.push(2);
  |     ^^^^^ cannot borrow as mutable
  |
help: consider changing this to be mutable
  |
5 |     let mut guard = queue.lock().unwrap();
  |         +++
```

The `Mutex` itself needs no `mut`. The guard does when you call a `&mut self`
method through it.

**Unwrapping a poisoned lock without meaning to.**

```text
thread 'main' (122913772) panicked at src/main.rs:13:30:
called `Result::unwrap()` on an `Err` value: PoisonError { .. }
```

This panic is a second failure caused by an earlier one. Look for the first
panic, in another thread, higher up in the output.

**Deadlock.** The compiler does not prevent it. Two patterns cause most
deadlocks:

```text
thread 1: lock A ... wants B        one thread: let g1 = m.lock();
thread 2: lock B ... wants A                    let g2 = m.lock();   // same mutex: never returns, or panics
```

For the first, make every thread take the locks in the same order. For the
second, remember that a `std` mutex cannot be locked twice by one thread; drop
the first guard, or pass the guard to the function that needs the data.

### When to use which

- `Mutex<T>`: the default for shared data that changes.
- `RwLock<T>` (lesson 06): many readers that hold the lock for a long time,
  few writers.
- An atomic (lesson 07): the shared state is one number or one flag.
- A channel (lesson 03): the data should move to one owner instead of being
  shared.

### Read more

- [`Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html), including the section on poisoning
- [`PoisonError`](https://doc.rust-lang.org/std/sync/struct.PoisonError.html)
- [Temporary scopes in the Rust Reference](https://doc.rust-lang.org/reference/destructors.html#temporary-scopes)
- [`if let` temporary scope in the Edition Guide](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-if-let-scope.html)

## Lesson 06 · RwLock and condition predicates

### The idea: RwLock

An `RwLock<T>` has two kinds of guard. `read()` gives a guard that works like
`&T`; any number of threads may hold read guards at the same time. `write()`
gives a guard that works like `&mut T`; it waits until no other guard exists,
and while it exists nobody else gets in. This is the borrowing rule of Chapter
5 (many readers or one writer), checked while the program runs.

```rust
use std::sync::RwLock;

let limits = RwLock::new(vec![10_u32, 20]);
{
    let first = limits.read().unwrap();
    let second = limits.read().unwrap(); // two readers at once: fine
    assert_eq!(first.len() + second.len(), 4);
} // both read guards are dropped here
limits.write().unwrap().push(30); // the writer needs the lock for itself
assert_eq!(limits.read().unwrap().len(), 3);
```

### The idea: Condvar

Sometimes a thread must wait until something becomes true: a job has arrived,
all workers are done, a buffer has room. Checking again and again in a loop
would burn a whole processor core. A **condition variable**, `Condvar`, lets
the thread sleep until another thread says "something changed, look again".

A `Condvar` holds no data. It always works together with a `Mutex` that
protects the real state, and with a **predicate**: a question about that
state, such as "is the counter zero?".

```text
waiting thread                               changing thread
lock the mutex
while the predicate is false:
    guard = condvar.wait(guard)              lock the mutex
      (unlocks, sleeps ............)         change the state
      (wakes, locks again)          ◄─────── condvar.notify_one() or notify_all()
use the state, with the lock held
```

`wait` takes the guard, releases the lock, and puts the thread to sleep. It
does both in one step, so that no change can slip in between. When the thread
wakes up, `wait` takes the lock again and gives the guard back.

### Worked example

The main thread waits until three workers have finished. The state is the
number of workers that are still running.

```rust
use std::sync::{Condvar, Mutex};
use std::thread;

fn main() {
    let running = Mutex::new(3_u32);
    let all_done = Condvar::new();

    thread::scope(|s| {
        for _ in 0..3 {
            s.spawn(|| {
                // ... the real work would happen here, without the lock ...
                let mut left = running.lock().unwrap();
                *left -= 1; // change the state under the lock
                if *left == 0 {
                    all_done.notify_all(); // then tell the waiter to look again
                }
            });
        }

        let mut left = running.lock().unwrap();
        while *left > 0 {
            left = all_done.wait(left).unwrap();
        }
        assert_eq!(*left, 0);
    });
}
```

This works in every timing. If all workers finish before the main thread
reaches its loop, the counter is already 0 and the main thread never waits. If
the main thread arrives first, it sleeps, and the last worker wakes it. The
notification is never the information. The counter is.

`Condvar::wait_while(guard, |state| ...)` contains the same loop: it waits as
long as the closure returns `true`.

### Mistakes people make

**Waiting first and looking afterwards.** A `notify` that happens when nobody
is waiting is lost. A thread that calls `wait` before it has checked the state
may sleep forever, although the state it waits for is already there. Always
check the predicate before the first `wait`.

**`if` instead of `while`.** `wait` may return although nobody called
`notify` (a *spurious wakeup*; the operating system allows it). And with
several waiters, another thread may have used up the state before this one
got the lock back. After `wait` returns you know only that it is time to
check again. So the check is a loop.

**Changing the state without the lock, or notifying without changing it.** The
waiter decides by the state under the mutex. A notification that is not
paired with a change of that state, made while holding the lock, wakes the
waiter to nothing, or misses it.

**Trying to write through a read guard.**

```rust,compile_fail,E0596
use std::sync::RwLock;

fn main() {
    let limits = RwLock::new(vec![10_u32, 20]);
    let reader = limits.read().unwrap();
    reader.push(30);
}
```

```text
error[E0596]: cannot borrow data in dereference of `std::sync::RwLockReadGuard<'_, Vec<u32>>` as mutable
 --> src/main.rs:6:5
  |
6 |     reader.push(30);
  |     ^^^^^^ cannot borrow as mutable
  |
  = help: trait `DerefMut` is required to modify through a dereference, but it is not implemented for `std::sync::RwLockReadGuard<'_, Vec<u32>>`
```

A read guard cannot be turned into a write guard. Calling `write()` while the
same thread still holds a read guard never succeeds: the thread waits for its
own read guard forever, or the call panics. Drop the read guard, take the
write lock, and check the state again, because another writer may have been
faster.

### When to use which

- `Mutex` or `RwLock`? An `RwLock` helps only when readers hold the lock for a
  long time and writers are rare. For short critical sections a `Mutex` is
  simpler and often faster. Whether waiting writers or new readers go first
  depends on the operating system, so do not rely on either.
- `Condvar`: wait for a condition on state that you already protect with a
  mutex.
- A channel: "wait until a value arrives" is exactly `recv`. Prefer it when
  the thing you wait for is a message.
- `Barrier`: a fixed number of threads must all reach one point before any of
  them continues. It has no predicate, so it cannot express "wait until the
  queue is not empty".

### Read more

- [`RwLock`](https://doc.rust-lang.org/std/sync/struct.RwLock.html)
- [`Condvar`](https://doc.rust-lang.org/std/sync/struct.Condvar.html) and [`wait_while`](https://doc.rust-lang.org/std/sync/struct.Condvar.html#method.wait_while)
- [`Barrier`](https://doc.rust-lang.org/std/sync/struct.Barrier.html)

## Lesson 07 · Atomic counters and local state

### The idea: atomics

An **atomic** type holds one small value (an integer, a `bool`, a pointer)
that several threads may read and write through `&`, without a lock. The
types live in `std::sync::atomic`: `AtomicUsize`, `AtomicU64`, `AtomicBool`
and so on. Each operation on an atomic is one indivisible step. No other
thread can see it half done, and no other operation can happen in the middle
of it.

That matters for operations that read *and* write. Adding one to a counter
means "read the value, add, write the result". With a separate `load` and
`store`, two threads can interleave like this:

```text
value = 7
thread A: load  -> 7
thread B: load  -> 7
thread A: store 8
thread B: store 8          one increment is lost
```

`fetch_add` does the read, the addition and the write as one step, so no
update is lost. The same family contains `fetch_sub`, `fetch_max`, `swap`,
and more. For an update that follows your own rule there are `update` and
`try_update`, which take a closure. (`try_update` was called `fetch_update` in
older versions of Rust; the old name still compiles, with a deprecation
warning.) Lesson 09 shows what they do inside.

Every operation takes an `Ordering` argument. Until lesson 09, use
`Ordering::Relaxed`. It is correct whenever the atomic stands for itself, like
a counter, and other memory does not depend on it.

### Worked example

Two workers count even and odd numbers into shared counters.

```rust
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

fn main() {
    let even = AtomicU32::new(0);
    let odd = AtomicU32::new(0);

    thread::scope(|s| {
        for chunk in [[1_u32, 2, 3, 4], [5, 6, 7, 9]] {
            let (even, odd) = (&even, &odd);
            s.spawn(move || {
                for n in chunk {
                    let counter = if n % 2 == 0 { even } else { odd };
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    });

    // `into_inner` needs no ordering: we own the atomics again.
    assert_eq!((even.into_inner(), odd.into_inner()), (3, 5));
}
```

The two counters are each correct. But they are two separate atomics: a third
thread that reads both while the workers run can see an `even` and an `odd`
that never existed together. When two values must agree with each other, use
one `Mutex` around both.

Atomic integers have no overflow check, in any build profile. Find out in the
predictions what `fetch_add` does at the upper limit, and use `try_update`
with a checked operation when the limit matters.

### The idea: thread-local values

Sometimes the opposite of sharing is wanted: every thread should have its own
copy. `thread_local!` declares such a variable.

```rust
use std::cell::RefCell;
use std::thread;

thread_local! {
    static TRAIL: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn main() {
    TRAIL.with_borrow_mut(|trail| trail.push("main was here"));

    // The worker gets a fresh copy, built from the initializer above.
    let seen_by_worker = thread::spawn(|| TRAIL.with_borrow(|trail| trail.len()))
        .join()
        .unwrap();

    assert_eq!(seen_by_worker, 0);
    assert_eq!(TRAIL.with_borrow(|trail| trail.len()), 1);
}
```

The name `TRAIL` has the type `LocalKey<RefCell<Vec<&str>>>`. It is a key,
not the value: each thread that uses the key reaches its own copy, which is
created when that thread uses it for the first time. No other thread can
reach a copy, so a thread-local needs no lock. `Cell` and `RefCell` are
enough. For a `LocalKey<Cell<T>>` there are the short forms `get`, `set`,
`take` and `replace`.

### Mistakes people make

**A mutable global.**

```rust,compile_fail,E0133
static mut HITS: u32 = 0;

fn main() {
    HITS += 1;
}
```

```text
error[E0133]: use of mutable static is unsafe and requires unsafe block
 --> src/main.rs:4:5
  |
4 |     HITS += 1;
  |     ^^^^ use of mutable static
  |
  = note: mutable statics can be mutated by multiple threads: aliasing violations or data races will cause undefined behavior
```

Every thread can reach a `static`. A `static mut` has no protection at all,
so each use is `unsafe`, and it is correct only if you can prove that no two
threads ever touch it at the same time. Do not add `unsafe`. Use a `static`
with a type that is safe to share: `static HITS: AtomicU32 =
AtomicU32::new(0);`, or `static LOG: Mutex<Vec<String>> =
Mutex::new(Vec::new());`. Both constructors can run at compile time.

**A separate `load` and `store`.** It compiles, and it loses updates, as the
trace above shows. Nothing warns you. Use one `fetch_*` operation, or
`update`.

**Expecting a thread-local to carry a value to another thread.** A worker
never sees what the main thread stored. Pass the value into the closure
instead.

### When to use which

- Atomic: one number or flag, each operation makes sense alone.
- `Mutex`: several fields that must change together, or a value that is not a
  small number.
- `thread_local!`: per-thread scratch space or per-thread statistics that
  each worker reports at the end.

### Read more

- [`std::sync::atomic`](https://doc.rust-lang.org/std/sync/atomic/index.html)
- [`AtomicUsize`](https://doc.rust-lang.org/std/sync/atomic/type.AtomicUsize.html), for `fetch_add` and the other operations
- [`thread_local!`](https://doc.rust-lang.org/std/macro.thread_local.html) and [`LocalKey`](https://doc.rust-lang.org/std/thread/struct.LocalKey.html)
- [`static mut` references in the Edition Guide](https://doc.rust-lang.org/edition-guide/rust-2024/static-mut-references.html)

## Lesson 09 · Retry loops, orderings, and one-time initialization

Do this lesson after lesson 07 and before the capstone. It has three parts
that build on atomics.

### Compare and exchange

`fetch_add` can only add. The general tool is **compare-and-exchange**, also
called compare-and-swap or CAS:

```text
atomic.compare_exchange(expected, new, success_ordering, failure_ordering)
```

In one indivisible step it checks whether the atomic holds `expected`. If
yes, it stores `new` and returns `Ok(previous value)`. If no, it changes
nothing and returns `Err(value that it found)`. So it means: "write this, but
only if the value is still the one that I saw".

A single compare-and-exchange is enough for a change of state that must
happen once:

```rust
use std::sync::atomic::{AtomicU8, Ordering};

const NEW: u8 = 0;
const RUNNING: u8 = 1;

let state = AtomicU8::new(NEW);

// Only the caller that finds NEW may start the job.
let first = state.compare_exchange(NEW, RUNNING, Ordering::Relaxed, Ordering::Relaxed);
let second = state.compare_exchange(NEW, RUNNING, Ordering::Relaxed, Ordering::Relaxed);

assert!(first.is_ok());
assert_eq!(second, Err(RUNNING)); // too late, and the error says what is there now
```

However many threads try at the same moment, exactly one of them gets `Ok`.

For an update that depends on the old value, compare-and-exchange goes into a
**retry loop**. This trace shows two threads that both want to double a
value:

```text
value = 5
thread A: load -> 5, computes 10
thread B: load -> 5, computes 10
thread A: compare_exchange(5, 10) -> Ok(5)         value = 10
thread B: compare_exchange(5, 10) -> Err(10)       nothing written; B now knows the value is 10
thread B: computes 20 from 10
thread B: compare_exchange(10, 20) -> Ok(10)       value = 20, both updates counted
```

The rule of the loop: after a failure, do not load again and do not reuse your
old value. The error already contains the fresh value. Compute again from it
and try again. `update` and `try_update` are this loop, written for you.

`compare_exchange_weak` is a variant that may fail even when the value is the
expected one. On some processors that makes it cheaper. Inside a retry loop
such a failure costs only one more turn, so use the weak form there. Use the
strong form, `compare_exchange`, when you try only once.

### Memory ordering

So far the question was "is this one operation indivisible?". The `Ordering`
argument answers a different question: "what does this operation promise
about *other* memory?".

Processors and compilers may reorder reads and writes of different variables,
as long as a single thread cannot notice. A second thread can notice. The
orderings say how much reordering you allow.

| Ordering | Promise | Typical use |
| --- | --- | --- |
| `Relaxed` | Only that this operation is atomic. Nothing about other memory. | Counters, statistics, IDs. A stop flag that guards nothing else. |
| `Release` (for a store) | Everything this thread wrote before the store is visible to a thread that reads the stored value with `Acquire`. | The last step of publishing. |
| `Acquire` (for a load) | The other half of that pair. | The first step of using what was published. |
| `AcqRel` | Both, for an operation that reads and writes. | Lock-like operations. |
| `SeqCst` | What `Acquire` and `Release` promise, and in addition all `SeqCst` operations in the program form one single order that every thread agrees on. | Rare. When in doubt during a first draft. |

The pair `Release` and `Acquire` is the one to understand. Suppose a writer
fills three atomics and then announces a new version:

```text
writer                                    reader
width.store(640, Relaxed)
height.store(480, Relaxed)
depth.store(24, Relaxed)
version.store(2, Release)   ───────────►  version.load(Acquire) == 2
                                          width.load(Relaxed)   == 640   guaranteed
                                          height.load(Relaxed)  == 480   guaranteed
```

The arrow is called a *happens-before* edge. Without it (with `Relaxed` on
`version`) the reader may see version 2 and still read an old width. That is
allowed by the rules, and it does happen on real processors. No test can
reliably show it, which is why you must reason about it.

Many things already give you such an edge, and then `Relaxed` is enough:
spawning a thread, joining a thread, sending on a channel, and locking a
mutex. A counter that you read after `join` needs no stronger ordering.

Here a stop flag guards nothing but itself, so `Relaxed` is correct:

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;

let stop = AtomicBool::new(false);
let turns = AtomicU64::new(0);

thread::scope(|s| {
    s.spawn(|| {
        while !stop.load(Ordering::Relaxed) {
            turns.fetch_add(1, Ordering::Relaxed);
            thread::yield_now();
        }
    });
    stop.store(true, Ordering::Relaxed);
});

// The scope joined the worker. That join orders this load after its writes.
let _total = turns.load(Ordering::Relaxed);
```

### Initializing once

A `static` must be built at compile time. A value that needs a run-time
computation cannot be written directly:

```rust,compile_fail,E0015
static GREETING: String = String::from("hello");

fn main() {
    println!("{GREETING}");
}
```

```text
error[E0015]: cannot call non-const associated function `<String as From<&str>>::from` in statics
 --> src/main.rs:1:27
  |
1 | static GREETING: String = String::from("hello");
  |                           ^^^^^^^^^^^^^^^^^^^^^
  |
  = note: calls in statics are limited to constant functions, tuple structs and tuple variants
  = note: consider wrapping this expression in `std::sync::LazyLock::new(|| ...)`
```

Two types solve this, and both are safe to share between threads.

```rust
use std::sync::{LazyLock, OnceLock};

// LazyLock: the value and the way to build it are declared together.
static GREETING: LazyLock<String> = LazyLock::new(|| format!("hello from {}", "LazyLock"));

// OnceLock: an empty cell. The code that uses it decides how to fill it.
fn retry_limit() -> u32 {
    static LIMIT: OnceLock<u32> = OnceLock::new();
    *LIMIT.get_or_init(|| "5".parse().expect("a number"))
}

fn main() {
    assert_eq!(GREETING.len(), 19); // the closure runs here, at the first use
    assert_eq!(retry_limit(), 5);
    assert_eq!(retry_limit(), 5); // already filled: the closure does not run again
}
```

A `OnceLock<T>` is written at most once and can then be read forever through
`&T`. `set` fills it if it is empty. `get` returns an `Option`. `get_or_init`
fills it if needed and returns the value. A `LazyLock` is the same idea with
the closure stored inside. If several threads arrive at an empty cell
together, one of them runs the closure and the others wait for it. The
predictions let you check the details.

### Mistakes people make

**An ordering that makes no sense for the operation.**

```rust,compile_fail
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    let ready = AtomicBool::new(false);
    println!("{}", ready.load(Ordering::Release));
}
```

```text
error: atomic loads cannot have `Release` or `AcqRel` ordering
 --> src/main.rs:5:31
  |
5 |     println!("{}", ready.load(Ordering::Release));
  |                               ^^^^^^^^^^^^^^^^^
  |
  = help: consider using ordering modes `Acquire`, `SeqCst` or `Relaxed`
  = note: `#[deny(invalid_atomic_ordering)]` on by default
```

A load releases nothing, and a store acquires nothing. The compiler rejects
the obvious cases.

**Reusing the stale value after a failed exchange.** The loop then tries the
same exchange forever, because the value it expects is gone. Continue with the
value from the `Err`.

**`Relaxed` on a flag that announces other data.** It compiles, passes every
test on most machines, and is wrong. Ask for each flag: "does a reader use
other memory because of what it read here?". If yes, the store is `Release`
and the load is `Acquire`.

### When to use which

- One-time change of state: one strong `compare_exchange`.
- Update from the old value: `update` / `try_update`, or your own loop with
  `compare_exchange_weak`.
- `OnceLock` when the code that first needs the value knows how to build it,
  or when it is set from outside (for example from `main`). `LazyLock` when
  the initializer is fixed at the declaration.
- Building locks, channels and reference counts from these parts, the *ABA
  problem* of retry loops over pointers, and testing with the model checker
  `loom` belong to the mastery track ([`mastery/m3_concurrency`](../../mastery/m3_concurrency/README.md)).

### Read more

- [`AtomicU64`](https://doc.rust-lang.org/std/sync/atomic/type.AtomicU64.html), for `compare_exchange`, `compare_exchange_weak`, `update` and `try_update`
- [`Ordering`](https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html)
- [Atomics in the Rustonomicon](https://doc.rust-lang.org/nomicon/atomics.html)
- [`OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html) and [`LazyLock`](https://doc.rust-lang.org/std/sync/struct.LazyLock.html)

## Lesson 08 · Capstone: bounded parallel pipeline

The capstone combines lessons 02, 03 and 05 into one shape that appears in
much real code: several producers, one bounded channel, one collector.

```text
input slice ──► chunk 0 ── producer 0 ──┐
                chunk 1 ── producer 1 ──┼──► sync_channel(2) ──► collector (the calling thread)
                chunk 2 ── producer 2 ──┘                         puts the results in input order
```

This section explains the approach. It does not give the code.

**Plan the ownership before you write.** For each of these questions, write
the answer in NOTES.md first.

- *Who borrows the input?* The producers only read it, and they finish before
  the function returns. That is the situation of lesson 02.
- *Who owns a sender?* Make a list of every `Sender` that will exist: one per
  producer, and the original. The collector's loop ends when the last one on
  that list is dropped. Decide at which line each one is dropped.
- *Which thread receives, and when?* The channel holds only two values. A
  producer with a third value waits until somebody receives. If the
  collecting starts only after the producers have been joined, the producers
  wait for the collector and the collector waits for the producers. That is a
  deadlock, and the tests report it as a timeout. The receiving must happen
  while the producers run.
- *How does the order come back?* Results arrive in the order in which the
  producers happen to run. Nothing about that order is promised. So each
  result must carry something that says where it belongs, and the collector
  uses that. Sorting by a position that the data carries is part of the
  contract. Sorting results only because a test compared them in order would
  hide the fact that the order was never defined.

**Task B is the other half of a real pipeline: failure.** A producer may send
`Err`. The collector should stop at the first error and return it. What
happens to the producers that are still sending? When the collector returns,
the receiver is dropped, and from then on every `send` fails (lesson 03).
That failed `send` is the cancellation signal. A producer that calls
`unwrap()` on `send` turns a clean stop into a panic; a producer that returns
on a failed `send` stops quietly.

**Check your design against the tests' questions.** Does every element pass
through `work` exactly once? Does each chunk have its own producer thread? Do
all producers run at the same time? What happens for an empty input, for a
chunk size larger than the input, and for a chunk size of zero?

### Read more

- [`sync_channel`](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html)
- [`slice::chunks`](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks)
- [`Receiver`](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html), for the ways to iterate over a channel
