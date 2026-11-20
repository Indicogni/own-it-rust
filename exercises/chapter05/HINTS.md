# Chapter 5 hints — Borrowing with references

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Try the first hint before you open the second. The constraints in each task ("do not allocate", "do not clone") are part of the exercise. Open *Check your explanation* only after you have written your own explanation in [NOTES.md](NOTES.md).

## 01 · Borrowing and dereferencing

<details><summary>Hint 1</summary>

A shared reference (`&T`) lets you read the value it points at. An exclusive reference (`&mut T`) also lets you write it.

</details>
<details><summary>Hint 2</summary>

Call `len` directly on the borrowed value. To store a new integer, write to `*slot`; `std::mem::replace` writes and returns the old value in one step.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The owner (the `String` or integer variable in the caller) keeps its storage. A reference only holds its address, so passing `&text` neither moves nor copies anything, and the owner is usable after the call. The `.` operator dereferences automatically as many times as needed to find a method or field, which is why `r.len()` works on a `&&&Vec`. Assignment has no such rule: `slot = value` would try to store a new reference in the local variable `slot`, so writing to the value behind the reference needs an explicit `*slot = value`.

</details>

## 02 · Reference bindings and identity

<details><summary>Hint 1</summary>

For A, think about what `*slot` is: it is the caller's `&i32` variable. For B, "equal contents" and "the same object" are two different questions with two different tools.

</details>
<details><summary>Hint 2</summary>

A: assign `second` to `*slot` when asked, then read the integer through two dereferences, `**slot`. B: `==` compares the text; `std::ptr::eq(left, right)` compares the addresses of the two `String` objects.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`let mut r = &x` makes the *variable* mutable: `r` can later point at another value, but neither value can be changed through it. `&mut x` is the opposite: the value can be changed, and the variable holding the reference may be immutable. With a reference to a reference (`&mut &i32`), `*rr = &y` changes where the inner reference points, and no integer is written. `==` on references compares the values they point at. `std::ptr::eq` compares addresses (and, for slices, lengths). Two different `String` objects are never the same object, even when their buffer addresses are equal, as for two empty strings that own no heap buffer.

</details>

## 03 · Optional references and fat pointers

<details><summary>Hint 1</summary>

A reference is never null. A checked lookup therefore returns `Option<&T>`, with `None` for "not there".

</details>
<details><summary>Hint 2</summary>

`get` returns `Option<&T>`. For the prefix, compute the end as the smaller of `n` and the length, then borrow a subslice with a range.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

References are never null, so a missing value is expressed as `Option<&T>`. Because the null address is unused, `Option<&T>` is the same size as `&T`. `&[T]` and `&str` are fat pointers: an address plus a length, two machine words, so a subslice can describe any region without copying. Borrowing a temporary directly in a `let`, as in `let r = &format!(..)`, extends that temporary's life to the end of the block. Returning such a borrow from a function is still rejected, because the temporary dies with the function. A reference never owns the value it points at, so dropping the reference drops nothing.

</details>

## 04 · Lifetimes through functions

<details><summary>Hint 1</summary>

Both results are parts of the input. The caller owns the input, so a reference into it stays valid after your function returns.

</details>
<details><summary>Hint 2</summary>

`split_once` returns two borrowed pieces of the original `str`. For the maximum, keep a reference to the best element so far and replace it only for a strictly greater value. (`Iterator::max` returns the *last* of several equal maxima.)

</details>
<details><summary>Check your explanation (after writing your own)</summary>

In `fn greatest<'a>(values: &'a [i32]) -> Option<&'a i32>`, the signature promises that the result points into `values`. The caller may therefore use the result only while that slice is still borrowed. The annotation describes a relationship that the compiler checks. It allocates nothing and cannot make an owner live longer. A function cannot return a reference to its own local variable, because the local is dropped at the end of the call; the two fixes are to return a borrow of an input or to return an owned value. Adding `'static` where it is not needed only forbids callers from passing ordinary borrowed data.

</details>

## 05 · Structs that borrow

<details><summary>Hint 1</summary>

The record stores two references into one line of text. It copies no text.

</details>
<details><summary>Hint 2</summary>

Use `split_once('=')`, trim both pieces, and store them in the struct. For B, return the field itself: it already is a `&'a str`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A struct that stores references needs a lifetime parameter, because the compiler must know how long the borrowed data lives in order to prevent a dangling field. `Record<'a>` says that both fields borrow data that stays valid for `'a`. Every `Record` built from one line is tied to the owner of that line, so the owner cannot be dropped or changed while the record is in use. Moving the record copies two fat pointers and leaves the source alone. A function like `value<'a>(record: &Record<'a>) -> &'a str` returns the field with the *source's* lifetime, so the result can still be used after the short borrow of the record has ended.

