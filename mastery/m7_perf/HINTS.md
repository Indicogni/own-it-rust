# Mastery M7 hints

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Labs](LABS.md)

Try the first hint before opening the second. Open *Check your explanation* only after writing your own answer to the lesson's EXPLAIN prompt. When a budget test fails, read its message first: it says what was counted and what the budget is.

## Lesson 01 · Benchmark design

<details><summary>Hint 1</summary>

`time_batch`: read the clock, run the loop, read the clock again, subtract. Nothing else may read the clock. In `time_batched`, do the work in three phases. First everything that is *not* to be timed: call `setup` `iters` times and keep the inputs in a `Vec`, and create the `Vec` for the outputs with `Vec::with_capacity(iters)`. Then the two clock readings with only the calls of `routine` between them. Then drop the outputs. `calibrate` is a loop around `time_batch`; a `&mut` reference to an `FnMut` closure is itself an `FnMut`, so you can pass `&mut routine` again and again.

</details>
<details><summary>Hint 2</summary>

`time_batched`: `for input in inputs { outputs.push(black_box(routine(black_box(input)))); }` between the two readings, then `let elapsed = clock.now() - start; drop(outputs); elapsed`. If the test says that an allocation happened between the readings, your outputs `Vec` grew: reserve it before the first reading. `calibrate`: `let mut iters = 1; while iters < MAX_BATCH && time_batch(clock, iters, &mut routine) < min_ticks { iters *= 2; }`. `summarize`: sort a copy; write a small helper for the median of a sorted slice, and compute the even case as `low + (high - low) / 2`. For the deviation, replace every value by `value.abs_diff(median)`, sort again, and use the same helper. `compare`: `let noise = NOISE_FACTOR.saturating_mul(base.mad.max(candidate.mad));` and two comparisons with `saturating_add`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A clock reading costs time and has a resolution. If each call is timed on its own, every sample contains the cost of a reading, and a call that is shorter than the resolution is measured as zero or as one whole step. One reading before and one after a batch spreads both errors over all the calls. `black_box` asks the compiler to treat a value as unknown and as used: on an input it prevents the compiler from computing the answer at compile time, and on a result it prevents the compiler from removing a computation whose result nobody reads. It is a hint with no guarantee, and it does not hide a value in any security sense. Noise only adds time, and now and then it adds a lot. One interrupted sample moves the mean a long way and does not move the median at all, so the median says more about the code and less about the machine.

</details>

## Lesson 02 · Reading a profile

<details><summary>Hint 1</summary>

Store the call *tree*, not strings. A node has a name, a parent, its children, a self cost and a call count. Keep the nodes in a `RefCell<Vec<Node>>` and refer to them by index; keep the index of the current node in a `Cell<usize>`. Node 0 can stand for "no span is open". `enter` looks among the children of the current node for one with the same name, creates it if it is missing, and makes it current. `Span` remembers which node was current before, and its `Drop` impl puts that back. `charge` adds to the current node: one index, no search, no allocation. Build the path strings only in `folded` and `hot_paths`.

</details>
<details><summary>Hint 2</summary>

`total_cost(name)`: a unit that was charged at some node counts if `name` appears anywhere on the path from that node up to the root. Walk up through the parents and stop at the first match; that is what counts recursion once. `self_cost(name)` and `calls(name)` are sums over all nodes with that name. For `folded`, collect `(path, cost)` for every node with a self cost above zero, sort, and format each as `"{path} {cost}\n"`. For `hot_paths`, sort the same list with `b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0))` and `truncate(n)`. If you get the panic `RefCell already borrowed`, a borrow is still alive while you call another method that borrows: end it first. Task B: the profile shows that the sort after every key and the search for every key are almost all of the cost. Sort once and remove neighbours that are equal: `sort_unstable` followed by `dedup`. To clone only the keys that survive, sort a `Vec<&Key>` and clone at the end.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The self cost of a frame is what was spent in its own code; the total cost adds everything that was spent in the frames it called. Total cost tells you which region matters, and self cost tells you which code to change, so you sort by self cost to find the place to work on. `charge` runs inside the hottest loops of the profiled program. If it allocated or searched, the profiler would add cost to exactly the places that are called most, and small, frequently called frames would look more expensive than they are. In task B most people expect the linear search to dominate. The profile shows that keeping the list sorted after every key costs about twice as much, which is the point of measuring first.

</details>

## Lesson 03 · Allocation budgets

<details><summary>Hint 1</summary>

