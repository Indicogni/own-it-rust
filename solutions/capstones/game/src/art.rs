//! The pictures: sprites parsed from `assets/sprites.txt`, plus the rock
//! tiles and the flue, which are generated in code.
//!
//! All of it is built once, the first time a game is created, and then only
//! read. A `OnceLock` gives that to every `Game` without passing a handle
//! around and without any work per frame.

use std::sync::OnceLock;

use crate::fb::Sprite;

const SHEET: &str = include_str!("../assets/sprites.txt");

/// How many different speckle patterns each rock tile shape comes in.
pub(crate) const ROCK_VARIANTS: usize = 4;

pub(crate) struct Art {
    pub(crate) nib_bodies: [Sprite; 4],
    pub(crate) nib_feet: [Sprite; 4],
    pub(crate) douser: [Sprite; 2],
    pub(crate) mistwing: [Sprite; 2],
    pub(crate) spark: [Sprite; 2],
    /// Index `edges * ROCK_VARIANTS + variant`.
    pub(crate) rock: Vec<Sprite>,
    pub(crate) flue_shut: Sprite,
    pub(crate) flue_open: [Sprite; 2],
}

/// Indices into `Art::nib_bodies`.
pub(crate) const BODY_A: usize = 0;
pub(crate) const BODY_B: usize = 1;
pub(crate) const BODY_UP: usize = 2;
pub(crate) const BODY_DOWN: usize = 3;
/// Indices into `Art::nib_feet`.
pub(crate) const FEET_STAND: usize = 0;
pub(crate) const FEET_LEFT: usize = 1;
pub(crate) const FEET_RIGHT: usize = 2;
pub(crate) const FEET_TUCK: usize = 3;

pub(crate) fn art() -> &'static Art {
    static ART: OnceLock<Art> = OnceLock::new();
    ART.get_or_init(build)
}

fn build() -> Art {
    let sheet = Sheet::parse(SHEET);
    let get = |name: &str| sheet.sprite(name);
    Art {
        nib_bodies: [
            get("nib_body_a"),
            get("nib_body_b"),
            get("nib_body_up"),
            get("nib_body_down"),
        ],
        nib_feet: [
            get("nib_feet_stand"),
            get("nib_feet_left"),
            get("nib_feet_right"),
            get("nib_feet_tuck"),
        ],
        douser: [get("douser_a"), get("douser_b")],
        mistwing: [get("mistwing_a"), get("mistwing_b")],
        spark: [get("spark_a"), get("spark_b")],
        rock: (0..16 * ROCK_VARIANTS)
            .map(|i| rock_tile((i / ROCK_VARIANTS) as u8, (i % ROCK_VARIANTS) as u32))
            .collect(),
        flue_shut: flue(None),
        flue_open: [flue(Some(0)), flue(Some(1))],
    }
}

/// The parsed sprite sheet: a palette and named blocks of rows.
struct Sheet<'a> {
    palette: Vec<(char, u32)>,
    sprites: Vec<(&'a str, String)>,
}

impl<'a> Sheet<'a> {
    /// The sheet is compiled into the program, so a mistake in it is a bug
    /// in this crate, not bad input from a user: panic with a clear message.
    fn parse(text: &'a str) -> Sheet<'a> {
        let mut sheet = Sheet {
            palette: Vec::new(),
            sprites: Vec::new(),
        };
        for line in text.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("color ") {
                let (key, hex) = rest
                    .split_once(' ')
                    .expect("sprite sheet: `color <c> <hex>`");
                let key = key.chars().next().expect("sprite sheet: empty colour key");
                let value = u32::from_str_radix(hex, 16).expect("sprite sheet: bad colour");
                sheet.palette.push((key, value));
            } else if let Some(name) = line.strip_prefix("sprite ") {
                sheet.sprites.push((name, String::new()));
            } else {
                let (_, rows) = sheet
                    .sprites
                    .last_mut()
                    .expect("sprite sheet: pixels before the first `sprite` line");
                rows.push_str(line);
                rows.push('\n');
            }
        }
        sheet
    }

    fn sprite(&self, name: &str) -> Sprite {
        let (_, rows) = self
            .sprites
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("sprite sheet: no sprite named {name}"));
        Sprite::from_art(rows, &self.palette)
            .unwrap_or_else(|e| panic!("sprite sheet: {name}: {e}"))
    }
}

/// A cheap, fixed scramble of three numbers; the "noise" behind the rock.
pub(crate) fn scramble(a: u32, b: u32, c: u32) -> u32 {
    let mut h = a
        .wrapping_mul(0x9E37_79B1)
        .wrapping_add(b.wrapping_mul(0x85EB_CA6B))
        .wrapping_add(c.wrapping_mul(0xC2B2_AE35));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h
}

