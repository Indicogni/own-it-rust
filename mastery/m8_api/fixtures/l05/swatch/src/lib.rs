//! swatch: colours for terminal programs.
//!
//! Lesson 05: this file and `Cargo.toml` are yours to change. The contract is
//! in `src/l05_features.rs`.

#[cfg(all(feature = "wide", feature = "tinyjson"))]
compile_error!("the features `wide` and `tinyjson` cannot be used together yet");

/// One channel of a colour.
#[cfg(not(feature = "wide"))]
pub type Channel = u8;

/// One channel of a colour.
#[cfg(feature = "wide")]
pub type Channel = u16;

/// A colour: red, green and blue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: Channel,
    pub g: Channel,
    pub b: Channel,
}

impl Color {
    pub const fn new(r: Channel, g: Channel, b: Channel) -> Color {
        Color { r, g, b }
    }

    /// The colour as `#rrggbb`.
    #[cfg(not(feature = "wide"))]
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// The colour as `#rrrrggggbbbb`.
    #[cfg(feature = "wide")]
    pub fn hex(&self) -> String {
        format!("#{:04x}{:04x}{:04x}", self.r, self.g, self.b)
    }

    /// The colour with this name: `red`, `green`, `blue`, `black` or `white`.
    #[cfg(not(feature = "no-names"))]
    pub fn named(name: &str) -> Option<Color> {
        let max = Channel::MAX;
        match name {
            "red" => Some(Color::new(max, 0, 0)),
            "green" => Some(Color::new(0, max, 0)),
            "blue" => Some(Color::new(0, 0, max)),
            "black" => Some(Color::new(0, 0, 0)),
            "white" => Some(Color::new(max, max, max)),
            _ => None,
        }
    }

    /// The colour as a JSON object: `{"r":255,"g":0,"b":0}`.
    #[cfg(feature = "tinyjson")]
    pub fn to_json(&self) -> String {
        tinyjson::object(&[
            ("r", u32::from(self.r)),
            ("g", u32::from(self.g)),
            ("b", u32::from(self.b)),
        ])
    }
}
