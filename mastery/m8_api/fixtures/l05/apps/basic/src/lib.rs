//! A user of `swatch` with its default features. You do not need to edit this file.

use swatch::Color;

/// The colour for a log level.
pub fn level_color(level: &str) -> Color {
    let name = match level {
        "error" => "red",
        "ok" => "green",
        _ => "white",
    };
    Color::named(name).expect("the three names are in the table of swatch")
}

/// A grey with the given brightness. This program stores colours in files,
/// one byte per channel.
pub fn grey(brightness: u8) -> Color {
    Color::new(brightness, brightness, brightness)
}

/// The three bytes that this program writes to a file.
pub fn bytes(color: Color) -> [u8; 3] {
    [color.r, color.g, color.b]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_have_names_and_hex_codes() {
        assert_eq!(level_color("error").hex(), "#ff0000");
        assert_eq!(level_color("ok"), Color::new(0, 255, 0));
        assert_eq!(Color::named("teal"), None);
    }

    #[test]
    fn channels_are_bytes() {
        assert_eq!(bytes(grey(128)), [128, 128, 128]);
        assert_eq!(grey(128).hex(), "#808080");
        assert_eq!(std::mem::size_of::<Color>(), 3, "a colour is three bytes");
    }
}
