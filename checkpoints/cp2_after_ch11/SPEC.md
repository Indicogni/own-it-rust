# Checkpoint 2 specification: `postage`

[All checkpoints](../README.md) · [Rubric](RUBRIC.md)

`postage` is a library for a small shop that sends parcels. It answers
three questions: Is this a valid parcel? What does each carrier charge for
it? Which carrier is the cheapest?

```text
a parcel:   1500 g, 300 x 200 x 100 mm, goods, not fragile
going to:   another city in the same country

Snail Mail  5.50
Dash        10.80
cheapest:   Snail Mail
```

This document is the contract. Where it says "must", a test checks it. The
public names and signatures are already in `src/lib.rs`, so that the tests
compile. Everything behind them is yours to design: the fields of every
struct, the order of the variants of `SizeClass`, which traits you derive
and which you write by hand, private helpers, and the files.

## 1. Rules that apply everywhere

1. **Units.** Weights are in grams, lengths in millimetres, volumes in
   cubic millimetres, liquids in millilitres, prices in cents. One
   millilitre is 1000 cubic millimetres.
2. **No function may panic** for any input, in a debug or in a release
   build. No function may print.
3. **No floating-point type** appears anywhere.
4. Only the standard library may be used. `unsafe` is not allowed.
5. **Modules.** The crate has three public modules with these contents:

   | module | public items |
   |---|---|
   | `postage::parcel` | `Grams`, `Dimensions`, `Contents`, `SizeClass`, `ParcelError`, `Parcel` |
   | `postage::carrier` | `Region`, `Zone`, `Carrier`, `Snail`, `Dash`, `Discounted` |
   | `postage::quote` | `Quote`, `cheapest`, `total` |

   The crate root re-exports every one of these items, so that
   `postage::Parcel` and `postage::parcel::Parcel` are the same type.
   `parcel` must not use anything from `carrier` or `quote`, and `carrier`
   must not use anything from `quote`.
6. **Validated values.** `Grams`, `Dimensions`, `Parcel`, `Dash` and
   `Discounted` have no public fields. Code outside their module can build
   them only through their `new` functions.

## 2. `Grams` and `Dimensions`

```text
impl Grams      { pub fn new(grams: u32) -> Grams;  pub fn get(self) -> u32 }
impl Dimensions { pub const MAX_SIDE_MM: u32 = 2000;
                  pub fn new(a: u32, b: u32, c: u32) -> Result<Dimensions, ParcelError>;
                  pub fn longest(&self) -> u32;  pub fn middle(&self) -> u32;
                  pub fn shortest(&self) -> u32; pub fn volume_mm3(&self) -> u64 }
```

`Grams` is a weight. Any `u32` is a valid `Grams`.

- `Grams::default()` is zero grams. `Grams` values compare like their
  numbers, and equal values have equal hashes.
- `Display` prints the number, a space and `g`: `250 g`.
- A `Grams` must be exactly as large as a `u32`.

`Dimensions` is a box with three sides.

- Every side must be in `1..=2000`. `Dimensions::new` checks `a`, then `b`,
  then `c`, and the first side that is out of range gives
  `Err(ParcelError::Side(that side))`.
- The order of the arguments does not matter for the result. `longest()`
  is the greatest side, `shortest()` the smallest, `middle()` the remaining
  one. Two `Dimensions` built from the same three numbers in any order are
  equal (`==`).
- `volume_mm3()` is the product of the three sides. The largest box has a
  volume of 8 000 000 000 mm³.
- A `Dimensions` must be at most 12 bytes large.

## 3. `Parcel`

```text
pub enum Contents  { Documents, Goods { fragile: bool }, Liquid { millilitres: u32 } }
pub enum SizeClass { Letter, Small, Medium, Large }        // in any order you like
pub enum ParcelError { Side(u32), Weight(Grams), Liquid(u32) }

impl Parcel { pub const MAX_WEIGHT_G: u32 = 30_000;
              pub fn new(weight: Grams, dimensions: Dimensions, contents: Contents)
                  -> Result<Parcel, ParcelError>;
              pub fn weight(&self) -> Grams;  pub fn dimensions(&self) -> Dimensions;
              pub fn contents(&self) -> Contents;
              pub fn billable(&self) -> Grams;  pub fn size_class(&self) -> SizeClass }
```

