//! The practice coach: compiler drills, progress, model solutions and spaced review.
//!
//! Use the Cargo aliases defined in `.cargo/config.toml`. They work from the
//! repository root and from any directory of the root package (`exercises/`,
//! `drills/`, ...), because every path is resolved from the repository root.
//!
//!   cargo drill                     summary of every drill set
//!   cargo drill ch04                work through chapter 4's drills
//!   cargo drill m1                  work through mastery module 1's drills
//!   cargo drill path/to/drill.rs    work on one drill
//!   cargo drill reset ch04          restore chapter 4's drills for another round
//!   cargo progress                  lesson status, next step, reviews due
//!   cargo progress all              the same, plus the project, ecosystem,
//!                                   mastery, capstone and checkpoint packages
//!   cargo progress reviewed ch04    record that you did chapter 4's review
//!   cargo progress redo ch04 l08    archive your lesson and restore its starter
//!   cargo progress solution ch04 l01    show where the model solution is
//!   cargo progress doctor           check the tools, print the fix for each
//!                                   missing one
//!
//! State lives in `.progress/` at the repository root. It is not ignored by
//! Git, so your history is saved with your commits (a codespace that is
//! deleted keeps nothing else). Delete it to start over.
//!
//! Time limits, in seconds, can be changed with environment variables:
//! `COACH_TEST_SECS` (one chapter's tests, default 120), `COACH_DRILL_SECS`
//! (one drill program, default 20) and `COACH_PACKAGE_SECS` (one package in
//! `cargo progress all`, default 900).

use std::collections::BTreeMap;
use std::fs;
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const STATE: &str = ".progress";
/// Days to wait before review 1, 2, 3, 4 and 5. Each wait starts when the
/// previous review (or, for review 1, the chapter) was completed.
const REVIEW_DAYS: [u64; 5] = [1, 3, 7, 21, 60];
/// Reviews explained in full per day. The rest are listed and can wait.
const REVIEWS_PER_DAY: usize = 2;

fn main() -> ExitCode {
    quiet_on_closed_pipe();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let repo = Repo::locate();
    let cwd = std::env::current_dir().unwrap_or_else(|_| repo.root.clone());
    let now = today();
    let result = match args.as_slice() {
        ["drill" | "progress", "help" | "--help" | "-h"] => {
            println!("{USAGE}");
            Ok(())
        }
        ["drill"] => repo.drill_summary(),
        ["drill", "reset"] => Err(
            "`cargo drill reset` needs a drill set, for example `cargo drill reset ch04`. \
             Run `cargo drill` to list the sets."
                .to_string(),
        ),
        ["drill", "reset", key] => repo.drill_reset(key).map(say),
        ["drill", target] => repo.drill_target(target, &cwd),
        ["progress"] => repo.progress(false, now),
        ["progress", "all"] => repo.progress(true, now),
        ["progress", "reviewed", chapter] => repo.reviewed(chapter, false, now).map(say),
        ["progress", "reviewed", chapter, "--early"]
        | ["progress", "reviewed", "--early", chapter] => {
            repo.reviewed(chapter, true, now).map(say)
        }
        ["progress", "redo", chapter, lesson] => repo.redo(chapter, lesson, now).map(say),
        ["progress", "solution", unit] => repo.solution(unit, None).map(say),
        ["progress", "solution", unit, item] => repo.solution(unit, Some(item)).map(say),
        ["progress", "doctor"] => repo.doctor(),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// `cargo progress | head` closes the pipe early. Printing then panics; that
/// is not an error worth a panic message, so the coach just stops.
fn quiet_on_closed_pipe() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let closed = info
            .payload()
            .downcast_ref::<String>()
            .is_some_and(|message| message.starts_with("failed printing to stdout"));
        if closed {
            std::process::exit(0);
        }
        default(info);
    }));
}

fn say(message: String) {
    println!("{message}");
}

const USAGE: &str = "usage:
  cargo drill                       summary of every drill set
  cargo drill ch04                  work through a drill set (ch04, m1, ...)
  cargo drill path/to/drill.rs      work on one drill
  cargo drill reset ch04            restore a drill set to predict it again
  cargo progress                    lesson status, next lesson, reviews due
  cargo progress all                also test the project, ecosystem, checkpoints, mastery, capstones
  cargo progress reviewed ch04      record a finished review (--early: before it is due)
  cargo progress redo ch04 l08      archive your lesson and restore its starter
  cargo progress solution ch04 l01  show where the model solution is
  cargo progress doctor             check your setup and show how to fix what is missing";

// ------------------------------------------------------------- repository

/// The course checkout. Every path the coach reads or writes starts here, so
/// the commands behave the same from any directory.
struct Repo {
    root: PathBuf,
    /// Cargo's target directory; drills are built in `<target>/drills`.
    target: PathBuf,
}

impl Repo {
    /// Finds the repository root: the directory of the root `Cargo.toml`.
    /// `cargo run` passes it in `CARGO_MANIFEST_DIR`; the value recorded when
    /// the coach was compiled is the fallback.
    fn locate() -> Repo {
        let root = std::env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .filter(|dir| dir.join("examples").join("coach.rs").is_file())
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .unwrap_or_else(|| root.join("target"));
        Repo { root, target }
    }

    fn state(&self) -> PathBuf {
        self.root.join(STATE)
    }

    /// Turns a root-relative key such as `drills/ch04/d01_x.rs` into a path.
    fn at(&self, key: &str) -> PathBuf {
        key.split('/')
            .fold(self.root.clone(), |path, part| path.join(part))
    }

    fn chapter_dir(&self, chapter: u32) -> PathBuf {
        self.root
            .join("exercises")
            .join(format!("chapter{chapter:02}"))
    }

    /// Chapters that exist: a `mod.rs` on disk and a test target in Cargo.toml.
    fn chapters(&self) -> Vec<u32> {
        let manifest = fs::read_to_string(self.root.join("Cargo.toml")).unwrap_or_default();
        (1..=99)
            .filter(|n| self.chapter_dir(*n).join("mod.rs").is_file())
            .filter(|n| manifest.contains(&format!("\"chapter{n:02}\"")))
            .collect()
    }

    /// Lesson modules of a chapter in study order: numbered lessons by
    /// number with the capstone last among them (a lesson added after the
    /// capstone still comes before it), then the review lesson, then the
    /// code review. Returns (module name, file).
    fn lesson_files(&self, chapter: u32) -> Vec<(String, PathBuf)> {
        let mut lessons: Vec<(String, PathBuf)> = list_dir(&self.chapter_dir(chapter))
            .into_iter()
            .filter(|path| path.extension().is_some_and(|e| e == "rs"))
            .filter_map(|path| {
                let module = path.file_stem()?.to_str()?.to_string();
                is_lesson_module(&module).then_some((module, path))
            })
            .collect();
        lessons.sort_by_key(|(module, _)| {
            (
                lesson_rank(module),
                module.contains("capstone"),
                module.clone(),
            )
        });
        lessons
    }

    /// The chapter's capstone: the lesson whose file name contains "capstone".
    /// A chapter without one falls back to its last numbered lesson.
    fn capstone(&self, chapter: u32) -> Option<String> {
        let lessons = self.lesson_files(chapter);
        lessons
            .iter()
            .find(|(module, _)| module.contains("capstone"))
            .or_else(|| lessons.iter().rfind(|(module, _)| module.starts_with('l')))
            .map(|(module, _)| module.clone())
    }

    /// Checks a chapter argument such as `ch04`, `chapter04` or `4`.
    fn chapter(&self, text: &str) -> Result<u32, String> {
        let chapters = self.chapters();
        match chapter_number(text) {
            Some(n) if chapters.contains(&n) => Ok(n),
            _ => Err(format!(
                "expected a chapter like ch04, got {text} (chapters here: {})",
                span(&chapters)
            )),
        }
    }
}

fn span(chapters: &[u32]) -> String {
    match (chapters.first(), chapters.last()) {
        (Some(first), Some(last)) => format!("ch{first:02} to ch{last:02}"),
        _ => "none".into(),
    }
}

fn list_dir(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    entries
}

/// `l01_scope`, `r01_review` and `c01_code_review` are lessons; `mod` is not.
fn is_lesson_module(module: &str) -> bool {
    let b = module.as_bytes();
    b.len() > 4
        && matches!(b[0], b'l' | b'r' | b'c')
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3] == b'_'
}

fn lesson_rank(module: &str) -> u8 {
    match module.as_bytes().first() {
        Some(b'l') => 0,
        Some(b'r') => 1,
        Some(b'c') => 2,
        _ => 3,
    }
}

/// What the learner typed for a lesson, reduced to a module name or its
/// number: `l08`, `L08_`, `l08_capstone` and `l08_capstone.rs` are all fine.
fn lesson_name(typed: &str) -> String {
    let lower = typed.to_ascii_lowercase();
    let name = lower.rsplit(['/', '\\']).next().unwrap_or(&lower);
    name.strip_suffix(".rs")
        .unwrap_or(name)
        .trim_end_matches('_')
        .to_string()
}

/// `l01_scope` -> `l01`.
fn short(module: &str) -> &str {
    module.split('_').next().unwrap_or(module)
}

/// `ch04`, `chapter04`, `ch4` and `4` all mean chapter 4.
fn chapter_number(text: &str) -> Option<u32> {
    let lower = text.to_ascii_lowercase();
    let digits = lower
        .strip_prefix("chapter")
        .or_else(|| lower.strip_prefix("ch"))
        .unwrap_or(&lower);
    if digits.is_empty() || digits.len() > 2 {
        return None;
    }
    digits.parse().ok().filter(|n| *n >= 1)
}

/// Sort key that puts `ch04` before `ch10` and `m2` before `m10`.
fn natural(name: &str) -> (String, u64, String) {
    let split = name
        .find(|c: char| c.is_ascii_digit())
        .unwrap_or(name.len());
    let (prefix, rest) = name.split_at(split);
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let number = rest[..end].parse().unwrap_or(0);
    (prefix.to_string(), number, rest[end..].to_string())
}

fn seconds(variable: &str, default: u64) -> Duration {
    let secs = std::env::var(variable)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|secs| *secs > 0)
        .unwrap_or(default);
    Duration::from_secs(secs)
}

/// Whether compiler output can be passed on in colour: the coach prints to a
/// terminal that understands colour codes, and `NO_COLOR` is not set. The
/// classic Windows console shows the codes as text, so on Windows colour is
/// used only in Windows Terminal (`WT_SESSION`) and in shells that set `TERM`.
fn colors_are_shown() -> bool {
    let set = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
    std::io::stdout().is_terminal()
        && !set("NO_COLOR")
        && (cfg!(not(windows)) || set("WT_SESSION") || set("TERM"))
}

fn flush() {
    let _ = std::io::stdout().flush();
}

// ------------------------------------------------------------ state files

/// Reads a state file. A missing file is an empty history; any other problem
/// stops the coach, so a history that cannot be read is never overwritten.
fn read_state(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
    }
}

/// Writes through a temporary file, so an interrupted run cannot leave half a
/// history behind.
fn write_state(path: &Path, text: &str) -> Result<(), String> {
    let explain = |e: std::io::Error| format!("cannot write {}: {e}", path.display());
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(explain)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, text).map_err(explain)?;
    fs::rename(&temporary, path).map_err(explain)
}

fn bad_line(path: &Path, number: usize) -> String {
    format!(
        "{} line {number} is not in the coach's format. Correct or delete that line, then run \
         the command again. Nothing was changed.",
        path.display()
    )
}

// ------------------------------------------------- running child programs

enum Ended {
    Exited(std::process::ExitStatus),
    TimedOut,
}

struct Finished {
    ended: Ended,
    stdout: String,
    stderr: String,
}

/// Collects one pipe of a child on a helper thread.
struct Drain {
    bytes: Arc<Mutex<Vec<u8>>>,
    done: mpsc::Receiver<()>,
}

impl Drain {
    fn start(pipe: Option<impl Read + Send + 'static>) -> Drain {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let (finished, done) = mpsc::channel();
        if let Some(mut pipe) = pipe {
            let bytes = Arc::clone(&bytes);
            std::thread::spawn(move || {
                let mut chunk = [0u8; 8192];
                while let Ok(n) = pipe.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    if let Ok(mut all) = bytes.lock() {
                        all.extend_from_slice(&chunk[..n]);
                    }
                }
                let _ = finished.send(());
            });
        }
        Drain { bytes, done }
    }

    /// Waits a short time for the pipe to close, then returns what arrived.
    /// A process that outlived the child may still hold the pipe open; the
    /// coach must not wait for it.
    fn finish(self) -> String {
        let _ = self.done.recv_timeout(Duration::from_secs(2));
        let bytes = self.bytes.lock().map(|all| all.clone()).unwrap_or_default();
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

/// Runs a command with captured output and a wall-clock limit. When the
/// limit passes, the child and the processes it started are killed.
fn run_limited(command: &mut Command, limit: Duration) -> Result<Finished, String> {
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            // NotFound covers both forms of a missing program: "os error 2"
            // on Unix and the "program not found" error of Windows.
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("{program} is not installed, or not on PATH")
            } else {
                format!("could not start {program}: {e}")
            }
        })?;
    let stdout = Drain::start(child.stdout.take());
    let stderr = Drain::start(child.stderr.take());
    let deadline = Instant::now() + limit;
    let ended = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ended::Exited(status),
            Ok(None) if Instant::now() >= deadline => {
                kill_tree(&mut child);
                break Ended::TimedOut;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => {
                kill_tree(&mut child);
                return Err(format!("could not wait for {program}: {e}"));
            }
        }
    };
    Ok(Finished {
        ended,
        stdout: stdout.finish(),
        stderr: stderr.finish(),
    })
}

/// Kills a child and, as far as the platform's own tools allow, the processes
/// it started (for example a test program started by Cargo).
fn kill_tree(child: &mut Child) {
    let pid = child.id().to_string();
    let mut helper = if cfg!(windows) {
        let mut taskkill = Command::new("taskkill");
        taskkill.args(["/T", "/F", "/PID", &pid]);
        taskkill
    } else {
        let mut pkill = Command::new("pkill");
        pkill.args(["-KILL", "-P", &pid]);
        pkill
    };
    let _ = helper
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    let _ = child.wait();
}

// ---------------------------------------------------------------- drills

/// What actually happened when a drill was compiled and run.
#[derive(Clone, PartialEq, Debug)]
enum Outcome {
    Ok,
    Panic,
    /// Error codes in order of appearance. Errors without a code are not listed.
    Errors(Vec<String>),
    /// The program was still running after this many seconds.
    Hang(u64),
}

impl Outcome {
    fn label(&self) -> String {
        match self {
            Outcome::Ok => "ok".into(),
            Outcome::Panic => "panic".into(),
            Outcome::Errors(codes) if codes.is_empty() => "error (no error code)".into(),
            Outcome::Errors(codes) => {
                let mut unique: Vec<&str> = Vec::new();
                for code in codes {
                    if !unique.contains(&code.as_str()) {
                        unique.push(code);
                    }
                }
                unique.join(" ")
            }
            Outcome::Hang(secs) => format!("still running after {secs} s"),
        }
    }
}

struct Run {
    outcome: Outcome,
    /// Compiler diagnostics or program output, ready to print.
    output: String,
}

#[derive(PartialEq, Debug)]
enum Verdict {
    Right,
    Partial,
    Wrong,
}

/// What an `EXPECT:` line says, after tolerant reading.
#[derive(PartialEq, Debug)]
enum Prediction {
    Ok,
    Panic,
    /// A compile error, with the predicted codes (possibly none).
    Error(Vec<String>),
}

/// Reads a prediction. Punctuation, capitals and natural wording are
/// accepted: `E0382.`, `error[E0382]`, `compile error`, `panics`, `Compiles`.
/// Returns `None` when the line cannot be understood.
fn read_prediction(expect: &str) -> Option<Prediction> {
    let lower = expect.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let is_code = |word: &str| {
        word.len() == 5 && word.starts_with('e') && word[1..].bytes().all(|b| b.is_ascii_digit())
    };
    let mut codes: Vec<String> = words
        .iter()
        .filter(|word| is_code(word))
        .map(|word| word.to_ascii_uppercase())
        .collect();
    codes.sort();
    codes.dedup();
    const FILLER: [&str; 9] = [
        "compile", "compiler", "runtime", "run", "time", "and", "a", "an", "it",
    ];
    let rest: Vec<&str> = words
        .into_iter()
        .filter(|word| !is_code(word) && !FILLER.contains(word))
        .collect();
    match rest.as_slice() {
        [] if !codes.is_empty() => Some(Prediction::Error(codes)),
        ["error" | "errors" | "fails"] => Some(Prediction::Error(codes)),
        ["ok" | "compiles"] if codes.is_empty() => Some(Prediction::Ok),
        ["panic" | "panics"] if codes.is_empty() => Some(Prediction::Panic),
        _ => None,
    }
}