A: two passes. The first computes the exact length of the row (separators, fields, two quotes per quoted field, one more per quote inside it); then `String::with_capacity(len)`; the second pass writes. Put the writing into a helper that appends to a `&mut String`, because B needs the same code. B: add up the lengths of all rows, call `out.reserve(total)` once, then append row by row with the helper. C: `input.find("  ")` tells you whether anything has to change. D: `HashMap<String, usize>::get_mut` accepts a `&str`. E: one `Vec<u32>` outside the loop; inside, `clear()` and `extend_from_slice(row)`.

</details>
<details><summary>Hint 2</summary>

A: a field needs quotes if `field.contains([',', '"', '\n', '\r'])`. To write a quoted field, push `"`, then for each `piece` of `field.split_inclusive('"')` push the piece and, if it ends with `"`, one more `"`; then push the closing `"`. C: when `find` returns `None`, return `Cow::Borrowed(input)`. Otherwise `String::with_capacity(input.len())`, and copy characters while skipping a space that follows a space. D: `match counts.get_mut(word) { Some(count) => *count += 1, None => { counts.insert(word.to_owned(), 1); } }`. E: give the scratch buffer the capacity of the longest row before the loop (`rows.iter().map(Vec::len).max()`), or it will grow several times; the median is `scratch.get(scratch.len() / 2).copied().unwrap_or(0)` after `sort_unstable`. Use `sort_unstable`, not `sort`: the stable sort allocates scratch memory of its own for a long row, and the last part of the test counts that.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A: the naive version starts with an empty `String` and grows it step by step; knowing the size in advance turns that into one allocation of the exact size. B: the naive version builds a `String` per row and copies it into the output; writing into the caller's buffer removes every one of them. C: the naive version always builds a new `String`; a `Cow` owns only when the text really changes. D: `entry(word.to_owned())` makes a `String` for every occurrence, although the map needs one only for a new word; looking up with the borrowed `&str` first removes all the others. E: `row.clone()` allocates per row; one reused scratch buffer does it once. An allocation is not worth removing when the code runs rarely, when the allocation is small next to the other work (a database call, a file read), or when removing it makes the code clearly harder to read and no profile asked for it.

</details>

## Lesson 04 · Data layout

<details><summary>Hint 1</summary>

A: write down the alignment of each field (8, 4, 2, 1, 1) and order the fields from the largest to the smallest. Do not remove `repr(C)`: the compiler would then reorder the fields for you, but the struct would no longer have a defined layout for the C side. The tests pass a `Sample` to an `extern "C"` function for this reason; without `repr(C)` the build stops with "`Sample`, which is not FFI-safe". B: which integer type can never hold one particular value? Store something in it that is never that value. C: the large array is the problem; the enum only needs a pointer to it. D: one `Vec` per field of `Order`.

</details>
<details><summary>Hint 2</summary>

B: `pub struct SlotId(NonZeroU32);` and store `index + 1`. `index.checked_add(1)` is `None` exactly for `u32::MAX`, and `NonZeroU32::new` turns the sum into the field. `index()` is `self.0.get() - 1`. C: `Trace(Box<[u64; 16]>)`; `Message::trace` becomes `Message::Trace(Box::new(words))`, and the supplied `trace_words` keeps compiling, because a `&Box<[u64; 16]>` coerces to `&[u64; 16]`. D: `with_capacity` must call `Vec::with_capacity(capacity)` for *every* column, or the "no allocation while pushing" test fails. `get` can start with `*self.ids.get(index)?` and index the other columns directly. `open_value`: zip the three columns, `filter` on the flag, multiply as `u64`, `sum`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

With `repr(C)`, fields lie in declared order, and each one starts at the next multiple of its alignment, so a small field in front of a large one forces padding. With the default representation the compiler is free to choose the order, and it chooses one with little padding. A niche is a bit pattern that a type never uses, such as 0 for `NonZeroU32` or null for a reference or a `Box`; the compiler stores `None` there, so `Option` costs no extra bytes. `SlotId` has one (through `NonZeroU32`), and so has the `Box` inside the `Trace` variant. (`Message` as a whole still stores a separate tag: `Data` uses all 8 bytes that the pointer occupies, so the one free pattern of the pointer cannot tell three variants apart. That is why a message takes 16 bytes and not 8.) A struct of arrays is better when a loop reads few fields of many records: it loads only bytes that it uses. It is worse when code reads or writes all fields of one record, which then touches one cache line per column, and it moves the job of keeping the columns in step from the type system to you.

