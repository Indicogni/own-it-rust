//! Support code for the lessons. You do not need to edit this file.
//!
//! The tests in this module's lessons check real files (manifests, lint tables,
//! workflow files) and run real `cargo` commands on the small workspaces under
//! `fixtures/` and `capstone/`. The helpers here locate those files, parse them, and
//! run commands with a time limit, so a stuck command fails with a message instead of
//! hanging the test run.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub use serde_json::Value;

/// The directory that holds this package's `Cargo.toml`.
pub fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `fixtures/<name>` inside this package.
pub fn fixture(name: &str) -> PathBuf {
    package_root().join("fixtures").join(name)
}

/// Reads a text file, or fails the test with a message that names the missing file.
#[track_caller]
pub fn read(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) => text.replace("\r\n", "\n"),
        Err(err) => panic!(
            "cannot read {}: {err}. If the task asks you to create this file, create it.",
            shown(path)
        ),
    }
}

/// A path relative to the package root, with `/` separators, for messages.
pub fn shown(path: &Path) -> String {
    let relative = path.strip_prefix(package_root()).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

/// Parses a TOML file into a JSON-style tree, so tests can use
/// [`Value::pointer`], for example `doc.pointer("/workspace/package/version")`.
#[track_caller]
pub fn toml(path: &Path) -> Value {
    let text = read(path);
    match toml::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(err) => panic!("{} is not valid TOML: {err}", shown(path)),
    }
}

/// Parses a YAML file into a JSON-style tree.
#[track_caller]
pub fn yaml(path: &Path) -> Value {
    let text = read(path);
    match serde_norway::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(err) => panic!("{} is not valid YAML: {err}", shown(path)),
    }
}

/// Parses a JSON file.
#[track_caller]
pub fn json(path: &Path) -> Value {
    let text = read(path);
    match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(err) => panic!("{} is not valid JSON: {err}", shown(path)),
    }
}

/// Looks up a JSON pointer and returns the string there, if any.
pub fn text_at<'a>(doc: &'a Value, pointer: &str) -> Option<&'a str> {
    doc.pointer(pointer).and_then(Value::as_str)
}

/// Looks up a JSON pointer and returns the strings of the array there.
/// A missing key or a non-array gives an empty list.
pub fn list_at(doc: &Value, pointer: &str) -> Vec<String> {
    doc.pointer(pointer)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The result of running a command.
#[derive(Debug, Clone)]
pub struct Run {
    /// The command line, for messages.
    pub command: String,
    /// `Some(0)` on success. `None` if the process was killed by a signal.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn ok(&self) -> bool {
        self.code == Some(0)
    }

    /// Fails the test, showing the command and its output, unless it exited with 0.
    #[track_caller]
    pub fn expect_ok(&self, why: &str) -> &Self {
        assert!(
            self.ok(),
            "{why}\n  command: {}\n  exit code: {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            self.command,
            self.code,
            tail(&self.stdout),
            tail(&self.stderr)
        );
        self
    }

    /// Parses stdout as one JSON value per line (Cargo's `--message-format=json`).
    pub fn json_lines(&self) -> Vec<Value> {
        self.stdout
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }

    /// Compiler messages of the given level (`"warning"` or `"error"`) from a run with
    /// `--message-format=json`. Summary lines such as "1 warning emitted" are skipped.
    pub fn diagnostics(&self, level: &str) -> Vec<Diagnostic> {
        self.json_lines()
            .iter()
            .filter(|line| text_at(line, "/reason") == Some("compiler-message"))
            .filter(|line| text_at(line, "/message/level") == Some(level))
            .filter(|line| !line.pointer("/message/spans").is_some_and(is_empty_array))
            .map(|line| Diagnostic {
                code: text_at(line, "/message/code/code").unwrap_or("").to_owned(),
                message: text_at(line, "/message/message").unwrap_or("").to_owned(),
                rendered: text_at(line, "/message/rendered").unwrap_or("").to_owned(),
            })
            .collect()
    }
}

fn is_empty_array(value: &Value) -> bool {
    value.as_array().is_some_and(Vec::is_empty)
}

/// One compiler or Clippy message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The lint name or error code, for example `unused_variables` or `E0308`.
    pub code: String,
    pub message: String,
    pub rendered: String,
}

/// Lists the lint names (or error codes) of some diagnostics, for messages.
pub fn codes(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|d| {
            if d.code.is_empty() {
                d.message.clone()
            } else {
                d.code.clone()
            }
        })
        .collect()
}

fn tail(text: &str) -> String {
    const KEEP: usize = 60;
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= KEEP {
        text.to_owned()
    } else {
        format!(
            "... ({} earlier lines not shown)\n{}",
            lines.len() - KEEP,
            lines[lines.len() - KEEP..].join("\n")
        )
    }
}