### 3.1 `Parcel::new`

The checks run in this order, and the first one that fails decides.

1. The weight must be in `1..=30_000` grams. Otherwise:
   `Err(ParcelError::Weight(weight))`.
2. If the contents are `Liquid { millilitres }`, the liquid must fit in the
   box: its volume must not be greater than `dimensions.volume_mm3()`.
   Otherwise: `Err(ParcelError::Liquid(millilitres))`. Every `u32` may be
   given as `millilitres`.

`weight()`, `dimensions()` and `contents()` return what was given. Two
parcels are equal when their weight, dimensions and contents are equal.
A `Parcel` must be at most 24 bytes large.

### 3.2 `billable`

Carriers charge for space as well as for weight. The *volumetric weight* of
a parcel is its volume divided by 5000 mm³ per gram, rounded **up** to a
whole gram. `billable()` is the weight or the volumetric weight, whichever
is greater.

| weight | box | volume | volumetric weight | billable |
|---|---|---|---|---|
| 1500 g | 300 × 200 × 100 | 6 000 000 mm³ | 1200 g | 1500 g |
| 8000 g | 600 × 400 × 300 | 72 000 000 mm³ | 14 400 g | 14 400 g |
| 1 g | 1667 × 3 × 1 | 5001 mm³ | 2 g | 2 g |

### 3.3 `size_class`

Read the table from the top. The first row whose conditions all hold gives
the class. The three limits of a row apply to the longest, the middle and
the shortest side, in that order.

| class | contents | weight | longest | middle | shortest |
|---|---|---|---|---|---|
| `Letter` | `Documents` only | ≤ 100 g | ≤ 240 | ≤ 165 | ≤ 5 |
| `Small` | any | ≤ 2000 g | ≤ 450 | ≤ 350 | ≤ 160 |
| `Medium` | any | ≤ 10 000 g | ≤ 800 | ≤ 500 | ≤ 400 |
| `Large` | any | any | any | any | any |

The real weight counts here, not the billable weight.

`SizeClass` values are ordered by size:
`Letter < Small < Medium < Large`.

### 3.4 `ParcelError`

`ParcelError` implements `std::error::Error` and has no source. `Display`
prints exactly:

| value | text |
|---|---|
| `ParcelError::Side(0)` | `side of 0 mm is outside 1..=2000 mm` |
| `ParcelError::Weight(Grams::new(30001))` | `weight of 30001 g is outside 1..=30000 g` |
| `ParcelError::Liquid(750)` | `750 ml of liquid do not fit in the parcel` |

## 4. `Zone` and the `Carrier` trait

```text
pub enum Region { Near, Far }
pub enum Zone   { Local, National, International(Region) }

pub trait Carrier {
    fn name(&self) -> &str;
    fn price(&self, parcel: &Parcel, zone: Zone) -> Option<u32>;
    fn accepts(&self, parcel: &Parcel, zone: Zone) -> bool { /* provided */ }
}
```

- `price` returns the price in cents, or `None` when the carrier does not
  take this parcel to this zone.
- `accepts` has a provided body: a carrier accepts exactly what it has a
  price for. A carrier may override it with something faster.
- The trait must work as a trait object: `Box<dyn Carrier>` and
  `&dyn Carrier` are used throughout.
- `Box<C>` is a carrier whenever `C` is one, also for `C = dyn Carrier`.
  **A wrapper forwards all three methods** to the carrier inside, so that
  an overridden `accepts` of that carrier is the one that runs. This rule
  holds for `Box<C>` and for `Discounted<C>` (section 6).

## 5. The two built-in carriers

### 5.1 `Snail`

`Snail` is a unit struct. Its name is `Snail Mail`. Its prices in cents:

