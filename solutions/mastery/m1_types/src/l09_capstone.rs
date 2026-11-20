//! LESSON 09 — Capstone: units of measure, checked by the compiler
//! Run (in mastery/m1_types/): cargo test --no-fail-fast l09_
//! Read first: "Lesson 09" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A length in metres and a length in kilometres are both one `f64`. What
//!    must differ so that adding them without a conversion does not compile?
//! 2. A length may be converted to another length, and never to a time. Where
//!    in the type system can "is a length" be written?
//! 3. Dividing a length by a time gives a speed. What is the TYPE of the
//!    unit of the result?
//!
//! IMPLEMENT the contract below from an empty file. Nothing is given: choose
//! the fields, the markers, the bounds and the impls. The programs in
//! `tests/cases/l09/` use only the names in the contract. This lesson uses
//! every earlier lesson of the module: marker types (06), `PhantomData` and
//! auto traits (04), a sealed trait (07), operator impls for foreign types
//! (08), and an associated type as a compile-time fact.
//! EXPLAIN afterward: For each of the five programs that must not compile,
//! name the trait bound or the type mismatch that stops it. Then list what a
//! `Quantity<Metres>` costs at run time compared with a plain `f64`.
//!
//! COMPILER LAB (optional): Add a unit `Minutes` and the dimension `Area` with
//! `Quantity<Metres> * Quantity<Metres>`. Note which impl would overlap if you
//! wrote the product for all pairs of units, and how you avoided it.
//!
//! CONTRACT — `Quantity<U>`: a number with a unit
//!
//! Dimensions (public types): `Length`, `Time`, and `Per<A, B>` for "A
//! divided by B".
//! Units (public types): `Metres`, `Kilometres` (dimension `Length`);
//! `Seconds`, `Hours` (dimension `Time`); and `Per<A, B>` for units `A` and
//! `B`, whose dimension is `Per<A::Dim, B::Dim>`.
//!
//! - `Unit`: a public, sealed trait with
//!   `type Dim;` (the unit's dimension),
//!   `const IN_BASE: f64;` (how many base units one of this unit is: 1.0 for
//!   metres and seconds, 1000.0 for kilometres, 3600.0 for hours, and
//!   `A::IN_BASE / B::IN_BASE` for `Per<A, B>`), and
//!   `fn symbol() -> String;` ("m", "km", "s", "h", and "m/s" for
//!   `Per<Metres, Seconds>`).
//! - Unit types are never instantiated: they are uninhabited, so
//!   `size_of::<Option<Metres>>()` is 0.
//! - `Quantity::<U>::new(value: f64)` and `value(self) -> f64`.
//! - `Quantity<U>` is 8 bytes, and `Copy`, `Send` and `Sync` for every unit.
//!   It is `PartialEq`, `PartialOrd` and `Debug`, comparing by value.
//! - `Display` prints the value and the symbol: `2000 m`, `1.5 km`, `72 km/h`.
//! - `+` and `-` work between two quantities of the SAME unit and give that
//!   unit. Nothing else can be added: not another unit, not a bare `f64`.
//! - `quantity * 2.0` and `2.0 * quantity` scale a quantity.
//! - `to::<V>()` converts to another unit `V` of the same dimension
//!   (1.5 km is 1500 m). Another dimension is a compile error.
//! - `Quantity<A> / Quantity<B>` gives `Quantity<Per<A, B>>`, and
//!   `Quantity<Per<A, B>> * Quantity<B>` gives `Quantity<A>` back.
//! - `Quantity<U>` implements `Sum`, for iterators of quantities and of
//!   references to quantities.

use std::cmp::Ordering;
use std::convert::Infallible;
use std::fmt;
use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::{Add, Div, Mul, Sub};

mod sealed {
    pub trait Sealed {}
}

/// A unit of measure. Sealed, so that this crate can add items to the trait
/// later without breaking an implementor in another crate.
pub trait Unit: sealed::Sealed {
    /// The dimension. Two units convert into each other exactly when their
    /// `Dim` types are equal.
    type Dim;
    /// How many base units one of this unit is.
    const IN_BASE: f64;
    fn symbol() -> String;
}

// Dimensions and units are empty enums: types without values. They exist
// only as type arguments.
pub enum Length {}
pub enum Time {}

/// Declares one unit: an uninhabited type and its `Unit` impl.
macro_rules! unit {
    ($name:ident, $dim:ty, $in_base:expr, $symbol:literal) => {
        pub enum $name {}

        impl sealed::Sealed for $name {}

        impl Unit for $name {
            type Dim = $dim;
            const IN_BASE: f64 = $in_base;
            fn symbol() -> String {
                $symbol.to_string()
            }
        }
    };
}

unit!(Metres, Length, 1.0, "m");
unit!(Kilometres, Length, 1000.0, "km");
unit!(Seconds, Time, 1.0, "s");
unit!(Hours, Time, 3600.0, "h");

/// "A divided by B", for dimensions and for units. The `Infallible` field
/// makes the type uninhabited; `fn() -> (A, B)` names both parameters without
/// owning them.
pub struct Per<A, B> {
    _never: Infallible,
    _marker: PhantomData<fn() -> (A, B)>,
}