/// Where nested Cargo commands put their build output: a directory inside this
/// package's own target directory, one per workspace, so they never fight over the
/// build lock with the `cargo test` that is running you.
pub fn nested_target_dir(workspace: &Path) -> PathBuf {
    let exe = std::env::current_exe().expect("the test executable has a path");
    // <target>/<profile>/deps/<test executable>
    let target = exe
        .ancestors()
        .nth(3)
        .expect("the test executable lives in <target>/<profile>/deps")
        .to_path_buf();
    let name = workspace
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "workspace".to_owned());
    target.join("m9-nested").join(name)
}

/// Held by a test while it runs a sequence of commands that must not be mixed with
/// another test's commands (build, then read the build's output, for example).
/// Dropping it lets the next test in.
#[derive(Debug)]
pub struct Turn {
    _lock: std::fs::File,
}

/// Waits for this test's turn at the nested builds. The lock is a file in the
/// target directory, so it also works when every test runs in its own process
/// (as under `cargo nextest`).
pub fn take_turn() -> Turn {
    let dir = nested_target_dir(Path::new("locks"));
    std::fs::create_dir_all(&dir).expect("the target directory is writable");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("turn.lock"))
        .expect("the lock file can be created");
    file.lock().expect("the lock file can be locked");
    Turn { _lock: file }
}

/// The `cargo` that is running this test (Cargo sets `CARGO` for test processes).
pub fn cargo_program() -> OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

/// Runs `cargo <args>` in `dir`, offline, with plain output and a private target
/// directory. Fails the test if the command is still running after five minutes.
#[track_caller]
pub fn cargo(dir: &Path, args: &[&str]) -> Run {
    cargo_with(dir, args, &[])
}

/// Like [`cargo`], with extra environment variables. An empty value removes the
/// variable instead of setting it.
#[track_caller]
pub fn cargo_with(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Run {
    // Without a manifest of its own, Cargo would walk up the directory tree and
    // run the command on this package instead.
    assert!(
        dir.join("Cargo.toml").is_file(),
        "{}/Cargo.toml does not exist yet; create it first",
        shown(dir)
    );
    let mut command = Command::new(cargo_program());
    command
        .args(args)
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", nested_target_dir(dir))
        .env("CARGO_TERM_COLOR", "never")
        .env("CARGO_NET_OFFLINE", "true")
        // Settings of the outer build must not leak into the nested one.
        .env_remove("RUSTFLAGS")
        .env_remove("RUSTDOCFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_BUILD_TARGET")
        .env_remove("RUSTC_WORKSPACE_WRAPPER");
    for (key, value) in env {
        if value.is_empty() {
            command.env_remove(key);
        } else {
            command.env(key, value);
        }
    }
    let shown = format!("cargo {} (in {})", args.join(" "), self::shown(dir));
    run(command, shown, Duration::from_secs(300))
}

/// Runs any program with a time limit and captures its output.
pub fn run(mut command: Command, shown: String, limit: Duration) -> Run {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => panic!("cannot start `{shown}`: {err}"),
    };
    // Drain both pipes on helper threads, so a chatty child never blocks on a full pipe.
    let mut out_pipe = child.stdout.take().expect("stdout is piped");
    let mut err_pipe = child.stderr.take().expect("stderr is piped");
    let out_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = out_pipe.read_to_end(&mut bytes);
        bytes
    });
    let err_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = err_pipe.read_to_end(&mut bytes);
        bytes
    });
    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "`{shown}` was still running after {} s and was stopped",
                    limit.as_secs()
                );
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(err) => panic!("cannot wait for `{shown}`: {err}"),
        }
    };
    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    Run {
        command: shown,
        code: status.code(),
        stdout: String::from_utf8_lossy(&stdout).replace("\r\n", "\n"),
        stderr: String::from_utf8_lossy(&stderr).replace("\r\n", "\n"),
    }
}

/// The text of a panic payload, if it has one.
fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic without a text message".to_owned())
}

/// Fails the current test with `message`, without calling the panic hook. Code
/// under test may have replaced the hook (lesson 06), and a broken hook must not be
/// able to swallow or block the report of a failure.
fn fail_without_hook(message: String) -> ! {
    eprintln!("{message}");
    std::panic::resume_unwind(Box::new(message))
}

