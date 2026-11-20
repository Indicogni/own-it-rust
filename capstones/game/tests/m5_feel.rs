//! Milestone 5: animation and feel (SPEC section 12).
//!
//! How your game looks is yours to decide, so these tests check only what
//! the SPEC fixes: the animation state machine, particle counts and
//! lifetimes, the shake pattern, the HUD strip, and that `render` is a pure
//! function of the game.

mod common;

use common::{JUMP, LEVEL_1, NONE, RIGHT, game, run, run_until};
use game::{Anim, Events, Framebuffer, Game, Input, SCREEN_H, SCREEN_W};

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

const DROP_ON_WALKER: &str = "\
#..P......E#
#..........#
#..........#
#..........#
#.#w#......#
############
";

const WALK_INTO_PLAYER: &str = "\
#..........E#
#...........#
#.P..w......#
#############
";

fn screen() -> Framebuffer {
    Framebuffer::new(SCREEN_W, SCREEN_H)
}

fn frame(game: &Game, alpha: u32) -> u64 {
    let mut fb = screen();
    game.render(&mut fb, alpha);
    fb.checksum()
}

#[test]
fn the_animation_follows_what_the_body_does() {
    let _watch = common::watch();
    let mut g = game(FLAT, 1);
    assert_eq!(
        g.anim(),
        (Anim::Fall, 0),
        "a new game starts in the air, not moving up"
    );
    g.step(NONE);
    assert_eq!(
        g.anim(),
        (Anim::Idle, 0),
        "step 1 lands: a new animation starts at frame 0"
    );
    run(&mut g, NONE, 23);
    assert_eq!(
        g.anim(),
        (Anim::Idle, 0),
        "idle shows each frame for 24 steps"
    );
    g.step(NONE);
    assert_eq!(g.anim(), (Anim::Idle, 1));
    run(&mut g, NONE, 24);
    assert_eq!(g.anim(), (Anim::Idle, 0), "idle has two frames");

    g.step(RIGHT);
    assert_eq!(g.anim(), (Anim::Run, 0));
    for expected in [0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 0] {
        g.step(RIGHT);
        assert_eq!(
            g.anim(),
            (Anim::Run, expected),
            "run shows each of 4 frames for 5 steps"
        );
    }

    g.step(Input(Input::RIGHT | Input::JUMP));
    assert_eq!(g.anim(), (Anim::Jump, 0));
    run(&mut g, JUMP, 19);
    assert_eq!(g.anim(), (Anim::Jump, 0), "rising: vy < 0");
    g.step(JUMP);
    assert_eq!(
        g.anim(),
        (Anim::Fall, 0),
        "at the top vy is 0, which counts as falling"
    );
    run_until(&mut g, NONE, Events::LANDED, 60);
    assert!(
        matches!(g.anim(), (Anim::Idle | Anim::Run, 0)),
        "landing ends the fall; got {:?}",
        g.anim()
    );
    run(&mut g, NONE, 10);
    assert_eq!(g.anim().0, Anim::Idle, "sliding to a stop ends in Idle");
}

#[test]
fn landing_dust_lives_between_10_and_40_steps() {
    let _watch = common::watch();
    let mut g = game(FLAT, 1);
    assert_eq!(g.particles(), 0);
    g.step(NONE);
    assert_eq!(g.particles(), 4, "LANDED spawns 4 particles");
    run(&mut g, NONE, 9);
    assert_eq!(
        g.particles(),
        4,
        "after 9 more steps every particle is still alive"
    );
    let mut last = 4;
    for _ in 0..31 {
        g.step(NONE);
        assert!(
            g.particles() <= last,
            "particles only disappear while nothing happens"
        );
        last = g.particles();
    }
    assert_eq!(
        g.particles(),
        0,
        "40 steps after spawning, no particle is left"
    );
}

