//! The framebuffer and the rasteriser (SPEC sections 3 to 5).
//!
//! Everything here is integer arithmetic on a `Vec<u32>`. There is no
//! window, no file and no clock in this module, which is why every pixel it
//! produces can be tested.

use std::io::{self, Write};

use crate::font;
use crate::hash::Fnv;
use crate::{ArtError, Flip};

const OPAQUE: u32 = 0xFF00_0000;

/// A rectangle of pixels in row-major order. Every stored pixel is opaque.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Framebuffer {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}

impl Framebuffer {
    /// A buffer filled with opaque black.
    ///
    /// # Panics
    /// Panics if either side is zero or larger than 16384.
    pub fn new(width: usize, height: usize) -> Framebuffer {
        assert!(
            (1..=16384).contains(&width) && (1..=16384).contains(&height),
            "framebuffer sides must be in 1..=16384"
        );
        Framebuffer {
            width,
            height,
            pixels: vec![OPAQUE; width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// All pixels, row by row from the top; `pixels()[y * width + x]`.
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color | OPAQUE);
    }

    pub fn get_pixel(&self, x: i32, y: i32) -> Option<u32> {
        self.index(x, y).map(|i| self.pixels[i])
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if let Some(i) = self.index(x, y) {
            self.pixels[i] = color | OPAQUE;
        }
    }

    /// Mixes `color` over the pixel already there. `alpha` 0 leaves the
    /// pixel alone, 255 replaces it.
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: u32, alpha: u8) {
        if let Some(i) = self.index(x, y) {
            self.pixels[i] = mix(self.pixels[i], color, u32::from(alpha));
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let Some((x0, x1, y0, y1)) = self.clip(x, y, i64::from(w), i64::from(h)) else {
            return;
        };
        let color = color | OPAQUE;
        for row in y0..y1 {
            let start = row * self.width;
            self.pixels[start + x0..start + x1].fill(color);
        }
    }

    /// `fill_rect` with blending, for glows and tinted panels.
    pub(crate) fn blend_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32, alpha: u8) {
        let Some((x0, x1, y0, y1)) = self.clip(x, y, i64::from(w), i64::from(h)) else {
            return;
        };
        for row in y0..y1 {
            let start = row * self.width;
            for p in &mut self.pixels[start + x0..start + x1] {
                *p = mix(*p, color, u32::from(alpha));
            }
        }
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        // i64 keeps `2 * err` and the differences exact for any i32 input.
        let (mut x, mut y) = (i64::from(x0), i64::from(y0));
        let (x1, y1) = (i64::from(x1), i64::from(y1));
        let dx = (x1 - x).abs();
        let dy = -(y1 - y).abs();
        let sx = if x < x1 { 1 } else { -1 };
        let sy = if y < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            // Off-screen points are simply not drawn: clipping per pixel is
            // the definition, and fast enough for the lines this game draws.
            if let (Ok(px), Ok(py)) = (i32::try_from(x), i32::try_from(y)) {
                self.set_pixel(px, py, color);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn blit(&mut self, sprite: &Sprite, x: i32, y: i32, flip: Flip) {
        // A sprite's sides are `usize`; anything too wide for an `i64` is
        // also far wider than a framebuffer, so saturating loses nothing.
        let sw = i64::try_from(sprite.width).unwrap_or(i64::MAX);
        let sh = i64::try_from(sprite.height).unwrap_or(i64::MAX);
        let Some((x0, x1, y0, y1)) = self.clip(x, y, sw, sh) else {
            return;
        };
        for dy in y0..y1 {
            // Column and row inside the sprite, before flipping. The clip
            // rectangle guarantees both are inside the sprite.
            let row = (dy as i64 - i64::from(y)) as usize;
            let src_row = if flip.y { sprite.height - 1 - row } else { row };
            let src = &sprite.pixels[src_row * sprite.width..(src_row + 1) * sprite.width];
            let dst = &mut self.pixels[dy * self.width..(dy + 1) * self.width];
            // Column inside the sprite of the first visible pixel.
            let first = (x0 as i64 - i64::from(x)) as usize;
            for (i, out) in dst[x0..x1].iter_mut().enumerate() {
                let col = first + i;
                let src_col = if flip.x { sprite.width - 1 - col } else { col };
                let p = src[src_col];
                if p >> 24 != 0 {
                    *out = p | OPAQUE;
                }
            }
        }
    }

    /// Draws `s` with the 5x7 font, top-left of the first cell at `(x, y)`.
    /// Returns the x coordinate where the next character would start.
    pub fn text(&mut self, s: &str, x: i32, y: i32, color: u32) -> i32 {
        let mut pen = i64::from(x);
        for ch in s.chars() {
            if ch != ' ' {
                let glyph = font::glyph(ch);
                // Skip glyphs that cannot touch the buffer; long strings
                // scrolled off-screen then cost nothing.
                if pen > -(font::ADVANCE as i64) && pen < self.width as i64 {
                    for (row, bits) in glyph.iter().enumerate() {
                        for col in 0..font::GLYPH_W {
                            if bits & (0b1_0000 >> col) != 0 {
                                self.set_pixel(
                                    pen as i32 + col,
                                    y.saturating_add(row as i32),
                                    color,
                                );
                            }
                        }
                    }
                }
            }
            pen += i64::from(font::ADVANCE);
        }
        pen.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }

    /// FNV-1a over the pixels, four little-endian bytes each.
    pub fn checksum(&self) -> u64 {
        let mut h = Fnv::new();
        for &p in &self.pixels {
            h.u32(p);
        }
        h.finish()
    }

    /// Writes the buffer as a binary PPM ("P6") image.
    pub fn write_ppm<W: Write>(&self, out: &mut W) -> io::Result<()> {
        write!(out, "P6\n{} {}\n255\n", self.width, self.height)?;
        // One row at a time on the stack: no allocation, few write calls.
        let mut row = [0u8; 3 * 256];
        for line in self.pixels.chunks(self.width) {
            for chunk in line.chunks(256) {
                for (i, &p) in chunk.iter().enumerate() {
                    row[3 * i] = (p >> 16) as u8;
                    row[3 * i + 1] = (p >> 8) as u8;
                    row[3 * i + 2] = p as u8;
                }
                out.write_all(&row[..3 * chunk.len()])?;
            }
        }
        Ok(())
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        let (x, y) = (usize::try_from(x).ok()?, usize::try_from(y).ok()?);
        (x < self.width && y < self.height).then(|| y * self.width + x)
    }

    /// Intersects the rectangle with the buffer. Returns `x0, x1, y0, y1`
    /// as half-open index ranges, or `None` if nothing is left.
    fn clip(&self, x: i32, y: i32, w: i64, h: i64) -> Option<(usize, usize, usize, usize)> {
        // In i64 the sum `x + w` is exact for any pair of i32 values;
        // saturating covers the sprite sides, which may be larger still.
        let x0 = i64::from(x).max(0);
        let y0 = i64::from(y).max(0);
        let x1 = i64::from(x).saturating_add(w).min(self.width as i64);
        let y1 = i64::from(y).saturating_add(h).min(self.height as i64);
        (x0 < x1 && y0 < y1).then_some((x0 as usize, x1 as usize, y0 as usize, y1 as usize))
    }
}

/// `(src * a + dst * (255 - a) + 127) / 255` on each colour channel.
fn mix(dst: u32, src: u32, a: u32) -> u32 {
    let channel = |shift: u32| {
        let s = (src >> shift) & 0xFF;
        let d = (dst >> shift) & 0xFF;
        ((s * a + d * (255 - a) + 127) / 255) << shift
    };
    OPAQUE | channel(16) | channel(8) | channel(0)
}

/// A small image with per-pixel transparency: alpha byte 0 means "skip".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sprite {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}

impl Sprite {
    /// Wraps row-major pixels. `None` if a side is zero or the length is not
    /// `width * height`.
    pub fn new(width: usize, height: usize, pixels: Vec<u32>) -> Option<Sprite> {
        (width > 0 && height > 0 && width.checked_mul(height) == Some(pixels.len())).then_some(
            Sprite {
                width,
                height,
                pixels,
            },
        )
    }

    /// Builds a sprite from text: one line per row, one character per pixel,
    /// each character looked up in `palette`.
    pub fn from_art(art: &str, palette: &[(char, u32)]) -> Result<Sprite, ArtError> {
        let mut width = 0;
        let mut height = 0;
        let mut pixels = Vec::new();
        for (i, line) in art.split('\n').enumerate() {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.is_empty() {
                continue;
            }
            let before = pixels.len();
            for ch in line.chars() {
                let color = palette
                    .iter()
                    .find(|(key, _)| *key == ch)
                    .map(|&(_, color)| color)
                    .ok_or(ArtError::UnknownChar { line: i + 1, ch })?;
                pixels.push(color);
            }
            let len = pixels.len() - before;
            if height == 0 {
                width = len;
            } else if len != width {
                return Err(ArtError::Ragged { line: i + 1 });
            }
            height += 1;
        }
        Sprite::new(width, height, pixels).ok_or(ArtError::Empty)
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// The pixel at `(x, y)`, or `None` outside the sprite.
    pub fn pixel(&self, x: usize, y: usize) -> Option<u32> {
        (x < self.width && y < self.height).then(|| self.pixels[y * self.width + x])
    }
}
