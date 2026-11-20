//! Milestone 4: physics and gameplay (SPEC sections 9 to 11).
//!
//! Each test builds a tiny level, feeds scripted input and checks positions
//! and events that follow from the rules and the constants in the SPEC.
//! Positions are in subpixels: 256 per pixel, 4096 per tile.

mod common;

use common::{JUMP, LEFT, NONE, RIGHT, SUB, TILE, fnv1a, game, run, run_until};
use game::{EnemyKind, EnemyView, Events, Game, Input, Phase, Rng};

const PLAYER_W: i32 = 10 * SUB;
const PLAYER_H: i32 = 14 * SUB;
const WALKER_W: i32 = 12 * SUB;

/// A flat floor. The spark in the top row is out of reach, so the exit
/// stays shut and nothing but movement happens.
const FLAT: &str = "\
#........o.........#
#..................#
#..................#
#..................#
#..................#
#..................#
#.P..............E.#
####################
";
const FLAT_X: i32 = 2 * TILE + 3 * SUB;
const FLAT_Y: i32 = 7 * TILE - PLAYER_H;

/// `FLAT` with the player settled on the floor (two steps run).
fn flat() -> Game {
    let mut g = game(FLAT, 1);
    run(&mut g, NONE, 2);
    g
}

#[test]
fn the_player_settles_on_the_floor_in_the_first_step() {
    let _watch = common::watch();
    let mut g = game(FLAT, 1);
    assert_eq!((g.player().x, g.player().y), (FLAT_X, FLAT_Y));
    assert!(
        !g.on_ground(),
        "before the first step nothing has been tested against the floor"
    );
    assert_eq!(g.phase(), Phase::Playing);
    assert_eq!((g.lives(), g.score()), (3, 0));

    let events = g.step(NONE);
    assert_eq!(
        events,
        Events(Events::LANDED),
        "gravity pulls the player 64 subpixels into the floor, the floor pushes it back: LANDED"
    );
    assert_eq!(g.tick(), 1);
    let p = g.player();
    assert_eq!((p.x, p.y, p.vx, p.vy), (FLAT_X, FLAT_Y, 0, 0));
    assert!(g.on_ground());

    assert_eq!(g.step(NONE), Events(0), "standing still is not an event");
    assert_eq!(g.player().y, FLAT_Y);
    assert!(g.on_ground(), "the floor is found again in every step");
}

#[test]
fn running_accelerates_to_a_top_speed_and_friction_stops_it() {
    let _watch = common::watch();
    let mut g = flat();
    let mut x = FLAT_X;
    for (i, vx) in [96, 192, 288, 384, 480, 576, 640, 640, 640, 640]
        .into_iter()
        .enumerate()
    {
        g.step(RIGHT);
        x += vx;
        assert_eq!(g.player().vx, vx, "speed after {} steps of RIGHT", i + 1);
        assert_eq!(g.player().x, x, "position after {} steps of RIGHT", i + 1);
    }
    for vx in [512, 384, 256, 128, 0, 0] {
        g.step(NONE);
        x += vx;
        assert_eq!(
            g.player().vx,
            vx,
            "friction takes 128 per step and stops at 0"
        );
        assert_eq!(g.player().x, x);
    }
    // Turning around uses the run acceleration, not friction.
    run(&mut g, RIGHT, 10);
    g.step(LEFT);
    assert_eq!(g.player().vx, 640 - 96);
    // LEFT and RIGHT together cancel: friction applies.
    g.step(Input(Input::LEFT | Input::RIGHT));
    assert_eq!(g.player().vx, 640 - 96 - 128);
    // Running left mirrors running right.
    run(&mut g, LEFT, 20);
    assert_eq!(g.player().vx, -640);
}