#[test]
fn each_event_spawns_its_number_of_particles() {
    let _watch = common::watch();
    // JUMPED: 4.
    let mut g = game(FLAT, 1);
    run(&mut g, NONE, 60);
    assert_eq!(g.particles(), 0);
    assert_eq!(g.step(JUMP), Events(Events::JUMPED));
    assert_eq!(g.particles(), 4, "JUMPED spawns 4");

    // STOMP: 12 for each enemy defeated.
    let mut g = game(DROP_ON_WALKER, 1);
    run(&mut g, NONE, 20);
    assert_eq!(g.particles(), 0);
    assert_eq!(g.step(NONE), Events(Events::STOMP));
    assert_eq!(g.particles(), 12, "STOMP spawns 12");

    // HURT: 16.
    let mut g = game(WALK_INTO_PLAYER, 1);
    run(&mut g, NONE, 74);
    assert_eq!(
        g.particles(),
        0,
        "the landing dust of step 1 is gone by step 74"
    );
    assert_eq!(g.step(NONE), Events(Events::HURT));
    assert_eq!(g.particles(), 16, "HURT spawns 16");

    // SPARK: 6 for each spark; WON: 48.
    let mut g = game("#.P..o....E#\n############\n", 1);
    run(&mut g, NONE, 60);
    assert_eq!(
        run_until(&mut g, RIGHT, Events::SPARK, 100),
        Events(Events::SPARK)
    );
    assert_eq!(g.particles(), 6, "SPARK spawns 6");
    run(&mut g, NONE, 60);
    assert_eq!(
        run_until(&mut g, RIGHT, Events::WON, 100),
        Events(Events::WON)
    );
    assert_eq!(g.particles(), 48, "WON spawns 48");
    run(&mut g, NONE, 40);
    assert_eq!(
        g.particles(),
        0,
        "particles keep ageing after the game is over"
    );
}

#[test]
fn the_particle_pool_holds_at_most_64() {
    let _watch = common::watch();
    // Nib falls down a shaft at top speed through four sparks and into the
    // open flue. The last three sparks (6 particles each) and the win (48)
    // fall within ten steps, and every particle lives at least ten steps:
    // 66 particles want to be alive at once.
    let shaft = "\
#P#
#.#
#.#
#.#
#.#
#.#
#o#
#o#
#o#
#o#
#E#
###
";
    let mut g = game(shaft, 3);
    let mut most = 0;
    let events = loop {
        let events = g.step(NONE);
        most = most.max(g.particles());
        assert!(
            g.particles() <= 64,
            "tick {}: {} particles",
            g.tick(),
            g.particles()
        );
        if events.has(Events::WON) {
            break events;
        }
        assert!(
            g.tick() < 200,
            "the fall down the shaft never reached the flue"
        );
    };
    assert!(events.has(Events::WON));
    assert_eq!(g.sparks_left(), 0);
    assert_eq!(
        g.particles(),
        64,
        "the pool is full after the winning step: particles that do not fit are dropped, and \
         no live particle is removed to make room"
    );
    assert_eq!(most, 64);
}

/// SPEC 12.3: the offset for a shake with `t` steps left.
fn shake_for(t: i32) -> (i32, i32) {
    if t == 0 {
        return (0, 0);
    }
    let amp = (t + 2) / 3;
    (
        if t % 2 == 1 { amp } else { -amp },
        if (t / 2) % 2 == 1 { amp } else { -amp },
    )
}

#[test]
fn the_shake_pattern_is_the_one_in_the_spec() {
    let _watch = common::watch();
    assert_eq!(shake_for(12), (-4, -4), "worked example in SPEC 12.3");
    assert_eq!(shake_for(11), (4, 4));
    assert_eq!(shake_for(1), (1, -1));

    let mut g = game(WALK_INTO_PLAYER, 1);
    for _ in 0..74 {
        g.step(NONE);
        assert_eq!(g.shake(), (0, 0), "no shake before anything happened");
    }
    assert_eq!(g.step(NONE), Events(Events::HURT));
    for t in (0..=12).rev() {
        assert_eq!(
            g.shake(),
            shake_for(t),
            "a HURT shake lasts 12 steps; {t} are left at tick {}",
            g.tick()
        );
        g.step(NONE);
    }
    assert_eq!(g.shake(), (0, 0));

    let mut g = game(DROP_ON_WALKER, 1);
    run(&mut g, NONE, 21);
    for t in (0..=6).rev() {
        assert_eq!(
            g.shake(),
            shake_for(t),
            "a STOMP shake lasts 6 steps; {t} are left"
        );
        g.step(NONE);
    }
}

