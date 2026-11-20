#![doc = include_str!("../../README.md")]

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

/// One line of a ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The date as written, `YYYY-MM-DD`.
    pub date: String,
    /// What the money was spent on.
    pub category: String,
    /// The amount in hundredths.
    pub cents: u64,
}

/// Why a line is not an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineError {
    /// The line does not have exactly three fields.
    FieldCount(usize),
    /// The first field is not a date `YYYY-MM-DD`.
    BadDate,
    /// The third field is not an amount such as `12`, `12.5` or `12.50`.
    BadAmount,
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldCount(found) => write!(f, "expected 3 fields, found {found}"),
            Self::BadDate => f.write_str("the date must look like 2026-11-03"),
            Self::BadAmount => f.write_str("the amount must look like 12, 12.5 or 12.50"),
        }
    }
}

impl std::error::Error for LineError {}

/// A [`LineError`] together with the number of the line it was found on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The line number, counted from 1.
    pub line: usize,
    /// What is wrong with that line.
    pub error: LineError,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.error)
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Parses one ledger line: `YYYY-MM-DD category amount`.
///
/// # Errors
///
/// Returns a [`LineError`] if the line does not have three fields, or if the date
/// or the amount is malformed. It never panics.
///
/// # Examples
///
/// ```
/// let entry = tally_core::parse_line("2026-11-03 groceries 12.5")?;
/// assert_eq!(entry.category, "groceries");
/// assert_eq!(entry.cents, 1250);
/// # Ok::<(), tally_core::LineError>(())
/// ```
pub fn parse_line(line: &str) -> Result<Entry, LineError> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let [date, category, amount] = fields.as_slice() else {
        return Err(LineError::FieldCount(fields.len()));
    };
    if !is_date(date) {
        return Err(LineError::BadDate);
    }
    Ok(Entry {
        date: (*date).to_owned(),
        category: (*category).to_owned(),
        cents: parse_cents(amount).ok_or(LineError::BadAmount)?,
    })
}

fn is_date(text: &str) -> bool {
    // Work on bytes: a slice such as `text[5..7]` would panic in the middle of a
    // multi-byte character.
    let bytes = text.as_bytes();
    let digits = |range: std::ops::Range<usize>| -> Option<u32> {
        let part = bytes.get(range)?;
        if part.iter().all(u8::is_ascii_digit) {
            std::str::from_utf8(part).ok()?.parse().ok()
        } else {
            None
        }
    };
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && digits(0..4).is_some()
        && digits(5..7).is_some_and(|month| (1..=12).contains(&month))
        && digits(8..10).is_some_and(|day| (1..=31).contains(&day))
}

fn parse_cents(text: &str) -> Option<u64> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if whole.is_empty() || !all_digits(whole) || !all_digits(fraction) {
        return None;
    }
    let hundredths = match fraction.len() {
        0 if !text.contains('.') => 0,
        // "12.5" means 50 hundredths, not 5.
        1 => fraction.parse::<u64>().ok()? * 10,
        2 => fraction.parse().ok()?,
        _ => return None,
    };
    whole
        .parse::<u64>()
        .ok()?
        .checked_mul(100)?
        .checked_add(hundredths)
}

/// Parses a whole ledger. Blank lines and lines that start with `#` are skipped.
///
/// # Errors
///
/// Returns the first line that is not an entry, with its line number.
pub fn parse(text: &str) -> Result<Vec<Entry>, ParseError> {
    let mut entries = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let entry = parse_line(line).map_err(|error| ParseError {
            line: index + 1,
            error,
        })?;
        entries.push(entry);
    }
    Ok(entries)
}

/// The sum per category, in alphabetical order of the category.
///
/// One amount fits in 64 bits; a sum of many may not. Sums are 128 bits wide, so
/// they are exact for any ledger a machine can hold, and nothing can overflow.
#[must_use]
pub fn totals(entries: &[Entry]) -> BTreeMap<&str, u128> {
    let mut totals = BTreeMap::new();
    for entry in entries {
        *totals.entry(entry.category.as_str()).or_insert(0) += u128::from(entry.cents);
    }
    totals
}

