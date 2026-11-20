# Capstone "nostd_collections": design hints

These are hints about design and about where to look. They contain no
code. Each milestone has two levels. Open the first when you do not know
how to start; open the second when you have tried and are stuck.

A general hint first: **write the invariant before the code.** For each
type, write two or three sentences that say which slots (or bytes) hold
live values, as a function of the fields. Every function may assume the
invariant when it starts and must restore it before it returns *or
panics*. Almost every bug in this capstone is a moment where the invariant
does not hold and something can still observe the type: a closure of the
caller runs, a destructor runs, or a panic unwinds through your function.

A second one: **use safe code wherever it does the job.** An array of
`MaybeUninit<T>` can be indexed, swapped, split and replaced without
`unsafe`. Only two kinds of step need it: reading a slot as a `T`, and
moving several slots at once. The fewer unsafe blocks you have, the fewer
proofs you owe.

## M1 · The core of `ArrayVec`

<details>
<summary>Hint 1: fields and the first functions</summary>

Two fields are enough: the storage and the length. The invariant is one
sentence about the slots below the length.

`new` must be a `const fn` and must work for a `T` that is neither `Copy`
nor `Default`. Look up how an array of uninitialised slots can be written
in a constant context; the documentation of `MaybeUninit` shows it under
"Initializing an array element-by-element".

Write `try_push` and `pop` first, then the slice view, and express as much
as you can through them. `push` is `try_push` plus a panic. For the slice
view you turn "the first `len` slots" into `&[T]`; `MaybeUninit` has
methods for slices of slots, and a pointer cast with
`core::slice::from_raw_parts` is the classic way. Once `Deref` works, half
of the tests of this milestone pass without more work.

</details>

<details>
<summary>Hint 2: Drop, truncate and the panicking destructor</summary>

Without a `Drop` implementation the compiler drops nothing: `MaybeUninit`
never runs a destructor. The ledger tests then report leaks.

For dropping several elements, do not write a loop. Build a raw slice
pointer to the elements and drop it in place with one call. That function
drops the remaining elements even when one destructor panics, which is
exactly what SPEC 4.3 asks for; a loop of your own stops at the panic.

The order of two lines decides whether a panicking destructor leads to a
double drop: "lower the length" and "drop the elements". Ask yourself
which elements the vector claims at the moment the destructor panics, and
what the vector's own `Drop` will do with them later.

Zero-sized types need no special code if you never do arithmetic on
addresses yourself. Check that nothing in your code divides by
`size_of::<T>()`.

</details>

## M2 · All of `ArrayVec`

<details>
<summary>Hint 1: moving elements, and which checks come first</summary>

`insert` and `remove` move a block of slots by one place. The source and
the destination overlap, so you need the copy function that allows
overlap. Decide for each function when the length changes, and make sure
that nothing between the move and the update of the length can panic.

`swap_remove` can be written without any block move.

Iteration by value needs a type that owns the storage and remembers which
part has not been yielded. Think about how the storage gets from the
vector into the iterator without the vector dropping the elements on the
way; there is a way that needs no unsafe code. The iterator needs its own
`Drop`.

`From<[T; N]>` and `into_inner` convert between an array of values and an
array of slots. One direction is safe code.

`Clone`, `Extend` and `FromIterator` are a few lines each when they are
written with `push`, and then they are panic-safe for free: if a `clone`
or an iterator panics, the partly built vector is an ordinary local
variable, and unwinding drops it.

</details>

<details>
<summary>Hint 2: retain and drain</summary>

Both functions run code of the caller, or destructors, while the vector is
in pieces. The technique for both is the same and has two parts.

*First, make the vector claim less than it owns.* Before the caller's code
runs, set the length to a value that covers only elements you will not
touch. For `drain`, that is the start of the range. If everything goes
wrong (the iterator is leaked), the vector is short but sound. This is
called leak amplification.

*Second, put the repair into a destructor.* A small private struct holds
the mutable borrow of the vector and a few counters (how many elements
were processed, how many were removed, where the tail is). Its `Drop`
moves the surviving elements into place and sets the final length. It runs
on normal exit and on unwinding, so you write the repair once.

For `retain`: update the counters *before* you drop a rejected element,
so that a panicking destructor cannot make you count it twice. To get the
state that SPEC 4.5 demands after a panic of the closure, treat the
element that the closure was looking at as "not yet processed".

For `drain`: the iterator type holds `&mut ArrayVec`. That borrow is what
makes the compile-fail test of SPEC 7.5 pass. In its `Drop`, two things
happen: the rest of the range is dropped, and the tail moves down. The
second must happen even if the first panics. Write down the invariant of
the iterator type as well; it has five numbers in it.

The range argument: `RangeBounds` gives you a start bound and an end
bound, each of which is included, excluded or unbounded. Adding 1 to an
included end can overflow.

</details>

## M3 · The core of `RingBuffer`

<details>
<summary>Hint 1: representation</summary>

SPEC 5.1 gives the storage model. You still choose how to represent it.
Two common choices are "slot of the front, and length" and "slot of the
front, and slot behind the back". Try to write `is_empty`, `is_full` and
`len` for both before you decide: with one of them, an empty buffer and a
full buffer look the same unless you give up a slot or add a flag.

Write one private function that turns "the i-th element from the front"
into a slot number, and use it everywhere. All wrap-around logic of the
type then lives in one place.

</details>

<details>
<summary>Hint 2: the wrap-around arithmetic and the two slices</summary>

