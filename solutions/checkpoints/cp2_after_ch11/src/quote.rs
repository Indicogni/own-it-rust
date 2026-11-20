//! Comparing carriers.

use crate::carrier::{Carrier, Zone};
use crate::parcel::Parcel;

/// A carrier and the price it asked.
///
/// The quote borrows the carrier. It does not own or copy it, so the caller
/// can go on to use the very carrier that won.
#[derive(Clone, Copy)]
pub struct Quote<'c> {
    pub carrier: &'c dyn Carrier,
    pub cents: u32,
}

/// The carrier with the lowest price for this parcel and zone. Among equal
/// prices the carrier that comes first in the slice wins.
pub fn cheapest<'c>(
    carriers: &'c [Box<dyn Carrier>],
    parcel: &Parcel,
    zone: Zone,
) -> Option<Quote<'c>> {
    let mut best: Option<Quote<'c>> = None;
    for carrier in carriers {
        // One call per carrier: `price` already says whether it accepts.
        let Some(cents) = carrier.price(parcel, zone) else {
            continue;
        };
        // Strictly lower: an equal price does not replace an earlier one.
        if best.is_none_or(|quote| cents < quote.cents) {
            best = Some(Quote {
                carrier: carrier.as_ref(),
                cents,
            });
        }
    }
    best
}

/// The price of sending all parcels with one carrier, or `None` if the
/// carrier declines one of them.
///
/// `?Sized` lets the function take a `&dyn Carrier` as well as a reference
/// to a concrete carrier.
pub fn total<C: Carrier + ?Sized>(carrier: &C, parcels: &[Parcel], zone: Zone) -> Option<u64> {
    let mut sum = 0u64;
    for parcel in parcels {
        sum += u64::from(carrier.price(parcel, zone)?);
    }
    Some(sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carrier::{Dash, Snail};
    use crate::parcel::{Contents, Dimensions, Grams};

    fn letter() -> Parcel {
        let dimensions = Dimensions::new(200, 100, 3).unwrap();
        Parcel::new(Grams::new(50), dimensions, Contents::Documents).unwrap()
    }

    #[test]
    fn the_first_of_two_equal_prices_wins() {
        let carriers: Vec<Box<dyn Carrier>> = vec![
            Box::new(Dash::new("first", 90, 0)),
            Box::new(Snail),
            Box::new(Dash::new("third", 90, 0)),
        ];
        let quote = cheapest(&carriers, &letter(), Zone::Local).unwrap();
        assert_eq!((quote.carrier.name(), quote.cents), ("first", 90));
    }

    #[test]
    fn total_stops_at_a_declined_parcel() {
        let far = Zone::International(crate::carrier::Region::Far);
        assert_eq!(total(&Snail, &[letter(), letter()], far), Some(800));
        assert_eq!(total(&Dash::new("Dash", 1, 1), &[letter()], far), None);
        assert_eq!(total(&Snail, &[], far), Some(0));
    }
}
