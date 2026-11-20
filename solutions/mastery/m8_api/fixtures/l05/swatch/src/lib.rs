//! swatch: colours for terminal programs.
//!
//! Lesson 05: this file and `Cargo.toml` are yours to change. The contract is
//! in `src/l05_features.rs`.
//!
//! Every feature of this crate only adds items. No item changes its type or
//! disappears when a feature is turned on, so the crate compiles, and means the
//! same, with any set of features.

/// A colour: red, green and blue, 8 bits each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }

    /// The colour as `#rrggbb`.
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// The colour with this name: `red`, `green`, `blue`, `black` or `white`.
    #[cfg(feature = "names")]
    pub fn named(name: &str) -> Option<Color> {
        match name {
            "red" => Some(Color::new(255, 0, 0)),
            "green" => Some(Color::new(0, 255, 0)),
            "blue" => Some(Color::new(0, 0, 255)),
            "black" => Some(Color::new(0, 0, 0)),
            "white" => Some(Color::new(255, 255, 255)),
            _ => None,
        }
    }

    /// The colour as a JSON object: `{"r":255,"g":0,"b":0}`.
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> String {
        tinyjson::object(&[
            ("r", u32::from(self.r)),
            ("g", u32::from(self.g)),
            ("b", u32::from(self.b)),
        ])
    }
}

/// A colour with 16 bits per channel.
///
/// A second type, not a wider `Color`: a program that was written for 8 bits
/// keeps its type when another program in the same build turns `wide` on.
#[cfg(feature = "wide")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color16 {
    pub r: u16,
    pub g: u16,
    pub b: u16,
}

#[cfg(feature = "wide")]
impl Color16 {
    pub const fn new(r: u16, g: u16, b: u16) -> Color16 {
        Color16 { r, g, b }
    }

    /// The colour as `#rrrrggggbbbb`.
    pub fn hex(&self) -> String {
        format!("#{:04x}{:04x}{:04x}", self.r, self.g, self.b)
    }

    /// The colour as a JSON object: `{"r":65535,"g":0,"b":0}`.
    ///
    /// Needs two features. Each of them alone still compiles.
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> String {
        tinyjson::object(&[
            ("r", u32::from(self.r)),
            ("g", u32::from(self.g)),
            ("b", u32::from(self.b)),
        ])
    }
}

/// Widens every channel by repeating its byte: `0xab` becomes `0xabab`, so
/// black stays black and white stays white.
#[cfg(feature = "wide")]
impl From<Color> for Color16 {
    fn from(color: Color) -> Color16 {
        let widen = |channel: u8| u16::from(channel) * 257;
        Color16::new(widen(color.r), widen(color.g), widen(color.b))
    }
}