</details>

## 06 · Independent lifetimes and elision

<details><summary>Hint 1</summary>

Use separate lifetime parameters when a result depends on only one of two independently borrowed inputs.

</details>
<details><summary>Hint 2</summary>

`pair` only builds the struct from its two arguments. `left_text` returns the left field, which already carries the left lifetime.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The elision rules: each reference parameter gets its own lifetime; if there is exactly one input lifetime, every elided output lifetime uses it; if a method takes `&self` or `&mut self`, elided outputs borrow from `self`. In every other case an output reference must be annotated. When a struct's fields borrow from different owners, separate parameters (`Pair<'left, 'right>`) let the compiler track them independently, so `left_text` can return a `&'left str` that stays valid after the right-hand owner is dropped. With a single shared lifetime, the result would be limited by the owner that lives for the shorter time. Lifetimes exist only during type checking; at run time a `Pair` is just two fat pointers.

</details>

## 07 · Sharing versus mutation

<details><summary>Hint 1</summary>

Finish reading before you ask for exclusive access.

</details>
<details><summary>Hint 2</summary>

A: store the length in a variable, then append. B: make the clone first, so that it is an owned `String` and no reference to the first element is left, then push it.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A borrow lasts until its last use, not to the end of the block, so you can read a length, stop using the reference, and then mutate. While a shared borrow of an element is in use, the vector cannot be mutated: a push may reallocate the buffer and leave the reference dangling. The compiler enforces this without looking at the run-time capacity, because the rule must hold for every run of the program, not only for the runs where the capacity happens to be large enough. `&mut` means exclusive: no other reference to the same data can be used while it is in use. A reborrow `&mut *r` pauses `r`, which becomes usable again after the last use of the reborrow.

One detail: the one-line form `values.push(values[0].clone())` also compiles. The exclusive borrow that `push` takes for its receiver is a *two-phase borrow*: it is only reserved while the argument is computed, and it becomes active when `push` starts to run. By then the clone is an owned `String`. See *Drill topics* in [CONCEPTS.md](CONCEPTS.md).

</details>

## 08 · Disjoint mutable borrows

<details><summary>Hint 1</summary>

Use a library method that takes one `&mut` slice and gives back two `&mut` pieces which cannot overlap.

</details>
<details><summary>Hint 2</summary>

A: check `mid` against the length first (or use `split_at_mut_checked`), then loop over each half. B: reject equal and out-of-range indices first. Split at the *larger* index: the smaller index is then in the left half, and the larger one is element 0 of the right half. Return the two references in the order the caller asked for.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`split_at_mut(mid)` returns two exclusive slices, `[..mid]` and `[mid..]`. The function itself guarantees that they never overlap: it checks `mid <= len` at run time (and panics otherwise), and it builds the two slices with a small amount of `unsafe` code inside the standard library. The return type, two ordinary `&mut [T]`, does not prove this; the implementation does. Because the guarantee always holds, the safe signature is sound, callers need no `unsafe`, and the borrow checker lets both halves be used at once. Both halves borrow the original slice, so the original is unusable until both are finished.

The compiler rejects `&mut values[0]` together with `&mut values[1]` because the borrow checker does not track index values at all, not even constants. For a `Vec`, `v[i]` is a call to `index_mut(&mut self, i)` that borrows the whole vector; for arrays and slices, all elements count as one place, shown as `values[_]` in the error. Distinct struct or tuple fields are separate places that it does track. For a few elements at known positions, `get_disjoint_mut([i, j])` does the same job as task B in one call and returns `Err` instead of handing out overlapping references.

</details>

## 09 · Capstone: borrowed roster queries

<details><summary>Hint 1</summary>

A returned reference keeps the roster borrowed. An index borrows nothing, but nobody tells you when it stops being correct.

</details>
<details><summary>Hint 2</summary>

A: loop with `enumerate`, count scalars with `chars().count()`, and replace the best candidate only for a strictly greater count. B: `get` the `String`, then `trim` it; `trim` returns a subslice of the same buffer.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Returning a borrow (`Option<&str>`) is cheap and always valid, but it keeps the roster borrowed, so the caller cannot edit the roster while holding the result. Returning an index borrows nothing, so the roster can change, but the index silently becomes wrong when elements are removed, inserted or sorted, and the caller must check it again. Cloning gives the caller independent data at the cost of an allocation, and the copy does not see later edits. A stable ID stored inside each entry survives reordering; looking it up is slower than indexing, but it never points at the wrong element.

</details>

## R01 · Spaced review of Chapters 3 and 4

<details><summary>Hint 1</summary>

A: find the index first, through a shared borrow. Once you hold a plain `usize`, the borrow is over, and you may call a method that moves the `String` out and keeps the order. B: decide the overflow policy per element with a method that *reports* overflow. C: a byte limit can fall inside a multi-byte character.

</details>
<details><summary>Hint 2</summary>

A: track the best index and its `chars().count()` in a loop, replace it only for a strictly greater count, then call `names.remove(i)`. B: `counter.checked_add(amount)` returns `None` exactly when the sum does not fit; store 255 and count it. C: start at `max_bytes.min(text.len())` and step down while `!text.is_char_boundary(end)`, then return `&text[..end]`. The test for C runs under a time limit, so a loop that never moves `end` is reported instead of hanging.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one function panics on input that its documentation says it skips, and one matches more labels than it should. Every `.clone()` in this file is unnecessary, including the one that "works around" the borrow checker.

</details>
<details><summary>Hint 2</summary>

`config_value` can take `&[String]` and `&str` and return `Option<&str>` borrowed from the lines; use `if let Some((k, v)) = line.split_once('=')`. Rename with `for label in labels.iter_mut()`, compare with `==`, and reuse the buffer with `label.clear(); label.push_str(new)`. For `bump_to_end`, find the index with `position`, which ends the borrow, then `remove` and `push` the same `String`.

</details>
<details><summary>Check your review</summary>

`config_value` unwraps `split_once`, so any comment or blank line before the match causes a panic. It also clones every line, demands a `&String` key where any `&str` would do, and copies a value that it could borrow. `rename_all` uses `contains`, so "older" and "bold" are renamed too; it clones each label only to compare it, and it allocates a new `String` for each match. `bump_to_end` behaves correctly, but it clones the whole vector to avoid holding a borrow while mutating. Computing the index first ends the borrow, so no snapshot is needed. Clippy also reports the `&Vec<String>` and `&String` parameters and the index loop.

</details>

## Drills

Run `cargo drill ch05`. The compiler output tells you *what* happened; these notes explain *why*. Read a note only after you have seen the drill's verdict.

<details><summary>d01 · Writing through a shared reference</summary>

E0596. A `&String` promises the owner that nothing changes while it is borrowed. Take `text: &mut String`; the caller already passes `&mut greeting`.

</details>
<details><summary>d02 · Returning a reference to a local</summary>

E0515. `upper` is dropped when `shout` returns, so a reference to it would dangle. Return the owned `String` instead; comparing a `String` with a `&str` still works.

</details>
<details><summary>d03 · Holding a borrow across a push</summary>

E0502. `push` needs `&mut scores` while `first` (a shared borrow) is still used later, and a push can reallocate the buffer that `first` points into. Move the line that reads `*first` above the push: after its last use, the borrow no longer blocks the push. If you would rather copy the value out, keep `first` a reference so that the KEEP line still compiles: `let copy = scores[0]; let first = &copy;`.

</details>
<details><summary>d04 · Two mutable borrows into one vector</summary>

E0499. The borrow checker does not track index values, not even the constants 0 and 1: `pair[i]` on a `Vec` is a call to `index_mut(&mut self, i)`, which borrows the whole vector. Use `pair.swap(0, 1)`. When you need both references at once, use `let [a, b] = pair.get_disjoint_mut([0, 1]).unwrap();`, or `split_at_mut(1)` and take one element from each half.

</details>
<details><summary>d05 · A borrow ends at its last use</summary>

Compiles and runs (`ok`). `view` is used for the last time on the `len()` line, so the shared borrow is over before `push_str` asks for exclusive access. Nothing needs fixing.

</details>
<details><summary>d06 · A reference that outlives its owner</summary>

E0597. `name` is dropped at the end of the inner block while `label` is still used afterwards. Declare the owner in the outer scope, so that it lives as long as the reference is used.

</details>
<details><summary>d07 · Which input does the result borrow?</summary>

E0106. With two reference parameters and no `self`, no elision rule gives the output a lifetime. Write `fn longer<'a>(a: &'a str, b: &'a str) -> &'a str`: the result can be either input, so it is valid only while both are.

</details>
<details><summary>d08 · Moving an owner while it is borrowed</summary>

E0505. `consume(source)` moves the `String`, and the function frees it, while `view` still points into it. Finish using the view before the move, or pass `&source` to a function that only borrows.

</details>
<details><summary>d09 · Assigning to a borrowed value</summary>

E0506. Assigning to `limit` while `r` is still used later would change what a shared reference sees. Move the line that reads `*r` above the assignment. If you would rather keep the old value, borrow a copy instead of `limit` itself: `let old = limit; let r = &old;`.

</details>
<details><summary>d10 · Splitting past the end</summary>

Panics at run time. `split_at_mut` checks `mid <= len` when it runs, because `mid` is a run-time value. Valid split points are `0..=len`; here `values.len()` gives an empty right half.

</details>
<details><summary>d11 · Reading a vector inside its own push</summary>

Compiles and runs (`ok`). `steps.push(steps.len())` takes `&mut steps` for the call and also reads `steps` for the argument. The exclusive borrow of a method receiver is a *two-phase borrow*: it is only reserved while the arguments are computed, so shared reads are still allowed, and it becomes active when `push` starts. Nothing needs fixing. The rule does not help an exclusive reference that you created yourself: `let r = &mut steps; r.push(steps.len());` is E0502.

</details>
<details><summary>d12 · Passing `&mut`: lent again, or given away?</summary>

E0382. `bump` declares its parameter as `&mut i32`, so each call receives a reborrow, `&mut *r`, and `r` stays usable. `describe` takes a generic `T`. The compiler infers `T = &mut i32` and passes `r` itself, which is a move, because `&mut` references are not `Copy`. Write the reborrow yourself: `describe(&*r)` is enough, since `describe` only reads; `describe(&mut *r)` also works.

</details>
<details><summary>d13 · Find it, or add it and return the new one</summary>

E0499 (reported twice, for the `push` and for `last_mut`). `found` is returned on one path, and a returned reference must be valid for all of `'a`. The current borrow checker then treats `names` as borrowed for the rest of the function on every path, including the one where nothing was found. No reference can really dangle here; this is a known limit of the analysis. Search for an index first: `match names.iter().position(|n| n == name) { Some(i) => &mut names[i], None => { names.push(name.to_string()); names.last_mut().unwrap() } }`. An index borrows nothing, so the search is over before either branch borrows `names` again. For a map, `entry(key).or_insert(value)` does the whole job inside one borrow.

