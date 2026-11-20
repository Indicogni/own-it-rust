//! Support code for the lessons. You do not need to edit this file.
//!
//! A library is judged from the outside: by the programs that other people
//! write against it. Many checks in this module are therefore not ordinary
//! unit tests. They are small programs that are compiled against a library
//! (a fixture, or your lesson file) in the same way a user would compile them.
//!
//! - [`guess`] marks a prediction you have not written yet.
//! - [`library_from_file`] and [`library_from_text`] compile a library crate.
//! - [`program`] compiles and runs a program against such libraries and
//!   reports the outcome as a word: `ok`, `panic`, `error`, or the error
//!   code(s) of the compiler, for example `E0063`.
//! - [`case`] checks a file whose first lines say which outcome is expected.
//! - [`cargo`] runs a second `cargo` on one of the packages in `fixtures/`.
//! - [`implements!`](crate::implements) answers "does this type implement
//!   this trait?" with a `bool`.
//! - [`assert_answer`] compares a written answer with the model answer
//!   without showing the model answer in the test.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

/// Placeholder for a prediction. Replace the whole call (`crate::guess()`)
/// with your answer before running the lesson, for example
/// `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

/// `implements!(Type: Bound)` is `true` when the concrete type `Type`
/// satisfies `Bound`, and `false` otherwise.
///
/// A bound that is not met is normally a compile error, and a test cannot
/// describe a compile error. This macro turns the question into a value.
///
/// How it works: `Probe<T>` has an inherent associated constant `YES` that
/// exists only when `T` meets the bound, and a trait constant `YES` that
/// always exists and is `false`. The path `<Probe<T>>::YES` prefers the
/// inherent constant when there is one. The type must be written out in full;
/// the macro cannot answer the question for a generic parameter.
#[macro_export]
macro_rules! implements {
    ($ty:ty : $($bound:tt)+) => {{
        struct Probe<T: ?Sized>(::core::marker::PhantomData<T>);
        trait Fallback {
            const YES: bool = false;
        }
        impl<T: ?Sized> Fallback for Probe<T> {}
        #[allow(dead_code)]
        impl<T: ?Sized + $($bound)+> Probe<T> {
            const YES: bool = true;
        }
        <Probe<$ty>>::YES
    }};
}

// ------------------------------------------------------------ written answers

/// A 64-bit FNV-1a digest of `answer`, ignoring case, surrounding white space,
/// and the amount of white space between words.
pub fn digest(answer: &str) -> u64 {
    let normalised = answer
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in normalised.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Fails the test unless the digest of `question` together with `answer` is
/// `expected`. The model answer is not in the test; only its digest is.
/// `hint` is shown when the answer is wrong.
#[track_caller]
pub fn assert_answer(question: &str, answer: &str, expected: u64, hint: &str) {
    assert!(
        digest(&format!("{question}: {answer}")) == expected,
        "{question}: `{answer}` is not the model answer. {hint}"
    );
}

// ------------------------------------------------------------------- files

/// The directory of this package (`mastery/m8_api`).
pub fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Reads a text file below the package directory. Line endings are normalised
/// to `\n`, so that the checks behave the same on every platform.
#[track_caller]
pub fn read(relative: &str) -> String {
    let path = package_dir().join(relative);
    match std::fs::read_to_string(&path) {
        Ok(text) => text.replace("\r\n", "\n"),
        Err(error) => panic!("cannot read {}: {error}", path.display()),
    }
}

/// The value of a `//@ name: value` or `//! NAME: value` line of a source
/// text, if there is one.
pub fn field(source: &str, name: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let line = line.trim_start();
        let rest = line
            .strip_prefix("//@ ")
            .or_else(|| line.strip_prefix("//! "))?;
        Some(rest.strip_prefix(name)?.trim().to_string())
    })
}

/// Where build output of the checks goes: a directory next to the test
/// program, inside Cargo's target directory. If that place cannot be found,
/// the system's temporary directory is used.
fn scratch_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let beside_tests = std::env::current_exe().ok().and_then(|exe| {
            let deps = exe.parent()?;
            let profile = deps.parent()?;
            (deps.file_name()? == "deps").then(|| profile.join("m8_api_scratch"))
        });
        let root = beside_tests.unwrap_or_else(|| std::env::temp_dir().join("m8_api_scratch"));
        std::fs::create_dir_all(&root)
            .unwrap_or_else(|e| panic!("cannot create {}: {e}", root.display()));
        root
    })
}

