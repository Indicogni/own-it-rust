//! Level text to tile map (SPEC section 8).

use crate::{EnemyKind, LevelError, Tile};

/// Side of a tile in subpixels.
pub(crate) const TILE: i32 = 4096;
const MAX_SIDE: usize = 256;

/// The static part of a level: the tile grid and its size.
#[derive(Clone)]
pub(crate) struct Map {
    cols: i32,
    rows: i32,
    tiles: Vec<Tile>,
}

/// Everything `Game::new` needs from the level text. Positions are tile
/// coordinates `(col, row)`, in reading order within each list.
pub(crate) struct Parsed {
    pub(crate) map: Map,
    pub(crate) player: (i32, i32),
    pub(crate) exit: (i32, i32),
    pub(crate) enemies: Vec<(EnemyKind, i32, i32)>,
    pub(crate) sparks: Vec<(i32, i32)>,
}

impl Map {
    pub(crate) fn cols(&self) -> i32 {
        self.cols
    }

    pub(crate) fn rows(&self) -> i32 {
        self.rows
    }

    /// Width and height in subpixels.
    pub(crate) fn size(&self) -> (i32, i32) {
        (self.cols * TILE, self.rows * TILE)
    }

    /// The tile at `(col, row)` with the outside-the-map rule applied: rock
    /// to the left, to the right and above; open air below.
    pub(crate) fn tile(&self, col: i32, row: i32) -> Tile {
        if col < 0 || col >= self.cols || row < 0 {
            Tile::Solid
        } else if row >= self.rows {
            Tile::Empty
        } else {
            self.tiles[(row * self.cols + col) as usize]
        }
    }

    pub(crate) fn solid(&self, col: i32, row: i32) -> bool {
        self.tile(col, row) == Tile::Solid
    }

    /// Is any tile of column `col` solid between subpixel rows `y` and
    /// `y + h - 1`?
    pub(crate) fn column_blocked(&self, col: i32, y: i32, h: i32) -> bool {
        (y.div_euclid(TILE)..=(y + h - 1).div_euclid(TILE)).any(|row| self.solid(col, row))
    }

    /// Is any tile of row `row` solid between subpixel columns `x` and
    /// `x + w - 1`?
    pub(crate) fn row_blocked(&self, row: i32, x: i32, w: i32) -> bool {
        (x.div_euclid(TILE)..=(x + w - 1).div_euclid(TILE)).any(|col| self.solid(col, row))
    }

    /// Does the box touch a water tile?
    pub(crate) fn touches_water(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        (y.div_euclid(TILE)..=(y + h - 1).div_euclid(TILE)).any(|row| {
            (x.div_euclid(TILE)..=(x + w - 1).div_euclid(TILE))
                .any(|col| self.tile(col, row) == Tile::Water)
        })
    }
}

pub(crate) fn parse(text: &str) -> Result<Parsed, LevelError> {
    let mut lines: Vec<&str> = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    let Some(first) = lines.first() else {
        return Err(LevelError::Empty);
    };
    let cols = first.chars().count();
    if cols == 0 {
        return Err(LevelError::Empty);
    }
    if cols > MAX_SIDE || lines.len() > MAX_SIDE {
        return Err(LevelError::TooLarge);
    }

    let mut tiles = Vec::with_capacity(cols * lines.len());
    let mut players = Vec::new();
    let mut exits = Vec::new();
    let mut enemies = Vec::new();
    let mut sparks = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        if line.chars().count() != cols {
            return Err(LevelError::Ragged { line: row + 1 });
        }
        for (col, ch) in line.chars().enumerate() {
            let at = (col as i32, row as i32);
            let mut tile = Tile::Empty;
            match ch {
                '.' => {}
                '#' => tile = Tile::Solid,
                '~' => tile = Tile::Water,
                'P' => players.push(at),
                'E' => exits.push(at),
                'o' => sparks.push(at),
                'w' => enemies.push((EnemyKind::Walker, at.0, at.1)),
                'f' => enemies.push((EnemyKind::Flyer, at.0, at.1)),
                _ => {
                    return Err(LevelError::UnknownChar {
                        line: row + 1,
                        column: col + 1,
                        ch,
                    });
                }
            }
            tiles.push(tile);
        }
    }
    let player = match players[..] {
        [] => return Err(LevelError::NoPlayer),
        [one] => one,
        _ => return Err(LevelError::ManyPlayers),
    };
    let exit = match exits[..] {
        [] => return Err(LevelError::NoExit),
        [one] => one,
        _ => return Err(LevelError::ManyExits),
    };
    Ok(Parsed {
        map: Map {
            cols: cols as i32,
            rows: lines.len() as i32,
            tiles,
        },
        player,
        exit,
        enemies,
        sparks,
    })
}