/// Runs `f` on a helper thread and returns its result, or fails the test with a
/// clear message if it has not finished after `secs` seconds. Tests use this around
/// code that would block forever under a wrong solution (a deadlock, a loop that
/// never advances). A panic inside `f` fails the test with the same message.
pub fn within<R: Send + 'static>(
    secs: u64,
    what: &str,
    f: impl FnOnce() -> R + Send + 'static,
) -> R {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::mpsc::{self, RecvTimeoutError};
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(catch_unwind(AssertUnwindSafe(f)));
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(Ok(value)) => value,
        Ok(Err(payload)) => {
            fail_without_hook(format!("{what} failed: {}", panic_text(payload.as_ref())))
        }
        Err(RecvTimeoutError::Timeout) => fail_without_hook(format!(
            "{what} was still running after {secs} s. It is probably blocked forever: \
             look for a lock that is never released, or a loop that never advances."
        )),
        Err(RecvTimeoutError::Disconnected) => fail_without_hook(format!(
            "{what} stopped without a result; see the panic message above"
        )),
    }
}

/// Runs the body of one test in a process of its own: this test executable is
/// started again for exactly the test `test` (its full name, for example
/// `l06_debug::tests::a_reports_message_and_location`).
///
/// Lesson 06 needs this. Its code replaces the panic hook, which is one setting
/// for the whole process. A mistake there (a hook that prints nothing, or that
/// waits on a lock) would otherwise silence or block every other test that runs in
/// the same process. In a child process it can only harm its own test, and the
/// parent stops a child that is still running after `secs` seconds.
pub fn in_own_process(test: &str, secs: u64, body: impl FnOnce()) {
    const MARK: &str = "M9_TOOLBOX_TEST_IN_OWN_PROCESS";
    if std::env::var_os(MARK).is_some_and(|value| value == test) {
        // This is the child: run the body, and report a failure without relying
        // on the panic hook.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
        if let Err(payload) = outcome {
            eprintln!("{test} failed: {}", panic_text(payload.as_ref()));
            std::process::exit(101);
        }
        return;
    }
    let exe = std::env::current_exe().expect("the test executable has a path");
    let mut command = Command::new(exe);
    command
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(MARK, test);
    let shown = format!(
        "the test {test} in a process of its own (if it was stopped, it is probably \
         blocked forever: look for a lock that is never released)"
    );
    let child = run(command, shown, Duration::from_secs(secs));
    if !child.ok() {
        fail_without_hook(format!(
            "{test} failed. It ran in a process of its own; this is what it printed:\n{}",
            tail(&child.stderr)
        ));
    }
}

/// Lines of Cargo's own stderr that start with `warning:` (manifest problems,
/// ignored keys, and so on). Compiler warnings are not included when the run used
/// `--message-format=json`.
pub fn cargo_warnings(run: &Run) -> Vec<String> {
    run.stderr
        .lines()
        .filter(|line| line.starts_with("warning:"))
        .map(str::to_owned)
        .collect()
}

