# Checkpoint 2 reference solution: commentary

The solution is the `src/` directory next to this file: `lib.rs`,
`parcel.rs`, `carrier.rs` and `quote.rs`. Read it after your own sitting,
next to your code. This note explains the decisions that are not obvious
from the code, and what a weaker solution usually does instead.

## Three files, dependencies in one direction

`lib.rs` contains only `pub mod` lines, `pub use` lines and the crate
documentation. `parcel` knows nothing about prices. `carrier` uses
`parcel`. `quote` uses both. Nothing points backwards, so each file can be
read and tested without the ones below it.

The re-exports in `lib.rs` mean that a caller writes `postage::Parcel`. The
module path `postage::parcel::Parcel` still works, and both name one type.

## Choose the representation so that `derive` is right

No comparison in the solution is written by hand.

- `Grams(u32)` derives `PartialOrd` and `Ord`. A tuple struct with one
  field compares like the field.
- `Dimensions` stores `[u32; 3]`, sorted from longest to shortest in `new`.
  The invariant is in the doc comment. Because every box has exactly one
  stored form, the derived `==` already treats a box turned on its side as
  the same box. A solution that stores the sides in argument order needs a
  hand-written `PartialEq`, and then a hand-written `Hash` that agrees with
  it.
- `SizeClass` lists its variants from `Letter` to `Large`. The derived
  order of an enum is the order of its variants. The starter lists them
  alphabetically, and deriving on that order gives `Large < Letter`.
- `Parcel` stores three facts and derives everything.

The array has a second use. `[u32; 3]` can be matched with an array
pattern.

## A `Parcel` stores facts only

`billable` and `size_class` are computed when asked. Storing them would
make `Parcel` larger than the 24 bytes the specification allows, and a
stored value can go out of date with the facts it was computed from. Both
computations are a few integer operations.

The size is easy to count: 4 bytes for `Grams`, 12 for `Dimensions`, and 8
for `Contents` (a `u32` payload, a tag, and padding up to the alignment of
4).

## Tables become `match` expressions

`size_class` matches on one tuple, `(contents, grams, sides)`:

```text
(Contents::Documents, ..=100, [..=240, ..=165, ..=5]) => SizeClass::Letter,
(_, ..=2_000, [..=450, ..=350, ..=160])               => SizeClass::Small,
```

Each arm is one row of the table in the specification, in the same order,
and the first arm that matches wins. You can check the code against the
table by reading them side by side. A chain of `if` and `else if` with `&&`
does the same work but hides the table.

`Snail::price` matches on `(size_class, zone)`. It has no wildcard arm for
the prices: every pair is listed, and the declined pairs are one arm with
`|`. If somebody adds a fifth size class later, the compiler reports every
pair that is missing.

## Arithmetic: widen first, then multiply

Three places would overflow a `u32`:

- the volume, `2000 × 2000 × 2000`;
- the liquid check, `millilitres × 1000`;
- the discount, `price × (100 − percent)`.

In each one the operands are converted with `u64::from` *before* the
multiplication. `u64::from(a * b)` is too late: the product has already
wrapped around or panicked.

`Dash::price` is different. There an overflow has a meaning in the
specification: the carrier declines. So it stays in `u32` and uses
`checked_mul` and `checked_add` with `?`. The `?` operator works on
`Option` in a function that returns `Option`.

Rounding up uses `div_ceil`. The old idiom `(a + b - 1) / b` can overflow
in the addition.

## The trait and its wrappers

`Carrier` has two required methods and one provided method. The provided
`accepts` calls `price`. That has two consequences.

**`cheapest` calls `price` only.** `accepts` followed by `price` computes
every price twice. `price` already returns an `Option`, and `let ... else {
continue }` skips the carriers that decline.

**A wrapper must forward `accepts` as well.** `impl Carrier for Box<C>`
compiles with only `name` and `price`. But then `accepts` on the box is the
provided body, which calls the box's `price`. If the carrier inside
overrides `accepts` (for example with a fast check), the override is never
used. The solution forwards all three methods in `Box<C>` and in
`Discounted<C>`. The same rule applies whenever you implement a trait for a
wrapper type: look at the provided methods too.

The implementation for `Box<C>` has the bound `C: Carrier + ?Sized`. Type
parameters are `Sized` unless you say otherwise, and `dyn Carrier` is not
`Sized`. Without `?Sized` the implementation would cover `Box<Snail>` but
not `Box<dyn Carrier>`. `total` has the same bound for the same reason.

`Discounted<C>` has no bound on the struct, and none on the `impl` block
with `new`, `inner` and `into_inner`. Only `impl Carrier for Discounted<C>`
needs `C: Carrier`. Putting bounds only where they are used keeps the type
usable in more places and keeps the error messages short.

## `cheapest`

The best quote so far is one `Option<Quote>`. No list of quotes is built,
so nothing is allocated. The comparison is `cents < quote.cents`, strictly
less, so an equal price later in the slice does not replace an earlier one.
`Iterator::min_by_key` would also work, because it returns the first
minimum. The loop was chosen because the tie rule is visible in it.

`Quote` holds `&'c dyn Carrier`, borrowed from the slice with
`carrier.as_ref()`. The lifetime `'c` is the lifetime of the slice, not of
the parcel: the signature says so by naming `'c` only on `carriers` and on
the result.

Writing `Quote { carrier, cents }` without `as_ref()` also compiles, and
the tests accept it. It means something else, though. `carrier` is a
`&Box<dyn Carrier>`, and `Box<dyn Carrier>` is a carrier itself, so the
compiler makes a trait object whose concrete type is the box. Every call
then goes through two pointers: first to the box in the slice, then to the
carrier inside it. `as_ref()` (or `&**carrier`) borrows the carrier inside
directly.

## Tests in the solution

Each module ends with a few unit tests of its own: the stored form of
`Dimensions`, rounding in `billable`, the checked arithmetic of `Dash`, and
the tie rule of `cheapest`. Tests inside a module can see private fields,
so they can check an invariant directly.