The obvious formula for the slot uses the remainder operator. Check it
against two cases of SPEC 5.1: `N == 0`, and a zero-sized element type
with `N` close to `usize::MAX`, where a sum of two slot numbers does not
fit into `usize`. There is a formulation with one comparison and one
subtraction that has neither problem.

`as_slices` is two ranges of slots; compute the ranges in one private
function and use it for the shared and the mutable version. For the
mutable version you need two non-overlapping mutable slices of one array.
The standard library can split a slice at an index; think about which
index separates the two parts.

Dropping: the contents are two runs of slots. Drop the run at the front
first. The second run must still be dropped if a destructor in the first
one panics; the tool is again a small struct with a `Drop`. As in
`ArrayVec`, the order of "mark the buffer empty" and "run the destructors"
matters.

`push_overwrite` is easiest as a combination of functions you already
have. Handle `N == 0` first.

</details>

## M4 · `RingBuffer` iterators and traits

<details>
<summary>Hint 1: an iterator from two slices</summary>

You already have the contents as two slices, and slices have iterators
that are double-ended and know their length. Your iterator can be a pair
of them. `next` takes from the first until it is empty; where does
`next_back` take from? The length is a sum that cannot overflow here;
write down why (this is the reason `Chain` cannot promise it in general
and you can).

The mutable iterator is the same idea with the mutable slice iterators.
The iterator by value can own the whole buffer and pop from it.

If you write all three this way, this milestone needs no unsafe code.

</details>

<details>
<summary>Hint 2: Clone, equality, and a derive that is too strict</summary>

`#[derive(Clone)]` on an iterator over `&T` adds the bound `T: Clone`,
which SPEC 5.5 does not allow. Write the implementation by hand.

Equality must not compare slots, and it must not compare the two slices
pair-wise either: two equal buffers can be split at different places.
Compare element sequences.

For `Debug`, look at what `Formatter` offers for printing a list from an
iterator. It handles `{:#?}` for you.

</details>

## M5 · `Arena`

<details>
<summary>Hint 1: fields and interior mutability</summary>

`alloc` takes `&self` and still has to change two things: the cursor and
the bytes. Both need interior mutability. For the cursor, a `Cell` is
enough. For the bytes, the only sound basis for writing through a shared
reference is `UnsafeCell`.

The storage is bytes that start uninitialised and that may stay partly
uninitialised (padding inside the values you store). Which element type
of the byte array expresses that?

SPEC 6.2 requires offset 0 to be at a multiple of 16. An attribute on a
wrapper struct around the storage does that. Check with the test
`placement_does_not_depend_on_where_the_arena_is`.

Write one private function that takes a `core::alloc::Layout` and returns
a pointer to reserved room, or nothing. Every public function is then a
thin layer over it. Use the checked arithmetic of the standard library in
that function; `Layout` also has a constructor for arrays that reports an
overflow of the size.

</details>

<details>
<summary>Hint 2: what Miri will tell you</summary>

The tests of this milestone pass with several designs that are not sound.
Miri separates them.

*Never create a reference to the storage as a whole.* If any function
makes a `&mut` (or a `&`) to the whole byte array while references to
earlier allocations are alive, those earlier references are invalid from
then on, although the addresses do not overlap. Stay with raw pointers
from the `UnsafeCell` until you have the pointer to the one value, and
create the reference to that value only.

*Derive every pointer from the storage pointer.* A pointer that you
rebuild from an integer address has lost the permission to access the
storage. Compute an offset as an integer if you like, but then apply it to
the storage pointer.

*Zero-sized values* need an address that is aligned and not null, and
nothing else. They must not read or move the cursor.

*Alignments above 16*: the offset alone no longer tells you whether an
address is aligned. You need the real address of the storage, as an
integer, for the padding computation only.

`alloc_slice_fill_with`: the caller's closure can call `alloc` on the same
arena. Decide where the cursor must be at that moment. The same closure
can panic; decide what the arena looks like then, and check that no
reference to a partly filled slice can exist.

The soundness argument that SPEC 6.3 asks for has four parts: why two
returned references never overlap; why a reference cannot outlive the
bytes; why `reset` cannot run while a reference is alive; and why two
threads cannot allocate at the same time. Each part is one sentence that
points to a field type or to a signature.

</details>

## M6 · Contracts and evidence

<details>
<summary>Hint 1: sizes, auto traits, variance</summary>

If you used arrays of `MaybeUninit<T>`, `Cell` and `UnsafeCell` as fields
and no raw pointers, all tests of `m6_contracts` should already pass: the
compiler derives `Send`, `Sync` and the variance from the field types.

If a test fails here, look for a raw pointer or a `PhantomData` among your
fields. A `*mut T` field makes a type invariant in `T` and removes `Send`
and `Sync`. Ask whether the field is needed at all before you add an
`unsafe impl`.

If the size test fails, something in the type costs memory per slot or
duplicates a field.

</details>

<details>
<summary>Hint 2: getting the most out of Miri</summary>

Miri runs one particular execution. Give it more to look at:

- run it with the second aliasing model (`-Zmiri-tree-borrows`); a
  finding under either model deserves a look;
- run it with several seeds, which changes addresses and therefore the
  padding in the arena;
- write a test of your own for each unsafe block that the given tests
  reach only on one path. A good source of cases is section 13 of the
  specification: for each place where a panic can happen, write the test
  that makes it happen.

A leak report from Miri names the allocation and the line that made it.
Follow that value through your code: which function took ownership of it
last?

</details>