/// Compares an `EXPECT:` line with what happened.
fn judge(expect: &str, actual: &Outcome) -> Verdict {
    match (read_prediction(expect), actual) {
        (Some(Prediction::Ok), Outcome::Ok) => Verdict::Right,
        (Some(Prediction::Panic), Outcome::Panic) => Verdict::Right,
        (Some(Prediction::Error(predicted)), Outcome::Errors(codes)) => {
            let mut actual_codes = codes.clone();
            actual_codes.sort();
            actual_codes.dedup();
            if predicted == actual_codes {
                Verdict::Right
            } else if predicted.is_empty() || predicted.iter().any(|c| actual_codes.contains(c)) {
                // "error" for an error with a code, or some of the codes.
                Verdict::Partial
            } else {
                Verdict::Wrong
            }
        }
        _ => Verdict::Wrong,
    }
}

fn expect_line(source: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let rest = line
            .trim_start_matches(['/', '!', ' '])
            .strip_prefix("EXPECT:")?;
        let value = rest.trim();
        (!value.is_empty() && value != "?").then(|| value.to_string())
    })
}

fn keep_lines(source: &str) -> Vec<String> {
    source
        .lines()
        .filter(|line| line.trim_end().ends_with("// KEEP"))
        .map(|line| line.trim().to_string())
        .collect()
}

/// The program without its `EXPECT:` line, to tell an edited drill from an
/// untouched one whatever the learner predicted.
fn without_expect(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter(|line| !line.contains("EXPECT:"))
        .collect()
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Deletes a drill's executable when dropped, together with any file the
/// linker wrote next to it under the same name (a `.pdb` on Windows).
struct BuiltDrill(PathBuf);

impl Drop for BuiltDrill {
    fn drop(&mut self) {
        let Some(dir) = self.0.parent() else {
            return;
        };
        let stem = self.0.file_stem();
        for path in list_dir(dir) {
            if path.file_stem() != stem {
                continue;
            }
            // Windows can keep a program file locked for a moment after it
            // exits (a virus scanner reads it), so try for up to two seconds.
            for _ in 0..100 {
                if fs::remove_file(&path).is_ok() || !path.exists() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

/// A recorded first prediction for one drill.
#[derive(Debug)]
struct Record {
    expect: String,
    actual: String,
    verdict: String,
}

#[derive(PartialEq, Debug)]
enum DrillState {
    NeedsPrediction,
    NeedsFix,
    Done,
}

impl Repo {
    /// Drill sets that exist, such as `ch04` or `m1`.
    fn drill_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = list_dir(&self.root.join("drills"))
            .into_iter()
            .filter(|dir| dir.is_dir())
            .filter_map(|dir| Some(dir.file_name()?.to_str()?.to_string()))
            .filter(|key| !self.drill_files(key).is_empty())
            .collect();
        keys.sort_by_key(|key| natural(key));
        keys
    }

    /// Accepts `ch04`, `chapter04`, `ch4`, `m1`, ... and returns the directory
    /// name under `drills/`.
    fn drill_key(&self, text: &str) -> Result<String, String> {
        let lower = text.to_ascii_lowercase();
        let wanted = match chapter_number(&lower) {
            Some(n) if lower.starts_with("ch") => format!("ch{n:02}"),
            _ => lower,
        };
        let keys = self.drill_keys();
        if keys.contains(&wanted) {
            Ok(wanted)
        } else {
            Err(format!(
                "no drills named {text}. Drill sets here: {}",
                keys.join(" ")
            ))
        }
    }

    /// Root-relative keys of a drill set's programs, in order.
    fn drill_files(&self, key: &str) -> Vec<String> {
        list_dir(&self.root.join("drills").join(key))
            .into_iter()
            .filter(|path| path.extension().is_some_and(|e| e == "rs"))
            .filter_map(|path| Some(format!("drills/{key}/{}", path.file_name()?.to_str()?)))
            .collect()
    }

    /// Turns a drill path typed by the learner into its root-relative key.
    /// The path may be relative to the current directory or to the
    /// repository root, absolute, or spelled with `./`; all give one key, so
    /// a drill has one record and one saved original.
    fn resolve_drill(&self, target: &str, cwd: &Path) -> Result<String, String> {
        let typed = Path::new(target);
        let file = [cwd.join(typed), self.root.join(typed)]
            .into_iter()
            .find(|path| path.is_file())
            .ok_or_else(|| format!("no drill file at {target}"))?;
        let explain = |e: std::io::Error| format!("cannot resolve {target}: {e}");
        let real = fs::canonicalize(&file).map_err(explain)?;
        let root = fs::canonicalize(&self.root).map_err(explain)?;
        let inside = real.strip_prefix(&root).ok().and_then(|relative| {
            let parts: Option<Vec<&str>> = relative
                .components()
                .map(|part| part.as_os_str().to_str())
                .collect();
            Some(parts?.join("/"))
        });
        match inside {
            Some(key) if key.starts_with("drills/") && key.ends_with(".rs") => Ok(key),
            _ => Err(format!(
                "{target} is not a drill: drills are the .rs files under {}",
                self.root.join("drills").display()
            )),
        }
    }

    /// Compiles and runs one program. `key` is a root-relative path.
    /// The executable goes to `<target>/drills` and is removed afterwards.
    /// `Err` means the tools failed (not the program), so no verdict is due.
    fn compile_and_run(&self, key: &str) -> Result<Run, String> {
        let out_dir = self.target.join("drills");
        fs::create_dir_all(&out_dir)
            .map_err(|e| format!("cannot create {}: {e}", out_dir.display()))?;
        let name: String = key
            .trim_end_matches(".rs")
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let exe = BuiltDrill(out_dir.join(format!(
            "{name}_{}{}",
            std::process::id(),
            std::env::consts::EXE_SUFFIX
        )));
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
        let color = if colors_are_shown() {
            "always"
        } else {
            "never"
        };
        let mut compile = Command::new(rustc);
        compile
            .current_dir(&self.root)
            .args(["--edition", "2024", "--crate-type", "bin"])
            .args(["-C", "debuginfo=0", "--color", color])
            .args(["-A", "dead_code", "-A", "unused_variables"])
            .args(["-A", "unused_assignments", "-A", "unused_mut"])
            .args(["-A", "unused_imports"])
            .arg("-o")
            .arg(&exe.0)
            .arg(key);
        let compiled = run_limited(&mut compile, Duration::from_secs(300))?;
        let diagnostics = compiled.stderr;
        let built = matches!(&compiled.ended, Ended::Exited(status) if status.success());
        if !built {
            let plain = strip_ansi(&diagnostics);
            let codes: Vec<String> = plain
                .lines()
                .filter_map(|line| line.strip_prefix("error[")?.split(']').next())
                .map(str::to_string)
                .collect();
            let points_into_source = plain
                .lines()
                .any(|line| line.trim_start().starts_with("--> "));
            if codes.is_empty() && !points_into_source {
                return Err(format!(
                    "rustc could not build {key}. This is a problem with the tools or the \
                     disk, not with your prediction, so nothing was recorded:\n{diagnostics}"
                ));
            }
            return Ok(Run {
                outcome: Outcome::Errors(codes),
                output: diagnostics,
            });
        }
        let limit = seconds("COACH_DRILL_SECS", 20);
        let mut program = Command::new(&exe.0);
        program.current_dir(&self.root);
        let ran = run_limited(&mut program, limit)?;
        let mut output = ran.stdout;
        output.push_str(&ran.stderr);
        let outcome = match ran.ended {
            Ended::Exited(status) if status.success() => Outcome::Ok,
            Ended::Exited(_) => Outcome::Panic,
            Ended::TimedOut => Outcome::Hang(limit.as_secs()),
        };
        Ok(Run { outcome, output })
    }

    fn records_path(&self) -> PathBuf {
        self.state().join("drills.tsv")
    }

    fn load_records(&self) -> Result<BTreeMap<String, Record>, String> {
        let path = self.records_path();
        let mut records = BTreeMap::new();
        for (index, line) in read_state(&path)?.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            let [key, expect, actual, verdict] = fields.as_slice() else {
                return Err(bad_line(&path, index + 1));
            };
            records.insert(
                key.to_string(),
                Record {
                    expect: expect.to_string(),
                    actual: actual.to_string(),
                    verdict: verdict.to_string(),
                },
            );
        }
        Ok(records)
    }

    fn save_records(&self, records: &BTreeMap<String, Record>) -> Result<(), String> {
        let text: String = records
            .iter()
            .map(|(k, r)| format!("{k}\t{}\t{}\t{}\n", r.expect, r.actual, r.verdict))
            .collect();
        write_state(&self.records_path(), &text)
    }

    /// Where the untouched copy of a drill is kept.
    fn original_key(key: &str) -> String {
        format!("{STATE}/originals/{key}")
    }

    /// Advances one drill as far as it can go and prints what the learner
    /// should do next. `verbose` prints compiler output for unfinished drills.
    fn step_drill(&self, key: &str, verbose: bool) -> Result<DrillState, String> {
        let path = self.at(key);
        let source = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(expect) = expect_line(&source) else {
            println!("  ·  {key}: write your prediction on the EXPECT: line first");
            return Ok(DrillState::NeedsPrediction);
        };
        let mut records = self.load_records()?;
        let original_key = Self::original_key(key);
        let original_path = self.at(&original_key);
        let just_judged = !records.contains_key(key);
        let mut first_run = None;
        if just_judged {
            if read_prediction(&expect).is_none() {
                println!(
                    "  ·  {key}: the coach cannot read `EXPECT: {expect}`. Write ok, panic, \
                     error, or the error code(s), for example `EXPECT: E0308`."
                );
                return Ok(DrillState::NeedsPrediction);
            }
            // The prediction is about the untouched program. A saved original
            // is never overwritten: if one exists and the file has been
            // edited since, the original is what gets judged.
            let saved = fs::read_to_string(&original_path).ok();
            let edited = saved
                .as_deref()
                .is_some_and(|original| without_expect(original) != without_expect(&source));
            if saved.is_none() {
                write_new(&original_path, &source)?;
            }
            let run = self.compile_and_run(if edited { &original_key } else { key })?;
            let verdict = judge(&expect, &run.outcome);
            let word = match verdict {
                Verdict::Right => "right",
                Verdict::Partial => "partly right",
                Verdict::Wrong => "wrong",
            };
            println!(
                "\n━━ {key}\n   You predicted: {expect}\n   Actual:        {}\n   Verdict:       {word}",
                run.outcome.label()
            );
            if edited {
                println!("   (Judged against the saved, unedited drill.)");
            }
            print!("{}", run.output);
            if verdict != Verdict::Right {
                println!(
                    "   Before fixing: explain in one sentence why the compiler or program behaved this way."
                );
            }
            records.insert(
                key.to_string(),
                Record {
                    expect,
                    actual: run.outcome.label(),
                    verdict: word.into(),
                },
            );
            self.save_records(&records)?;
            if !edited {
                first_run = Some(run);
            }
        }
        let original = fs::read_to_string(&original_path).unwrap_or_else(|_| source.clone());
        let missing: Vec<String> = keep_lines(&original)
            .into_iter()
            .filter(|line| !source.lines().any(|l| l.trim() == line))
            .collect();
        if !missing.is_empty() {
            println!("  ✗  {key}: restore these lines; they must stay in the program:");
            for line in missing {
                println!("       {line}");
            }
            return Ok(DrillState::NeedsFix);
        }
        let run = match first_run {
            Some(run) => run,
            None => self.compile_and_run(key)?,
        };
        if run.outcome == Outcome::Ok {
            println!("  ✓  {key}");
            return Ok(DrillState::Done);
        }
        println!(
            "  ✗  {key}: {} — fix it so it compiles and runs cleanly",
            run.outcome.label()
        );
        if verbose && !just_judged {
            print!("{}", run.output);
        }
        Ok(DrillState::NeedsFix)
    }

    fn drill_target(&self, target: &str, cwd: &Path) -> Result<(), String> {
        if target.ends_with(".rs") {
            let key = self.resolve_drill(target, cwd)?;
            self.step_drill(&key, true)?;
            return Ok(());
        }
        // `drills/ch04` and `drills/ch04/` (as tab completion writes them)
        // mean the set `ch04`.
        let name = target
            .trim_end_matches(['/', '\\'])
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(target);
        let key = self.drill_key(name)?;
        println!("{key} drills (stops at the first unfinished one)");
        for file in self.drill_files(&key) {
            if self.step_drill(&file, true)? != DrillState::Done {
                println!(
                    "\nEdit {} and run `cargo drill {key}` again.",
                    self.at(&file).display()
                );
                return Ok(());
            }
        }
        println!("\nAll {key} drills done. Retry them later with `cargo drill reset {key}`.");
        Ok(())
    }

    fn drill_summary(&self) -> Result<(), String> {
        let records = self.load_records()?;
        println!("Drills: predicted / total, right on the first try");
        for key in self.drill_keys() {
            let files = self.drill_files(&key);
            let mine: Vec<&Record> = files.iter().filter_map(|f| records.get(f)).collect();
            let right = mine.iter().filter(|r| r.verdict == "right").count();
            println!(
                "  {key:<5} {:>2}/{:<2}  {right} right",
                mine.len(),
                files.len()
            );
        }
        println!("Run `cargo drill ch04` (or another name from the list) to continue a set.");
        Ok(())
    }

    fn drill_reset(&self, target: &str) -> Result<String, String> {
        let key = self.drill_key(target)?;
        let mut records = self.load_records()?;
        let mut restored = 0;
        for file in self.drill_files(&key) {
            if let Ok(original) = fs::read_to_string(self.at(&Self::original_key(&file))) {
                let blank: String = original
                    .lines()
                    .map(|line| match line.find("EXPECT:") {
                        Some(i) => format!("{}EXPECT: ?\n", &line[..i]),
                        None => format!("{line}\n"),
                    })
                    .collect();
                let path = self.at(&file);
                fs::write(&path, blank)
                    .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
                restored += 1;
            }
            records.remove(&file);
        }
        self.save_records(&records)?;
        Ok(format!(
            "Restored {restored} drills in {key}. Predict them again without looking at your notes."
        ))
    }

    /// Drills of a chapter whose first prediction was not right.
    fn missed_drills(&self, chapter: u32) -> Result<Vec<String>, String> {
        let prefix = format!("drills/ch{chapter:02}/");
        Ok(self
            .load_records()?
            .iter()
            .filter(|(key, record)| key.starts_with(&prefix) && record.verdict != "right")
            .map(|(key, _)| key[prefix.len()..].to_string())
            .collect())
    }
}

/// Writes a file that must not exist yet. An existing file is left alone.
fn write_new(path: &Path, text: &str) -> Result<(), String> {
    let explain = |e: std::io::Error| format!("cannot write {}: {e}", path.display());
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(explain)?;
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => file.write_all(text.as_bytes()).map_err(explain),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(explain(e)),
    }
}

// ------------------------------------------------------- test-run parsing

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mark {
    Passed,
    Failed,
    Ignored,
}

/// What one test program printed.
#[derive(Default, Debug)]
struct TestOutput {
    /// N from the `running N tests` line.
    announced: Option<usize>,
    /// Every `test NAME ... RESULT` line, in order.
    results: Vec<(String, Mark)>,
    /// Whether the final `test result:` line was printed. A program that
    /// aborts (stack overflow, segfault, `abort`) never prints it.
    summary: bool,
}

/// Splits captured `cargo test` output into one block per test program.
///
/// After its result lines a test program prints a `failures:` section with
/// the captured output of every failed test. That output is the learner's
/// and may itself look like test output (a test that runs another test
/// program and prints what it said), so nothing in that section is read
/// except the program's own `test result:` line.
fn parse_test_output(stdout: &str) -> Vec<TestOutput> {
    #[derive(PartialEq)]
    enum Part {
        /// Before the first program, or after a program's summary.
        Between,
        /// The `test NAME ... RESULT` lines.
        Results,
        /// The `failures:` section, up to the summary.
        Failures,
    }
    let mut blocks: Vec<TestOutput> = Vec::new();
    let mut part = Part::Between;
    for line in stdout.lines() {
        if part == Part::Failures {
            if let Some(block) = blocks.last_mut()
                && is_summary_of(line, block)
            {
                block.summary = true;
                part = Part::Between;
            }
            continue;
        }
        if let Some(count) = line
            .strip_prefix("running ")
            .and_then(|rest| rest.strip_suffix(" tests").or(rest.strip_suffix(" test")))
            .and_then(|count| count.parse().ok())
        {
            blocks.push(TestOutput {
                announced: Some(count),
                ..TestOutput::default()
            });
            part = Part::Results;
            continue;
        }
        let Some(block) = blocks.last_mut().filter(|_| part == Part::Results) else {
            continue;
        };
        if line == "failures:" {
            part = Part::Failures;
        } else if line.starts_with("test result:") {
            block.summary = true;
            part = Part::Between;
        } else if let Some((name, result)) = line
            .strip_prefix("test ")
            .and_then(|rest| rest.rsplit_once(" ... "))
        {
            let mark = match result.trim() {
                "ok" => Mark::Passed,
                r if r.starts_with("FAILED") => Mark::Failed,
                r if r.starts_with("ignored") => Mark::Ignored,
                _ => continue,
            };
            let name = name.strip_suffix(" - should panic").unwrap_or(name);
            block.results.push((name.to_string(), mark));
        }
    }
    blocks
}

/// Whether `line` is the `test result:` line that belongs to `block`: its
/// passed and failed counts are the ones the block's result lines add up to.
/// A `test result:` line without readable counts is accepted.
fn is_summary_of(line: &str, block: &TestOutput) -> bool {
    let Some(rest) = line.strip_prefix("test result:") else {
        return false;
    };
    let count = |word: &str| -> Option<usize> {
        rest.split(';')
            .find_map(|part| part.trim().strip_suffix(word))
            .and_then(|number| number.split_whitespace().next_back()?.parse().ok())
    };
    let have = |mark: Mark| block.results.iter().filter(|(_, m)| *m == mark).count();
    match (count(" passed"), count(" failed")) {
        (Some(passed), Some(failed)) => {
            passed == have(Mark::Passed) && failed == have(Mark::Failed)
        }
        _ => true,
    }
}

/// Decides whether a test program's results can be trusted. Returns the
/// reason when they cannot: the program crashed, was cut short, or exited in
/// a way a normal test run never does. `code` is the exit code (a test
/// program exits with 0, or 101 when tests failed) and `shown` describes the
/// exit status for the learner.
fn crash_reason(output: &TestOutput, code: Option<i32>, shown: &str) -> Option<String> {
    let normal_exit = matches!(code, Some(0 | 101));
    if !normal_exit {
        return Some(shown.to_string());
    }
    if !output.summary {
        return Some("the test program stopped before printing its `test result:` line".into());
    }
    if output.announced != Some(output.results.len()) {
        return Some(format!(
            "{} tests were announced but {} reported a result",
            output.announced.unwrap_or(0),
            output.results.len()
        ));
    }
    None
}

/// Names from `--list --format terse` output that have no result line.
fn unfinished(listing: &str, output: &TestOutput) -> Vec<String> {
    listing
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .filter(|name| !output.results.iter().any(|(done, _)| done == name))
        .map(str::to_string)
        .collect()
}

/// The test that was running when a one-at-a-time run (`--test-threads 1`)
/// stopped. In that mode a test program prints `test NAME ... ` before it
/// runs the test and the result after it, so a crash leaves the name of the
/// guilty test at the end of the output, without a result.
fn interrupted_test(stdout: &str) -> Option<String> {
    let last = stdout.lines().rfind(|line| !line.trim().is_empty())?;
    let name = last
        .strip_prefix("test ")?
        .trim_end()
        .strip_suffix(" ...")?;
    let name = name.strip_suffix(" - should panic").unwrap_or(name);
    Some(name.to_string())
}

/// Finds `"key":"value"` in one line of JSON and returns the decoded value.
/// Enough for the flat string fields Cargo prints; not a general JSON parser.
fn json_string(line: &str, key: &str) -> Option<String> {
    let start = line.find(&format!("\"{key}\":\""))? + key.len() + 4;
    let mut value = String::new();
    let mut chars = line[start..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(value),
            '\\' => match chars.next()? {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    value.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                other => value.push(other),
            },
            other => value.push(other),
        }
    }
    None
}

/// Reads `cargo build --message-format=json` output and returns the
/// test executable of each chapter that compiled.
fn chapter_executables(json: &str) -> BTreeMap<u32, PathBuf> {
    json.lines()
        .filter(|line| line.contains("\"reason\":\"compiler-artifact\""))
        .filter_map(|line| {
            let executable = json_string(line, "executable")?;
            let target = &line[line.find("\"target\":{")?..];
            let name = json_string(target, "name")?;
            let chapter = name.strip_prefix("chapter")?.parse().ok()?;
            Some((chapter, PathBuf::from(executable)))
        })
        .collect()
}

// -------------------------------------------------------------- progress

#[derive(Default)]
struct Lesson {
    passed: usize,
    failed: usize,
}

/// The result of running one chapter's tests.
enum ChapterRun {
    /// Trusted results: (lesson module, counts) in study order.
    Finished(Vec<(String, Lesson)>),
    NotCompiled,
    Crashed {
        reason: String,
        /// The test that was running when the program died, when a second,
        /// one-at-a-time run could tell.
        culprit: Option<String>,
        unfinished: Vec<String>,
    },
    TimedOut {
        secs: u64,
        unfinished: Vec<String>,
    },
}

/// One chapter's review history.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Review {
    /// Day the chapter was first seen complete.
    completed: u64,
    /// Reviews recorded so far.
    done: usize,
    /// Day of the last review, or of completion when none was done yet.
    last: u64,
}