/// Removes every whitespace character. Predictions about generated code compare
/// tokens, not layout.
pub fn squeeze(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Checks a lab answer without showing it. The answer is trimmed and lower-cased,
/// then hashed together with the lab's key and compared with a stored hash.
pub fn lab_answer_is_right(key: &str, answer: &str) -> bool {
    let normal = answer.trim().to_lowercase();
    let hash = fnv1a(format!("{key}\u{1f}{normal}").as_bytes());
    LAB_ANSWERS
        .iter()
        .any(|(k, expected)| *k == key && *expected == hash)
}

/// The hash used for lab answers (64-bit FNV-1a).
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Hashes of the accepted lab answers. One key may accept several spellings.
const LAB_ANSWERS: &[(&str, u64)] = &[
    ("l04.mutants.missed", 0xcf68dc992a617ff4),
    ("l06.debugger.quantity", 0x8e077cbdf3417e2),
];

/// Helpers for reading GitHub Actions workflow files in tests.
pub mod workflow {
    use super::Value;
    use std::collections::BTreeMap;

    /// One step of one job, with the environment it runs in.
    #[derive(Debug, Clone)]
    pub struct Step {
        /// The key of the job in `jobs:`.
        pub job: String,
        /// `uses: owner/action@ref`, if the step calls an action.
        pub uses: Option<String>,
        /// The command lines of `run:`, one entry per line. A line that ends in `\`
        /// is joined with the next one.
        pub commands: Vec<String>,
        /// `env:` of the workflow, the job and the step, merged (the step wins).
        pub env: BTreeMap<String, String>,
        /// The step's `with:` map, or `Null`.
        pub with: Value,
        /// `continue-on-error: true` on the step or on its job.
        pub continue_on_error: bool,
    }

    fn as_text(value: &Value) -> Option<String> {
        match value {
            Value::String(text) => Some(text.clone()),
            Value::Bool(flag) => Some(flag.to_string()),
            Value::Number(number) => Some(number.to_string()),
            _ => None,
        }
    }

    fn env_of(value: &Value) -> BTreeMap<String, String> {
        value
            .get("env")
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .filter_map(|(key, value)| Some((key.clone(), as_text(value)?)))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn command_lines(script: &str) -> Vec<String> {
        let mut lines = Vec::new();
        let mut pending = String::new();
        for line in script.lines() {
            let line = line.trim();
            if let Some(continued) = line.strip_suffix('\\') {
                pending.push_str(continued);
                pending.push(' ');
            } else {
                pending.push_str(line);
                if !pending.trim().is_empty() {
                    lines.push(pending.split_whitespace().collect::<Vec<_>>().join(" "));
                }
                pending.clear();
            }
        }
        if !pending.trim().is_empty() {
            lines.push(pending.split_whitespace().collect::<Vec<_>>().join(" "));
        }
        lines
    }

    /// The jobs of a workflow: `(key, job)`.
    pub fn jobs(doc: &Value) -> Vec<(String, Value)> {
        doc.get("jobs")
            .and_then(Value::as_object)
            .map(|jobs| jobs.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    /// Every step of every job.
    pub fn steps(doc: &Value) -> Vec<Step> {
        let workflow_env = env_of(doc);
        let mut all = Vec::new();
        for (key, job) in jobs(doc) {
            let mut job_env = workflow_env.clone();
            job_env.extend(env_of(&job));
            let job_continues = job.get("continue-on-error") == Some(&Value::Bool(true));
            let steps = job.get("steps").and_then(Value::as_array);
            for step in steps.into_iter().flatten() {
                let mut env = job_env.clone();
                env.extend(env_of(step));
                all.push(Step {
                    job: key.clone(),
                    uses: step.get("uses").and_then(Value::as_str).map(str::to_owned),
                    commands: step
                        .get("run")
                        .and_then(Value::as_str)
                        .map(command_lines)
                        .unwrap_or_default(),
                    env,
                    with: step.get("with").cloned().unwrap_or(Value::Null),
                    continue_on_error: job_continues
                        || step.get("continue-on-error") == Some(&Value::Bool(true)),
                });
            }
        }
        all
    }

    /// The events that start the workflow: the keys of `on:`.
    ///
    /// YAML 1.1 parsers read the key `on` as the boolean `true`; both spellings are
    /// looked up here.
    pub fn triggers(doc: &Value) -> Vec<String> {
        let on = doc.get("on").or_else(|| doc.get("true"));
        match on {
            Some(Value::String(event)) => vec![event.clone()],
            Some(Value::Array(events)) => events.iter().filter_map(as_text).collect(),
            Some(Value::Object(events)) => events.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }

    /// The first step with a command line that contains every one of `words` as a
    /// whole word (`cargo`, `--locked`, ...), in any order.
    pub fn find_command<'a>(steps: &'a [Step], words: &[&str]) -> Option<&'a Step> {
        steps.iter().find(|step| {
            step.commands.iter().any(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                words.iter().all(|word| parts.contains(word))
            })
        })
    }

    /// The first step that uses the action `owner/name` (any ref).
    pub fn find_action<'a>(steps: &'a [Step], action: &str) -> Option<&'a Step> {
        steps.iter().find(|step| {
            step.uses
                .as_deref()
                .and_then(|uses| uses.split('@').next())
                .is_some_and(|name| name.eq_ignore_ascii_case(action))
        })
    }

    /// True if some step runs `rustup toolchain install` without naming a toolchain
    /// (options such as `--profile minimal` are fine). rustup then installs the
    /// toolchain that the repository's `rust-toolchain.toml` pins.
    pub fn installs_toolchain_from_file(steps: &[Step]) -> bool {
        steps.iter().flat_map(|step| &step.commands).any(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            words.starts_with(&["rustup", "toolchain", "install"])
                && !words[3..].iter().any(|word| {
                    // A toolchain name: a channel, a version number, or a variable.
                    ["stable", "beta", "nightly"]
                        .iter()
                        .any(|channel| word.starts_with(channel))
                        || word.starts_with(|c: char| c.is_ascii_digit())
                        || word.contains('$')
                })
        })
    }

    /// The refs of all `uses:` that point at a moving branch instead of a tag or a
    /// commit, for example `actions/checkout@master`.
    pub fn floating_actions(steps: &[Step]) -> Vec<String> {
        steps
            .iter()
            .filter_map(|step| step.uses.clone())
            .filter(|uses| {
                // Local actions (`./path`) have no ref.
                !uses.starts_with("./")
                    && match uses.split_once('@') {
                        None => true,
                        Some((_, reference)) => {
                            matches!(reference, "main" | "master" | "latest" | "HEAD" | "dev")
                        }
                    }
            })
            .collect()
    }
}
