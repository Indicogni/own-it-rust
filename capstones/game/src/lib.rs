//! Underkiln: your game.
//!
//! This file is the API facade from SPEC section 15. The tests reach your
//! game only through the names below (and through the `game` binary), so
//! keep every name and signature. Everything behind them is yours to
//! design: replace the placeholder fields, add modules, move code out of
//! this file. Read SPEC.md first, then work through the milestones in order.

// Remove this line when nothing is left to do; it only keeps the starter
// free of warnings about the placeholders below.
#![allow(unused_variables, dead_code)]

use std::io;

// ---------------------------------------------------------------------------
// M1: framebuffer and rasteriser (SPEC sections 3 to 5)
// ---------------------------------------------------------------------------

/// A rectangle of pixels in row-major order. Every stored pixel is opaque.
pub struct Framebuffer {
    todo: (),
}

impl Framebuffer {
    /// A `width` by `height` buffer filled with opaque black (`0xFF000000`).
    /// Panics if a side is zero or larger than 16384.
    pub fn new(width: usize, height: usize) -> Framebuffer {
        todo!("M1: Framebuffer::new")
    }

    pub fn width(&self) -> usize {
        todo!("M1: Framebuffer::width")
    }

    pub fn height(&self) -> usize {
        todo!("M1: Framebuffer::height")
    }

    /// All pixels, row by row from the top: `pixels()[y * width + x]`.
    pub fn pixels(&self) -> &[u32] {
        todo!("M1: Framebuffer::pixels")
    }

    /// Sets every pixel to `color` with the alpha byte forced to `0xFF`.
    pub fn clear(&mut self, color: u32) {
        todo!("M1: Framebuffer::clear")
    }

    /// The pixel at `(x, y)`, or `None` outside the buffer.
    pub fn get_pixel(&self, x: i32, y: i32) -> Option<u32> {
        todo!("M1: Framebuffer::get_pixel")
    }

    /// Sets one pixel (alpha forced to `0xFF`). Does nothing outside the
    /// buffer.
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        todo!("M1: Framebuffer::set_pixel")
    }

    /// Mixes `color` over the pixel at `(x, y)` with weight `alpha / 255`
    /// (SPEC 4.5). Does nothing outside the buffer.
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: u32, alpha: u8) {
        todo!("M1: Framebuffer::blend_pixel")
    }

    /// Fills the pixels with `x <= px < x + w` and `y <= py < y + h`,
    /// clipped to the buffer. Must not overflow for any arguments.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        todo!("M1: Framebuffer::fill_rect")
    }

    /// Draws the line from `(x0, y0)` to `(x1, y1)`, both ends included,
    /// with the algorithm in SPEC 4.3, clipped to the buffer.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        todo!("M1: Framebuffer::line")
    }

    /// Copies `sprite` with its top-left corner at `(x, y)`, skipping
    /// transparent pixels, mirrored as `flip` says, clipped to the buffer.
    pub fn blit(&mut self, sprite: &Sprite, x: i32, y: i32, flip: Flip) {
        todo!("M1: Framebuffer::blit")
    }

    /// Draws `s` with the 5x7 font (SPEC section 5); the first character
    /// cell has its top-left corner at `(x, y)`. Returns the x coordinate
    /// where the next character would start.
    pub fn text(&mut self, s: &str, x: i32, y: i32, color: u32) -> i32 {
        todo!("M1: Framebuffer::text")
    }

    /// The FNV-1a checksum of the pixels (SPEC 3.3).
    pub fn checksum(&self) -> u64 {
        todo!("M1: Framebuffer::checksum")
    }

    /// Writes the buffer as a binary PPM image (SPEC 3.4).
    pub fn write_ppm<W: io::Write>(&self, out: &mut W) -> io::Result<()> {
        todo!("M1: Framebuffer::write_ppm")
    }
}

/// A small image with per-pixel transparency: a pixel whose alpha byte is
/// zero is not drawn.
pub struct Sprite {
    todo: (),
}