impl Review {
    /// The day the next review is due, or `None` when the schedule is over.
    fn due_day(&self) -> Option<u64> {
        REVIEW_DAYS
            .get(self.done)
            .map(|days| self.last.saturating_add(*days))
    }
}

fn today() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0)
}

/// Days since 1970-01-01 to YYYY-MM-DD (Howard Hinnant's civil_from_days).
fn date(days: u64) -> String {
    let z = i64::try_from(days).unwrap_or(i64::MAX / 2).min(100_000_000) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// A short name for what review number `n` (0-based) is about.
fn review_topic(n: usize) -> &'static str {
    match n {
        0 => "what you missed the first time",
        1 => "mastery questions",
        2 => "the capstone again",
        3 => "mixed: all drills and two questions",
        _ => "mixed: the review lesson and a summary",
    }
}

impl Repo {
    fn reviews_path(&self) -> PathBuf {
        self.state().join("chapters.tsv")
    }

    fn load_reviews(&self) -> Result<BTreeMap<u32, Review>, String> {
        let path = self.reviews_path();
        let mut reviews = BTreeMap::new();
        for (index, line) in read_state(&path)?.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            let parsed = match fields.as_slice() {
                [chapter, completed, done, last] => chapter.parse().ok().zip(
                    completed
                        .parse()
                        .ok()
                        .zip(done.parse().ok())
                        .zip(last.parse().ok()),
                ),
                _ => None,
            };
            let Some((chapter, ((completed, done), last))) = parsed else {
                return Err(bad_line(&path, index + 1));
            };
            reviews.insert(
                chapter,
                Review {
                    completed,
                    done,
                    last,
                },
            );
        }
        Ok(reviews)
    }

    fn save_reviews(&self, reviews: &BTreeMap<u32, Review>) -> Result<(), String> {
        let text: String = reviews
            .iter()
            .map(|(c, r)| format!("{c}\t{}\t{}\t{}\n", r.completed, r.done, r.last))
            .collect();
        write_state(&self.reviews_path(), &text)
    }

    fn starter_path(&self, file: &Path) -> PathBuf {
        let relative = file.strip_prefix(&self.root).unwrap_or(file);
        self.state().join("starters").join(relative)
    }

    /// Compiles every chapter's tests in one Cargo run and returns the test
    /// executable of each chapter that compiled. `cargo build --test NAME`
    /// builds the same program that `cargo test --test NAME` runs, and
    /// `--keep-going` builds the other chapters when one does not compile.
    fn build_chapters(&self, chapters: &[u32]) -> Result<BTreeMap<u32, PathBuf>, String> {
        if chapters.is_empty() {
            return Ok(BTreeMap::new());
        }
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let mut build = Command::new(cargo);
        build
            .current_dir(&self.root)
            .args(["build", "--keep-going"])
            .arg("--message-format=json");
        for chapter in chapters {
            build.args(["--test", &format!("chapter{chapter:02}")]);
        }
        let output = build
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map_err(|e| format!("could not run cargo: {e}"))?;
        Ok(chapter_executables(&String::from_utf8_lossy(
            &output.stdout,
        )))
    }

    /// Runs one chapter's test program under a time limit and groups the
    /// results by lesson. Results are trusted only when the program printed
    /// its `test result:` line and exited the way a test run does.
    fn run_chapter(&self, chapter: u32, exe: &Path, limit: Duration) -> Result<ChapterRun, String> {
        let test = |arguments: &[&str], limit| {
            let mut program = Command::new(exe);
            program
                .current_dir(&self.root)
                .env("CARGO_MANIFEST_DIR", &self.root)
                .args(arguments);
            run_limited(&mut program, limit)
        };
        let finished = test(&["--format", "pretty", "--color", "never"], limit)?;
        let output = parse_test_output(&finished.stdout)
            .into_iter()
            .next()
            .unwrap_or_default();
        let problem = match &finished.ended {
            Ended::TimedOut => None,
            Ended::Exited(status) => {
                match crash_reason(&output, status.code(), &status.to_string()) {
                    Some(reason) => Some(reason),
                    None => return Ok(ChapterRun::Finished(self.group(chapter, &output))),
                }
            }
        };
        // Something went wrong: find out which tests never reported.
        let listing = test(&["--list", "--format", "terse"], Duration::from_secs(60))
            .map(|listed| listed.stdout)
            .unwrap_or_default();
        let unfinished = unfinished(&listing, &output);
        // Tests run in parallel, so after a crash most of them are
        // unfinished. Running them again one at a time shows which one
        // takes the program down.
        let culprit = match &problem {
            Some(_) => test(
                &[
                    "--test-threads",
                    "1",
                    "--format",
                    "pretty",
                    "--color",
                    "never",
                ],
                limit,
            )
            .ok()
            .filter(|serial| matches!(serial.ended, Ended::Exited(status) if !status.success()))
            .and_then(|serial| interrupted_test(&serial.stdout)),
            None => None,
        };
        Ok(match problem {
            Some(reason) => ChapterRun::Crashed {
                reason,
                culprit,
                unfinished,
            },
            None => ChapterRun::TimedOut {
                secs: limit.as_secs(),
                unfinished,
            },
        })
    }

    /// Counts results per lesson module. Lessons without tests are left out.
    fn group(&self, chapter: u32, output: &TestOutput) -> Vec<(String, Lesson)> {
        self.lesson_files(chapter)
            .into_iter()
            .map(|(module, _)| {
                let prefix = format!("{module}::");
                let mut lesson = Lesson::default();
                for (name, mark) in &output.results {
                    if name.starts_with(&prefix) {
                        match mark {
                            Mark::Passed => lesson.passed += 1,
                            Mark::Failed => lesson.failed += 1,
                            Mark::Ignored => {}
                        }
                    }
                }
                (module, lesson)
            })
            .filter(|(_, lesson)| lesson.passed + lesson.failed > 0)
            .collect()
    }

    /// Keeps an untouched copy of an unfinished lesson so `redo` can restore
    /// it. The first sighting of an unfinished lesson is taken as its
    /// starter, so run `cargo progress` before editing a lesson.
    fn snapshot_starter(&self, file: &Path) -> Result<(), String> {
        let starter = self.starter_path(file);
        if starter.exists() {
            return Ok(());
        }
        let text = fs::read(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
        let explain = |e: std::io::Error| format!("cannot write {}: {e}", starter.display());
        if let Some(dir) = starter.parent() {
            fs::create_dir_all(dir).map_err(explain)?;
        }
        fs::write(&starter, text).map_err(explain)
    }

    fn progress(&self, all: bool, now: u64) -> Result<(), String> {
        let mut reviews = self.load_reviews()?;
        let chapters = self.chapters();
        println!("Lessons: ✓ all tests pass   ◐ some tests pass   · no test passes yet\n");
        print!("Building the lesson tests (the first run takes longer) ... ");
        flush();
        let built = self.build_chapters(&chapters)?;
        println!("done");
        let limit = seconds("COACH_TEST_SECS", 120);
        let mut next: Option<String> = None;
        let (mut lessons_done, mut lessons_total) = (0, 0);
        for &chapter in &chapters {
            // The label comes first, so a slow or stuck chapter is visible.
            print!("ch{chapter:02} ");
            flush();
            let rerun = format!("cargo test --test chapter{chapter:02}");
            let files = self.lesson_files(chapter);
            let run = match built.get(&chapter) {
                Some(exe) => self.run_chapter(chapter, exe, limit)?,
                None => ChapterRun::NotCompiled,
            };
            let lessons = match run {
                ChapterRun::Finished(lessons) => lessons,
                problem => {
                    lessons_total += files.len();
                    println!("{}", describe_problem(&problem, &rerun));
                    next.get_or_insert(rerun);
                    continue;
                }
            };
            let mut line = String::new();
            let mut done = 0;
            for (module, lesson) in &lessons {
                let mark = if lesson.failed == 0 {
                    done += 1;
                    "✓"
                } else if lesson.passed > 0 {
                    "◐"
                } else {
                    "·"
                };
                line.push_str(&format!(" {mark}{}", short(module)));
                if lesson.failed > 0 {
                    next.get_or_insert_with(|| format!("{rerun} {}_", short(module)));
                    if let Some((_, file)) = files.iter().find(|(m, _)| m == module) {
                        self.snapshot_starter(file)?;
                    }
                }
            }
            lessons_done += done;
            lessons_total += lessons.len();
            if done == lessons.len() && !lessons.is_empty() {
                reviews.entry(chapter).or_insert(Review {
                    completed: now,
                    done: 0,
                    last: now,
                });
            }
            println!("{done:>2}/{:<2}{line}", lessons.len());
        }
        self.save_reviews(&reviews)?;
        if let Some(percent) = (lessons_done * 100).checked_div(lessons_total) {
            println!("\nCore lessons passing: {lessons_done} of {lessons_total} ({percent}%)");
        }
        if all {
            self.other_tracks();
        }
        println!();
        for line in self.review_report(&reviews, now)? {
            println!("{line}");
        }
        match next {
            Some(command) => println!("Next lesson: {command}"),
            None => println!("Every core lesson passes. Keep the reviews going."),
        }
        if !all {
            println!(
                "(`cargo progress all` also tests the project, ecosystem, checkpoint, mastery and \
                 capstone packages.)"
            );
        }
        Ok(())
    }

    /// The review section of `cargo progress`: what is due and what to do.
    fn review_report(
        &self,
        reviews: &BTreeMap<u32, Review>,
        now: u64,
    ) -> Result<Vec<String>, String> {
        let mut due: Vec<(u64, u32, usize)> = reviews
            .iter()
            .filter_map(|(chapter, review)| Some((review.due_day()?, *chapter, review.done)))
            .filter(|(day, _, _)| now >= *day)
            .collect();
        due.sort();
        let mut lines = Vec::new();
        if due.is_empty() {
            let upcoming = reviews
                .iter()
                .filter_map(|(chapter, review)| Some((review.due_day()?, *chapter, review.done)))
                .min();
            if let Some((day, chapter, n)) = upcoming {
                lines.push(format!(
                    "Next review: ch{chapter:02} on {} (review {} of {}: {}). Dates are UTC.",
                    date(day),
                    n + 1,
                    REVIEW_DAYS.len(),
                    review_topic(n)
                ));
            }
            return Ok(lines);
        }
        lines.push(
            "Reviews due. Each one is short and different, so you recall instead of rereading:"
                .into(),
        );
        lines.push(
            "  1 what you missed · 2 mastery questions · 3 capstone again · 4 and 5 mixed, \
             spaced 1, 3, 7, 21 and 60 days after the one before."
                .into(),
        );
        for (_, chapter, n) in due.iter().take(REVIEWS_PER_DAY) {
            lines.push(String::new());
            lines.extend(self.review_plan(*chapter, *n)?);
        }
        if due.len() > REVIEWS_PER_DAY {
            let later: Vec<String> = due[REVIEWS_PER_DAY..]
                .iter()
                .map(|(_, chapter, _)| format!("ch{chapter:02}"))
                .collect();
            lines.push(String::new());
            lines.push(format!(
                "Also due, for another day: {}. Two reviews a day is enough; waiting a day loses nothing.",
                later.join(", ")
            ));
        }
        lines.push(String::new());
        Ok(lines)
    }

    /// The steps of review number `n` (0-based) for one chapter.
    fn review_plan(&self, chapter: u32, n: usize) -> Result<Vec<String>, String> {
        let ch = format!("ch{chapter:02}");
        let notes = format!("exercises/chapter{chapter:02}/NOTES.md");
        let (minutes, steps): (&str, Vec<String>) = match n {
            0 => {
                let missed = self.missed_drills(chapter)?;
                let drills = if missed.is_empty() {
                    format!(
                        "You missed no {ch} drill on the first try. Pick any two in drills/{ch}/ \
                         and say the outcome and the reason before you open them."
                    )
                } else {
                    format!(
                        "Drills you missed on the first try: {}. For each, say the outcome and \
                         the reason before you open drills/{ch}/.",
                        missed.join(", ")
                    )
                };
                (
                    "10",
                    vec![
                        format!(
                            "In {notes}, find each \"Predictions I got wrong\" entry. Cover your \
                             note, answer the prediction again, then compare."
                        ),
                        drills,
                    ],
                )
            }
            1 => (
                "10",
                vec![format!(
                    "Answer the \"Mastery check\" questions at the end of {notes} from memory, \
                     aloud or on paper. Then compare with what you wrote there and correct it."
                )],
            ),
            2 => {
                let capstone = self.capstone(chapter);
                let lesson = capstone.as_deref().map(short).unwrap_or("l01");
                (
                    "30 to 45",
                    vec![
                        format!(
                            "`cargo progress redo {ch} {lesson}` saves your capstone and restores its starter."
                        ),
                        format!(
                            "Solve it again from the contract (`cargo test --test chapter{chapter:02} {lesson}_`), \
                             then compare with your saved version in .progress/attempts/."
                        ),
                    ],
                )
            }
            3 => (
                "15",
                vec![
                    format!(
                        "`cargo drill reset {ch}`, then `cargo drill {ch}`: predict every drill again."
                    ),
                    format!("Answer any two \"Mastery check\" questions in {notes} from memory."),
                ],
            ),
            _ => {
                let has_review = self
                    .lesson_files(chapter)
                    .iter()
                    .any(|(module, _)| module.starts_with("r01_"));
                let lesson = if has_review { "r01" } else { "c01" };
                (
                    "20",
                    vec![
                        format!(
                            "`cargo progress redo {ch} {lesson}`, then solve that lesson again \
                             (`cargo test --test chapter{chapter:02} {lesson}_`)."
                        ),
                        "Without notes, write five sentences that teach this chapter's hardest idea to a colleague."
                            .to_string(),
                    ],
                )
            }
        };
        let mut lines = vec![format!(
            "  {ch}, review {} of {} (about {minutes} minutes): {}",
            n + 1,
            REVIEW_DAYS.len(),
            review_topic(n)
        )];
        lines.extend(steps.into_iter().map(|step| format!("    - {step}")));
        lines.push(format!("    - When done: `cargo progress reviewed {ch}`"));
        Ok(lines)
    }

    /// Records a finished review. A review that is not due yet is recorded
    /// only with `--early`, so repeating the command cannot use up the
    /// schedule by accident.
    fn reviewed(&self, chapter: &str, early: bool, now: u64) -> Result<String, String> {
        let chapter = self.chapter(chapter)?;
        let ch = format!("ch{chapter:02}");
        let mut reviews = self.load_reviews()?;
        let Some(entry) = reviews.get_mut(&chapter) else {
            return Err(format!(
                "{ch} is not complete yet; run `cargo progress` after all its lessons pass"
            ));
        };
        let Some(due) = entry.due_day() else {
            return Ok(format!(
                "{ch} has finished its review schedule. Nothing to record."
            ));
        };
        if now < due && !early {
            return Ok(format!(
                "Nothing recorded: review {} of {} for {ch} is not due until {} (UTC). \
                 To record it today anyway, run `cargo progress reviewed {ch} --early`.",
                entry.done + 1,
                REVIEW_DAYS.len(),
                date(due)
            ));
        }
        entry.done += 1;
        entry.last = now;
        let recorded = entry.done;
        let message = match entry.due_day() {
            Some(day) => format!(
                "Recorded review {recorded} of {} for {ch}. Next: review {} ({}) on {} (UTC), {} days from today.",
                REVIEW_DAYS.len(),
                recorded + 1,
                review_topic(recorded),
                date(day),
                REVIEW_DAYS[recorded]
            ),
            None => format!(
                "Recorded review {recorded} of {}. {ch} has finished its review schedule.",
                REVIEW_DAYS.len()
            ),
        };
        self.save_reviews(&reviews)?;
        Ok(message)
    }

    /// Archives the learner's version of a lesson and restores its starter.
    /// An archive is never overwritten, and a lesson that already equals its
    /// starter is left alone.
    fn redo(&self, chapter: &str, lesson: &str, now: u64) -> Result<String, String> {
        let chapter = self.chapter(chapter)?;
        let wanted = lesson_name(lesson);
        let prefix = format!("{wanted}_");
        let (module, file) = self
            .lesson_files(chapter)
            .into_iter()
            .find(|(module, _)| *module == wanted || module.starts_with(&prefix))
            .ok_or_else(|| {
                format!(
                    "no lesson {lesson} in {}",
                    self.chapter_dir(chapter).display()
                )
            })?;
        let starter = self.starter_path(&file);
        let starter_bytes = fs::read(&starter).map_err(|_| {
            format!(
                "no saved starter for {}, so nothing was changed. The coach saves a starter \
                 when `cargo progress` first sees a lesson unfinished; this lesson was already \
                 finished then, or .progress/ was deleted since. To redo it, copy the lesson \
                 file from a fresh copy of the course repository over yours (keep your version \
                 under another name first).",
                file.display()
            )
        })?;
        let mine = fs::read(&file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
        if mine == starter_bytes {
            return Ok(format!(
                "{} already equals its starter, so there is nothing to archive. Solve it from \
                 the contract.",
                short(&module)
            ));
        }
        let archive = self.state().join("attempts");
        fs::create_dir_all(&archive)
            .map_err(|e| format!("cannot create {}: {e}", archive.display()))?;
        let base = format!("ch{chapter:02}_{}_{module}", date(now));
        let saved = save_unique(&archive, &base, &mine)?;
        fs::write(&file, starter_bytes)
            .map_err(|e| format!("cannot write {}: {e}", file.display()))?;
        Ok(format!(
            "Saved your solution to {} and restored the starter.\n\
             Solve it again from the contract, then compare the two versions.",
            saved.display()
        ))
    }

    /// Points at the model solution of a lesson or drill, without printing it.
    fn solution(&self, unit: &str, item: Option<&str>) -> Result<String, String> {
        let lower = unit.trim_end_matches('/').to_ascii_lowercase();
        // A unit is a name or a path below solutions/, never a way out of it.
        let plain = |part: &str| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        };
        if !lower.split('/').all(plain) {
            return Err(format!(
                "expected a unit such as ch04, m1, project, cap_kvstore or cp1, got {unit}"
            ));
        }
        let solutions = self.root.join("solutions");
        let mut places: Vec<PathBuf> = Vec::new();
        if let Some(n) = chapter_number(&lower) {
            places.push(solutions.join("exercises").join(format!("chapter{n:02}")));
            places.push(solutions.join("drills").join(format!("ch{n:02}")));
        } else {
            places.push(self.at(&format!("solutions/{lower}")));
            places.push(self.at(&format!("solutions/drills/{lower}")));
            for group in ["mastery", "capstones", "checkpoints"] {
                for dir in list_dir(&solutions.join(group)) {
                    let name = dir.file_name().unwrap_or_default().to_string_lossy();
                    // `m1` for mastery/m1_types, `cp1` for a checkpoint,
                    // `game` or `cap_game` for capstones/game.
                    if *name == lower
                        || name.starts_with(&format!("{lower}_"))
                        || (group == "capstones" && lower == format!("cap_{name}"))
                    {
                        places.push(dir);
                    }
                }
            }
        }
        let mut files: Vec<PathBuf> = Vec::new();
        for place in places.iter().filter(|place| place.is_dir()) {
            collect_files(place, &mut files);
        }
        if files.is_empty() {
            return Err(format!(
                "no model solutions for {unit} under {}",
                solutions.display()
            ));
        }
        // Checkpoints and the final assessment have no hints: they are
        // graded against their RUBRIC.md before the solution is read.
        let assessed = |file: &PathBuf| {
            file.starts_with(solutions.join("checkpoints"))
                || file.starts_with(solutions.join("capstones").join("final_assessment"))
        };
        let reminder = |shown: &[&PathBuf]| {
            if shown.iter().all(|file| assessed(file)) {
                GRADE_FIRST
            } else {
                ATTEMPT_FIRST
            }
        };
        let Some(item) = item else {
            let mut lines = vec![format!("Model solutions for {unit}:")];
            lines.extend(files.iter().map(|file| format!("  {}", file.display())));
            lines.push(String::new());
            lines.push(reminder(&files.iter().collect::<Vec<_>>()).to_string());
            return Ok(lines.join("\n"));
        };
        let wanted = lesson_name(item);
        let prefix = format!("{wanted}_");
        let found: Vec<&PathBuf> = files
            .iter()
            .filter(|file| {
                let name = file.file_name().unwrap_or_default().to_string_lossy();
                name.starts_with(&prefix) || file.file_stem().is_some_and(|stem| *stem == *wanted)
            })
            .collect();
        if found.is_empty() {
            return Err(format!(
                "no model solution for {unit} {item}. Run `cargo progress solution {unit}` to list what exists."
            ));
        }
        let mut lines = vec![format!("Model solution for {unit} {item}:")];
        lines.extend(found.iter().map(|file| format!("  {}", file.display())));
        lines.push(String::new());
        lines.push(reminder(&found).to_string());
        Ok(lines.join("\n"))
    }
}

