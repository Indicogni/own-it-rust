//! What is being sent: weight, size and contents, checked once at
//! construction.
//!
//! Every type here keeps its fields private. A value that exists has passed
//! its checks, so no other module has to check again.

use std::error::Error;
use std::fmt;

/// A weight in grams.
///
/// A tuple struct around one `u32`: the same size as the number, but a
/// function that wants grams cannot be handed millimetres by mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Grams(u32);

impl Grams {
    pub const fn new(grams: u32) -> Grams {
        Grams(grams)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for Grams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} g", self.0)
    }
}

/// The three sides of a box in millimetres.
///
/// Invariant: `sides` is sorted from longest to shortest and every side is in
/// `1..=MAX_SIDE_MM`. Because of the fixed order, the derived `PartialEq`
/// already says that a box turned on its side is the same box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dimensions {
    sides: [u32; 3],
}

impl Dimensions {
    pub const MAX_SIDE_MM: u32 = 2000;

    pub fn new(a: u32, b: u32, c: u32) -> Result<Dimensions, ParcelError> {
        let mut sides = [a, b, c];
        // Check in argument order, before sorting, so that the error names
        // the first bad argument.
        if let Some(&bad) = sides
            .iter()
            .find(|&&side| !(1..=Self::MAX_SIDE_MM).contains(&side))
        {
            return Err(ParcelError::Side(bad));
        }
        sides.sort_unstable_by(|x, y| y.cmp(x));
        Ok(Dimensions { sides })
    }

    pub fn longest(&self) -> u32 {
        self.sides[0]
    }

    pub fn middle(&self) -> u32 {
        self.sides[1]
    }

    pub fn shortest(&self) -> u32 {
        self.sides[2]
    }

    /// The volume in cubic millimetres. Three sides of 2000 mm give
    /// 8 000 000 000, which does not fit in a `u32`.
    pub fn volume_mm3(&self) -> u64 {
        self.sides.iter().map(|&side| u64::from(side)).product()
    }
}

/// What is inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Contents {
    Documents,
    Goods { fragile: bool },
    Liquid { millilitres: u32 },
}

/// The four size classes, from smallest to largest.
///
/// The derived `Ord` compares variants in the order they are written, so
/// this order is part of the behaviour: do not sort the list alphabetically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SizeClass {
    Letter,
    Small,
    Medium,
    Large,
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
        match self {
            ParcelError::Side(mm) => {
                write!(
                    f,
                    "side of {mm} mm is outside 1..={} mm",
                    Dimensions::MAX_SIDE_MM
                )
            }
            ParcelError::Weight(weight) => {
                write!(
                    f,
                    "weight of {weight} is outside 1..={} g",
                    Parcel::MAX_WEIGHT_G
                )
            }
            ParcelError::Liquid(ml) => write!(f, "{ml} ml of liquid do not fit in the parcel"),
        }
    }
}

impl Error for ParcelError {}

/// A parcel that passed every check.
///
/// Only facts are stored. `billable` and `size_class` are computed when
/// asked: they are cheap, and a stored copy could disagree with the facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Parcel {
    weight: Grams,
    dimensions: Dimensions,
    contents: Contents,
}

impl Parcel {
    pub const MAX_WEIGHT_G: u32 = 30_000;

    /// Grams per cubic millimetre are not a whole number; 5000 mm³ count as
    /// one gram.
    const MM3_PER_GRAM: u64 = 5000;

    pub fn new(
        weight: Grams,
        dimensions: Dimensions,
        contents: Contents,
    ) -> Result<Parcel, ParcelError> {
        if !(1..=Self::MAX_WEIGHT_G).contains(&weight.get()) {
            return Err(ParcelError::Weight(weight));
        }
        if let Contents::Liquid { millilitres } = contents {
            // One millilitre is 1000 mm³. In u64 the product cannot overflow.
            if u64::from(millilitres) * 1000 > dimensions.volume_mm3() {
                return Err(ParcelError::Liquid(millilitres));
            }
        }
        Ok(Parcel {
            weight,
            dimensions,
            contents,
        })
    }

    pub fn weight(&self) -> Grams {
        self.weight
    }

    pub fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    pub fn contents(&self) -> Contents {
        self.contents
    }

    /// The weight a carrier charges for: the real weight, or the weight that
    /// the volume counts as, whichever is greater.
    pub fn billable(&self) -> Grams {
        let by_volume = self.dimensions.volume_mm3().div_ceil(Self::MM3_PER_GRAM);
        // At most 2000³ / 5000 = 1 600 000, so the conversion cannot fail.
        // `unwrap_or` keeps the function total without a panic path.
        let by_volume = Grams(u32::try_from(by_volume).unwrap_or(u32::MAX));
        self.weight.max(by_volume)
    }

    pub fn size_class(&self) -> SizeClass {
        let grams = self.weight.get();
        let Dimensions { sides } = self.dimensions;
        // One pattern per row of the table in the SPEC. The first arm that
        // matches wins, so each row may assume that the rows above failed.
        match (self.contents, grams, sides) {
            (Contents::Documents, ..=100, [..=240, ..=165, ..=5]) => SizeClass::Letter,
            (_, ..=2_000, [..=450, ..=350, ..=160]) => SizeClass::Small,
            (_, ..=10_000, [..=800, ..=500, ..=400]) => SizeClass::Medium,
            _ => SizeClass::Large,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sides_are_stored_longest_first() {
        let d = Dimensions::new(10, 300, 20).unwrap();
        assert_eq!(d.sides, [300, 20, 10]);
    }

    #[test]
    fn billable_rounds_the_volume_up() {
        // 1667 * 3 * 1 = 5001 mm³, one more than a full gram: 2 g.
        let d = Dimensions::new(1667, 3, 1).unwrap();
        let p = Parcel::new(Grams::new(1), d, Contents::Documents).unwrap();
        assert_eq!(p.billable(), Grams::new(2));
    }
}