#[test]
fn a_full_jump_follows_the_worked_example() {
    let _watch = common::watch();
    // SPEC 10.3, worked example.
    let mut g = flat();
    let events = g.step(JUMP);
    assert_eq!(events, Events(Events::JUMPED));
    assert_eq!(
        g.player().vy,
        -1280,
        "launch speed -1344, then one step of gravity"
    );
    assert_eq!(g.player().y, FLAT_Y - 1280);
    assert!(!g.on_ground());

    let mut y = FLAT_Y - 1280;
    let mut vy = -1280;
    for _ in 0..40 {
        let events = g.step(JUMP);
        vy += 64;
        y += vy;
        assert_eq!(
            events,
            Events(0),
            "tick {}: nothing happens in mid-air",
            g.tick()
        );
        assert_eq!((g.player().y, g.player().vy), (y, vy), "tick {}", g.tick());
    }
    assert_eq!(
        y, FLAT_Y,
        "after 41 steps the feet are level with the floor again"
    );
    assert!(
        !g.on_ground(),
        "touching is not overlapping: the floor is found one step later"
    );
    assert_eq!(g.step(JUMP), Events(Events::LANDED));
    assert_eq!((g.player().y, g.player().vy), (FLAT_Y, 0));
    assert_eq!(
        g.step(JUMP),
        Events(0),
        "holding JUMP does not jump again: a jump needs a new press"
    );
}

#[test]
fn the_highest_point_of_a_full_jump_is_13440_subpixels_up() {
    let _watch = common::watch();
    let mut g = flat();
    let mut top = FLAT_Y;
    for _ in 0..60 {
        g.step(JUMP);
        top = top.min(g.player().y);
    }
    assert_eq!(
        FLAT_Y - top,
        13440,
        "64 * (20 + 19 + ... + 1) == 13440 (52.5 pixels)"
    );
}

#[test]
fn releasing_jump_cuts_the_rise_short() {
    let _watch = common::watch();
    let mut g = flat();
    g.step(JUMP);
    let mut y = FLAT_Y - 1280;
    // Not held and rising faster than 448: clamp to -448, then gravity as usual.
    for vy in [-448, -384, -320, -256, -192, -128, -64, 0] {
        g.step(NONE);
        y += vy;
        assert_eq!((g.player().y, g.player().vy), (y, vy), "tick {}", g.tick());
    }
    assert_eq!(FLAT_Y - y, 3072, "a tap of JUMP rises 12 pixels");
}

#[test]
fn a_jump_needs_ground_and_a_fresh_press() {
    let _watch = common::watch();
    let mut g = flat();
    let mut jumps = 0;
    for _ in 0..200 {
        if g.step(JUMP).has(Events::JUMPED) {
            jumps += 1;
        }
    }
    assert_eq!(
        jumps, 1,
        "one press, one jump, however long the button is held"
    );

    let mut g = flat();
    g.step(JUMP);
    run(&mut g, NONE, 3);
    assert_eq!(
        g.step(JUMP),
        Events(0),
        "a new press in mid-air does nothing (there is no double jump)"
    );
}

#[test]
fn a_press_just_before_landing_is_remembered() {
    let _watch = common::watch();
    // The jump of `a_full_jump_follows_the_worked_example` starts in step 3
    // and lands in step 44. A press is remembered for 6 steps.
    for (press_at, expect_jump) in [(40, true), (39, false)] {
        let mut g = flat();
        // Hold JUMP for the whole flight, let go for one step, press again.
        while g.tick() + 2 < press_at {
            g.step(JUMP);
        }
        g.step(NONE);
        let mut jumped_at = None;
        while g.tick() < 50 {
            if g.step(JUMP).has(Events::JUMPED) {
                jumped_at = Some(g.tick());
            }
        }
        if expect_jump {
            assert_eq!(
                jumped_at,
                Some(45),
                "JUMP pressed in step 40 is still buffered in step 45, the first step that \
                 starts on the ground"
            );
        } else {
            assert_eq!(jumped_at, None, "a press in step 39 has expired by step 45");
        }
    }
}

