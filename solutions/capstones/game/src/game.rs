//! The simulation (SPEC sections 9 to 12): one `Game` is one attempt at one
//! level. `step` is the only way to change it.

use crate::fx::{Fx, Look};
use crate::hash::Fnv;
use crate::level::{self, Map, TILE};
use crate::rng::Rng;
use crate::store::{Id, Store};
use crate::{Anim, Body, EnemyKind, EnemyView, Events, Input, LevelError, Phase, Tile};
use crate::{SCREEN_H, SCREEN_W};

// All lengths are subpixels (256 per pixel); all speeds are subpixels per
// step. The names and values are the table in SPEC section 9.
pub(crate) const SUB: i32 = 256;
pub(crate) const PLAYER_W: i32 = 10 * SUB;
pub(crate) const PLAYER_H: i32 = 14 * SUB;
pub(crate) const WALKER_W: i32 = 12 * SUB;
pub(crate) const WALKER_H: i32 = 10 * SUB;
pub(crate) const FLYER_W: i32 = 12 * SUB;
pub(crate) const FLYER_H: i32 = 8 * SUB;
pub(crate) const SPARK_SIZE: i32 = 8 * SUB;

const GRAVITY: i32 = 64;
const MAX_FALL: i32 = 1536;
const JUMP_SPEED: i32 = 1344;
const JUMP_CUT: i32 = 448;
const RUN_ACCEL: i32 = 96;
const RUN_FRICTION: i32 = 128;
const MAX_RUN: i32 = 640;
const COYOTE_TICKS: u32 = 6;
const BUFFER_TICKS: u32 = 6;
pub(crate) const INVULN_TICKS: u32 = 90;
const START_LIVES: u32 = 3;
const KNOCKBACK_X: i32 = 512;
const KNOCKBACK_Y: i32 = 448;
const STOMP_BOUNCE: i32 = 1088;
const STOMP_DEPTH: i32 = 8 * SUB;
const WALKER_SPEED: i32 = 128;
const FLYER_SPEED: i32 = 96;
const FLYER_PERIOD: u32 = 48;
const FLYER_LEASH: i32 = 3 * TILE;

const SPARK_SCORE: u32 = 10;
const WALKER_SCORE: u32 = 50;
const FLYER_SCORE: u32 = 80;
const LIFE_BONUS: u32 = 100;

const SHAKE_HURT: u32 = 12;
const SHAKE_STOMP: u32 = 6;

/// The eight headings a Mistwing can pick, clockwise from "right".
const HEADINGS: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

#[derive(Clone)]
pub(crate) struct Player {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) vx: i32,
    pub(crate) vy: i32,
    /// Position at the start of the latest step, for interpolation.
    pub(crate) prev: (i32, i32),
    pub(crate) on_ground: bool,
    /// +1 facing right, -1 facing left.
    pub(crate) facing: i32,
    coyote: u32,
    buffer: u32,
    pub(crate) invuln: u32,
    jump_was_held: bool,
    pub(crate) anim: Anim,
    pub(crate) anim_ticks: u32,
}

#[derive(Clone)]
pub(crate) struct Enemy {
    pub(crate) kind: EnemyKind,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) vx: i32,
    pub(crate) vy: i32,
    pub(crate) prev: (i32, i32),
    /// Flyers: steps until the next change of heading.
    timer: u32,
    /// Flyers: where the leash is tied.
    home: (i32, i32),
}

