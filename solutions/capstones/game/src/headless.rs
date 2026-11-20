//! Headless mode (SPEC section 14): run a replay with no window, print the
//! state hash and frame checksum of every step, optionally save pictures.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use game::{Framebuffer, Game, Phase, Replay, SCREEN_H, SCREEN_W};

use crate::Options;

pub(crate) fn run(level: &str, replay: &Replay, options: &Options) -> Result<(), String> {
    let mut game = Game::new(level, replay.seed).map_err(|e| format!("level: {e}"))?;
    let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);

    let mut out: Box<dyn Write> = match &options.out {
        Some(path) => {
            let file =
                File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
            Box::new(BufWriter::new(file))
        }
        None => Box::new(BufWriter::new(io::stdout().lock())),
    };

    let last = replay.inputs.len() as u64;
    if let Some(dir) = &options.shots {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }

    let report = |e: io::Error| format!("cannot write output: {e}");
    for &input in &replay.inputs {
        game.step(input);
        game.render(&mut fb, 256);
        let tick = game.tick();
        writeln!(
            out,
            "{tick} {:016x} {:016x}",
            game.state_hash(),
            fb.checksum()
        )
        .map_err(report)?;

        if let Some(dir) = &options.shots {
            let wanted = if options.shot_at.is_empty() {
                tick == last
            } else {
                options.shot_at.contains(&tick)
            };
            if wanted {
                let path = dir.join(format!("frame_{tick:06}.ppm"));
                save(&fb, &path).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
            }
        }
    }

    let phase = match game.phase() {
        Phase::Playing => "playing",
        Phase::Won => "won",
        Phase::Lost => "lost",
    };
    writeln!(
        out,
        "end phase={phase} score={} lives={} ticks={}",
        game.score(),
        game.lives(),
        game.tick()
    )
    .map_err(report)?;
    out.flush().map_err(report)
}

pub(crate) fn save(fb: &Framebuffer, path: &Path) -> io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    fb.write_ppm(&mut file)?;
    file.flush()
}
