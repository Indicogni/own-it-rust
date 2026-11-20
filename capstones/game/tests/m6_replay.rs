//! Milestone 6: replays and the headless binary (SPEC sections 13 and 14).

mod common;

use std::fs::File;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use common::{DEMO_REPLAY, LEVEL_1, ROUGH_REPLAY, game};
use game::{Framebuffer, Input, Phase, Replay, ReplayError, SCREEN_H, SCREEN_W};

const DEMO_TRACE: &str = include_str!("golden/demo.trace");
const ROUGH_TRACE: &str = include_str!("golden/rough.trace");

#[track_caller]
fn replay(text: &str) -> Replay {
    match Replay::from_text(text) {
        Ok(replay) => replay,
        Err(e) => panic!("Replay::from_text rejected a valid replay with {e:?}"),
    }
}

#[test]
fn to_text_is_canonical() {
    let _watch = common::watch();
    let r = Replay {
        seed: 18_446_744_073_709_551_615,
        inputs: [0, 0, 0, 2, 2, 6, 2, 2, 0].map(Input).to_vec(),
    };
    assert_eq!(
        r.to_text(),
        "underkiln-replay 1\nseed 18446744073709551615\nticks 9\n3 0\n2 2\n1 6\n2 2\n1 0\n",
        "header, seed, ticks, then one line per run of equal inputs; every line ends in \\n"
    );
    let empty = Replay {
        seed: 0,
        inputs: Vec::new(),
    };
    assert_eq!(empty.to_text(), "underkiln-replay 1\nseed 0\nticks 0\n");
}

#[test]
fn from_text_reads_what_to_text_writes() {
    let _watch = common::watch();
    let r = Replay {
        seed: 77,
        inputs: (0..5000u32)
            .map(|i| Input((i * i / 97 % 8) as u8))
            .collect(),
    };
    assert_eq!(replay(&r.to_text()), r);
    let demo = replay(DEMO_REPLAY);
    assert_eq!(
        demo.to_text(),
        DEMO_REPLAY.replace("\r\n", "\n"),
        "the shipped replay is canonical"
    );
}

#[test]
fn from_text_is_tolerant_about_layout() {
    let _watch = common::watch();
    let text = "# recorded by hand\r\nunderkiln-replay 1\r\n\r\nseed 5\r\nticks 6\r\n\
                # runs need not be merged\r\n2 1\r\n1 1\r\n3 4\r\n";
    assert_eq!(
        replay(text),
        Replay {
            seed: 5,
            inputs: [1, 1, 1, 4, 4, 4].map(Input).to_vec(),
        },
        "blank lines, `#` comments, \\r\\n and unmerged runs are all accepted"
    );
}

#[test]
fn from_text_reports_what_is_wrong() {
    let _watch = common::watch();
    #[track_caller]
    fn check(text: &str, expected: ReplayError) {
        assert_eq!(
            Replay::from_text(text),
            Err(expected),
            "for the text:\n{text}"
        );
    }
    check("", ReplayError::BadHeader);
    check(
        "underkiln-replay 2\nseed 1\nticks 0\n",
        ReplayError::BadHeader,
    );
    check("underkiln-replay 1\nticks 0\n", ReplayError::BadSeed);
    check(
        "underkiln-replay 1\nseed -1\nticks 0\n",
        ReplayError::BadSeed,
    );
    check(
        "underkiln-replay 1\nseed +1\nticks 0\n",
        ReplayError::BadSeed,
    );
    check(
        "underkiln-replay 1\nseed 18446744073709551616\nticks 0\n",
        ReplayError::BadSeed,
    );
    check("underkiln-replay 1\nseed 1\n", ReplayError::BadTicks);
    check(
        "underkiln-replay 1\nseed 1\nticks many\n",
        ReplayError::BadTicks,
    );
    check(
        "underkiln-replay 1\nseed 1\nticks 10000001\n",
        ReplayError::BadTicks,
    );
    check(
        "underkiln-replay 1\nseed 1\nticks 3\n3\n",
        ReplayError::BadRun { line: 4 },
    );
    check(
        "underkiln-replay 1\nseed 1\nticks 3\n2 1\n\n1 8\n",
        ReplayError::BadRun { line: 6 },
    );
    check(
        "underkiln-replay 1\nseed 1\nticks 3\n0 1\n3 1\n",
        ReplayError::BadRun { line: 4 },
    );
    check(
        "underkiln-replay 1\nseed 1\nticks 3\n3 1 1\n",
        ReplayError::BadRun { line: 4 },
    );
    check(
        "underkiln-replay 1\nseed 1\nticks 3\n18446744073709551616 1\n",
        ReplayError::BadRun { line: 4 },
    );
    check(
        "underkiln-replay 1\nseed 1\nticks 3\n2 1\n",
        ReplayError::LengthMismatch {
            expected: 3,
            found: 2,
        },
    );
    // Counts are added up without overflow: the sum stops at u64::MAX.
    check(
        "underkiln-replay 1\nseed 1\nticks 3\n18446744073709551615 1\n18446744073709551615 0\n",
        ReplayError::LengthMismatch {
            expected: 3,
            found: u64::MAX,
        },
    );
    check(
        "underkiln-replay 1\nseed 1\nticks 3\n2 1\n2 0\n",
        ReplayError::LengthMismatch {
            expected: 3,
            found: 4,
        },
    );
}

