//! Underkiln: the reference solution of the game capstone.
//!
//! The crate is split along the line that makes a graphical game testable:
//!
//! - `fb`, `font`: the software rasteriser. Pure functions of their inputs.
//! - `game`, `level`, `store`, `rng`, `fx`: the simulation. Integer state
//!   advanced one fixed step at a time; no clock, no window, no floats.
//! - `render`, `art`: turns simulation state into pixels. Reads, never
//!   writes, the simulation.
//! - `timestep`, `replay`: the pieces that connect the simulation to real
//!   time and to recorded input.
//!
//! The window lives in the binary (`src/platform.rs`), behind the `window`
//! feature. Nothing in this library knows it exists. `DESIGN.md` explains
//! the choices.

#![forbid(unsafe_code)]

mod art;
mod fb;
mod font;
mod fx;
mod game;
mod hash;
mod level;
mod render;
mod replay;
mod rng;
mod store;
mod timestep;

pub use fb::{Framebuffer, Sprite};
pub use game::Game;
pub use rng::Rng;
pub use timestep::FixedStep;

// ---------------------------------------------------------------------------
// Plain data named by the SPEC. Identical to the starter from here on.
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
