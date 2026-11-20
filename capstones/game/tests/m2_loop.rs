//! Milestone 2: the fixed timestep (SPEC section 6).
//!
//! The window itself cannot be tested here. What can be tested is the part
//! of the loop that matters for correctness: how real time is turned into
//! simulation steps, and that the result does not depend on the frame rate.

mod common;

use common::{LEVEL_1, RIGHT, game};
use game::{FixedStep, Game, Input, MAX_STEPS, STEP_MICROS};

#[test]
fn advance_returns_whole_steps_and_keeps_the_rest() {
    let _watch = common::watch();
    let mut clock = FixedStep::new(1000, 5);
    assert_eq!(clock.alpha(), 0, "a new clock has no left-over time");
    assert_eq!(
        clock.advance(2500),
        2,
        "2500 us is two whole steps of 1000 us"
    );
    assert_eq!(clock.alpha(), 128, "500 us left of 1000 us is 128/256");
    assert_eq!(clock.advance(499), 0, "999 us is not yet a step");
    assert_eq!(clock.alpha(), 255, "999 * 256 / 1000 rounds down to 255");
    assert_eq!(
        clock.advance(1),
        1,
        "the left-over time counts toward the next step"
    );
    assert_eq!(clock.alpha(), 0);
    assert_eq!(clock.advance(0), 0);
    assert_eq!(clock.advance(5000), 5, "exactly max_steps is allowed");
    assert_eq!(clock.alpha(), 0);
}

#[test]
fn a_long_pause_is_capped_and_the_backlog_is_dropped() {
    let _watch = common::watch();
    let mut clock = FixedStep::new(1000, 5);
    clock.advance(700);
    assert_eq!(
        clock.advance(60_000),
        5,
        "never more than max_steps per call"
    );
    assert_eq!(
        clock.alpha(),
        0,
        "when the cap is hit, all left-over time is thrown away (SPEC 6.2)"
    );
    assert_eq!(
        clock.advance(999),
        0,
        "the dropped backlog must not come back later"
    );
    assert_eq!(
        clock.advance(u64::MAX),
        5,
        "huge elapsed times must not overflow"
    );
    assert_eq!(clock.advance(u64::MAX), 5);
}

#[test]
fn alpha_is_exact_for_the_real_step_length() {
    let _watch = common::watch();
    let mut clock = FixedStep::new(STEP_MICROS, MAX_STEPS);
    assert_eq!(clock.advance(STEP_MICROS + 5000), 1);
    assert_eq!(clock.alpha(), 76, "5000 * 256 / 16667 == 76 (rounded down)");
    let mut steps = 0;
    for _ in 0..1000 {
        steps += clock.advance(6944); // a 144 Hz display
        assert!(clock.alpha() <= 255, "alpha stays in 0..=255");
    }
    // 5000 + 1000 * 6944 us = 416 whole steps and 15528 us.
    assert_eq!(steps, 416);
    assert_eq!(clock.alpha(), 15528 * 256 / 16667);
}

/// Runs the game loop of SPEC 6.3 over a list of frame times and returns
/// the number of steps and the final state hash.
fn drive(frame_micros: impl IntoIterator<Item = u64>) -> (u32, u64) {
    let mut game: Game = game(LEVEL_1, 7);
    let mut clock = FixedStep::new(STEP_MICROS, MAX_STEPS);
    let mut steps = 0;
    for elapsed in frame_micros {
        for _ in 0..clock.advance(elapsed) {
            game.step(RIGHT);
            steps += 1;
        }
    }
    (steps, game.state_hash())
}

#[test]
fn the_simulation_does_not_depend_on_the_frame_rate() {
    let _watch = common::watch();
    // Two seconds of game time (120 steps), cut into frames in five ways.
    let total = 120 * STEP_MICROS;
    let even = |n: u64| {
        let mut frames = vec![total / n; n as usize];
        frames[0] += total % n;
        frames
    };
    let irregular = {
        let mut frames = Vec::new();
        let mut left = total;
        let mut i = 0u64;
        while left > 0 {
            let want = 3000 + (i * 7919) % 29_000;
            let take = want.min(left);
            frames.push(take);
            left -= take;
            i += 1;
        }
        frames
    };
    let reference = drive(even(120));
    assert_eq!(
        reference.0, 120,
        "120 frames of exactly one step each run 120 steps"
    );
    for (name, frames) in [
        ("30 Hz", even(60)),
        ("144 Hz", even(288)),
        ("1000 Hz", even(2000)),
        ("irregular frame times", irregular),
    ] {
        assert_eq!(
            drive(frames),
            reference,
            "at {name} the game ran a different number of steps or reached a different state; \
             the simulation must only ever advance by whole fixed steps"
        );
    }
}

#[test]
fn the_same_seed_and_inputs_give_the_same_game() {
    let _watch = common::watch();
    let inputs = |i: u64| Input((i / 13 % 8) as u8);
    let mut a = game(LEVEL_1, 99);
    let mut b = game(LEVEL_1, 99);
    for i in 0..600 {
        assert_eq!(
            a.step(inputs(i)),
            b.step(inputs(i)),
            "events differ at step {}",
            i + 1
        );
        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "states differ after step {}",
            i + 1
        );
    }
    // A different seed sends the Mistwings another way from the first step.
    let mut d = game(LEVEL_1, 99);
    let mut e = game(LEVEL_1, 100);
    let differs = (0..60).any(|_| {
        d.step(Input::NONE);
        e.step(Input::NONE);
        (d.enemy(0), d.enemy(1)) != (e.enemy(0), e.enemy(1))
    });
    assert!(
        differs,
        "seeds 99 and 100 must send the two Mistwings of level 1 different ways (their first \
         headings are below(8) of each stream: see SPEC 10.8); the seed must drive the \
         simulation's random stream"
    );
}
