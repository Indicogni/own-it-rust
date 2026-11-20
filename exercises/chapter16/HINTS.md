# Chapter 16 hints — Standard collections

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Read the lesson's section in [CONCEPTS.md](CONCEPTS.md) and the compiler's message first. Each lesson has two hints: open the first one only after a real attempt, and the second one only if the first was not enough. The rules in each task's doc comment ("in place", "hold at most k + 1 numbers") are part of the exercise. *Check your explanation* is a model answer to the lesson's EXPLAIN prompt; read it after you have written your own. Model answers for the mastery check are at the end of this file.

## 01 · Vector access and structural editing

<details><summary>Hint 1</summary>

Choose order-preserving operations when the contract exposes sequence order. Each of the three tasks is one or two calls to a `Vec` or slice method.

</details>
<details><summary>Hint 2</summary>

Use `get` for checked access (`copied()` turns `Option<&i32>` into `Option<i32>`), `split_off` to move the tail into its own vector, and `reverse` on that tail. For C, `swap` exchanges two elements and `fill` overwrites a range; a slice pattern such as `[first, interior @ .., last]` matches only when there are at least two elements.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`len` is how many elements exist; `capacity` is how many fit before the vector must reallocate, and `capacity >= len` always. `reserve` and `with_capacity` change only capacity, never length, and guarantee *at least* the requested room, so tests must not assume an exact number. `split_off(i)` keeps elements `..i` in the original buffer and moves the tail into a newly allocated vector; ownership of each tail element moves with it. `remove` preserves order by shifting the tail (O(n)); `swap_remove` fills the hole with the last element (O(1)) and changes order. `swap` and `fill` edit in place and never change the length.

</details>

## 02 · Growth, removal, and borrow validity

<details><summary>Hint 1</summary>

`split_at_mut(mid)` turns one `&mut` slice into two that cannot overlap. For two indexes `a` and `b`, there is always a split point that puts them on different sides. Which one is it when `a > b`?

</details>
<details><summary>Hint 2</summary>

Return `None` first for `a == b` and for an index that is not smaller than `len`. Then split at the *larger* index: the smaller index is still valid in the left part, and the larger one is element 0 of the right part. Build the pair in the order (a, b), which means two branches. For B use `retain`; for C use `swap_remove`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A push *may* reallocate, and the borrow checker reasons about types, not about today's capacity, so any `&v[i]` that is still used after `v.push(..)` is rejected (E0502) even if spare capacity exists. The fixes are to copy the element, end the borrow before mutating, or keep an index and reborrow afterward. `split_at_mut` proves disjointness: it returns two non-overlapping `&mut` slices, so two mutable borrows can coexist without `unsafe`. `get_disjoint_mut([a, b])` is the std shortcut for arbitrary positions; it checks at run time and returns `Err` for an index that is out of range or used twice. `retain` removes in place and keeps relative order; `swap_remove` is the O(1) unordered removal.

</details>

## 03 · Sorting, binary search, and slices

<details><summary>Hint 1</summary>

Find insertion ranges with `partition_point` when duplicate boundaries matter. A stable sort must not look at the labels at all.

</details>
<details><summary>Hint 2</summary>

Stable-sort by score only (`sort_by_key(|record| record.0)`), so equal-score labels keep input order. Two `partition_point` calls find the lower and upper bounds: the first counts the elements `< target`, the second the elements `<= target`. For C, sort and then `dedup`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A stable sort never reorders elements whose keys compare equal, so a secondary order that already exists in the input (arrival time, say) survives sorting by the primary key. An unstable sort may permute equal elements, but it needs no scratch buffer and is often faster, so prefer it when elements that compare equal are interchangeable (primitives, or keys that are unique), since then instability cannot be observed. `binary_search` on duplicates returns *some* matching index, not necessarily the first, and on unsorted input its result is meaningless. `partition_point` gives exact boundaries: the first index where the predicate turns false. `dedup` removes only *consecutive* repeats, so sort first for global uniqueness.

</details>

## 04 · Queues and ring buffers

<details><summary>Hint 1</summary>

A bounded history adds the newest value at the back and lets the oldest values leave at the front. Check what your loop does when the limit is zero: `pop_front` on an empty queue returns `None` and changes nothing.