const LEDGE: &str = "\
#........o.........#
#..................#
#..................#
#.P................#
####...............#
#..................#
#..................#
#..................#
#................E.#
####################
";

#[test]
fn a_jump_still_works_shortly_after_walking_off_a_ledge() {
    let _watch = common::watch();
    for late in 1..=7 {
        let mut g = game(LEDGE, 1);
        run(&mut g, NONE, 2);
        // Run until the step in which the ground is lost.
        while g.on_ground() {
            g.step(RIGHT);
            assert!(g.tick() < 100, "the player never left the ledge");
        }
        // The ledge ends at x = 4 tiles; the box must be fully past it.
        assert!(
            g.player().x >= 4 * TILE,
            "the player fell while still over the ledge"
        );
        run(&mut g, RIGHT, late - 1);
        let events = g.step(Input(Input::RIGHT | Input::JUMP));
        assert_eq!(
            events.has(Events::JUMPED),
            late <= 5,
            "JUMP pressed {late} step(s) after the step that lost the ground: coyote time \
             allows 1 to 5 and refuses 6 and later"
        );
    }
}

#[test]
fn walls_stop_the_player_flush_against_the_tile() {
    let _watch = common::watch();
    let mut g = flat();
    run(&mut g, RIGHT, 150);
    let p = g.player();
    assert_eq!(
        p.x,
        19 * TILE - PLAYER_W,
        "right edge of the box on the left edge of the wall"
    );
    assert_eq!(p.vx, 0, "hitting a wall zeroes the horizontal speed");
    assert_eq!(
        g.phase(),
        Phase::Playing,
        "the exit is shut while a spark is left"
    );

    run(&mut g, LEFT, 150);
    let p = g.player();
    assert_eq!(
        p.x, TILE,
        "left edge of the box on the right edge of the wall"
    );
    assert_eq!(p.vx, 0);
    assert_eq!(p.y, FLAT_Y, "sliding along the floor never changes y");
}

#[test]
fn a_ceiling_stops_a_jump() {
    let _watch = common::watch();
    let low = "\
####################
#.P..............E.#
####################
";
    let mut g = game(low, 1);
    run(&mut g, NONE, 2);
    let floor_y = 2 * TILE - PLAYER_H;
    assert_eq!(g.player().y, floor_y);
    assert_eq!(g.step(JUMP), Events(Events::JUMPED));
    assert_eq!(
        (g.player().y, g.player().vy),
        (TILE, 0),
        "the head stops flush under the ceiling and the upward speed is lost"
    );
    assert!(!g.on_ground(), "hitting a ceiling is not landing");
    let mut y = TILE;
    for vy in [64, 128, 192] {
        assert_eq!(g.step(JUMP), Events(0));
        y += vy;
        assert_eq!(g.player().y, y);
    }
    assert_eq!(g.step(JUMP), Events(Events::LANDED));
    assert_eq!(g.player().y, floor_y);
}

#[test]
fn water_costs_a_life_and_sends_the_player_back_to_the_start() {
    let _watch = common::watch();
    let pool = "\
#........o.........#
#..................#
#.P.............E..#
####~~##############
####################
";
    let mut g = game(pool, 1);
    let start = (g.player().x, g.player().y);
    let events = run_until(&mut g, RIGHT, Events::HURT, 100);
    assert_eq!(events, Events(Events::HURT));
    assert_eq!(g.lives(), 2);
    let p = g.player();
    assert_eq!(
        (p.x, p.y, p.vx, p.vy),
        (start.0, start.1, 0, 0),
        "back at the start, at rest"
    );
    assert!(!g.on_ground());
    assert_eq!(g.phase(), Phase::Playing);
    assert_eq!(
        g.step(RIGHT),
        Events(Events::LANDED),
        "the next step finds the floor again"
    );

    let events = run_until(&mut g, RIGHT, Events::HURT, 100);
    assert_eq!(events, Events(Events::HURT));
    assert_eq!(g.lives(), 1);

    let events = run_until(&mut g, RIGHT, Events::HURT, 100);
    assert_eq!(
        events,
        Events(Events::HURT | Events::LOST),
        "the last life: HURT and LOST"
    );
    assert_eq!((g.lives(), g.phase()), (0, Phase::Lost));
    assert!(
        g.player().x >= 4 * TILE,
        "a lost game leaves the player where it died; there is no respawn"
    );
}

