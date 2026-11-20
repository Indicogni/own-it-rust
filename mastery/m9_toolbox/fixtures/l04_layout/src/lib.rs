//! A library with unit tests and documentation examples.

/// Doubles a number.
///
/// ```
/// assert_eq!(layout::double(4), 8);
/// ```
///
/// Output of the command-line tool:
///
/// ```text
/// $ layout 4
/// 8
/// ```
///
/// ```ignore
/// // Needs a network connection, so it is not run by default.
/// assert_eq!(layout::double(fetch_number()), 8);
/// ```
pub fn double(n: i64) -> i64 {
    n * 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles() {
        assert_eq!(double(2), 4);
    }

    #[test]
    #[ignore = "slow: run with -- --ignored"]
    fn doubles_everything() {
        for n in -1_000_000..1_000_000 {
            assert_eq!(double(n), n + n);
        }
    }
}