/// The HUD strip as SPEC 12.4 defines it, drawn with the rasteriser.
fn expected_hud(sparks: (u32, u32), lives: u32, score: u32) -> Framebuffer {
    let mut fb = screen();
    fb.fill_rect(0, 0, 320, 11, 0xFF10_101C);
    let line = format!(
        "SPARKS {}/{}  LIVES {}  SCORE {:06}",
        sparks.0, sparks.1, lives, score
    );
    fb.text(&line, 4, 2, 0xFFF4_F0E0);
    fb
}

#[track_caller]
fn assert_hud(g: &Game, sparks: (u32, u32), lives: u32, score: u32) {
    let mut fb = screen();
    fb.clear(0xFFFF_00FF);
    g.render(&mut fb, 256);
    let expected = expected_hud(sparks, lives, score);
    let rows = 11 * SCREEN_W;
    assert!(
        fb.pixels()[..rows] == expected.pixels()[..rows],
        "the top 11 rows must be the HUD of SPEC 12.4 showing sparks {}/{}, lives {lives}, \
         score {score}",
        sparks.0,
        sparks.1
    );
}

#[test]
fn the_hud_shows_sparks_lives_and_score() {
    let _watch = common::watch();
    let g = game(LEVEL_1, 1);
    assert_hud(&g, (0, 14), 3, 0);

    let pool = "\
#.........o........#
#..................#
#.P.o.o.........E..#
#######~~###########
####################
";
    let mut g = game(pool, 1);
    assert_hud(&g, (0, 3), 3, 0);
    run_until(&mut g, RIGHT, Events::SPARK, 200);
    assert_hud(&g, (1, 3), 3, 10);
    // Keep running: Nib takes the second spark and drowns in the pool.
    run_until(&mut g, RIGHT, Events::HURT, 400);
    assert_hud(&g, (2, 3), 2, 20);
    // The HUD does not shake.
    assert_ne!(g.shake(), (0, 0));
    assert_hud(&g, (2, 3), 2, 20);

    let mut g = game(DROP_ON_WALKER, 1);
    run(&mut g, NONE, 21);
    assert_hud(&g, (0, 0), 3, 50);
}

#[test]
fn render_overwrites_every_pixel() {
    let _watch = common::watch();
    let mut g = game(LEVEL_1, 1);
    run(&mut g, RIGHT, 30);
    let mut a = screen();
    let mut b = screen();
    a.clear(0xFFFF_00FF);
    b.clear(0xFF00_FF00);
    g.render(&mut a, 256);
    g.render(&mut b, 256);
    assert!(
        a.pixels() == b.pixels(),
        "what was in the buffer before must not show through: render draws the whole frame"
    );
}

#[test]
fn render_depends_only_on_the_game() {
    let _watch = common::watch();
    let inputs = |i: u64| Input((i / 9 % 8) as u8);
    let mut a = game(LEVEL_1, 5);
    let mut b = game(LEVEL_1, 5);
    let mut distinct = std::collections::HashSet::new();
    let mut places = std::collections::HashSet::new();
    for i in 0..300 {
        a.step(inputs(i));
        b.step(inputs(i));
        let p = a.player();
        places.insert((p.x >> 8, p.y >> 8));
        // `a` is drawn every step, `b` only now and then: drawing must not
        // change the game, and the same state must give the same picture.
        let hash_before = a.state_hash();
        let picture = frame(&a, 256);
        assert_eq!(a.state_hash(), hash_before, "render changed the simulation");
        distinct.insert(picture);
        if i % 50 == 49 {
            assert_eq!(
                frame(&b, 256),
                picture,
                "tick {}: two games with the same history drew different frames",
                a.tick()
            );
            assert_eq!(
                frame(&a, 256),
                picture,
                "drawing the same state twice gave two pictures"
            );
        }
    }
    // No fixed number here: how lively the picture is, is yours to decide.
    // But Nib is drawn where it is, so a picture with no animation at all
    // still changes about as often as Nib reaches a new pixel. Half of that
    // leaves room for any way of rounding.
    assert!(
        places.len() >= 60,
        "the test expects Nib to get around; it visited only {} pixel positions",
        places.len()
    );
    assert!(
        distinct.len() >= places.len() / 2,
        "in 300 steps of running and jumping Nib stood at {} different pixel positions, but \
         only {} different frames were drawn; is the player drawn at its position?",
        places.len(),
        distinct.len()
    );
}

