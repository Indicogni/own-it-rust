//! A user of `swatch` that stores wide colours as JSON. You do not need to
//! edit this file.
//!
//! It asks for two features, `wide` and `json`. One item of `swatch` exists
//! only when both are on: `Color16::to_json`.

use swatch::Color16;

/// A palette of wide colours as a JSON array.
pub fn export(palette: &[Color16]) -> String {
    let colors: Vec<String> = palette.iter().map(Color16::to_json).collect();
    format!("[{}]", colors.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use swatch::Color;

    #[test]
    fn wide_colours_become_json() {
        let palette = [Color16::new(65535, 0, 4660), Color16::new(0, 256, 0)];
        assert_eq!(
            export(&palette),
            r#"[{"r":65535,"g":0,"b":4660},{"r":0,"g":256,"b":0}]"#
        );
    }

    #[test]
    fn narrow_colours_keep_their_own_json() {
        let narrow = Color::new(255, 128, 0);
        assert_eq!(narrow.to_json(), r#"{"r":255,"g":128,"b":0}"#);
        assert_eq!(
            Color16::from(narrow).to_json(),
            r#"{"r":65535,"g":32896,"b":0}"#
        );
    }
}
