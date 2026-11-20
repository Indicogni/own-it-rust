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

// Your code goes here.

#[cfg(test)]
mod tests {
    use crate::guess;
    use std::cmp::Ordering;
    use std::marker::PhantomData;

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. `{}` on an `f64`. Your `Display` impl prints the value this way.
        let p1: (&str, &str) = guess();
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
        let p2: (usize, usize) = guess();
        assert_eq!(
            p2,
            (size_of::<Tagged>(), size_of::<Option<Never>>()),
            "P2: what the markers cost (left: your prediction, right: actual)"
        );

        // P3. Why a quantity is `PartialOrd` and cannot be `Ord`: what does
        // `partial_cmp` return for (NaN and NaN, 1.0 and 2.0)?
        let p3: (Option<Ordering>, Option<Ordering>) = guess();
        assert_eq!(
            p3,
            (f64::NAN.partial_cmp(&f64::NAN), 1.0_f64.partial_cmp(&2.0)),
            "P3: partial_cmp on floats (left: your prediction, right: actual)"
        );
    }
}