#[test]
fn interpolation_only_matters_when_something_moved() {
    let _watch = common::watch();
    let mut g = game(FLAT, 1);
    run(&mut g, NONE, 100);
    assert_eq!(g.particles(), 0);
    let still = frame(&g, 256);
    for alpha in [0, 1, 128, 255] {
        assert_eq!(
            frame(&g, alpha),
            still,
            "nothing moved in the latest step, so alpha = {alpha} must draw the same frame as 256"
        );
    }
    run(&mut g, RIGHT, 30);
    assert_ne!(
        frame(&g, 0),
        frame(&g, 256),
        "the player moved 2.5 pixels in the latest step: alpha 0 draws it at the old position, \
         alpha 256 at the new one"
    );
}

/// A wide, flat cavern: the camera follows Nib, so a frame drawn at the
/// wrong position differs in most of its pixels.
const LONG_HALL: &str = "\
#......................................#
#......................................#
#.P.................E..................#
########################################
";

/// The same hall with a pool in the way.
const LONG_POOL: &str = "\
#......................................#
#......................................#
#.P..................................E.#
####################~~~#################
########################################
";

#[track_caller]
fn assert_alpha_does_not_matter(g: &Game, why: &str) {
    let still = frame(g, 256);
    for alpha in [0, 128, 255] {
        assert_eq!(
            frame(g, alpha),
            still,
            "tick {}, alpha = {alpha}: {why}",
            g.tick()
        );
    }
}

#[test]
fn a_finished_game_does_not_slide_between_steps() {
    let _watch = common::watch();
    // Won at full running speed: Nib moved in the winning step.
    let mut g = game(LONG_HALL, 1);
    run_until(&mut g, RIGHT, Events::WON, 300);
    assert_eq!(g.player().vx, 640);
    // The win burst lives at most 40 steps. After that nothing moves.
    run(&mut g, RIGHT, 41);
    assert_eq!(g.particles(), 0);
    assert_alpha_does_not_matter(
        &g,
        "the game is won and the latest step moved nothing, so the frame must not depend on \
         alpha. \"Previous\" is the position at the start of the latest step, also when that \
         step did nothing",
    );

    // Lost in the pool, again in the middle of a movement.
    let mut g = game(LONG_POOL, 1);
    run_until(&mut g, RIGHT, Events::LOST, 2000);
    run(&mut g, RIGHT, 41);
    assert_eq!(g.particles(), 0);
    assert_alpha_does_not_matter(
        &g,
        "the game is lost and the latest step moved nothing, so the frame must not depend on \
         alpha",
    );
}

#[test]
fn a_respawn_is_not_interpolated() {
    let _watch = common::watch();
    let mut g = game(LONG_POOL, 1);
    let start = g.player();
    let events = run_until(&mut g, RIGHT, Events::HURT, 600);
    assert_eq!(events, Events(Events::HURT));
    let p = g.player();
    assert_eq!(
        (p.x, p.y),
        (start.x, start.y),
        "back at the start in the step of the drowning"
    );
    // Nib went from the pool to the start within this step. That is a
    // teleport, not a movement of 18 tiles: at every alpha Nib is drawn at
    // the start. (Nothing else moved: there are no enemies, and the splash
    // particles spawned in this step have not moved yet.)
    assert_alpha_does_not_matter(
        &g,
        "a respawn must not be interpolated: \"previous\" becomes the start position too",
    );
}