#[test]
fn a_lost_game_only_counts_ticks() {
    let _watch = common::watch();
    let pool = "\
#.P.E#
##~~##
";
    let mut g = game(pool, 1);
    run_until(&mut g, RIGHT, Events::LOST, 300);
    let (tick, body, score) = (g.tick(), g.player(), g.score());
    for _ in 0..10 {
        assert_eq!(
            g.step(Input(Input::ALL)),
            Events(0),
            "nothing happens after the game is lost"
        );
    }
    assert_eq!(g.tick(), tick + 10, "tick still counts steps");
    assert_eq!(
        (g.player(), g.score(), g.phase()),
        (body, score, Phase::Lost)
    );
}

#[test]
fn falling_out_of_the_map_costs_a_life() {
    let _watch = common::watch();
    let pit = "\
#........o.........#
#..................#
#.P.............E..#
####..##############
";
    let mut g = game(pit, 1);
    let start = (g.player().x, g.player().y);
    let mut deepest = 0;
    let events = loop {
        let events = g.step(RIGHT);
        if events.has(Events::HURT) {
            break events;
        }
        deepest = deepest.max(g.player().y);
        assert!(g.tick() < 200, "the player never fell out of the map");
    };
    assert_eq!(events, Events(Events::HURT));
    assert!(
        deepest < 4 * TILE && deepest + 1536 >= 4 * TILE,
        "the player dies in the step in which the top of its box reaches the bottom edge of \
         the map (y >= 4 tiles), not earlier; it was last seen at y = {deepest}"
    );
    assert_eq!((g.player().x, g.player().y), start);
    assert_eq!(g.lives(), 2);
}

#[test]
fn sparks_score_and_the_exit_opens_when_none_is_left() {
    let _watch = common::watch();
    let level = "\
#..................#
#.P.o..o.......E...#
####################
";
    let mut g = game(level, 1);
    assert_eq!(g.sparks_left(), 2);

    let events = run_until(&mut g, RIGHT, Events::SPARK, 100);
    assert_eq!(events, Events(Events::SPARK));
    assert_eq!((g.score(), g.sparks_left()), (10, 1));
    // The spark's 8x8 box starts 4 pixels into tile 4; boxes must overlap.
    let reach = 4 * TILE + 4 * SUB - PLAYER_W;
    let p = g.player();
    assert!(
        p.x > reach && p.x - p.vx <= reach,
        "the spark is collected in the first step in which the boxes overlap (x > {reach}), \
         but x went from {} to {}",
        p.x - p.vx,
        p.x
    );

    run_until(&mut g, RIGHT, Events::SPARK, 100);
    assert_eq!((g.score(), g.sparks_left()), (20, 0));

    let events = run_until(&mut g, RIGHT, Events::WON, 200);
    assert_eq!(events, Events(Events::WON));
    assert_eq!(g.phase(), Phase::Won);
    assert_eq!(
        g.score(),
        20 + 3 * 100,
        "winning adds 100 for every life left"
    );
    let p = g.player();
    assert!(
        p.x + PLAYER_W > 15 * TILE && p.x - p.vx + PLAYER_W <= 15 * TILE,
        "the game is won in the first step in which the player overlaps the exit tile"
    );

    let (tick, body) = (g.tick(), g.player());
    assert_eq!(
        g.step(RIGHT),
        Events(0),
        "nothing happens after the game is won"
    );
    assert_eq!((g.tick(), g.player(), g.score()), (tick + 1, body, 320));
}