impl Sprite {
    /// Wraps row-major pixels. `None` if a side is zero or
    /// `pixels.len() != width * height`.
    pub fn new(width: usize, height: usize, pixels: Vec<u32>) -> Option<Sprite> {
        todo!("M1: Sprite::new")
    }

    /// Builds a sprite from text: one line per row, one character per
    /// pixel, each character looked up in `palette` (SPEC 4.4).
    pub fn from_art(art: &str, palette: &[(char, u32)]) -> Result<Sprite, ArtError> {
        todo!("M1: Sprite::from_art")
    }

    pub fn width(&self) -> usize {
        todo!("M1: Sprite::width")
    }

    pub fn height(&self) -> usize {
        todo!("M1: Sprite::height")
    }

    /// The stored pixel at `(x, y)`, or `None` outside the sprite.
    pub fn pixel(&self, x: usize, y: usize) -> Option<u32> {
        todo!("M1: Sprite::pixel")
    }
}

// ---------------------------------------------------------------------------
// M2: the fixed timestep (SPEC section 6)
// ---------------------------------------------------------------------------

/// Turns irregular frame times into whole simulation steps.
pub struct FixedStep {
    todo: (),
}

impl FixedStep {
    /// `step_micros` is the length of one step; `max_steps` caps how many
    /// steps one `advance` may return. Panics if `step_micros` is zero.
    pub fn new(step_micros: u64, max_steps: u32) -> FixedStep {
        todo!("M2: FixedStep::new")
    }

    /// Adds `elapsed_micros` of real time; returns how many simulation
    /// steps to run now.
    pub fn advance(&mut self, elapsed_micros: u64) -> u32 {
        todo!("M2: FixedStep::advance")
    }

    /// How far the left-over time reaches into the next step, in 256ths:
    /// always in `0..=255`.
    pub fn alpha(&self) -> u32 {
        todo!("M2: FixedStep::alpha")
    }
}

// ---------------------------------------------------------------------------
// M3: the seeded generator (SPEC section 7)
// ---------------------------------------------------------------------------

/// The generator from SPEC section 7.
pub struct Rng {
    todo: (),
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        todo!("M3: Rng::new")
    }

    pub fn next_u64(&mut self) -> u64 {
        todo!("M3: Rng::next_u64")
    }

    /// The high 32 bits of `next_u64`.
    pub fn next_u32(&mut self) -> u32 {
        todo!("M3: Rng::next_u32")
    }

    /// A value in `0..n`; `below(0)` is 0. Always consumes one draw.
    pub fn below(&mut self, n: u32) -> u32 {
        todo!("M3: Rng::below")
    }

    /// The current 64-bit state.
    pub fn state(&self) -> u64 {
        todo!("M3: Rng::state")
    }
}

// ---------------------------------------------------------------------------
// M3 to M5: the game (SPEC sections 8 to 12)
// ---------------------------------------------------------------------------

/// One attempt at one level.
pub struct Game {
    todo: (),
}

impl Game {
    /// Builds a game from level text (SPEC section 8) and a seed.
    pub fn new(level: &str, seed: u64) -> Result<Game, LevelError> {
        todo!("M3: Game::new")
    }

    /// Advances the game by one fixed step (SPEC section 10) and reports
    /// what happened in it.
    pub fn step(&mut self, input: Input) -> Events {
        todo!("M4: Game::step")
    }

    /// Draws the game into `fb` (SPEC section 12). `alpha` is in `0..=256`:
    /// how far to draw between the start (0) and the end (256) of the
    /// latest step. Must not change the game.
    pub fn render(&self, fb: &mut Framebuffer, alpha: u32) {
        todo!("M5: Game::render")
    }

    /// How many steps have been run.
    pub fn tick(&self) -> u64 {
        todo!("M3: Game::tick")
    }

    pub fn phase(&self) -> Phase {
        todo!("M4: Game::phase")
    }

    pub fn score(&self) -> u32 {
        todo!("M4: Game::score")
    }

    pub fn lives(&self) -> u32 {
        todo!("M4: Game::lives")
    }