#[test]
fn a_huge_claimed_length_does_not_allocate_before_it_is_checked() {
    let _watch = common::watch();
    // A tiny file that claims the longest allowed replay but holds one step.
    let text = "underkiln-replay 1\nseed 1\nticks 10000000\n1 0\n";
    assert_eq!(
        Replay::from_text(text),
        Err(ReplayError::LengthMismatch {
            expected: 10_000_000,
            found: 1,
        })
    );
}

/// Runs `replay_text` on level 1 and compares against a golden trace: lines
/// of `tick x y score lives state_hash`.
#[track_caller]
fn check_trace(replay_text: &str, trace: &str) -> game::Game {
    let r = replay(replay_text);
    let mut g = game(LEVEL_1, r.seed);
    let mut lines = trace
        .lines()
        .filter(|line| !line.starts_with('#'))
        .peekable();
    for &input in &r.inputs {
        g.step(input);
        let Some(&line) = lines.peek() else { continue };
        let fields: Vec<&str> = line.split(' ').collect();
        let tick: u64 = fields[0].parse().expect("golden trace: tick");
        if tick != g.tick() {
            continue;
        }
        lines.next();
        let p = g.player();
        let actual = format!(
            "{} {} {} {} {} {:016x}",
            g.tick(),
            p.x,
            p.y,
            g.score(),
            g.lives(),
            g.state_hash()
        );
        assert!(
            actual == line,
            "the run leaves the reference at or before tick {tick}.\n\
             fields:   tick x y score lives state_hash\n\
             expected: {line}\n\
             actual:   {actual}\n\
             If x, y, score and lives agree and only the hash differs, an enemy, the random \
             stream or a velocity is off. Earlier milestones test each rule on its own."
        );
    }
    assert!(
        lines.next().is_none(),
        "the replay ended before the golden trace did"
    );
    g
}

#[test]
fn the_demo_replay_wins_level_1_exactly_as_recorded() {
    let _watch = common::watch();
    let g = check_trace(DEMO_REPLAY, DEMO_TRACE);
    assert_eq!(g.phase(), Phase::Won);
    assert_eq!(
        (g.score(), g.lives(), g.tick()),
        (DEMO_SCORE, 3, DEMO_TICKS)
    );
    assert_eq!(g.sparks_left(), 0);
}
const DEMO_SCORE: u32 = 590;
const DEMO_TICKS: u64 = 1016;

#[test]
fn the_rough_replay_loses_level_1_exactly_as_recorded() {
    let _watch = common::watch();
    let g = check_trace(ROUGH_REPLAY, ROUGH_TRACE);
    assert_eq!(g.phase(), Phase::Lost);
    assert_eq!(g.lives(), 0);
}

#[test]
fn a_replay_reproduces_a_run_bit_for_bit() {
    let _watch = common::watch();
    // Play with scripted "live" input while recording, then replay the
    // recording through its text form: every state and every frame agree.
    let live = |i: u64| Input(((i / 7) ^ (i / 31) ^ (i / 3 % 2 * 4)) as u8 & Input::ALL);
    let seed = 2024;
    let mut recorded = Replay {
        seed,
        inputs: Vec::new(),
    };
    let mut first = game(LEVEL_1, seed);
    let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
    let mut history = Vec::new();
    for i in 0..900 {
        let input = live(i);
        first.step(input);
        recorded.inputs.push(input);
        first.render(&mut fb, 256);
        history.push((first.state_hash(), fb.checksum()));
    }

    let loaded = replay(&recorded.to_text());
    let mut second = game(LEVEL_1, loaded.seed);
    for (i, &input) in loaded.inputs.iter().enumerate() {
        second.step(input);
        second.render(&mut fb, 256);
        assert_eq!(
            (second.state_hash(), fb.checksum()),
            history[i],
            "tick {}: the replay differs from the original run (state hash, frame checksum)",
            i + 1
        );
    }
}

