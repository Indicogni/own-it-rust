//! Windowed play: the real-time loop around the fixed-step simulation.

use std::path::Path;
use std::time::Instant;

use game::{
    FixedStep, Framebuffer, Game, MAX_STEPS, Phase, Replay, SCREEN_H, SCREEN_W, STEP_MICROS,
};

use crate::Options;
use crate::headless::save;
use crate::platform::{Command, Platform};

/// Room for an hour of play, so recording does not allocate mid-game.
const RECORD_CAPACITY: usize = 60 * 60 * 60;

pub(crate) fn run(
    levels: &[String],
    playback: Option<Replay>,
    options: &Options,
) -> Result<(), String> {
    let mut seed = match &playback {
        Some(replay) => replay.seed,
        None => options.seed.unwrap_or(1),
    };
    let mut level = 0;
    let new_game = |level: usize, seed: u64| {
        Game::new(&levels[level], seed).map_err(|e| format!("level: {e}"))
    };
    let mut game = new_game(level, seed)?;
    let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
    let mut platform = Platform::open("Underkiln", SCREEN_W, SCREEN_H, options.scale.unwrap_or(4))?;

    let mut recorded = Replay {
        seed,
        inputs: Vec::with_capacity(RECORD_CAPACITY),
    };
    let mut clock = FixedStep::new(STEP_MICROS, MAX_STEPS);
    let mut last = Instant::now();
    let mut frames = 0u64;

    while platform.is_open() && options.frames.is_none_or(|limit| frames < limit) {
        let now = Instant::now();
        let elapsed = now
            .duration_since(last)
            .as_micros()
            .min(u128::from(u64::MAX)) as u64;
        last = now;

        if platform.pressed(Command::Restart) && playback.is_none() {
            // A new attempt is a new game with a new seed and a new record.
            // After a win it is an attempt at the next level.
            if game.phase() == Phase::Won {
                level = (level + 1) % levels.len();
            }
            seed = seed.wrapping_add(1);
            game = new_game(level, seed)?;
            recorded.seed = seed;
            recorded.inputs.clear();
        }

        // Keys are sampled once per displayed frame; every step that frame
        // owes gets the same input.
        let keys = platform.input();
        for _ in 0..clock.advance(elapsed) {
            let input = match &playback {
                Some(replay) => match replay.inputs.get(game.tick() as usize) {
                    Some(&input) => input,
                    // The replay is over: the simulation stops here.
                    None => break,
                },
                None => keys,
            };
            game.step(input);
            recorded.inputs.push(input);
        }

        // Once a replay has run out there is no "next step" to blend toward:
        // show its final state exactly, instead of sliding between the last
        // two positions for as long as the window stays open.
        let replay_over = playback
            .as_ref()
            .is_some_and(|replay| game.tick() >= replay.inputs.len() as u64);
        let alpha = if replay_over { 256 } else { clock.alpha() };
        game.render(&mut fb, alpha);
        if platform.pressed(Command::Screenshot) {
            let name = format!("underkiln-{:06}.ppm", game.tick());
            match save(&fb, Path::new(&name)) {
                Ok(()) => eprintln!("game: saved {name}"),
                Err(e) => eprintln!("game: cannot save {name}: {e}"),
            }
        }
        platform.present(&fb)?;
        frames += 1;
    }

    if let Some(path) = &options.record {
        std::fs::write(path, recorded.to_text())
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        eprintln!(
            "game: recorded {} steps of level {} to {}",
            recorded.inputs.len(),
            level + 1,
            path.display()
        );
    }
    Ok(())
}