#[test]
fn a_level_without_sparks_has_an_open_exit() {
    let _watch = common::watch();
    let mut g = game("#.P.E#\n######\n", 1);
    let events = run_until(&mut g, RIGHT, Events::WON, 100);
    assert_eq!(events, Events(Events::WON));
    assert_eq!(g.score(), 300);
}

#[test]
fn a_walker_turns_at_ledges() {
    let _watch = common::watch();
    let level = "\
#P.........E#
##..........#
#...........#
#....w......#
#.########..#
#...........#
#############
";
    let mut g = game(level, 1);
    let y = 4 * TILE - 10 * SUB;
    let at = |g: &Game| {
        let e = g.enemy(0).expect("one walker");
        assert_eq!(
            e.y,
            y,
            "tick {}: the walker stays on its platform",
            g.tick()
        );
        e.x
    };
    assert_eq!(at(&g), 5 * TILE + 2 * SUB);
    g.step(NONE);
    assert_eq!(
        at(&g),
        5 * TILE + 2 * SUB - 128,
        "a walker starts out to the left, 128 per step"
    );
    run(&mut g, NONE, 99);
    assert_eq!(
        at(&g),
        2 * TILE,
        "after 100 steps: at the left end of the platform"
    );
    g.step(NONE);
    assert_eq!(
        at(&g),
        2 * TILE,
        "step 101: the next step would hang over the ledge; turn instead"
    );
    g.step(NONE);
    assert_eq!(at(&g), 2 * TILE + 128, "step 102: walking right");
    run(&mut g, NONE, 231);
    assert_eq!(
        at(&g),
        10 * TILE - WALKER_W,
        "step 333: at the right end of the platform"
    );
    g.step(NONE);
    assert_eq!(at(&g), 10 * TILE - WALKER_W, "step 334: turn");
    g.step(NONE);
    assert_eq!(
        at(&g),
        10 * TILE - WALKER_W - 128,
        "step 335: walking left again"
    );
    assert_eq!(g.lives(), 3);
}

#[test]
fn a_walker_turns_at_walls() {
    let _watch = common::watch();
    let level = "\
#P....E#
##.....#
#......#
#.#w.#.#
########
";
    let mut g = game(level, 1);
    let x = |g: &Game| g.enemy(0).expect("one walker").x;
    assert_eq!(x(&g), 3 * TILE + 2 * SUB);
    run(&mut g, NONE, 4);
    assert_eq!(
        x(&g),
        3 * TILE,
        "after 4 steps: touching the wall on the left"
    );
    g.step(NONE);
    assert_eq!(x(&g), 3 * TILE, "step 5: blocked, so turn and stay");
    run(&mut g, NONE, 40);
    assert_eq!(
        x(&g),
        5 * TILE - WALKER_W,
        "step 45: touching the wall on the right"
    );
    g.step(NONE);
    assert_eq!(
        x(&g),
        5 * TILE - WALKER_W,
        "step 46: blocked, so turn and stay"
    );
    g.step(NONE);
    assert_eq!(x(&g), 5 * TILE - WALKER_W - 128);
}

#[test]
fn a_walker_that_falls_out_of_the_map_is_removed() {
    let _watch = common::watch();
    // The walker starts in the air over a hole in the floor.
    let level = "\
#.w..P.E#
##.######
";
    let mut g = game(level, 1);
    let start_y = TILE - 10 * SUB;
    assert_eq!(run(&mut g, NONE, 13), Events(Events::LANDED));
    assert_eq!(
        g.enemy(0),
        Some(EnemyView {
            kind: EnemyKind::Walker,
            alive: true,
            x: 2 * TILE + 2 * SUB,
            y: start_y + 32 * 13 * 14,
        }),
        "13 steps of gravity (64, 128, ... per step) and no floor: still falling, and a walker \
         in the air does not move sideways"
    );
    assert_eq!(
        g.step(NONE),
        Events(0),
        "a walker leaving the map is not an event"
    );
    assert_eq!(
        g.enemy(0),
        Some(EnemyView {
            kind: EnemyKind::Walker,
            alive: false,
            x: 0,
            y: 0
        }),
        "step 14: the top of the walker's box is below the last row, so it is removed; a \
         removed enemy reports alive: false at (0, 0), however it went"
    );
    assert_eq!(g.enemy_count(), 1);
    assert_eq!(g.score(), 0, "nobody defeated it: no score");
    assert_eq!(
        g.state_hash(),
        expected_hash(&g, 1, &[]),
        "the state hash sees a removed enemy as alive 0, x 0, y 0"
    );
}

