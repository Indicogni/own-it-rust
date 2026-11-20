//! A user of `swatch` that needs JSON. You do not need to edit this file.

use swatch::Color;

/// A palette as a JSON array.
pub fn export(palette: &[Color]) -> String {
    let colors: Vec<String> = palette.iter().map(Color::to_json).collect();
    format!("[{}]", colors.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_palette_becomes_json() {
        let palette = [Color::new(255, 0, 0), Color::new(0, 0, 16)];
        assert_eq!(
            export(&palette),
            r#"[{"r":255,"g":0,"b":0},{"r":0,"g":0,"b":16}]"#
        );
        assert_eq!(palette[1].hex(), "#000010");
    }
}
