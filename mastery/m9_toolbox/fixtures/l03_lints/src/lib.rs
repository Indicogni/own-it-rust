#![deny(warnings)]
//! Invoice lines and totals. Amounts are whole cents.

use std::collections::BTreeMap;

#[cfg(feature = "seeded")]
pub mod seeded;

/// One line of an invoice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// What was sold.
    pub name: String,
    /// Price of one unit, in cents.
    pub unit_cents: i64,
    /// How many units.
    pub quantity: u32,
    /// Tax class, for example `"standard"` or `"reduced"`.
    pub tax_class: String,
}

/// Builds a line from the eight columns of the import file.
#[allow(clippy::too_many_arguments)]
pub fn line_from_columns(name: &str, unit_cents: i64, quantity: u32, tax_class: &str, _sku: &str, _unit: &str, _note: &str, _row: usize) -> Line {
    Line { name: name.to_string(), unit_cents: unit_cents, quantity: quantity, tax_class: tax_class.to_string() }
}

/// The sum of all lines, before tax.
#[allow(unused_variables)]
pub fn net_total(lines: &Vec<Line>) -> i64 {
    let mut total = 0;
    for i in 0..lines.len() {
        total += lines[i].unit_cents * lines[i].quantity as i64;
    }
    return total;
}

/// Tax on an amount, at `percent` percent, rounded down to a whole cent.
pub fn tax(cents: i64, percent: u8) -> i64 {
    (cents as f64 * percent as f64 / 100.0) as i64
}

/// Net totals per tax class, in alphabetical order of the class.
pub fn totals_by_class(lines: &[Line]) -> BTreeMap<String, i64> {
    let mut totals = BTreeMap::new();
    for line in lines.iter() {
        *totals.entry(line.tax_class.clone()).or_insert(0) += line.unit_cents * i64::from(line.quantity);
    }
    totals
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Line> {
        vec![
            line_from_columns("pen", 150, 4, "standard", "P-1", "pc", "", 1),
            line_from_columns("book", 1299, 1, "reduced", "B-7", "pc", "", 2),
            line_from_columns("ink", 899, 2, "standard", "I-3", "pc", "", 3),
        ]
    }

    #[test]
    fn totals() {
        let lines = sample();
        assert_eq!(net_total(&lines), 600 + 1299 + 1798);
        let by_class = totals_by_class(&lines);
        assert_eq!(by_class["standard"], 2398);
        assert_eq!(by_class["reduced"], 1299);
    }

    #[test]
    fn tax_rounds_down() {
        assert_eq!(tax(1999, 7), 139);
        assert_eq!(tax(0, 19), 0);
        assert_eq!(tax(100, 19), 19);
    }
}