    /// The player's box and velocity.
    pub fn player(&self) -> Body {
        todo!("M3: Game::player")
    }

    pub fn on_ground(&self) -> bool {
        todo!("M4: Game::on_ground")
    }

    /// The map size in tiles: `(columns, rows)`.
    pub fn map_size(&self) -> (u32, u32) {
        todo!("M3: Game::map_size")
    }

    /// The tile at `(col, row)`, with the outside-the-map rule of SPEC 8.3.
    pub fn tile(&self, col: i32, row: i32) -> Tile {
        todo!("M3: Game::tile")
    }

    /// How many enemies the level started with.
    pub fn enemy_count(&self) -> usize {
        todo!("M3: Game::enemy_count")
    }

    /// Enemy number `index` in spawn order, or `None` past the end.
    pub fn enemy(&self, index: usize) -> Option<EnemyView> {
        todo!("M3: Game::enemy")
    }

    /// Sparks not yet collected.
    pub fn sparks_left(&self) -> u32 {
        todo!("M3: Game::sparks_left")
    }

    /// Top-left corner of the view in map pixels, without shake (SPEC 8.5).
    pub fn camera(&self) -> (i32, i32) {
        todo!("M3: Game::camera")
    }

    /// The screen-shake offset in pixels for the current step (SPEC 12.3).
    pub fn shake(&self) -> (i32, i32) {
        todo!("M5: Game::shake")
    }

    /// The number of live particles (SPEC 12.2).
    pub fn particles(&self) -> usize {
        todo!("M5: Game::particles")
    }

    /// The player's animation and the frame within it (SPEC 12.1).
    pub fn anim(&self) -> (Anim, u32) {
        todo!("M5: Game::anim")
    }

    /// The checksum of the canonical state (SPEC section 11).
    pub fn state_hash(&self) -> u64 {
        todo!("M4: Game::state_hash")
    }
}

// ---------------------------------------------------------------------------
// M6: replays (SPEC section 13)
// ---------------------------------------------------------------------------

impl Replay {
    /// The canonical replay text.
    pub fn to_text(&self) -> String {
        todo!("M6: Replay::to_text")
    }

    /// Parses replay text.
    pub fn from_text(text: &str) -> Result<Replay, ReplayError> {
        todo!("M6: Replay::from_text")
    }
}

// ---------------------------------------------------------------------------
// Plain data named by the SPEC. These are complete; you do not need to
// change them (but you may add derives or methods).
// ---------------------------------------------------------------------------

/// Width of the game picture in pixels.
pub const SCREEN_W: usize = 320;
/// Height of the game picture in pixels.
pub const SCREEN_H: usize = 180;
/// Length of one simulation step in microseconds (about 1/60 s).
pub const STEP_MICROS: u64 = 16_667;
/// The most simulation steps the game loop runs for one displayed frame.
pub const MAX_STEPS: u32 = 5;

/// Mirroring applied by `Framebuffer::blit`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flip {
    /// Mirror left to right.
    pub x: bool,
    /// Mirror top to bottom.
    pub y: bool,
}

impl Flip {
    pub const NONE: Flip = Flip { x: false, y: false };
    pub const X: Flip = Flip { x: true, y: false };
    pub const Y: Flip = Flip { x: false, y: true };
    pub const BOTH: Flip = Flip { x: true, y: true };
}

/// The buttons held during one simulation step, as a bit mask.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input(pub u8);

impl Input {
    pub const NONE: Input = Input(0);
    pub const LEFT: u8 = 1;
    pub const RIGHT: u8 = 2;
    pub const JUMP: u8 = 4;
    /// Every defined button. A replay may not contain any other bit.
    pub const ALL: u8 = 7;

    /// Is `button` (one of the constants above) held?
    pub fn held(self, button: u8) -> bool {
        self.0 & button != 0
    }
}

/// What happened during one simulation step, as a bit mask.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events(pub u32);