/// The reminder for a checkpoint or the final assessment, which have no hints.
const GRADE_FIRST: &str = "Read this only after you have graded your own work against RUBRIC.md.";

const ATTEMPT_FIRST: &str =
    "Attempt the task first. A solution you only read is forgotten within days; one you \
compare with your own attempt stays.
  1. Make an honest attempt and run the tests.
  2. If you are stuck, open hint 1, then hint 2, in the HINTS.md of that chapter or package.
  3. Only then open the file above. Read it, close it, and finish your own version from memory.
  4. Write in NOTES.md what the model solution does differently, and why.";

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for path in list_dir(dir) {
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

/// Writes `bytes` to `<dir>/<base>.rs`, or `<base>.2.rs`, `<base>.3.rs`, ...
/// when that name is taken. Never replaces an existing file.
fn save_unique(dir: &Path, base: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    for attempt in 1..10_000 {
        let name = if attempt == 1 {
            format!("{base}.rs")
        } else {
            format!("{base}.{attempt}.rs")
        };
        let path = dir.join(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(bytes)
                    .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("cannot write {}: {e}", path.display())),
        }
    }
    Err(format!(
        "too many archives named {base} in {}",
        dir.display()
    ))
}

fn describe_problem(problem: &ChapterRun, rerun: &str) -> String {
    // A crash leaves most tests unfinished; a few names are enough.
    const SHOWN: usize = 5;
    let list = |names: &[String]| match names.len() {
        0 => String::new(),
        n if n <= SHOWN => format!(" Tests that did not finish: {}.", names.join(", ")),
        n => format!(
            " {n} tests did not finish, for example {}.",
            names[..SHOWN].join(", ")
        ),
    };
    match problem {
        ChapterRun::Finished(_) => String::new(),
        ChapterRun::NotCompiled => format!("does not compile: run `{rerun}` to see why"),
        ChapterRun::Crashed {
            reason,
            culprit: Some(test),
            ..
        } => format!(
            "crashed ({reason}) while running {test}; nothing was counted. Run \
             `{rerun} -- --test-threads 1` to see the crash."
        ),
        ChapterRun::Crashed {
            reason, unfinished, ..
        } => format!(
            "crashed ({reason}); nothing was counted.{} Run `{rerun}` to see the crash.",
            list(unfinished)
        ),
        ChapterRun::TimedOut { secs, unfinished } => format!(
            "did not finish in {secs} s and was stopped; nothing was counted.{} A test is \
             probably looping or blocked forever. Run `{rerun}` to watch it.",
            list(unfinished)
        ),
    }
}

// ------------------------------------------- other packages (progress all)

/// Results of one test group (a lesson module or a test file) in a package.
#[derive(Default, Debug, PartialEq)]
struct Group {
    passed: usize,
    failed: usize,
    crashed: bool,
}

/// Groups in natural order: sort key -> (name shown, results).
type Groups = BTreeMap<(String, u64, String), (String, Group)>;

/// `m03_types` and `l01_variance` are shown as `m03` and `l01`.
fn group_entry<'a>(groups: &'a mut Groups, name: &str) -> &'a mut Group {
    let numbered = name.as_bytes().get(1).is_some_and(u8::is_ascii_digit);
    let name = if numbered { short(name) } else { name };
    &mut groups
        .entry(natural(name))
        .or_insert_with(|| (name.to_string(), Group::default()))
        .1
}

/// Groups `cargo test` output of a whole package. `stderr` names the test
/// programs in order (`Running ...` lines) and `stdout` holds their results.
/// Tests inside a module are grouped by the module (`l01_variance::...`);
/// tests at the top of a test file are grouped by the file.
fn package_groups(stdout: &str, stderr: &str) -> Groups {
    let targets: Vec<String> = stderr
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            if line.starts_with("Doc-tests ") {
                return Some("doc".to_string());
            }
            let rest = line.strip_prefix("Running ")?;
            let source = rest.strip_prefix("unittests ").unwrap_or(rest);
            let source = source.split(" (").next().unwrap_or(source);
            let stem = Path::new(source).file_stem()?.to_str()?;
            Some(stem.to_string())
        })
        .collect();
    let mut groups = Groups::new();
    for (index, block) in parse_test_output(stdout).iter().enumerate() {
        // Doc-tests run last and can print two blocks for one `Doc-tests`
        // line (merged tests, then the ones that run alone), so a block
        // without a target of its own is a doc-test block.
        let target = targets.get(index).map_or("doc", String::as_str);
        for (name, mark) in &block.results {
            let group = match name.split_once("::") {
                Some((module, _)) if target != "doc" => group_entry(&mut groups, module),
                _ => group_entry(&mut groups, target),
            };
            match mark {
                Mark::Passed => group.passed += 1,
                Mark::Failed => group.failed += 1,
                Mark::Ignored => {}
            }
        }
        if !block.summary {
            group_entry(&mut groups, target).crashed = true;
        }
    }
    groups
}

impl Repo {
    /// Standalone packages next to the core course, as root-relative paths,
    /// in course order: the project and the ecosystem track, then every
    /// package found under checkpoints/, mastery/ and capstones/.
    fn packages(&self) -> Vec<String> {
        let mut found: Vec<String> = ["project", "ecosystem"]
            .into_iter()
            .map(String::from)
            .collect();
        for group in ["checkpoints", "mastery", "capstones"] {
            let mut names: Vec<String> = list_dir(&self.root.join(group))
                .into_iter()
                .filter_map(|dir| Some(dir.file_name()?.to_str()?.to_string()))
                .collect();
            names.sort_by_key(|name| natural(name));
            found.extend(names.into_iter().map(|name| format!("{group}/{name}")));
        }
        found.retain(|package| self.at(package).join("Cargo.toml").is_file());
        found
    }

    /// Runs each standalone package's tests and prints one row per package.
    fn other_tracks(&self) {
        let packages = self.packages();
        if packages.is_empty() {
            return;
        }
        let limit = seconds("COACH_PACKAGE_SECS", 900);
        let width = packages.iter().map(String::len).max().unwrap_or(0);
        println!(
            "\nOther tracks (each row runs `cargo test` in that package; the first run may \
             download and build its crates):"
        );
        for package in packages {
            print!("{package:<width$}  ");
            flush();
            println!("{}", self.package_row(&package, limit));
        }
    }