</details>

## Lesson 05 · Traversal and bounds checks

<details><summary>Hint 1</summary>

A: there is one order in which the cells lie in memory, index 0, 1, 2, and so on. Walk in that order, and work out from the index which column a cell belongs to. The sums are a small `Vec<u64>` that you update as you go. B: two plain loops are not enough, whichever grid you follow. Split the grid into square blocks and finish one block of both grids before you start the next. A block must be small enough that its source lines and its target lines fit into 64 lines together. C: you do not need an index. D: `slice(0..n)` is one check for a whole range, and it panics when the range does not fit, which is exactly the panic the contract asks for.

</details>
<details><summary>Hint 2</summary>

A: `for index in 0..cells.len() { sums[index % width] += u64::from(cells.read(index)); }`, after an assertion that `width > 0` and that `width` divides the length. B: four nested loops. The outer two step through block origins with `(0..n).step_by(BLOCK)`; the inner two go over `row_start..(row_start + BLOCK).min(n)` and the same for columns, and do `dst.write(column * n + row, src.read(row * n + column))`. With `BLOCK = 16`, a block touches 16 lines of each grid. Check the sizes with an `assert!` before the loops. C: `a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()`. D: `let left = left.slice(0..n); let right = right.slice(0..n);`, `Vec::with_capacity(2 * n)`, then a `zip` over the two iterators that pushes both elements.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Both transposes read and write every cell once, so they do the same work; what differs is how often a cache line is still in the cache when it is needed again. The plain version goes down a column of the target: 256 cells in 256 different lines, more than the cache holds, so each line has been evicted before its next cell is written. A block of 16 by 16 cells uses 32 lines, which stay in the cache until all 256 cells of the block are done. The `Lru` model leaves out the several cache levels, the rule that a line can only be stored in a few places chosen by its address, prefetching, and the cost of translating addresses. A check in front of the loop is paid once and makes the loop body free of panics, which leaves the optimizer free to rewrite the loop; a check in every round can stand in the optimizer's way. (For a simple loop a current compiler often splits the loop by itself and keeps the checks only for a few rounds; Lab 5, part 3, shows it. You cannot count on that in a complicated loop.) An iterator is better still because there is no index that could be wrong, so nothing has to be checked or proven.

</details>

## Lesson 06 · Dispatch, inlining and code size

<details><summary>Hint 1</summary>

A: the default body of `apply_all` is the obvious loop that calls `self.apply`. `run_pipeline` then has to choose which loop is the outer one: over the stages or over the values. Only one choice makes one virtual call per stage. B: `checksum` has one line. C: an enum with one variant per kind of step; reduce the rotation count when you parse, so that the variant can hold a small integer. D: the function must be a field of type `F`. A `Box<dyn Fn>` or a `fn` pointer has a size of its own, whatever `F` is.

</details>
<details><summary>Hint 2</summary>

A: `for stage in stages { stage.apply_all(values); }`. B: `checksum_bytes(input.as_ref())`, and in the core `bytes.iter().fold(0x811c_9dc5, |hash, &byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193))`. C: `enum Op { Add(u64), Mul(u64), Xor(u64), RotateLeft(u32) }`, `apply` is a `match *self` with `wrapping_add`, `wrapping_mul`, `^` and `rotate_left`. `run_ops` can call `op.apply_all(values)` for each op: that is the default method, compiled for `Op`, with no trait object anywhere. D: `pub struct Each<F> { function: F }`, `impl<F: Fn(u64) -> u64> Each<F>` with `new`, and `impl<F: Fn(u64) -> u64> Stage for Each<F>` whose `apply` is `(self.function)(value)`. Remove the `PhantomData` import and the `'static` bounds; they are no longer needed.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A call through `dyn Trait` loads the function's address from the vtable and jumps to it. The jump itself is cheap when the processor predicts it; the real cost is that the optimizer cannot inline the function or simplify caller and callee together. A generic function has none of that cost, and instead exists once per type: more compile time, more machine code. One virtual call per batch pays the dispatch once and then runs a loop inside the concrete type, where `apply` is a direct call that can be inlined, so the per-element cost of `dyn` disappears. A closure's type is a struct of its captured variables. A closure that captures nothing is a struct with no fields, which has size zero; `Each<F>` has exactly one field of that type, so it has size zero too, and calling it needs no pointer at all, because the type alone says which code to run.

</details>

## Lesson 07 · Buffering