const DROP_ON_WALKER: &str = "\
#..P......E#
#..........#
#..........#
#..........#
#.#w#......#
############
";

#[test]
fn landing_on_an_enemy_defeats_it() {
    let _watch = common::watch();
    let mut g = game(DROP_ON_WALKER, 1);
    assert_eq!(
        run(&mut g, NONE, 20),
        Events(0),
        "20 steps of free fall, nothing touches"
    );
    let before = g.player();
    assert_eq!(before.vy, 1280);
    let events = g.step(NONE);
    assert_eq!(
        events,
        Events(Events::STOMP),
        "step 21: the falling player reaches the walker"
    );
    assert_eq!(g.score(), 50);
    assert_eq!(
        g.enemy(0),
        Some(EnemyView {
            kind: EnemyKind::Walker,
            alive: false,
            x: 0,
            y: 0
        }),
        "a defeated enemy keeps its number and reports alive: false at (0, 0)"
    );
    assert_eq!(
        g.enemy_count(),
        1,
        "enemy_count is how many the level started with"
    );
    let p = g.player();
    assert_eq!(
        p.y,
        before.y + 1344,
        "the stomp does not move the player in that step"
    );
    assert_eq!(p.vy, -1088, "the player bounces");
    assert_eq!(g.lives(), 3);

    g.step(NONE);
    assert_eq!(
        g.player().vy,
        -448,
        "without JUMP held the bounce is cut like a jump"
    );

    let mut g = game(DROP_ON_WALKER, 1);
    run(&mut g, JUMP, 21);
    assert_eq!(g.player().vy, -1088);
    g.step(JUMP);
    assert_eq!(g.player().vy, -1024, "holding JUMP keeps the whole bounce");
}

const WALK_INTO_PLAYER: &str = "\
#..........E#
#...........#
#.P..w......#
#############
";

#[test]
fn touching_an_enemy_from_the_side_hurts() {
    let _watch = common::watch();
    let mut g = game(WALK_INTO_PLAYER, 1);
    let px = 2 * TILE + 3 * SUB;
    assert_eq!(run(&mut g, NONE, 74), Events(Events::LANDED));
    assert_eq!(
        g.enemy(0).expect("one walker").x,
        px + PLAYER_W,
        "after 74 steps the walker touches the player's box but does not overlap it"
    );
    let events = g.step(NONE);
    assert_eq!(
        events,
        Events(Events::HURT),
        "step 75: overlap, and the player is not falling"
    );
    assert_eq!(g.lives(), 2);
    let p = g.player();
    assert_eq!(
        (p.x, p.vx, p.vy),
        (px, -512, -448),
        "knocked up and away from the enemy (which is to the right); the push starts moving \
         the player in the next step"
    );
    assert!(!g.on_ground());
    assert!(g.enemy(0).expect("one walker").alive);

    g.step(NONE);
    let p = g.player();
    assert_eq!(
        (p.x, p.vx, p.vy),
        (px - 384, -384, -384),
        "friction and gravity act as usual"
    );
}