impl Enemy {
    pub(crate) fn size(&self) -> (i32, i32) {
        match self.kind {
            EnemyKind::Walker => (WALKER_W, WALKER_H),
            EnemyKind::Flyer => (FLYER_W, FLYER_H),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Spark {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) taken: bool,
}

/// One attempt at one level.
#[derive(Clone)]
pub struct Game {
    pub(crate) map: Map,
    /// Per tile: which of its four neighbours are open. Only the renderer
    /// reads it, but it is level data, so it is computed once here.
    pub(crate) edges: Vec<u8>,
    rng: Rng,
    pub(crate) tick: u64,
    pub(crate) phase: Phase,
    pub(crate) score: u32,
    pub(crate) lives: u32,
    pub(crate) player: Player,
    spawn: (i32, i32),
    pub(crate) enemies: Store<Enemy>,
    /// Enemy ids in spawn order. An id stays here after its enemy is
    /// removed; it then simply stops resolving.
    pub(crate) roster: Vec<(EnemyKind, Id)>,
    pub(crate) sparks: Vec<Spark>,
    pub(crate) sparks_left: u32,
    /// Top-left corner of the exit tile.
    pub(crate) exit: (i32, i32),
    pub(crate) fx: Fx,
}

/// Moves `v` toward `target` by at most `step`, without overshooting.
fn approach(v: i32, target: i32, step: i32) -> i32 {
    if v < target {
        (v + step).min(target)
    } else {
        (v - step).max(target)
    }
}

/// Moves a box horizontally by `vx` and stops it at the first solid column.
/// Returns the new x and whether it was stopped.
fn slide_x(map: &Map, x: i32, y: i32, w: i32, h: i32, vx: i32) -> (i32, bool) {
    let nx = x + vx;
    if vx > 0 {
        let col = (nx + w - 1).div_euclid(TILE);
        if map.column_blocked(col, y, h) {
            return (col * TILE - w, true);
        }
    } else if vx < 0 {
        let col = nx.div_euclid(TILE);
        if map.column_blocked(col, y, h) {
            return ((col + 1) * TILE, true);
        }
    }
    (nx, false)
}

/// The vertical twin of `slide_x`.
fn slide_y(map: &Map, x: i32, y: i32, w: i32, h: i32, vy: i32) -> (i32, bool) {
    let ny = y + vy;
    if vy > 0 {
        let row = (ny + h - 1).div_euclid(TILE);
        if map.row_blocked(row, x, w) {
            return (row * TILE - h, true);
        }
    } else if vy < 0 {
        let row = ny.div_euclid(TILE);
        if map.row_blocked(row, x, w) {
            return ((row + 1) * TILE, true);
        }
    }
    (ny, false)
}

/// Strict overlap of two boxes given as `(x, y, w, h)`.
fn overlap(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// Where a box of size `(w, h)` born in tile `(col, row)` stands: centred
/// left to right, feet on the bottom edge of the tile.
fn stand_in(col: i32, row: i32, w: i32, h: i32) -> (i32, i32) {
    (col * TILE + (TILE - w) / 2, (row + 1) * TILE - h)
}

impl Game {
    /// Builds a game from level text (SPEC section 8) and a seed.
    pub fn new(level: &str, seed: u64) -> Result<Game, LevelError> {
        let parsed = level::parse(level)?;
        // Build the pictures now, during start-up, so that the first frame
        // has nothing left to allocate.
        crate::art::art();
        let map = parsed.map;
        let spawn = stand_in(parsed.player.0, parsed.player.1, PLAYER_W, PLAYER_H);

        let mut enemies = Store::with_capacity(parsed.enemies.len());
        let mut roster = Vec::with_capacity(parsed.enemies.len());
        for &(kind, col, row) in &parsed.enemies {
            let (w, h) = match kind {
                EnemyKind::Walker => (WALKER_W, WALKER_H),
                EnemyKind::Flyer => (FLYER_W, FLYER_H),
            };
            let (x, y) = stand_in(col, row, w, h);
            let id = enemies.insert(Enemy {
                kind,
                x,
                y,
                vx: match kind {
                    EnemyKind::Walker => -WALKER_SPEED,
                    EnemyKind::Flyer => 0,
                },
                vy: 0,
                prev: (x, y),
                timer: 0,
                home: (x, y),
            });
            roster.push((kind, id));
        }

        let sparks: Vec<Spark> = parsed
            .sparks
            .iter()
            .map(|&(col, row)| Spark {
                x: col * TILE + (TILE - SPARK_SIZE) / 2,
                y: row * TILE + (TILE - SPARK_SIZE) / 2,
                taken: false,
            })
            .collect();

        let mut edges = Vec::with_capacity((map.cols() * map.rows()) as usize);
        for row in 0..map.rows() {
            for col in 0..map.cols() {
                let open = |c: i32, r: i32| u8::from(map.tile(c, r) != map.tile(col, row));
                edges.push(
                    open(col, row - 1)
                        | open(col + 1, row) << 1
                        | open(col, row + 1) << 2
                        | open(col - 1, row) << 3,
                );
            }
        }

        Ok(Game {
            edges,
            rng: Rng::new(seed),
            tick: 0,
            phase: Phase::Playing,
            score: 0,
            lives: START_LIVES,
            player: Player {
                x: spawn.0,
                y: spawn.1,
                vx: 0,
                vy: 0,
                prev: spawn,
                on_ground: false,
                facing: 1,
                coyote: 0,
                buffer: 0,
                invuln: 0,
                jump_was_held: false,
                anim: Anim::Fall,
                anim_ticks: 0,
            },
            spawn,
            enemies,
            roster,
            sparks_left: sparks.len() as u32,
            sparks,
            exit: (parsed.exit.0 * TILE, parsed.exit.1 * TILE),
            fx: Fx::new(seed),
            map,
        })
    }

    /// Advances the game by one fixed step.
    pub fn step(&mut self, input: Input) -> Events {
        self.tick += 1;
        self.fx.age();
        // "Previous" for interpolation is where things are now, before this
        // step moves them. This runs in a finished game too: there nothing
        // moves any more, so previous and current must become equal, or the
        // picture would keep sliding between the last two positions.
        self.player.prev = (self.player.x, self.player.y);
        for &(_, id) in &self.roster {
            if let Some(e) = self.enemies.get_mut(id) {
                e.prev = (e.x, e.y);
            }
        }
        if self.phase != Phase::Playing {
            return Events(0);
        }
        let mut events = 0;
        self.step_player(input, &mut events);
        if self.phase == Phase::Playing {
            self.step_enemies();
            self.meet_enemies(&mut events);
        }
        if self.phase == Phase::Playing {
            self.collect(&mut events);
        }
        self.animate();
        Events(events)
    }

    /// SPEC 10.1 to 10.7: input, gravity, movement against tiles, hazards.
    fn step_player(&mut self, input: Input, events: &mut u32) {
        let p = &mut self.player;
        p.invuln = p.invuln.saturating_sub(1);

        let dir = i32::from(input.held(Input::RIGHT)) - i32::from(input.held(Input::LEFT));
        if dir != 0 {
            p.vx = approach(p.vx, dir * MAX_RUN, RUN_ACCEL);
            p.facing = dir;
        } else {
            p.vx = approach(p.vx, 0, RUN_FRICTION);
        }

        let jump = input.held(Input::JUMP);
        if jump && !p.jump_was_held {
            p.buffer = BUFFER_TICKS;
        } else {
            p.buffer = p.buffer.saturating_sub(1);
        }
        p.jump_was_held = jump;
        if p.on_ground {
            p.coyote = COYOTE_TICKS;
        } else {
            p.coyote = p.coyote.saturating_sub(1);
        }
        if p.buffer > 0 && p.coyote > 0 {
            p.vy = -JUMP_SPEED;
            p.buffer = 0;
            p.coyote = 0;
            *events |= Events::JUMPED;
            self.fx
                .burst(Look::Dust, 4, p.x + PLAYER_W / 2, p.y + PLAYER_H);
        }

        p.vy = (p.vy + GRAVITY).min(MAX_FALL);
        if !jump && p.vy < -JUMP_CUT {
            p.vy = -JUMP_CUT;
        }

        let (x, hit_wall) = slide_x(&self.map, p.x, p.y, PLAYER_W, PLAYER_H, p.vx);
        p.x = x;
        if hit_wall {
            p.vx = 0;
        }

        let was_on_ground = p.on_ground;
        let falling = p.vy > 0;
        let (y, hit) = slide_y(&self.map, p.x, p.y, PLAYER_W, PLAYER_H, p.vy);
        p.y = y;
        p.on_ground = hit && falling;
        if hit {
            p.vy = 0;
        }
        if p.on_ground && !was_on_ground {
            *events |= Events::LANDED;
            self.fx
                .burst(Look::Dust, 4, p.x + PLAYER_W / 2, p.y + PLAYER_H);
        }

        let drowned = self.map.touches_water(p.x, p.y, PLAYER_W, PLAYER_H);
        if drowned || p.y >= self.map.size().1 {
            let look = if drowned { Look::Splash } else { Look::Ember };
            self.fx
                .burst(look, 16, p.x + PLAYER_W / 2, p.y + PLAYER_H / 2);
            self.lose_life(events);
            if self.phase == Phase::Playing {
                let p = &mut self.player;
                (p.x, p.y) = self.spawn;
                // A teleport is not a movement: do not interpolate across it.
                p.prev = self.spawn;
                (p.vx, p.vy) = (0, 0);
                p.on_ground = false;
                p.coyote = 0;
                p.buffer = 0;
            }
        }
    }

    /// The part every way of getting hurt shares.
    fn lose_life(&mut self, events: &mut u32) {
        self.lives -= 1;
        *events |= Events::HURT;
        self.fx.shake(SHAKE_HURT);
        if self.lives == 0 {
            self.phase = Phase::Lost;
            *events |= Events::LOST;
        } else {
            self.player.invuln = INVULN_TICKS;
        }
    }

    /// SPEC 10.8: every living enemy moves, in spawn order.
    fn step_enemies(&mut self) {
        let floor = self.map.size().1;
        for i in 0..self.roster.len() {
            let (kind, id) = self.roster[i];
            let Some(e) = self.enemies.get_mut(id) else {
                continue;
            };
            match kind {
                EnemyKind::Walker => {
                    e.vy = (e.vy + GRAVITY).min(MAX_FALL);
                    let (y, grounded) = slide_y(&self.map, e.x, e.y, WALKER_W, WALKER_H, e.vy);
                    e.y = y;
                    if grounded {
                        e.vy = 0;
                        let (nx, hit_wall) = slide_x(&self.map, e.x, e.y, WALKER_W, WALKER_H, e.vx);
                        // One subpixel below the leading foot.
                        let probe_x = if e.vx > 0 {
                            e.x + e.vx + WALKER_W - 1
                        } else {
                            e.x + e.vx
                        };
                        let ledge = !self
                            .map
                            .solid(probe_x.div_euclid(TILE), (e.y + WALKER_H).div_euclid(TILE));
                        if hit_wall || ledge {
                            e.vx = -e.vx;
                        } else {
                            e.x = nx;
                        }
                    } else if e.y >= floor {
                        self.enemies.remove(id);
                    }
                }
                EnemyKind::Flyer => {
                    if e.timer == 0 {
                        let (hx, hy) = HEADINGS[self.rng.below(8) as usize];
                        (e.vx, e.vy) = (hx * FLYER_SPEED, hy * FLYER_SPEED);
                        e.timer = FLYER_PERIOD;
                    }
                    e.timer -= 1;
                    e.x += e.vx;
                    e.y += e.vy;
                    if (e.x - e.home.0 > FLYER_LEASH && e.vx > 0)
                        || (e.home.0 - e.x > FLYER_LEASH && e.vx < 0)
                    {
                        e.vx = -e.vx;
                    }
                    if (e.y - e.home.1 > FLYER_LEASH && e.vy > 0)
                        || (e.home.1 - e.y > FLYER_LEASH && e.vy < 0)
                    {
                        e.vy = -e.vy;
                    }
                }
            }
        }
    }

    /// SPEC 10.9: the player against every living enemy, in spawn order.
    fn meet_enemies(&mut self, events: &mut u32) {
        // Decide "was the player falling" once, so that bouncing off the
        // first enemy does not turn a second stomp into an injury.
        let falling = self.player.vy > 0;
        for i in 0..self.roster.len() {
            let (kind, id) = self.roster[i];
            let Some(e) = self.enemies.get(id) else {
                continue;
            };
            let (ew, eh) = e.size();
            let (ex, ey) = (e.x, e.y);
            let p = &mut self.player;
            // An invulnerable player is a ghost to enemies: it can neither be
            // hurt nor stomp. (Without this, the knock-back hop would drop
            // the player onto the enemy that just hurt it.)
            if p.invuln > 0 || !overlap((p.x, p.y, PLAYER_W, PLAYER_H), (ex, ey, ew, eh)) {
                continue;
            }
            if falling && p.y + PLAYER_H - ey <= STOMP_DEPTH {
                p.vy = -STOMP_BOUNCE;
                self.enemies.remove(id);
                self.score += match kind {
                    EnemyKind::Walker => WALKER_SCORE,
                    EnemyKind::Flyer => FLYER_SCORE,
                };
                *events |= Events::STOMP;
                self.fx.shake(SHAKE_STOMP);
                let look = match kind {
                    EnemyKind::Walker => Look::Splash,
                    EnemyKind::Flyer => Look::Glint,
                };
                self.fx.burst(look, 12, ex + ew / 2, ey + eh / 2);
            } else {
                let from_left = p.x + PLAYER_W / 2 < ex + ew / 2;
                p.vx = if from_left { -KNOCKBACK_X } else { KNOCKBACK_X };
                p.vy = -KNOCKBACK_Y;
                p.on_ground = false;
                let at = (p.x + PLAYER_W / 2, p.y + PLAYER_H / 2);
                self.fx.burst(Look::Ember, 16, at.0, at.1);
                self.lose_life(events);
                if self.phase != Phase::Playing {
                    return;
                }
            }
        }
    }

    /// SPEC 10.10 and 10.11: sparks, then the exit.
    fn collect(&mut self, events: &mut u32) {
        let p = &self.player;
        let body = (p.x, p.y, PLAYER_W, PLAYER_H);
        for spark in &mut self.sparks {
            if !spark.taken && overlap(body, (spark.x, spark.y, SPARK_SIZE, SPARK_SIZE)) {
                spark.taken = true;
                self.sparks_left -= 1;
                self.score += SPARK_SCORE;
                *events |= Events::SPARK;
                self.fx.burst(
                    Look::Glint,
                    6,
                    spark.x + SPARK_SIZE / 2,
                    spark.y + SPARK_SIZE / 2,
                );
            }
        }
        if self.sparks_left == 0 && overlap(body, (self.exit.0, self.exit.1, TILE, TILE)) {
            self.phase = Phase::Won;
            self.score += LIFE_BONUS * self.lives;
            *events |= Events::WON;
            self.fx.burst(
                Look::Ember,
                48,
                self.exit.0 + TILE / 2,
                self.exit.1 + TILE / 2,
            );
        }
    }

    /// SPEC 12.1: pick the animation from what the body is doing.
    fn animate(&mut self) {
        let p = &mut self.player;
        let anim = if !p.on_ground {
            if p.vy < 0 { Anim::Jump } else { Anim::Fall }
        } else if p.vx != 0 {
            Anim::Run
        } else {
            Anim::Idle
        };
        if anim == p.anim {
            p.anim_ticks += 1;
        } else {
            p.anim = anim;
            p.anim_ticks = 0;
        }
    }

    /// How many steps have been run.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn lives(&self) -> u32 {
        self.lives
    }

    /// The player's box and velocity.
    pub fn player(&self) -> Body {
        let p = &self.player;
        Body {
            x: p.x,
            y: p.y,
            vx: p.vx,
            vy: p.vy,
        }
    }

    pub fn on_ground(&self) -> bool {
        self.player.on_ground
    }

    /// The map size in tiles: `(columns, rows)`.
    pub fn map_size(&self) -> (u32, u32) {
        (self.map.cols() as u32, self.map.rows() as u32)
    }

    /// The tile at `(col, row)`, with the outside-the-map rule applied.
    pub fn tile(&self, col: i32, row: i32) -> Tile {
        self.map.tile(col, row)
    }

    /// How many enemies the level started with.
    pub fn enemy_count(&self) -> usize {
        self.roster.len()
    }

    /// Enemy number `index` in spawn order, or `None` past the end.
    pub fn enemy(&self, index: usize) -> Option<EnemyView> {
        let &(kind, id) = self.roster.get(index)?;
        Some(match self.enemies.get(id) {
            Some(e) => EnemyView {
                kind,
                alive: true,
                x: e.x,
                y: e.y,
            },
            None => EnemyView {
                kind,
                alive: false,
                x: 0,
                y: 0,
            },
        })
    }

    /// Sparks not yet collected.
    pub fn sparks_left(&self) -> u32 {
        self.sparks_left
    }

    /// Top-left corner of the view in map pixels, without shake.
    pub fn camera(&self) -> (i32, i32) {
        self.camera_for(self.player.x, self.player.y)
    }

    /// The camera for a player box at subpixel `(x, y)`; the renderer calls
    /// this with an interpolated position.
    pub(crate) fn camera_for(&self, x: i32, y: i32) -> (i32, i32) {
        fn axis(center: i32, screen: i32, map: i32) -> i32 {
            if map <= screen {
                (map - screen) / 2
            } else {
                (center - screen / 2).clamp(0, map - screen)
            }
        }
        let (map_w, map_h) = self.map.size();
        (
            axis((x + PLAYER_W / 2) >> 8, SCREEN_W as i32, map_w >> 8),
            axis((y + PLAYER_H / 2) >> 8, SCREEN_H as i32, map_h >> 8),
        )
    }

    /// The screen-shake offset in pixels for the current step.
    pub fn shake(&self) -> (i32, i32) {
        self.fx.shake_offset()
    }

    /// Live particles.
    pub fn particles(&self) -> usize {
        self.fx.particles().len()
    }

    /// The player's animation and the frame within it.
    pub fn anim(&self) -> (Anim, u32) {
        let p = &self.player;
        let (period, frames) = match p.anim {
            Anim::Idle => (24, 2),
            Anim::Run => (5, 4),
            Anim::Jump | Anim::Fall => (1, 1),
        };
        (p.anim, (p.anim_ticks / period) % frames)
    }

    /// The checksum of the canonical state (SPEC section 11).
    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv::new();
        h.u64(self.tick);
        h.u64(self.rng.state());
        h.u32(match self.phase {
            Phase::Playing => 0,
            Phase::Won => 1,
            Phase::Lost => 2,
        });
        h.u32(self.score);
        h.u32(self.lives);
        let p = &self.player;
        for v in [p.x, p.y, p.vx, p.vy] {
            h.i32(v);
        }
        h.u32(self.roster.len() as u32);
        for i in 0..self.roster.len() {
            // `enemy` already applies the "dead means zeros" rule.
            let view = self.enemy(i).expect("index is in range");
            h.u32(match view.kind {
                EnemyKind::Walker => 0,
                EnemyKind::Flyer => 1,
            });
            h.u32(u32::from(view.alive));
            h.i32(view.x);
            h.i32(view.y);
        }
        h.u32(self.sparks.len() as u32);
        for spark in &self.sparks {
            h.u32(u32::from(spark.taken));
        }
        h.finish()
    }
}
