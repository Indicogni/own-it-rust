# Chapter 14 concepts — Closures and captures

[Chapter guide](README.md) · [Hints](HINTS.md)

Read one section, then do the lesson with the same number. Every code block here was compiled with Rust 1.99 (edition 2024), and every compiler message was copied from that compiler and shortened.

## What you will be able to do

**After this chapter you can**

- say, for any closure, how it captures each variable: by shared borrow, by mutable borrow, or by value;
- choose the weakest of `Fn`, `FnMut` and `FnOnce` for a function that takes a callback;
- return closures from functions, with `impl Fn` or with `Box<dyn Fn>`, and explain the lifetime rules for both;
- predict when a closure is `Copy` or `Clone`;
- build a registry that stores callbacks of different types and calls them later.

## Before you start

Ownership and moves (chapter 4), references and borrows (chapter 5), structs (chapter 9), traits, generics, `impl Trait` and `dyn Trait` (chapter 11), comparing values with `cmp` and `Ordering` (chapter 12), and `Clone`, `Copy` and `Drop` (chapter 13). The examples use `Vec`, `String`, and simple iterator calls such as `iter().map(..).collect()`. Iterators get their own chapter next.

## Time

People differ a lot; use these numbers only for planning.

| Part | Estimate |
| --- | --- |
| Lesson 01 · Borrowed captures and sorting | 45 min |
| Lesson 02 · Mutation, move, and consuming captures | 50 min |
| Lesson 03 · Choosing callable bounds | 50 min |
| Lesson 04 · Returning borrowed and erased closures | 50 min |
| Lesson 05 · Copy and Clone for captures | 45 min |
| Lesson 06 · Callback registration and invocation | 50 min |
| Lesson 07 · Capstone | 45 min |
| R01 review, C01 code review, 11 drills | 2 h |

## Lesson 01 · Borrowed captures and sorting

A **closure** is a function that you write in place, as an expression: `|parameters| body`. A function declared with `fn` can use only its parameters. A closure can also use the local variables around it. Using an outside variable is called **capturing** it.

One mental model explains almost everything in this chapter:

> A closure is a value of a struct type that the compiler invents. The struct has one field for each captured variable. The body is a method of that struct.

You cannot write the name of this type, but it behaves like any other struct. It has a size, it lives on the stack unless you box it, and the borrow rules of chapter 5 apply to its fields.

When the body only **reads** a variable, the field is a shared reference to that variable. The compiler always picks the weakest kind of capture that lets the body compile, and a shared borrow is the weakest.

```rust
fn main() {
    let limit = 30;
    let readings = [28, 31, 35, 22];

    // `is_hot` reads `limit`, so it captures a shared borrow of `limit`.
    let is_hot = |t: i32| t > limit;

    let mut hot = 0;
    for t in readings {
        if is_hot(t) {
            hot += 1;
        }
    }
    assert_eq!(hot, 2);

    // A shared borrow takes nothing away. `limit` is still yours.
    assert_eq!(limit, 30);
}
```

```text
stack frame of main
  limit    30  <--------------+
  is_hot   { limit: &i32 } ---+     the closure value: a struct with one field
(nothing is on the heap)
```

The borrow lasts while the closure is still **used**, not until the end of the block. After the last call of `is_hot`, nothing borrows `limit` any more.

A capture is also precise. A closure that reads `job.name` borrows only that field, so other fields of `job` stay free:

```rust
struct Job {
    name: String,
    retries: u32,
}

fn main() {
    let mut job = Job { name: String::from("sync"), retries: 0 };
    let describe = || format!("job {}", job.name); // borrows `job.name` only
    job.retries += 1; // a different field: allowed
    assert_eq!(describe(), "job sync");
    assert_eq!(job.retries, 1);
}
```

### Closures as sorting rules

Slices have sorting methods that take a closure:

- `sort_by_key(|item| key)`: you return a key for each item, and the items are ordered by their keys.
- `sort_by(|a, b| ordering)`: you receive two items and return a `std::cmp::Ordering`.

Both are **stable**: items that compare as equal keep the order they had before. To order by one thing and break ties by a second, join two comparisons with `Ordering::then_with` (chapter 12): `first.then_with(|| second)` runs the second comparison only when the first one says "equal".

```rust
/// The last word of a full name, borrowed from it.
fn surname(full: &str) -> &str {
    full.rsplit(' ').next().unwrap_or(full)
}

fn main() {
    // The key closure borrows `origin` while `stops` is being sorted.
    let origin = 50;
    let mut stops: Vec<i32> = vec![70, 45, 10, 52];
    stops.sort_by_key(|&km| (km - origin).abs());
    assert_eq!(stops, [52, 45, 70, 10]);

    // `sort_by` lends you two items, so you can compare borrowed parts.
    let mut crew = vec!["Mei Tan", "Ada Obi", "Lars Berg"];
    crew.sort_by(|a, b| surname(a).cmp(surname(b)));
    assert_eq!(crew, ["Lars Berg", "Ada Obi", "Mei Tan"]);
}
```

