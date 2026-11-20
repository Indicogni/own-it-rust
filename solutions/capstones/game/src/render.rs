//! Drawing a `Game` (SPEC section 12).
//!
//! `render` takes `&self`: it cannot change the simulation, so drawing a
//! frame twice, or not at all, or at any frame rate, never changes what the
//! game does. Everything here is integer work on the caller's framebuffer;
//! nothing is allocated.

use std::fmt::Write as _;

use crate::art::{self, Art, ROCK_VARIANTS, scramble};
use crate::fb::Framebuffer;
use crate::fx::Look;
use crate::game::{
    FLYER_H, FLYER_W, Game, INVULN_TICKS, PLAYER_H, PLAYER_W, SPARK_SIZE, SUB, WALKER_H, WALKER_W,
};
use crate::level::TILE;
use crate::{Anim, EnemyKind, Flip, Phase, SCREEN_H, SCREEN_W, Tile};

const W: i32 = SCREEN_W as i32;
const H: i32 = SCREEN_H as i32;
const TILE_PX: i32 = TILE / SUB;

pub(crate) const HUD_H: i32 = 11;
pub(crate) const HUD_BG: u32 = 0xFF10_101C;
pub(crate) const HUD_FG: u32 = 0xFFF4_F0E0;

const SKY_TOP: (i32, i32, i32) = (0x10, 0x0C, 0x22);
const SKY_BOTTOM: (i32, i32, i32) = (0x30, 0x16, 0x2A);
const FAR_ROCK: u32 = 0xFF1A_1430;
const NEAR_ROCK: u32 = 0xFF22_1A38;
const WATER: u32 = 0xFF2A_5BC0;
const WATER_FOAM: u32 = 0xFF9F_DCFF;
const WARM: u32 = 0xFFFF_9A3C;
const GOLD: u32 = 0xFFFF_D75E;

/// A line of text built on the stack. `write!` into it never allocates;
/// text that does not fit is cut off.
struct Line {
    bytes: [u8; 64],
    len: usize,
}

