# Chapter 16 concepts — Standard collections

[Chapter guide](README.md) · [Hints](HINTS.md)

Read one section, then do its lesson. Every code block on this page is compiled and run by the course checks, so you can paste any of them into [the scratch example](../../examples/ch16_scratch.rs) and change it.

## What you will be able to do

A *collection* is a value that owns a changing number of other values. The standard library has eight general collections. This chapter uses seven of them and leaves out `LinkedList`, which is rarely the right choice.

```text
sequence    Vec<T>          growable array; fast at the back, fast by index
            VecDeque<T>     ring buffer; fast at both ends
            BinaryHeap<T>   always gives you the greatest element first
by key      HashMap<K, V>   finds a key by its hash; no order
            BTreeMap<K, V>  keeps keys sorted; range queries
membership  HashSet<T>      a HashMap with keys only
            BTreeSet<T>     a BTreeMap with keys only
```

After this chapter you will be able to:

- choose a collection from the operations that a task needs, and say what each operation costs;
- say which operations keep the order of elements, and which may move the elements to a new place in memory;
- explain why the compiler rejects a change to a collection while a reference into it is alive, and repair such code;
- sort, search and remove duplicates without building a second collection;
- update a map with one lookup, and produce output that does not depend on hash order;
- keep `Eq` and `Hash` in agreement, and plug your own hash function into a map.

## Before you start

Ownership and moves (Chapter 4), references and borrowing (Chapter 5), `Option` and patterns (Chapters 7 and 10), traits such as `Ord`, `Eq`, `Hash`, `Default` and `Borrow` (Chapters 11 and 13), closures (Chapter 14) and iterators (Chapter 15).

## Time

These are estimates for reading the section, doing the lesson, and writing your notes.

| Lesson | Estimate |
| --- | --- |
| 01 Vector access and editing | 40 min |
| 02 Growth, removal, borrow validity | 50 min |
| 03 Sorting and searching | 50 min |
| 04 Queues and ring buffers | 35 min |
| 05 Priority queues | 55 min |
| 06 Maps and entries | 50 min |
| 07 Sets | 45 min |
| 08 Hashing | 45 min |
| 09 Capstone | 2 h |
| R01, C01, and the drills | 2 h 30 min |
| Optional lab: random elements | 30 min |

## Cost words

"O(1)" means that the cost does not grow with the number of elements `n`. "O(log n)" grows very slowly: about 20 steps for a million elements. "O(n)" touches every element. "Amortized" means averaged over many calls: one call may be slow, but a long series is cheap.

## Lesson 01 · Vector access and structural editing

**The model.** A `Vec<T>` is three numbers on the stack and one buffer on the heap. You met this picture in Chapter 4; now the two counters matter.

```text
let mut v = Vec::with_capacity(4);  v.push(7);  v.push(9);

v: pointer  ──────►  [ 7 | 9 | ? | ? ]     heap buffer
   capacity 4          ▲       ▲
   length   2          │       └ room for two more elements, not initialized
                       └ the elements: v[0], v[1]
```

*Length* is the number of elements that exist. *Capacity* is the number of elements that fit in the buffer. Capacity is never smaller than length. When a `push` finds the buffer full, the vector allocates a larger buffer, moves every element into it, and frees the old one. The vector grows in large steps and not by one slot at a time, so a long series of pushes is cheap: `push` is amortized O(1).

`with_capacity(n)` and `reserve(n)` change only the capacity; they add no elements. `with_capacity(n)` promises room for *at least* `n` elements. `reserve(n)` promises room for at least `n` *more* elements than the vector holds now. Both may give more than you asked for, so do not write code or tests that expect an exact capacity. Removing elements never makes the buffer smaller; only `shrink_to_fit` and `shrink_to` give memory back.

A *slice*, `&[T]` or `&mut [T]`, is a pointer and a length: a view of some elements that another value owns. A slice cannot grow or shrink, so every method that changes the length belongs to `Vec`, and every method that only reads or rearranges elements belongs to slices. A `Vec` gives you all slice methods through automatic dereferencing.

```text
changes the length (Vec only)                  keeps the length (any slice)
push, pop            back, O(1)                 swap(i, j), reverse()
insert(i, x)         shifts the tail, O(n)      fill(x)
remove(i)            shifts the tail, O(n)      sort(), rotate_left(k)
swap_remove(i)       last fills the hole, O(1)
truncate(n), clear   drops the tail
split_off(i)         tail moves to a new Vec
extend(iter)         appends many
```

Reading has two forms. `v[i]` panics when `i` is out of range. `v.get(i)` returns an `Option`. Both forms accept a range and then give a slice: `&v[1..3]`, `v.get(1..3)`. A range must lie completely inside the vector: if any part of it is past the end, `v[..]` panics and `get` returns `None`. There are also `first()`, `last()`, and the splitting views `split_first()` and `split_last()`.

**Worked example.** A buffer of sensor readings is flushed in batches. `split_off` moves the newer part into its own vector; the older part stays where it is.

```rust
let mut readings = Vec::with_capacity(8);
readings.extend([21, 22, 22, 25, 24]);
assert_eq!(readings.len(), 5);
assert!(readings.capacity() >= 8); // "at least", never "exactly"

let place = readings.as_ptr();
let newer = readings.split_off(3);
assert_eq!(readings, [21, 22, 22]);
assert_eq!(newer, [25, 24]);
assert_eq!(readings.as_ptr(), place); // the older part did not move

// Slice methods do not change the length.
readings.reverse();
readings.swap(0, 2);
assert_eq!(readings, [21, 22, 22]);
assert_eq!(readings.first(), Some(&21));
assert_eq!(readings.get(5), None);
```

Slices can also be viewed in pieces, and pieces can be glued together. You do not need these in the exercises, but you will meet them in other people's code:

