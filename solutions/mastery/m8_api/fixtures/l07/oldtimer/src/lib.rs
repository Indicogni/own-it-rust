//! oldtimer: durations for a command-line timer.
//!
//! Lesson 07, task B. `Cargo.toml` says `rust-version = "1.74"`, so this file
//! uses only what Rust 1.74 has.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

/// The units of a duration and their length in seconds.
///
/// `LazyLock` came with Rust 1.80. `OnceLock` (1.70) does the same work when
/// the reader asks for the value through a function.
fn units() -> &'static [(char, u64)] {
    static UNITS: OnceLock<Vec<(char, u64)>> = OnceLock::new();
    UNITS.get_or_init(|| vec![('h', 3600), ('m', 60), ('s', 1)])
}

/// How many texts `parse_duration` has rejected since the program started.
static REJECTED: AtomicU64 = AtomicU64::new(0);

/// The number of seconds in the unit `letter`: `h`, `m` or `s`.
pub fn unit_seconds(letter: char) -> Option<u64> {
    units()
        .iter()
        .find(|(unit, _)| *unit == letter)
        .map(|(_, seconds)| *seconds)
}

/// How many texts `parse_duration` has rejected since the program started.
pub fn rejected() -> u64 {
    REJECTED.load(Ordering::Relaxed)
}

/// One part of a duration, such as `30m`, in seconds.
fn parse_part(part: &str) -> Result<u64, String> {
    let digits = part.trim_end_matches(|c: char| !c.is_ascii_digit());
    let number: u64 = digits
        .parse()
        .map_err(|_| format!("`{part}`: missing number"))?;
    let mut unit = part[digits.len()..].chars();
    match (unit.next().and_then(unit_seconds), unit.next()) {
        (Some(seconds), None) => Ok(number * seconds),
        _ => Err(format!("`{part}`: unknown unit")),
    }
}

/// The length in seconds of a text such as `1h 30m`: the sum of its parts.
///
/// # Errors
///
/// Returns a message that names the first part that is not a number followed
/// by `h`, `m` or `s`.
pub fn parse_duration(text: &str) -> Result<u64, String> {
    let total: Result<u64, String> = text.split_whitespace().map(parse_part).sum();
    // `Result::inspect_err` came with Rust 1.76.
    if total.is_err() {
        REJECTED.fetch_add(1, Ordering::Relaxed);
    }
    total
}

/// Whether `seconds` is allowed by `limit`. No limit allows everything.
pub fn is_within(limit: Option<u64>, seconds: u64) -> bool {
    // `Option::is_none_or` came with Rust 1.82.
    match limit {
        None => true,
        Some(limit) => seconds <= limit,
    }
}

/// Whether no lap time is smaller than the one before it.
pub fn laps_in_order(laps: &[u64]) -> bool {
    // `is_sorted` came with Rust 1.82.
    laps.windows(2).all(|pair| pair[0] <= pair[1])
}