</details>
<details><summary>Hint 2</summary>

Push at the back first, then pop from the front *while the queue is longer than the limit*. That one loop also handles an oversized queue and a limit of zero. For B, `pop_front()?` returns early for an empty queue; push a clone back and return the value.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`VecDeque` is a ring buffer: it tracks a head index inside one allocation, so `pop_front` just moves the head instead of shifting every element like `Vec::remove(0)`. As a result the elements may wrap around the end of the buffer, and `as_slices` can return two pieces; where the split falls is not something to depend on. `make_contiguous` rotates the storage so a single slice holds every element in logical order, which is what you need before calling slice methods such as `sort`. A FIFO queue pushes at the back and pops from the front.

</details>

## 05 · Priority queues and heap ordering

<details><summary>Hint 1</summary>

`BinaryHeap` uses `Ord`; `Reverse` flips the comparison, so the *smallest* number is on top. For A: after a push the heap may hold one value too many. Which value has to go, and where is it? For C: look at the top with `peek` before you take anything.

</details>
<details><summary>Hint 2</summary>

A: `push(Reverse(value))`, then `pop()` once if `len() > k`; the pop removes the smallest value. B: call `offer` for every value of the stream; `into_sorted_vec()` on `Reverse` values is ascending by `Reverse`, which is descending by number. C: loop while `peek()` shows a due time `<= now`, `pop` inside the loop, and stop at the first entry that is not due. A loop that only *skips* such an entry never ends, because the same entry stays on top.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`BinaryHeap` is a max-heap that only guarantees the *top* element: `iter()` and `into_vec()` visit the internal array in heap order, which is not sorted. `pop` repeatedly (or `into_sorted_vec`, which is ascending) to get priority order. `Reverse<T>` is a wrapper whose `Ord` is flipped, so a `BinaryHeap<Reverse<T>>` pops the smallest value first without a second collection type. Tuples compare field by field, which is how a heap of `(priority, name)` breaks ties. To keep the k largest values of a long stream, hold them in a min-heap: its top is the weakest candidate, which is the one a new value must beat and the one to remove.

</details>

## 06 · Hash maps, tree maps, and entries

<details><summary>Hint 1</summary>

`entry` combines lookup and update in one step. It needs an owned key, which is why the function asks for `K: Clone`.

</details>
<details><summary>Hint 2</summary>

Use `*map.entry(item.clone()).or_insert(0) += 1` for counting. For B, collect the entries into a vector and sort it by key (or collect into a `BTreeMap` and read it back). For C, `range(start..end)` is the half-open query; clone the values.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Use a `BTreeMap` when the keys' order is part of the answer: range queries, sorted output, smallest/largest key. `HashMap` iteration order is unspecified and changes between runs, so tests and output must sort explicitly. `entry` does one lookup and returns `Occupied` or `Vacant`; `or_insert_with(f)` calls `f` only for a vacant entry. Because `String: Borrow<str>`, a `HashMap<String, V>` can be queried with `&str` via `get`/`contains_key`, with no temporary String allocation. `entry` itself needs an owned key, so with expensive keys that mostly repeat, `get_mut` first and `insert` on a miss can be cheaper.

</details>

## 07 · Set algebra and equal-but-distinct values

<details><summary>Hint 1</summary>

Set membership tracks identity, not necessarily every payload field. `replace` changes the stored representative.

</details>
<details><summary>Hint 2</summary>

Use `BTreeSet` for deterministic algebra output: `union`, `intersection`, `difference` and `symmetric_difference` return iterators, so add `.copied().collect()`. Hash `User` by `id` only, and use `replace` to retrieve the prior representative.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Hash collections assume `a == b` implies `hash(a) == hash(b)`; if `Hash` reads a field that `Eq` ignores, two equal values land in different buckets and lookups silently fail. So hash exactly the identity fields that `Eq` compares. `insert` of an equal value keeps the *existing* element and returns false; `replace` swaps in the new representative and returns the old one. Mutating an element's identity while it sits in a set (through interior mutability, for example) is a logic error: it is now stored under the wrong hash or position, and the set's behavior becomes unspecified, though not undefined.

</details>

## 08 · Hashing and custom builders

<details><summary>Hint 1</summary>