<details><summary>Hint 1</summary>

A: wrap `out` in a `BufWriter`, format each row straight into it with `writeln!`, and end the function with the result of `flush()`. B: wrap `input` in a `BufReader`; keep one `String` outside the loop and call `read_line` on it; a result of 0 means the end. C: three fields are enough: the inner writer, a `Vec<u8>` created with `Vec::with_capacity(capacity)`, and the capacity. Write a private `drain` method first and use it in `write`, `flush`, `into_inner` and `drop`. Because the type has a `Drop` impl, you cannot move the inner writer out of `self` in `into_inner`; think about what kind of field allows a value to be taken out.

</details>
<details><summary>Hint 2</summary>

B: `line.clear()` before every `read_line`, `line.trim()`, skip if empty, and turn a parse failure into an error with `io::Error::new(io::ErrorKind::InvalidData, error)`. C: make the field `inner: Option<W>`; `into_inner` drains and then returns `self.inner.take()`, and `drop` finds `None` and does nothing. `drain` is a loop with a counter `written`: while `written < buffer.len()`, call `inner.write(&buffer[written..])`; `Ok(0)` ends with `Err(io::ErrorKind::WriteZero.into())` (an error made from a kind alone does not allocate), `Ok(n)` adds to the counter, an `Interrupted` error tries again, another error ends. After the loop, on success *and* on failure, remove exactly the written part with `self.buffer.drain(..written)`, then return the result. In `write`: `if data.len() > self.capacity - self.buffer.len() { self.drain()?; }`, then either pass a large piece straight to the inner writer or `extend_from_slice`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`BufWriter` does write its remaining bytes when it is dropped, but `drop` cannot return anything, so a failed write there is ignored. If the last piece of data only leaves the buffer during the drop, a function without a flush returns `Ok(())` although the data was lost; the explicit `flush()?` is the only place where that failure can reach the caller. `write_all` reports an error without saying how many bytes were accepted before it. A buffer that must carry on after an error has to know that number, or it will send bytes twice or drop them, so it calls `write` itself and counts. After a failed drain the buffer must hold exactly the bytes that the inner writer has not accepted, in order. `sum_lines` reuses one `String` because `read_line` appends to a buffer you own; `clear()` keeps its capacity, so after the first few lines no line costs an allocation, and the memory in use does not depend on the size of the input.

</details>

## Lesson 08 · Capstone

<details><summary>Hint 1</summary>

Do Lab 2 and step 1 of Lab 8 first, and let the numbers choose your first change. Then ask of the baseline, one at a time: does it need the whole text in memory (lesson 07, task B)? Does a line need to be a `String` of its own? Do the fields need to be `String`s, or even a `Vec`? Does it need every request, or only four running numbers per service? Does it need every latency, or only the best `top` so far? How many times does it call `write`, and what happens to an error? For the services, you need a map that you can ask with a `&str` and that can give you its keys in sorted order.

</details>
<details><summary>Hint 2</summary>

Read with `BufReader` and `read_line` into one reused `String`. Parse with `line.split_whitespace()`: take four fields with `next()?`, parse three of them with `.parse().ok()?`, and reject the line if a fifth field exists. Keep `BTreeMap<String, Stats>` with a small `Stats` struct (requests, errors, bytes, largest latency); look up with `get_mut(service)` and insert `service.to_owned()` only on a miss. For the slowest requests, keep a `BinaryHeap<Reverse<u32>>` of at most `top` entries: push while it is smaller than `top`; otherwise compare the new value with `peek_mut()` and replace the top if the new value is larger. Do not create the heap with `with_capacity(top)`: `top` may be `usize::MAX`. At the end, `into_sorted_vec()` of `Reverse` values is already in descending order. Write through a `BufWriter` with `write!` and `writeln!`, and return `out.flush()`. `top_k` is the same heap, generic over `T: Ord`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

In the model solution, the largest gain in allocations came from no longer making a `String` for the line and for each field (six allocations per line become none). The largest gain in peak memory came from reading line by line and keeping running totals instead of the text, the lines and all requests. In time, on the author's machine, the whole set of changes made `summarize` about four times faster, and almost all of that is the allocations; the heap for the slowest requests made `top_k` about ten times faster on its own benchmark, but the sort was only about 2% of the baseline's time. A thing that looks slow and can stay is the idea of a linear search over twelve service names: it is cheap at that size. (The model solution uses a sorted map anyway, because it also removes the sort at the end and the copy of the key.)

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs that change results, and one place where an error is thrown away. Each bug sits next to a comment that explains why the code is fast. Ask of `checksum_words`: what happens to a length that is not a multiple of four? Ask of `render_groups`: what is in the buffer when the second group starts? Ask of `export`: who finds out when the flush fails, and how many system calls does it make for 1,000 pairs?