| size class | `Local` | `National` | `International(Near)` | `International(Far)` |
|---|---|---|---|---|
| `Letter` | 90 | 110 | 250 | 400 |
| `Small` | 400 | 550 | 1400 | 2600 |
| `Medium` | 700 | 950 | 3100 | declined |
| `Large` | 1500 | 1900 | declined | declined |

Two more rules:

- `Liquid` contents are declined for both international zones.
- `Goods { fragile: true }` cost 200 cents more than the table says. The
  surcharge never turns a declined parcel into an accepted one.

### 5.2 `Dash`

```text
impl Dash { pub fn new(name: &str, base: u32, per_kg: u32) -> Dash }
```

`Dash` is a courier that charges by weight. Its name is the `name` it was
built with; it keeps its own copy.

1. `International(Far)` is declined. `Liquid` contents are declined
   everywhere except `Local`.
2. *Kilograms* is the billable weight, rounded up to whole kilograms:
   1000 g is 1, 1001 g is 2.
3. The price is `(base + per_kg × kilograms) × factor`, where the factor is
   1 for `Local`, 2 for `National` and 5 for `International(Near)`.
4. If the price, or any step on the way to it, does not fit in a `u32`,
   the parcel is declined.

Example: `Dash::new("Dash", 300, 120)` charges `(300 + 120 × 2) × 2 = 1080`
for the 1500 g parcel of section 3.2 to `National`.

## 6. `Discounted`

```text
impl<C> Discounted<C> { pub fn new(inner: C, percent: u8) -> Option<Discounted<C>>;
                        pub fn inner(&self) -> &C;  pub fn into_inner(self) -> C }
```

`Discounted<C>` wraps any carrier and takes a percentage off its prices.

- `new` returns `None` when `percent` is greater than 100.
- `name()` is the name of the carrier inside (the same `&str`, not a copy).
- `price()` is `None` when the carrier inside declines. Otherwise it is
  the inner price × (100 − percent) / 100, rounded **up** to a whole cent.
  85 % of 110 is 93.5 and becomes 94. Every inner price up to `u32::MAX`
  must give the exact result.
- `accepts()` asks the carrier inside (section 4).
- `inner()` borrows the carrier inside. `into_inner()` gives it back: the
  same value that was passed to `new`.
- `Discounted<C>` is a carrier for every carrier `C`, including
  `Box<dyn Carrier>` and another `Discounted`.

## 7. `cheapest` and `total`

```text
pub struct Quote<'c> { pub carrier: &'c dyn Carrier, pub cents: u32 }

pub fn cheapest<'c>(carriers: &'c [Box<dyn Carrier>], parcel: &Parcel, zone: Zone)
    -> Option<Quote<'c>>
pub fn total<C: Carrier + ?Sized>(carrier: &C, parcels: &[Parcel], zone: Zone) -> Option<u64>
```

`cheapest` returns the carrier with the lowest price for this parcel and
zone, with that price.

- Carriers that decline are skipped. If every carrier declines, or the
  slice is empty, the result is `None`.
- Among equal prices the carrier that comes first in the slice wins.
- `Quote::carrier` must refer to the carrier in the slice itself.
- `cheapest` asks each carrier for its price exactly once.

`total` returns the sum of the prices of all parcels with one carrier, or
`None` when the carrier declines at least one of them. The total of no
parcels is `Some(0)`. The sum must be exact for any number of prices up to
`u32::MAX` each that fits in a `u64`.

## 8. Cost

None of the following may allocate memory: `Dimensions::new`,
`Parcel::new`, every method of `Dimensions` and `Parcel`, `name`, `price`
and `accepts` of `Snail`, `Dash` and `Discounted`, `cheapest`, and `total`.
(`Dash::new` allocates, for its name.)

## 9. What is not specified

Anything this document does not mention is your decision. You may add
derives, trait implementations, private items and files. You may replace a
trait implementation whose body is `todo!()` by a derive. You may not
remove or change a public name, a public field, or a signature.
