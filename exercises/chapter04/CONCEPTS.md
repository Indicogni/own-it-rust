# Chapter 4 concepts — Ownership and moves

[Chapter guide](README.md) · [Hints](HINTS.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every code block in this file was compiled and
run with the course toolchain (Rust 1.99.0, edition 2024), and every compiler
message was copied from that compiler. Long messages are shortened: a line with
`...` marks a cut in the middle, and the end of a message is often left out.

## What you will be able to do

After this chapter you can:

- say which variable owns any value in a program, and at which line the value
  is destroyed;
- predict whether an assignment, a function call or a `return` moves a value or
  copies it;
- read and repair the "moved value" compiler errors (`E0382` and `E0507`);
- take a value out of a vector, or out of a place behind `&mut`, without
  copying its data;
- choose between moving a value, cloning it, and sharing it with `Rc` or `Arc`.

## Before you start

Chapters 1 to 3: variables and `mut`, functions, `if`
and `for`, tuples, arrays, `Vec<T>`, `String` and `&str`, and `Option<T>` with
its two forms `Some(value)` and `None`. You should be able to
read a parameter such as `text: &str` as "this function looks at some text that
it does not own". The rules for references come in Chapter 5; this chapter does
not need them.

A few examples use a feature from a later chapter, because there is no other
way to make ownership visible. Each time, the text tells you the one thing you
need to know about that feature.

## Time

These are estimates for a learner who is new to Rust. They include
reading the section, the predictions, the code, and your notes.

| Lesson | Estimate |
| --- | --- |
| 01 · Ownership scopes | 40 minutes |
| 02 · Moves across assignments and fields | 45 minutes |
| 03 · Moves and control flow | 50 minutes |
| 04 · Moving from indexed collections | 45 minutes |
| 05 · Leaving a replacement behind | 40 minutes |
| 06 · Copy and explicit cloning | 40 minutes |
| 07 · Rc and Arc | 60 minutes |
| 08 · Capstone | 45 minutes |
| R01 review, C01 code review, drills | 90 minutes |

## Words used in this chapter

- **Owner**: the variable (or the field, or the vector slot) that is
  responsible for a value.
- **Drop**: to destroy a value. Rust runs the value's clean-up code and then
  frees its memory.
- **Heap buffer**: a block of memory that a value asked for while the program
  was running. The text of a `String` and the elements of a `Vec` live in heap
  buffers.
- **Header**: the small fixed-size part of a `String` or `Vec` that lives in
  the variable itself: a pointer to the heap buffer, a capacity and a length.
- **Move**: to transfer ownership of a value from one place to another.

## Lesson 01 · Ownership scopes

### The idea

Every value in a Rust program has exactly one **owner**. The owner is usually a
variable. It can also be a place inside another value: a field of a tuple, or
an element slot of a vector.

The owner is responsible for the value. When the owner goes away, Rust
**drops** the value: it runs the value's clean-up code and frees its memory.
You never call a `free` function, and there is no garbage collector that runs
later. The compiler puts the clean-up at a fixed place in your program.

A variable goes away when execution leaves the block `{ ... }` in which the
variable was declared. That block is the variable's **scope**.

### What a String looks like in memory

A `String` has two parts. The **header** is three machine words that live in
the variable: a pointer, a capacity and a length. The text lives in a **heap
buffer** that the pointer points to. A `Vec<T>` has the same shape.

```rust
fn main() {
    let reading = (String::from("pump-2"), vec![410_u16, 395, 402]);
    assert_eq!(reading.0.len(), 6);
    assert_eq!(reading.1.len(), 3);
} // `reading` goes out of scope here. Both heap buffers are freed.
```

```text
the variable `reading`                     heap

 reading.0   pointer ───────────────────►  [ p | u | m | p | - | 2 ]
             capacity
             length    6
 reading.1   pointer ───────────────────►  [ 410 | 395 | 402 ]
             capacity
             length    3

 owner tree:   reading
               ├── field 0: String ── owns its text buffer
               └── field 1: Vec    ── owns its element buffer
```

Ownership forms a tree. `reading` owns the tuple. The tuple owns a `String` and
a `Vec`. Each of those owns one heap buffer. When the root of a tree is
dropped, everything below it is dropped too.

### Watching a drop happen

To see the moment of a drop, we need a value that prints when it is dropped.
The next example defines one. The `impl Drop` syntax is explained in Chapter
13. For now you need one fact: Rust calls the function `drop` at the moment the
value is destroyed.

```rust
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropping {}", self.0);
    }
}

fn main() {
    let _a = Loud("a");
    {
        let _b = Loud("b");
        println!("end of inner block");
    }
    let _c = Loud("c");
    println!("end of main");
}
```

```text
end of inner block
dropping b
end of main
dropping c
dropping a
```

Read the output slowly. `b` is dropped when the inner block ends, not later.
At the end of `main`, the variables are dropped in the **reverse** of the order
in which they were declared: `c` first, then `a`. The reason for the reverse
order is that a later variable may refer to an earlier one, so the later one
must go first.

### A value can leave its block

The last expression of a block, written without a semicolon, is the value of
the block. Ownership of that value moves out to whoever receives it. Nothing is
dropped on the way.

```rust
let greeting = {
    let mut text = String::from("good");
    text.push_str(" morning");
    text // no semicolon: this String leaves the block
};
assert_eq!(greeting, "good morning");
```

The variable `text` is gone after the block, but the `String` it owned is not.
It has a new owner, `greeting`. A function body is a block too, so returning a
value from a function works in the same way.

### Mistakes people make

**Using a value after its scope has ended.**

```rust,compile_fail,E0425
fn main() {
    {
        let ticket = String::from("A-17");
        println!("inside: {ticket}");
    }
    println!("outside: {ticket}");
}
```

```text
error[E0425]: cannot find value `ticket` in this scope
 --> src/main.rs:6:25
  |
6 |     println!("outside: {ticket}");
  |                         ^^^^^^
  |
help: the binding `ticket` is available in a different scope in the same function
```

Fix: declare the variable in the outer block, or let the inner block return
the value as shown above.

**Thinking that a value is dropped after its last use.** It is dropped at the
end of its scope. In the output above, `end of main` is printed before the two
drops, although `_a` and `_c` are never used. If you need a value to be
destroyed earlier, call `drop(value)`. That function takes ownership of its
argument and does nothing else, so the value is dropped right there.

**Writing `let _ = ...` when you mean `let _name = ...`.** The pattern `_` does
not create a variable. A value that the right-hand side has just created
therefore gets no owner, and it is dropped at the end of that statement. A name
that starts with an underscore is a real variable, and it keeps the value until
the end of its scope.

```rust
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropping {}", self.0);
    }
}

fn main() {
    let _ = Loud("ignored");
    let _kept = Loud("kept");
    println!("end of main");
}
```

```text
dropping ignored
end of main
dropping kept
```

### Read more

- [Destructors, in the Rust Reference](https://doc.rust-lang.org/reference/destructors.html)
- [How a `String` is represented](https://doc.rust-lang.org/std/string/struct.String.html#representation)
- [`std::mem::drop`](https://doc.rust-lang.org/std/mem/fn.drop.html)

## Lesson 02 · Moves across assignments and fields

This section builds on Lesson 01: one owner per value, and the header and
buffer picture of a `String`.

### The idea

For a type such as `String` or `Vec`, these operations **move** the value:

- assigning it: `let b = a;` or `b = a;`
- passing it to a function: `f(a)`
- returning it from a function or a block
- putting it into another value: `v.push(a)`, `(a, 1)`, `[a, b]`

A move does two things. It copies the header (three machine words) into the new
place. And the compiler marks the old place as **having no value**: you may not
read it any more. The heap buffer is not touched. After the move there is
still exactly one owner.

```text
before  let published = draft;            after

draft      pointer ──► [ t i d e ... ]    draft      (no value: it was moved)
           capacity
           length                         published  pointer ──► [ t i d e ... ]
                                                     capacity      the same buffer
                                                     length
```

So a move is cheap, and its cost does not depend on how long the text is.

### Giving a value to a function and getting it back

A function parameter of type `String` takes ownership. If the caller wants the
value back, the function returns it.

```rust
fn sign(mut letter: String, name: &str) -> String {
    letter.push_str("\n-- ");
    letter.push_str(name);
    letter
}

let letter = String::from("See you on Friday.");
let letter = sign(letter, "Mina"); // moved in, changed, moved back out
assert!(letter.ends_with("-- Mina"));
```

`name` is different: its type `&str` means that `sign` only looks at the name,
and the caller keeps it.

### Moving one field

You can move a single field out of a tuple. This is a **partial move**. The
other fields stay where they are and you can still use them one by one. The
tuple as a whole can no longer be used, because a part of it is missing.

```rust
let entry = (String::from("colour"), String::from("teal"));
let key = entry.0; // moves field 0 only
assert_eq!(key, "colour");
assert_eq!(entry.1, "teal"); // field 1 still belongs to `entry`
```

A pattern can take a tuple apart in one step. This moves every field into its
own new variable, so nothing is left half-used.

```rust
let record = (String::from("Lagos"), vec![2019, 2023], 4.5);
let (city, visits, rating) = record;
assert_eq!(city, "Lagos");
assert_eq!(visits.len(), 2);
assert_eq!(rating, 4.5);
```

The pattern `_` means "do not bind this part". It does not move the part. After
`let (city, _, _) = record;` the vector is still inside `record`.

### Mistakes people make

**Using a variable after its value was moved away.**

```rust,compile_fail,E0382
fn shout(text: String) -> String {
    text.to_uppercase()
}

fn main() {
    let name = String::from("ada");
    let loud = shout(name);
    println!("{name} -> {loud}");
}
```

```text
error[E0382]: borrow of moved value: `name`
 --> src/main.rs:8:16
  |
6 |     let name = String::from("ada");
  |         ---- move occurs because `name` has type `String`, which does not implement the `Copy` trait
7 |     let loud = shout(name);
  |                      ---- value moved here
8 |     println!("{name} -> {loud}");
  |                ^^^^ value borrowed here after move
  |
note: consider changing this parameter type in function `shout` to borrow instead if owning the value isn't necessary
...
help: consider cloning the value if the performance cost is acceptable
  |
7 |     let loud = shout(name.clone());
  |                          ++++++++
```

Learn to read the three labels of `E0382`. The first says where the variable
was declared and why assignment moves it. The second says where the value was
moved. The third says where you tried to use the empty variable. The word
"borrow" appears because `println!` only looks at its arguments.

The compiler offers two repairs, and there is a third:

1. Change the function so that it does not take ownership (`text: &str`). This
   is the best repair here, because `shout` only reads the text.
2. Clone the value, so that the function gets its own copy. This costs an
   allocation.
3. Let the function return the value and bind it again, as `sign` does above.

**Using the whole value after a partial move.**

```rust,compile_fail,E0382
fn main() {
    let entry = (String::from("colour"), String::from("teal"));
    let key = entry.0;
    let copy = entry;
    println!("{key} {}", copy.1);
}
```

```text
error[E0382]: use of partially moved value: `entry`
 --> src/main.rs:4:16
  |
3 |     let key = entry.0;
  |               ------- value partially moved here
4 |     let copy = entry;
  |                ^^^^^ value used here after partial move
  |
  = note: partial move occurs because `entry.0` has type `String`, which does not implement the `Copy` trait
```

Fix: take the tuple apart completely with a pattern, or use only the fields
that are still there.

**Believing that a move copies the text.** It does not. Only the header is
copied. The prediction test of this lesson lets you check this with pointers
and with an allocation counter.

### When to use which

| The function ... | Parameter type |
| --- | --- |
| keeps the value, or changes it and returns it | `String` (a move) |
| only reads the value | `&str` (a borrow; Chapter 5 has the rules) |

Call `.clone()` only when two owners really need two separate copies.

### Read more

- [Moved and copied types, in the Rust Reference](https://doc.rust-lang.org/reference/expressions.html#moved-and-copied-types)
- [Error code E0382](https://doc.rust-lang.org/error_codes/E0382.html)
- [The wildcard pattern `_`](https://doc.rust-lang.org/reference/patterns.html#wildcard-pattern)

## Lesson 03 · Moves and control flow

This section builds on Lesson 02: a move leaves the old variable without a
value, and reading such a variable is an error.

### The idea

Programs have branches and loops, so "was this variable moved?" can depend on
the path that execution takes. The compiler does not guess which path will be
taken. It checks **every** path. At each point of a function, each variable is
in one of three states:

- it definitely has a value;
- it definitely has no value (it was moved, or never given one);
- it **may** have no value, because at least one path to this point moved it.

Reading a variable is allowed only in the first state. This check happens when
you compile. The compiler does not run your program and does not evaluate your
conditions.

### A move in one branch

```rust,compile_fail,E0382
fn archive(note: String) {
    println!("archived: {note}");
}

fn main() {
    let note = String::from("call the lab");
    let urgent = std::env::args().count() > 1;
    if urgent {
        archive(note);
    }
    println!("still on my desk: {note}");
}
```

```text
error[E0382]: borrow of moved value: `note`
  --> src/main.rs:11:34
   |
 6 |     let note = String::from("call the lab");
   |         ---- move occurs because `note` has type `String`, which does not implement the `Copy` trait
...
 9 |         archive(note);
   |                 ---- value moved here
10 |     }
11 |     println!("still on my desk: {note}");
   |                                  ^^^^ value borrowed here after move
```

When `urgent` is false, `note` still has its value at line 11. That does not
help: one path reaches line 11 with an empty variable, so the use is rejected.

A moved variable is not dead for ever. You can assign a new value to it, and
then it is usable again. If the branch that moves also assigns, every path
reaches the end of the `if` with a value:

```rust
fn archive(note: String) -> usize {
    note.len()
}

let mut note = String::from("call the lab");
let urgent = true;
if urgent {
    archive(note);
    note = String::from("(desk is clear)"); // `note` has a value again
}
assert!(!note.is_empty());
```

### A move inside a loop

The body of a loop can run many times. A move in the body empties the variable,
and the next round would then use an empty variable.

```rust,compile_fail,E0382
fn main() {
    let ticket = String::from("A-17");
    let mut holders = Vec::new();
    for _ in 0..2 {
        holders.push(ticket);
    }
    println!("{}", holders.len());
}
```

```text
error[E0382]: use of moved value: `ticket`
 --> src/main.rs:5:22
  |
2 |     let ticket = String::from("A-17");
  |         ------ move occurs because `ticket` has type `String`, which does not implement the `Copy` trait
3 |     let mut holders = Vec::new();
4 |     for _ in 0..2 {
  |     ------------- inside of this loop
5 |         holders.push(ticket);
  |                      ^^^^^^ value moved here, in previous iteration of loop
```

A loop body may move a variable if it also gives the variable a value again
before the next round. A common shape is "hand the value to a function, and
take the result back":

```rust
fn add_lap(mut laps: Vec<u32>, seconds: u32) -> Vec<u32> {
    laps.push(seconds);
    laps
}

let mut laps = Vec::new();
for seconds in [61, 59, 64] {
    laps = add_lap(laps, seconds); // moved out, and a value moved back in
}
assert_eq!(laps, [61, 59, 64]);
```

Between the call and the assignment, `laps` has no value. That is fine, because
nothing reads it in that moment. The vector's buffer is the same one all the
time; only the header travels.

### What happens at run time: drop flags

If a variable is moved on some paths only, the program must decide at run time
whether to drop it at the end of its scope. The compiler adds a hidden boolean
for such a variable. It is called a **drop flag**. The example uses the `Loud`
type from Lesson 01.

```rust
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropping {}", self.0);
    }
}

fn send(parcel: Loud) {
    println!("sent {}", parcel.0);
} // `parcel` is dropped here: `send` owned it

fn main() {
    let parcel = Loud("parcel");
    let express = std::env::args().count() > 1;
    if express {
        send(parcel);
    }
    println!("end of main");
}
```

```text
run without arguments        run with one argument

end of main                  sent parcel
dropping parcel              dropping parcel
                             end of main
```

On the left, `main` still owns the parcel at its end and drops it. On the
right, the parcel was moved into `send` and dropped there, and `main` drops
nothing. The value is dropped exactly once on both paths.

### Mistakes people make

**Moving in one branch and using the variable afterwards**, and **moving in a
loop body**: both are shown above. Ask first whether the code needs to move at
all. Often the function should only read the value (Chapter 5). If the move is
right, assign a new value on the path that moved.

**Giving a variable its first value in only one branch.** The same path check
applies to a variable that is declared first and assigned later.

```rust,compile_fail,E0381
fn main() {
    let total;
    let ready = std::env::args().count() > 1;
    if ready {
        total = 10;
    }
    println!("{total}");
}
```

```text
error[E0381]: used binding `total` is possibly-uninitialized
 --> src/main.rs:7:16
  |
2 |     let total;
  |         ----- binding declared here but left uninitialized
3 |     let ready = std::env::args().count() > 1;
4 |     if ready {
  |        ----- if this `if` condition is `false`, `total` is not initialized
5 |         total = 10;
6 |     }
  |      - an `else` arm might be missing here, initializing `total`
7 |     println!("{total}");
  |                ^^^^^ `total` used here but it is possibly-uninitialized
```

Fix: give the variable a value on every path, for example with an `else`
branch, or write `let total = if ready { 10 } else { 0 };`.

### Read more

- [Drop scopes, in the Rust Reference](https://doc.rust-lang.org/reference/destructors.html#drop-scopes)
- [Drop flags, in the Rustonomicon](https://doc.rust-lang.org/nomicon/drop-flags.html)
- [Error code E0381](https://doc.rust-lang.org/error_codes/E0381.html)

## Lesson 04 · Moving from indexed collections

This section builds on Lesson 02 (moves) and on what Chapter 3 said about
`Vec`: a length, a capacity, and elements stored side by side in one buffer.

### The idea

A vector makes a promise: every slot from index `0` to `len - 1` holds a valid
value. The vector relies on this promise when it is dropped, because it then
drops every element.

The expression `v[i]` names a slot inside the vector. For a type that is copied
on assignment, such as `u32`, `let x = v[i];` copies the number out and the
slot keeps its value. For a `String` the same line would have to move the value
out, and the slot would be left empty. That would break the promise, so the
compiler rejects it.

### Ways to get at an element

| You want to ... | Use | Cost | Order of the other elements |
| --- | --- | --- | --- |
| look at element `i` | `&v[i]` or `v.get(i)` | constant | unchanged |
| have your own copy of it | `v[i].clone()` | copies the data | unchanged |
| take the last element | `v.pop()` | constant | unchanged |
| take element `i`, keep the order | `v.remove(i)` | moves every later element | kept |
| take element `i`, order is not important | `v.swap_remove(i)` | constant | changed |
| take every element | `for x in v` | consumes the vector | not applicable |

The methods that take an element out also repair the vector, so the promise
holds again when they return. The difference between `remove` and
`swap_remove` is how they close the gap:

```rust
let mut v = vec![10, 20, 30, 40, 50];
assert_eq!(v.remove(1), 20);
assert_eq!(v, [10, 30, 40, 50]); // 30, 40 and 50 each moved one slot left

let mut v = vec![10, 20, 30, 40, 50];
assert_eq!(v.swap_remove(1), 20);
assert_eq!(v, [10, 50, 30, 40]); // only 50 moved: into the gap
```

```text
remove(1)        [10 | 20 | 30 | 40 | 50]   ->   [10 | 30 | 40 | 50]
                        take   <─── shift left

swap_remove(1)   [10 | 20 | 30 | 40 | 50]   ->   [10 | 50 | 30 | 40]
                        take          └── the last element fills the gap
```

`remove` does work in proportion to the number of elements after index `i`.
`swap_remove` does the same small amount of work for any vector size, but the
last element changes its position.

### A worked example

```rust
let mut tools = vec![
    String::from("saw"),
    String::from("drill"),
    String::from("plane"),
];

let last: Option<String> = tools.pop(); // moves "plane" out
assert_eq!(last.as_deref(), Some("plane"));

let mut lengths = Vec::new();
for tool in tools {
    // each round, `tool` owns one String that was moved out of the vector
    lengths.push(tool.len());
}
assert_eq!(lengths, [3, 5]);
// `tools` cannot be used here: the loop took the whole vector.
```

`pop` returns an `Option` because the vector may be empty. No text was copied
anywhere in this example. Every `String` kept its heap buffer and only changed
its owner.

### An index is checked when the program runs

The compiler does not know how long a vector will be. `v[i]`, `v.remove(i)`
and `v.swap_remove(i)` therefore check the index at run time and **panic** if
it is too large. `v.get(i)` and `v.pop()` return an `Option` and never panic.

```rust,should_panic
fn main() {
    let mut queue = vec![String::from("job")];
    let taken = queue.remove(3); // compiles, then panics
    println!("{taken}");
}
```

```text
thread 'main' (4242) panicked at src/main.rs:3:23:
removal index (is 3) should be < len (is 1)
```

The number in parentheses identifies the thread and is different on every run.
If an index comes from outside your function, compare it with `v.len()` before
you call `remove` or `swap_remove`.

### Mistakes people make

**Moving out through an index.**

```rust,compile_fail,E0507
fn main() {
    let tools = vec![String::from("saw"), String::from("drill")];
    let first = tools[0];
    println!("{first}");
}
```

```text
error[E0507]: cannot move out of index of `Vec<String>`
 --> src/main.rs:3:17
  |
3 |     let first = tools[0];
  |                 ^^^^^^^^ move occurs because value has type `String`, which does not implement the `Copy` trait
  |
help: consider borrowing here
  |
3 |     let first = &tools[0];
  |                 +
help: consider cloning the value if the performance cost is acceptable
  |
3 |     let first = tools[0].clone();
  |                         ++++++++
```

Both suggestions leave the element in the vector. If you want to take it out,
use one of the methods in the table.

**Using a vector after a `for` loop consumed it.**

```rust,compile_fail,E0382
fn main() {
    let tools = vec![String::from("saw"), String::from("drill")];
    for tool in tools {
        println!("{tool}");
    }
    println!("{}", tools.len());
}
```

```text
error[E0382]: borrow of moved value: `tools`
   --> src/main.rs:6:20
    |
...
  3 |     for tool in tools {
    |                 ----- `tools` moved due to this implicit call to `.into_iter()`
...
  6 |     println!("{}", tools.len());
    |                    ^^^^^ value borrowed here after move
...
help: consider iterating over a slice of the `Vec<String>`'s content to avoid moving into the `for` loop
    |
  3 |     for tool in &tools {
    |                 +
```

`for x in v` moves the vector into the loop. `for x in &v` only looks at the
elements, and `v` is still usable afterwards.

### Read more

- [`Vec::remove`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.remove),
  [`Vec::swap_remove`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.swap_remove)
  and [`Vec::pop`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.pop)
- [Error code E0507](https://doc.rust-lang.org/error_codes/E0507.html)

## Lesson 05 · Leaving a replacement behind

This section builds on Lesson 04. There, a vector slot could not be left empty.
Here the same rule appears for any place that you reach through `&mut`.

### The idea

A parameter of type `&mut String` lets a function change a `String` that
someone else owns. The owner will use the value again after the call, and will
drop it one day. So the place behind a `&mut` must hold a valid value whenever
anyone can look at it. You cannot move the value out and leave nothing.

You **can** exchange it. If you put a new value in and take the old value out
in one step, the place is never empty. The standard library has small functions
for this:

| Function | What it leaves in the place | What it returns |
| --- | --- | --- |
| `std::mem::replace(place, new)` | `new` | the old value |
| `std::mem::take(place)` | the type's default value | the old value |
| `std::mem::swap(a, b)` | the value from the other place | nothing |
| `option.take()` | `None` | the old `Option` |
| `option.replace(new)` | `Some(new)` | the old `Option` |

The **default value** of a type is what `Default::default()` returns for it: an
empty vector for `Vec`, an empty string for `String`, zero for the number
types.

None of these functions copies heap data. They move headers.

### Worked examples

A function that empties a list of pending commands and gives them to its
caller:

```rust
fn flush(pending: &mut Vec<String>) -> Vec<String> {
    std::mem::take(pending)
}

let mut pending = vec![String::from("save"), String::from("print")];
let batch = flush(&mut pending);
assert_eq!(batch, ["save", "print"]);
assert!(pending.is_empty()); // the caller still has a valid vector
pending.push(String::from("close")); // and can go on using it
```

```text
before the call                        after  let batch = flush(&mut pending);

pending ──► [ save | print ]           pending ──► (an empty vector)
                                       batch   ──► [ save | print ]
                                                   the buffer that `pending` had
```

Putting a chosen value in, instead of the default:

```rust
let mut scores = vec![7, 9, 4];
let last_round = std::mem::replace(&mut scores, vec![0, 0, 0]);
assert_eq!(last_round, [7, 9, 4]);
assert_eq!(scores, [0, 0, 0]);
```

A place that may be empty is an `Option`. Its `take` method moves the content
out and leaves `None`:

```rust
let mut seat: Option<&str> = Some("Ana");
let who = seat.take();
assert_eq!(who, Some("Ana"));
assert_eq!(seat, None);
```

### Mistakes people make

**Moving out through a `&mut`.**

```rust,compile_fail,E0507
fn steal(slot: &mut String) -> String {
    let old = *slot;
    old
}
```

```text
error[E0507]: cannot move out of `*slot` which is behind a mutable reference
 --> src/main.rs:2:15
  |
2 |     let old = *slot;
  |               ^^^^^ move occurs because `*slot` has type `String`, which does not implement the `Copy` trait
```

The rest of the message offers two changes. Removing the `*` gives you the
reference and not the `String`, so the function can no longer return a
`String`. Writing `slot.clone()` compiles, but it copies the text and leaves
the old value in the place. If you want the old value itself, use `replace` or
`take`.

**Cloning and then overwriting.** `let old = place.clone(); *place = new;`
gives the right result with one useless allocation: it copies the old text, and
then the assignment drops the original. One call to `std::mem::replace` does
the same job with no allocation. The tests of this lesson count allocations, so
they see the difference.

**Calling `take` on a type that has no default value.**

```rust,compile_fail,E0277
struct Meter {
    id: u32,
}

fn reset(slot: &mut Meter) -> Meter {
    std::mem::take(slot)
}
```

```text
error[E0277]: the trait bound `Meter: Default` is not satisfied
   --> src/main.rs:6:20
    |
  6 |     std::mem::take(slot)
    |     -------------- ^^^^ the trait `Default` is not implemented for `Meter`
    |     |
    |     required by a bound introduced by this call
...
help: consider annotating `Meter` with `#[derive(Default)]`
```

Fix: use `std::mem::replace(slot, Meter { id: 0 })` with a value that you
choose, or give the type a default (Chapter 9 shows `#[derive(Default)]`).

### When to use which

- The type has a cheap default and "empty" is a sensible state: `mem::take`.
- You have a specific new value: `mem::replace`.
- Two places should exchange their values: `mem::swap`.
- "Nothing there" is a real state of your program: store an `Option<T>` and
  call `take`.

### Read more

- [`std::mem::replace`](https://doc.rust-lang.org/std/mem/fn.replace.html),
  [`std::mem::take`](https://doc.rust-lang.org/std/mem/fn.take.html) and
  [`std::mem::swap`](https://doc.rust-lang.org/std/mem/fn.swap.html)
- [`Option::take`](https://doc.rust-lang.org/std/option/enum.Option.html#method.take)

## Lesson 06 · Copy and explicit cloning

This section builds on Lesson 02. There, assignment moved. Here you meet the
types for which assignment copies, and the explicit way to copy everything
else.

### The idea

Some values are only a few bytes with no heap buffer behind them: an `i32`, an
`f64`, a `bool`, a `char`. Copying those bytes gives a second value that is
complete and independent. Nothing needs to be cleaned up for either of them.
Such types are **`Copy`** types. For a `Copy` type, assignment and argument
passing copy the value, and the source stays usable.

`Copy` is a **trait**: a named ability that a type can have. Traits are the
subject of Chapter 11. Here you only need to know which types have this one.

| `Copy` | Not `Copy` |
| --- | --- |
| integers, floats, `bool`, `char` | `String`, `Vec<T>`, `Box<T>` |
| shared references `&T` | exclusive references `&mut T` |
| a tuple or array whose elements are all `Copy` | a tuple or array with any non-`Copy` element |
| your own type, if you ask for it and all fields are `Copy` | any type that has clean-up code (`Drop`) |

So `[u8; 4]` and `(i32, char)` are copied, while `[String; 2]` and
`(i32, String)` are moved.

```rust
let a = (3, 'x');
let b = a; // a copy: both tuples can be used
assert_eq!(a, b);
```

### Asking for Copy on your own type

A type that you define is not `Copy` unless you ask. You ask with a `derive`
attribute, which tells the compiler to write the implementation for you.
`Copy` needs `Clone`, so you always derive both.

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
struct Celsius(f64);

let noon = Celsius(21.5);
let logged = noon; // a copy
assert_eq!(noon, logged); // `noon` is still usable
```

Why must you ask? `Copy` is a promise to everyone who uses your type. If you
later add a `String` field, the type can no longer be `Copy`, and all the code
that relied on copying stops compiling. Rust makes you take that decision on
purpose.

### Clone: a copy that you can see

For the other types, a duplicate is made only when you write `.clone()`. Rust
never calls it for you. A clone of a `String` allocates a new heap buffer and
copies the bytes. A clone of a vector allocates a new element buffer and then
clones every element.

```rust
let original = vec![String::from("draft")];
let mut copy = original.clone();
copy[0].push_str(" v2");

assert_eq!(original[0], "draft"); // the original did not change
assert_ne!(original[0].as_ptr(), copy[0].as_ptr()); // two separate text buffers
```

```text
original ──► [ header ] ──► [ d r a f t ]
copy     ──► [ header ] ──► [ d r a f t _ v 2 ]

two owner trees with nothing in common
```

Because `.clone()` is always written out, you can find the possibly expensive
copies in a program by searching for that word.

### Mistakes people make

**Asking for `Copy` when a field is not `Copy`.**

```rust,compile_fail,E0204
#[derive(Clone, Copy)]
struct Sample {
    id: u32,
    bytes: Vec<u8>,
}
```

```text
error[E0204]: the trait `Copy` cannot be implemented for this type
 --> src/main.rs:2:8
  |
1 | #[derive(Clone, Copy)]
  |                 ---- in this derive macro expansion
2 | struct Sample {
  |        ^^^^^^
3 |     id: u32,
4 |     bytes: Vec<u8>,
  |     -------------- this field does not implement `Copy`
```

Fix: derive only `Clone`. Then a duplicate needs an explicit `.clone()`.

**Asking for `Copy` on a type with clean-up code.**

```rust,compile_fail,E0184
#[derive(Clone, Copy)]
struct Handle(u32);

impl Drop for Handle {
    fn drop(&mut self) {
        println!("closing {}", self.0);
    }
}
```

```text
error[E0184]: the trait `Copy` cannot be implemented for this type; the type has a destructor
 --> src/main.rs:2:8
  |
1 | #[derive(Clone, Copy)]
  |                 ---- in this derive macro expansion
2 | struct Handle(u32);
  |        ^^^^^^ `Copy` not allowed on types with destructors
```

Think about what a copy would mean here. Two values would hold the same handle
number, and each would run the clean-up when it is dropped. The same thing
would be closed twice. A `String` has the same problem: two copies of the
header would both free one buffer.

**Expecting an array to be copied because arrays "are values".** An array is
`Copy` only if its element type is.

```rust,compile_fail,E0382
fn main() {
    let pair = [String::from("up"), String::from("down")];
    let other = pair;
    println!("{} {}", pair[0], other[1]);
}
```

```text
error[E0382]: borrow of moved value: `pair`
 --> src/main.rs:4:23
  |
2 |     let pair = [String::from("up"), String::from("down")];
  |         ---- move occurs because `pair` has type `[String; 2]`, which does not implement the `Copy` trait
3 |     let other = pair;
  |                 ---- value moved here
4 |     println!("{} {}", pair[0], other[1]);
  |                       ^^^^^^^ value borrowed here after move
```

**Adding `.clone()` only to make an error go away.** The compiler often
suggests a clone, and it adds "if the performance cost is acceptable". Ask
first whether a move or a borrow would do.

### When to use which

- Derive `Copy` for small types made only of numbers, booleans and characters,
  such as an identifier, a coordinate or a temperature, when you are sure that
  the type will never own heap data.
- Derive only `Clone` for everything else that should be duplicable.
- Call `.clone()` where two owners need independent data. Otherwise move the
  value or lend it.

### Read more

- [`Copy`, with the section "When can my type be `Copy`?"](https://doc.rust-lang.org/std/marker/trait.Copy.html)
- [`Clone`](https://doc.rust-lang.org/std/clone/trait.Clone.html)
- [Error code E0204](https://doc.rust-lang.org/error_codes/E0204.html) and
  [error code E0184](https://doc.rust-lang.org/error_codes/E0184.html)

## Lesson 07 · Rc and Arc

This section builds on Lessons 01 and 06: one owner per value, and the
difference between copying a pointer and cloning data.

### The idea

Sometimes several parts of a program need the same value, and none of them is
the natural single owner. For this case the standard library has pointer types
with **shared ownership**.

`Rc<T>` means "reference counted". `Rc::new(value)` moves the value into a new
heap block, next to a counter that starts at 1. The counter is the **strong
count**: the number of `Rc` pointers that own this block.

- `Rc::clone(&a)` makes another pointer to the same block and adds 1 to the
  count. It does not copy the value.
- Dropping an `Rc` subtracts 1.
- When the count reaches 0, the value is dropped.

```rust
use std::rc::Rc;

let recipe = Rc::new(String::from("2 eggs, 1 cup of flour"));
assert_eq!(Rc::strong_count(&recipe), 1);
{
    let kitchen = Rc::clone(&recipe);
    let hall = Rc::clone(&recipe);
    assert_eq!(Rc::strong_count(&recipe), 3);
    assert!(Rc::ptr_eq(&kitchen, &hall)); // both point at one block
    assert_eq!(kitchen.len(), hall.len()); // read through any of the owners
} // `hall` and `kitchen` are dropped here
assert_eq!(Rc::strong_count(&recipe), 1);
```

```text
inside the block:

recipe  ──┐
kitchen ──┼──► [ strong count: 3 | String header ] ──► [ 2   e g g s ... ]
hall    ──┘         one heap block                      one text buffer
```

The text exists once. Three pointers own it together. `Rc::ptr_eq` tells you
whether two `Rc` values point at the same block. (The block holds a second
counter, for the non-owning `Weak` pointers of Chapter 9. The sketch leaves it
out.)

You can write `recipe.clone()` instead of `Rc::clone(&recipe)`; it does the
same thing. Many Rust programmers prefer the long form, because a reader sees
at once that only a pointer is cloned. `(*recipe).clone()` is different: `*recipe` is
the `String` inside, so this clones the text.

### Shared means read-only

An `Rc<T>` gives you `&T`: you can read the value and call methods that only
read. You cannot change the value, because the other owners may be looking at
it. Chapter 9 shows `RefCell`, which makes controlled changes possible.

Rust also looks through the pointer when a function wants a plain reference.
Where a `&String` or a `&str` is expected, you can pass a reference to the
`Rc`:

```rust
use std::rc::Rc;

fn shout(text: &str) -> String {
    text.to_uppercase()
}

let motto = Rc::new(String::from("measure twice"));
assert_eq!(shout(&motto), "MEASURE TWICE"); // `&motto` is accepted as a `&str`
assert_eq!(Rc::strong_count(&motto), 1); // looking does not add an owner
```

### Arc: the same thing for threads

A **thread** is a second line of execution inside your program. Chapter 19 is
about threads. Here you need three facts:

- `std::thread::spawn(f)` starts a new thread that runs `f`, and returns a
  handle.
- `handle.join()` waits until that thread has finished. It returns a `Result`
  that holds the value `f` returned, or an error if the thread panicked.
- `move || expression` is a **closure**: a small function without a name,
  written in place (Chapter 14). The word `move` means that the closure takes
  ownership of the variables it uses.

A thread started with `spawn` may live longer than the function that started
it. So it cannot borrow that function's variables; it must own what it uses.
To give one value to two threads, each thread needs its own owning pointer.

`Rc` is not allowed to cross into another thread. Its counter is a plain
number, and two threads that change it at the same moment could lose an
update. `Arc<T>` ("atomically reference counted") has the same methods as `Rc`
but changes its counter with atomic operations, which are safe between
threads. Those operations are a little slower, so `Rc` is still the right
choice inside one thread.

```rust
use std::sync::Arc;
use std::thread;

let prices = Arc::new(vec![3_u64, 5, 8]);
let for_worker = Arc::clone(&prices); // a second owner, made for the thread

let worker = thread::spawn(move || for_worker.iter().sum::<u64>());
let total = worker.join().expect("the worker thread panicked");

assert_eq!(total, 16);
assert_eq!(prices.len(), 3); // the first owner still works in this thread
assert_eq!(Arc::strong_count(&prices), 1); // the worker's Arc is gone
```

The closure took ownership of `for_worker`. When the worker finished, its
`Arc` was dropped and the count went back to 1.

### Mistakes people make

**Sending an `Rc` to a thread.**

```rust,compile_fail,E0277
use std::rc::Rc;
use std::thread;

fn main() {
    let prices = Rc::new(vec![3u64, 5, 8]);
    let for_worker = Rc::clone(&prices);
    let total = thread::spawn(move || for_worker.iter().sum::<u64>()).join().unwrap();
    println!("{total} {}", prices.len());
}
```

```text
error[E0277]: `Rc<Vec<u64>>` cannot be sent between threads safely
   --> src/main.rs:7:31
...
    = help: within `{closure@src/main.rs:7:31: 7:38}`, the trait `Send` is not implemented for `Rc<Vec<u64>>`
...
note: required by a bound in `spawn`
```

`Send` is the ability "may be moved to another thread". `Rc` does not have it;
`Arc` does. Fix: use `Arc`.

**Moving the only `Arc` into the thread and then using it.**

```rust,compile_fail,E0382
use std::sync::Arc;
use std::thread;

fn main() {
    let prices = Arc::new(vec![3u64, 5, 8]);
    let worker = thread::spawn(move || prices.iter().sum::<u64>());
    println!("{} prices, total {}", prices.len(), worker.join().unwrap());
}
```

```text
error[E0382]: the type `Arc` does not implement `Copy`
    --> src/main.rs:7:37
     |
   5 |     let prices = Arc::new(vec![3u64, 5, 8]);
     |         ------ this move could be avoided by cloning the original `Arc`, which is inexpensive
   6 |     let worker = thread::spawn(move || prices.iter().sum::<u64>());
     |                                ------- ------ variable moved due to use in closure
     |                                |
     |                                value moved into closure here
   7 |     println!("{} prices, total {}", prices.len(), worker.join().unwrap());
     |                                     ^^^^^^ value borrowed here after move
     |
     = note: consider using `Arc::clone`
```

An `Arc` is moved like any other non-`Copy` value. Fix: clone the `Arc` before
the closure, under a new name, and let the closure take the clone.

**Trying to change the value through an `Rc`.**

```rust,compile_fail,E0596
use std::rc::Rc;

fn main() {
    let prices = Rc::new(vec![3u64, 5, 8]);
    prices.push(13);
}
```

```text
error[E0596]: cannot borrow data in an `Rc` as mutable
 --> src/main.rs:5:5
  |
5 |     prices.push(13);
  |     ^^^^^^ cannot borrow as mutable
```

**Building a cycle.** If value A owns B through an `Rc`, and B owns A through
another `Rc`, each keeps the other's count above 0. Neither is ever dropped,
even when the rest of the program has forgotten both. This is a memory leak,
and the compiler does not prevent it. To build such a cycle you must change a
value after it has been shared, which needs `RefCell`. So you will write one,
and repair it with the non-owning pointer `Weak`, in Chapter 9.

### When to use which

| Situation | Use |
| --- | --- |
| one clear owner | a plain value (this is the normal case) |
| several owners in one thread | `Rc<T>` |
| several owners in different threads | `Arc<T>` |
| shared and also changed | `Rc<RefCell<T>>` (Chapter 9) or `Arc<Mutex<T>>` (Chapter 19) |

Do not start with `Rc`. First ask whether one part of the program can own the
value and lend it to the others.

### Read more

- [The `std::rc` module](https://doc.rust-lang.org/std/rc/index.html)
- [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)
- [`std::thread::spawn`](https://doc.rust-lang.org/std/thread/fn.spawn.html)
- [`Send`](https://doc.rust-lang.org/std/marker/trait.Send.html)

## Lesson 08 · Capstone: owned dispatch queue

The capstone has no new rule. It asks you to choose, for each piece of data,
among the tools of Lessons 01 to 07. This section gives you a way to choose. It
does not give you the code.

### Three questions for every piece of data

1. **How many parts of the program need it at the same time?** One: it has a
   single owner. Several: go on to question 2.
2. **Does anyone change it after it was created?** If each user changes its
   own version, each user needs its own value. If nobody changes it, the users
   can share one.
3. **Does the code that gives it away need it afterwards?** If not, move it.
   A move costs nothing and makes a copy unnecessary.

| Answers | Tool |
| --- | --- |
| one user at a time, and the giver is finished with it | move |
| each user changes its own version, and the giver keeps the original | clone |
| many users, nobody changes it, one thread | `Rc` |
| many users, nobody changes it, several threads | `Arc` |

An example from another area: a photo program prepares a batch of prints. Each
print has a caption that the user can edit, so each print owns its caption. All
prints use the same paper settings, which nobody edits during the batch, so one
copy of the settings is shared by all prints.

### Before you write code

Draw the picture first, as in the sketches of this chapter. For the capstone,
write down:

- every heap allocation that exists after `dispatch` returns, and its owner or
  owners;
- the strong count you expect when there are `n` jobs, and when there are 0;
- what `finish` receives, what it gives back, and which count changes when it
  returns.

Then check each line of your plan against the tests. They compare pointers, to
see that a value was moved and not rebuilt. They count allocations, to see
that nothing was cloned. A solution that produces the right text with the wrong
ownership fails them.

### After the tests pass

Go back to the three questions and say the answers aloud for the labels and
for the configuration. If you can explain why the two get different tools, you
have the main idea of this chapter.