</details>

## Mastery check: model answers

Answer the questions in the [chapter guide](README.md#mastery-check) from memory first. Then compare.

<details><summary>Model answers (open only after writing your own)</summary>

1. **Re-pointing a reference versus changing the value.** With `let mut r = &a; r = &b;` the variable `r` now holds a different address; `a` and `b` are unchanged, and nothing can be written through `r`. With `let r = &mut a; *r = 5;` the value of `a` changes, and `r` still points at `a`. The first needs `mut` on the variable; the second needs `&mut` in the borrow (and `let mut a`).

2. **Automatic dereferencing, equality and identity, slices.** The `.` operator (method calls and field access) dereferences as often as needed, and formatting shows the value behind the reference. Assignment never dereferences for you: writing needs `*r = value`. `==` on references compares the values they point at; `std::ptr::eq` compares addresses and answers "is this the same object?". A `&[T]` or `&str` holds an address and a length, two machine words, so a subslice is a new pair pointing into the same storage.

3. **Returning references and what annotations do.** A local variable is dropped when its function returns, so a reference to it would point at freed memory; the compiler rejects it with E0515. A lifetime annotation states which input a returned reference points into. The compiler checks the function body against that statement and checks every caller against it. The annotation does not keep anything alive and produces no code.

4. **Elision rules and two lifetime parameters.** (1) Each reference parameter gets its own lifetime. (2) If there is exactly one input lifetime, all elided output lifetimes use it. (3) In a method with `&self` or `&mut self`, elided outputs use the lifetime of `self`. A struct needs two parameters when its fields borrow from unrelated owners and you want to hand one field back out after the other owner is gone, for example a view with a long-lived table and a short-lived query that returns matches from the table.

5. **The sharing rule and fixes without cloning a collection.** At any point a value may be reached through many shared references or through one exclusive reference, not both; a borrow lasts until its last use. Fixes: reorder so that the read finishes before the write; take out only what you need (copy a number, clone one element); remember an index or a length instead of a reference; collect the changes first and apply them afterwards.

6. **Two `&mut` into one collection.** The borrow checker does not track index values. `v[i]` on a `Vec` borrows the whole vector through `index_mut(&mut self, i)`, and all elements of a slice count as one place, so the second borrow overlaps the first (E0499). Library methods that return several exclusive references which cannot overlap solve it: `split_at_mut`, `split_first_mut`, `get_disjoint_mut`, `chunks_mut`, `iter_mut`; for a simple exchange, `swap`.

7. **Capstone redo.** There is no single answer. A good changed requirement is "the roster can be sorted while a label is selected". Then an index is no longer enough, because sorting moves elements; a stable ID stored in each entry stays correct, at the cost of a search or a map from ID to position.

</details>