</details>
<details><summary>Hint 2</summary>

`checksum_words`: `chunks_exact(4)` leaves out the last one to three bytes. `data.as_chunks::<4>()` returns the whole words as arrays and the rest as a slice; pad the rest with zeros in a `[0u8; 4]`. `render_groups`: call `buffer.clear()` at the start of each group, format numbers with `write!(buffer, "{number}")` (it needs `use std::fmt::Write as _;`), iterate with `for group in groups` and `enumerate`, and take `&[Vec<u32>]`. Keep the one reused buffer and push `buffer.clone()`: the clone is a single allocation of exactly the right size, while a fresh `String` per group would grow in several steps. `export`: wrap `out` in a `BufWriter`, write with `writeln!(out, "{key}={value}")?`, and return `out.flush()`.

</details>
<details><summary>Check your review</summary>

`checksum_words` was rewritten with `chunks_exact(4)` "for speed" and silently drops up to three trailing bytes, so two inputs that differ only at the end get the same checksum. `render_groups` reuses one buffer and never clears it, so every result also contains all earlier groups; a test with a single group cannot see this. The reuse does not even save much: `to_string()` still allocates for every number. `export` writes every line with its own `write_all`, which is a system call per line on a real file or socket; it builds a `String` per line and clones each key for no reason; and it ignores the result of `flush`, so the caller is told `Ok` when the data may not have arrived. Clippy points at the two index loops, at `&Vec<Vec<u32>>`, and at `chunks_exact` with a constant size, for which it suggests `as_chunks`. The lesson for reviews: when a change claims to be "the same, only faster", ask for the test that shows the "same" on inputs with awkward lengths, repeated calls and failing writers.

</details>

## Drills

<details><summary>d01 · One buffer for every line</summary>

**E0597.** `fields` is declared outside the loop and holds `&str` values. `upper` is created inside the loop and destroyed at the end of each round, while `fields` still exists and is used in the next round. The borrow checker does not know that `clear()` removes the old borrows. Fix: the text must outlive the buffer. Build all the owned strings first (`let upper: Vec<String> = ...`), then loop over references to them.

</details>
<details><summary>d02 · Fields in declared order</summary>

**panic.** The program compiles. With `repr(C)`, `offset` must start at a multiple of 8, so 7 bytes of padding follow `tag`; `kind` and `len` then need 8 more bytes. The size is 24, and the assertion that expects 16 fails. Fix: declare the fields from the largest alignment to the smallest: `offset`, `len`, `tag`, `kind`.

</details>
<details><summary>d03 · Whole chunks only</summary>

**panic.** `chunks_exact(2)` yields only complete pairs. The fifth value is left out, the sum is 10 instead of 15, and the second assertion fails. Fix: add the values after the last whole pair (`values[whole..]`, or the `remainder()` of the iterator).

</details>
<details><summary>d04 · Looking at the target of a buffered writer</summary>

**E0502.** `writer` holds the mutable borrow of `out`. `BufWriter` has a destructor, so that borrow lasts until `writer` is dropped, not just until its last use; reading `out.len()` before that is a second borrow. And even without the borrow problem the assertion would be wrong at that line: the five bytes are still in the buffer. Fix: flush, `drop(writer)`, then look at `out`.

</details>
<details><summary>d05 · A generic method behind `dyn`</summary>

**E0038.** A vtable has one slot per method. `put<T>` is a different function for every `T`, so no single slot can hold it, and the trait is not dyn compatible. Fix: make the method take `&dyn Display`, one function for all types. (Adding `where Self: Sized` to the method would also compile the trait object, but then `put` could not be called on it.)

</details>
<details><summary>d06 · Which <code>Option</code>s are free?</summary>

**ok.** `NonZeroU32` never holds 0 and a `Box` is never null, so `None` is stored in that unused bit pattern and the `Option` has the size of its content. A `u32` uses all of its bit patterns, so `Option<u32>` needs a separate tag.

</details>
<details><summary>d07 · Skipping the bounds check</summary>