/// A private directory for this test process. Directories that earlier test
/// runs left behind are removed when they are more than 15 minutes old.
fn process_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let root = scratch_root();
        let stale = Duration::from_secs(15 * 60);
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let old = entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|time| SystemTime::now().duration_since(time).ok())
                    .is_some_and(|age| age > stale);
                if old && entry.file_name().to_string_lossy().starts_with("run-") {
                    let _ = std::fs::remove_dir_all(entry.path());
                }
            }
        }
        let dir = root.join(format!("run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("cannot create {}: {e}", dir.display()));
        dir
    })
}

/// A new, empty directory inside [`process_dir`]. `label` makes its name
/// readable.
fn fresh_dir(label: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let safe: String = label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let dir = process_dir().join(format!(
        "{:03}-{safe}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", dir.display()));
    dir
}

// --------------------------------------------------------------- processes

/// How long one compiler run, one program or one cargo command may take.
const LIMIT: Duration = Duration::from_secs(300);

/// Runs a command to its end, or stops it after [`LIMIT`], so that a check can
/// fail with a message but never hang.
fn run_limited(command: &mut Command, what: &str) -> Output {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot start {what}: {e}. Is it on your PATH?"));
    // Read both pipes on helper threads, so that a full pipe never blocks the child.
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let read_out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let read_err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait().expect("waiting for a child process") {
            Some(status) => break status,
            None if started.elapsed() > LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "{what} was still running after {} s and was stopped. \
                     Look for a loop that never ends.",
                    LIMIT.as_secs()
                );
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    Output {
        status,
        stdout: read_out.join().unwrap_or_default(),
        stderr: read_err.join().unwrap_or_default(),
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

/// Shortens long tool output for a failure message: the first `lines` lines.
pub fn head(output: &str, lines: usize) -> String {
    let all: Vec<&str> = output.trim_end().lines().collect();
    if all.len() <= lines {
        return all.join("\n");
    }
    format!(
        "{}\n... ({} more lines)",
        all[..lines].join("\n"),
        all.len() - lines
    )
}

// ----------------------------------------------------------------- rustc

/// How to compile one crate with `rustc`.
#[derive(Clone, Copy)]
pub struct Build<'a> {
    /// The edition. The lessons use 2024 unless they say otherwise.
    pub edition: &'a str,
    /// Libraries the crate may name: `(crate name, path of the .rlib)`.
    pub externs: &'a [(&'a str, &'a Path)],
    /// Extra compiler flags, for example `["--cfg", "mutant"]`.
    pub flags: &'a [&'a str],
    /// `false` passes `-A warnings`, so that only errors are reported.
    pub warnings: bool,
}

impl Default for Build<'_> {
    fn default() -> Self {
        Build {
            edition: "2024",
            externs: &[],
            flags: &[],
            warnings: false,
        }
    }
}

fn rustc(build: &Build<'_>) -> Command {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let mut command = Command::new(rustc);
    // Run from the package directory, so that rust-toolchain.toml applies.
    command
        .current_dir(package_dir())
        .args(["--edition", build.edition, "--color", "never"])
        .args(["-C", "debuginfo=0"]);
    if !build.warnings {
        command.args(["-A", "warnings"]);
    }
    command.args(build.flags);
    for (name, rlib) in build.externs {
        let mut value = OsString::from(format!("{name}="));
        value.push(rlib.as_os_str());
        command.arg("--extern").arg(value);
        if let Some(dir) = rlib.parent() {
            let mut search = OsString::from("dependency=");
            search.push(dir.as_os_str());
            command.arg("-L").arg(search);
        }
    }
    command
}

/// Error codes in a compiler message, sorted and without repeats.
pub fn error_codes(diagnostics: &str) -> Vec<String> {
    let mut codes: Vec<String> = diagnostics
        .lines()
        .filter_map(|line| line.strip_prefix("error[")?.split(']').next())
        .map(str::to_string)
        .collect();
    codes.sort();
    codes.dedup();
    codes
}