`Hasher` accepts a stream of bytes over one or more `write` calls; a builder constructs a fresh hash state for each key.

</details>
<details><summary>Hint 2</summary>

Convert each byte with `u64::from` and add with `wrapping_add`, so debug and release builds agree and the sum is not limited to 255. `finish` only returns the state. Create the map with `HashMap::default()`; `HashMap::new()` exists only for the standard builder.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A hash only picks a bucket; maps still compare keys with `Eq`, so a collision costs time, never correctness. A deliberately weak hasher like a byte sum makes collisions easy to construct ("ab" and "ba" collide), and an attacker who controls keys can force every insert into one bucket and turn O(1) operations into O(n): a denial-of-service. `RandomState` uses a keyed hash function with random keys to resist that, at some speed cost. `DefaultHasher` output is not stable across Rust versions, so never persist it as an identifier. `BuildHasherDefault<H>` is a zero-sized builder that creates a fresh `H::default()` for each hash.

</details>

## 09 · Capstone: a task scheduler and audit

<details><summary>Hint 1</summary>

A and B: use a map to combine duplicate names, then a heap for scheduling, with a tie-break that covers every case. C: no single collection is enough. Decide which collection is the truth about pending tasks, and write one sentence that says how the others relate to it. A heap cannot remove an entry from its middle, so either choose an ordered collection that can, or let dead entries stay and recognize them later.

</details>
<details><summary>Hint 2</summary>

A: keep the maximum priority per name (`get_mut` and `max`, or `entry`). B: heap entries `(priority, Reverse(name))`; pop at most `limit` times and stop when the heap is empty. C, one possible set of fields: `HashMap<String, u8>` for pending tasks; for the order either a `BinaryHeap<(u8, Reverse<String>)>` or a `BTreeSet<(Reverse<u8>, String)>` (then `pop_first` gives the next task and `remove` withdraws one); a `VecDeque<String>` for the history, as in lesson 04; a `BTreeMap<u8, usize>` with `range` for the audit. With the heap, a popped entry is valid only if the map still has this name *with exactly this priority*; skip every other entry. Remove the name from the map when it runs.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Merging duplicates is a `HashMap` update: O(1) expected per task. A `BinaryHeap` gives O(log n) push and pop, and popping only `limit` items avoids sorting everything. Hash iteration order is unspecified, so the tie-break must be explicit in the heap's ordering: `(priority, Reverse(name))` pops the highest priority first and the lexically smallest name among equals. In the long-lived scheduler the map is the source of truth and the ordered structure is an index into it. With lazy deletion, `cancel` is one map removal (O(1)), and the dead heap entry costs memory and one extra pop later. With an ordered set, `cancel` removes the entry at once in O(log n). A search through all pending tasks on every run would pass the tests but costs O(n) per run; tests cannot measure that, so it is your reasoning that has to exclude it.

</details>

## R01 · Spaced review of Chapters 7, 10, 13–15

<details><summary>Hint 1</summary>

A map can borrow its keys from the input text, so counting words needs no per-word String. Compute a closure's key from `&item` *before* moving the item into the map. Match on the entry to tell a new key from a repeated one.

</details>
<details><summary>Hint 2</summary>

`HashMap<&str, usize>`, then collect and `sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)))`, then `truncate(n)`. For grouping, `map.entry(key(&item)).or_default().push(item)`. For the config, `lines().enumerate()`, `split_once('=')`, `parse::<u32>().map_err(...)?`, and `match map.entry(k) { Entry::Vacant(e) => ..., Entry::Occupied(_) => return Err(...) }`.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one function lets the hasher choose the output order, one returns one item too many. Most `.to_string()` and `.clone()` calls in this file are unnecessary. One function searches its own output again for every input item.

</details>
<details><summary>Hint 2</summary>

Count into a `HashMap<&str, usize>` with `entry`, and add `.then(a.0.cmp(b.0))` to the sort. For `recent_unique`, change the signature to `fn recent_unique<T: Eq + Hash>(events: &[T], limit: usize) -> Vec<&T>`, remember seen events in a `HashSet<&T>` (`insert` returns false for a repeat), and stop when `limit` results exist, for example with `take(limit)`. `users.into_iter().collect()` builds the index by moving each name.