**E0133.** `get_unchecked` is an `unsafe fn`; calling it outside an `unsafe` block is an error. The compiler is not asking for the keyword but for a reason why `i` is in range, and this function has none: nothing ties `n` to the length. Fix without `unsafe`: `values[..n].iter().sum()`, one check for the range and none in the loop.

</details>
<details><summary>d08 · A check moved in front of the loop</summary>

**panic.** `&b[..n]` with `n = a.len()` requires `b` to have at least as many elements as `a`. For the second call it does not: `range end index 3 out of range for slice of length 2`. Fix: `zip` stops at the shorter slice, or take `n` as the smaller of the two lengths first.

</details>
<details><summary>d09 · Two closures, one return type</summary>

**E0308.** Each closure expression has its own type, and `impl Fn(u64) -> u64 + Copy` names exactly one type, so the two branches cannot both be returned. Fix: one closure that captures `amount` and `add` and decides inside. `Box<dyn Fn>` is ruled out by the `Copy` bound, and it would cost an allocation and an indirect call. Two closures that capture nothing would have compiled, because both can become the same `fn` pointer.

</details>
<details><summary>d10 · A reference into a packed struct</summary>

**E0793.** In a packed struct, `value` starts at offset 1. A `&u32` promises an address that is a multiple of 4, and merely creating a reference that breaks this promise is undefined behavior, so the compiler refuses. Fix: copy the field into a local variable. When you need a pointer, use `&raw const record.value` with `read_unaligned`.

</details>

## Mastery check: model answers

<details><summary>Open after answering in NOTES.md</summary>

1. The input was a constant that the compiler could see, so it computed the sum at compile time; or the result was not used, so the loop was removed. Confirm by passing the input, and then the result, through `black_box` and watching the time become plausible, or by reading the compiler's output for the benchmark and finding no loop. A time that does not grow when the input grows is the same symptom.
2. Reading the clock costs time and has a resolution, and both errors are per reading. Two readings around many calls spread them over all the calls. The harness starts with a small batch and doubles it until one batch takes long enough to time well.
3. `escape`: it has the largest self cost, and `render`'s large total is mostly `escape`'s time passing through. If `escape` became free, the run would take 45% of the old time; that is the most any change to `escape` can gain.
4. Borrow instead of owning (return a `&str` into the input). Own only when needed (`Cow` when the result is usually the input). Know the size (`with_capacity` when the length can be computed). Reuse a buffer (`clear()` in a loop that needs a temporary of the same kind each round). Also: look up with a borrowed key before making an owned one.
5. 32 bytes: `u8` at 0, `u64` at 8, `u16` at 16, `u32` at 20, `u8` at 24, and the size rounded up to a multiple of 8. Ordered as `u64`, `u32`, `u16`, `u8`, `u8` it is 16 bytes, with no padding.
6. A `Box` is never null, so the null pattern is free to mean `None`. Every bit pattern of a `u64` is a valid `u64`, so `None` needs extra space, and alignment rounds the size up to 16.
7. Faster when a loop reads a few fields of many records, because it loads only what it uses. Slower when code handles all fields of single records, because each record is spread over several cache lines. In code, the columns must be kept at the same length by hand, and you cannot hand out a `&Record`.
8. Column by column, each step moves a whole row ahead in memory, so every access is in a different cache line; if the grid is larger than the cache, those lines are evicted before the next column needs them. Row by row, 16 `u32` values in a row share one line, and the processor can fetch the next lines in advance.
9. The index loop asks for a check on every access, and the compiler may or may not be able to remove them. With sub-slices there is one check per slice in front of the loop, and the loop is provably in range. `zip` has no checks. The first two panic when a slice is too short; `zip` silently stops at the shortest one.
10. A `dyn` call is an indirect call that cannot be inlined; a generic function is copied for every type, which costs compile time and code size. Give the trait a method that takes a whole slice: the dynamic call then happens once per batch, and the loop inside it uses direct calls.
11. The last part of the data is written when the `BufWriter` is dropped. If that write fails, `drop` has no way to report it, so the function has already returned `Ok(())` and data is lost without notice. Only an explicit `flush()?` reports it.
12. After an error, `write_all` does not say how many bytes were accepted. The buffer would not know which bytes to keep, so a retry would send some twice or leave some out.
13. Make the behavior checkable (tests, or a differential test against the current version). Measure on a realistic input, twice, on a quiet machine. Profile, and find the largest self cost. Then one hypothesis, one change, tests, measure again; undo what did not help. Stop when the goal or budget is met and the profile has no single peak left, and write down what was measured.

</details>