```rust
let levels = [3, 5, 4, 8];
// Every pair of neighbours: how much did the level change?
let steps: Vec<i32> = levels.windows(2).map(|pair| pair[1] - pair[0]).collect();
assert_eq!(steps, [2, -1, 4]);
// Pieces of two; the last piece may be shorter.
let sums: Vec<i32> = levels.chunks(2).map(|piece| piece.iter().sum()).collect();
assert_eq!(sums, [8, 12]);

assert_eq!([[1, 2], [3, 4]].concat(), [1, 2, 3, 4]);
assert_eq!(["usr", "local", "bin"].join("/"), "usr/local/bin");
```

**Mistakes people make.**

Reserving room and then indexing into it. Capacity is not length: no element exists yet.

```rust,should_panic
let mut v: Vec<i32> = Vec::with_capacity(3);
v[0] = 7;
```

```text
index out of bounds: the len is 0 but the index is 0
```

Use `push`, or create the elements first with `vec![0; 3]`.

Moving an element out with an index. Indexing gives access to a place inside the vector; it cannot leave a hole there.

```rust,compile_fail,E0507
fn main() {
    let names = vec![String::from("ana"), String::from("ben")];
    let first = names[0];
    println!("{first}");
}
```

```text
error[E0507]: cannot move out of index of `Vec<String>`
3 |     let first = names[0];
  |                 ^^^^^^^^ move occurs because value has type `String`, which does not implement the `Copy` trait
help: consider borrowing here
3 |     let first = &names[0];
  |                 +
```

Borrow (`&names[0]`), clone, or take the element out with a method that repairs the vector: `remove`, `swap_remove`, `pop`.

Using `remove` in a loop when the order does not matter. Each `remove(i)` moves every later element one place down. If order is not part of the contract, `swap_remove(i)` does the same job in constant time.

**When to use which.** Index with `v[i]` when an out-of-range index would be a bug in your own code; use `get` when the index comes from outside. Use `remove` to keep order, `swap_remove` when order is free, `truncate` or `split_off` for a whole tail.

