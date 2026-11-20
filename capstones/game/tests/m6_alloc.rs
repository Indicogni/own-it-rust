//! Milestone 6: the per-frame allocation budget (SPEC section 16).
//!
//! This file is a test binary of its own because it replaces the global
//! allocator with one that counts.

mod common;

use common::{Counting, DEMO_REPLAY, LEVEL_1, ROUGH_REPLAY, count_allocations, game};
use game::{FixedStep, Framebuffer, Input, MAX_STEPS, Replay, SCREEN_H, SCREEN_W, STEP_MICROS};

#[global_allocator]
static ALLOCATOR: Counting = Counting;

#[test]
fn the_counting_allocator_counts() {
    let _watch = common::watch();
    let (v, n) = count_allocations(|| vec![1u8; 100]);
    assert_eq!(v.len(), 100);
    assert!(
        n >= 1,
        "the harness is broken: a Vec was built and no allocation was counted"
    );
}

#[test]
fn a_frame_allocates_nothing_after_start_up() {
    let _watch = common::watch();
    for (name, text) in [("demo", DEMO_REPLAY), ("rough", ROUGH_REPLAY)] {
        let replay = Replay::from_text(text).expect("a shipped replay parses");
        // Start-up: everything may allocate here.
        let mut g = game(LEVEL_1, replay.seed);
        let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
        let mut clock = FixedStep::new(STEP_MICROS, MAX_STEPS);
        g.render(&mut fb, 256);

        // From here on: a whole recorded game, one frame per step, as the
        // window loop would run it. Jumps, pickups, stomps, injuries, the
        // HUD changing, particles, shake, winning and losing all happen.
        let mut worst: Option<(u64, usize)> = None;
        let mut total = 0;
        for &input in &replay.inputs {
            let ((), n) = count_allocations(|| {
                for _ in 0..clock.advance(STEP_MICROS) {
                    g.step(input);
                }
                g.render(&mut fb, clock.alpha());
                std::hint::black_box(fb.checksum());
            });
            total += n;
            if n > 0 && worst.is_none_or(|(_, most)| n > most) {
                worst = Some((g.tick(), n));
            }
        }
        if let Some((tick, n)) = worst {
            panic!(
                "{name} replay: {total} heap allocation(s) after start-up; the worst frame is \
                 tick {tick} with {n}. `step`, `render` and the rasteriser must not allocate: \
                 look for format!, String, Vec::new/collect, Box and to_string in the frame path"
            );
        }
        std::hint::black_box(Input::NONE);
    }
}