### Mistakes people make

**Changing a variable while a closure that borrowed it is still in use.**

```rust,compile_fail,E0506
fn main() {
    let mut limit = 30;
    let is_hot = |t: i32| t > limit;
    limit = 35;
    println!("{}", is_hot(32));
}
```

```text
error[E0506]: cannot assign to `limit` because it is borrowed
 --> main.rs:4:5
  |
3 |     let is_hot = |t: i32| t > limit;
  |                  --------     ----- borrow occurs due to use in closure
  |                  |
  |                  `limit` is borrowed here
4 |     limit = 35;
  |     ^^^^^^^^^^ `limit` is assigned to here but it was already borrowed
5 |     println!("{}", is_hot(32));
  |                    ------ borrow later used here
```

Fix: assign after the last call of the closure, or give the closure its own copy with `move` (lesson 02).

**Returning a borrowed key from `sort_by_key`.** The sort lends each item to your closure only for the length of one call, and the key must stay usable after that call. So the key cannot be a reference into the item.

```rust,compile_fail
fn main() {
    let mut crew = vec![String::from("Mei Tan"), String::from("Ada Obi")];
    crew.sort_by_key(|full| full.as_str());
}
```

```text
error: lifetime may not live long enough
 --> main.rs:3:29
  |
3 |     crew.sort_by_key(|full| full.as_str());
  |                       ----- ^^^^^^^^^^^^^ returning this value requires that `'1` must outlive `'2`
  |                       |   |
  |                       |   return type of closure is &'2 str
  |                       has type `&'1 String`
```

Fix: use `sort_by`, which lends both items to your closure: `crew.sort_by(|a, b| a.cmp(b))`. Cloning the key also compiles, but it allocates a new `String` for every comparison.

### When to use which

| Method | Use it when |
| --- | --- |
| `sort_by_key` | the key is a cheap value: a number, a `char`, a tuple of these |
| `sort_by` | you compare borrowed data, or the order is not a simple key |
| `sort_by_cached_key` | the key is expensive to compute; it is computed once per item and stored |

### Read more

- [Closure expressions](https://doc.rust-lang.org/reference/expressions/closure-expr.html) in the Rust Reference
- [Closure types: capture modes and capture precision](https://doc.rust-lang.org/reference/types/closure.html#capture-modes) in the Rust Reference
- [`slice::sort_by_key`](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by_key) and [`slice::sort_by`](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by)

## Lesson 02 · Mutation, move, and consuming captures

Lesson 01 showed the weakest capture, a shared borrow. There are three capture modes, and the compiler chooses one for each captured variable by looking at what the body does with it.

| The body ... | Capture without `move` | The closure can be called |
| --- | --- | --- |
| only reads `x` | `&x` (shared borrow) | many times, through `&closure` |
| changes `x` | `&mut x` (mutable borrow) | many times, through `&mut closure` |
| moves `x` out (returns it, drops it, passes it by value) | `x` itself (by value) | once: the call uses the closure up |

The third column has names. They are traits, and lesson 03 studies them: `Fn`, `FnMut` and `FnOnce`.

The third row is about types that are not `Copy`. A `Copy` value cannot be moved out: using it by value copies it, which only reads the original. So without `move`, a closure such as `|| drop(n)` with an integer `n` captures `&n`, and you can call it many times.

### A closure that changes its capture

```rust
fn main() {
    let mut next = 0;
    // The body changes `next`, so the closure holds `&mut next`.
    // Calling it changes the closure's state, so the binding needs `mut`.
    let mut fresh_id = || {
        next += 1;
        next
    };
    assert_eq!(fresh_id(), 1);
    assert_eq!(fresh_id(), 2);
    // The closure is not used again, so its mutable borrow has ended.
    assert_eq!(next, 2);
}
```

While `fresh_id` is still going to be used, nothing else may read or write `next`. This is the normal rule for `&mut` from chapter 5. The closure only makes the borrow harder to see, because no `&mut` appears in the source.

### What `move` does

`move` before the parameter list changes the middle column of the table: **every** captured variable goes into the closure by value. It does not change the last column. How often a closure can be called still depends only on what the body does.

```rust
fn main() {
    let banned = vec!["spam", "scam"];
    // `move`: the closure now owns the vector. The body only reads it,
    // so the closure can still be called many times.
    let is_banned = move |word: &str| banned.contains(&word);
    assert!(is_banned("spam"));
    assert!(!is_banned("soup"));
    // `banned` cannot be used here any more: it was moved into `is_banned`.
}
```

```text
before `move`      main: banned ---------------------> heap: ["spam", "scam"]

after `move`       main: banned (moved, unusable)
                   main: is_banned { banned: Vec } --> heap: ["spam", "scam"]