/// Compiles the file `source` as the library crate `name`. Returns the path
/// of the `.rlib`, or the compiler's messages when the library does not
/// compile.
pub fn library(name: &str, source: &Path, build: &Build<'_>) -> Result<PathBuf, String> {
    let out_dir = fresh_dir(&format!("lib-{name}"));
    let mut command = rustc(build);
    command
        .args(["--crate-type", "rlib", "--crate-name", name])
        .arg("--out-dir")
        .arg(&out_dir)
        .arg(source);
    let output = run_limited(&mut command, "rustc");
    if output.status.success() {
        Ok(out_dir.join(format!("lib{name}.rlib")))
    } else {
        Err(text(&output.stderr))
    }
}

/// Compiles a file of this package (a path below the package directory, for
/// example `src/l04_errors.rs`) as the library crate `name`, with the default
/// [`Build`]. The result is remembered, so that every check of one test run
/// uses the same build.
#[track_caller]
pub fn library_from_file(name: &str, relative: &str) -> PathBuf {
    static BUILT: OnceLock<Mutex<HashMap<String, Result<PathBuf, String>>>> = OnceLock::new();
    let key = format!("{name}={relative}");
    let built = {
        let mut cache = BUILT
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache
            .entry(key)
            .or_insert_with(|| library(name, Path::new(relative), &Build::default()))
            .clone()
    };
    match built {
        Ok(rlib) => rlib,
        Err(errors) => panic!(
            "{relative} does not compile as the library `{name}`, so no check can run:\n{}",
            head(&errors, 60)
        ),
    }
}

/// Writes `source` to a file and compiles it as the library crate `name`.
pub fn library_from_text(name: &str, source: &str, build: &Build<'_>) -> Result<PathBuf, String> {
    let dir = fresh_dir(&format!("src-{name}"));
    let file = dir.join(format!("{name}.rs"));
    std::fs::write(&file, source)
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", file.display()));
    library(name, &file, build)
}

/// What happened when a program was compiled and run.
#[derive(Debug, Clone)]
pub struct Run {
    /// `ok` (compiled, ran, exit status 0), `panic` (compiled, but the run
    /// failed), `error` (a compile error without an error code), or the error
    /// code(s) of the compiler joined by commas, for example `E0063`.
    pub outcome: String,
    /// The compiler's messages, or what the program printed when it failed.
    pub output: String,
    /// The compiler's warnings, when [`Build::warnings`] was `true` and the
    /// program compiled.
    pub warnings: String,
}

/// Compiles the program in the file `source` and, when it compiles, runs it.
/// `label` names the check in file names and messages.
pub fn program(label: &str, source: &Path, build: &Build<'_>) -> Run {
    let out_dir = fresh_dir(label);
    let exe = out_dir.join(format!("program{}", std::env::consts::EXE_SUFFIX));
    let mut command = rustc(build);
    command
        .args(["--crate-type", "bin", "--crate-name", "program"])
        .arg("-o")
        .arg(&exe)
        .arg(source);
    let compiled = run_limited(&mut command, "rustc");
    let diagnostics = text(&compiled.stderr);
    if !compiled.status.success() {
        let codes = error_codes(&diagnostics);
        let outcome = if codes.is_empty() {
            assert!(
                diagnostics.contains("error"),
                "rustc failed without a diagnostic for {label}:\n{diagnostics}"
            );
            "error".to_string()
        } else {
            codes.join(",")
        };
        return Run {
            outcome,
            output: diagnostics,
            warnings: String::new(),
        };
    }
    let ran = run_limited(Command::new(&exe).current_dir(&out_dir), label);
    Run {
        outcome: if ran.status.success() { "ok" } else { "panic" }.to_string(),
        output: format!("{}{}", text(&ran.stdout), text(&ran.stderr)),
        warnings: diagnostics,
    }
}

/// Like [`program`], for a program given as text.
pub fn program_from_text(label: &str, source: &str, build: &Build<'_>) -> Run {
    let dir = fresh_dir(&format!("src-{label}"));
    let file = dir.join("main.rs");
    std::fs::write(&file, source)
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", file.display()));
    program(label, &file, build)
}

/// Whether two outcome words mean the same. Error codes are compared as sets;
/// case and white space do not matter.
pub fn same_outcome(a: &str, b: &str) -> bool {
    let normal = |text: &str| {
        let mut parts: Vec<String> = text
            .split(',')
            .map(|part| part.trim().to_lowercase())
            .filter(|part| !part.is_empty())
            .collect();
        parts.sort();
        parts.dedup();
        parts
    };
    normal(a) == normal(b)
}