// ---------------------------------------------------------------------------
// The headless binary
// ---------------------------------------------------------------------------

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(name)
}

/// A fresh, empty directory for this test's files.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create a scratch directory under target/tmp");
    dir
}

/// Runs `game --headless` with `args` and waits for it, but not for ever: a
/// run that does not finish is stopped and fails the test.
#[track_caller]
fn headless(args: &[&std::ffi::OsStr]) -> Output {
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("m6_io");
    std::fs::create_dir_all(&dir).expect("create a scratch directory under target/tmp");
    let run = RUNS.fetch_add(1, Ordering::Relaxed);
    let out_path = dir.join(format!("{}-{run}.out", std::process::id()));
    let err_path = dir.join(format!("{}-{run}.err", std::process::id()));
    // The child writes into files, so no pipe can fill up while we wait.
    let mut child = Command::new(env!("CARGO_BIN_EXE_game"))
        .arg("--headless")
        .args(args)
        .stdin(Stdio::null())
        .stdout(File::create(&out_path).expect("create a file for the child's output"))
        .stderr(File::create(&err_path).expect("create a file for the child's errors"))
        .spawn()
        .expect("the game binary starts");
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("ask the child for its status") {
            break status;
        }
        if started.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "`game --headless` did not finish within 30 seconds and was stopped; the \
                 shipped replays take well under a second"
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let output = Output {
        status,
        stdout: std::fs::read(&out_path).expect("read the child's output"),
        stderr: std::fs::read(&err_path).expect("read the child's errors"),
    };
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&err_path);
    output
}