```

Only the three-word `Vec` header moved. The heap buffer stayed where it was. The vector is now dropped when the closure is dropped.

You need `move` when the closure must live longer than the variables it uses: when you return it (lesson 04), store it (lesson 06), or send it to another thread (chapter 19).

### A closure that consumes its capture

```rust
fn main() {
    let mut archive: Vec<String> = Vec::new();
    let title = String::from("Q3 report");
    // `push` takes its argument by value, so the body moves `title` out of
    // the closure. That can happen only once.
    let file_it = || archive.push(title);
    file_it();
    assert_eq!(archive, ["Q3 report"]);
}
```

No `move` keyword is needed here. The body uses `title` by value, so the closure captures it by value. `archive` is only changed, so it is captured as `&mut archive`.

### Mistakes people make

**Calling a consuming closure twice.**

```rust,compile_fail,E0382
fn main() {
    let mut archive: Vec<String> = Vec::new();
    let title = String::from("Q3 report");
    let file_it = || archive.push(title);
    file_it();
    file_it();
}
```

```text
error[E0382]: use of moved value: `file_it`
 --> main.rs:6:5
  |
5 |     file_it();
  |     --------- `file_it` moved due to this call
6 |     file_it();
  |     ^^^^^^^ value used here after move
  |
note: closure cannot be invoked more than once because it moves the variable `title` out of its environment
 --> main.rs:4:35
  |
4 |     let file_it = || archive.push(title);
  |                                   ^^^^^
```

Fix: if every call needs its own value, clone inside the body (`archive.push(title.clone())`). The body then only reads `title`, and the closure can be called again.

**Forgetting `mut` on a closure that changes its captures.**

```rust,compile_fail,E0596
fn main() {
    let mut next = 0;
    let fresh_id = || {
        next += 1;
        next
    };
    println!("{}", fresh_id());
}
```

```text
error[E0596]: cannot borrow `fresh_id` as mutable, as it is not declared as mutable
 --> main.rs:7:20
  |
4 |         next += 1;
  |         ---- calling `fresh_id` requires mutable binding due to mutable borrow of `next`
...
7 |     println!("{}", fresh_id());
  |                    ^^^^^^^^ cannot borrow as mutable
  |
help: consider changing this to be mutable
  |