    fn package_row(&self, package: &str, limit: Duration) -> String {
        let dir = self.at(package);
        let rerun = format!("cargo test --manifest-path {package}/Cargo.toml");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let mut test = Command::new(cargo);
        // The package directory is the working directory, so its own
        // .cargo/config.toml applies. (Every package pins the same toolchain
        // as the root, which is the one this Cargo belongs to.)
        test.current_dir(&dir)
            .env_remove("CARGO_MANIFEST_DIR")
            .args(["test", "--no-fail-fast", "--manifest-path"])
            .arg(dir.join("Cargo.toml"))
            .args(["--", "--color", "never"]);
        let finished = match run_limited(&mut test, limit) {
            Ok(finished) => finished,
            Err(message) => return message,
        };
        let groups = package_groups(&finished.stdout, &finished.stderr);
        if matches!(finished.ended, Ended::TimedOut) {
            return format!(
                "did not finish in {} s and was stopped: run `{rerun}`",
                limit.as_secs()
            );
        }
        let errors = strip_ansi(&finished.stderr);
        if errors.contains("error: could not compile") {
            return format!("does not compile: run `{rerun}` to see why");
        }
        if groups.is_empty() {
            let why = errors
                .lines()
                .find(|line| line.starts_with("error"))
                .unwrap_or("no test ran");
            return format!("could not test ({why}): run `{rerun}`");
        }
        let done = groups
            .values()
            .filter(|(_, g)| g.failed == 0 && g.passed > 0 && !g.crashed)
            .count();
        let mut row = format!("{done:>2}/{:<2}", groups.len());
        if groups.len() <= 30 {
            for (name, group) in groups.values() {
                let mark = if group.crashed {
                    "✗"
                } else if group.failed == 0 && group.passed > 0 {
                    "✓"
                } else if group.passed > 0 {
                    "◐"
                } else {
                    "·"
                };
                row.push_str(&format!(" {mark}{name}"));
            }
        }
        if groups.values().any(|(_, group)| group.crashed) {
            row.push_str(&format!("  (✗ crashed: run `{rerun}`)"));
        }
        row
    }
}

// ------------------------------------------------------------ setup check

/// The dated nightly of the Miri labs (README, "The nightly toolchain").
const NIGHTLY: &str = "nightly-2026-11-20";
/// How long one tool may take to report its version.
const PROBE_SECS: u64 = 15;

/// Where the coach runs. The fixes and the advice differ a little.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Place {
    Codespaces,
    /// The course's dev container (it sets DEVCONTAINER=true), or any
    /// container that VS Code opened (REMOTE_CONTAINERS=true).
    DevContainer,
    /// Some other container, such as toolbox or distrobox: only a marker
    /// file such as `/.dockerenv` says so.
    OtherContainer,
    OwnComputer,
}

impl Place {
    fn label(self) -> &'static str {
        match self {
            Place::Codespaces => "GitHub Codespaces",
            Place::DevContainer => "the course dev container",
            Place::OtherContainer => "a container (not the course dev container)",
            Place::OwnComputer => "your own computer",
        }
    }
}

/// Finds the environment from its variables (`var`) and from whether a
/// container marker file such as `/.dockerenv` exists.
fn place_from(var: impl Fn(&str) -> Option<String>, container_file: bool) -> Place {
    let is_true = |name: &str| var(name).is_some_and(|value| value.trim() == "true");
    if is_true("CODESPACES") {
        Place::Codespaces
    } else if is_true("REMOTE_CONTAINERS") || is_true("DEVCONTAINER") {
        Place::DevContainer
    } else if container_file {
        Place::OtherContainer
    } else {
        Place::OwnComputer
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Os {
    Mac,
    Linux,
    Windows,
}

impl Os {
    fn from_name(name: &str) -> Os {
        match name {
            "macos" => Os::Mac,
            "windows" => Os::Windows,
            _ => Os::Linux,
        }
    }

    /// Picks the text for this system.
    fn pick<'a>(self, mac: &'a str, linux: &'a str, windows: &'a str) -> &'a str {
        match self {
            Os::Mac => mac,
            Os::Linux => linux,
            Os::Windows => windows,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Need {
    /// Every chapter needs it: the doctor fails without it.
    Required,
    /// Only some labs need it: reported, never a failure.
    Optional,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Status {
    Found,
    MissingRequired,
    MissingOptional,
}

impl Status {
    fn classify(need: Need, found: bool) -> Status {
        match (found, need) {
            (true, _) => Status::Found,
            (false, Need::Required) => Status::MissingRequired,
            (false, Need::Optional) => Status::MissingOptional,
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Status::Found => "✓",
            Status::MissingRequired => "✗",
            Status::MissingOptional => "·",
        }
    }
}

/// The result of one check.
struct Check {
    name: &'static str,
    need: Need,
    /// What the tool is needed for.
    purpose: &'static str,
    /// `Ok(version or detail)` when found, `Err(what is wrong)` when not.
    found: Result<String, String>,
    /// The command or step that fixes it.
    fix: String,
}

impl Check {
    fn status(&self) -> Status {
        Status::classify(self.need, self.found.is_ok())
    }
}

/// `rustc 1.99.0 (b940084d7 2026-09-28)` -> `1.99.0`. The same for cargo.
fn tool_version(output: &str) -> Option<&str> {
    let mut words = output.lines().next()?.split_whitespace();
    words.next()?;
    words.next()
}

/// The `channel` of a `rust-toolchain.toml`.
fn toolchain_channel(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "channel")
            .then(|| value.trim().trim_matches('"').to_string())
            .filter(|channel| !channel.is_empty())
    })
}

/// Whether `rustup toolchain list` shows the toolchain `name`, which is
/// listed with the host added: `nightly-2026-11-20-aarch64-apple-darwin`.
fn lists_toolchain(listing: &str, name: &str) -> bool {
    listing.lines().any(|line| {
        line.split_whitespace().next().is_some_and(|toolchain| {
            toolchain == name || toolchain.starts_with(&format!("{name}-"))
        })
    })
}

/// Whether `rustup component list --installed` shows the component `name`
/// (`miri-aarch64-apple-darwin`, or `rust-src` with no host).
fn lists_component(listing: &str, name: &str) -> bool {
    listing.lines().any(|line| {
        let line = line.trim();
        line == name || line.starts_with(&format!("{name}-"))
    })
}

/// The first line of a tool's answer, short enough for one row.
fn first_line(text: &str) -> String {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty());
    let line = line.unwrap_or("");
    match line.char_indices().nth(60) {
        Some((cut, _)) => format!("{}...", &line[..cut]),
        None => line.to_string(),
    }
}

impl Repo {
    /// Runs `program args` from the repository root, so rustup applies the
    /// course's `rust-toolchain.toml`, and returns its first line of output.
    /// Never waits longer than `PROBE_SECS`, and never lets rustup download.
    fn probe(&self, program: &str, args: &[&str]) -> Result<String, String> {
        self.probe_output(program, args)
            .map(|(stdout, stderr)| first_line(&format!("{stdout}\n{stderr}")))
    }

    /// Like `probe`, but returns the whole standard output.
    fn probe_all(&self, program: &str, args: &[&str]) -> Result<String, String> {
        self.probe_output(program, args).map(|(stdout, _)| stdout)
    }

    /// Runs a probe and returns (stdout, stderr) when it succeeded.
    fn probe_output(&self, program: &str, args: &[&str]) -> Result<(String, String), String> {
        let mut command = Command::new(program);
        command
            .current_dir(&self.root)
            .env("RUSTUP_AUTO_INSTALL", "0")
            .args(args);
        let finished = run_limited(&mut command, Duration::from_secs(PROBE_SECS))?;
        match finished.ended {
            Ended::Exited(status) if status.success() => Ok((finished.stdout, finished.stderr)),
            Ended::Exited(status) => {
                let why = first_line(&finished.stderr);
                Err(if why.is_empty() {
                    format!("{program} failed ({status})")
                } else {
                    format!("{program} failed: {why}")
                })
            }
            Ended::TimedOut => Err(format!("{program} did not answer within {PROBE_SECS} s")),
        }
    }

    /// `cargo progress doctor`: checks the tools the course uses and prints
    /// the fix for each missing one. Fails only when a required tool is
    /// missing.
    fn doctor(&self) -> Result<(), String> {
        let os = Os::from_name(std::env::consts::OS);
        let place = place_from(
            |name| std::env::var(name).ok(),
            Path::new("/.dockerenv").exists() || Path::new("/run/.containerenv").exists(),
        );
        let checks = self.setup_checks(os, place);
        let width = checks
            .iter()
            .map(|check| check.name.len())
            .max()
            .unwrap_or(0);
        println!("Setup check");
        let system = os.pick("macOS", "Linux", "Windows");
        println!("Environment: {} ({system})", place.label());
        println!("✓ found   ✗ missing, every chapter needs it   · missing, some labs need it");
        for (need, title) in [
            (Need::Required, "\nNeeded for every chapter:"),
            (
                Need::Optional,
                "\nNeeded only for some labs (install each when you get there):",
            ),
        ] {
            println!("{title}");
            for check in checks.iter().filter(|check| check.need == need) {
                let detail = match &check.found {
                    Ok(detail) | Err(detail) => detail,
                };
                println!(
                    "  {} {:<width$}  {detail}",
                    check.status().symbol(),
                    check.name
                );
                if check.found.is_err() {
                    println!("      needed for: {}", check.purpose);
                    println!("      fix: {}", check.fix);
                }
            }
        }
        let missing = |status: Status| {
            checks
                .iter()
                .filter(|check| check.status() == status)
                .count()
        };
        let required = missing(Status::MissingRequired);
        let optional = missing(Status::MissingOptional);
        println!();
        if os == Os::Linux && required + optional > 0 {
            println!(
                "Package names are those of Debian and Ubuntu. Other Linux systems use \
                 similar names."
            );
        }
        let preinstalled_missing = checks
            .iter()
            .any(|check| check.found.is_err() && !matches!(check.name, "Miri" | "Docker"));
        if matches!(place, Place::Codespaces | Place::DevContainer) && preinstalled_missing {
            println!(
                "The dev container should already contain this. Rebuild it: in VS Code, \
                 run \"Dev Containers: Rebuild Container\" (in Codespaces: \"Codespaces: \
                 Rebuild Container\") from the Command Palette."
            );
        }
        if required > 0 {
            return Err(format!(
                "{required} required check(s) failed. Run the fix commands above, then run \
                 `cargo progress doctor` again."
            ));
        }
        match optional {
            0 => println!("Everything is installed."),
            n => println!(
                "Everything the chapters need is installed. {n} optional tool(s) are missing; \
                 install each one when you reach the lab that needs it."
            ),
        }
        Ok(())
    }

    fn setup_checks(&self, os: Os, place: Place) -> Vec<Check> {
        let pinned = fs::read_to_string(self.root.join("rust-toolchain.toml"))
            .ok()
            .and_then(|text| toolchain_channel(&text));
        // `None` when rustup itself is missing.
        let toolchains = self.probe_all("rustup", &["toolchain", "list"]).ok();
        let fix_toolchain = match (&toolchains, &pinned) {
            (None, _) => "install rustup from https://rustup.rs, then run `rustup toolchain \
                          install` in the repository folder"
                .to_string(),
            (Some(list), Some(want)) if lists_toolchain(list, want) => format!(
                "leave out any `+toolchain` in the command, unset RUSTUP_TOOLCHAIN, and run \
                 `rustup override unset` in the repository folder ({want} is installed, but \
                 another version was chosen)"
            ),
            (Some(_), _) => "rustup toolchain install   (in the repository folder: it installs \
                             the version that rust-toolchain.toml names)"
                .to_string(),
        };
        let version_check = |name: &'static str, program: &str| {
            let found = self.probe(program, &["--version"]).and_then(|answer| {
                let version = tool_version(&answer).unwrap_or("unknown").to_string();
                match &pinned {
                    Some(want) if *want == version => Ok(version),
                    Some(want) => Err(format!("{version} is active; the course needs {want}")),
                    None => Err(format!(
                        "{version} is active, but rust-toolchain.toml names no version"
                    )),
                }
            });
            Check {
                name,
                need: Need::Required,
                purpose: "every chapter",
                found,
                fix: fix_toolchain.clone(),
            }
        };
        let for_toolchain = pinned
            .as_ref()
            .map(|want| format!(" --toolchain {want}"))
            .unwrap_or_default();
        let mut checks = vec![
            version_check("rustc", "rustc"),
            version_check("cargo", "cargo"),
            Check {
                name: "clippy",
                need: Need::Required,
                purpose: "`cargo clippy`, the linter the lessons use",
                found: self.probe("cargo", &["clippy", "--version"]),
                fix: "rustup component add clippy".into(),
            },
            Check {
                name: "rustfmt",
                need: Need::Required,
                purpose: "`cargo fmt`, the code formatter",
                found: self.probe("rustfmt", &["--version"]),
                fix: "rustup component add rustfmt".into(),
            },
        ];