/// Checks one case file (a path below the package directory). Its first lines
/// say what must happen and why:
///
/// ```text
/// //@ expect: run        (or an error code such as E0451, or `error`)
/// //@ why: one sentence that says what the check means
/// ```
///
/// `run` means: the program compiles, runs, and exits normally. Anything else
/// means: the compiler must reject the program with exactly this outcome.
/// When two correct designs are rejected in different ways, the line lists
/// both outcomes, as in `//@ expect: E0616 or E0609`.
#[track_caller]
pub fn case(relative: &str, externs: &[(&str, &Path)]) {
    let source = read(relative);
    let expect =
        field(&source, "expect:").unwrap_or_else(|| panic!("{relative}: no `//@ expect:` line"));
    let why = field(&source, "why:").unwrap_or_default();
    let build = Build {
        externs,
        ..Build::default()
    };
    let run = program(relative, &package_dir().join(relative), &build);
    if expect == "run" {
        assert!(
            run.outcome == "ok",
            "{relative}\n  This program must compile and run: {why}\n  It gave `{}`:\n{}",
            run.outcome,
            head(&run.output, 40)
        );
    } else if run.outcome == "ok" || run.outcome == "panic" {
        panic!(
            "{relative}\n  The compiler must reject this program with `{expect}`, but it \
             compiled.\n  {why}"
        );
    } else {
        assert!(
            expect
                .split(" or ")
                .any(|accepted| same_outcome(&run.outcome, accepted)),
            "{relative}\n  The compiler must reject this program with `{expect}`: {why}\n  \
             It was rejected with `{}` instead. That usually means that an item the program \
             uses is missing or has another name or type:\n{}",
            run.outcome,
            head(&run.output, 40)
        );
    }
}

// ------------------------------------------------------------------ cargo

/// The target directory that [`cargo`] uses for `target_key`.
pub fn cargo_target_dir(target_key: &str) -> PathBuf {
    scratch_root().join("cargo").join(target_key)
}

/// What a `cargo` command did.
#[derive(Debug, Clone)]
pub struct CargoRun {
    /// Whether cargo exited with status 0.
    pub ok: bool,
    /// What cargo wrote to standard output.
    pub stdout: String,
    /// What cargo wrote to standard error (its progress and the diagnostics).
    pub stderr: String,
}

