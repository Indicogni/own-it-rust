//! Who can send a parcel, and for how much.

use crate::parcel::{Contents, Parcel, SizeClass};

/// How far away another country is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    Near,
    Far,
}

/// Where a parcel goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    Local,
    National,
    International(Region),
}

/// A company that carries parcels.
///
/// The trait has no generic methods and no `Self` in argument or return
/// position, so `dyn Carrier` works.
pub trait Carrier {
    /// The name of the company.
    fn name(&self) -> &str;

    /// The price in cents, or `None` if the carrier does not take this
    /// parcel to this zone.
    fn price(&self, parcel: &Parcel, zone: Zone) -> Option<u32>;

    /// Whether the carrier takes this parcel to this zone. A carrier that
    /// can answer faster than by computing a price may override this.
    fn accepts(&self, parcel: &Parcel, zone: Zone) -> bool {
        self.price(parcel, zone).is_some()
    }
}

/// A box is a carrier if its content is one. All three methods are
/// forwarded: without the explicit `accepts`, the box would use the default
/// body and ignore an override of the carrier inside.
impl<C: Carrier + ?Sized> Carrier for Box<C> {
    fn name(&self) -> &str {
        (**self).name()
    }

    fn price(&self, parcel: &Parcel, zone: Zone) -> Option<u32> {
        (**self).price(parcel, zone)
    }

    fn accepts(&self, parcel: &Parcel, zone: Zone) -> bool {
        (**self).accepts(parcel, zone)
    }
}

/// The national post: slow, with fixed prices per size class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Snail;

impl Snail {
    const FRAGILE_SURCHARGE: u32 = 200;
}

impl Carrier for Snail {
    fn name(&self) -> &str {
        "Snail Mail"
    }

    fn price(&self, parcel: &Parcel, zone: Zone) -> Option<u32> {
        use Region::{Far, Near};
        use SizeClass::{Large, Letter, Medium, Small};
        use Zone::{International, Local, National};

        let contents = parcel.contents();
        if matches!(
            (contents, zone),
            (Contents::Liquid { .. }, International(_))
        ) {
            return None;
        }
        // The table of the SPEC, one row per size class.
        let base = match (parcel.size_class(), zone) {
            (Letter, Local) => 90,
            (Letter, National) => 110,
            (Letter, International(Near)) => 250,
            (Letter, International(Far)) => 400,
            (Small, Local) => 400,
            (Small, National) => 550,
            (Small, International(Near)) => 1400,
            (Small, International(Far)) => 2600,
            (Medium, Local) => 700,
            (Medium, National) => 950,
            (Medium, International(Near)) => 3100,
            (Large, Local) => 1500,
            (Large, National) => 1900,
            (Medium, International(Far)) | (Large, International(_)) => return None,
        };
        let surcharge = match contents {
            Contents::Goods { fragile: true } => Self::FRAGILE_SURCHARGE,
            _ => 0,
        };
        Some(base + surcharge)
    }
}

/// A courier that charges by weight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dash {
    name: String,
    base: u32,
    per_kg: u32,
}

impl Dash {
    pub fn new(name: &str, base: u32, per_kg: u32) -> Dash {
        Dash {
            name: name.to_string(),
            base,
            per_kg,
        }
    }
}

impl Carrier for Dash {
    fn name(&self) -> &str {
        &self.name
    }

    fn price(&self, parcel: &Parcel, zone: Zone) -> Option<u32> {
        let factor = match (zone, parcel.contents()) {
            (Zone::International(Region::Far), _) => return None,
            (Zone::National | Zone::International(_), Contents::Liquid { .. }) => return None,
            (Zone::Local, _) => 1,
            (Zone::National, _) => 2,
            (Zone::International(Region::Near), _) => 5,
        };
        let kilograms = parcel.billable().get().div_ceil(1000);
        // Every step is checked: a price that does not fit in a u32 is a
        // parcel that this carrier declines, not a wrapped number.
        self.per_kg
            .checked_mul(kilograms)?
            .checked_add(self.base)?
            .checked_mul(factor)
    }
}

/// Any carrier, with a percentage taken off its prices.
///
/// The struct itself has no bound on `C`. Bounds go on the `impl` blocks
/// that need them, so the type can be named in places where the bound is not
/// known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discounted<C> {
    inner: C,
    percent: u8,
}

impl<C> Discounted<C> {
    /// `None` if `percent` is greater than 100.
    pub fn new(inner: C, percent: u8) -> Option<Discounted<C>> {
        (percent <= 100).then_some(Discounted { inner, percent })
    }

    pub fn inner(&self) -> &C {
        &self.inner
    }

    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<C: Carrier> Carrier for Discounted<C> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn price(&self, parcel: &Parcel, zone: Zone) -> Option<u32> {
        let full = u64::from(self.inner.price(parcel, zone)?);
        // u32::MAX * 100 fits easily in a u64. Rounding up: the customer
        // never pays a fraction of a cent less than the exact price.
        let reduced = (full * u64::from(100 - self.percent)).div_ceil(100);
        // `reduced <= full`, so it fits. `ok()` avoids a panic path anyway.
        u32::try_from(reduced).ok()
    }

    fn accepts(&self, parcel: &Parcel, zone: Zone) -> bool {
        // A discount never changes whether the carrier takes the parcel.
        self.inner.accepts(parcel, zone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parcel::{Dimensions, Grams};

    fn goods(grams: u32) -> Parcel {
        let dimensions = Dimensions::new(100, 100, 100).unwrap();
        Parcel::new(
            Grams::new(grams),
            dimensions,
            Contents::Goods { fragile: false },
        )
        .unwrap()
    }

    #[test]
    fn dash_charges_every_started_kilogram() {
        let dash = Dash::new("Dash", 100, 10);
        assert_eq!(dash.price(&goods(1000), Zone::Local), Some(110));
        assert_eq!(dash.price(&goods(1001), Zone::Local), Some(120));
        assert_eq!(dash.price(&goods(1), Zone::National), Some(220));
    }

    #[test]
    fn dash_declines_instead_of_wrapping() {
        let greedy = Dash::new("Greedy", u32::MAX, 1);
        assert_eq!(greedy.price(&goods(1), Zone::Local), None);
        let steep = Dash::new("Steep", 0, u32::MAX);
        assert_eq!(steep.price(&goods(1), Zone::Local), Some(u32::MAX));
        assert_eq!(steep.price(&goods(1), Zone::National), None);
    }

    #[test]
    fn a_boxed_discount_keeps_the_name_inside() {
        let boxed: Box<dyn Carrier> = Box::new(Discounted::new(Snail, 50).unwrap());
        assert_eq!(boxed.name(), "Snail Mail");
        assert_eq!(boxed.price(&goods(500), Zone::Local), Some(200));
        assert!(Discounted::new(Snail, 101).is_none());
    }
}
