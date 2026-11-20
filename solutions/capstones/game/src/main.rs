//! The `game` binary: command line, then one of two front ends.
//!
//! - windowed play (`platform.rs`, only with the `window` feature), and
//! - headless replay (`headless.rs`, always available).
//!
//! Both drive the same library through the same few calls. Run with
//! `--help` for the options.

#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod headless;
#[cfg(feature = "window")]
mod platform;
#[cfg(feature = "window")]
mod play;

/// The levels compiled into the program, in playing order. The first one
/// is the level the SPEC ships; the second exists only in this solution.
const LEVELS: [&str; 2] = [
    include_str!("../assets/level1.txt"),
    include_str!("../assets/level2.txt"),
];

const HELP: &str = "\
Underkiln

USAGE:
    game [OPTIONS]                          play in a window
    game --headless --replay FILE [OPTIONS] run a replay without a window

OPTIONS:
    --level FILE       level to load (default: the built-in levels, starting with the first)
    --seed N           seed for a new game (default: 1)
    --replay FILE      feed the game from a replay instead of the keyboard
    --record FILE      windowed: save the session's inputs as a replay on exit
    --frames N         windowed: quit after showing N frames
    --scale N          windowed: 1, 2 or 4 screen pixels per game pixel (default: 4)
    --headless         no window: step the replay, print one line per step
    --out FILE         headless: write the lines to FILE instead of standard output
    --shots DIR        headless: save screenshots (PPM) into DIR
    --shot-at T,T,...  headless: steps to capture (default: the last step)
    -h, --help         show this text

KEYS:
    left/right or A/D  run          R       start again (after a win: next level)
    space, up, W or Z  jump         F2      save a screenshot
                                    escape  quit
";

/// Everything the command line can say. Plain data; no behaviour.
#[derive(Default)]
struct Options {
    level: Option<PathBuf>,
    seed: Option<u64>,
    replay: Option<PathBuf>,
    record: Option<PathBuf>,
    frames: Option<u64>,
    scale: Option<u32>,
    headless: bool,
    out: Option<PathBuf>,
    shots: Option<PathBuf>,
    shot_at: Vec<u64>,
    help: bool,
}

/// Arguments arrive as `OsString`, not `String`: a file name need not be
/// valid Unicode, and `std::env::args` would panic on one that is not.
/// Paths stay as the system gave them; only numbers must be text.
fn parse_args(args: impl Iterator<Item = OsString>) -> Result<Options, String> {
    fn number<T: std::str::FromStr>(flag: &str, value: &OsStr) -> Result<T, String> {
        value
            .to_str()
            .and_then(|text| text.parse().ok())
            .ok_or_else(|| {
                format!(
                    "{flag}: `{}` is not a valid number",
                    value.to_string_lossy()
                )
            })
    }

    let mut options = Options::default();
    let mut args = args;
    while let Some(arg) = args.next() {
        let unknown = || format!("unknown option `{}` (try --help)", arg.to_string_lossy());
        let flag = arg.to_str().ok_or_else(unknown)?;
        let mut value = || args.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag {
            "-h" | "--help" => options.help = true,
            "--headless" => options.headless = true,
            "--level" => options.level = Some(value()?.into()),
            "--replay" => options.replay = Some(value()?.into()),
            "--record" => options.record = Some(value()?.into()),
            "--out" => options.out = Some(value()?.into()),
            "--shots" => options.shots = Some(value()?.into()),
            "--seed" => options.seed = Some(number(flag, &value()?)?),
            "--frames" => options.frames = Some(number(flag, &value()?)?),
            "--scale" => options.scale = Some(number(flag, &value()?)?),
            "--shot-at" => {
                let list = value()?;
                let text = list
                    .to_str()
                    .ok_or_else(|| format!("{flag}: expected numbers separated by commas"))?;
                for part in text.split(',') {
                    options.shot_at.push(number(flag, OsStr::new(part))?);
                }
            }
            _ => return Err(unknown()),
        }
    }
    Ok(options)
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

fn run() -> Result<(), String> {
    let options = parse_args(std::env::args_os().skip(1))?;
    if options.help {
        print!("{HELP}");
        return Ok(());
    }
    // One level from a file, or the built-in ones in order.
    let levels: Vec<String> = match &options.level {
        Some(path) => vec![read(path)?],
        None => LEVELS.iter().map(|&level| level.to_owned()).collect(),
    };
    let replay = match &options.replay {
        Some(path) => {
            let text = read(path)?;
            let replay = game::Replay::from_text(&text);
            Some(replay.map_err(|e| format!("{}: {e}", path.display()))?)
        }
        None => None,
    };

    if options.headless {
        let replay = replay.ok_or("--headless needs --replay FILE")?;
        return headless::run(&levels[0], &replay, &options);
    }

    #[cfg(feature = "window")]
    {
        play::run(&levels, replay, &options)
    }
    #[cfg(not(feature = "window"))]
    {
        let _ = (levels, replay);
        Err(
            "this build has no window (it was built without the `window` feature); \
             use --headless --replay FILE"
                .to_owned(),
        )
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("game: {message}");
            ExitCode::from(2)
        }
    }
}
