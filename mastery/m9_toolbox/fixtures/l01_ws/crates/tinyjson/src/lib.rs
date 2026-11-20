//! A stand-in for a JSON crate, so the lesson needs no download.

/// `{"key":value}` for one numeric field.
pub fn number_field(key: &str, value: u64) -> String {
    format!("{{\"{key}\":{value}}}")
}
