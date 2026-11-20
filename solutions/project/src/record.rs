//! Milestones m09 (Chapters 9–10), m11 (Display), and m17 (text).
//!
//! A [`Record`] is one parsed line. It does not own any text: the timestamp,
//! target, and fields borrow from the line, so a record cannot outlive the
//! line's buffer. Code that must keep data longer (m16 aggregation) copies
//! exactly what it needs.

use crate::error::{ErrorKind, ParseError};
use crate::level::Level;
use crate::num::NumError;
use crate::num::{Overflow, parse_u64};
use crate::scan::{Field, column_of, find_field, parse_fields, split_word};
use std::{borrow::Cow, fmt};

/// One parsed log line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record<'a> {
    /// `YYYY-MM-DDTHH:MM:SSZ`, validated by [`is_timestamp`]. Timestamps in
    /// this exact format sort in time order when compared as strings.
    pub timestamp: &'a str,
    /// The level.
    pub level: Level,
    /// The component that wrote the line, such as `auth`.
    pub target: &'a str,
    /// The fields, in line order. Keys may repeat.
    pub fields: Vec<Field<'a>>,
}

/// m09 C. Whether `s` is exactly `YYYY-MM-DDTHH:MM:SSZ`: 20 ASCII bytes,
/// digits where the format has letters, with month `01`–`12`, day `01`–`31`,
/// hour `00`–`23`, minute `00`–`59`, and second `00`–`60` (leap second).
/// The calendar is not checked further: `2026-02-31T00:00:00Z` is accepted.
///
/// Use a slice pattern on `s.as_bytes()` to name the twenty positions.
pub fn is_timestamp(s: &str) -> bool {
    let &[
        y1,
        y2,
        y3,
        y4,
        b'-',
        mo1,
        mo2,
        b'-',
        d1,
        d2,
        b'T',
        h1,
        h2,
        b':',
        mi1,
        mi2,
        b':',
        s1,
        s2,
        b'Z',
    ] = s.as_bytes()
    else {
        return false;
    };
    let digits = [y1, y2, y3, y4, mo1, mo2, d1, d2, h1, h2, mi1, mi2, s1, s2];
    if !digits.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let two = |a: u8, b: u8| (a - b'0') * 10 + (b - b'0');
    matches!(two(mo1, mo2), 1..=12)
        && matches!(two(d1, d2), 1..=31)
        && two(h1, h2) <= 23
        && two(mi1, mi2) <= 59
        && two(s1, s2) <= 60
}

/// m09 D. Parse one line: `TIMESTAMP LEVEL TARGET FIELD...`.
///
/// Words are separated by whitespace; leading and trailing whitespace is
/// ignored. Errors, with the 1-based character column they point at:
///
/// | Problem | Kind | Column |
/// | --- | --- | --- |
/// | nothing but whitespace | `Empty` | 1 |
/// | first word fails [`is_timestamp`] | `BadTimestamp` | start of that word |
/// | no second word | `MissingLevel` | the end: one past the line's last character |
/// | second word is not a level ([`Level::from_name`]) | `UnknownLevel` | start of that word |
/// | no third word | `MissingTarget` | the end: one past the line's last character |
/// | third word contains `=` | `MissingTarget` | start of that word |
/// | a field error | from [`crate::scan::parse_fields`] | start of that field |
///
/// "Start of a word" and "the end" are both [`crate::scan::column_of`] of
/// the text that remains when you look for the word (trim the line's start
/// first). Use [`crate::scan::split_word`], `let ... else`, and `?`. The
/// record borrows everything from `line`.
pub fn parse_record(line: &str) -> Result<Record<'_>, ParseError> {
    let at = |kind, rest: &str| ParseError::new(kind, column_of(line, rest));
    let rest = line.trim_ascii_start();
    let Some((timestamp, after_ts)) = split_word(rest) else {
        return Err(ParseError::new(ErrorKind::Empty, 1));
    };
    if !is_timestamp(timestamp) {
        return Err(at(ErrorKind::BadTimestamp, rest));
    }
    let Some((level, after_level)) = split_word(after_ts) else {
        return Err(at(ErrorKind::MissingLevel, after_ts));
    };
    let level = Level::from_name(level).ok_or_else(|| at(ErrorKind::UnknownLevel, after_ts))?;
    let Some((target, after_target)) = split_word(after_level) else {
        return Err(at(ErrorKind::MissingTarget, after_level));
    };
    if target.contains('=') {
        return Err(at(ErrorKind::MissingTarget, after_level));
    }
    let fields = parse_fields(line, after_target)?;
    Ok(Record {
        timestamp,
        level,
        target,
        fields,
    })
}

impl<'a> Record<'a> {
    /// m09 E. The raw value of the first field named `key` (see
    /// [`crate::scan::find_field`]). The result borrows from the line, not
    /// from the record.
    pub fn get(&self, key: &str) -> Option<&'a str> {
        find_field(&self.fields, key)
    }

    /// m09 F. The first field named `key` as a number, read with
    /// [`crate::num::parse_u64`] under [`crate::num::Overflow::Saturate`].
    /// `Ok(None)` if there is no such field.
    pub fn number(&self, key: &str) -> Result<Option<u64>, NumError> {
        self.get(key)
            .map(|raw| parse_u64(raw, Overflow::Saturate))
            .transpose()
    }

    /// m17 D. The decoded text of the first field named `key` (see
    /// [`Field::text`]), or `None` if there is no such field.
    pub fn text(&self, key: &str) -> Option<Cow<'a, str>> {
        self.fields.iter().find(|f| f.key == key).map(Field::text)
    }
}

/// m11 D. The canonical line: `TIMESTAMP LEVEL TARGET` followed by
/// ` key=raw` for each bare field or ` key="raw"` for each quoted one, with
/// single spaces. `parse_record(&record.to_string())` gives back an equal
/// record. Write straight into the formatter: no intermediate `String`s.
impl fmt::Display for Record<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.timestamp, self.level, self.target)?;
        for field in &self.fields {
            if field.quoted {
                write!(f, " {}=\"{}\"", field.key, field.raw)?;
            } else {
                write!(f, " {}={}", field.key, field.raw)?;
            }
        }
        Ok(())
    }
}