impl<A, B> sealed::Sealed for Per<A, B> {}

impl<A: Unit, B: Unit> Unit for Per<A, B> {
    type Dim = Per<A::Dim, B::Dim>;
    const IN_BASE: f64 = A::IN_BASE / B::IN_BASE;
    fn symbol() -> String {
        format!("{}/{}", A::symbol(), B::symbol())
    }
}

/// A number with a unit. The unit is a type parameter and takes no space.
pub struct Quantity<U: Unit> {
    value: f64,
    // `fn() -> U`: no `U` is owned, so the quantity is `Send + Sync` and
    // covariant whatever `U` is.
    _unit: PhantomData<fn() -> U>,
}

impl<U: Unit> Quantity<U> {
    pub const fn new(value: f64) -> Self {
        Quantity {
            value,
            _unit: PhantomData,
        }
    }

    pub const fn value(self) -> f64 {
        self.value
    }

    /// Converts to another unit of the same dimension. The bound
    /// `Dim = U::Dim` is what rejects metres-to-seconds.
    pub fn to<V: Unit<Dim = U::Dim>>(self) -> Quantity<V> {
        Quantity::new(self.value * U::IN_BASE / V::IN_BASE)
    }
}

// Hand-written, because `#[derive]` would demand `U: Clone`, `U: PartialEq`
// and so on, and the unit types implement none of them.
impl<U: Unit> Clone for Quantity<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U: Unit> Copy for Quantity<U> {}

impl<U: Unit> PartialEq for Quantity<U> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<U: Unit> PartialOrd for Quantity<U> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<U: Unit> fmt::Debug for Quantity<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Quantity({} {})", self.value, U::symbol())
    }
}

impl<U: Unit> fmt::Display for Quantity<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.value, U::symbol())
    }
}

impl<U: Unit> Add for Quantity<U> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.value + rhs.value)
    }
}

impl<U: Unit> Sub for Quantity<U> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.value - rhs.value)
    }
}

impl<U: Unit> Mul<f64> for Quantity<U> {
    type Output = Self;
    fn mul(self, factor: f64) -> Self {
        Self::new(self.value * factor)
    }
}

// Foreign trait, foreign `Self` type (`f64`): allowed by the orphan rule
// because the local type `Quantity<U>` is a type argument of `Mul`.
impl<U: Unit> Mul<Quantity<U>> for f64 {
    type Output = Quantity<U>;
    fn mul(self, quantity: Quantity<U>) -> Quantity<U> {
        Quantity::new(self * quantity.value)
    }
}

impl<A: Unit, B: Unit> Div<Quantity<B>> for Quantity<A> {
    type Output = Quantity<Per<A, B>>;
    fn div(self, rhs: Quantity<B>) -> Self::Output {
        Quantity::new(self.value / rhs.value)
    }
}

// Does not overlap with `Mul<f64>`: the right-hand types differ.
impl<A: Unit, B: Unit> Mul<Quantity<B>> for Quantity<Per<A, B>> {
    type Output = Quantity<A>;
    fn mul(self, rhs: Quantity<B>) -> Quantity<A> {
        Quantity::new(self.value * rhs.value)
    }
}

impl<U: Unit> Sum for Quantity<U> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::new(iter.map(Quantity::value).sum())
    }
}

impl<'a, U: Unit> Sum<&'a Quantity<U>> for Quantity<U> {
    fn sum<I: Iterator<Item = &'a Quantity<U>>>(iter: I) -> Self {
        iter.copied().sum()
    }
}

#[cfg(test)]
mod tests {
    use crate::guess;
    use std::cmp::Ordering;
    use std::marker::PhantomData;

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. `{}` on an `f64`. Your `Display` impl prints the value this way.
        let p1: (&str, &str) = ("2000", "1.5");
        assert_eq!(
            (p1.0.to_string(), p1.1.to_string()),
            (format!("{}", 2000.0_f64), format!("{}", 1.5_f64)),
            "P1: how `{{}}` prints 2000.0 and 1.5 (left: your prediction, right: actual)"
        );

        // P2. In bytes: (a struct of an f64 and a PhantomData<fn() -> String>,
        // an Option of an empty enum).
        enum Never {}
        struct Tagged {
            _value: f64,
            _tag: PhantomData<fn() -> String>,
        }
        let p2: (usize, usize) = (8, 0);
        assert_eq!(
            p2,
            (size_of::<Tagged>(), size_of::<Option<Never>>()),
            "P2: what the markers cost (left: your prediction, right: actual)"
        );

        // P3. Why a quantity is `PartialOrd` and cannot be `Ord`: what does
        // `partial_cmp` return for (NaN and NaN, 1.0 and 2.0)?
        let p3: (Option<Ordering>, Option<Ordering>) = (None, Some(Ordering::Less));
        assert_eq!(
            p3,
            (f64::NAN.partial_cmp(&f64::NAN), 1.0_f64.partial_cmp(&2.0)),
            "P3: partial_cmp on floats (left: your prediction, right: actual)"
        );
    }
}