**Read more.** [`Vec`](https://doc.rust-lang.org/std/vec/struct.Vec.html) · [Capacity and reallocation](https://doc.rust-lang.org/std/vec/struct.Vec.html#capacity-and-reallocation) · [the slice type](https://doc.rust-lang.org/std/primitive.slice.html) · [`std::collections`: when to use which collection](https://doc.rust-lang.org/std/collections/index.html#when-should-you-use-which-collection)

## Lesson 02 · Growth, removal, and borrow validity

**The model.** A reference to an element, `&v[0]`, is a pointer into the vector's heap buffer. Lesson 01 showed that growing a vector may move all elements to a new buffer and free the old one. A reference that was taken before the move would then point into freed memory.

Rust prevents this without looking at capacities. The rule from Chapter 5 is enough: while a shared reference into a value is alive, nothing may change that value. `push`, `insert`, `remove`, `clear` and all other editing methods take `&mut self`. The compiler reads only these signatures. It does not ask whether *this* push will reallocate, so reserved capacity does not help. A removal is rejected by the same rule, and for a good reason too: it moves elements, so an old reference could point at a different element.

**Worked example.** The newest log entry is borrowed, then the log is changed, then the borrow is used.

```rust,compile_fail,E0502
fn main() {
    let mut log = vec![String::from("boot"), String::from("login")];
    let newest = log.last().unwrap();
    log.insert(0, String::from("power on"));
    println!("{newest}");
}
```

```text
error[E0502]: cannot borrow `log` as mutable because it is also borrowed as immutable
3 |     let newest = log.last().unwrap();
  |                  --- immutable borrow occurs here
4 |     log.insert(0, String::from("power on"));
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs here
5 |     println!("{newest}");
  |                ------ immutable borrow later used here
```

There are three repairs. Each one is right in some situation.

```rust
let mut log = vec![String::from("boot"), String::from("login")];

// 1. Finish using the reference before the change.
let newest = log.last().unwrap();
assert_eq!(newest, "login");
log.insert(0, String::from("power on"));

// 2. Copy or clone what you need; the copy does not borrow the vector.
let newest_len = log.last().unwrap().len();
log.push(String::from("logout"));
assert_eq!(newest_len, 5);

// 3. Remember a position, and borrow again after the change.
let position = log.len() - 1;
log.push(String::from("shutdown"));
assert_eq!(log[position], "logout");
```

An index is only a number. It borrows nothing, so it stays usable. It is your job to know that it still means the same element.

**Removing in place.** These methods remove elements inside the existing buffer. They do not allocate; only collecting what `drain` hands out does:

```rust
let mut v = vec![4, -1, -2, 5, 5, 9];
v.retain(|&x| x > 0);                           // keep what passes; order stays
assert_eq!(v, [4, 5, 5, 9]);
v.dedup();                                      // drop repeats that are neighbours
assert_eq!(v, [4, 5, 9]);
let middle: Vec<i32> = v.drain(1..2).collect(); // take a range out, close the gap
assert_eq!((middle, v), (vec![5], vec![4, 9]));
```

**Two mutable references into one slice.** `&mut v[0]` borrows the whole vector mutably, because indexing is a method call on the whole vector. A second `&mut v[1]` is therefore a second mutable borrow of the same value, and the compiler does not reason about the values of indexes.

```rust,compile_fail,E0499
fn main() {
    let mut rows = vec![vec![1, 2], vec![3]];
    let target = &mut rows[0];
    let source = &mut rows[1];
    target.append(source);
    println!("{rows:?}");
}
```

```text
error[E0499]: cannot borrow `rows` as mutable more than once at a time
3 |     let target = &mut rows[0];
  |                       ---- first mutable borrow occurs here
4 |     let source = &mut rows[1];
  |                       ^^^^ second mutable borrow occurs here
5 |     target.append(source);
  |     ------ first borrow later used here
  = help: use `.split_at_mut(position)` to obtain two mutable non-overlapping sub-slices
```

`split_at_mut(mid)` consumes one `&mut [T]` and returns two: the elements before `mid` and the elements from `mid` on. The two parts cannot overlap, and the compiler knows it from the signature alone.

```text
rows.split_at_mut(1)

 [ [1, 2] | [3] ]
   └─left─┘ └right┘      left[0] is rows[0], right[0] is rows[1]
```

```rust
let mut rows = vec![vec![1, 2], vec![3]];
let (left, right) = rows.split_at_mut(1);
left[0].append(&mut right[0]);
assert_eq!(rows, [vec![1, 2, 3], vec![]]);
```

When you need elements at positions that are known only at run time, `get_disjoint_mut` does the splitting for you. It checks the indexes while the program runs and returns an error for an index that is out of range or used twice.

```rust
let mut scores = [10, 20, 30];
if let Ok([low, high]) = scores.get_disjoint_mut([0, 2]) {
    *low += *high;
}
assert_eq!(scores, [40, 20, 30]);
assert!(scores.get_disjoint_mut([1, 1]).is_err());
```

**Mistakes people make.** Besides the two compile errors above, one mistake compiles and fails at run time: removing while walking the indexes of the old length.

```rust,should_panic
let mut v = vec![4, -1, -2, 5];
for i in 0..v.len() {
    if v[i] < 0 {
        v.remove(i);
    }
}
```

```text
index out of bounds: the len is 3 but the index is 3
```

The range `0..4` was fixed before the loop, and the vector became shorter. Before the panic the loop also skipped `-2`, which slid into the place that had just been checked. `retain` has neither problem.

**When to use which.** `retain` for "keep what passes a test"; `drain(range)` when you want the removed elements; `swap_remove` for one element when order is free. For two mutable elements: `swap(i, j)` if you only exchange them, `split_at_mut` when a split point is natural, `get_disjoint_mut` for arbitrary positions.

**Read more.** [`Vec::retain`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.retain) · [`Vec::drain`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.drain) · [`split_at_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_at_mut) · [`get_disjoint_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.get_disjoint_mut) · [Splitting borrows (Rustonomicon)](https://doc.rust-lang.org/nomicon/borrow-splitting.html)

## Lesson 03 · Sorting, binary search, and slices

**The model.** Sorting needs a *total order*: for any two elements the comparison answers less, equal or greater, and the answers never contradict each other. The trait `Ord` promises this. `sort()` uses the type's own `Ord`. `sort_by_key(f)` compares the keys that `f` computes. `sort_by(cmp)` takes a comparison function, which is what you need for a descending order or for several fields.

A sort is *stable* if elements that compare as equal keep the order they had before. `sort`, `sort_by` and `sort_by_key` are stable. The `sort_unstable` family may reorder equal elements; in exchange it needs no extra memory and is often faster. If equal elements cannot be told apart, as with plain integers, you cannot see the difference, and the unstable sort is the better choice.

A sorted slice can be searched in O(log n) steps:

- `binary_search(&x)` returns `Ok(index)` of *some* element equal to `x`, or `Err(index)` with the position where `x` could be inserted to keep the order. With several equal elements it does not promise the first one.
- `partition_point(pred)` assumes that `pred` is true for a front part of the slice and false for the rest. It returns the length of the front part. This gives exact boundaries.

Slices compare with each other like words in a dictionary: element by element, and if one is a prefix of the other, the shorter one is less.

**Worked example.** Staff records are sorted by department, and inside a department by the highest salary first. `Ordering::then` adds the second rule; `then_with` does the same and computes the second comparison only when it is needed.

```rust
#[derive(Debug, Clone, PartialEq)]
struct Staff {
    name: &'static str,
    dept: &'static str,
    salary: u32,
}
let staff = |name, dept, salary| Staff { name, dept, salary };
let mut team = vec![
    staff("Ines", "ops", 50),
    staff("Tomas", "dev", 70),
    staff("Mira", "ops", 65),
    staff("Jon", "dev", 70),
];

team.sort_by(|a, b| a.dept.cmp(b.dept).then(b.salary.cmp(&a.salary)));
let names: Vec<&str> = team.iter().map(|s| s.name).collect();
// Tomas and Jon are equal under this comparison. The sort is stable,
// so they keep their input order.
assert_eq!(names, ["Tomas", "Jon", "Mira", "Ines"]);

// The slice is now sorted by department, so it can be cut by department.
let first_ops = team.partition_point(|s| s.dept < "ops");
assert_eq!(first_ops, 2);
assert_eq!(team[first_ops..].len(), 2);

// binary_search_by_key finds one record with that key, not a promised one.
let found = team.binary_search_by_key(&"dev", |s| s.dept);
assert!(matches!(found, Ok(0) | Ok(1)));
assert_eq!(team.binary_search_by_key(&"hr", |s| s.dept), Err(2));
```

Stability also allows sorting in two passes: first by the less important key, then by the more important one. The second pass keeps the order of the first among equal elements.

**Mistakes people make.**

Sorting floating-point numbers with `partial_cmp(..).unwrap()`. `f64` is not `Ord`, because NaN is neither less than, equal to, nor greater than any number. The `unwrap` works until the first NaN arrives:

```rust,should_panic
let mut v = vec![2.0, f64::NAN, 1.0];
v.sort_by(|a, b| a.partial_cmp(b).unwrap());
```

```text
called `Option::unwrap()` on a `None` value
```

`f64::total_cmp` is a total order over all floating-point values, NaN included: `v.sort_by(f64::total_cmp)`.

Searching a slice that is not sorted. Nothing fails; the answer is simply wrong.

```rust
let unsorted = [9, 1, 5];
// The documentation calls the result "unspecified" here. This compiler
// version answers "not found", although 9 is there.
assert_eq!(unsorted.binary_search(&9), Err(3));
assert!(unsorted.contains(&9));                  // the linear search finds it
```

Expecting `dedup` to remove all duplicates. It removes an element only when it equals its *neighbour*, so the usual pair is `sort` (or `sort_unstable`) followed by `dedup`.

Returning a borrowed key from `sort_by_key`. The closure's result type cannot borrow from the element it was given; drill d06 shows the error. Compare two elements directly with `sort_by`, or let `sort_by_cached_key` compute an owned key once per element.

**When to use which.** `sort_unstable` for values where equal means identical. A stable sort when earlier order must survive among equals. `sort_by_cached_key` when the key is expensive to compute. `binary_search` to ask "is it there, and where could it go"; `partition_point` to ask "where does the group begin and end". For a slice that is searched only once, `contains` or `iter().position()` is simpler than sorting first.

**Optional lab: random order.** The standard library has no random number generator. The [random-lab](random-lab/README.md) package uses the `rand` crate to choose an element, shuffle a vector and take a sample. It also practises a testing habit: a test of random code checks properties (same elements, right size, same result for the same seed), not one expected sequence.

**Read more.** [`sort`](https://doc.rust-lang.org/std/primitive.slice.html#method.sort) · [`sort_unstable`](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_unstable) · [`binary_search`](https://doc.rust-lang.org/std/primitive.slice.html#method.binary_search) · [`partition_point`](https://doc.rust-lang.org/std/primitive.slice.html#method.partition_point) · [`f64::total_cmp`](https://doc.rust-lang.org/std/primitive.f64.html#method.total_cmp) · [`Ordering::then`](https://doc.rust-lang.org/std/cmp/enum.Ordering.html#method.then)

## Lesson 04 · Queues and ring buffers

**The model.** A queue adds at one end and removes at the other. A `Vec` is bad at this: `v.remove(0)` moves every remaining element one place down, so it costs O(n).

`VecDeque<T>` ("vec deck", a double-ended queue) solves this with a *ring buffer*. It owns one buffer, like a `Vec`, and remembers where the first element is. Removing the front element only moves that start position. Adding at the back uses the next free slot, and after the last slot of the buffer comes slot 0 again.

```text
VecDeque with capacity 4, after: push_back 1, 2, 3; pop_front, pop_front; push_back 4, 5, 6

slot:     0    1    2    3
        [ 5  | 6  | 3  | 4  ]
                    ▲
                    front            logical order: 3, 4, 5, 6
as_slices() = ([3, 4], [5, 6])
```

`push_front`, `push_back`, `pop_front` and `pop_back` are all O(1) (amortized, because the buffer can grow like a vector's). Indexing with `q[i]` is O(1) too; index 0 is the front.

The price is that the elements are not always one continuous piece of memory, so a `VecDeque` cannot give you a single slice for free. `as_slices()` returns two slices: the front part and the wrapped part. Where the cut falls depends on the history of the queue, and you must not depend on it. `make_contiguous()` rearranges the storage so that everything is in one piece, and returns that piece as `&mut [T]`.

**Worked example.** Visit a small network level by level, starting at node 0. This is the classic use of a queue: nodes wait in arrival order, and each node adds its neighbours at the back.

```rust
use std::collections::VecDeque;

// links[n] lists the nodes that node n points to.
let links = [vec![1, 2], vec![3], vec![3], vec![]];
let mut seen = [false; 4];
let mut order = Vec::new();

let mut waiting = VecDeque::from([0]);
seen[0] = true;
while let Some(node) = waiting.pop_front() {
    order.push(node);
    for &next in &links[node] {
        if !seen[next] {
            seen[next] = true;
            waiting.push_back(next);
        }
    }
}
assert_eq!(order, [0, 1, 2, 3]);
```

The wrapped state from the diagram, and the two ways to deal with it:

```rust
use std::collections::VecDeque;

let mut q = VecDeque::with_capacity(4);
q.extend([1, 2, 3]);
q.pop_front();
q.pop_front();
q.extend([4, 5, 6]);

// Do not depend on where the cut is. The two parts together are everything.
let (front, back) = q.as_slices();
assert_eq!(front.len() + back.len(), 4);
assert_eq!(q.iter().copied().collect::<Vec<_>>(), [3, 4, 5, 6]);

// One slice, so that slice methods can be used.
q.push_front(9);
q.make_contiguous().sort();
assert_eq!(q, [3, 4, 5, 6, 9]);
```

A `Vec` and a `VecDeque` convert into each other without a new allocation. `VecDeque::from(vec)` takes over the vector's buffer as it is. `Vec::from(deque)` keeps the buffer too, and moves elements inside it only when they do not already start at its beginning.

**Mistakes people make.**

Calling a slice method on the queue. `Vec` lends you all slice methods; `VecDeque` cannot, for the reason above.

```rust,compile_fail,E0599
use std::collections::VecDeque;
fn main() {
    let mut q = VecDeque::from([3, 1, 2]);
    q.sort();
}
```

```text
error[E0599]: no method named `sort` found for struct `VecDeque<T, A>` in the current scope
4 |     q.sort();
  |       ^^^^ method not found in `VecDeque<{integer}>`
```

Write `q.make_contiguous().sort()`.

Treating the first slice of `as_slices()` as the whole queue. It is the whole queue only as long as nothing has wrapped, so such code passes small tests and fails later.

Writing a loop that waits for a condition which `pop_front` cannot produce. `pop_front()` on an empty queue returns `None` and changes nothing. A loop such as "pop while the queue is not shorter than the limit" never ends when the limit is 0. Check what your loop does for an empty queue and for a limit of zero.

**When to use which.** `Vec` when you add and remove at the back only (a stack). `VecDeque` when you add or remove at the front: queues, a history that forgets its oldest entry, a sliding window. Its `rotate_left(k)` moves the first `k` elements to the back, and `rotate_right(k)` moves the last `k` to the front; both work in place. `LinkedList` is almost never faster than either, because each element is its own allocation.

**Read more.** [`VecDeque`](https://doc.rust-lang.org/std/collections/struct.VecDeque.html) · [`as_slices`](https://doc.rust-lang.org/std/collections/struct.VecDeque.html#method.as_slices) · [`make_contiguous`](https://doc.rust-lang.org/std/collections/struct.VecDeque.html#method.make_contiguous)

## Lesson 05 · Priority queues and heap ordering

**The model.** A priority queue answers one question quickly: which element is the greatest? `BinaryHeap<T>` stores its elements in a `Vec` in an arrangement called a *heap*: each element is at least as great as the two elements "below" it. That is much less than sorted, and much cheaper to maintain.

```text
push 3, 1, 4, 1, 5              the same heap as a vector
            5
          ┌─┴─┐                 [ 5 | 4 | 3 | 1 | 1 ]
          4   3                   ▲
         ┌┴┐                      └ peek() and pop() look only here
         1 1
```

Only the top is guaranteed. `peek()` reads it in O(1). `pop()` removes it and repairs the arrangement in O(log n). `push` needs at most O(log n) comparisons, and far fewer on average; like a vector's `push`, it sometimes has to grow the buffer as well. Everything else in the vector is in an order that you must not rely on: `iter()` and `into_vec()` walk the vector as it is. To get all elements in order, call `pop` until the heap is empty, or call `into_sorted_vec()`, which returns them in ascending order.

"Greatest" is decided by `Ord`, so the heap is a *max-heap*. Two tools change what "greatest" means without a new collection type:

- `std::cmp::Reverse(x)` is a wrapper that compares the other way round. In a `BinaryHeap<Reverse<T>>` the smallest `T` is on top.
- Tuples compare field by field, left to right. A heap of `(priority, detail)` pairs is ordered by priority, and by the detail only among equal priorities.

**Worked example.** A clinic calls the patient with the highest severity. Among equal severity, the patient who arrived first goes first. A higher severity must win, but a *lower* arrival number must win, so only the second field is reversed.

```rust
use std::cmp::Reverse;
use std::collections::BinaryHeap;

let mut waiting = BinaryHeap::new();
// (severity, Reverse(arrival number), name)
waiting.push((2, Reverse(1), "Ana"));
waiting.push((5, Reverse(2), "Ben"));
waiting.push((2, Reverse(3), "Caro"));
waiting.push((5, Reverse(4), "Dan"));

assert_eq!(waiting.peek().map(|entry| entry.2), Some("Ben"));

let mut called = Vec::new();
while let Some((_, _, name)) = waiting.pop() {
    called.push(name);
}
assert_eq!(called, ["Ben", "Dan", "Ana", "Caro"]);
```

`peek_mut()` gives mutable access to the top. When the returned guard is dropped, the heap puts the changed element where it now belongs:

```rust
use std::collections::BinaryHeap;

let mut heap = BinaryHeap::from([5, 1, 8, 3]);
if let Some(mut top) = heap.peek_mut() {
    *top = 2; // the old top was 8
}
assert_eq!(heap.peek(), Some(&5));
```

**Keeping only the best k.** Suppose a million numbers pass by and you want the ten largest. Sorting everything would need memory for a million numbers. Instead, keep a small heap of the best candidates. The question to answer for each new number is "is it better than the *worst* candidate I hold?", so the worst candidate must be the one that is cheap to find and to remove. A heap that holds "the largest so far" therefore has its smallest member on top: it is a min-heap. Lesson tasks A and B build exactly this, so the code is not shown here.

**Mistakes people make.**

Reading a heap with a `for` loop and expecting sorted output. The loop compiles and prints every element, in the vector's internal order. Use `into_sorted_vec()` or repeated `pop()`.

Forgetting that the default is a max-heap. A queue of deadlines or distances usually wants the smallest value first: wrap the values in `Reverse`.

Putting floating-point numbers into a heap:

```rust,compile_fail,E0277
use std::collections::BinaryHeap;
fn main() {
    let mut heap = BinaryHeap::new();
    heap.push(2.5_f64);
}
```

```text
error[E0277]: the trait bound `f64: Ord` is not satisfied
4 |     heap.push(2.5_f64);
  |          ^^^^ ------- `f64` doesn't satisfy the trait bound
  |          |
  |          the trait `Ord` is not implemented for `f64`
```

The heap needs a total order, and `f64` has none (Lesson 03). Give the heap a small wrapper type whose `Ord` uses `total_cmp`:

```rust
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug, PartialEq)]
struct Score(f64);
impl Eq for Score {}
impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Score {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

let mut heap = BinaryHeap::from([Score(2.5), Score(-1.0), Score(7.25)]);
assert_eq!(heap.pop(), Some(Score(7.25)));
```

The same pattern, your own `Ord`, is how you order heap entries by one field of a struct.

**When to use which.** A heap when elements arrive and leave over time and you always want the next best one. One `sort` when all the data is present and you want all of it in order. A `BTreeSet` or `BTreeMap` when you also need to find or remove elements that are not at the top; a heap cannot do that efficiently.

**Read more.** [`BinaryHeap`](https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html) · [`Reverse`](https://doc.rust-lang.org/std/cmp/struct.Reverse.html) · [`peek_mut`](https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html#method.peek_mut) · [the `binary_heap` module, with a shortest-path example](https://doc.rust-lang.org/std/collections/binary_heap/index.html)

## Lesson 06 · Hash maps, tree maps, and entries

**The model.** A map stores values under keys. The two map types find a key in different ways, and this decides everything else about them.

`HashMap<K, V>` computes a number from the key, its *hash*, and uses the number to choose a place in a table. Lookup, insertion and removal take O(1) time on average. Keys need `Eq + Hash`. The table has no meaningful order: iterating a `HashMap` gives the entries in an order that is unspecified and usually differs from one run of the program to the next.

`BTreeMap<K, V>` keeps the keys sorted in a tree. Operations take O(log n). Keys need `Ord`. Iteration is always in ascending key order, and the map can answer questions about order: `range(a..b)`, `first_key_value()`, `last_key_value()`. "Ascending" means the key type's `Ord`. For strings that is byte by byte, so every ASCII capital letter comes before every small letter: `"Zoo" < "ant"`.

Both maps have the same core methods: `insert` (returns the old value, if any), `get`, `get_mut`, `contains_key`, `remove`, and iteration with `iter`, `keys`, `values`, `values_mut`. Keys can never be changed in place, because a changed key would be in the wrong position.

**Borrowed lookup.** `get` does not ask for `&K`. It accepts `&Q` for any type `Q` that the key can be borrowed as (`K: Borrow<Q>`, Chapter 13). Because `String: Borrow<str>`, a map with `String` keys is searched with a plain `&str`, and no `String` has to be built for the search.

**Entries.** "Add one to the counter, or start it at one" needs the key's place twice: to look, and to write. The `entry` method finds the place once and returns an `Entry`, which is either `Occupied` or `Vacant`. Helper methods cover the common cases:

- `or_insert(value)` and `or_insert_with(|| value)` fill a vacant entry and return `&mut V` in both cases. The closure runs only when the entry is vacant.
- `or_default()` fills with `V::default()`.
- `and_modify(|v| ...)` changes an existing value and does nothing for a vacant entry.

**Worked example.** A stock list, and an index of words by their first letter.

```rust
use std::collections::{BTreeMap, HashMap};

let mut stock: HashMap<String, u32> = HashMap::new();
for (item, amount) in [("nails", 40), ("glue", 2), ("nails", 15)] {
    stock
        .entry(item.to_string())
        .and_modify(|have| *have += amount)
        .or_insert(amount);
}
assert_eq!(stock.get("nails"), Some(&55)); // a &str finds a String key
assert_eq!(stock.get("tape"), None);

// or_default creates the empty Vec the first time a letter is seen.
let mut index: BTreeMap<char, Vec<&str>> = BTreeMap::new();
for word in ["pear", "apple", "plum", "cherry", "apricot"] {
    let first = word.chars().next().unwrap_or(' ');
    index.entry(first).or_default().push(word);
}
assert_eq!(index[&'a'], ["apple", "apricot"]);
assert_eq!(index.first_key_value().map(|(k, _)| *k), Some('a'));

// Keys from 'b' up to, but not including, 'p': only 'c'.
let middle: Vec<char> = index.range('b'..'p').map(|(k, _)| *k).collect();
assert_eq!(middle, ['c']);
```

When the two cases need different code, match on the entry itself. A vacant entry is filled with `insert`; an occupied one gives access to the value that is already there with `get`, `get_mut` or `into_mut`. Here a seat can be taken only once:

```rust
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

fn reserve<'a>(seats: &mut BTreeMap<u32, &'a str>, seat: u32, guest: &'a str) -> Result<(), &'a str> {
    match seats.entry(seat) {
        Entry::Vacant(free) => {
            free.insert(guest);
            Ok(())
        }
        // The error says who already has the seat.
        Entry::Occupied(taken) => Err(*taken.get()),
    }
}

let mut seats = BTreeMap::new();
assert_eq!(reserve(&mut seats, 12, "Lena"), Ok(()));
assert_eq!(reserve(&mut seats, 12, "Omar"), Err("Lena"));
assert_eq!(seats[&12], "Lena");
```

Each map type has its own `Entry` type: `std::collections::hash_map::Entry` and `std::collections::btree_map::Entry`.

`entry` takes the key by value, because a vacant entry may have to store it. With `String` keys this means one new `String` per call, also when the key is already present. When most keys repeat and the key is expensive to build, look with the borrowed key first and build the owned key only on a miss:

```rust
use std::collections::HashMap;

let mut visits: HashMap<String, u32> = HashMap::new();
for page in ["home", "about", "home"] {
    match visits.get_mut(page) {
        Some(count) => *count += 1,
        None => {
            visits.insert(page.to_string(), 1);
        }
    }
}
assert_eq!(visits["home"], 2);
```

This costs two lookups for a new key and one for a known key, and it allocates only for new keys. What you should not write is `contains_key` followed by `get_mut` or `insert`: two lookups in every case and nothing gained.

**Mistakes people make.**

Keeping a reference to one value while changing the map. An insertion may move every entry to a larger table, so the rule from Lesson 02 applies to maps as well.

```rust,compile_fail,E0499
use std::collections::HashMap;
fn main() {
    let mut stock = HashMap::from([("nails", 40)]);
    let nails = stock.get_mut("nails").unwrap();
    stock.insert("screws", 10);
    *nails += 5;
}
```

```text
error[E0499]: cannot borrow `stock` as mutable more than once at a time
4 |     let nails = stock.get_mut("nails").unwrap();
  |                 ----- first mutable borrow occurs here
5 |     stock.insert("screws", 10);
  |     ^^^^^ second mutable borrow occurs here
6 |     *nails += 5;
  |     ----------- first borrow later used here
```

Finish with the first value before you touch the map again. For two values at once, `HashMap` has its own `get_disjoint_mut`.

Reading with `map[key]` when the key may be missing. Indexing a map is for reading only, and it panics for an absent key:

```rust,should_panic
use std::collections::HashMap;
let stock = HashMap::from([("nails", 40)]);
let glue = stock["glue"];
```

```text
no entry found for key
```

Use `get`, which returns an `Option`.

Letting hash order reach the outside. Printing a `HashMap`, collecting its keys into a `Vec`, or comparing such a `Vec` in a test gives a different order on different runs. Sort the result, or use a `BTreeMap` from the start.

**When to use which.** `HashMap` by default: it is the fastest when you only need "find by key". `BTreeMap` when output must be ordered, when you need the smallest or largest key or a range of keys, or when the key type has `Ord` but no `Hash`.

**Read more.** [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html) · [`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html) · [`hash_map::Entry`](https://doc.rust-lang.org/std/collections/hash_map/enum.Entry.html) · [`Borrow`](https://doc.rust-lang.org/std/borrow/trait.Borrow.html) · [`BTreeMap::range`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html#method.range)

## Lesson 07 · Set algebra and equal-but-distinct values

**The model.** A set stores values and answers "is this value in the set?". `HashSet<T>` is a `HashMap` with keys only, and `BTreeSet<T>` is a `BTreeMap` with keys only, so everything from Lesson 06 carries over: `HashSet` needs `Eq + Hash` and has no order; `BTreeSet` needs `Ord` and iterates in ascending order.

`insert` returns `true` if the value was new and `false` if an equal value was already present. This makes "have I seen this before?" a single call.

Two sets can be combined in four ways. Each method returns an iterator that borrows both sets and builds nothing until you collect it:

```text
a = {1, 2, 3}    b = {3, 4}

a.union(&b)                  1 2 3 4     in a, in b, or in both
a.intersection(&b)           3           in both
a.difference(&b)             1 2         in a, not in b
a.symmetric_difference(&b)   1 2 4       in exactly one of them
```

The operators `|`, `&`, `-` and `^` on two set *references* do the same and return a new set. `is_subset`, `is_superset` and `is_disjoint` answer yes or no.

**Equal but not identical.** A set treats two values as the same when `==` says so. `==` is whatever the type's `Eq` implementation compares, and that may be less than all fields. Two values can therefore be equal for the set and still differ in a field that `Eq` ignores. Three methods deal with this:

- `insert(v)` keeps the value that is *already stored* and drops `v`.
- `replace(v)` stores `v` and returns the old value, if there was one.
- `get(&v)` returns a reference to the *stored* value that equals `v`.

**The rule that connects `Eq` and `Hash`.** A `HashSet` first goes to the place that the hash selects and only then compares with `==`. If two equal values had different hashes, the set would look in the wrong place and report "not present". So: **whenever `a == b`, `hash(a)` must equal `hash(b)`**. In practice, `Hash` must feed the hasher exactly the fields that `Eq` compares, in the same form. `#[derive(PartialEq, Eq, Hash)]` together always satisfies the rule. If you write one by hand, write the other by hand too.

**Worked example.** A keyword is the same keyword however it is capitalized, but the spelling that was stored first should be kept for display.

```rust
use std::collections::{BTreeSet, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Debug)]
struct Keyword(String);

impl PartialEq for Keyword {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}
impl Eq for Keyword {}
impl Hash for Keyword {
    // Eq ignores ASCII case, so the hash must ignore it too.
    fn hash<H: Hasher>(&self, state: &mut H) {
        for byte in self.0.bytes() {
            state.write_u8(byte.to_ascii_lowercase());
        }
    }
}

let mut keywords = HashSet::new();
assert!(keywords.insert(Keyword("Rust".into())));
assert!(!keywords.insert(Keyword("RUST".into()))); // equal: not inserted
let stored = keywords.get(&Keyword("rust".into())).map(|k| k.0.as_str());
assert_eq!(stored, Some("Rust")); // the first spelling was kept

let old = keywords.replace(Keyword("RUST".into()));
assert_eq!(old.map(|k| k.0), Some("Rust".to_string()));

// Set algebra, with BTreeSet for results in a fixed order.
let a = BTreeSet::from([1, 2, 3]);
let b = BTreeSet::from([3, 4]);
assert_eq!(a.union(&b).copied().collect::<Vec<_>>(), [1, 2, 3, 4]);
assert_eq!(&a - &b, BTreeSet::from([1, 2]));
assert!(BTreeSet::from([2]).is_subset(&a));
```

**Mistakes people make.**

Deriving `Hash` and writing `PartialEq` by hand (or the reverse). The derived `Hash` reads every field, the hand-written `==` ignores some, and the rule above is broken. Nothing crashes: `contains` returns `false` for a value that you can see in the set when you print it. Clippy rejects this combination with the lint `derived_hash_with_manual_eq`.

Trying to change an element where it is stored:

```rust,compile_fail,E0599
use std::collections::HashSet;
fn main() {
    let mut words: HashSet<String> = HashSet::from([String::from("tree")]);
    for word in words.iter_mut() {
        word.push('s');
    }
}
```

```text
error[E0599]: no method named `iter_mut` found for struct `HashSet<T, S, A>` in the current scope
4 |     for word in words.iter_mut() {
  |                       ^^^^^^^^
help: there is a method `iter` with a similar name
```

A set has no `iter_mut` and no `get_mut`, on purpose. An element's position depends on its hash or its order; a changed element would be in the wrong position and could not be found any more. Take the element out (`take`, `remove`), change it, and insert it again. The same danger exists if an element changes its identity through interior mutability (`Cell`, `RefCell`). The compiler cannot stop that. The documentation calls it a logic error: the set may then give wrong answers, but it will not cause undefined behavior.

Using `f64` as an element. It has no `Eq` and no `Hash`, for the NaN reason from Lesson 03.

**When to use which.** `HashSet` for fast membership when order does not matter. `BTreeSet` when you need sorted output, the smallest or largest element, or a range. A sorted `Vec` with `dedup` when the data is collected once and then only read: it uses the least memory and `binary_search` finds members.

**Read more.** [`HashSet`](https://doc.rust-lang.org/std/collections/struct.HashSet.html) · [`BTreeSet`](https://doc.rust-lang.org/std/collections/struct.BTreeSet.html) · [`HashSet::replace`](https://doc.rust-lang.org/std/collections/struct.HashSet.html#method.replace) · [`Hash` and `Eq`](https://doc.rust-lang.org/std/hash/trait.Hash.html#hash-and-eq)

## Lesson 08 · Hashing and custom builders

**The model.** Three traits share the work of hashing.

- `Hash` is implemented by the *key* type. Its `hash` method does not compute a number. It feeds the key's data, piece by piece, into a hasher.
- `Hasher` is the *algorithm*. `write(&mut self, bytes)` takes in data; it may be called many times for one key. `finish(&self)` returns the `u64` result for everything written so far.
- `BuildHasher` is a *factory*. A map keeps one builder and asks it for a fresh hasher for every key it has to hash.

```text
key.hash(&mut hasher)  ──►  hasher.write(..), hasher.write(..)  ──►  hasher.finish() = u64
       Hash                           Hasher                         chooses a place in the table
```

The full type of a hash map is `HashMap<K, V, S>`, where `S` is the builder. If you do not name `S`, it is `RandomState`. `RandomState` picks random keys for its hash function when it is created, so the same string gets different hashes in different maps and in different runs. This is why iteration order changes between runs, and it is a defence: someone who can choose the keys of your map cannot predict which keys will collide.

A *collision* means that two different keys have the same hash, or land in the same place of the table. Collisions are normal and never make a map wrong, because the map always confirms a candidate with `==`. They only cost time. If every key collided, each lookup would compare with many keys, and the map would be as slow as a list.

`BuildHasherDefault<H>` turns any `Hasher` that implements `Default` into a builder: each new hasher is `H::default()`. The builder itself stores nothing.

**Worked example.** FNV-1a is a simple, public hash function: start with a fixed number, and for each byte do one XOR and one multiplication.

```rust
use std::collections::HashMap;
use std::hash::{BuildHasher, BuildHasherDefault, Hasher};

struct Fnv1a(u64);

impl Default for Fnv1a {
    fn default() -> Self {
        Fnv1a(0xcbf2_9ce4_8422_2325) // the starting value that FNV defines
    }
}
impl Hasher for Fnv1a {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

// The state carries on from one write to the next.
let mut one = Fnv1a::default();
one.write(b"ab");
let mut two = Fnv1a::default();
two.write(b"a");
two.write(b"b");
assert_eq!(one.finish(), two.finish());

type FnvBuild = BuildHasherDefault<Fnv1a>;

// No random keys: the same key has the same hash in every builder.
assert_eq!(FnvBuild::default().hash_one("tide"), FnvBuild::default().hash_one("tide"));

let mut ports: HashMap<&str, u16, FnvBuild> = HashMap::default();
ports.insert("http", 80);
ports.insert("https", 443);
assert_eq!(ports.get("https"), Some(&443));
```

The map works exactly as before. Only the function that chooses places in the table has changed.

**Mistakes people make.**

Creating a map with a custom builder through `HashMap::new()`. `new` and `with_capacity` exist only for the default builder:

```rust,compile_fail,E0308
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, DefaultHasher};
fn main() {
    let mut map: HashMap<u32, u32, BuildHasherDefault<DefaultHasher>> = HashMap::new();
    map.insert(1, 2);
}
```

```text
error[E0308]: mismatched types
4 |     let mut map: HashMap<u32, u32, BuildHasherDefault<DefaultHasher>> = HashMap::new();
  |                  ----------------------------------------------------   ^^^^^^^^^^^^^^ expected `HashMap<u32, u32, BuildHasherDefault<_>>`, found `HashMap<_, _>`
  = note: expected struct `HashMap<u32, u32, BuildHasherDefault<DefaultHasher>>`
             found struct `HashMap<_, _, RandomState>`
```

Use `HashMap::default()`, or `HashMap::with_hasher(builder)` when the builder has state.

Storing hash values outside the program. `DefaultHasher` and the `Hash` implementations of standard types may change between Rust versions. A hash written to a file or sent over a network by one build may not match the next build. For stored data, use an algorithm with a published definition and feed it bytes that you define yourself.

Assuming how a type calls `write`. A `Hash` implementation may call `write` several times and may add extra bytes; `str`, for example, at present writes one more byte after its text, so that `("ab", "c")` and `("a", "bc")` hash differently. Test a hasher by calling `write` yourself, not by guessing what `"text".hash(..)` sends.

Reducing the state too early. A hasher that folds every `write` into a `u8` has only 256 possible results, however many keys there are.

**When to use which.** Keep `RandomState` whenever keys can come from outside the program: network input, file names, user text. Use a simpler, faster hash function only for keys that you control (small integers, internal identifiers), and only after a measurement shows that hashing costs real time. Published crates such as `rustc-hash`, `ahash` and `foldhash` provide such functions. Never weaken the hasher of a map that an attacker can fill.

**Read more.** [`std::hash`](https://doc.rust-lang.org/std/hash/index.html) · [`Hasher`](https://doc.rust-lang.org/std/hash/trait.Hasher.html) · [`BuildHasher`](https://doc.rust-lang.org/std/hash/trait.BuildHasher.html) · [`BuildHasherDefault`](https://doc.rust-lang.org/std/hash/struct.BuildHasherDefault.html) · [`RandomState`](https://doc.rust-lang.org/std/hash/struct.RandomState.html)

## Lesson 09 · Capstone: a task scheduler and audit

The capstone gives you no new library feature. It asks for a decision: which collections, holding what, so that every operation is cheap. This section describes a way to make that decision. It does not contain the solution.

**Start from the operations, not from the data.** Write down each thing the scheduler must do and what kind of question it is:

```text
is this name pending, and with which priority?      find by key
which pending task comes next?                      greatest by (priority, then name)
forget this pending task                            remove by key
which tasks ran last?                               oldest out, newest in, small fixed size
how many runs at priorities 4 to 7?                 ordered keys, a range
```

No single collection answers all five questions well. Lessons 04 to 06 each answer one or two of them. A design with several collections is normal; the work is to keep them in agreement.

**Choose one source of truth.** When two collections describe the same tasks, decide which one is authoritative. The other one is an *index*: a second view that exists only to answer one question quickly. Every method must leave the two in a state that you can describe in one sentence. Write that sentence as a comment above the struct before you write any method. It is called an *invariant*.

**The hard operation is removal from the middle.** "Next by priority" suggests a heap, but a `BinaryHeap` can remove only its top (Lesson 05). There are two honest answers, and both pass the tests:

1. *Remove directly.* Use an ordered collection in place of the heap. It finds the greatest element and can also remove any element in O(log n). The price is that you must know the full ordering key of the element you remove.
2. *Remove later.* Keep the heap, let cancelled or outdated entries stay in it, and recognize them when they reach the top by asking the source of truth. This is called *lazy deletion*. The price is memory for dead entries, and one question that you must answer exactly: when is a heap entry still valid? "The name is pending" is not a sufficient answer. Find the sequence of calls that breaks it before the tests do.

A third answer, searching all pending tasks for the best one on every run, also passes the tests, because tests cannot measure time reliably. It costs O(n) per run, and the contract excludes it. One technique test does check that `run_next` builds no new collection per call.

**Tie-breaks belong in the type.** "Highest priority first, then smallest name" mixes two directions. Lesson 05 showed how a tuple and `Reverse` express that. The order of your results must come from this ordering and never from a hash map's iteration order.

**Before you run the tests,** predict in NOTES.md what your design does for: the same name submitted twice with different priorities; cancel followed by a new submit of the same name; a task that runs and is submitted again; a history limit of zero.

**Read more.** [When should you use which collection?](https://doc.rust-lang.org/std/collections/index.html#when-should-you-use-which-collection) · [Performance table of the standard collections](https://doc.rust-lang.org/std/collections/index.html#performance)