3 |     let mut fresh_id = || {
  |         +++
```

**Expecting `move` to share a `Copy` value.** Moving a `Copy` value copies it. A `move` closure that changes a captured integer changes its own private copy, and the original variable keeps its old value. There is no compiler error, only a surprising result. If you want the original to change, do not write `move`, so that the closure borrows it mutably.

### When to use which

- No `move`: the closure is used and finished inside the current function. This is the common case for `map`, `filter`, `sort_by` and similar calls.
- `move`: the closure leaves the current function or thread, or you want it to own its data.
- `move`, but one variable should stay borrowed: make a reference first (`let names = &names;`) and let the closure move the reference.

### Read more

- [Capture modes](https://doc.rust-lang.org/reference/types/closure.html#capture-modes) in the Rust Reference
- [`move` closures](https://doc.rust-lang.org/std/keyword.move.html) in the standard library documentation
- [Error E0382](https://doc.rust-lang.org/error_codes/E0382.html) and [error E0596](https://doc.rust-lang.org/error_codes/E0596.html)

## Lesson 03 · Choosing callable bounds

The three ways to call a closure from lesson 02 are three traits. In a simplified form:

```text
trait FnOnce<Args>               { fn call_once(self, args: Args) -> Output; }
trait FnMut<Args>: FnOnce<Args>  { fn call_mut(&mut self, args: Args) -> Output; }
trait Fn<Args>: FnMut<Args>      { fn call(&self, args: Args) -> Output; }
```

Look at the `self` parameter of each method.

- `FnOnce`: the call takes the closure by value. Every closure implements it, because you can always call a closure at least once.
- `FnMut`: the call needs `&mut` access. A closure implements it when its body does not move a capture out.
- `Fn`: the call needs only `&` access. A closure implements it when its body also does not change a capture.

So the closures are nested: every `Fn` closure is also `FnMut` and `FnOnce`.

Now change sides. When **you** write a function that takes a closure, the bound is a promise about how your function will call it. Ask for the weakest trait your body needs, because a weaker bound accepts more closures:

| Your function calls the closure | Bound | It accepts |
| --- | --- | --- |
| at most once | `FnOnce` | every closure |
| several times, one call after another | `FnMut` | closures that read or change captures |
| through a shared reference, or from several places at once | `Fn` | closures that only read captures |

```rust
/// Calls `op` with the attempt number until it returns a value.
fn retry<T>(attempts: u32, mut op: impl FnMut(u32) -> Option<T>) -> Option<T> {
    for attempt in 1..=attempts {
        if let Some(value) = op(attempt) {
            return Some(value);
        }
    }
    None
}

/// Returns the cached value, or computes one. `compute` runs at most once.
fn cached_or<T>(cache: Option<T>, compute: impl FnOnce() -> T) -> T {
    match cache {
        Some(value) => value,
        None => compute(),
    }
}

fn main() {
    // A closure that changes a capture is welcome, because `retry` asks for FnMut.
    let mut tried = Vec::new();
    let found = retry(5, |attempt| {
        tried.push(attempt);
        (attempt == 3).then_some("connected")
    });
    assert_eq!(found, Some("connected"));
    assert_eq!(tried, [1, 2, 3]);

    // A closure that gives away its capture is welcome, because `cached_or`
    // asks only for FnOnce.
    let fallback = String::from("offline copy");
    assert_eq!(cached_or(None, move || fallback), "offline copy");
}
```

Notice `mut op` in `retry`. Calling an `FnMut` needs `&mut` access, so the parameter must be declared `mut`. Without it you get E0596, as in lesson 02.

### Functions are callable too

Each closure has its own unique type, even when two closures look the same. That is why functions that take closures are generic (`impl Fn(..)` or `F: Fn(..)`). A closure itself is not generic: once `let same = |x| x;` has been called with an integer, calling it with a `&str` is a type error (E0308).

A function declared with `fn` also implements all three traits, so you can pass its name wherever a closure is expected. The type `fn(i32) -> i32` is a **function pointer**: one address, no captured data. A closure that captures nothing can be converted to a function pointer. A closure that captures something cannot.

```rust
fn double(x: i32) -> i32 {
    x * 2
}

fn main() {
    // One array type for a function and a closure without captures.
    let ops: [fn(i32) -> i32; 2] = [double, |x| x + 1];
    assert_eq!(ops.map(|op| op(10)), [20, 11]);
}
```

### Mistakes people make

**Asking for `Fn` when `FnMut` is enough.** The function below only calls `action` several times. Its `Fn` bound rejects a harmless closure.

```rust,compile_fail,E0525
fn repeat(times: u32, action: impl Fn()) {
    for _ in 0..times {
        action();
    }
}

fn main() {
    let mut log = Vec::new();
    let record = || log.push("tick");
    repeat(3, record);
}
```

```text
error[E0525]: expected a closure that implements the `Fn` trait, but this closure only implements `FnMut`
  --> main.rs:9:18
   |
 9 |     let record = || log.push("tick");
   |                  ^^ --- closure is `FnMut` because it mutates the variable `log` here
   |                  |
   |                  this closure implements `FnMut`, not `Fn`
10 |     repeat(3, record);
   |     ------    ------ the requirement to implement `Fn` derives from here
```

Fix the function, not the caller: `fn repeat(times: u32, mut action: impl FnMut())`.

**A closure that returns a reference taken from its argument.** When a closure is stored in a variable, the compiler works out its signature alone, and it does not connect the lifetime of the returned reference to the lifetime of the argument.

```rust,compile_fail
fn main() {
    let first_word = |line: &str| line.split(' ').next().unwrap_or("");
    println!("{}", first_word("hello world"));
}
```

```text
error: lifetime may not live long enough
 --> main.rs:2:35
  |
2 |     let first_word = |line: &str| line.split(' ').next().unwrap_or("");
  |                             -   - ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ returning this value requires that `'1` must outlive `'2`
  |                             |   |
  |                             |   return type of closure is &'2 str
  |                             let's call the lifetime of this reference `'1`
```

Fix: write a `fn` instead. A function signature follows the lifetime elision rules of chapter 5, so `fn first_word(line: &str) -> &str` means what you expect. Writing the closure directly as an argument to a function with the bound `F: Fn(&str) -> &str` also works, because the bound gives the compiler the full signature. That bound is short for `for<'a> Fn(&'a str) -> &'a str`, a *higher-ranked* bound. The mastery track ([`mastery/m1_types`](../../mastery/m1_types/README.md)) studies these bounds in depth.

### When to use which

| Parameter type | Good for | Cost |
| --- | --- | --- |
| `impl Fn(..)` or `F: Fn(..)` | most functions | one compiled copy per closure type; calls can be inlined |
| `&dyn Fn(..)` or `Box<dyn Fn(..)>` | storing different closures together; keeping code size small | a call through a pointer table |
| `fn(..) -> ..` | tables of plain functions; C interfaces (chapter 24) | cannot capture anything |

### Read more

- [`FnOnce`](https://doc.rust-lang.org/std/ops/trait.FnOnce.html), [`FnMut`](https://doc.rust-lang.org/std/ops/trait.FnMut.html) and [`Fn`](https://doc.rust-lang.org/std/ops/trait.Fn.html) in the standard library documentation
- [Call traits and coercions](https://doc.rust-lang.org/reference/types/closure.html#call-traits-and-coercions) in the Rust Reference
- [Function pointer types](https://doc.rust-lang.org/reference/types/function-pointer.html) in the Rust Reference
- [Higher-ranked trait bounds](https://doc.rust-lang.org/nomicon/hrtb.html) in the Rustonomicon

## Lesson 04 · Returning borrowed and erased closures

A function can build a closure and return it. Two problems appear: you cannot write the closure's type, and the closure may hold borrows that must stay valid.

### Two return types

- `-> impl Fn(u32) -> u32` means "one concrete type that implements this trait, and I will not say which". The closure is returned by value, like any struct. There is no allocation, and calls are direct. All return paths must give the **same** type.
- `-> Box<dyn Fn(u32) -> u32>` puts the closure on the heap and returns a pointer to it together with a pointer to a table of its methods (a trait object, chapter 11). Different return paths may box **different** closures. The price is one allocation and an indirect call. A closure with no captures has size zero, and boxing a zero-sized value does not allocate.

```rust
struct Tariff {
    base: u32,
    per_km: u32,
}

/// One closure type, so `impl Fn` is enough. The closure borrows `tariff`.
fn fare_for(tariff: &Tariff) -> impl Fn(u32) -> u32 {
    move |km| tariff.base + tariff.per_km * km
}

/// Three different callables behind one boxed type.
fn rounding(mode: &str) -> Box<dyn Fn(f64) -> f64> {
    match mode {
        "down" => Box::new(f64::floor),
        "up" => Box::new(f64::ceil),
        _ => Box::new(|x| x),
    }
}

fn main() {
    let city = Tariff { base: 300, per_km: 120 };
    let fare = fare_for(&city);
    assert_eq!(fare(5), 900);
    assert_eq!(rounding("down")(2.7), 2.0);
    assert_eq!(rounding("up")(2.1), 3.0);
}
```

```text
fare_for(&city) returns the closure by value:

  main:  city  { base: 300, per_km: 120 }  <--+
         fare  { tariff: &Tariff } -----------+     one reference, on the stack

rounding("down") returns a box:

  main:  (data pointer, table pointer)
               |              +--> table: how to call and drop this callable
               +--> the callable itself (zero bytes here, so nothing was allocated)
```

In `fare_for`, `move` copies the reference `tariff` into the closure. It does not copy the `Tariff`. A returned closure almost always needs `move`, because the function's own variables are gone after it returns.

### Lifetimes of returned closures

`fare` holds a reference to `city`, so `fare` must not be used after `city` is gone. Nothing in the signature of `fare_for` says this, and it still works. The rule in edition 2024 is: **a returned `impl Trait` may use every lifetime and type parameter that is in scope.** The compiler assumes that the returned value borrows from every reference argument.

Two details are worth knowing.

- In editions before 2024, this signature was an error (E0700) until you wrote `impl Fn(u32) -> u32 + '_`. You will see that bound in older code. It is still allowed.
- If the returned value does *not* borrow from an argument, you can say so with `use<>`. Then callers may keep the result longer:

```rust
/// The result holds a number, not the label, so it captures no lifetime.
fn label_len(label: &str) -> impl Fn() -> usize + use<> {
    let len = label.len();
    move || len
}

fn main() {
    let len;
    {
        let label = String::from("temporary");
        len = label_len(&label);
    } // `label` is dropped here, and `len` is still usable.
    assert_eq!(len(), 9);
}
```

`Box<dyn Fn(..)>` has a different rule. A boxed trait object in a return type means `Box<dyn Fn(..) + 'static>` unless you write a lifetime. `'static` means "holds no borrow that can end". So a boxed closure that borrows from an argument needs `Box<dyn Fn(..) + '_>`.

### Mistakes people make

**Forgetting `move`.**

```rust,compile_fail,E0373
fn every_nth(n: u32) -> impl FnMut() -> bool {
    let mut seen = 0;
    || {
        seen += 1;
        seen % n == 0
    }
}

fn main() {
    let mut third = every_nth(3);
    println!("{} {} {}", third(), third(), third());
}
```

```text
error[E0373]: closure may outlive the current function, but it borrows `seen`, which is owned by the current function
 --> main.rs:3:5
  |
3 |     || {
  |     ^^ may outlive borrowed value `seen`
4 |         seen += 1;
  |         ---- `seen` is borrowed here
  |
help: to force the closure to take ownership of `seen` (and any other referenced variables), use the `move` keyword
```

Fix: write `move || { .. }`. The counter then lives inside the closure, and every call of `every_nth` returns a closure with its own counter.

**Using the closure after its borrowed data is gone.**

```rust,compile_fail,E0597
struct Tariff {
    base: u32,
    per_km: u32,
}

fn fare_for(tariff: &Tariff) -> impl Fn(u32) -> u32 {
    move |km| tariff.base + tariff.per_km * km
}

fn main() {
    let fare;
    {
        let city = Tariff { base: 300, per_km: 120 };
        fare = fare_for(&city);
    }
    println!("{}", fare(5));
}
```

```text
error[E0597]: `city` does not live long enough
  --> main.rs:14:25
   |
13 |         let city = Tariff { base: 300, per_km: 120 };
   |             ---- binding `city` declared here
14 |         fare = fare_for(&city);
   |                         ^^^^^ borrowed value does not live long enough
15 |     }
   |     - `city` dropped here while still borrowed
16 |     println!("{}", fare(5));
   |                    ---- borrow later used here
   |
note: this call may capture more lifetimes than intended, because Rust 2024 has adjusted the `impl Trait` lifetime capture rules
```

Fix: keep `city` alive as long as `fare`, or let the closure own what it needs (copy the two numbers into it with `move`).

### When to use which

| Return type | Choose it when |
| --- | --- |
| `impl Fn(..)` | there is one closure type; this is the default |
| `Box<dyn Fn(..)>` | branches return different closures, or the caller stores many different ones in one collection |
| `Box<dyn Fn(..) + '_>` | as above, and the closure borrows from an argument |

### Read more

- [Lifetime capture rules for returned `impl Trait`](https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html) in the Edition Guide
- [Precise capturing with `use<..>`](https://doc.rust-lang.org/reference/types/impl-trait.html#precise-capturing) in the Rust Reference
- [Default trait object lifetimes](https://doc.rust-lang.org/reference/lifetime-elision.html#default-trait-object-lifetimes) in the Rust Reference
- [Error E0373](https://doc.rust-lang.org/error_codes/E0373.html)

## Lesson 05 · Copy and Clone for captures

Go back to the model from lesson 01: a closure is a struct with one field per capture. Chapter 13 said that a struct can be `Copy` only if all its fields are `Copy`, and `Clone` only if all its fields are `Clone`. For closures the compiler applies the same rule, and it does so automatically. You never write `#[derive(Clone)]` on a closure.

| The closure holds | `Copy`? | `Clone`? |
| --- | --- | --- |
| nothing | yes | yes |
| shared references (`&T`) | yes | yes |
| `Copy` values by `move` (numbers, `bool`, `char`) | yes | yes |
| owned `Clone` values by `move` (`String`, `Vec<T>`) | no | yes: the clone gets its own values |
| a mutable reference (`&mut T`) | no | no |

The last row follows from chapter 5: two copies of a closure that holds `&mut T` would be two mutable references to the same value.

A `Copy` closure can be used again after you pass it by value, like an integer. A `Clone` closure can be duplicated with `.clone()`, and the two copies then live separate lives.

```rust
fn main() {
    // Captures one i32 by value, so the closure is Copy.
    let scale = 3;
    let triple = move |x: i32| x * scale;
    // The array method `map` takes its closure by value ...
    assert_eq!([1, 2].map(triple), [3, 6]);
    // ... and `triple` can still be used, because a copy was passed.
    assert_eq!([5, 6].map(triple), [15, 18]);

    // Owns a String, so the closure is Clone but not Copy.
    let label = String::from("sensor");
    let tag = move |reading: i32| format!("{label}={reading}");
    let spare = tag.clone(); // clones the String inside
    assert_eq!([1, 2].map(tag), ["sensor=1", "sensor=2"]); // `tag` is moved away
    assert_eq!(spare(7), "sensor=7"); // the clone is still here
}
```

```text
tag    { label: String } --> heap "sensor"
spare  { label: String } --> heap "sensor"     a second, separate buffer
```

### State is copied too

A closure that owns a counter and changes it is `FnMut`. If the counter is an integer, the closure is also `Copy`. Copying such a closure copies the counter **as it is at that moment**. After that, each copy counts alone.

```rust
fn run_twice(mut f: impl FnMut() -> i32) -> i32 {
    f();
    f()
}

fn main() {
    let mut count = 0;
    let mut tick = move || {
        count += 1;
        count
    };

    // `tick` is Copy, so this call receives a copy. The copy counts to 2.
    assert_eq!(run_twice(tick), 2);
    // The original has not counted yet.
    assert_eq!(tick(), 1);

    // To let a function advance the original, lend it: `&mut F` is FnMut too.
    assert_eq!(run_twice(&mut tick), 3);
    assert_eq!(tick(), 4);
}
```

This surprises people, because no error and no warning appears. When a function should change your closure's state, pass `&mut closure`.

### Mistakes people make

**Using a closure twice when it is not `Copy`.**

```rust,compile_fail,E0382
fn main() {
    let label = String::from("sensor");
    let tag = move |reading: i32| format!("{label}={reading}");
    let a = tag;
    let b = tag;
    println!("{} {}", a(1), b(2));
}
```

```text
error[E0382]: use of moved value: `tag`
 --> main.rs:5:13
  |
3 |     let tag = move |reading: i32| format!("{label}={reading}");
  |         --- move occurs because `tag` has type `{closure@main.rs:3:15: 3:34}`, which does not implement the `Copy` trait
4 |     let a = tag;
  |             --- value moved here
5 |     let b = tag;
  |             ^^^ value used here after move
  |
help: consider cloning the value if the performance cost is acceptable
  |
4 |     let a = tag.clone();
  |                ++++++++
```

Fix: `tag.clone()` if you need two owners, or `&tag` if a borrow is enough. A shared reference to an `Fn` closure is itself `Fn`.

**Cloning a closure that holds a mutable borrow.**

```rust,compile_fail,E0277
fn main() {
    let mut n = 0;
    let mut inc = || n += 1;
    let mut twin = inc.clone();
    inc();
    twin();
}
```

```text
error[E0277]: the trait bound `&mut i32: Clone` is not satisfied in `{closure@main.rs:3:19: 3:21}`
   --> main.rs:4:24
    |
  3 |     let mut inc = || n += 1;
    |                   -- within this `{closure@main.rs:3:19: 3:21}`
  4 |     let mut twin = inc.clone();
    |                        ^^^^^ within `{closure@main.rs:3:19: 3:21}`, the trait `Clone` is not implemented for `&mut i32`
    |
    = note: `Clone` is implemented for `&i32`, but not for `&mut i32`
```

The message names the hidden field: the closure contains a `&mut i32`. Fix: add `move`, so that each clone owns its own counter. If both must change the same number, share it through `Rc<Cell<i32>>` (chapters 4 and 9).

### When to use which

- Ask for `F: Copy` or `F: Clone` in a bound only when your function really needs several owners of the closure, for example one per worker.
- Otherwise take the closure once and lend it: `&F` for `Fn`, `&mut F` for `FnMut`.

### Read more

- [Other traits implemented by closures](https://doc.rust-lang.org/reference/types/closure.html#other-traits) in the Rust Reference
- [`Clone`](https://doc.rust-lang.org/std/clone/trait.Clone.html) and [`Copy`](https://doc.rust-lang.org/std/marker/trait.Copy.html) in the standard library documentation

## Lesson 06 · Callback registration and invocation

A **callback** is a closure that you hand over now so that other code can call it later. To call it later, that code must store it. Storing closures brings together three ideas from earlier lessons.

1. **Every closure has its own type**, so a `Vec` cannot hold two different closures directly. The elements must be trait objects: `Vec<Box<dyn FnMut(&str)>>`. Each element is a pointer to the closure's data and a pointer to its method table. The closures behind the boxes may have different sizes and captures.
2. **A stored closure outlives the call that registered it.** Inside a struct field, `Box<dyn FnMut(&str)>` means `Box<dyn FnMut(&str) + 'static>`: the closure may not hold a borrow that can end. Callers therefore register `move` closures that own their data.
3. **The call trait decides how you call them.** `dyn Fn` can be called through `&`. `dyn FnMut` needs `&mut`, so the method that runs the callbacks takes `&mut self`. `Box<dyn FnOnce()>` can be called only by value, so you must take each box out of the collection first.

The worked example stores `FnOnce` steps and runs them in reverse order, like the cleanup at the end of a job. The lesson's exercise stores `FnMut` callbacks instead.

```rust
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct Cleanup {
    steps: Vec<Box<dyn FnOnce()>>,
}

impl Cleanup {
    /// Stores a step. Nothing runs yet.
    fn defer(&mut self, step: impl FnOnce() + 'static) {
        self.steps.push(Box::new(step));
    }

    /// Runs the steps, newest first. `pop` gives each box by value,
    /// which is what calling an FnOnce needs.
    fn run(&mut self) {
        while let Some(step) = self.steps.pop() {
            step();
        }
    }
}

fn main() {
    // The steps and `main` share one log. Each step owns one Rc handle to it.
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut cleanup = Cleanup::default();

    let sink = Rc::clone(&log);
    cleanup.defer(move || sink.borrow_mut().push("close file"));
    let sink = Rc::clone(&log);
    cleanup.defer(move || sink.borrow_mut().push("flush buffer"));

    assert!(log.borrow().is_empty()); // registering ran nothing
    cleanup.run();
    assert_eq!(*log.borrow(), ["flush buffer", "close file"]);
}
```

```text
cleanup.steps: Vec --> [ (data, table) , (data, table) ]
                          |      |
                          |      +--> table of closure 1: size, drop, call_once
                          +--> heap: closure 1 { sink: Rc } --+
                                                              +--> the shared log
main: log: Rc ------------------------------------------------+
```

Notice the two lines `let sink = Rc::clone(&log);`. A `move` closure takes whole variables. To give a closure its own handle and keep yours, clone first and move the clone. This "clone, then move" pattern is very common with `Rc` and `Arc`.

A closure's captures are dropped when the closure is dropped. A registry therefore keeps every captured value alive until the registry itself is dropped, or until it removes the callback.

### Mistakes people make

**Registering a closure that borrows a local variable.**

```rust,compile_fail,E0373
#[derive(Default)]
struct Cleanup {
    steps: Vec<Box<dyn FnOnce()>>,
}

impl Cleanup {
    fn defer(&mut self, step: impl FnOnce() + 'static) {
        self.steps.push(Box::new(step));
    }
}

fn main() {
    let mut cleanup = Cleanup::default();
    let mut log = Vec::new();
    cleanup.defer(|| log.push("close file"));
}
```

```text
error[E0373]: closure may outlive the current function, but it borrows `log`, which is owned by the current function
  --> main.rs:15:19
   |
15 |     cleanup.defer(|| log.push("close file"));
   |                   ^^ --- `log` is borrowed here
   |                   |
   |                   may outlive borrowed value `log`
   |
note: function requires argument type to outlive `'static`
  --> main.rs:15:5
   |
15 |     cleanup.defer(|| log.push("close file"));
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

Fix: move owned data into the closure. If the caller must also see the data, share it with `Rc<RefCell<..>>` as in the worked example.

**Calling stored `FnMut` callbacks through a shared reference.**

```rust,compile_fail,E0596
struct Bell {
    listeners: Vec<Box<dyn FnMut(u32)>>,
}

impl Bell {
    fn ring(&self, times: u32) {
        for listener in &self.listeners {
            listener(times);
        }
    }
}

fn main() {}
```

```text
error[E0596]: cannot borrow `*listener` as mutable, as it is behind a `&` reference
 --> main.rs:8:13
  |
7 |         for listener in &self.listeners {
  |                         --------------- this iterator yields `&` references
8 |             listener(times);
  |             ^^^^^^^^ `listener` is a `&` reference, so it cannot be borrowed as mutable
  |
help: use a mutable iterator instead
  |
7 |         for listener in &mut self.listeners {
  |                          +++
```

Fix: take `&mut self` and iterate with `&mut self.listeners` (or `iter_mut()`).

### When to use which

| Design | Callbacks may capture | Trade-off |
| --- | --- | --- |
| `Box<dyn FnMut(&str)>` (means `+ 'static`) | owned data only | simple; the registry can be stored or returned anywhere |
| `Registry<'a>` holding `Box<dyn FnMut(&str) + 'a>` | borrows that live for `'a` | no `Rc` needed; the registry cannot outlive the borrowed data |
| add `+ Send` to the trait object | data that may move to another thread | needed when callbacks run on other threads (chapter 19) |

Store `dyn Fn` when callbacks only read their state, `dyn FnMut` when they may change it, and `dyn FnOnce` when each one runs at most once.

### Read more

- [Trait objects](https://doc.rust-lang.org/reference/types/trait-object.html) in the Rust Reference
- [Default trait object lifetimes](https://doc.rust-lang.org/reference/lifetime-elision.html#default-trait-object-lifetimes) in the Rust Reference
- [`Box<dyn FnOnce>` and the `FnOnce` implementation for `Box`](https://doc.rust-lang.org/std/boxed/struct.Box.html#impl-FnOnce%3CArgs%3E-for-Box%3CF,+A%3E) in the standard library documentation

## Lesson 07 · Capstone: configurable text processing

The capstone has no new language feature. It asks you to write a small function whose **policy** comes from the caller as closures, while the function itself fixes the **order** in which things happen. Many library functions have this shape: `retain`, `sort_by`, `Iterator::filter_map`, and the `retry` function of lesson 03.

How to approach it:

1. **Read the contract as a timeline.** For one row, write down which callback runs first, which runs next, and which callbacks do not run at all when the row is rejected. Then write the timeline for two rows. One test records every callback call in a log and compares the whole log, so the order between rows matters as much as the order inside one row.
2. **Check each bound against the timeline.** A callback that runs once per row runs many times in total, so it cannot be `FnOnce`. Ask yourself why the starter chose `FnMut` and not `Fn`: which callers would `Fn` turn away? A callback that runs exactly once for the whole job can have the weakest bound of all.
3. **Look at what flows between the callbacks.** One callback produces a value, and another one must see that value, not the original input. Decide who owns the value at each step, and lend it where a borrow is enough. You should not need `clone`.
4. **Prefer the plain solution.** A `for` loop with an `if` is a good answer here. Chapter 15 will show how to write the same pipeline with iterator adapters, and why side effects in callbacks make the order of calls matter there too.

A tiny example of "the function fixes the order, the caller supplies the policy", with a different job than the capstone:

```rust
/// Calls `step` for 0, 1, 2, ... until `stop` says yes. `stop` is always
/// asked before `step` runs, and `step` never runs for the stopping value.
fn drive(mut stop: impl FnMut(u32) -> bool, mut step: impl FnMut(u32)) {
    let mut n = 0;
    while !stop(n) {
        step(n);
        n += 1;
    }
}

fn main() {
    let mut seen = Vec::new();
    drive(|n| n == 3, |n| seen.push(n));
    assert_eq!(seen, [0, 1, 2]);
}
```

After the tests pass, do the lab in the lesson header: write a predicate with its own state, then replace one generic parameter with `Box<dyn FnMut(..)>` and note what changes for the caller and for the function.

### Read more

- [`Vec::retain`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.retain) and [`Iterator::filter_map`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.filter_map): two standard functions with caller-supplied policy