</details>
<details><summary>Check your review</summary>

`word_counts` sorts by count only, so equal counts come out in hash iteration order, which is different on every run; it also allocates two Strings per word just to look one up (`get`/`contains_key` accept `&str`), does a double lookup that `entry` replaces, and clones every key again when collecting. `recent_unique` checks `out.len() > limit` after pushing, so it returns `limit + 1` items (and one item for a limit of 0); `out.contains` searches the whole output for every event, which is O(n·k) where a `HashSet` is O(n); it clones every event and takes `&Vec<T>` where `&[T]` will do. `index_by_id` owns the vector yet clones each name; iterate by value or `collect()` directly. Clippy flags the `contains_key`/`insert` pair, the needless `to_string` calls, the hand-written `sort_by`, and the `&Vec` parameter. Clippy does not see the repeated search or the wrong limit; the tests `b_recent_unique_does_not_rescan_its_output` and `b_recent_unique_respects_the_limit` do.

</details>

## Drills

Run `cargo drill ch16`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · Holding a reference across push</summary>E0502. `first` borrows the vector immutably and is used after `push`, which needs `&mut`. Spare capacity doesn't matter: the borrow checker only sees that `push` *may* reallocate. Move the push below the first assertion so the borrow ends first, or copy the value out (`let first = scores[0];`) and compare it directly.</details>
<details><summary>d02 · Updating a map through an index</summary>E0594. `HashMap` implements `Index` but not `IndexMut`, so `counts["a"]` is read-only (and would panic for a missing key). Use `*counts.entry("a").or_insert(0) += 1`, or `if let Some(n) = counts.get_mut("a") { *n += 1 }`.</details>
<details><summary>d03 · Sorting floats</summary>E0277: `f64` is only `PartialOrd`, because NaN is not comparable with anything, so `sort` (which needs `Ord`) is unavailable. Use `readings.sort_by(f64::total_cmp)`, which defines a total order that includes NaN.</details>
<details><summary>d04 · A key type without Hash</summary>E0277: `HashMap::insert` requires `K: Eq + Hash`, and `Cell` doesn't implement `Hash`. Add `Hash` to the derive; derived `Hash` and `Eq` use the same fields, so they stay consistent. (A `BTreeMap` would need `Ord` instead.)</details>
<details><summary>d05 · Two mutable indices</summary>E0499. Each `&mut slots[i]` borrows the whole vector mutably; the compiler doesn't reason about index values. For a plain swap use `slots.swap(0, 2)`. To really hold both elements, use `slots.get_disjoint_mut([0, 2])`, which checks at run time that the indexes are in range and different and returns `Ok([a, b])`, or `split_at_mut` to get two disjoint `&mut` slices.</details>
<details><summary>d06 · Sorting by a borrowed key</summary>error (no code): "lifetime may not live long enough". `sort_by_key` is declared with a closure of type `FnMut(&T) -> K`, where `K` is one single type. `K` cannot mention the lifetime of the `&T` that each call receives, so a key that borrows from the element cannot be expressed. Use `sort_by(|a, b| a.name.cmp(&b.name))`, which borrows both elements only during one comparison, or `sort_by_cached_key(|t| t.name.clone())` when an owned key is acceptable and expensive to compute.</details>
<details><summary>d07 · The heap's default order</summary>Panics: `BinaryHeap` is a max-heap, so `pop` returns 30. Store `Reverse(deadline)` values (the import is already there) and unwrap with `let Reverse(soonest) = deadlines.pop().unwrap();`.</details>
<details><summary>d08 · dedup on unsorted data</summary>Panics: `dedup` removes only *consecutive* duplicates, and no two equal tags are adjacent, so all four remain. Sort first (`tags.sort(); tags.dedup();`), or use a set if you need to keep first-seen order.</details>
<details><summary>d09 · contains on a Vec&lt;String&gt;</summary>E0308: `Vec<String>::contains` takes `&String`, and `&str` doesn't coerce to it. Use `names.iter().any(|n| n == "grace")`, which compares `String` with `&str` directly.</details>
<details><summary>d10 · Looking up a String key with &amp;str</summary>Compiles and runs. `get` and `contains_key` accept any `&Q` where `String: Borrow<Q>`, and `String: Borrow<str>`, so no temporary `String` is needed for a lookup.</details>

