//! A user of `swatch` that needs 16 bits per channel. You do not need to edit this file.

use swatch::Color16;

/// The colour halfway between two wide colours.
pub fn mix(a: Color16, b: Color16) -> Color16 {
    let middle = |x: u16, y: u16| ((u32::from(x) + u32::from(y)) / 2) as u16;
    Color16::new(middle(a.r, b.r), middle(a.g, b.g), middle(a.b, b.b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use swatch::Color;

    #[test]
    fn wide_colours_have_four_hex_digits_per_channel() {
        assert_eq!(Color16::new(65535, 0, 4660).hex(), "#ffff00001234");
        let middle = mix(Color16::new(0, 0, 0), Color16::new(65535, 100, 0));
        assert_eq!(middle, Color16::new(32767, 50, 0));
    }

    #[test]
    fn narrow_colours_convert_to_wide_ones() {
        // 0xff becomes 0xffff, and 0x80 becomes 0x8080: every byte is repeated.
        let wide = Color16::from(Color::new(255, 128, 0));
        assert_eq!(wide, Color16::new(65535, 32896, 0));
        // The narrow type is as it always was.
        assert_eq!(Color::new(255, 128, 0).hex(), "#ff8000");
    }
}
