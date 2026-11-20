//! Shipping prices. The tests below run every line of `shipping_cents`.

/// The price of a parcel in cents.
///
/// - up to 500 g: 300
/// - up to 2000 g: 550
/// - above 2000 g: 550, plus 120 for every started kilogram above 2000 g
/// - express doubles the price
pub fn shipping_cents(weight_g: u32, express: bool) -> u32 {
    let base = if weight_g <= 500 {
        300
    } else if weight_g <= 2000 {
        550
    } else {
        550 + (weight_g - 2000).div_ceil(1000) * 120
    };
    if express { base * 2 } else { base }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_parcel() {
        assert_eq!(shipping_cents(100, false), 300);
    }

    #[test]
    fn medium_parcel() {
        assert_eq!(shipping_cents(1000, false), 550);
    }

    #[test]
    fn heavy_parcels_cost_more() {
        assert!(shipping_cents(5000, false) > 550);
    }

    #[test]
    fn express_costs_more() {
        assert!(shipping_cents(100, true) > shipping_cents(100, false));
    }
}