impl Events {
    pub const JUMPED: u32 = 1;
    pub const LANDED: u32 = 2;
    pub const SPARK: u32 = 4;
    pub const STOMP: u32 = 8;
    pub const HURT: u32 = 16;
    pub const WON: u32 = 32;
    pub const LOST: u32 = 64;

    /// Did `event` (one of the constants above) happen?
    pub fn has(self, event: u32) -> bool {
        self.0 & event != 0
    }
}

/// Where a game stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Playing,
    Won,
    Lost,
}

/// One cell of the tile map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Empty,
    Solid,
    Water,
}

/// The two kinds of enemy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    /// A Douser: walks along the ground.
    Walker,
    /// A Mistwing: drifts through the air.
    Flyer,
}

/// Position and velocity of the player's box, in subpixels and subpixels
/// per step. `(x, y)` is the top-left corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
}

/// What the outside may know about one enemy. A removed enemy reports
/// `alive: false` and `x == 0 && y == 0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyView {
    pub kind: EnemyKind,
    pub alive: bool,
    pub x: i32,
    pub y: i32,
}

/// The player's animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anim {
    Idle,
    Run,
    Jump,
    Fall,
}

/// Why `Sprite::from_art` refused its input. Lines count from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtError {
    /// No non-blank line.
    Empty,
    /// This line is not as long as the first non-blank line.
    Ragged { line: usize },
    /// This character is not in the palette.
    UnknownChar { line: usize, ch: char },
}

/// Why `Game::new` refused a level. Lines and columns count from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelError {
    /// No lines, or an empty first line.
    Empty,
    /// More than 256 columns or more than 256 rows.
    TooLarge,
    /// This line is not as long as the first line.
    Ragged {
        line: usize,
    },
    /// A character outside the level alphabet.
    UnknownChar {
        line: usize,
        column: usize,
        ch: char,
    },
    NoPlayer,
    ManyPlayers,
    NoExit,
    ManyExits,
}

/// Why `Replay::from_text` refused its input. Lines count from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayError {
    BadHeader,
    BadSeed,
    BadTicks,
    /// This line is not a valid `count mask` run.
    BadRun {
        line: usize,
    },
    /// The runs add up to `found` steps but the header promised `expected`.
    LengthMismatch {
        expected: u64,
        found: u64,
    },
}

/// A recorded game: the seed and the input of every step, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Replay {
    pub seed: u64,
    pub inputs: Vec<Input>,
}

impl std::fmt::Display for ArtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtError::Empty => write!(f, "sprite art has no rows"),
            ArtError::Ragged { line } => write!(f, "sprite art line {line} has a different length"),
            ArtError::UnknownChar { line, ch } => {
                write!(f, "sprite art line {line}: {ch:?} is not in the palette")
            }
        }
    }
}

impl std::fmt::Display for LevelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LevelError::Empty => write!(f, "the level is empty"),
            LevelError::TooLarge => write!(f, "the level is larger than 256 by 256 tiles"),
            LevelError::Ragged { line } => write!(f, "level line {line} has a different length"),
            LevelError::UnknownChar { line, column, ch } => {
                write!(
                    f,
                    "level line {line}, column {column}: unknown character {ch:?}"
                )
            }
            LevelError::NoPlayer => write!(f, "the level has no player start `P`"),
            LevelError::ManyPlayers => write!(f, "the level has more than one player start `P`"),
            LevelError::NoExit => write!(f, "the level has no exit `E`"),
            LevelError::ManyExits => write!(f, "the level has more than one exit `E`"),
        }
    }
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::BadHeader => write!(f, "not an Underkiln replay (bad first line)"),
            ReplayError::BadSeed => write!(f, "the replay has no valid `seed` line"),
            ReplayError::BadTicks => write!(f, "the replay has no valid `ticks` line"),
            ReplayError::BadRun { line } => write!(f, "replay line {line} is not `count mask`"),
            ReplayError::LengthMismatch { expected, found } => {
                write!(f, "the replay promises {expected} steps but holds {found}")
            }
        }
    }
}

impl std::error::Error for ArtError {}
impl std::error::Error for LevelError {}
impl std::error::Error for ReplayError {}