#[test]
fn a_stomp_needs_the_feet_near_the_top_of_the_enemy() {
    let _watch = common::watch();
    // The walker reaches the player's box in step 75. A tap of JUMP makes
    // the player come down just then: falling in both cases, but only in
    // one of them with its feet at most 8 pixels below the walker's top.
    //
    // Tap in step 58: in step 75 the feet are 192 subpixels above the
    // floor, 2368 below the top of the 10 pixel high walker. Too deep.
    let mut g = game(WALK_INTO_PLAYER, 1);
    run(&mut g, NONE, 57);
    assert_eq!(g.step(JUMP), Events(Events::JUMPED));
    assert_eq!(run(&mut g, NONE, 16), Events(0));
    assert!(g.player().vy > 0, "the player is on the way down");
    assert_eq!(
        g.step(NONE),
        Events(Events::HURT),
        "falling is not enough for a stomp: the feet are more than STOMP_DEPTH below the \
         enemy's top, so this is a hit from the side"
    );
    assert_eq!((g.lives(), g.score()), (2, 0));

    // Tap in step 59: one step earlier in the fall, the feet are 768
    // subpixels above the floor, 1792 below the walker's top. A stomp.
    let mut g = game(WALK_INTO_PLAYER, 1);
    run(&mut g, NONE, 58);
    assert_eq!(g.step(JUMP), Events(Events::JUMPED));
    assert_eq!(run(&mut g, NONE, 15), Events(0));
    assert_eq!(g.step(NONE), Events(Events::STOMP));
    assert_eq!((g.lives(), g.score()), (3, 50));
}

#[test]
fn a_hurt_player_cannot_be_hurt_again_for_a_while() {
    let _watch = common::watch();
    let mut g = game(WALK_INTO_PLAYER, 1);
    run(&mut g, NONE, 75);
    assert_eq!(g.lives(), 2);
    let mut overlapped = false;
    // Protected in steps 76 to 164; the walker walks through the player.
    while g.tick() < 164 {
        let events = g.step(NONE);
        assert!(
            !events.has(Events::HURT),
            "tick {}: hurt while invulnerable",
            g.tick()
        );
        let (p, e) = (g.player(), g.enemy(0).expect("one walker"));
        overlapped |= p.x < e.x + WALKER_W && e.x < p.x + PLAYER_W && p.y + PLAYER_H > e.y;
    }
    assert!(
        overlapped,
        "the test expects the walker to pass through the player"
    );
    assert_eq!(g.lives(), 2);
}

#[test]
fn a_flyer_follows_the_random_stream() {
    let _watch = common::watch();
    let level = "\
#...........#
#...........#
#.....f.....#
#...........#
#...........#
#...........#
#...........#
#.P.......E.#
#############
";
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
    for seed in [0, 1, 7, 12345, u64::MAX] {
        let mut g = game(level, seed);
        // The model: SPEC 10.8, flyers.
        let mut rng = Rng::new(seed);
        let home = (6 * TILE + 2 * SUB, 3 * TILE - 8 * SUB);
        let (mut x, mut y) = home;
        let (mut vx, mut vy) = (0, 0);
        let mut timer = 0;
        let mut turned = false;
        for _ in 0..3000 {
            g.step(NONE);
            if timer == 0 {
                let (hx, hy) = HEADINGS[rng.below(8) as usize];
                (vx, vy) = (hx * 96, hy * 96);
                timer = 48;
            }
            timer -= 1;
            x += vx;
            y += vy;
            let (old_vx, old_vy) = (vx, vy);
            if (x - home.0 > 3 * TILE && vx > 0) || (home.0 - x > 3 * TILE && vx < 0) {
                vx = -vx;
            }
            if (y - home.1 > 3 * TILE && vy > 0) || (home.1 - y > 3 * TILE && vy < 0) {
                vy = -vy;
            }
            turned |= (old_vx, old_vy) != (vx, vy);
            let e = g.enemy(0).expect("one flyer");
            assert_eq!(
                (e.x, e.y),
                (x, y),
                "seed {seed}, tick {}: the flyer left the path SPEC 10.8 gives it",
                g.tick()
            );
        }
        assert!(
            turned,
            "seed {seed}: in 3000 steps the leash should have turned the flyer"
        );
    }
}

