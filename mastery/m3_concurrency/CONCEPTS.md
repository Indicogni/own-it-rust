# Mastery module M3: concepts

[Module guide](README.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Read one section, then do the lesson with the same number.

## What you will be able to do

After this module you can build the tools that concurrent Rust programs are made of: a lock, a one-shot channel, a reference-counted pointer, a mutex that puts threads to sleep, a bounded channel, a lock-free stack and a wait-free queue. For each of them you can write down the invariant that makes it correct, choose the memory ordering of every atomic operation and say why a weaker one would be wrong, and write the safety argument for every `unsafe` block and every `unsafe impl Send` or `Sync`. You can also check such code with two tools: **loom**, which runs a small test under every possible interleaving, and **Miri**, which detects undefined behaviour in one run.

This is advanced material. It is slow to read, and that is normal. A section is finished when you can repeat its argument without looking.

## Before you start

- Chapter 19 (threads, `Mutex`, `Condvar`, channels, `Arc`, first use of atomics and `compare_exchange` loops).
- Chapter 23 (`unsafe`, raw pointers, `UnsafeCell`, SAFETY comments).
- Chapter 13 (`Drop`, `Deref`, `Send` and `Sync` as marker traits) and Chapter 9 (`Rc` and `Weak`).
- From mastery module M2, if you did it: `MaybeUninit`, `NonNull`, `PhantomData` and the drop check. Each is explained again here where it is first used.

## Time

| Lesson | Reading | Exercise |
| --- | --- | --- |
| 01 · Memory orderings and happens-before | 60 min | 2–3 h |
| 02 · A spin lock with a guard | 40 min | 2 h |
| 03 · A one-shot channel | 40 min | 2–3 h |
| 04 · `MyArc` and `MyWeak` | 45 min | 3–4 h |
| 05 · A mutex that puts waiting threads to sleep | 45 min | 3–4 h |
| 06 · A bounded channel with `Mutex` and `Condvar` | 30 min | 2–3 h |
| 07 · A lock-free stack, and the ABA problem | 60 min | 3–4 h |
| 08 · Capstone: a wait-free ring buffer | 20 min | 4–6 h |
| C01 · Code review and the drills | — | 3 h |

These are estimates for a learner who meets the assumptions above. The whole module is about 30 to 40 hours.

## How the model tests work

An ordinary test of concurrent code runs one interleaving: the one that the operating system happened to choose. A bug that needs a thread switch at one exact instruction may not show up in a million runs. Each lesson therefore has a second test module, `model`, which runs under [loom](https://docs.rs/loom/0.7.3/loom/).

Loom replaces the threads, atomics and cells with its own versions. It runs the body of a model test many times. Each time it makes a different choice wherever the result could depend on a choice: which thread runs next, and which of several allowed values a load returns. It stops when it has tried every combination, or when one run fails. A model test looks like this:

```rust,ignore
use m3_concurrency::sync::atomic::{AtomicUsize, Ordering};
use m3_concurrency::sync::{Arc, thread};

loom::model(|| {
    let hits = Arc::new(AtomicUsize::new(0));
    let worker = {
        let hits = Arc::clone(&hits);
        thread::spawn(move || {
            hits.fetch_add(1, Ordering::Relaxed);
        })
    };
    hits.fetch_add(1, Ordering::Relaxed);
    worker.join().unwrap();
    assert_eq!(hits.load(Ordering::Relaxed), 2);
});
```

(This block is not run as a documentation test, because it needs loom. The same code is a real test in `src/sync.rs`.)

For this to work, your code must use loom's types when it is compiled for a model. That is the job of `src/sync.rs`: with `--cfg loom` it gives you loom's types, and otherwise the standard library's. **Always import atomics, cells, threads, `Arc`, `Mutex` and `Condvar` from `crate::sync`.** Code that uses `std::sync::atomic` directly still compiles, but loom cannot see it, and the model then proves nothing.

One type in `crate::sync` has a different interface from its standard-library twin: the cell. The worked examples in this text use `std::cell::UnsafeCell` and its `get()`. `crate::sync::UnsafeCell` has no `get`. It has `with(|pointer| ...)` for a read and `with_mut(|pointer| ...)` for a write, and the closure receives the raw pointer. Each call tells loom where one access begins and ends, which is how loom notices two unordered accesses to the same cell. The top of `src/sync.rs` has a short example.

Loom reports three kinds of failure, besides a failed assertion:

- **`Causality violation: Concurrent read and write accesses`**. Two threads used the same `UnsafeCell` (or the same tracked test value), and nothing ordered the two accesses. This is how an ordering that is too weak shows up.
- **`deadlock; threads = [...]`**. Every thread is blocked. In this module that means a lost wake-up.
- **`Model exceeded maximum number of branches`**. A thread keeps going around a loop without any other thread being able to help it. Call `hint::spin_loop()` in spin loops; loom understands it as "let somebody else run".

Read the *first* panic message. After a failed run the helper `support::model` stops the model and adds a summary line. In some cases loom 0.7.3 cannot stop cleanly: after some deadlocks, or when the failed access leaves a cell marked as "in use" and a destructor touches it afterwards. The test program then ends with `panic in a destructor during cleanup` and a long backtrace, and the other model tests of that run are not reported. The first message is still the one that matters. Fix it and run again.

A model is only as good as its tool. Loom 0.7.3 has limits that you should know, because you will meet them:

- It does not check `SeqCst` loads and stores precisely. It treats them almost like `AcqRel`, so correct code that depends on `SeqCst` can fail a model. `fence(SeqCst)` is modelled correctly. Lesson 01 comes back to this.
- It cannot produce every strange result that `Relaxed` allows on real hardware. A model that passes is strong evidence. It is not a proof.
- A thread can lose the processor only just before an atomic, lock or thread operation. A write to a cell right after an atomic store is never separated from it.
- Its `unpark` wakes a thread out of any blocking call, not only out of `park`. The models are written so that this cannot happen.
- With three threads the number of runs grows very fast, and two threads that spin on the same lock can keep each other busy without end. Such models are limited to a small number of forced thread switches.
- It does not notice a use of freed memory. Miri does. Run the solved lessons under both.

Miri is an interpreter for Rust programs that checks every memory access against the language rules. It runs the ordinary tests (`cargo +nightly-2026-11-20 miri test`), one interleaving at a time, with a random scheduler. It reports data races, uses of freed or uninitialised memory, and leaks. Its weak compare-exchange fails on purpose now and then, which catches code that forgot the retry loop.

## Lesson 01 · Memory orderings and happens-before

### The model

A **data race** is two accesses to the same memory location from two threads, at least one of them a write, with nothing that orders them. In Rust a data race is undefined behaviour: the compiler may assume that it never happens. Safe Rust cannot produce one. Code with `unsafe` can.

"Ordered" has a precise meaning. Operation A **happens before** operation B in several cases. The main cases are:

1. A and B are in the same thread, and A comes first in the program.
2. A is the call that spawns a thread, and B is anything in that thread. Or A is anything in a thread, and B comes after `join` returned for that thread.
3. A is a `Release` store to an atomic, and B is an `Acquire` load that reads the value which A stored.
4. A is the unlock of a mutex and B is a later lock of the same mutex. A is `unpark` and B is the return of the `park` that used its token. (A channel, a `Condvar` and `Arc`'s counter are built from such pairs.)
5. A happens before C, and C happens before B.

Two more rules complete the picture. You need them later in this module, so they are only named here:

- **Fences.** `std::sync::atomic::fence` gives the `Release` or `Acquire` side to atomic operations that are themselves `Relaxed`. A `fence(Ordering::Release)` followed by a store in one thread pairs with a load that reads that store followed by a `fence(Ordering::Acquire)` in another thread, like rule 3. Lesson 04 uses an `Acquire` fence in the `drop` of `MyArc`.
- **Release sequences.** An `Acquire` load that reads the value written by a later read-modify-write operation (such as `fetch_add` or `compare_exchange`) on the same atomic can still be ordered after an earlier `Release` store. Lesson 07 explains the rule and uses it for the lock-free stack.

If neither A happens before B nor B before A, the two are **concurrent**. Concurrent does not mean "at the same moment on the clock". It means that your program has no right to assume either order. The compiler and the processor both use that freedom: stores wait in a buffer, loads are answered from a cache, instructions are reordered.

Operations on atomics never race with each other. That is what "atomic" buys you. The `Ordering` argument answers a different question: **which other memory accesses does this atomic operation put in order?**

| Ordering | Use on | What it adds |
| --- | --- | --- |
| `Relaxed` | any | Nothing. The operation itself is atomic, and that is all. |
| `Release` | stores | Everything this thread did before the store happens before any `Acquire` load that reads the stored value. |
| `Acquire` | loads | The other half of that pair. |
| `AcqRel` | read-modify-write | `Acquire` for the read and `Release` for the write. |
| `SeqCst` | any | `Acquire`/`Release`, and all `SeqCst` operations in the program form one single order that every thread agrees on. |

Two more facts that people often get wrong:

- A `Relaxed` (or `Acquire`) load may return an **older** value than the newest one. It may return any stored value that the rules cannot exclude. What is excluded: a thread never sees the values of one variable go backwards, and it never sees a value older than one whose store happens before the load.
- A read-modify-write operation (`fetch_add`, `swap`, `compare_exchange`) always works on the newest value, with any ordering. That is why a counter is correct with `Relaxed`.

### Worked example: publishing a pointer

One thread builds a value and publishes a pointer to it. Other threads use the value once they see the pointer.

```rust
use std::sync::atomic::{AtomicPtr, Ordering};
use std::{ptr, thread};

struct Config {
    retries: u32,
    name: String,
}

static CURRENT: AtomicPtr<Config> = AtomicPtr::new(ptr::null_mut());

fn publish(config: Config) {
    let pointer = Box::into_raw(Box::new(config)); // (1) plain writes
    CURRENT.store(pointer, Ordering::Release); // (2)
}

fn current() -> Option<&'static Config> {
    let pointer = CURRENT.load(Ordering::Acquire); // (3)
    // SAFETY: a non-null pointer was stored by `publish`, which leaked a Box,
    // so the Config lives for the rest of the program and is never written
    // again. The Release store (2) and this Acquire load (3) order the
    // initialisation (1) before every read through the returned reference.
    unsafe { pointer.as_ref() } // (4) plain reads
}

thread::scope(|s| {
    s.spawn(|| publish(Config { retries: 3, name: "primary".into() }));
    s.spawn(|| {
        if let Some(config) = current() {
            assert_eq!((config.retries, config.name.as_str()), (3, "primary"));
        }
    });
});
assert_eq!(current().map(|config| config.retries), Some(3));
```

```text
writer thread                         reader thread
(1) write Config on the heap
        |  same thread
(2) store(pointer, Release) ------->  (3) load(Acquire) reads that pointer
                                              |  same thread
                                      (4) read retries, name
```

(1) happens before (2) by rule 1, (2) before (3) by rule 3, (3) before (4) by rule 1. By rule 5, (1) happens before (4): no data race. Now replace `Release` or `Acquire` by `Relaxed`. The arrow in the middle is gone. The reader still gets the right pointer, because the store is atomic. But the write (1) and the read (4) are now concurrent, and that is a data race on the heap block. On a real machine the reader can see the pointer and a `Config` that is not completely written.

The last line needs no ordering argument of its own: `thread::scope` joins both threads (rule 2).

### Mistakes people really make

**A weaker ordering compiles.** No compiler error tells you that an ordering is too weak. This is why this module uses loom. The model test of lesson 01, with a `Relaxed` store in the wrong place, ends like this:

```text
thread 'l01_orderings::model::b_reader_sees_a_complete_value' ([..]) panicked at [..]/loom-0.7.3/src/rt/location.rs:115:9:
Causality violation: Concurrent read and write accesses.
```

(In the reports quoted in this text, `[..]` stands for what differs from run to run and from machine to machine: the number of the thread, and the directory of your Cargo registry.)

**An ordering that makes no sense for the operation.** This one the compiler does catch:

```rust,compile_fail
use std::sync::atomic::{AtomicBool, Ordering};

static READY: AtomicBool = AtomicBool::new(false);

fn main() {
    let _ = READY.load(Ordering::Release);
}
```

```text
error: atomic loads cannot have `Release` or `AcqRel` ordering
 --> e.rs:6:24
  |
6 |     let _ = READY.load(Ordering::Release);
  |                        ^^^^^^^^^^^^^^^^^
  |
  = help: consider using ordering modes `Acquire`, `SeqCst` or `Relaxed`
  = note: `#[deny(invalid_atomic_ordering)]` on by default
```

This is a lint that is an error by default, so it has no error code.

**Check, then act.** `if counter.load(..) < limit { counter.fetch_add(1, ..) }` is two operations. Two threads can both pass the check before either adds. No ordering repairs this; the two steps must become one. Use a `compare_exchange` loop:

```rust
use std::sync::atomic::{AtomicU32, Ordering};

/// Adds 1 unless the counter already holds `limit`. Returns whether it added.
fn add_below(counter: &AtomicU32, limit: u32) -> bool {
    let mut seen = counter.load(Ordering::Relaxed);
    loop {
        if seen >= limit {
            return false;
        }
        // "Store seen + 1, but only if the value is still `seen`."
        match counter.compare_exchange_weak(seen, seen + 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return true,
            Err(now) => seen = now, // somebody else changed it; decide again
        }
    }
}

let counter = AtomicU32::new(1);
assert!(add_below(&counter, 2));
assert!(!add_below(&counter, 2));
assert_eq!(counter.load(Ordering::Relaxed), 2);
```

`compare_exchange_weak` may fail although the value matched. That is harmless inside a loop, and it is cheaper on some processors. Outside a loop, use `compare_exchange`.

**Believing that `Release`/`Acquire` is always enough.** Look at this program, called *store buffering*:

```text
thread 1:  X.store(1);  r1 = Y.load();
thread 2:  Y.store(1);  r2 = X.load();
```

Can both threads read 0? With `Release` stores and `Acquire` loads: yes. Each load reads the *initial* value, which no `Release` store wrote, so rule 3 orders nothing. On a real processor each store can still sit in its core's store buffer while the other core loads. Only `SeqCst` on all four operations forbids `r1 == 0 && r2 == 0`: the four operations are then in one order, the first operation in that order is a store, and the other thread's load comes later in that order and must see it. A `fence(Ordering::SeqCst)` between each store and load gives the same guarantee with `Relaxed` operations. Two threads that each "raise my flag, then look at yours" have exactly this shape. So do a thread that goes to sleep ("announce that I sleep, then check for work") and its waker ("add work, then check for sleepers").

### Which ordering?

- A counter or a flag that guards no other memory: `Relaxed`.
- "I wrote something, now I announce it" and "I saw the announcement, now I read it": `Release` on the store, `Acquire` on the load. This covers most lock and channel code.
- Each of two threads writes one variable and then reads the other, and at least one must see the other's write: `SeqCst`.
- `SeqCst` everywhere "to be safe" is a bad habit. It is not wrong, but it hides which operations your argument really needs. When you change the code later, nobody knows what may be weakened and what may not.

The task `try_enter` in the lesson uses the fence form, because of the loom limit described above.

### Read more

- [`std::sync::atomic::Ordering`](https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html) and the [module documentation](https://doc.rust-lang.org/std/sync/atomic/index.html), which says that Rust uses the C++20 rules for atomics.
- The Rustonomicon on [atomics](https://doc.rust-lang.org/nomicon/atomics.html) and on [data races](https://doc.rust-lang.org/nomicon/races.html).
- The Rust Reference, [behavior considered undefined](https://doc.rust-lang.org/reference/behavior-considered-undefined.html).
- [`compare_exchange`](https://doc.rust-lang.org/std/sync/atomic/type.AtomicUsize.html#method.compare_exchange) and [`fence`](https://doc.rust-lang.org/std/sync/atomic/fn.fence.html).

## Lesson 02 · A spin lock with a guard

### The model

A lock is two things next to each other: a flag that says "taken", and the protected value in an `UnsafeCell`. `UnsafeCell<T>` is the one type through which Rust allows a write behind a shared reference. It gives you a raw pointer, and you promise that no two accesses conflict.

The promise of a lock is an **invariant**, a statement that is true whenever nobody is in the middle of an operation:

> While the flag is `false`, nobody accesses the value. While it is `true`, exactly one guard exists, and only that guard accesses the value.

Every `unsafe` block in a lock is justified by this sentence, and every safe function must leave it true.

The **guard** is how the invariant reaches the type system. `lock` returns a value; having that value is the proof that the flag is `true` and yours. The guard implements `Deref` and `DerefMut`, so it behaves like `&mut T`, and its `Drop` clears the flag. You cannot forget to unlock, and a panic unlocks too, because unwinding drops the guard.

Why does the flag need `Acquire` and `Release`? Thread 1 writes the value and unlocks. Thread 2 locks and reads the value. Those two accesses to the cell must be ordered. The unlock is a `Release` store of `false`. The lock is a compare-exchange that reads this `false` with `Acquire`. That is rule 3 from lesson 01, and it gives: everything the previous owner did happens before everything the next owner does.

A spin lock waits by trying again. Between attempts it calls `std::hint::spin_loop()`, which tells the processor that this is a waiting loop.

### `Sync` for the lock and for the guard

`UnsafeCell<T>` is never `Sync`, so a struct that contains one is not `Sync` either, and you cannot share it between threads. You state the truth with an `unsafe impl`:

- `SpinLock<T>: Sync` needs `T: Send`. The lock hands `&mut T` to one thread after another. That is the same as moving the value from thread to thread. It does **not** need `T: Sync`, because two threads never look at the value together.
- The guard is a different question. `&SpinGuard<T>` gives `&T`. So sharing a guard between threads shares the `T`, and that needs `T: Sync`.

The compiler derives `Send` and `Sync` for your types from their fields. A guard that holds only `&'a SpinLock<T>` gets the rule of that reference: it is `Sync` whenever `SpinLock<T>` is `Sync`, that is for every `T: Send`. For `T = Cell<u32>` that is wrong: `Cell<u32>` is `Send` but not `Sync`. The standard library had exactly this hole in `MutexGuard` for years.

The repair is to tell the compiler what the guard really is. `PhantomData<X>` is a field of size zero that makes the compiler treat your type as if it contained an `X`.

### Worked example: a cell that one caller at a time may use

This type has no guard and never waits. `try_with` runs a closure on the value if nobody else is using it.

```rust
use std::cell::{Cell, UnsafeCell};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

pub struct Exclusive<T> {
    busy: AtomicBool,
    value: UnsafeCell<T>,
}

// SAFETY: `try_with` lets only one caller at a time reach the value (see the
// invariant in `try_with`), so sharing an `Exclusive<T>` moves access to the
// `T` from thread to thread. That needs `T: Send` and nothing else.
unsafe impl<T: Send> Sync for Exclusive<T> {}

impl<T> Exclusive<T> {
    pub const fn new(value: T) -> Self {
        Exclusive { busy: AtomicBool::new(false), value: UnsafeCell::new(value) }
    }

    /// Runs `f` on the value, or returns `None` if another call is running.
    pub fn try_with<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        // Invariant: `busy` is true exactly while one call is between this
        // compare-exchange and the store at the end.
        self.busy
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()?;
        // The flag is cleared by `Drop`, so that a panic in `f` clears it too.
        struct Clear<'a>(&'a AtomicBool);
        impl Drop for Clear<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _clear = Clear(&self.busy);
        // SAFETY: this call set `busy`, so by the invariant no other call is
        // using the value, and the `&mut T` ends before `_clear` is dropped.
        Some(f(unsafe { &mut *self.value.get() }))
    }
}

let total = Exclusive::new(Cell::new(0_u32)); // a Cell is Send, not Sync: fine
thread::scope(|s| {
    for _ in 0..4 {
        s.spawn(|| {
            // Try until it is our turn.
            while total.try_with(|cell| cell.set(cell.get() + 1)).is_none() {
                std::hint::spin_loop();
            }
        });
    }
});
assert_eq!(total.try_with(|cell| cell.get()), Some(4));
```

The lesson asks for more: a `lock` that waits, and a guard with the right `Sync` rule.

### Mistakes people really make

**No `unsafe impl Sync`.** The compiler stops you, with a message that names the cell:

```rust,compile_fail,E0277
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

struct Lock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

impl<T> Lock<T> {
    fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }
}

fn main() {
    let lock = Lock { locked: AtomicBool::new(false), value: UnsafeCell::new(0_u64) };
    thread::scope(|s| {
        s.spawn(|| lock.is_locked());
    });
    assert_eq!(lock.value.into_inner(), 0);
}
```

```text
error[E0277]: `UnsafeCell<u64>` cannot be shared between threads safely
   --> a.rs:19:17
    |
 19 |         s.spawn(|| lock.is_locked());
    |           ----- ^^^^^^^^^^^^^^^^^^^ `UnsafeCell<u64>` cannot be shared between threads safely
    |           |
    |           required by a bound introduced by this call
    |
    = help: within `Lock<u64>`, the trait `Sync` is not implemented for `UnsafeCell<u64>`
note: required because it appears within the type `Lock<u64>`
    = note: required for `&Lock<u64>` to implement `Send`
```

The fix is not "add `unsafe impl<T> Sync`". It is to work out the bound (`T: Send`) and to write the SAFETY comment that proves it.

**A guard that is `Sync` for the wrong types.** Nothing stops you; the hole compiles. You have to ask the question yourself. Here the marker makes the compiler give the right answer:

```rust,compile_fail,E0277
use std::cell::Cell;
use std::marker::PhantomData;

struct Guard<'a, T> {
    flag: &'a std::sync::atomic::AtomicBool,
    _acts_like: PhantomData<&'a mut T>,
}

fn must_be_sync<T: Sync>() {}

fn main() {
    must_be_sync::<Guard<'static, u32>>(); // fine
    must_be_sync::<Guard<'static, Cell<u32>>>(); // error: Cell<u32> is not Sync
}
```

```text
error[E0277]: `Cell<u32>` cannot be shared between threads safely
   --> f.rs:13:20
    |
 13 |     must_be_sync::<Guard<'static, Cell<u32>>>(); // error: Cell<u32> is not Sync
    |                    ^^^^^^^^^^^^^^^^^^^^^^^^^ `Cell<u32>` cannot be shared between threads safely
    |
    = help: within `Guard<'static, Cell<u32>>`, the trait `Sync` is not implemented for `Cell<u32>`
    = note: required because it appears within the type `&'static mut Cell<u32>`
note: required because it appears within the type `PhantomData<&'static mut Cell<u32>>`
```

Remove the `_acts_like` field and the same program compiles, although the guard would hand `&Cell<u32>` to two threads.

**A reference created by accident from a raw pointer.** A method call on `(*pointer)` silently creates a reference to the whole value. Since that is easy to get wrong, the compiler refuses it:

```rust,compile_fail
fn main() {
    let text = String::from("ab");
    let pointer: *const String = &text;
    let length = unsafe { (*pointer).len() };
    assert_eq!(length, 2);
}
```

```text
error: implicit autoref creates a reference to the dereference of a raw pointer
    --> b.rs:4:27
     |
   4 |     let length = unsafe { (*pointer).len() };
     |                           ^^-------^^^^^^^
     |                             |
     |                             this raw pointer has type `*const String`
     |
     = note: creating a reference requires the pointer target to be valid and imposes aliasing requirements
note: autoref is being applied to this expression, resulting in: `&String`
     = note: `#[deny(dangerous_implicit_autorefs)]` on by default
help: try using a raw pointer method instead; or if this reference is intentional, make it explicit
     |
   4 |     let length = unsafe { (&(*pointer)).len() };
     |                           ++          +
```

Write the reference explicitly, `(&*p).len()`, and justify it in the SAFETY comment. Or stay with pointer methods such as `p.read()` and `p.cast::<T>()`.

**`try_lock` with a weak compare-exchange.** It fails now and then on a free lock. On your machine it may never fail; under Miri it fails often. A function without a retry loop needs the strong version.

### When to use a spin lock

Almost never in application code. A spinning thread burns a whole core while the owner of the lock may not even be running. A spin lock is right when the critical section is a handful of instructions and sleeping is impossible or more expensive than the wait: inside an operating-system kernel, inside the slow path of a better lock (lesson 05 uses one that way), or on a system without threads to sleep on. Otherwise use `std::sync::Mutex`.

### Read more

- [`UnsafeCell`](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html) and the Rustonomicon on [`Send` and `Sync`](https://doc.rust-lang.org/nomicon/send-and-sync.html).
- [`PhantomData`](https://doc.rust-lang.org/std/marker/struct.PhantomData.html).
- [`std::hint::spin_loop`](https://doc.rust-lang.org/std/hint/fn.spin_loop.html).
- [`AtomicBool::compare_exchange_weak`](https://doc.rust-lang.org/std/sync/atomic/type.AtomicBool.html#method.compare_exchange_weak).

## Lesson 03 · A one-shot channel

### The model

A one-shot channel carries one value from one thread to another. It needs three things that are new in this module.

**A slot that can be empty.** `MaybeUninit<T>` is memory with the size and alignment of a `T` that may or may not hold a valid `T`. The compiler does not track which. You do: writing is safe, and every read is `unsafe`, because you claim that the slot is initialised. A `MaybeUninit<T>` never drops its content. If a value is inside when the slot goes away, and you do nothing, the value is leaked. If you drop the content twice, or read it when nothing was written, that is undefined behaviour.

**A state machine in an atomic.** Because the compiler does not know whether the slot is full, an atomic integer records it. Write the meaning of each state down before you write code, as an invariant: "the slot is initialised exactly when the state is `READY`". Then every `unsafe` block has something to point to.

**A way to sleep.** `std::thread::park()` blocks the current thread. `thread.unpark()` wakes that thread. The pair has three properties that decide how you must use it:

1. Each thread has one **token**. `unpark` makes the token available. `park` returns immediately if the token is available, and takes it. So "unpark first, park second" does not hang. The token does not count: two `unpark` calls allow one `park` to return early.
2. `park` may return although nobody called `unpark` (a *spurious* wake-up). A token may also be left over from earlier code on the same thread.
3. Because of 2, `park` alone means nothing. It is always used as "sleep until this condition holds": a loop that checks a shared state and parks while the state says "not yet".

From these follows the one correct shape:

```text
waiting thread                      waking thread
loop {                              1. change the state
    if state says "go": break       2. unpark the waiting thread
    park()
}
```

The order on the right matters. If the waker unparks first and changes the state second, the waiter can wake up between the two, see the old state, and park again. Nobody will wake it a second time. This is a **lost wake-up**.

Why does the correct order never lose a wake-up? Look at the last time the waiter checks the state before it parks. Either that check comes after step 1, and then it sees "go". Or it comes before step 1, and then step 2 comes after the check, so the token is available by the time `park` runs, or arrives while it sleeps.

### Worked example: an event that one thread waits for

No value is passed here, only "it happened".

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, Thread};

/// Created by the thread that will wait.
struct Event {
    happened: AtomicBool,
    waiter: Thread,
}

impl Event {
    fn new() -> Arc<Event> {
        Arc::new(Event { happened: AtomicBool::new(false), waiter: thread::current() })
    }

    /// Call from any thread.
    fn signal(&self) {
        self.happened.store(true, Ordering::Release); // 1. state
        self.waiter.unpark(); // 2. wake
    }

    /// Call only from the thread that created the event.
    fn wait(&self) {
        while !self.happened.load(Ordering::Acquire) {
            thread::park();
        }
    }
}

let event = Event::new();
let worker = {
    let event = Arc::clone(&event);
    thread::spawn(move || {
        let result = 6 * 7;
        event.signal();
        result
    })
};
event.wait();
assert_eq!(worker.join().unwrap(), 42);
```

"Call only from the thread that created the event" is a rule in a comment. If another thread calls `wait`, it parks, and `signal` wakes the wrong thread. In the lesson the same rule is enforced by the type system: the receiving half is not `Send`, so it cannot leave the thread that created the channel. A type becomes `!Send` when it contains something that is not `Send`. A raw pointer is not, so the marker `PhantomData<*const ()>` does it.

### Mistakes people really make

**Unpark before the state change.** The model test finds the interleaving and reports that every thread is blocked:

```text
thread 'l03_oneshot::model::c_recv_gets_the_message' ([..]) panicked at [..]/loom-0.7.3/src/rt/execution.rs:215:13:
deadlock; threads = [(Id(0), Blocked(Location(None))), (Id(1), Blocked(Location(None)))]
```

Thread 0 is the test's main thread, blocked in `join`. Thread 1 is the receiver, blocked in `park`.

**`if` instead of a loop around `park`.** It works in almost every run, and fails on the day `park` returns early. You cannot wait for a spurious wake-up in a test, but you can produce an early return on purpose: a thread that calls `thread::current().unpark()` leaves its own token behind, and its next `park` returns at once. The lesson has a test that does this.

**Forgetting the message that nobody received.** The sender wrote it, the receiver never called `recv`, and both halves are dropped. `MaybeUninit` drops nothing, so the shared state needs a `Drop` impl that drops the message if the state says that one is there. The effect in two lines:

```rust
use std::mem::MaybeUninit;

let slot = MaybeUninit::new(String::from("never received"));
drop(slot); // the String's heap buffer is not freed
```

The program runs, and it leaks. Miri reports it when the program ends:

```text
error: memory leaked: alloc306 (Rust heap, size: 14, align: 1), allocated here:
```

**Reading the message twice.** `recv` moves the message out with a pointer read. The bytes are still in the slot. If the state still says "full" when the shared state is dropped, the message is dropped a second time. Change the state when you take the message.

**Reading a slot that was never written** is undefined behaviour even for an integer. Check the state first, with `Acquire`.

### When to use which

For one value from one thread to another, `std::sync::mpsc::channel` works and is simple. Async runtimes have their own one-shot channels, which wake a task instead of a thread. For "compute once, read many times", use `std::sync::OnceLock`. You build this primitive yourself in order to learn the three techniques above; all later lessons use them.

### Read more

- [`MaybeUninit`](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html).
- [`std::thread::park`](https://doc.rust-lang.org/std/thread/fn.park.html), which describes the token and the memory-ordering guarantee of `unpark`.
- The Rustonomicon on [uninitialized memory](https://doc.rust-lang.org/nomicon/uninitialized.html).

## Lesson 04 · `MyArc` and `MyWeak`

### The model

`Arc<T>` puts a value on the heap next to a counter. Every `Arc` is a pointer to that block. Cloning adds one to the counter, dropping subtracts one, and whoever brings the counter to zero drops the value and frees the block.

`Weak<T>` complicates this in a useful way. A `Weak` points to the same block but does not keep the *value* alive. It must still be able to ask "is the value alive?", so it must keep the *block* alive. That gives two counters and two separate events:

| Counter | Counts | When it reaches zero |
| --- | --- | --- |
| strong | the `Arc`s | the value is dropped |
| weak | the `Weak`s, plus one for all the `Arc`s together | the block is freed |

The "plus one" is a trick worth understanding. All strong pointers together hold a single weak reference, and the last strong pointer gives it back after it has dropped the value. Without it, "the last `Arc` and the last `Weak` go away at the same moment on two threads" needs both counters to be examined together, which one atomic operation cannot do. With it, freeing the block has exactly one trigger: the weak counter reaching zero.

```text
        MyArc      MyArc      MyWeak
          |          |          |
          v          v          v
        +---------------------------+
        | strong = 2                |
        | weak   = 1 + 1            |   1 MyWeak, + 1 for "some MyArc exists"
        | value: T                  |
        +---------------------------+
```

### The ordering of each operation

**Increment (`clone`, `downgrade`): `Relaxed`.** The thread already holds a pointer, so the block is alive and stays alive. The increment does not announce anything and does not rely on anything. It only has to be atomic.

**Decrement in `drop`: `Release`, then an `Acquire` fence in the thread that reached zero.** Think of two threads that each hold an `Arc`. Thread 1 reads the value and drops its `Arc`. Thread 2 then drops the last `Arc` and with it the value. Thread 1's read and thread 2's drop of the value are two accesses to the same memory, one of them a write. They must be ordered. Thread 1's decrement is the announcement "I am done with the value" (`Release`). Thread 2 must receive the announcements of all the others before it destroys the value (`Acquire`). Only the thread that reaches zero needs the `Acquire` side, so the standard library does a `Release` decrement and then, only on the zero path, `fence(Ordering::Acquire)`. An `AcqRel` decrement is also correct.

**`upgrade`: a compare-exchange loop.** "If strong is not zero, add one" must be one step. With a plain `fetch_add`, a `Weak` could bring the count from 0 back to 1 after the value has been dropped, and hand out an `Arc` to a dead value. The loop reads the count, gives up at zero, and otherwise tries to replace `n` by `n + 1`.

**Overflow.** A program that leaks handles in a loop (`mem::forget(arc.clone())`) could wrap the counter around to zero, and the block would be freed while pointers to it exist. The standard library aborts the process when a count passes `isize::MAX`. Aborting, not panicking: a panic can be caught, and the count could then still wrap.

### `Send` and `Sync` need both bounds

`Arc<T>` is `Send` only if `T: Send + Sync`, and the same for `Sync`. Each bound has its own reason:

- `T: Sync`, because two `Arc`s on two threads both give out `&T`. With `T: Send` alone, `Arc<Cell<u32>>` could be cloned, one clone sent away, and two threads would call `Cell::set` on one cell.
- `T: Send`, because the last `Arc` drops the value, and the last one may be on any thread. With `T: Sync` alone, a value that must be dropped on the thread that created it (a `MutexGuard`, for example) could be dropped elsewhere.

`&Arc<T>` can be cloned into an `Arc<T>`, so sharing must require everything that sending requires. A `Weak` can be upgraded, so it needs the same bounds.

### Three details of the representation

- **`NonNull<Inner<T>>`** instead of `*mut Inner<T>`. It tells the compiler that the pointer is never null, so `Option<MyArc<T>>` can use the null pattern for `None` and is as small as the pointer.
- **`PhantomData<Inner<T>>`**. A raw pointer says nothing about ownership. The marker records that your type owns an `Inner<T>`, as a `Box` would; the standard library's `Arc` has the same field. (Your type has a `Drop` impl, so the drop checker is already careful about `T`. The marker keeps the declaration honest if that ever changes.)
- **The value sits in an `UnsafeCell`.** When the last `MyArc` drops the value, `MyWeak`s on other threads may hold `&Inner<T>` at that moment (they are reading a counter). A write to memory behind a shared reference is allowed only inside an `UnsafeCell`. Wrap the value in `ManuallyDrop` too: you decide when it is dropped, and freeing the block later must not drop it again.

### Worked example: counting by hand through raw pointers

The standard library lets you take an `Arc` apart into a raw pointer and count by hand. This is how handles are passed through C code. It shows the rule that your own type must keep: every pointer that exists stands for exactly one count.

```rust
use std::sync::Arc;

struct Loud(&'static str);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropping {}", self.0);
    }
}

let original = Arc::new(Loud("the value"));
let observer = Arc::downgrade(&original);

// Turn the Arc into a raw pointer. The count stays 1; the pointer now "is" it.
let raw: *const Loud = Arc::into_raw(original);
assert_eq!(observer.strong_count(), 1);

// SAFETY: `raw` came from `Arc::into_raw` and its count has not been given up.
unsafe { Arc::increment_strong_count(raw) }; // a second pointer may now exist
assert_eq!(observer.strong_count(), 2);

// SAFETY: each `from_raw` takes over one of the two counts held for `raw`.
let first = unsafe { Arc::from_raw(raw) };
let second = unsafe { Arc::from_raw(raw) };
assert!(Arc::ptr_eq(&first, &second));

drop(first);
assert!(observer.upgrade().is_some(), "one Arc is left, the value is alive");
drop(second); // prints "dropping the value"
assert!(observer.upgrade().is_none(), "the value is gone");
// The block itself is freed only now, when the last Weak goes away.
drop(observer);
```

One `from_raw` too many would be a double free. One too few would be a leak. Your `Drop` impls must get the same accounting right on every path.

### Mistakes people really make

**A count that is not atomic.** With `Cell<usize>` counters you have written `Rc`, and the compiler will not let it cross a thread:

```rust,compile_fail,E0277
use std::rc::Rc;

fn main() {
    let shared = Rc::new(5);
    std::thread::spawn(move || println!("{shared}"));
}
```

```text
error[E0277]: `Rc<i32>` cannot be sent between threads safely
   --> c.rs:5:24
    |
  5 |     std::thread::spawn(move || println!("{shared}"));
    |     ------------------ -------^^^^^^^^^^^^^^^^^^^^^
    |     |                  |
    |     |                  `Rc<i32>` cannot be sent between threads safely
    |     |                  within this `{closure@c.rs:5:24: 5:31}`
    |     required by a bound introduced by this call
    |
    = help: within `{closure@c.rs:5:24: 5:31}`, the trait `Send` is not implemented for `Rc<i32>`
```

**`unsafe impl<T: Send> Send for MyArc<T>`.** It compiles, and the tests of your own code pass with `u32`. The lesson's tests ask the question with `Cell<u32>` and with `MutexGuard`.

**`Relaxed` on the decrement.** It compiles and passes every ordinary test on an x86 machine. The model test reports that the drop of the value is not ordered after the other thread's last read:

```text
thread 'l04_arc::model::a_last_drop_on_either_thread' ([..]) panicked at [..]/loom-0.7.3/src/rt/location.rs:115:9:
Causality violation: Concurrent read and write accesses to `UnsafeCell`.
```

**`upgrade` with `fetch_add`.** See above: it revives a count of zero.

**The same mistake on the weak counter.** The model cannot see it, because the model does not track freed memory. Miri can, when its scheduler produces the right interleaving. This is why you run both tools.

### When to use which

Use `std::sync::Arc`. Its `get_mut`, `make_mut` and `try_unwrap` are much harder to get right than what you build here, because they must exclude a concurrent `upgrade`. Build your own only to learn, or when you need a different layout, for example a count inside a type that C code also uses.

### Read more

- [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html) and [`Weak`](https://doc.rust-lang.org/std/sync/struct.Weak.html).
- The Rustonomicon chapter on [implementing `Arc`](https://doc.rust-lang.org/nomicon/arc-mutex/arc.html).
- [`NonNull`](https://doc.rust-lang.org/std/ptr/struct.NonNull.html), [`ManuallyDrop`](https://doc.rust-lang.org/std/mem/struct.ManuallyDrop.html) and the Rustonomicon on the [drop check](https://doc.rust-lang.org/nomicon/dropck.html).

## Lesson 05 · A mutex that puts waiting threads to sleep

### The model

The spin lock of lesson 02 has the right interface and the wrong way of waiting. A real mutex has two paths:

- The **fast path**: the mutex is free. One compare-exchange takes it. No system call, no allocation, no queue.
- The **slow path**: the mutex is taken. The thread must sleep until it is free, and the thread that unlocks must wake it.

All the difficulty is in the slow path, and all of it is one question: **what if the unlock happens between "I saw that it is taken" and "I am asleep"?** The unlocking thread looks for sleepers, finds none, and leaves. Then the waiter goes to sleep, and nobody will wake it. That is a lost wake-up again.

Lesson 03 solved this for one waiter that was known in advance. Now any number of threads may wait, so there must be a list of waiters, and that list is shared state of its own.

A protocol that works has this shape:

```text
thread that must wait                    thread that unlocks
1. register: put myself in the list      1. free the mutex (store false)
2. try once more to take the mutex       2. look at the list
3. if that failed: park                  3. if it names a waiter: unpark it
   (in a loop: after waking, go to 2)
4. once I own the mutex: leave the list
```

The argument for "no lost wake-up" compares waiter step 1 with unlocker step 2. Both touch the list, and the list has its own small lock, so one of them comes first.

- The waiter registered first. Then the unlocker sees it in the list and unparks it. If the waiter has not parked yet, the token waits for it.
- The unlocker looked first, and saw nothing. Then its step 1 (free the mutex) is also before the waiter's step 1, and therefore before the waiter's step 2. The waiter's second attempt runs after the mutex was freed, so it either takes the mutex or loses to a third thread, which will unlock later and look at the list then.

The important point is the order inside each column. The waiter registers *before* its final attempt. The unlocker frees the mutex *before* it looks at the list. Swap either pair and there is an interleaving in which the wake-up is lost.

Two more facts about this design:

- A woken thread does not own the mutex. It must compete for it, and a thread that arrives at that moment may take it first. This is called *barging*. It makes the mutex unfair, and usually faster. Because of barging, and because of spurious wake-ups, parking happens in a loop.
- The list has its own lock. That lock is held only for a push, a look or a removal, never across a `park`, so a spin lock is acceptable for it. This is the one place in the module where a spin lock is the right tool.

### Worked example: a gate that opens once

A `Gate` starts closed. Any number of threads can wait at it. `open` lets all of them through, and the gate stays open. The same "register, check again, then park" order appears, in a simpler setting: there is no ownership to hand over, and `open` wakes everybody.

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, Thread};

struct Gate {
    open: AtomicBool,
    waiting: Mutex<Vec<Thread>>,
}

impl Gate {
    fn new() -> Self {
        Gate { open: AtomicBool::new(false), waiting: Mutex::new(Vec::new()) }
    }

    fn wait(&self) {
        if self.open.load(Ordering::Acquire) {
            return; // fast path
        }
        self.waiting.lock().unwrap().push(thread::current()); // 1. register
        while !self.open.load(Ordering::Acquire) {
            // 2. check again   3. park
            thread::park();
        }
    }

    fn open(&self) {
        self.open.store(true, Ordering::Release); // 1. change the state
        let waiting = std::mem::take(&mut *self.waiting.lock().unwrap()); // 2. look
        for thread in waiting {
            thread.unpark(); // 3. wake
        }
    }
}

let gate = Arc::new(Gate::new());
let runners: Vec<_> = (0..3)
    .map(|n| {
        let gate = Arc::clone(&gate);
        thread::spawn(move || {
            gate.wait();
            n * 10
        })
    })
    .collect();
gate.open();
let results: Vec<i32> = runners.into_iter().map(|r| r.join().unwrap()).collect();
assert_eq!(results, [0, 10, 20]);
```

(The example uses `std::sync::Mutex` for the list to stay short. A mutex cannot be built on top of the mutex it is trying to be; yours uses the spin lock.)

### Mistakes people really make

**Check, register, park, with no second check.** This is the lost wake-up described above. The model test reports it as a deadlock, and it needs only two threads:

```text
thread 'l05_mutex::model::b_two_threads_take_turns' ([..]) panicked at [..]/loom-0.7.3/src/rt/execution.rs:215:13:
deadlock; threads = [(Id(0), Blocked(Location(None))), (Id(1), Terminated), (Id(2), Blocked(Location(None)))]
```

Thread 0 is the test's main thread, which waits in `join`. Thread 1 locked, unlocked and finished. Thread 2 is asleep in `park`, and nobody is left to wake it.

**Waking before freeing.** The woken thread tries to take the mutex, finds it still taken, and parks again. Then the mutex is freed, and nobody is woken.

**Parking while holding the lock of the list.** The unlocker then waits for the list lock, which the sleeper holds: a deadlock that looks like a hang.

**Removing a waiter from the list in `unlock`, and forgetting that it may have to wait again.** If the woken thread loses the race against a barging thread, it parks again, and now it is in no list. Either the waiter registers again each time before it parks, or (simpler) it stays in the list until it owns the mutex. This bug needs three threads: an owner, a sleeper and a thread that barges in. The lesson has a model test with exactly that cast.

**Spinning "just a little" instead of parking.** A thread that never parks never needs waking, so the tests for lost wake-ups pass, and the mutex burns a core per waiter. The lesson's tests check that a blocked `lock` really parks, and that it parks again when a `park` returned although the mutex is still taken.

**Believing that a thread which woke up owns the mutex.** `park` returns for three reasons: an unlock woke the thread, a token was left over, or no reason at all. Only a successful compare-exchange makes a thread the owner.

### What production mutexes do differently

`std::sync::Mutex` on Linux keeps no list in user space. The kernel's *futex* call does "sleep if this integer still has this value" as one step, which is exactly the "check again, then sleep" pair, and the kernel keeps the queue. The integer has three states: free, locked, and locked with waiters, so an unlock without waiters costs one atomic operation and no system call. Your mutex locks the list on every unlock; that is its main cost. The standard mutex also *poisons* itself when a thread panics while holding it. Yours does not: the guard simply unlocks.

### Read more

- [`std::thread::park`](https://doc.rust-lang.org/std/thread/fn.park.html) and [`Thread::unpark`](https://doc.rust-lang.org/std/thread/struct.Thread.html#method.unpark).
- [`std::sync::Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html), in particular the section on poisoning.
- The standard library's [`Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html).

## Lesson 06 · A bounded channel with `Mutex` and `Condvar`

### The model

This lesson has no `unsafe` and no atomics. It is about the most common way to make threads wait for each other correctly: a **monitor**. A monitor is shared state behind one mutex, plus one or more **condition variables**.

A condition variable (`Condvar`) is a place to sleep. `condvar.wait(guard)` does three things: it unlocks the mutex, puts the thread to sleep, and locks the mutex again before it returns. Unlocking and going to sleep are one step: no notification can fall in between. That is the same guarantee that lesson 05 built by hand.

The rule for using it has three parts:

1. Every wait has a **predicate**: a condition on the shared state that says "I must keep sleeping". For a sender: "the queue is full and the receiver is alive".
2. The predicate is checked **in a loop**, with the mutex held: `while predicate(&state) { state = condvar.wait(state).unwrap(); }`.
3. Every piece of code that can make a predicate **false** must notify the condition variable on which threads wait for that.

Part 2 is usually explained with spurious wake-ups, and they do exist. But there is a second reason that has nothing to do with them. A notified thread does not get the mutex at once. Another thread can take the mutex first and change the state back. For example: one slot becomes free, a waiting sender is notified, and before it runs, a *different* sender, which was not waiting, takes the slot. When the first sender wakes up, the queue is full again. An `if` would push anyway. The model test for this lesson finds exactly this run.

Part 3 is where bugs hide. List every event that can change a predicate:

| Event | Whose predicate changes | Notify |
| --- | --- | --- |
| a message is pushed | the receiver's ("queue empty") | one waiter is enough |
| a message is popped | one sender's ("queue full") | one waiter is enough |
| the last sender is dropped | the receiver's (nothing will ever come) | everyone waiting there |
| the receiver is dropped | every sender's (nobody will ever make room) | **all** of them |

`notify_one` wakes one waiting thread. It is right when one thread can use the change. `notify_all` is needed when the change concerns every waiter. If the receiver disappears and you wake one of three blocked senders, the other two sleep forever.

A **bounded** channel is also a tool for *backpressure*: when the consumer is slower than the producers, the producers are made to wait, and the queue cannot eat all memory. That is the reason to prefer it to an unbounded channel in a service.

### Worked example: a counting semaphore

A semaphore holds a number of permits. `acquire` takes one and waits if none is free. `release` gives one back.

```rust
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore { permits: Mutex::new(permits), available: Condvar::new() }
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        // Predicate: "no permit is free". Checked in a loop.
        while *permits == 0 {
            permits = self.available.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        *self.permits.lock().unwrap() += 1;
        // This can make one waiter's predicate false: wake one.
        self.available.notify_one();
    }
}

let semaphore = Arc::new(Semaphore::new(2));
let inside = Arc::new(Mutex::new((0_u32, 0_u32))); // (now inside, most ever inside)
let workers: Vec<_> = (0..6)
    .map(|_| {
        let (semaphore, inside) = (Arc::clone(&semaphore), Arc::clone(&inside));
        thread::spawn(move || {
            semaphore.acquire();
            {
                let mut inside = inside.lock().unwrap();
                inside.0 += 1;
                inside.1 = inside.1.max(inside.0);
            }
            inside.lock().unwrap().0 -= 1;
            semaphore.release();
        })
    })
    .collect();
for worker in workers {
    worker.join().unwrap();
}
let (now, most) = *inside.lock().unwrap();
assert_eq!(now, 0);
assert!(most <= 2, "never more than two threads hold a permit");
```

`wait` takes the guard by value and returns it. That is why the loop assigns the result back to `permits`.

### Mistakes people really make

**Using the guard after `wait` took it.** The compiler explains it:

```rust,compile_fail,E0382
use std::sync::{Condvar, Mutex};

fn main() {
    let ready = Mutex::new(false);
    let changed = Condvar::new();
    let guard = ready.lock().unwrap();
    while !*guard {
        changed.wait(guard).unwrap();
    }
}
```

```text
error[E0382]: borrow of moved value: `guard`
   --> d.rs:7:13
    |
  6 |     let guard = ready.lock().unwrap();
    |         ----- move occurs because `guard` has type `std::sync::MutexGuard<'_, bool>`, which does not implement the `Copy` trait
  7 |     while !*guard {
    |     --------^^^^^
    |     |       |
    |     |       value borrowed here after move
    |     inside of this loop
  8 |         changed.wait(guard).unwrap();
    |                      ----- value moved here, in previous iteration of loop
```

Ignore the compiler's suggestion here. The fix is `let mut guard = ...;` and `guard = changed.wait(guard).unwrap();` inside the loop.

**`if` instead of `while`.** Explained above. It is wrong even on a system without spurious wake-ups.

**A missing notification on a path that is rarely taken.** The usual candidates are the `Drop` impls. A test that only sends and receives never notices. The lesson's tests drop each end while the other end is blocked.

**`notify_one` where every waiter is concerned.**

**Changing the state without the mutex.** If the shared state were an atomic that is changed without locking, a waiter could check it, then be notified, and only then go to sleep: a lost wake-up. The state that a predicate reads must be changed with the mutex held.

**Running slow or foreign code while the mutex is held.** Dropping a message can run any `Drop` impl. Take the messages out of the queue first, unlock, and drop them afterwards.

### When to use which

In real programs, use `std::sync::mpsc::sync_channel` for a bounded channel between threads, or a channel from a well-tested crate when you need several receivers or `select`. Reach for a hand-written monitor when the waiting condition is specific to your program: "wait until the cache is warm", "wait until fewer than N jobs run". The three-part rule is the same every time.

### Read more

- [`std::sync::Condvar`](https://doc.rust-lang.org/std/sync/struct.Condvar.html), including `wait_while`, which writes the loop for you.
- [`std::sync::mpsc::sync_channel`](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html).

## Lesson 07 · A lock-free stack, and the ABA problem

### The model

A data structure is **lock-free** if, whenever several threads work on it, at least one of them finishes its operation in a bounded number of steps, whatever the others do, even if one of them is stopped for ever. A mutex does not have this property: if the owner is stopped, everybody waits. A lock-free structure has no owner. Each thread prepares its change privately and then tries to install it with one compare-exchange. If that fails, somebody else succeeded, and the thread tries again. (*Wait-free* is stronger: every thread finishes in a bounded number of steps. Lesson 08 builds such a queue.)

The simplest lock-free structure is a stack made of a linked list and one atomic pointer, `head`. It is named after R. Kent Treiber, who described it in 1986.

```text
head --> [ C | next ] --> [ B | next ] --> [ A | null ]
```

**Push** needs no care beyond lesson 01. Allocate a node. Read `head`. Point the node's `next` at what you read. Compare-exchange `head` from what you read to your node. On failure, read again and repeat. The node is private until the compare-exchange succeeds, and the success publishes it: `Release`.

**Pop** looks just as simple, and is not:

```text
1. read head              -> pointer to node C
2. read C.next            -> pointer to node B
3. compare-exchange head from C to B
```

Between step 1 and step 3 the thread holds a pointer to a node that it does not own. Two different things can go wrong.

**Use after free.** Another thread pops C and frees it. Step 2 then reads freed memory.

**ABA.** Suppose step 2 survived. Another thread pops C, pops B, and pushes a node again. If C was freed, the allocator may hand out the same address for the new node. Now `head` holds the same pointer value as in step 1. The compare-exchange in step 3 compares addresses, sees "unchanged", and installs B. But B is no longer in the stack. The stack is now corrupt. The name says what happened: the value went from A to B and back to A, and a compare-exchange cannot tell.

```text
this thread                 another thread
read head = C
read C.next = B
                            pop C         head = B
                            pop B         head = A
                            push C again  head = C, C.next = A
compare-exchange C -> B  succeeds!        head = B, a node that was removed
```

Both problems have one cause: **a node was freed, or reused, while another thread still held a pointer to it.** So the real subject of this lesson is *memory reclamation*: when may a node be freed?

### The answer in this lesson: not before `Drop`

`pop` does not free the node it removed. It puts the node aside, on a second list, and the stack's `Drop` frees everything. `Drop` has `&mut self`, so no other thread can be in the middle of an operation.

This gives two invariants, and each kills one problem:

1. *A node that was ever published stays allocated until `Drop`.* So step 2 never reads freed memory.
2. *A node enters the stack once and leaves it once.* So if `head` still holds the pointer from step 1, the node never left, nothing below it changed, and its `next` is still right. There is no ABA.

The price is memory: a stack that is pushed and popped for a long time keeps every node it ever had. For a teaching implementation, and for stacks with a short life, that is acceptable. It is not what production code does.

### What production code does

- **Hazard pointers.** Before step 2, a thread publishes "I am looking at this node" in a shared table. A thread that wants to free a node checks the table first.
- **Epochs.** Threads announce when they enter and leave the data structure. A removed node is freed after every thread that could have seen it has left. The `crossbeam-epoch` crate does this, and it is what you would use.
- **Version counters.** Store a counter next to the pointer or the index, and add one on every change. The compare-exchange then compares both, and "back to the same pointer" has a different counter. The code review lesson of this module needs exactly this fix. It solves ABA; it does not solve use after free.

### Why `Acquire` on one load is enough

In `pop`, the load of `head` must be `Acquire`: the thread is about to read the node's fields, which the pushing thread wrote, and that needs rule 3 of lesson 01.

But look closely. A pop installs B as the new head with its own compare-exchange. A later thread loads `head` and gets B *from the store of the popping thread*, not from the store of the thread that pushed B. Is the write of B's fields ordered before this read?

It is, by the release-sequence rule that lesson 01 only named. A `Release` operation (a store, or a read-modify-write operation such as the successful compare-exchange in `push`) starts a **release sequence**: the operation itself, and every read-modify-write operation that follows it on the same atomic without an ordinary store in between. The later operations of the sequence may have any ordering, also `Relaxed`. An `Acquire` load that reads from *any* operation of the sequence is ordered after the `Release` operation that started it. Every change of `head` is a compare-exchange, which is a read-modify-write operation. So every earlier push starts an unbroken sequence that reaches the value you read, and an `Acquire` load of `head` is ordered after *all* earlier pushes. This is why the successful compare-exchange in `pop` can be `Relaxed`. If your code ever changes `head` with a plain `store`, the argument is gone.

### Worked example: a log that only grows

This structure only pushes. Readers walk the list through shared references while writers add to the front. Nothing is removed before `Drop`, so neither problem above can occur, and the two orderings stand out.

```rust
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::thread;

struct Entry<T> {
    value: T,
    older: *mut Entry<T>,
}

pub struct Log<T> {
    newest: AtomicPtr<Entry<T>>,
}

// SAFETY: `add` moves a `T` in from any thread (`T: Send`), and `newest_first`
// gives `&T` to any thread (`T: Sync`). Entries are never changed after they
// are published and never freed before `Drop`.
unsafe impl<T: Send + Sync> Sync for Log<T> {}
// SAFETY: the log owns its entries; moving it moves the `T`s.
unsafe impl<T: Send> Send for Log<T> {}

impl<T> Log<T> {
    pub fn new() -> Self {
        Log { newest: AtomicPtr::new(ptr::null_mut()) }
    }

    pub fn add(&self, value: T) {
        let entry = Box::into_raw(Box::new(Entry { value, older: ptr::null_mut() }));
        let mut newest = self.newest.load(Ordering::Relaxed);
        loop {
            // SAFETY: `entry` is not published yet; this thread owns it.
            unsafe { (*entry).older = newest };
            match self.newest.compare_exchange_weak(
                newest,
                entry,
                Ordering::Release, // publishes the entry's fields
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(now) => newest = now,
            }
        }
    }

    pub fn newest_first(&self) -> Vec<&T> {
        let mut seen = Vec::new();
        let mut entry = self.newest.load(Ordering::Acquire);
        // SAFETY: a published entry is initialised (Release/Acquire), is never
        // changed again, and lives until `Drop`, which needs `&mut self`. So
        // the reference may live as long as `&self`. The same holds for every
        // `older` link, which was published before this entry.
        while let Some(current) = unsafe { entry.as_ref() } {
            seen.push(&current.value);
            entry = current.older;
        }
        seen
    }
}

impl<T> Drop for Log<T> {
    fn drop(&mut self) {
        let mut entry = *self.newest.get_mut();
        while !entry.is_null() {
            // SAFETY: every entry came from `Box::into_raw` and is freed once.
            let owned = unsafe { Box::from_raw(entry) };
            entry = owned.older;
        }
    }
}

let log = Log::new();
thread::scope(|s| {
    for id in 0..4 {
        let log = &log;
        s.spawn(move || log.add(id));
    }
});
let mut seen: Vec<i32> = log.newest_first().into_iter().copied().collect();
seen.sort_unstable();
assert_eq!(seen, [0, 1, 2, 3]);
```

Your stack must also remove entries. That is where the two invariants above have to be kept by your code.

### Mistakes people really make

**Freeing the node in `pop`.** It passes every ordinary test, almost every time. Miri sees the conflict between one thread reading `next` and another thread taking the node back as a `Box`:

```text
error: Undefined Behavior: Data race detected between (1) non-atomic read on thread `unnamed-8` and (2) retag write of type `l07_treiber::Node<support::Counted>` on thread `unnamed-7` at alloc108974+0x10
     = help: retags occur on all (re)borrows and as well as when references are copied or moved
     = help: therefore from the perspective of data races, a retag has the same implications as a read or write
```

"Retag write" is Miri's name for the moment at which `Box::from_raw` claims the whole node as its own. Which report you get depends on the schedule that Miri happens to choose. With another seed the same mistake is reported as a plain use after free, at the line that reads `next`:

```text
error: Undefined Behavior: in-bounds pointer arithmetic failed: alloc461816 has been freed, so this pointer is dangling
```

Loom does not see it, because loom does not track freed memory. The lesson has an ordinary test that counts frees instead.

**`Relaxed` in `push`.** The model test reports the missing order between the write of the value and its read after `pop`. This mistake is not only a matter of theory. On a processor that reorders stores, such as the ARM processors in current phones and laptops, a thread in `pop` can see the new `head` before it can see the node's `next` field, follow an old link, and corrupt the list. The ordinary stress test of the lesson then fails, or the test program crashes; on the Apple M-series machine on which this text was checked, that happened in every run. On an x86 processor the same code usually passes every ordinary test, because x86 does not reorder stores. The bug is still there: the compiler is allowed to reorder the two writes as well.

**Re-reading with `Relaxed` after a failed compare-exchange.** If your loop uses the value that the failed compare-exchange returned as the next node to look at, that value needs `Acquire` too: the failure ordering.

**No `Send` and `Sync` impls at all.** In the earlier lessons, forgetting them was a compile error. Here it is worse: it compiles. `AtomicPtr<Node<T>>` is `Send` and `Sync` for every `T`, so a struct made of atomic pointers is automatically both, also for `T = Rc<u8>`. Writing `unsafe impl<T: Send> Send for Stack<T> {}` and the same for `Sync` replaces the automatic impls, so here the explicit impls are what *restricts* the type. A stack moves values from the pushing thread to the popping thread; that is sending, so `T: Send` is the bound for both.

### When to use which

`Mutex<Vec<T>>` is simpler, and for most workloads it is also faster: an uncontended mutex is two atomic operations, and a `Vec` has no allocation per element. Choose a lock-free structure when you need its guarantee: code that must not block because it holds a deadline or runs where sleeping is not allowed (an interrupt or signal handler; there the code must not allocate either, so the nodes must come from a pool), or a measured contention problem. Then use a tested crate. Lock-free code that looks right and is wrong is the normal case.

### Read more

- [`AtomicPtr`](https://doc.rust-lang.org/std/sync/atomic/type.AtomicPtr.html).
- The [module documentation of `std::sync::atomic`](https://doc.rust-lang.org/std/sync/atomic/index.html) for the memory model that these arguments rest on.
- The Rustonomicon on [working with `Box::into_raw` and ownership](https://doc.rust-lang.org/nomicon/ownership.html).

## Lesson 08 · Capstone: a wait-free ring buffer

The capstone has no worked example, on purpose. This section gives you the design space and a checklist.

### The shape of the problem

A ring buffer is a fixed array of slots and two positions. The producer writes at `tail` and moves `tail` forward. The consumer reads at `head` and moves `head` forward. Both wrap around at the end of the array.

```text
slots:   [ . ][ x ][ x ][ x ][ . ][ . ]
                ^              ^
               head           tail
```

With exactly one producer and one consumer, something remarkable is true: **each index is written by one thread only.** No compare-exchange is needed, and no loop. `push` and `pop` each read both indices, touch one slot, and store one index. That is wait-free.

### Decisions you must make

**How to tell "full" from "empty".** If both mean `head == tail`, they cannot be told apart. Three common answers:

- Keep one slot unused: empty is `head == tail`, full is `tail + 1 == head` (wrapping). Simple; costs one slot.
- Let the indices count up for ever and take them modulo the capacity when you index. The number of stored values is `tail - head`, computed with `wrapping_sub`. This is exact, but only if the slot sequence stays continuous when the counter wraps past `usize::MAX`, and that is true only for a capacity that is a power of two. The predictions in the lesson show why.
- Keep a separate atomic count. Now two threads write the same atomic, and the design is no longer as simple.

**Which slots are initialised.** Write it as an invariant in terms of `head` and `tail`. `Drop` needs it to drop the values that were never popped, and nothing else.

**The four orderings.** Each side loads its own index and the other side's index, and stores its own. For each of the four operations, ask what it must be ordered with:

- The consumer reads a slot that the producer wrote. Which store announces the write, and which load receives the announcement?
- The producer writes a slot that the consumer emptied on an earlier lap. Reading out of a slot and writing into it are two accesses to the same memory. Which store announces that the consumer is done with the slot, and which load receives that?
- A thread that loads the index that only it writes: what can it possibly be ordered with?

**`Send` and `Sync`.** The two ends are separate values on separate threads, and they share one block. Which type needs which impl, with which bound? A queue moves values; it never shares them.

### Checklist before you call it done

- Capacity 1 works. Capacity 3 (not a power of two) works. The indices wrap many times in a test.
- A value that is pushed and never popped is dropped exactly once. A value that is popped is not dropped by the queue.
- A rejected `push` hands back the same value.
- `push` and `pop` have no loop and no allocation.
- All three kinds of check pass: ordinary tests in debug and release, the model, and Miri.
- You wrote one more model test of your own, and you can say which mistake it would catch.

### Faster, later

Two optimisations are worth knowing, and neither belongs in a first version. Each end can keep a private copy of the other side's index and reload it only when the copy says "full" or "empty"; that removes most of the cross-thread loads. And `head` and `tail` can be placed on different cache lines (`#[repr(align(N))]` on a wrapper type, where `N` is the line size: 64 bytes on most x86-64 processors, 128 on some 64-bit ARM processors), so that the producer's stores do not invalidate the cache line that the consumer reads: this effect is called *false sharing*. Measure before and after; the mastery module on performance shows how.

### Read more

- [`AtomicUsize`](https://doc.rust-lang.org/std/sync/atomic/type.AtomicUsize.html) and [`usize::wrapping_sub`](https://doc.rust-lang.org/std/primitive.usize.html#method.wrapping_sub).
- [`MaybeUninit`](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html), in particular the section on arrays of uninitialised values.
- The Miri [README](https://github.com/rust-lang/miri#readme) for its flags, for example `-Zmiri-many-seeds`.