        // The whole path: `probe` shortens its answer for display, and a
        // sysroot is often longer than that (for example in the dev
        // container, /home/vscode/.rustup/toolchains/1.99.0-<host>).
        let sysroot = self
            .probe_all("rustc", &["--print", "sysroot"])
            .ok()
            .map(|out| out.trim().to_string())
            .filter(|out| !out.is_empty())
            .map(PathBuf::from);
        let in_sysroot = |parts: &[&str]| {
            sysroot.as_ref().is_some_and(|root| {
                parts
                    .iter()
                    .fold(root.clone(), |path, part| path.join(part))
                    .exists()
            })
        };
        let installed = |yes: bool, what: &str| {
            if yes {
                Ok("installed".to_string())
            } else {
                Err(format!("{what} is not installed for this toolchain"))
            }
        };
        checks.push(Check {
            name: "rust-src",
            need: Need::Optional,
            purpose: "rust-analyzer, to show and analyse the standard library",
            found: installed(
                in_sysroot(&["lib", "rustlib", "src", "rust", "library"]),
                "rust-src",
            ),
            fix: "rustup component add rust-src".into(),
        });
        checks.push(Check {
            name: "git",
            need: Need::Optional,
            purpose: "saving your work and your .progress/ history in commits",
            found: self.probe("git", &["--version"]),
            fix: os
                .pick(
                    "xcode-select --install   (or: brew install git)",
                    "sudo apt install git",
                    "winget install --id Git.Git -e --source winget",
                )
                .into(),
        });
        checks.push(Check {
            name: "aarch64-unknown-none",
            need: Need::Optional,
            purpose: "the kernel capstone (capstones/kernel)",
            found: installed(
                in_sysroot(&["lib", "rustlib", "aarch64-unknown-none"]),
                "the target",
            ),
            fix: format!("rustup target add aarch64-unknown-none{for_toolchain}"),
        });
        checks.push(self.qemu_check(os));
        checks.push(self.c_compiler_check(os));
        checks.push(self.debugger_check(os, &in_sysroot));
        checks.push(self.miri_check(toolchains.as_deref()));
        checks.push(Check {
            name: "Docker",
            need: Need::Optional,
            purpose: "the Dockerfile lab of M11 and the Docker labs of M9",
            found: self.probe("docker", &["--version"]),
            fix: match place {
                Place::Codespaces | Place::DevContainer => {
                    "Docker is not part of the dev container. Do the Docker labs on your \
                     own computer, with Docker installed there."
                }
                Place::OtherContainer => {
                    "Docker usually does not run inside a container. Do the Docker labs \
                     on the computer that runs this container, with Docker installed there."
                }
                Place::OwnComputer => os.pick(
                    "install Docker Desktop, or a free alternative such as Podman, Rancher \
                     Desktop or Colima (OrbStack is free for personal use)",
                    "install Docker Engine: https://docs.docker.com/engine/install/",
                    "install Docker Desktop, or a free alternative such as Podman or \
                     Rancher Desktop",
                ),
            }
            .into(),
        });
        checks
    }

    fn qemu_check(&self, os: Os) -> Check {
        let mut found = self.probe("qemu-system-aarch64", &["--version"]);
        // The Windows installer does not add its folder to PATH.
        if os == Os::Windows && found.is_err() {
            let folder = std::env::var_os("ProgramFiles")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
                .join("qemu");
            if folder.join("qemu-system-aarch64.exe").is_file() {
                found = Err(format!(
                    "installed in {}, but that folder is not on PATH",
                    folder.display()
                ));
            }
        }
        Check {
            name: "qemu-system-aarch64",
            need: Need::Optional,
            purpose: "the kernel capstone, which runs your kernel in QEMU",
            found,
            fix: os
                .pick(
                    "brew install qemu",
                    "sudo apt install qemu-system-arm   (it contains qemu-system-aarch64)",
                    "install QEMU from https://www.qemu.org/download/, then add its folder \
                     (usually C:\\Program Files\\qemu) to PATH",
                )
                .into(),
        }
    }

    fn c_compiler_check(&self, os: Os) -> Check {
        let first_found = |programs: &[&str]| {
            let mut last = Err(String::new());
            for program in programs {
                last = self.probe(program, &["--version"]);
                if last.is_ok() {
                    break;
                }
            }
            last
        };
        let found = match os {
            // Without the command line tools, `cc` on macOS is a stub that
            // opens an installer window, so ask xcode-select first.
            Os::Mac => self
                .probe("xcode-select", &["-p"])
                .map_err(|_| "the Xcode command line tools are not installed".to_string())
                .and_then(|_| self.probe("cc", &["--version"])),
            Os::Linux => first_found(&["cc", "gcc", "clang"])
                .map_err(|_| "no cc, gcc or clang on PATH".to_string()),
            Os::Windows => self.msvc_or_gcc(),
        };
        Check {
            name: "C compiler",
            need: Need::Optional,
            purpose: "the labs of chapter 24, M6 (FFI) and M11",
            found,
            fix: os
                .pick(
                    "xcode-select --install",
                    "sudo apt install build-essential",
                    "install the Visual Studio Build Tools with the \"Desktop development \
                     with C++\" workload (Rust's MSVC toolchain needs it anyway)",
                )
                .into(),
        }
    }

    /// On Windows: the Visual C++ tools (found with vswhere, which the
    /// Visual Studio installer puts in a fixed place), or gcc for the GNU
    /// toolchain.
    fn msvc_or_gcc(&self) -> Result<String, String> {
        let host = self
            .probe("rustc", &["--print", "host-tuple"])
            .unwrap_or_default();
        if host.contains("windows-gnu") {
            return self.probe("gcc", &["--version"]);
        }
        let tools = if host.starts_with("aarch64") {
            "Microsoft.VisualStudio.Component.VC.Tools.ARM64"
        } else {
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64"
        };
        let vswhere = std::env::var_os("ProgramFiles(x86)")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"))
            .join("Microsoft Visual Studio")
            .join("Installer")
            .join("vswhere.exe");
        let program = vswhere.to_string_lossy().into_owned();
        match self.probe(
            &program,
            &[
                "-latest",
                "-products",
                "*",
                "-requires",
                tools,
                "-property",
                "installationPath",
            ],
        ) {
            Ok(path) if !path.is_empty() => Ok(format!("Visual C++ tools in {path}")),
            _ => self
                .probe("cl", &[])
                .map(|_| "cl.exe on PATH".to_string())
                .map_err(|_| "the Visual C++ build tools were not found".to_string()),
        }
    }

    fn debugger_check(&self, os: Os, in_sysroot: &dyn Fn(&[&str]) -> bool) -> Check {
        let wrapper = |name: &str| {
            let exe = format!("{name}{}", std::env::consts::EXE_SUFFIX);
            in_sysroot(&["bin", name]) || in_sysroot(&["bin", &exe])
        };
        // On macOS without the command line tools, `lldb` is a stub that
        // opens an installer window, so ask xcode-select first.
        let lldb_allowed = os != Os::Mac || self.probe("xcode-select", &["-p"]).is_ok();
        let lldb = wrapper("rust-lldb") && lldb_allowed;
        let found = if lldb && let Ok(version) = self.probe("lldb", &["--version"]) {
            Ok(format!("rust-lldb ({version})"))
        } else if wrapper("rust-gdb")
            && let Ok(version) = self.probe("gdb", &["--version"])
        {
            Ok(format!("rust-gdb ({version})"))
        } else if !wrapper("rust-lldb") && !wrapper("rust-gdb") {
            Err("this toolchain has no rust-gdb or rust-lldb".to_string())
        } else {
            Err("neither lldb nor gdb is installed".to_string())
        };
        Check {
            name: "debugger",
            need: Need::Optional,
            purpose: "the debugger lab of M9 (rust-lldb or rust-gdb)",
            found,
            fix: os
                .pick(
                    "xcode-select --install   (it contains lldb)",
                    "sudo apt install gdb",
                    "Rust for Windows usually has no rust-gdb or rust-lldb. Do the M9 \
                     debugger lab in the dev container.",
                )
                .into(),
        }
    }

    /// `toolchains` is the output of `rustup toolchain list`, `None` without
    /// rustup.
    fn miri_check(&self, toolchains: Option<&str>) -> Check {
        let found = match toolchains {
            None => Err("rustup is not installed, so no nightly toolchain can be checked".into()),
            Some(list) if lists_toolchain(list, NIGHTLY) => {
                let components = self
                    .probe_all(
                        "rustup",
                        &["component", "list", "--installed", "--toolchain", NIGHTLY],
                    )
                    .unwrap_or_default();
                if lists_component(&components, "miri") {
                    Ok(format!("{NIGHTLY} with miri"))
                } else {
                    Err(format!("{NIGHTLY} is installed, but without miri"))
                }
            }
            Some(_) => Err(format!("{NIGHTLY} is not installed")),
        };
        Check {
            name: "Miri",
            need: Need::Optional,
            purpose: "the Miri labs (chapter 23, the project, M2, M6 and others)",
            found,
            fix: format!(
                "rustup toolchain install {NIGHTLY} --component miri --component rust-src"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A throwaway repository in the system temp directory. Its name has a
    /// space in it, because learners' paths do too.
    struct Scratch(Repo);

    impl Scratch {
        fn new() -> Scratch {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "coach test {}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let target = root.join("target");
            Scratch(Repo { root, target })
        }

        fn write(&self, key: &str, text: &str) -> PathBuf {
            let path = self.0.at(key);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, text).unwrap();
            path
        }

        fn read(&self, key: &str) -> String {
            fs::read_to_string(self.0.at(key)).unwrap()
        }

        /// A chapter with the given lesson files and its Cargo test target.
        fn chapter(&self, chapter: u32, lessons: &[&str]) {
            let mut manifest =
                fs::read_to_string(self.0.root.join("Cargo.toml")).unwrap_or_default();
            manifest.push_str(&format!("[[test]]\nname = \"chapter{chapter:02}\"\n"));
            self.write("Cargo.toml", &manifest);
            self.write(&format!("exercises/chapter{chapter:02}/mod.rs"), "");
            for lesson in lessons {
                self.write(
                    &format!("exercises/chapter{chapter:02}/{lesson}.rs"),
                    &format!("// starter of {lesson}\n"),
                );
            }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }

    fn errors(codes: &[&str]) -> Outcome {
        Outcome::Errors(codes.iter().map(|c| c.to_string()).collect())
    }

    #[test]
    fn judge_compares_predictions_with_outcomes() {
        assert!(judge("ok", &Outcome::Ok) == Verdict::Right);
        assert!(judge("OK", &Outcome::Panic) == Verdict::Wrong);
        assert!(judge("panic", &Outcome::Panic) == Verdict::Right);
        assert!(judge("E0382", &errors(&["E0382", "E0382"])) == Verdict::Right);
        assert!(judge("e0382, E0499", &errors(&["E0499", "E0382"])) == Verdict::Right);
        assert!(judge("E0382", &errors(&["E0382", "E0499"])) == Verdict::Partial);
        assert!(judge("error", &errors(&["E0382"])) == Verdict::Partial);
        assert!(judge("error", &errors(&[])) == Verdict::Right);
        assert!(judge("E0505", &errors(&["E0382"])) == Verdict::Wrong);
        assert!(judge("ok", &errors(&["E0382"])) == Verdict::Wrong);
        assert!(judge("ok", &Outcome::Hang(20)) == Verdict::Wrong);
    }

    #[test]
    fn predictions_are_read_tolerantly() {
        let codes =
            |codes: &[&str]| Prediction::Error(codes.iter().map(|c| c.to_string()).collect());
        assert_eq!(read_prediction("E0382."), Some(codes(&["E0382"])));
        assert_eq!(read_prediction("error[E0382]"), Some(codes(&["E0382"])));
        assert_eq!(
            read_prediction("error E0382 and e0499"),
            Some(codes(&["E0382", "E0499"]))
        );
        assert_eq!(read_prediction("compile error"), Some(codes(&[])));
        assert_eq!(read_prediction("ok."), Some(Prediction::Ok));
        assert_eq!(read_prediction("Compiles"), Some(Prediction::Ok));
        assert_eq!(read_prediction("Panic!"), Some(Prediction::Panic));
        assert_eq!(read_prediction("runtime panic"), Some(Prediction::Panic));
        assert_eq!(read_prediction("panics"), Some(Prediction::Panic));
        // Not understood: the coach asks again instead of recording "wrong".
        assert_eq!(read_prediction("it moves the string"), None);
        assert_eq!(read_prediction("ok E0382"), None);
        assert_eq!(read_prediction("E382"), None);
        assert!(judge("error[E0382].", &errors(&["E0382"])) == Verdict::Right);
        assert!(judge("panics", &Outcome::Panic) == Verdict::Right);
    }

    #[test]
    fn expect_line_ignores_placeholders() {
        assert_eq!(expect_line("//! EXPECT: ?\nfn main() {}"), None);
        assert_eq!(expect_line("//! EXPECT:\n"), None);
        assert_eq!(expect_line("//! EXPECT: E0382\n").as_deref(), Some("E0382"));
        assert_eq!(expect_line("// EXPECT: ok\n").as_deref(), Some("ok"));
    }

    #[test]
    fn keep_lines_and_ansi() {
        let src = "let a = 1;\n    assert!(a == 1); // KEEP\n";
        assert_eq!(keep_lines(src), ["assert!(a == 1); // KEEP"]);
        assert_eq!(
            strip_ansi("\u{1b}[1m\u{1b}[38;5;9merror[E0382]\u{1b}[0m: x"),
            "error[E0382]: x"
        );
        assert_eq!(errors(&["E0382", "E0499", "E0382"]).label(), "E0382 E0499");
    }

    #[test]
    fn dates_and_chapters() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(20_384), "2025-10-23");
        assert_eq!(date(11_016), "2000-02-29");
        assert!(!date(u64::MAX).is_empty(), "an absurd day must not panic");
        assert_eq!(chapter_number("ch04"), Some(4));
        assert_eq!(chapter_number("chapter24"), Some(24));
        assert_eq!(chapter_number("CH2"), Some(2));
        assert_eq!(chapter_number("7"), Some(7));
        assert_eq!(chapter_number("ch0"), None);
        assert_eq!(chapter_number("ch123"), None);
        assert_eq!(chapter_number("m1"), None);
    }

    // Fix 1: every path comes from the repository root, whatever the
    // current directory is and however a drill path is spelled.
    #[test]
    fn paths_resolve_from_the_repository_root() {
        let scratch = Scratch::new();
        let repo = &scratch.0;
        let drill = scratch.write("drills/ch04/d01_move.rs", "fn main() {}\n");
        let inside = repo.root.join("drills").join("ch04");
        let elsewhere = repo.root.join("exercises");
        fs::create_dir_all(&elsewhere).unwrap();
        let key = "drills/ch04/d01_move.rs";
        assert_eq!(
            repo.resolve_drill("drills/ch04/d01_move.rs", &repo.root)
                .as_deref(),
            Ok(key)
        );
        assert_eq!(
            repo.resolve_drill("./drills/ch04/d01_move.rs", &repo.root)
                .as_deref(),
            Ok(key)
        );
        assert_eq!(
            repo.resolve_drill("d01_move.rs", &inside).as_deref(),
            Ok(key)
        );
        assert_eq!(
            repo.resolve_drill("../drills/ch04/d01_move.rs", &elsewhere)
                .as_deref(),
            Ok(key)
        );
        // Root-relative spelling also works from a subdirectory.
        assert_eq!(
            repo.resolve_drill("drills/ch04/d01_move.rs", &elsewhere)
                .as_deref(),
            Ok(key)
        );
        assert_eq!(
            repo.resolve_drill(drill.to_str().unwrap(), &elsewhere)
                .as_deref(),
            Ok(key)
        );
        // Files outside drills/ are refused, and so are missing files.
        scratch.write("exercises/chapter04/l01_scope.rs", "");
        assert!(
            repo.resolve_drill("exercises/chapter04/l01_scope.rs", &repo.root)
                .is_err()
        );
        assert!(
            repo.resolve_drill("drills/ch04/missing.rs", &repo.root)
                .is_err()
        );
        // State and originals live under the root, never under the cwd.
        assert_eq!(repo.state(), repo.root.join(".progress"));
        assert_eq!(
            repo.at(&Repo::original_key(key)),
            repo.root
                .join(".progress")
                .join("originals")
                .join("drills")
                .join("ch04")
                .join("d01_move.rs")
        );
    }

    // Fix 2: results from a crashed or cut-short test program are not trusted.
    #[test]
    fn a_crashed_test_program_is_never_counted() {
        let aborted =
            "\nrunning 3 tests\ntest l01_a::tests::one ... ok\ntest l01_a::tests::two ... ok\n";
        let output = parse_test_output(aborted).pop().unwrap();
        assert_eq!(output.announced, Some(3));
        assert_eq!(output.results.len(), 2);
        assert!(!output.summary);
        // Killed by a signal: no exit code at all.
        assert_eq!(
            crash_reason(&output, None, "signal: 6 (SIGABRT)").as_deref(),
            Some("signal: 6 (SIGABRT)")
        );
        // An exit code a test run never produces (a Windows stack overflow).
        assert!(crash_reason(&output, Some(-1073741571), "exit code: 0xc00000fd").is_some());
        // Even a clean exit code is not enough without the summary line.
        assert!(
            crash_reason(&output, Some(0), "exit status: 0")
                .unwrap()
                .contains("test result")
        );

        let complete = "running 2 tests\ntest l01_a::tests::one ... ok\ntest l01_a::tests::two ... FAILED\n\n\
                        failures:\n    l01_a::tests::two\n\ntest result: FAILED. 1 passed; 1 failed; 0 ignored\n";
        let output = parse_test_output(complete).pop().unwrap();
        assert!(output.summary);
        assert_eq!(crash_reason(&output, Some(101), "exit status: 101"), None);
        assert_eq!(crash_reason(&output, Some(0), "exit status: 0"), None);
        // A summary with a signal exit is still a crash.
        assert!(crash_reason(&output, None, "signal: 11 (SIGSEGV)").is_some());

        // A result line is missing although the summary was printed.
        let short = "running 3 tests\ntest a::x ... ok\ntest result: ok. 1 passed\n";
        let output = parse_test_output(short).pop().unwrap();
        assert!(
            crash_reason(&output, Some(0), "exit status: 0")
                .unwrap()
                .contains("3 tests were announced")
        );

        let problem = ChapterRun::Crashed {
            reason: "signal: 6 (SIGABRT)".into(),
            culprit: None,
            unfinished: vec!["l05_capstone::tests::recursion".into()],
        };
        let line = describe_problem(&problem, "cargo test --test chapter12");
        assert!(line.starts_with("crashed (signal: 6 (SIGABRT))"), "{line}");
        assert!(line.contains("l05_capstone::tests::recursion"), "{line}");

        // Tests run in parallel, so a crash leaves many unfinished: the list
        // is cut short, and the guilty test is named when it is known.
        let many: Vec<String> = (1..=40).map(|n| format!("l01_a::tests::t{n}")).collect();
        let problem = ChapterRun::Crashed {
            reason: "signal: 6 (SIGABRT)".into(),
            culprit: None,
            unfinished: many.clone(),
        };
        let line = describe_problem(&problem, "cargo test --test chapter12");
        assert!(line.contains("40 tests did not finish"), "{line}");
        assert!(line.contains("t5") && !line.contains("t6,"), "{line}");
        let problem = ChapterRun::Crashed {
            reason: "signal: 6 (SIGABRT)".into(),
            culprit: Some("l05_capstone::tests::recursion".into()),
            unfinished: many,
        };
        let line = describe_problem(&problem, "cargo test --test chapter12");
        assert!(
            line.contains("while running l05_capstone::tests::recursion"),
            "{line}"
        );
        assert!(!line.contains("l01_a"), "{line}");

        let serial =
            "\nrunning 3 tests\ntest l01_a::tests::one ... ok\ntest l05_c::tests::deep ... ";
        assert_eq!(
            interrupted_test(serial).as_deref(),
            Some("l05_c::tests::deep")
        );
        assert_eq!(
            interrupted_test("running 1 test\ntest m::boom - should panic ... \n").as_deref(),
            Some("m::boom")
        );
        // A run that finished has no interrupted test.
        assert_eq!(interrupted_test(complete), None);
        assert_eq!(interrupted_test(""), None);
    }

    #[test]
    fn test_output_is_parsed_per_program() {
        let stdout = "running 1 test\ntest solo ... ok\n\ntest result: ok. 1 passed\n\n\
                      running 3 tests\ntest m::a - should panic ... ok\ntest m::b ... ignored, slow\n\
                      test m::c ... FAILED\n\ntest result: FAILED. 1 passed; 1 failed; 1 ignored\n";
        let blocks = parse_test_output(stdout);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].results, [("solo".to_string(), Mark::Passed)]);
        assert_eq!(
            blocks[1].results,
            [
                ("m::a".to_string(), Mark::Passed),
                ("m::b".to_string(), Mark::Ignored),
                ("m::c".to_string(), Mark::Failed)
            ]
        );
        assert!(crash_reason(&blocks[1], Some(101), "").is_none());

        // The captured output of a failed test can look like test output
        // (lesson tests start other test programs). It is not read.
        let nested = "running 2 tests\ntest l09_process::tests::child ... FAILED\n\
                      test l09_process::tests::other ... ok\n\nfailures:\n\n\
                      ---- l09_process::tests::child stdout ----\nrunning 1 test\n\
                      test inner ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored\n\n\
                      failures:\n    l09_process::tests::child\n\n\
                      test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n";
        let blocks = parse_test_output(nested);
        assert_eq!(
            blocks.len(),
            1,
            "the nested run is not a program of its own"
        );
        assert_eq!(blocks[0].results.len(), 2);
        assert!(blocks[0].summary);
        assert!(crash_reason(&blocks[0], Some(101), "").is_none());
        // Lines after a summary belong to nobody until the next program starts.
        let stray = "running 1 test\ntest a ... ok\n\ntest result: ok. 1 passed; 0 failed\n\
                     test stray ... ok\nrunning 1 test\ntest b ... ok\n";
        let blocks = parse_test_output(stray);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].results.len(), 1);
        assert!(blocks[0].summary && !blocks[1].summary);
    }

    // Fix 3: redo keeps every archived solution and does nothing when the
    // lesson is already the starter.
    #[test]
    fn redo_never_overwrites_an_archive() {
        let scratch = Scratch::new();
        let repo = &scratch.0;
        scratch.chapter(12, &["l01_arithmetic", "l05_capstone"]);
        let lesson = "exercises/chapter12/l05_capstone.rs";
        scratch.write(&format!(".progress/starters/{lesson}"), "// starter\n");
        scratch.write(lesson, "// my first solution\n");

        let first = repo.redo("ch12", "l05", 20_777).unwrap();
        assert!(first.contains("Saved your solution"), "{first}");
        assert_eq!(scratch.read(lesson), "// starter\n");
        let archive = ".progress/attempts/ch12_2026-11-20_l05_capstone.rs";
        assert_eq!(scratch.read(archive), "// my first solution\n");

        // The same command again, on the same day: nothing to archive, and
        // the archived solution is still there.
        let again = repo.redo("ch12", "l05", 20_777).unwrap();
        assert!(again.contains("already equals its starter"), "{again}");
        assert_eq!(scratch.read(archive), "// my first solution\n");
        assert_eq!(list_dir(&repo.state().join("attempts")).len(), 1);

        // A second solution on the same day gets its own file.
        scratch.write(lesson, "// my second solution\n");
        repo.redo("chapter12", "l05_", 20_777).unwrap();
        assert_eq!(scratch.read(archive), "// my first solution\n");
        assert_eq!(
            scratch.read(".progress/attempts/ch12_2026-11-20_l05_capstone.2.rs"),
            "// my second solution\n"
        );
        assert_eq!(scratch.read(lesson), "// starter\n");

        // The lesson may be named by its number, its module or its file.
        for spelling in [
            "l05_capstone",
            "L05_Capstone.rs",
            "exercises/chapter12/l05_capstone.rs",
        ] {
            scratch.write(lesson, "// my third solution\n");
            let saved = repo.redo("12", spelling, 20_778).unwrap();
            assert!(saved.contains("Saved your solution"), "{spelling}: {saved}");
            assert_eq!(scratch.read(lesson), "// starter\n");
        }
        assert_eq!(list_dir(&repo.state().join("attempts")).len(), 5);
        assert!(repo.redo("ch12", "l05_caps", 20_777).is_err());

        assert!(
            repo.redo("ch12", "l09", 20_777)
                .unwrap_err()
                .contains("no lesson l09")
        );
        assert!(
            repo.redo("ch12", "l01", 20_777)
                .unwrap_err()
                .contains("no saved starter")
        );
        assert!(repo.redo("ch13", "l01", 20_777).is_err());
    }

    /// Not a real test: `a_hung_program_is_killed` runs this test program
    /// again with `--ignored` to get a child that never finishes by itself.
    #[test]
    #[ignore = "helper for a_hung_program_is_killed"]
    fn helper_that_never_finishes() {
        println!("started");
        std::thread::sleep(Duration::from_secs(600));
    }

    /// A missing program gets the same plain message on every system (on
    /// Windows the error has no OS code, only "program not found").
    #[test]
    fn a_missing_program_is_reported_as_not_installed() {
        let name = "own-it-no-such-program-4711";
        let message = run_limited(&mut Command::new(name), Duration::from_secs(5))
            .err()
            .expect("the program does not exist");
        assert_eq!(message, format!("{name} is not installed, or not on PATH"));
    }

    // Fix 4: a wall-clock limit with kill, and the names of unfinished tests.
    #[test]
    fn a_hung_program_is_killed() {
        let mut hung = Command::new(std::env::current_exe().unwrap());
        hung.args(["--ignored", "--exact", "tests::helper_that_never_finishes"])
            .args(["--nocapture", "--test-threads", "1"]);
        let started = Instant::now();
        let finished = run_limited(&mut hung, Duration::from_secs(2)).unwrap();
        assert!(matches!(finished.ended, Ended::TimedOut));
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the child must be killed at the limit, not waited for"
        );
        assert!(
            finished.stdout.contains("running 1 test"),
            "{}",
            finished.stdout
        );

        let mut quick = Command::new(std::env::current_exe().unwrap());
        quick.args(["--list", "--format", "terse"]);
        let finished = run_limited(&mut quick, Duration::from_secs(60)).unwrap();
        assert!(matches!(finished.ended, Ended::Exited(status) if status.success()));
        assert!(
            finished
                .stdout
                .contains("tests::a_hung_program_is_killed: test")
        );
    }

    #[test]
    fn unfinished_tests_are_named() {
        let listing = "l01_a::tests::one: test\nl05_c::tests::loops: test\nl05_c::tests::fine: test\n\n3 tests, 0 benchmarks\n";
        let output = parse_test_output(
            "running 3 tests\ntest l01_a::tests::one ... ok\ntest l05_c::tests::fine ... ok\n",
        )
        .pop()
        .unwrap();
        assert_eq!(unfinished(listing, &output), ["l05_c::tests::loops"]);
        let problem = ChapterRun::TimedOut {
            secs: 120,
            unfinished: unfinished(listing, &output),
        };
        let line = describe_problem(&problem, "cargo test --test chapter05");
        assert!(line.contains("did not finish in 120 s"), "{line}");
        assert!(line.contains("l05_c::tests::loops"), "{line}");
        assert_eq!(seconds("COACH_NO_SUCH_VARIABLE", 7), Duration::from_secs(7));
    }

    // Fix 5: chapters, capstones and drill sets are discovered, not listed.
    #[test]
    fn chapters_capstones_and_drill_keys_are_discovered() {
        let scratch = Scratch::new();
        let repo = &scratch.0;
        scratch.chapter(1, &["l01_first_test", "l06_capstone"]);
        scratch.chapter(
            4,
            &[
                "l01_scope",
                "l08_capstone",
                "l09_extra",
                "r01_review",
                "c01_code_review",
            ],
        );
        scratch.chapter(24, &["l01_ffi", "l02_more"]);
        // A directory without a Cargo test target is not a chapter yet.
        scratch.write("exercises/chapter07/mod.rs", "");
        scratch.write("exercises/chapter04/NOTES.md", "");
        assert_eq!(repo.chapters(), [1, 4, 24]);
        assert_eq!(repo.chapter("ch01"), Ok(1));
        assert_eq!(repo.chapter("chapter24"), Ok(24));
        assert!(repo.chapter("ch07").is_err());
        assert!(repo.chapter("ch25").is_err());

        let modules: Vec<String> = repo.lesson_files(4).into_iter().map(|(m, _)| m).collect();
        assert_eq!(
            modules,
            [
                "l01_scope",
                "l09_extra",
                "l08_capstone",
                "r01_review",
                "c01_code_review"
            ],
            "study order: lessons by number, the capstone last even when a later lesson was added"
        );
        // The capstone is found by name, even when it is not the last lesson.
        assert_eq!(repo.capstone(4).as_deref(), Some("l08_capstone"));
        assert_eq!(repo.capstone(1).as_deref(), Some("l06_capstone"));
        assert_eq!(repo.capstone(24).as_deref(), Some("l02_more"));

        for key in ["ch04", "ch10", "m1", "m10", "m2"] {
            scratch.write(&format!("drills/{key}/d01_x.rs"), "fn main() {}\n");
        }
        fs::create_dir_all(repo.root.join("drills").join("empty")).unwrap();
        assert_eq!(repo.drill_keys(), ["ch04", "ch10", "m1", "m2", "m10"]);
        assert_eq!(repo.drill_key("m1").as_deref(), Ok("m1"));
        assert_eq!(repo.drill_key("M10").as_deref(), Ok("m10"));
        assert_eq!(repo.drill_key("ch4").as_deref(), Ok("ch04"));
        assert_eq!(repo.drill_key("chapter10").as_deref(), Ok("ch10"));
        assert!(
            repo.drill_key("m3")
                .unwrap_err()
                .contains("ch04 ch10 m1 m2 m10")
        );
        assert_eq!(repo.drill_files("m1"), ["drills/m1/d01_x.rs"]);
        // A set may be spelled as its directory, as tab completion does.
        for spelling in ["m1", "drills/m1", "drills/m1/", "./drills/M1"] {
            assert!(
                repo.drill_target(spelling, &repo.root).is_ok(),
                "{spelling}"
            );
        }
        assert!(repo.drill_target("drills/nope/", &repo.root).is_err());
    }

    // Fixes 1 and 6, end to end with the real compiler: drills are built
    // under target/drills, the executable is removed, and one drill has one
    // record and one never-overwritten original however its path is spelled.
    #[test]
    fn drills_build_under_target_and_keep_their_original() {
        let scratch = Scratch::new();
        let repo = &scratch.0;
        let key = "drills/m1/d01_move.rs";
        let broken = "//! EXPECT: E0382\nfn main() {\n    let a = String::from(\"x\");\n    let b = a;\n    \
                      assert_eq!(a.len() + b.len(), 2); // KEEP\n}\n";
        scratch.write(key, broken);

        let run = repo.compile_and_run(key).unwrap();
        assert_eq!(run.outcome, errors(&["E0382"]));
        scratch.write(
            "drills/m1/d02_ok.rs",
            "fn main() { println!(\"hello\"); }\n",
        );
        let run = repo.compile_and_run("drills/m1/d02_ok.rs").unwrap();
        assert_eq!(run.outcome, Outcome::Ok);
        assert_eq!(run.output, "hello\n");
        scratch.write("drills/m1/d03_panic.rs", "fn main() { panic!(\"no\"); }\n");
        assert_eq!(
            repo.compile_and_run("drills/m1/d03_panic.rs")
                .unwrap()
                .outcome,
            Outcome::Panic
        );
        let build_dir = repo.target.join("drills");
        assert!(
            build_dir.starts_with(&repo.root),
            "drills are built inside the repository"
        );
        assert_eq!(
            list_dir(&build_dir),
            Vec::<PathBuf>::new(),
            "executables are removed after the run"
        );

        // First step: judged, recorded, original saved.
        assert_eq!(repo.step_drill(key, false), Ok(DrillState::NeedsFix));
        let original = ".progress/originals/drills/m1/d01_move.rs";
        assert_eq!(scratch.read(original), broken);
        assert_eq!(repo.load_records().unwrap()[key].verdict, "right");

        // Fix the drill, then reach it through other spellings of its path.
        scratch.write(key, &broken.replace("let b = a;", "let b = a.clone();"));
        let inside = repo.root.join("drills").join("m1");
        for (spelling, cwd) in [
            ("./drills/m1/d01_move.rs", &repo.root),
            ("d01_move.rs", &inside),
        ] {
            let resolved = repo.resolve_drill(spelling, cwd).unwrap();
            assert_eq!(repo.step_drill(&resolved, false), Ok(DrillState::Done));
        }
        assert_eq!(
            repo.load_records().unwrap().len(),
            1,
            "one drill, one record"
        );
        assert_eq!(
            scratch.read(original),
            broken,
            "the original is never overwritten"
        );

        // A lost record does not turn the fixed program into the original:
        // the saved original is judged, and reset still restores the drill.
        fs::remove_file(repo.records_path()).unwrap();
        assert_eq!(repo.step_drill(key, false), Ok(DrillState::Done));
        assert_eq!(repo.load_records().unwrap()[key].actual, "E0382");
        assert_eq!(scratch.read(original), broken);
        let message = repo.drill_reset("m1").unwrap();
        assert!(message.contains("Restored 1 drills in m1"), "{message}");
        assert_eq!(
            scratch.read(key),
            broken.replace("EXPECT: E0382", "EXPECT: ?")
        );
        assert!(repo.load_records().unwrap().is_empty());
        assert_eq!(list_dir(&build_dir), Vec::<PathBuf>::new());

        // A prediction the coach cannot read is not recorded as wrong.
        scratch.write(key, &broken.replace("E0382", "it breaks somehow"));
        assert_eq!(repo.step_drill(key, false), Ok(DrillState::NeedsPrediction));
        assert!(repo.load_records().unwrap().is_empty());
    }

    // Fix 7: reviews are short, different each time, and keep their spacing.
    #[test]
    fn reviews_are_spaced_and_different() {
        let scratch = Scratch::new();
        let repo = &scratch.0;
        scratch.chapter(
            4,
            &["l01_scope", "l08_capstone", "r01_review", "c01_code_review"],
        );
        scratch.chapter(3, &["l01_inference", "l13_capstone", "c01_code_review"]);
        scratch.write(
            ".progress/drills.tsv",
            "drills/ch04/d01_a.rs\tE0382\tE0382\tright\n\
             drills/ch04/d02_b.rs\tok\tE0507\twrong\n\
             drills/ch04/d07_c.rs\terror\tE0596\tpartly right\n\
             drills/ch05/d01_z.rs\tok\tpanic\twrong\n",
        );
        let plan = |chapter, n| repo.review_plan(chapter, n).unwrap().join("\n");

        let first = plan(4, 0);
        assert!(first.contains("ch04, review 1 of 5"), "{first}");
        assert!(first.contains("Predictions I got wrong"), "{first}");
        assert!(first.contains("d02_b.rs, d07_c.rs"), "{first}");
        assert!(
            !first.contains("d01_a.rs") && !first.contains("d01_z.rs"),
            "{first}"
        );
        assert!(plan(3, 0).contains("You missed no ch03 drill"));
        assert!(plan(4, 1).contains("Mastery check"));
        let third = plan(4, 2);
        assert!(third.contains("cargo progress redo ch04 l08"), "{third}");
        assert!(
            third.contains("cargo test --test chapter04 l08_"),
            "{third}"
        );
        assert!(plan(3, 2).contains("cargo progress redo ch03 l13"));
        assert!(plan(4, 3).contains("cargo drill reset ch04"));
        assert!(plan(4, 4).contains("cargo progress redo ch04 r01"));
        assert!(
            plan(3, 4).contains("cargo progress redo ch03 c01"),
            "no r01 in chapter 3"
        );
        for n in 0..5 {
            assert!(plan(4, n).contains("cargo progress reviewed ch04"));
        }

        // The schedule: 1, 3, 7, 21 and 60 days, each after the one before.
        let day = 20_000;
        assert!(
            repo.reviewed("ch04", false, day)
                .unwrap_err()
                .contains("not complete yet")
        );
        scratch.write(".progress/chapters.tsv", &format!("4\t{day}\t0\t{day}\n"));
        let review = |now| {
            repo.load_reviews().unwrap()[&4]
                .due_day()
                .map(|due| (due, now >= due))
        };
        assert_eq!(review(day), Some((day + 1, false)));

        // Not due yet: nothing is recorded, so the command cannot use up the schedule.
        let early = repo.reviewed("ch04", false, day).unwrap();
        assert!(
            early.contains("Nothing recorded") && early.contains("--early"),
            "{early}"
        );
        assert_eq!(repo.load_reviews().unwrap()[&4].done, 0);

        let mut now = day;
        for (index, wait) in REVIEW_DAYS.iter().enumerate() {
            now += wait;
            let message = repo.reviewed("ch04", false, now).unwrap();
            assert!(
                message.contains(&format!("Recorded review {} of 5", index + 1)),
                "{message}"
            );
            let entry = repo.load_reviews().unwrap()[&4];
            assert_eq!(
                (entry.completed, entry.done, entry.last),
                (day, index + 1, now)
            );
        }
        assert_eq!(repo.load_reviews().unwrap()[&4].due_day(), None);
        assert!(
            repo.reviewed("ch04", false, now + 100)
                .unwrap()
                .contains("finished its review schedule")
        );

        // `--early` records a review before its day.
        scratch.write(".progress/chapters.tsv", &format!("4\t{day}\t1\t{day}\n"));
        assert!(
            repo.reviewed("ch04", true, day)
                .unwrap()
                .contains("Recorded review 2 of 5")
        );
        assert_eq!(repo.load_reviews().unwrap()[&4].due_day(), Some(day + 7));
    }

    #[test]
    fn the_review_report_explains_what_is_due() {
        let scratch = Scratch::new();
        let repo = &scratch.0;
        for chapter in [3, 4, 5] {
            scratch.chapter(chapter, &["l01_a", "l02_capstone"]);
        }
        let day = 20_000;
        let entry = |done, last| Review {
            completed: day,
            done,
            last,
        };
        let reviews = BTreeMap::from([
            (3, entry(2, day - 7)),
            (4, entry(0, day - 3)),
            (5, entry(1, day - 3)),
        ]);
        let report = repo.review_report(&reviews, day).unwrap().join("\n");
        // The two oldest are explained; the third waits.
        assert!(report.contains("ch03, review 3 of 5"), "{report}");
        assert!(report.contains("ch04, review 1 of 5"), "{report}");
        assert!(
            report.contains("Also due, for another day: ch05"),
            "{report}"
        );
        assert!(report.contains("1, 3, 7, 21 and 60 days"), "{report}");

        let reviews = BTreeMap::from([(4, entry(1, day))]);
        let report = repo.review_report(&reviews, day).unwrap().join("\n");
        assert_eq!(
            report,
            format!(
                "Next review: ch04 on {} (review 2 of 5: mastery questions). Dates are UTC.",
                date(day + 3)
            )
        );
        // An absurd stored day must not overflow.
        let reviews = BTreeMap::from([(4, entry(0, u64::MAX))]);
        assert!(repo.review_report(&reviews, day).is_ok());
    }

    #[test]
    fn unreadable_state_is_reported_not_erased() {
        let scratch = Scratch::new();
        let repo = &scratch.0;
        scratch.write(
            ".progress/chapters.tsv",
            "4\t20000\t1\t20001\nnot a record\n",
        );
        let error = repo.load_reviews().unwrap_err();
        assert!(
            error.contains("chapters.tsv") && error.contains("line 2"),
            "{error}"
        );
        scratch.write(".progress/drills.tsv", "drills/ch04/d01.rs\tok\n");
        assert!(repo.load_records().unwrap_err().contains("line 1"));
        // A missing file is simply an empty history.
        fs::remove_file(repo.reviews_path()).unwrap();
        assert!(repo.load_reviews().unwrap().is_empty());
    }

    // Checkpoints and the final assessment have no hints: their reminder
    // sends the learner to the rubric, not to HINTS.md.
    #[test]
    fn assessment_solutions_point_at_the_rubric_not_at_hints() {
        let scratch = Scratch::new();
        let repo = &scratch.0;
        scratch.write("solutions/checkpoints/cp1_after_ch07/src/lib.rs", "");
        scratch.write("solutions/checkpoints/cp1_after_ch07/tests/hidden.rs", "");
        scratch.write("solutions/capstones/final_assessment/src/lib.rs", "");
        scratch.write("solutions/capstones/game/src/world.rs", "");
        scratch.write("solutions/exercises/chapter04/l01_scope.rs", "");
        let rubric = "Read this only after you have graded your own work against RUBRIC.md.";
        for (unit, item) in [
            ("cp1", None),
            ("cp1", Some("lib")),
            ("checkpoints/cp1_after_ch07", None),
            ("final", None),
            ("final_assessment", Some("lib")),
            ("cap_final_assessment", None),
        ] {
            let shown = repo.solution(unit, item).unwrap();
            assert!(shown.contains(rubric), "{unit} {item:?}: {shown}");
            assert!(
                !shown.contains("HINTS.md"),
                "{unit} {item:?} has no hints, so the reminder must not mention them: {shown}"
            );
        }
        for (unit, item) in [
            ("ch04", Some("l01")),
            ("game", Some("world")),
            ("ch04", None),
        ] {
            let shown = repo.solution(unit, item).unwrap();
            assert!(shown.contains("HINTS.md"), "{unit} {item:?}: {shown}");
            assert!(!shown.contains(rubric), "{unit} {item:?}: {shown}");
        }
    }

    // Fix 8: the solution command points at the file and asks for an attempt.
    #[test]
    fn solution_prints_the_path_and_a_reminder() {
        let scratch = Scratch::new();
        let repo = &scratch.0;
        let lesson = scratch.write("solutions/exercises/chapter04/l01_scope.rs", "");
        scratch.write("solutions/exercises/chapter04/l08_capstone.rs", "");
        let drill = scratch.write("solutions/drills/ch04/d01_use_after_move.rs", "");
        let mastery = scratch.write("solutions/mastery/m1_types/src/l01_variance.rs", "");
        scratch.write("solutions/drills/m1/d01_covariance.rs", "");
        let project = scratch.write("solutions/project/src/record.rs", "");
        let capstone = scratch.write("solutions/capstones/game/src/world.rs", "");
        let checkpoint = scratch.write("solutions/checkpoints/cp1_after_ch07/src/lib.rs", "");

        let shown = repo.solution("ch04", Some("l01")).unwrap();
        assert!(shown.contains(&lesson.display().to_string()), "{shown}");
        assert!(shown.contains("Attempt the task first"), "{shown}");
        assert!(!shown.contains("l08_capstone"), "{shown}");
        assert!(
            repo.solution("chapter04", Some("d01"))
                .unwrap()
                .contains(&drill.display().to_string())
        );
        assert!(
            repo.solution("m1", Some("l01"))
                .unwrap()
                .contains(&mastery.display().to_string())
        );
        assert!(
            repo.solution("m1", Some("d01"))
                .unwrap()
                .contains("d01_covariance.rs")
        );
        assert!(
            repo.solution("project", Some("record"))
                .unwrap()
                .contains(&project.display().to_string())
        );
        for unit in ["cap_game", "game", "capstones/game"] {
            assert!(
                repo.solution(unit, Some("world"))
                    .unwrap()
                    .contains(&capstone.display().to_string()),
                "{unit} names the game capstone"
            );
        }
        assert!(
            repo.solution("cp1", None)
                .unwrap()
                .contains(&checkpoint.display().to_string()),
            "cp1 names checkpoint 1"
        );
        let listed = repo.solution("ch04", None).unwrap();
        assert!(
            listed.contains("l08_capstone.rs") && listed.contains("d01_use_after_move.rs"),
            "{listed}"
        );
        assert!(
            repo.solution("ch04", Some("l05"))
                .unwrap_err()
                .contains("no model solution for ch04 l05")
        );
        assert!(
            repo.solution("ch09", Some("l01"))
                .unwrap_err()
                .contains("no model solutions for ch09")
        );
        assert!(
            repo.solution("mastery/m1_types/", Some("l01_variance.rs"))
                .unwrap()
                .contains(&mastery.display().to_string())
        );
        // A unit is never a path out of solutions/.
        scratch.write("secret.rs", "");
        let outside = std::env::temp_dir();
        for unit in [
            "..",
            ".",
            "../drills",
            "solutions/..",
            "",
            "/",
            outside.to_str().unwrap(),
        ] {
            let refused = repo.solution(unit, None).unwrap_err();
            assert!(refused.contains("expected a unit"), "{unit}: {refused}");
        }
    }

    #[test]
    fn cargo_json_gives_the_chapter_executables() {
        assert_eq!(
            json_string(
                r#"{"a":"x","executable":"C:\\dir\\a b\\t.exe"}"#,
                "executable"
            )
            .as_deref(),
            Some("C:\\dir\\a b\\t.exe")
        );
        assert_eq!(json_string(r#"{"executable":null}"#, "executable"), None);
        assert_eq!(
            json_string(r#"{"name":"caf\u00e9 \"x\""}"#, "name").as_deref(),
            Some("café \"x\"")
        );
        let json = concat!(
            r#"{"reason":"compiler-artifact","package_id":"path+file:///r#own_it@0.1.0","target":{"kind":["lib"],"name":"practice","src_path":"/r/src/lib.rs"},"filenames":["/t/libpractice.rlib"],"executable":null,"fresh":true}"#,
            "\n",
            r#"{"reason":"compiler-artifact","package_id":"path+file:///r#own_it@0.1.0","target":{"kind":["test"],"name":"chapter04","src_path":"/r/exercises/chapter04/mod.rs"},"filenames":["/t/chapter04-1a2b"],"executable":"/t/my dir/chapter04-1a2b","fresh":true}"#,
            "\n",
            r#"{"reason":"compiler-message","message":{"rendered":"error: \"executable\":\"/fake\""}}"#,
            "\n",
            r#"{"reason":"build-finished","success":false}"#,
        );
        let executables = chapter_executables(json);
        assert_eq!(
            executables,
            BTreeMap::from([(4, PathBuf::from("/t/my dir/chapter04-1a2b"))])
        );
    }

    // Fix 8: `cargo progress all` rows for the standalone packages.
    #[test]
    fn package_rows_are_discovered_and_grouped() {
        let scratch = Scratch::new();
        let repo = &scratch.0;
        for package in [
            "project",
            "mastery/m10_api",
            "mastery/m2_unsafe",
            "capstones/game",
            "checkpoints/one",
        ] {
            scratch.write(&format!("{package}/Cargo.toml"), "[package]\n");
        }
        fs::create_dir_all(repo.root.join("mastery").join("notes")).unwrap();
        assert_eq!(
            repo.packages(),
            [
                "project",
                "checkpoints/one",
                "mastery/m2_unsafe",
                "mastery/m10_api",
                "capstones/game"
            ],
            "course order: checkpoints, mastery modules (m2 before m10), capstones"
        );

        let stderr = "   Compiling m1 v0.1.0\n     Running unittests src/lib.rs (target/debug/deps/m1-0a)\n     \
                      Running tests/m03_types.rs (target/debug/deps/m03_types-0b)\n     \
                      Running tests/m04_crash.rs (target/debug/deps/m04_crash-0c)\n   Doc-tests m1\n";
        let stdout = "running 3 tests\ntest l01_variance::tests::a ... ok\ntest l01_variance::tests::b ... ok\n\
                      test l02_bounds::tests::a ... FAILED\n\ntest result: FAILED. 2 passed; 1 failed\n\n\
                      running 2 tests\ntest a_parses ... ok\ntest p_predictions ... FAILED\n\n\
                      test result: FAILED. 1 passed; 1 failed\n\n\
                      running 2 tests\ntest first ... ok\n\n\
                      running 1 test\ntest src/lib.rs - guess (line 5) ... ok\n\ntest result: ok. 1 passed\n\n\
                      running 1 test\ntest src/support.rs - support::within (line 9) - compile fail ... ok\n\n\
                      test result: ok. 1 passed\n";
        let groups: Vec<(String, Group)> = package_groups(stdout, stderr).into_values().collect();
        let group = |passed, failed, crashed| Group {
            passed,
            failed,
            crashed,
        };
        assert_eq!(
            groups,
            [
                // Both doc-test blocks, also the one without a line in stderr.
                ("doc".to_string(), group(2, 0, false)),
                ("l01".to_string(), group(2, 0, false)),
                ("l02".to_string(), group(0, 1, false)),
                ("m03".to_string(), group(1, 1, false)),
                // No `test result:` line: this program crashed.
                ("m04".to_string(), group(1, 0, true)),
            ]
        );
    }

    #[test]
    fn doctor_reads_versions_and_the_pinned_channel() {
        assert_eq!(
            tool_version("rustc 1.99.0 (b940084d7 2026-09-28)\n"),
            Some("1.99.0")
        );
        assert_eq!(
            tool_version("cargo 1.99.0 (5f94df478 2026-08-27)"),
            Some("1.99.0")
        );
        assert_eq!(
            tool_version("rustc 1.102.0-nightly (6905905bb 2026-11-19)"),
            Some("1.102.0-nightly")
        );
        assert_eq!(tool_version(""), None);
        assert_eq!(tool_version("rustc"), None);
        let file = "[toolchain]\nchannel = \"1.99.0\"\nprofile = \"minimal\"\n";
        assert_eq!(toolchain_channel(file).as_deref(), Some("1.99.0"));
        assert_eq!(
            toolchain_channel("channel=\"1.99.0\""),
            Some("1.99.0".into())
        );
        assert_eq!(
            toolchain_channel("[toolchain]\nprofile = \"minimal\"\n"),
            None
        );
        assert_eq!(toolchain_channel("channel = \"\"\n"), None);
    }

    #[test]
    fn doctor_finds_the_environment() {
        let env = |pairs: &[(&str, &str)]| {
            let map: BTreeMap<String, String> = pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            move |name: &str| map.get(name).cloned()
        };
        assert_eq!(place_from(env(&[]), false), Place::OwnComputer);
        assert_eq!(
            place_from(env(&[("CODESPACES", "true")]), false),
            Place::Codespaces
        );
        // A codespace is also a container; Codespaces wins.
        assert_eq!(
            place_from(
                env(&[("CODESPACES", "true"), ("REMOTE_CONTAINERS", "true")]),
                true
            ),
            Place::Codespaces
        );
        assert_eq!(
            place_from(env(&[("REMOTE_CONTAINERS", "true")]), false),
            Place::DevContainer
        );
        assert_eq!(
            place_from(env(&[("DEVCONTAINER", "true")]), false),
            Place::DevContainer
        );
        // A marker file alone (toolbox, distrobox, another image) is not
        // the course dev container.
        assert_eq!(place_from(env(&[]), true), Place::OtherContainer);
        assert_eq!(
            place_from(env(&[("DEVCONTAINER", "true")]), true),
            Place::DevContainer
        );
        assert_eq!(
            place_from(env(&[("CODESPACES", "false")]), false),
            Place::OwnComputer
        );
        assert_eq!(Os::from_name("macos"), Os::Mac);
        assert_eq!(Os::from_name("windows"), Os::Windows);
        assert_eq!(Os::from_name("linux"), Os::Linux);
        assert_eq!(Os::Windows.pick("m", "l", "w"), "w");
    }

    #[test]
    fn doctor_classifies_results() {
        assert_eq!(Status::classify(Need::Required, true), Status::Found);
        assert_eq!(Status::classify(Need::Optional, true), Status::Found);
        assert_eq!(
            Status::classify(Need::Required, false),
            Status::MissingRequired
        );
        assert_eq!(
            Status::classify(Need::Optional, false),
            Status::MissingOptional
        );
        let check = |need, found: Result<&str, &str>| Check {
            name: "x",
            need,
            purpose: "",
            found: found.map(str::to_string).map_err(str::to_string),
            fix: String::new(),
        };
        assert_eq!(
            check(Need::Optional, Err("gone")).status(),
            Status::MissingOptional
        );
        assert_eq!(check(Need::Required, Ok("1.99.0")).status(), Status::Found);

        let toolchains = "stable-aarch64-apple-darwin (default)\n\
                          nightly-aarch64-apple-darwin\n\
                          nightly-2026-11-20-aarch64-apple-darwin\n\
                          1.99.0-aarch64-apple-darwin (active)\n";
        assert!(lists_toolchain(toolchains, "nightly-2026-11-20"));
        assert!(lists_toolchain(toolchains, "1.99.0"));
        assert!(!lists_toolchain(toolchains, "nightly-2026-11-21"));
        assert!(!lists_toolchain(
            "nightly-aarch64-apple-darwin\n",
            "nightly-2026-11-20"
        ));
        let components = "cargo-aarch64-apple-darwin\nmiri-aarch64-apple-darwin\nrust-src\n";
        assert!(lists_component(components, "miri"));
        assert!(lists_component(components, "rust-src"));
        assert!(!lists_component("cargo-x\nrust-src\n", "miri"));

        assert_eq!(
            first_line("\n  git version 2.51.0\nmore\n"),
            "git version 2.51.0"
        );
        assert_eq!(
            first_line(&"a".repeat(70)),
            format!("{}...", "a".repeat(60))
        );
    }
}
