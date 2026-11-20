//! Checkpoint 2: `postage`, parcels and the carriers that price them.
//!
//! Read `SPEC.md` first. It is the contract; this file only holds the public
//! names that the tests call, so that the tests compile. Everything else is
//! your design: the fields of every struct, the order of enum variants, which
//! traits you derive and which you write by hand, private helpers, and how
//! the code is spread over files.
//!
//! The three modules are written inline here to keep the starter in one
//! file. You may move each one to its own file. The public paths must stay
//! the same: `postage::parcel::Parcel`, `postage::Parcel`, and so on.
//!
//! Rules of the checkpoint (see `../README.md`): closed notes, one sitting,
//! 2 to 4 hours. Do not change the files in `tests/`. Do not change a public
//! name or signature. A struct marked "your design" has a placeholder field;
//! replace it. Trait implementations whose body is `todo!()` may be replaced
//! by a derive.

// Starter bodies are `todo!()`, so parameters and fields are unused for now.
// Delete this line when you have implemented everything: the compiler's
// warnings are part of your feedback.
#![allow(dead_code, unused_variables)]

pub use carrier::{Carrier, Dash, Discounted, Region, Snail, Zone};
pub use parcel::{Contents, Dimensions, Grams, Parcel, ParcelError, SizeClass};
pub use quote::{Quote, cheapest, total};

/// What is being sent. SPEC sections 2 and 3.
pub mod parcel {
    use std::cmp::Ordering;
    use std::error::Error;
    use std::fmt;

    /// A weight in grams. The representation is your design.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct Grams {
        _replace_me: (),
    }

    impl Grams {
        pub fn new(grams: u32) -> Grams {
            todo!()
        }

        pub fn get(self) -> u32 {
            todo!()
        }
    }

    impl fmt::Display for Grams {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            todo!()
        }
    }

    /// The three sides of a box in millimetres. The fields are your design.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Dimensions {
        _replace_me: (),
    }

    impl Dimensions {
        pub const MAX_SIDE_MM: u32 = 2000;

        pub fn new(a: u32, b: u32, c: u32) -> Result<Dimensions, ParcelError> {
            todo!()
        }

        pub fn longest(&self) -> u32 {
            todo!()
        }

        pub fn middle(&self) -> u32 {
            todo!()
        }

        pub fn shortest(&self) -> u32 {
            todo!()
        }

        pub fn volume_mm3(&self) -> u64 {
            todo!()
        }
    }

    /// What is inside.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Contents {
        Documents,
        Goods { fragile: bool },
        Liquid { millilitres: u32 },
    }

    /// The four size classes. They are listed alphabetically here; the order
    /// in which you write them is your decision.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SizeClass {
        Large,
        Letter,
        Medium,
        Small,
    }

    impl PartialOrd for SizeClass {
        fn partial_cmp(&self, other: &SizeClass) -> Option<Ordering> {
            todo!()
        }
    }

    impl Ord for SizeClass {
        fn cmp(&self, other: &SizeClass) -> Ordering {
            todo!()
        }
    }

    /// Why a parcel cannot exist.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ParcelError {
        Side(u32),
        Weight(Grams),
        Liquid(u32),
    }

    impl fmt::Display for ParcelError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            todo!()
        }
    }

    impl Error for ParcelError {}

    /// A parcel that passed every check. The fields are your design.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Parcel {
        _replace_me: (),
    }

    impl Parcel {
        pub const MAX_WEIGHT_G: u32 = 30_000;

        pub fn new(
            weight: Grams,
            dimensions: Dimensions,
            contents: Contents,
        ) -> Result<Parcel, ParcelError> {
            todo!()
        }

        pub fn weight(&self) -> Grams {
            todo!()
        }

        pub fn dimensions(&self) -> Dimensions {
            todo!()
        }

        pub fn contents(&self) -> Contents {
            todo!()
        }

        pub fn billable(&self) -> Grams {
            todo!()
        }

        pub fn size_class(&self) -> SizeClass {
            todo!()
        }
    }
}

/// Who can send a parcel, and for how much. SPEC sections 4 to 6.
pub mod carrier {
    use crate::parcel::Parcel;

    /// How far away another country is.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Region {
        Near,
        Far,
    }

    /// Where a parcel goes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Zone {
        Local,
        National,
        International(Region),
    }

    /// A company that carries parcels.
    pub trait Carrier {
        fn name(&self) -> &str;

        fn price(&self, parcel: &Parcel, zone: Zone) -> Option<u32>;

        fn accepts(&self, parcel: &Parcel, zone: Zone) -> bool {
            todo!()
        }
    }

    impl<C: Carrier + ?Sized> Carrier for Box<C> {
        fn name(&self) -> &str {
            todo!()
        }

        fn price(&self, parcel: &Parcel, zone: Zone) -> Option<u32> {
            todo!()
        }
    }

    /// The national post.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Snail;

    impl Carrier for Snail {
        fn name(&self) -> &str {
            todo!()
        }

        fn price(&self, parcel: &Parcel, zone: Zone) -> Option<u32> {
            todo!()
        }
    }

    /// A courier that charges by weight. The fields are your design.
    #[derive(Debug)]
    pub struct Dash {
        _replace_me: (),
    }

    impl Dash {
        pub fn new(name: &str, base: u32, per_kg: u32) -> Dash {
            todo!()
        }
    }

    impl Carrier for Dash {
        fn name(&self) -> &str {
            todo!()
        }

        fn price(&self, parcel: &Parcel, zone: Zone) -> Option<u32> {
            todo!()
        }
    }

    /// Any carrier, with a percentage taken off its prices. The fields are
    /// your design.
    #[derive(Debug)]
    pub struct Discounted<C> {
        _replace_me: C,
    }

    impl<C> Discounted<C> {
        pub fn new(inner: C, percent: u8) -> Option<Discounted<C>> {
            todo!()
        }

        pub fn inner(&self) -> &C {
            todo!()
        }

        pub fn into_inner(self) -> C {
            todo!()
        }
    }

    impl<C: Carrier> Carrier for Discounted<C> {
        fn name(&self) -> &str {
            todo!()
        }

        fn price(&self, parcel: &Parcel, zone: Zone) -> Option<u32> {
            todo!()
        }
    }
}

/// Comparing carriers. SPEC section 7.
pub mod quote {
    use crate::carrier::{Carrier, Zone};
    use crate::parcel::Parcel;

    /// A carrier and the price it asked.
    #[derive(Clone, Copy)]
    pub struct Quote<'c> {
        pub carrier: &'c dyn Carrier,
        pub cents: u32,
    }

    pub fn cheapest<'c>(
        carriers: &'c [Box<dyn Carrier>],
        parcel: &Parcel,
        zone: Zone,
    ) -> Option<Quote<'c>> {
        todo!()
    }

    pub fn total<C: Carrier + ?Sized>(carrier: &C, parcels: &[Parcel], zone: Zone) -> Option<u64> {
        todo!()
    }
}
