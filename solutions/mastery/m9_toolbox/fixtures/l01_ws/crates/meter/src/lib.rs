//! Durations written as text: `"1500ms"`, `"2s"`, `"3m"`.

/// Parses a duration into milliseconds. Units: `ms`, `s`, `m`.
pub fn parse_millis(text: &str) -> Option<u64> {
    let split = text.find(|c: char| !c.is_ascii_digit())?;
    let (digits, unit) = text.split_at(split);
    let value: u64 = digits.parse().ok()?;
    let factor = match unit {
        "ms" => 1,
        "s" => 1_000,
        "m" => 60_000,
        _ => return None,
    };
    value.checked_mul(factor)
}

/// True when the build script switched on the fast path (`METER_FAST` was set
/// while building).
pub fn has_fast_path() -> bool {
    cfg!(has_fast_path)
}

/// The duration as a JSON object. Only with JSON support compiled in.
#[cfg(feature = "json")]
pub fn to_json(millis: u64) -> String {
    tinyjson::number_field("millis", millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_units() {
        assert_eq!(parse_millis("1500ms"), Some(1500));
        assert_eq!(parse_millis("2s"), Some(2000));
        assert_eq!(parse_millis("3m"), Some(180_000));
        assert_eq!(parse_millis("3h"), None);
        assert_eq!(parse_millis("ms"), None);
    }
}