## Mastery check · model answers

Answer the questions in the [chapter guide](README.md#mastery-check) from memory first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**1. Choosing a collection from the operations.**
List what the task does most often. Add and remove at the back, or read by position: `Vec`. Add or remove at the front as well: `VecDeque`. Always take the greatest (or, with `Reverse`, the smallest) element next: `BinaryHeap`, with O(log n) `push` and `pop`. Find by key: `HashMap`, O(1) on average. Find by key *and* need sorted output, the smallest or largest key, or a range of keys: `BTreeMap`, O(log n). Only membership: `HashSet` or `BTreeSet`, by the same reasoning. If a task needs two of these abilities (next by priority and removal by name, for example), use two collections and decide which one is the truth.

**2. Order, moved buffers, and rejected changes.**
`remove`, `insert`, `retain`, `dedup`, `drain` and a stable `sort` keep the relative order of the elements that stay. `swap_remove` and the `sort_unstable` family do not. `push`, `insert`, `extend`, `reserve` and `shrink_to_fit` may move a vector's elements to a new buffer; inserting into a hash map may move every entry to a larger table. A reference to an element points into that buffer. All methods that change a collection take `&mut self`, and Chapter 5's rule forbids a `&mut` borrow while a shared reference is alive. The compiler checks only the signatures, so it rejects the change even when today's capacity would be enough (E0502).

**3. Two mutable references into one slice.**
`split_at_mut(mid)` returns the part before `mid` and the part from `mid` on; take one element from each. `get_disjoint_mut([a, b])` returns `Ok([&mut x, &mut y])` after checking at run time that both indexes are in range and different. Further ways: a slice pattern such as `[first, .., last]`, `iter_mut()` (each item is a different element), or `swap(a, b)` when exchanging is all you need.

**4. Sorting by several fields, stability, equal ranges.**
`v.sort_by(|a, b| a.dept.cmp(&b.dept).then(b.salary.cmp(&a.salary)))` sorts by department ascending and salary descending. A stable sort keeps the input order of elements that compare equal, so it can also be applied twice, less important key first. An unstable sort may reorder equal elements; it allocates nothing and is the right choice when equal elements are interchangeable. In a sorted slice, the elements equal to `t` are `s.partition_point(|x| *x < t)..s.partition_point(|x| *x <= t)`. `binary_search` alone returns only one of them, not a promised one.

**5. One lookup, deterministic output.**
`*map.entry(key).or_insert(0) += 1` finds the key's place once and either fills it or updates it; `and_modify(..).or_insert(..)` and `or_default()` are variations. A `HashMap` iterates in an unspecified order that changes between runs. For stable output, collect the entries and sort them with a comparison that leaves no ties, or build a `BTreeMap`.

**6. `Eq`, `Hash`, and equal-but-distinct values.**
Whenever `a == b`, `a` and `b` must produce the same hash, so `Hash` must feed the hasher exactly what `Eq` compares. If it reads more, equal values are searched for in different places and lookups fail without any error. `insert` with a value equal to a stored one keeps the stored value, drops the new one and returns `false`. `replace` stores the new value and returns the old one.

**7. `Hash`, `Hasher`, `BuildHasher`, collisions, weak hashers.**
`Hash` (on the key type) feeds the key's data into a hasher. `Hasher` (the algorithm) takes bytes through `write` and returns a `u64` from `finish`. `BuildHasher` (owned by the map) creates a fresh hasher for each key. The hash only selects where to look; the map then compares keys with `==`, so two keys with the same hash are still two entries. Collisions cost time. With a weak or predictable hash function, someone who chooses the keys can make all of them collide and turn every operation into a walk over all entries. `RandomState` prevents the prediction with random keys, so replace it only for keys that you control.

**8. The capstone again, and a changed requirement.**
If a second `submit` could also *lower* a priority, the map would simply store the new priority. In a design with lazy deletion, push a new heap entry; the old, higher entry is now outdated, and the validity check "this name is pending with exactly this priority" already skips it. In a design with an ordered set, remove the old `(priority, name)` key and insert the new one. A design that judged validity by the name alone would run the task at its old, higher priority, which is wrong.

</details>