/// The canonical state encoding of SPEC section 11, from public values.
fn expected_hash(g: &Game, rng_state: u64, sparks_taken: &[bool]) -> u64 {
    let mut bytes = Vec::new();
    bytes.extend(g.tick().to_le_bytes());
    bytes.extend(rng_state.to_le_bytes());
    let phase: u32 = match g.phase() {
        Phase::Playing => 0,
        Phase::Won => 1,
        Phase::Lost => 2,
    };
    bytes.extend(phase.to_le_bytes());
    bytes.extend(g.score().to_le_bytes());
    bytes.extend(g.lives().to_le_bytes());
    let p = g.player();
    for v in [p.x, p.y, p.vx, p.vy] {
        bytes.extend(v.to_le_bytes());
    }
    bytes.extend((g.enemy_count() as u32).to_le_bytes());
    for i in 0..g.enemy_count() {
        let e = g.enemy(i).expect("index below enemy_count");
        let kind: u32 = match e.kind {
            EnemyKind::Walker => 0,
            EnemyKind::Flyer => 1,
        };
        bytes.extend(kind.to_le_bytes());
        bytes.extend(u32::from(e.alive).to_le_bytes());
        bytes.extend(e.x.to_le_bytes());
        bytes.extend(e.y.to_le_bytes());
    }
    bytes.extend((sparks_taken.len() as u32).to_le_bytes());
    for &taken in sparks_taken {
        bytes.extend(u32::from(taken).to_le_bytes());
    }
    fnv1a(bytes)
}

#[test]
fn state_hash_matches_the_worked_example() {
    let _watch = common::watch();
    // SPEC section 11: 52 bytes, written out there one field per line.
    let g = game("#.P.E#\n######\n", 0);
    assert_eq!(g.state_hash(), 0x0017_b151_15d8_6541);
    assert_eq!(g.state_hash(), expected_hash(&g, 0, &[]));
}

#[test]
fn state_hash_is_the_canonical_encoding() {
    let _watch = common::watch();
    // No flyer: the random stream is never used, so its state is the seed.
    let level = "\
#.........o#
#..........#
#..........#
#..........#
#..........#
#..........#
#.P.o.w..E.#
############
";
    let seed = 0xDEAD_BEEF_0BAD_F00D;
    let mut g = game(level, seed);
    assert_eq!(
        g.state_hash(),
        expected_hash(&g, seed, &[false, false]),
        "the hash of a new game; see SPEC section 11 for the field order"
    );
    let mut taken = [false, false];
    for i in 0..400 {
        let input = if i < 200 {
            RIGHT
        } else {
            Input(Input::LEFT | Input::JUMP)
        };
        if g.step(input).has(Events::SPARK) {
            // Sparks are listed in reading order: the unreachable one in the
            // top row is number 0, the one on the floor is number 1.
            taken[1] = true;
        }
        assert_eq!(
            g.state_hash(),
            expected_hash(&g, seed, &taken),
            "tick {}: state_hash does not match the canonical encoding",
            g.tick()
        );
    }
    assert!(
        taken[1],
        "the player should have collected the spark on the floor"
    );
    assert!(
        g.lives() < 3,
        "the script runs into the walker, so the hash saw a life lost"
    );
}

#[test]
fn state_hash_covers_the_random_stream() {
    let _watch = common::watch();
    let level = "\
#....f.....#
#..........#
#..........#
#.P......E.#
############
";
    let seed = 31;
    let mut g = game(level, seed);
    let mut rng = Rng::new(seed);
    for tick in 1..=200u64 {
        g.step(NONE);
        // One flyer draws once in steps 1, 49, 97, ...
        if (tick - 1) % 48 == 0 {
            rng.below(8);
        }
        assert_eq!(
            g.state_hash(),
            expected_hash(&g, rng.state(), &[]),
            "tick {tick}: the hash must include the state of the simulation's generator, and \
             only flyers may draw from it"
        );
    }
}