impl CargoRun {
    /// Standard output followed by standard error.
    pub fn all(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// Runs `cargo ARGS` in a directory below the package directory, for example
/// `cargo("fixtures/l05/apps", "l05-apps", &["test", "--workspace"], &[])`.
///
/// The command works offline (the fixtures have no dependencies from a
/// registry). It builds into the directory `target_key` inside this package's
/// own target directory; commands with the same key share their build output
/// and wait for each other. `env` adds environment variables.
pub fn cargo(directory: &str, target_key: &str, args: &[&str], env: &[(&str, &str)]) -> CargoRun {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .current_dir(package_dir().join(directory))
        .args(args)
        .env("CARGO_TARGET_DIR", cargo_target_dir(target_key))
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TERM_COLOR", "never")
        // Flags of the outer build must not leak into the fixture's build.
        .env_remove("RUSTFLAGS")
        .env_remove("RUSTDOCFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTDOCFLAGS");
    for (name, value) in env {
        command.env(name, value);
    }
    let output = run_limited(&mut command, "cargo");
    CargoRun {
        ok: output.status.success(),
        stdout: text(&output.stdout),
        stderr: text(&output.stderr),
    }
}

/// The totals of a test run, added up over all its `test result:` lines.
/// (`rustdoc --test` and `cargo test --doc` print more than one such line.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Totals {
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
}

/// Reads the totals from the output of a test run.
pub fn totals(output: &str) -> Totals {
    let mut totals = Totals::default();
    for line in output
        .lines()
        .filter(|line| line.starts_with("test result:"))
    {
        let number_before = |word: &str| -> usize {
            line.split(';')
                .flat_map(|part| part.split('.'))
                .find_map(|part| part.trim().strip_suffix(word)?.trim().parse().ok())
                .unwrap_or(0)
        };
        totals.passed += number_before("passed");
        totals.failed += number_before("failed");
        totals.ignored += number_before("ignored");
    }
    totals
}

/// Runs `rustdoc --test` on a Markdown file below the package directory and
/// returns what it printed.
pub fn rustdoc_test(relative: &str) -> String {
    let rustdoc = std::env::var_os("RUSTDOC").unwrap_or_else(|| "rustdoc".into());
    let mut command = Command::new(rustdoc);
    command
        .current_dir(package_dir())
        .args(["--edition", "2024", "--color", "never", "--test"])
        .arg(relative);
    let output = run_limited(&mut command, "rustdoc");
    format!("{}{}", text(&output.stdout), text(&output.stderr))
}

/// The text of the string value of `"key"` in a JSON text, when the value has
/// no escapes: enough for names and version numbers in `cargo metadata`.
pub fn json_string(json: &str, key: &str) -> Option<String> {
    let start = json.find(&format!("\"{key}\":\""))? + key.len() + 4;
    let end = json[start..].find('"')?;
    Some(json[start..start + end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_ignores_case_and_spacing() {
        assert_eq!(digest("  C2:  Major "), digest("c2: major"));
        assert_ne!(digest("c2: major"), digest("c2: minor"));
    }

    #[test]
    fn fields_are_read_from_both_comment_styles() {
        let source = "//@ expect: E0451\n//! EXPECT: ?\nfn main() {}\n";
        assert_eq!(field(source, "expect:").as_deref(), Some("E0451"));
        assert_eq!(field(source, "EXPECT:").as_deref(), Some("?"));
        assert_eq!(field(source, "why:"), None);
    }

    #[test]
    fn a_case_may_accept_more_than_one_outcome() {
        let dir = fresh_dir("support-case");
        let write = |name: &str, text: &str| -> String {
            let path = dir.join(name);
            std::fs::write(&path, text).expect("the scratch directory is writable");
            // `case` joins its argument to the package directory; an absolute
            // path replaces that directory.
            path.to_string_lossy().into_owned()
        };
        let runs = write("runs.rs", "//@ expect: run\nfn main() {}\n");
        case(&runs, &[]);
        let rejected = write(
            "rejected.rs",
            "//@ expect: E0425 or E0308\nfn main() { let x: u8 = \"a\"; }\n",
        );
        case(&rejected, &[]);
        let other = write(
            "other.rs",
            "//@ expect: E0425 or E0599\nfn main() { let x: u8 = \"a\"; }\n",
        );
        let failed = std::panic::catch_unwind(|| case(&other, &[]));
        assert!(failed.is_err(), "E0308 is not one of the accepted outcomes");
    }

    #[test]
    fn outcomes_compare_as_sets() {
        assert!(same_outcome("E0277,E0599", "e0599, E0277"));
        assert!(same_outcome("OK", "ok"));
        assert!(!same_outcome("E0277", "E0599"));
    }

    #[test]
    fn implements_reports_bounds_as_values() {
        assert!(implements!(String: Send + Sync));
        assert!(!implements!(std::rc::Rc<u8>: Send));
    }

    #[test]
    fn totals_add_up_all_result_lines() {
        let output = "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured\n\n\
                      test result: FAILED. 1 passed; 2 failed; 1 ignored; 0 measured\n";
        assert_eq!(
            totals(output),
            Totals {
                passed: 6,
                failed: 2,
                ignored: 1
            }
        );
    }

    #[test]
    fn json_strings_are_found_by_key() {
        let json = r#"{"name":"swatch","version":"1.2.0","id":"x"}"#;
        assert_eq!(json_string(json, "version").as_deref(), Some("1.2.0"));
        assert_eq!(json_string(json, "license"), None);
    }

    #[test]
    fn programs_report_their_outcome_as_a_word() {
        let build = Build::default();
        let ok = program_from_text("support-ok", "fn main() {}", &build);
        assert_eq!(ok.outcome, "ok", "{}", ok.output);
        let panics = program_from_text("support-panic", "fn main() { panic!(\"no\") }", &build);
        assert_eq!(panics.outcome, "panic");
        let rejected =
            program_from_text("support-error", "fn main() { let x: u8 = \"a\"; }", &build);
        assert_eq!(rejected.outcome, "E0308");
    }

    #[test]
    fn programs_can_use_a_library() {
        let lib = library_from_text("tiny", "pub fn two() -> u8 { 2 }", &Build::default())
            .expect("the library compiles");
        let build = Build {
            externs: &[("tiny", &lib)],
            ..Build::default()
        };
        let run = program_from_text(
            "support-extern",
            "fn main() { assert_eq!(tiny::two(), 2) }",
            &build,
        );
        assert_eq!(run.outcome, "ok", "{}", run.output);
    }
}