#[track_caller]
fn stdout_of(output: &Output) -> String {
    assert!(
        output.status.success(),
        "the headless run failed with {}.\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone())
        .expect("headless output is ASCII")
        .replace("\r\n", "\n")
}

#[test]
fn headless_prints_a_state_hash_and_a_frame_checksum_for_every_step() {
    let _watch = common::watch();
    let level = asset("level1.txt");
    let demo = asset("demo.replay");
    let args = [
        "--level".as_ref(),
        level.as_os_str(),
        "--replay".as_ref(),
        demo.as_os_str(),
    ];
    let text = stdout_of(&headless(&args));

    let r = replay(DEMO_REPLAY);
    let mut g = game(LEVEL_1, r.seed);
    let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
    let mut lines = text.lines();
    for &input in &r.inputs {
        g.step(input);
        g.render(&mut fb, 256);
        let expected = format!(
            "{} {:016x} {:016x}",
            g.tick(),
            g.state_hash(),
            fb.checksum()
        );
        assert_eq!(
            lines.next(),
            Some(expected.as_str()),
            "line {} of the headless output: `tick state_hash frame_checksum`, where the frame \
             is rendered with alpha 256 after the step",
            g.tick()
        );
    }
    assert_eq!(
        lines.next(),
        Some(format!("end phase=won score={DEMO_SCORE} lives=3 ticks={DEMO_TICKS}").as_str()),
        "the last line summarises the run"
    );
    assert_eq!(lines.next(), None, "nothing follows the `end` line");
    assert!(text.ends_with('\n'), "every line ends with a newline");

    let again = stdout_of(&headless(&args));
    assert!(
        again == text,
        "two runs of the same replay must print exactly the same text"
    );
}

#[test]
fn headless_writes_its_lines_to_a_file_on_request() {
    let _watch = common::watch();
    let dir = scratch("m6_out");
    let out = dir.join("lines.txt");
    let level = asset("level1.txt");
    let rough = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/rough.replay");
    let to_stdout = stdout_of(&headless(&[
        "--level".as_ref(),
        level.as_os_str(),
        "--replay".as_ref(),
        rough.as_os_str(),
    ]));
    let output = headless(&[
        "--level".as_ref(),
        level.as_os_str(),
        "--replay".as_ref(),
        rough.as_os_str(),
        "--out".as_ref(),
        out.as_os_str(),
    ]);
    assert_eq!(
        stdout_of(&output),
        "",
        "with --out nothing is printed on standard output"
    );
    let in_file = std::fs::read_to_string(&out)
        .expect("--out FILE creates the file")
        .replace("\r\n", "\n");
    assert!(
        in_file == to_stdout,
        "--out FILE holds exactly what would have been printed"
    );
    assert!(
        to_stdout
            .lines()
            .last()
            .is_some_and(|line| line.starts_with("end phase=lost ")),
        "the rough replay ends with all lives lost"
    );
}

#[test]
fn headless_saves_screenshots_as_ppm() {
    let _watch = common::watch();
    let dir = scratch("m6_shots");
    let level = asset("level1.txt");
    let demo = asset("demo.replay");
    stdout_of(&headless(&[
        "--level".as_ref(),
        level.as_os_str(),
        "--replay".as_ref(),
        demo.as_os_str(),
        "--shots".as_ref(),
        dir.as_os_str(),
        "--shot-at".as_ref(),
        "1,250".as_ref(),
    ]));

    let r = replay(DEMO_REPLAY);
    let mut g = game(LEVEL_1, r.seed);
    let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
    for &input in &r.inputs[..250] {
        g.step(input);
        if g.tick() == 1 || g.tick() == 250 {
            g.render(&mut fb, 256);
            let mut expected = Vec::new();
            fb.write_ppm(&mut expected)
                .expect("writing to a Vec cannot fail");
            let name = format!("frame_{:06}.ppm", g.tick());
            let saved = std::fs::read(dir.join(&name))
                .unwrap_or_else(|e| panic!("--shots DIR --shot-at 1,250 must create {name}: {e}"));
            assert!(
                saved == expected,
                "{name} must be the PPM of the frame rendered with alpha 256 after step {}",
                g.tick()
            );
        }
    }
    let count = std::fs::read_dir(&dir)
        .expect("the shots directory exists")
        .count();
    assert_eq!(count, 2, "only the requested steps are saved");
}

#[test]
fn headless_fails_cleanly_on_bad_input() {
    let _watch = common::watch();
    let dir = scratch("m6_bad");
    let level = asset("level1.txt");
    let missing = dir.join("no-such.replay");
    let broken = dir.join("broken.replay");
    std::fs::write(&broken, "underkiln-replay 1\nseed 1\nticks 2\n1 0\n").expect("write a file");
    let bad_level = dir.join("bad-level.txt");
    std::fs::write(&bad_level, "#.E#\n####\n").expect("write a file");
    let demo = asset("demo.replay");

    // The control: good input works. (Otherwise a binary that always fails
    // would pass this test.)
    let good = headless(&[
        "--level".as_ref(),
        level.as_os_str(),
        "--replay".as_ref(),
        demo.as_os_str(),
    ]);
    assert!(
        stdout_of(&good).ends_with("ticks=1016\n"),
        "a valid level and replay must run to the `end` line"
    );

    let cases: [(&str, Vec<&std::ffi::OsStr>); 5] = [
        (
            "an unknown option",
            vec![
                "--level".as_ref(),
                level.as_os_str(),
                "--replay".as_ref(),
                demo.as_os_str(),
                "--no-such-option".as_ref(),
            ],
        ),
        ("no --replay", vec!["--level".as_ref(), level.as_os_str()]),
        (
            "a replay file that does not exist",
            vec![
                "--level".as_ref(),
                level.as_os_str(),
                "--replay".as_ref(),
                missing.as_os_str(),
            ],
        ),
        (
            "a replay whose runs do not add up",
            vec![
                "--level".as_ref(),
                level.as_os_str(),
                "--replay".as_ref(),
                broken.as_os_str(),
            ],
        ),
        (
            "a level without a player",
            vec![
                "--level".as_ref(),
                bad_level.as_os_str(),
                "--replay".as_ref(),
                demo.as_os_str(),
            ],
        ),
    ];
    for (what, args) in cases {
        let output = headless(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{what}: the exit code must be 2 (no panic, no success)"
        );
        assert!(
            output.stdout.is_empty(),
            "{what}: nothing may be printed on standard output when the run cannot start"
        );
        assert!(
            !output.stderr.is_empty(),
            "{what}: a message on standard error tells the user what is wrong"
        );
    }
}