/// The report: one row per category, sorted by name, then a `total` row.
#[must_use]
pub fn report(entries: &[Entry]) -> String {
    let totals = totals(entries);
    let total: u128 = totals.values().sum();
    let rows: Vec<(&str, String)> = totals
        .iter()
        .map(|(name, cents)| (*name, money(*cents)))
        .chain([("total", money(total))])
        .collect();
    let name_width = rows
        .iter()
        .map(|(name, _)| name.chars().count())
        .max()
        .unwrap_or(0);
    let amount_width = rows
        .iter()
        .map(|(_, amount)| amount.len())
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for (name, amount) in &rows {
        // Writing to a `String` cannot fail.
        let _ = writeln!(out, "{name:<name_width$}  {amount:>amount_width$}");
    }
    out
}

fn money(cents: u128) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// The entries in input order, as one line of JSON.
#[cfg(feature = "json")]
#[must_use]
pub fn to_json(entries: &[Entry]) -> String {
    let objects: Vec<String> = entries
        .iter()
        .map(|entry| {
            format!(
                "{{\"date\":{},\"category\":{},\"cents\":{}}}",
                json_string(&entry.date),
                json_string(&entry.category),
                entry.cents
            )
        })
        .collect();
    format!("[{}]", objects.join(","))
}

#[cfg(feature = "json")]
fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {
                // Writing to a `String` cannot fail.
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_amounts_in_three_spellings() {
        for (text, cents) in [("12", 1200), ("12.5", 1250), ("12.50", 1250), ("0.07", 7)] {
            let entry = parse_line(&format!("2026-11-03 food {text}")).unwrap();
            assert_eq!(entry.cents, cents, "amount {text}");
        }
    }

    #[test]
    fn rejects_bad_lines_without_panicking() {
        assert_eq!(parse_line(""), Err(LineError::FieldCount(0)));
        assert_eq!(parse_line("2026-11-03 food"), Err(LineError::FieldCount(2)));
        assert_eq!(parse_line("2026-13-03 food 1"), Err(LineError::BadDate));
        assert_eq!(parse_line("2026-11-32 food 1"), Err(LineError::BadDate));
        assert_eq!(parse_line("20261103 food 1"), Err(LineError::BadDate));
        assert_eq!(parse_line("2026-1é-03 food 1"), Err(LineError::BadDate));
        for amount in ["", ".", ".5", "1.", "1.234", "-1", "1e3", "1,5", "é"] {
            assert_eq!(
                parse_line(&format!("2026-11-03 food {amount}")).map(|e| e.cents),
                if amount.is_empty() {
                    Err(LineError::FieldCount(2))
                } else {
                    Err(LineError::BadAmount)
                },
                "amount {amount:?}"
            );
        }
        assert_eq!(
            parse_line("2026-11-03 food 999999999999999999999"),
            Err(LineError::BadAmount),
            "an amount that does not fit is an error, not an overflow"
        );
    }

    #[test]
    fn sums_are_exact_beyond_64_bits() {
        let entries = parse(&"2026-11-03 big 184467440737095516.15\n".repeat(2)).unwrap();
        assert_eq!(entries[0].cents, u64::MAX);
        assert_eq!(
            report(&entries),
            "big    368934881474191032.30\ntotal  368934881474191032.30\n"
        );
    }

    #[test]
    fn parse_reports_the_line_number() {
        let error = parse("# header\n\n2026-11-03 food 1\noops\n").unwrap_err();
        assert_eq!(error.line, 4);
        assert_eq!(error.to_string(), "line 4: expected 3 fields, found 1");
    }

    #[cfg(feature = "json")]
    #[test]
    fn json_escapes_text() {
        let entries = parse("2026-11-03 a\"b\\c 1").unwrap();
        assert_eq!(
            to_json(&entries),
            r#"[{"date":"2026-11-03","category":"a\"b\\c","cents":100}]"#
        );
        assert_eq!(to_json(&[]), "[]");
    }
}
