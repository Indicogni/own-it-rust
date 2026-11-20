//! The `Date` header (SPEC section 6.3).
//!
//! The standard library can tell how many seconds have passed since
//! 1970-01-01 00:00:00 UTC, but it has no calendar. Turning a day count
//! into year, month and day is the only real work here.

use std::time::{SystemTime, UNIX_EPOCH};

const WEEKDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The current time in the fixed HTTP format, for example
/// `Sun, 06 Nov 1994 08:49:37 GMT`.
pub fn now() -> String {
    // A clock set before 1970 is not worth a failed request: report the
    // epoch instead.
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    format(seconds)
}

/// Formats seconds since the Unix epoch (UTC, no leap seconds).
pub fn format(seconds: u64) -> String {
    let days = seconds / 86_400;
    let second_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        // Day 0 was a Thursday.
        WEEKDAYS[(days % 7) as usize],
        day,
        MONTHS[(month - 1) as usize],
        year,
        second_of_day / 3600,
        second_of_day / 60 % 60,
        second_of_day % 60,
    )
}

/// Converts days since 1970-01-01 to `(year, month, day)` in the Gregorian
/// calendar.
///
/// The trick is to count from 1 March of year 0 instead. Then a leap day is
/// the last day of its year, and every 400-year cycle (146 097 days) looks
/// the same. Inside a cycle the year and the day of the year follow from
/// the three leap rules; inside a March-based year the months have the
/// regular lengths 31 30 31 30 31 31 30 31 30 31 31 28/29, which one linear
/// formula describes.
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    // Days from 0000-03-01 to 1970-01-01.
    let shifted = days + 719_468;
    let cycle = shifted / 146_097;
    let day_of_cycle = shifted % 146_097;
    // Remove the leap days that came before this day, then divide by 365.
    let year_of_cycle = (day_of_cycle - day_of_cycle / 1_460 + day_of_cycle / 36_524
        - day_of_cycle / 146_096)
        / 365;
    let day_of_year =
        day_of_cycle - (365 * year_of_cycle + year_of_cycle / 4 - year_of_cycle / 100);
    // Month 0 is March, month 11 is February.
    let march_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * march_month + 2) / 5 + 1;
    let month = if march_month < 10 {
        march_month + 3
    } else {
        march_month - 9
    };
    // January and February belong to the next calendar year.
    let year = 400 * cycle + year_of_cycle + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_instants() {
        assert_eq!(format(0), "Thu, 01 Jan 1970 00:00:00 GMT");
        assert_eq!(format(784_111_777), "Sun, 06 Nov 1994 08:49:37 GMT");
        assert_eq!(format(951_782_400), "Tue, 29 Feb 2000 00:00:00 GMT");
        assert_eq!(format(1_709_251_199), "Thu, 29 Feb 2024 23:59:59 GMT");
        assert_eq!(format(1_709_251_200), "Fri, 01 Mar 2024 00:00:00 GMT");
        assert_eq!(format(4_107_542_400), "Mon, 01 Mar 2100 00:00:00 GMT");
        assert_eq!(format(1_798_761_599), "Thu, 31 Dec 2026 23:59:59 GMT");
    }

    /// Walks four centuries day by day and checks every step against the
    /// calendar rules written the slow, obvious way.
    #[test]
    fn every_day_of_400_years_follows_the_calendar() {
        fn is_leap(year: u64) -> bool {
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
        }
        let (mut year, mut month, mut day) = (1970, 1, 1);
        for days in 0..146_097 * 2 {
            assert_eq!(civil_from_days(days), (year, month, day), "day {days}");
            let length = match month {
                2 if is_leap(year) => 29,
                2 => 28,
                4 | 6 | 9 | 11 => 30,
                _ => 31,
            };
            day += 1;
            if day > length {
                day = 1;
                month += 1;
            }
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }
}