const TOP: u8 = 1;
const RIGHT: u8 = 2;
const BOTTOM: u8 = 4;
const LEFT: u8 = 8;

/// One 16x16 rock tile. `edges` says which sides face open air: an open top
/// gets a crust of baked clay, the other open sides get light or shadow.
fn rock_tile(edges: u8, variant: u32) -> Sprite {
    const BASE: u32 = 0xFF3B_3352;
    const DARK: u32 = 0xFF2C_2641;
    const LIGHT: u32 = 0xFF4C_4468;
    const SEAM: u32 = 0xFF24_1F36;
    const CRUST: u32 = 0xFF9A_5C3E;
    const CRUST_LIGHT: u32 = 0xFFD0_8A52;
    const CRUST_DARK: u32 = 0xFF6E_3F33;

    let mut pixels = Vec::with_capacity(256);
    for y in 0..16u32 {
        for x in 0..16u32 {
            let n = scramble(x, y, variant) % 100;
            let mut c = match n {
                0..=13 => DARK,
                14..=24 => LIGHT,
                _ => BASE,
            };
            // Cracks: a few short dark seams, different in each variant.
            let seam_row = 4 + scramble(variant, 7, 1) % 8;
            let seam_from = scramble(variant, 9, 2) % 8;
            if y == seam_row && (seam_from..seam_from + 6).contains(&x) {
                c = SEAM;
            }
            if edges & LEFT != 0 && x == 0 {
                c = LIGHT;
            }
            if edges & RIGHT != 0 && x == 15 {
                c = SEAM;
            }
            if edges & BOTTOM != 0 {
                // A ragged dark underside.
                let depth = 1 + scramble(x, variant, 3) % 3;
                if y >= 16 - depth {
                    c = SEAM;
                }
            }
            if edges & TOP != 0 {
                let depth = 3 + scramble(x / 2, variant, 5) % 2;
                if y == 0 {
                    c = CRUST_LIGHT;
                } else if y < depth {
                    c = CRUST;
                } else if y == depth {
                    c = CRUST_DARK;
                }
            }
            pixels.push(c);
        }
    }
    Sprite::new(16, 16, pixels).expect("16 * 16 pixels")
}

/// The flue: a 16x32 stone arch. Shut, it holds a dark grate. Open, it is
/// full of fire; `frame` picks one of two flicker patterns.
fn flue(frame: Option<u32>) -> Sprite {
    const STONE: u32 = 0xFF6B_6480;
    const STONE_DARK: u32 = 0xFF48_425C;
    const STONE_LIGHT: u32 = 0xFF8C_86A3;
    const SOOT: u32 = 0xFF14_101C;
    const BAR: u32 = 0xFF3A_3550;
    const FIRE: [u32; 4] = [0xFF8F_2A1E, 0xFFC2_501E, 0xFFEE_7B2E, 0xFFFF_C94A];

    let mut pixels = Vec::with_capacity(16 * 32);
    for y in 0..32i32 {
        for x in 0..16i32 {
            // The cap is the full width; the shaft below is one pixel
            // narrower on each side. The top corners are cut off.
            let in_cap = (2..6).contains(&y) && !(y == 2 && (x == 0 || x == 15));
            let in_shaft = y >= 6 && (1..15).contains(&x);
            // The opening: columns 4..12 with a rounded top.
            let round = match y {
                11 => 3,
                12 => 1,
                _ => 0,
            };
            let in_opening = y >= 11 && (4 + round..12 - round).contains(&x);
            let c = if in_opening {
                match frame {
                    None => {
                        if x % 3 == 1 || y == 20 {
                            BAR
                        } else {
                            SOOT
                        }
                    }
                    Some(f) => {
                        // Hotter toward the bottom, with a ragged edge that
                        // moves between the two frames.
                        let jitter = (scramble(x as u32, y as u32 / 2, f) % 5) as i32;
                        FIRE[((y - 11 + jitter) / 6).clamp(0, 3) as usize]
                    }
                }
            } else if in_cap {
                if y == 2 { STONE_LIGHT } else { STONE }
            } else if in_shaft {
                // Brick courses: a seam every fourth row, joints staggered.
                let course = y / 4;
                if y % 4 == 1 || (x + course * 4) % 8 == 0 {
                    STONE_DARK
                } else if x == 1 {
                    STONE_LIGHT
                } else {
                    STONE
                }
            } else {
                0
            };
            pixels.push(c);
        }
    }
    Sprite::new(16, 32, pixels).expect("16 * 32 pixels")
}