impl Line {
    fn new() -> Line {
        Line {
            bytes: [0; 64],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        // Only whole `&str`s are ever copied in, so the bytes are UTF-8.
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

impl std::fmt::Write for Line {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len + s.len();
        if end > self.bytes.len() {
            return Err(std::fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// `from + (to - from) * alpha / 256`, rounded down.
fn lerp(from: i32, to: i32, alpha: i32) -> i32 {
    from + (((to - from) * alpha) >> 8)
}

/// A soft round light: strongest in the middle, gone at `radius`.
fn glow(fb: &mut Framebuffer, cx: i32, cy: i32, radius: i32, color: u32, strength: i32) {
    if cx + radius < 0 || cx - radius >= W || cy + radius < 0 || cy - radius >= H {
        return;
    }
    let r2 = radius * radius;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let d2 = dx * dx + dy * dy;
            if d2 < r2 {
                let t = (r2 - d2) * 255 / r2;
                let a = strength * t * t / (255 * 255);
                fb.blend_pixel(cx + dx, cy + dy, color, a as u8);
            }
        }
    }
}

fn centered(fb: &mut Framebuffer, s: &str, y: i32, color: u32) {
    let x = (W - s.chars().count() as i32 * 6) / 2;
    // A dark copy one pixel down and right keeps text readable on any scene.
    fb.text(s, x + 1, y + 1, 0xFF00_0000);
    fb.text(s, x, y, color);
}

impl Game {
    /// Draws the game as it is `alpha / 256` of the way from the start of
    /// the latest step to its end; 256 draws the current state exactly.
    pub fn render(&self, fb: &mut Framebuffer, alpha: u32) {
        let art = art::art();
        let alpha = alpha.min(256) as i32;
        let p = &self.player;
        let (px, py) = (lerp(p.prev.0, p.x, alpha), lerp(p.prev.1, p.y, alpha));
        let (cam_x, cam_y) = self.camera_for(px, py);
        let (shake_x, shake_y) = self.fx.shake_offset();
        // Map pixel -> screen pixel is "subtract `ox, oy`".
        let (ox, oy) = (cam_x + shake_x, cam_y + shake_y);
        let tick = self.tick as u32;

        self.draw_backdrop(fb, cam_x, cam_y, tick);
        self.draw_tiles(fb, art, ox, oy, tick);

        // The flue, standing on the bottom edge of its tile.
        let (ex, ey) = (self.exit.0 / SUB - ox, self.exit.1 / SUB - oy);
        let open = self.sparks_left == 0;
        if open {
            glow(fb, ex + 8, ey + 6, 34, WARM, 110);
            let frame = (tick / 6 % 2) as usize;
            fb.blit(&art.flue_open[frame], ex, ey - 16, Flip::NONE);
        } else {
            fb.blit(&art.flue_shut, ex, ey - 16, Flip::NONE);
        }

        // Light first, then the things that give it off, so sprites stay
        // crisp on top of their own glow.
        let nib_x = (px >> 8) - ox;
        let nib_y = (py >> 8) - oy;
        // A snuffed-out Nib gives no light.
        if self.phase != Phase::Lost {
            glow(fb, nib_x + 5, nib_y + 7, 30, WARM, 80);
        }

        for (i, spark) in self.sparks.iter().enumerate() {
            if spark.taken {
                continue;
            }
            let phase = tick / 8 + i as u32;
            let bob = [0, -1, -2, -1][(phase % 4) as usize];
            let x = spark.x / SUB - ox;
            let y = spark.y / SUB - oy + bob;
            glow(fb, x + 4, y + 4, 9, GOLD, 120);
            fb.blit(&art.spark[(phase / 2 % 2) as usize], x, y, Flip::NONE);
        }
        debug_assert_eq!(SPARK_SIZE / SUB, 8);

        for &(kind, id) in &self.roster {
            let Some(e) = self.enemies.get(id) else {
                continue;
            };
            let x = (lerp(e.prev.0, e.x, alpha) >> 8) - ox;
            let y = (lerp(e.prev.1, e.y, alpha) >> 8) - oy;
            let flip = Flip {
                x: e.vx > 0,
                y: false,
            };
            match kind {
                EnemyKind::Walker => {
                    let frame = (tick / 10 % 2) as usize;
                    // The 16x12 picture around the 12x10 box.
                    fb.blit(
                        &art.douser[frame],
                        x - (16 - WALKER_W / SUB) / 2,
                        y - (12 - WALKER_H / SUB),
                        flip,
                    );
                }
                EnemyKind::Flyer => {
                    let frame = (tick / 8 % 2) as usize;
                    fb.blit(
                        &art.mistwing[frame],
                        x - (16 - FLYER_W / SUB) / 2,
                        y - (12 - FLYER_H / SUB) / 2,
                        flip,
                    );
                }
            }
        }

        // Nib blinks while invulnerable and is not drawn once snuffed out.
        let blink = p.invuln > 0 && p.invuln < INVULN_TICKS && (p.invuln / 3).is_multiple_of(2);
        if self.phase != Phase::Lost && !blink {
            let (anim, frame) = self.anim();
            let (body, feet, lift) = match (anim, frame) {
                (Anim::Idle, 0) => (art::BODY_A, art::FEET_STAND, 0),
                (Anim::Idle, _) => (art::BODY_B, art::FEET_STAND, 0),
                (Anim::Run, 0) => (art::BODY_A, art::FEET_LEFT, 0),
                (Anim::Run, 2) => (art::BODY_B, art::FEET_RIGHT, 0),
                (Anim::Run, _) => (art::BODY_B, art::FEET_STAND, 1),
                (Anim::Jump, _) => (art::BODY_UP, art::FEET_TUCK, 0),
                (Anim::Fall, _) => (art::BODY_DOWN, art::FEET_STAND, 0),
            };
            let flip = Flip {
                x: p.facing < 0,
                y: false,
            };
            // The 16x16 picture around the 10x14 box, feet on the bottom.
            let x = nib_x - (16 - PLAYER_W / SUB) / 2;
            let y = nib_y - (16 - PLAYER_H / SUB);
            fb.blit(&art.nib_bodies[body], x, y - lift, flip);
            fb.blit(&art.nib_feet[feet], x, y + 14, flip);
        }

        for part in self.fx.particles() {
            let x = part.x / SUB - ox;
            let y = part.y / SUB - oy;
            let young = part.life * 2 > part.span;
            let color = match (part.look, young) {
                (Look::Dust, true) => 0xFFB9_A99A,
                (Look::Dust, false) => 0xFF6E_6272,
                (Look::Ember, true) => 0xFFFF_C94A,
                (Look::Ember, false) => 0xFFC2_501E,
                (Look::Splash, true) => 0xFF9F_DCFF,
                (Look::Splash, false) => 0xFF3F_7FD9,
                (Look::Glint, true) => 0xFFFF_F3C4,
                (Look::Glint, false) => 0xFFFF_C94A,
            };
            let size = if young { 2 } else { 1 };
            fb.fill_rect(x, y, size, size, color);
        }

        self.draw_hud(fb);
        self.draw_messages(fb);
    }

    /// The far wall of the cavern: a gradient, two layers of rock teeth
    /// that scroll slower than the map, and drifting motes of ash.
    fn draw_backdrop(&self, fb: &mut Framebuffer, cam_x: i32, cam_y: i32, tick: u32) {
        for y in 0..H {
            let mix = |a: i32, b: i32| (a + (b - a) * y / (H - 1)) as u32;
            let color = 0xFF00_0000
                | mix(SKY_TOP.0, SKY_BOTTOM.0) << 16
                | mix(SKY_TOP.1, SKY_BOTTOM.1) << 8
                | mix(SKY_TOP.2, SKY_BOTTOM.2);
            fb.fill_rect(0, y, W, 1, color);
        }
        // Each layer: (colour, scroll divisor, tooth width, tallest tooth).
        for (layer, &(color, slow, cell, tall)) in [(FAR_ROCK, 4, 26, 46), (NEAR_ROCK, 2, 38, 64)]
            .iter()
            .enumerate()
        {
            let lift = cam_y / (slow * 2);
            for x in 0..W {
                let wx = x + cam_x / slow;
                let (n, inside) = (wx.div_euclid(cell), wx.rem_euclid(cell));
                // A tooth is a triangle: tallest in the middle of its cell.
                let slope = cell / 2 - (inside - cell / 2).abs();
                let up = 12 + (scramble(n as u32, layer as u32, 1) % tall as u32) as i32;
                let down = 6 + (scramble(n as u32, layer as u32, 2) % (tall as u32 / 2)) as i32;
                let rise = up * slope / (cell / 2);
                let drop = down * slope / (cell / 2);
                fb.fill_rect(x, H - rise - lift + 8, 1, rise + lift, color);
                fb.fill_rect(x, HUD_H, 1, drop, color);
            }
        }
        for i in 0..28u32 {
            let speed = 1 + scramble(i, 1, 0) % 3;
            let x = (scramble(i, 2, 0) % 400) as i32 - cam_x / 3;
            // The rise wraps around the screen, so reduce it before it meets
            // an `i32`: a session of any length then stays in range.
            let rise = (self.tick * u64::from(speed) / 8 % H as u64) as i32;
            let y = (scramble(i, 3, 0) % 240) as i32 - rise - cam_y / 3;
            let sway = [0, 1, 1, 0, -1, -1][((tick / 16 + i) % 6) as usize];
            fb.blend_pixel(
                (x + sway).rem_euclid(W),
                y.rem_euclid(H),
                WARM,
                (60 + scramble(i, 4, 0) % 120) as u8,
            );
        }
    }

    fn draw_tiles(&self, fb: &mut Framebuffer, art: &Art, ox: i32, oy: i32, tick: u32) {
        let cols = ox.div_euclid(TILE_PX)..=(ox + W - 1).div_euclid(TILE_PX);
        let rows = oy.div_euclid(TILE_PX)..=(oy + H - 1).div_euclid(TILE_PX);
        for row in rows {
            if row < 0 || row >= self.map.rows() {
                continue;
            }
            for col in cols.clone() {
                if col < 0 || col >= self.map.cols() {
                    continue;
                }
                let (x, y) = (col * TILE_PX - ox, row * TILE_PX - oy);
                let edges = self.edges[(row * self.map.cols() + col) as usize];
                match self.map.tile(col, row) {
                    Tile::Empty => {}
                    Tile::Solid => {
                        let variant = scramble(col as u32, row as u32, 11) as usize % ROCK_VARIANTS;
                        let sprite = &art.rock[edges as usize * ROCK_VARIANTS + variant];
                        fb.blit(sprite, x, y, Flip::NONE);
                    }
                    Tile::Water => {
                        let surface = edges & 1 != 0;
                        let top = if surface { 4 } else { 0 };
                        fb.blend_rect(x, y + top, TILE_PX, TILE_PX - top, WATER, 170);
                        if surface {
                            // Foam: a dashed line that crawls sideways.
                            for dx in 0..TILE_PX {
                                let wave = (col * TILE_PX + dx + (tick / 6) as i32).rem_euclid(8);
                                if wave < 5 {
                                    let dip = i32::from(wave == 0 || wave == 4);
                                    fb.set_pixel(x + dx, y + top + dip, WATER_FOAM);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// SPEC 12.4: the one part of the picture that is specified exactly.
    fn draw_hud(&self, fb: &mut Framebuffer) {
        fb.fill_rect(0, 0, W, HUD_H, HUD_BG);
        let total = self.sparks.len() as u32;
        let mut line = Line::new();
        let _ = write!(
            line,
            "SPARKS {}/{}  LIVES {}  SCORE {:06}",
            total - self.sparks_left,
            total,
            self.lives,
            self.score
        );
        fb.text(line.as_str(), 4, 2, HUD_FG);
    }

    fn draw_messages(&self, fb: &mut Framebuffer) {
        match self.phase {
            Phase::Playing => {
                if self.tick < 240 {
                    let goal = if self.sparks.is_empty() {
                        "REACH THE FLUE"
                    } else {
                        "GATHER EVERY SPARK, THEN LIGHT THE FLUE"
                    };
                    fb.blend_rect(0, H - 15, W, 15, 0xFF00_0000, 150);
                    centered(fb, goal, H - 11, HUD_FG);
                }
            }
            Phase::Won | Phase::Lost => {
                fb.blend_rect(0, HUD_H, W, H - HUD_H, 0xFF00_0000, 120);
                let (title, color) = if self.phase == Phase::Won {
                    ("THE FLUE IS LIT!", GOLD)
                } else {
                    ("SNUFFED OUT", 0xFF9F_DCFF)
                };
                fb.fill_rect(60, 62, 200, 56, HUD_BG);
                fb.fill_rect(60, 62, 200, 1, color);
                fb.fill_rect(60, 117, 200, 1, color);
                centered(fb, title, 72, color);
                let mut line = Line::new();
                let _ = write!(line, "SCORE {:06}", self.score);
                centered(fb, line.as_str(), 88, HUD_FG);
                let prompt = if self.phase == Phase::Won {
                    "PRESS R TO GO ON"
                } else {
                    "PRESS R TO TRY AGAIN"
                };
                centered(fb, prompt, 102, HUD_FG);
            }
        }
    }
}
