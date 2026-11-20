//! Temperatures as distinct types, so a Celsius value cannot be passed where
//! Fahrenheit is expected.

/// Degrees Celsius.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Celsius(pub f64);

/// Degrees Fahrenheit.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Fahrenheit(pub f64);

/// Why a text is not a temperature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The text does not end in `C`.
    MissingUnit,
    /// The part before the unit is not a number.
    BadNumber,
}

impl Celsius {
    /// A temperature that exists.
    ///
    /// # Panics
    ///
    /// Panics if `degrees` is below absolute zero (-273.15).
    pub fn new(degrees: f64) -> Self {
        assert!(degrees >= -273.15, "below absolute zero: {degrees}");
        Self(degrees)
    }
}

/// Converts to Fahrenheit.
///
/// # Examples
///
/// ```
/// use thermo::{Celsius, convert};
///
/// assert_eq!(convert(Celsius(100.0)).0, 210.0);
/// ```
pub fn to_fahrenheit(celsius: Celsius) -> Fahrenheit {
    Fahrenheit(celsius.0 * 9.0 / 5.0 + 32.0)
}

/// Parses text such as `"21.5C"`.
///
/// # Errors
///
/// [`ParseError::MissingUnit`] if the text does not end in `C`, and
/// [`ParseError::BadNumber`] if the rest is not a number.
///
/// # Examples
///
/// ```
/// let t = thermo::parse_celsius("21.5C").unwrap();
/// assert_eq!(t.0, 21.0);
/// ```
pub fn parse_celsius(text: &str) -> Result<Celsius, ParseError> {
    let number = text.strip_suffix('C').ok_or(ParseError::MissingUnit)?;
    number
        .trim()
        .parse()
        .map(Celsius)
        .map_err(|_| ParseError::BadNumber)
}
