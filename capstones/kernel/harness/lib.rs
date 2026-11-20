//! Test support for the kernel capstone. You do not need to edit this file,
//! but it is worth reading: it shows how a host program builds a bare-metal
//! binary, runs it in an emulator and keeps full control over the child
//! process.
//!
//! - [`run_kernel`] builds `kernel/` for `aarch64-unknown-none`, boots it
//!   in `qemu-system-aarch64` and returns everything the kernel wrote to
//!   its serial port. The emulator is always killed before the function
//!   returns: after a time limit, after too much output, or when the test
//!   panics.
//! - [`within`] runs a closure with a time limit, so that a test of a data
//!   structure with a broken loop fails with a message instead of hanging.
//! - [`transcript`] gives the serial output that SPEC.md section 4 requires.
//!
//! Environment variables (all optional):
//!
//! | Variable | Meaning | Default |
//! | --- | --- | --- |
//! | `KERNEL_QEMU` | emulator binary; when set, a binary that cannot be started is a failure, not a skip | `qemu-system-aarch64` |
//! | `KERNEL_QEMU_TIMEOUT_SECS` | time limit for one emulator run | `10` |
//! | `KERNEL_PROFILE` | `debug` or `release` kernel build | same as the test build |
//! | `KERNEL_REQUIRE_QEMU` | `1`: a missing emulator or target is a failure, not a skip | unset |

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

/// The bare-metal target the kernel is built for.
pub const TARGET: &str = "aarch64-unknown-none";

/// The machine every acceptance run uses (SPEC 2.1). The path of the kernel
/// image follows `-kernel`.
pub const QEMU_ARGS: &[&str] = &[
    "-machine",
    "virt,gic-version=2",
    "-accel",
    "tcg",
    "-cpu",
    "cortex-a72",
    "-smp",
    "1",
    "-m",
    "128M",
    "-display",
    "none",
    "-monitor",
    "none",
    "-serial",
    "stdio",
    "-no-reboot",
    "-nic",
    "none",
    "-kernel",
];

/// A kernel that prints more than this is cut off; the transcript is about
/// one kilobyte.
const OUTPUT_LIMIT: usize = 256 * 1024;

/// The serial output required by SPEC 4.1, one line per element.
pub fn transcript() -> Vec<&'static str> {
    include_str!("../tests/transcript.txt").lines().collect()
}

/// The lines of `text` that start with `tag`, for example `"[m3] "`.
pub fn tagged<'a>(text: impl IntoIterator<Item = &'a str>, tag: &str) -> Vec<&'a str> {
    text.into_iter()
        .filter(|line| line.starts_with(tag))
        .collect()
}

/// What to build and where.
pub struct Config<'a> {
    /// A directory for build output. Tests pass `env!("CARGO_TARGET_TMPDIR")`.
    pub scratch_dir: &'a str,
    /// Cargo features of the `kernel` package to enable.
    pub features: &'a [&'a str],
}

/// How an emulator run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum End {
    /// The emulator exited by itself, with this exit code (`None`: killed
    /// by a signal).
    Exited(Option<i32>),
    /// The kernel had printed nothing when a third of the time limit had
    /// passed. The harness killed the emulator.
    Silent(Duration),
    /// The emulator was still running at the time limit. The harness
    /// killed it.
    TimedOut(Duration),
    /// The kernel printed more than the output limit. The harness killed
    /// the emulator.
    TooMuchOutput,
}

impl std::fmt::Display for End {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            End::Exited(Some(code)) => write!(f, "QEMU exited with status {code}"),
            End::Exited(None) => write!(f, "QEMU was killed by a signal"),
            End::Silent(after) => write!(
                f,
                "the kernel printed nothing in {:.1} s, so the harness killed QEMU \
                 (is there an entry point at the right address, and does it reach the UART?)",
                after.as_secs_f64()
            ),
            End::TimedOut(after) => write!(
                f,
                "QEMU was still running after {:.1} s, so the harness killed it \
                 (the kernel must power the machine off: SPEC 3.5)",
                after.as_secs_f64()
            ),
            End::TooMuchOutput => write!(
                f,
                "the kernel printed more than {OUTPUT_LIMIT} bytes, so the harness killed QEMU"
            ),
        }
    }
}

/// The result of one boot.
#[derive(Debug, Clone)]
pub struct Run {
    /// Serial output with every `\r\n` replaced by `\n`.
    pub output: String,
    /// What QEMU itself wrote to standard error.
    pub qemu_stderr: String,
    pub end: End,
    /// First line of `qemu-system-aarch64 --version`.
    pub qemu_version: String,
    /// The command line, to repeat the run by hand.
    pub command: String,
}

impl Run {
    pub fn lines(&self) -> Vec<&str> {
        self.output.lines().collect()
    }

    /// A description of the run for assertion messages.
    pub fn report(&self) -> String {
        let mut report = format!(
            "\n--- how the run ended ---\n{}\n--- serial output ---\n{}",
            self.end, self.output
        );
        if !self.output.ends_with('\n') && !self.output.is_empty() {
            report.push_str("\n(no line end after the last line)");
        }
        if !self.qemu_stderr.trim().is_empty() {
            report.push_str("\n--- QEMU standard error ---\n");
            report.push_str(self.qemu_stderr.trim_end());
        }
        report.push_str("\n--- to repeat by hand ---\n");
        report.push_str(&self.command);
        report.push_str("\n(");
        report.push_str(&self.qemu_version);
        report.push(')');
        report
    }
}

/// The result of [`run_kernel`].
#[derive(Debug, Clone)]
pub enum Outcome {
    /// The kernel was built and booted.
    Ran(Run),
    /// A tool is missing on this machine; the text says which one.
    Skipped(String),
    /// `cargo build` failed for the kernel; the text holds the compiler
    /// output.
    BuildFailed(String),
}

/// Writes straight to the process's standard error. Unlike `eprintln!`,
/// this is not captured by the test harness, so a skip is visible in a
/// normal `cargo test` run.
fn announce(message: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "{message}");
    let _ = stderr.flush();
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| value == "1")
}

/// Kills and reaps the child when dropped, so no emulator outlives a test,
/// whatever path leaves the function that owns the guard.
struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        // Both calls fail harmlessly if the child has already exited.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Collects everything `stream` produces on a helper thread.
fn drain(mut stream: impl Read + Send + 'static) -> (Arc<Mutex<Vec<u8>>>, thread::JoinHandle<()>) {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let shared = Arc::clone(&buffer);
    let handle = thread::spawn(move || {
        let mut chunk = [0_u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => shared
                    .lock()
                    .expect("no code panics while holding this lock")
                    .extend_from_slice(&chunk[..n]),
            }
        }
    });
    (buffer, handle)
}

fn kernel_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("kernel")
}

fn profile() -> String {
    std::env::var("KERNEL_PROFILE").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
        .to_string()
    })
}

fn time_limit() -> Duration {
    let secs = std::env::var("KERNEL_QEMU_TIMEOUT_SECS")
        .ok()
        .and_then(|text| text.parse::<u64>().ok())
        .filter(|&secs| secs > 0)
        .unwrap_or(10);
    Duration::from_secs(secs)
}

/// `Ok(first line of --version)` when the emulator can be started.
fn qemu_version(qemu: &str) -> Result<String, String> {
    let output = Command::new(qemu)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("cannot start `{qemu}`: {error}"))?;
    if !output.status.success() {
        return Err(format!("`{qemu} --version` failed with {}", output.status));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .next()
        .unwrap_or("unknown QEMU version")
        .to_string())
}

/// Whether the standard library for the bare-metal target is installed for
/// the toolchain that `rustc` resolves to in the kernel directory.
fn target_installed() -> Result<(), String> {
    let output = Command::new("rustc")
        .args(["--print", "target-libdir", "--target", TARGET])
        .current_dir(kernel_dir())
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("cannot start `rustc`: {error}"))?;
    let dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() && !dir.is_empty() && Path::new(&dir).is_dir() {
        Ok(())
    } else {
        Err(format!(
            "the Rust target `{TARGET}` is not installed (run: rustup target add {TARGET})"
        ))
    }
}

/// Builds the kernel and returns the path of the ELF image.
fn build_kernel(config: &Config, profile: &str) -> Result<PathBuf, String> {
    // Each feature set gets its own target directory. Two builds that
    // differ only in features write the same file name, and tests run in
    // parallel.
    let variant = if config.features.is_empty() {
        "default".to_string()
    } else {
        config.features.join("+")
    };
    let target_dir = Path::new(config.scratch_dir).join(format!("kernel-{variant}"));

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut command = Command::new(cargo);
    command
        .current_dir(kernel_dir())
        .args(["build", "--target", TARGET, "--target-dir"])
        .arg(&target_dir)
        .stdin(Stdio::null());
    if profile == "release" {
        command.arg("--release");
    }
    if !config.features.is_empty() {
        command.args(["--features", &config.features.join(",")]);
    }
    let output = command
        .output()
        .map_err(|error| format!("cannot start cargo to build the kernel: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "the kernel does not build for {TARGET}:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let image = target_dir.join(TARGET).join(profile).join("kernel");
    if image.is_file() {
        Ok(image)
    } else {
        Err(format!(
            "the build succeeded but {} does not exist",
            image.display()
        ))
    }
}

/// Boots `image` and waits until the emulator exits or a limit is reached.
fn boot(qemu: &str, version: String, image: &Path) -> Run {
    let limit = time_limit();
    let silence_limit = limit / 3;

    let mut command = Command::new(qemu);
    command
        .args(QEMU_ARGS)
        .arg(image)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let command_text = format!("{qemu} {} {}", QEMU_ARGS.join(" "), image.display());

    let started = Instant::now();
    let child = command
        .spawn()
        .unwrap_or_else(|error| panic!("cannot start `{command_text}`: {error}"));
    let mut child = KillOnDrop(child);
    let (stdout, stdout_thread) = drain(child.0.stdout.take().expect("stdout is piped"));
    let (stderr, stderr_thread) = drain(child.0.stderr.take().expect("stderr is piped"));

    let end = loop {
        match child.0.try_wait() {
            Ok(Some(status)) => break End::Exited(exit_code(status)),
            Ok(None) => {}
            Err(error) => panic!("cannot wait for QEMU: {error}"),
        }
        let printed = stdout.lock().expect("reader thread does not panic").len();
        let elapsed = started.elapsed();
        if printed > OUTPUT_LIMIT {
            break End::TooMuchOutput;
        }
        if printed == 0 && elapsed >= silence_limit {
            break End::Silent(elapsed);
        }
        if elapsed >= limit {
            break End::TimedOut(elapsed);
        }
        thread::sleep(Duration::from_millis(10));
    };

    // Kill (if still running) and reap. That closes the pipes, so the
    // reader threads see end-of-file and finish.
    drop(child);
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    let bytes = stdout.lock().expect("reader thread has finished").clone();
    let output = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
    let qemu_stderr =
        String::from_utf8_lossy(&stderr.lock().expect("reader thread has finished")).into_owned();
    Run {
        output,
        qemu_stderr,
        end,
        qemu_version: version,
        command: command_text,
    }
}

fn exit_code(status: ExitStatus) -> Option<i32> {
    status.code()
}

/// Builds the kernel with `config` and boots it once.
///
/// Returns [`Outcome::Skipped`] (after printing a line that starts with
/// `SKIPPED`) when QEMU or the bare-metal target is not installed, and
/// [`Outcome::BuildFailed`] when the kernel does not compile.
///
/// # Panics
///
/// Panics when a tool is missing and `KERNEL_REQUIRE_QEMU=1` is set, and
/// when `KERNEL_QEMU` names a program that cannot be started.
pub fn run_kernel(config: &Config) -> Outcome {
    let chosen = std::env::var("KERNEL_QEMU").ok();
    let qemu = chosen
        .clone()
        .unwrap_or_else(|| "qemu-system-aarch64".to_string());
    let missing = |reason: String| {
        let message = format!(
            "SKIPPED: {reason}. The QEMU acceptance tests did not run and prove nothing. \
             See README.md, \"Install the tools\"."
        );
        assert!(
            !env_flag("KERNEL_REQUIRE_QEMU"),
            "KERNEL_REQUIRE_QEMU=1 is set, so this is a failure. {message}"
        );
        announce(&message);
        Outcome::Skipped(message)
    };

    let version = match qemu_version(&qemu) {
        Ok(version) => version,
        // Somebody who names an emulator wants it used. A wrong path must
        // not look like "QEMU is not installed".
        Err(reason) if chosen.is_some() => panic!("KERNEL_QEMU is set, but {reason}"),
        Err(reason) => return missing(reason),
    };
    if let Err(reason) = target_installed() {
        return missing(reason);
    }

    let profile = profile();
    match build_kernel(config, &profile) {
        Ok(image) => Outcome::Ran(boot(&qemu, version, &image)),
        Err(message) => Outcome::BuildFailed(message),
    }
}

/// Runs `work` on a helper thread and returns its result, or panics with
/// `what` in the message when it takes longer than `secs` seconds.
///
/// A panic inside `work` is passed on unchanged.
pub fn within<R: Send + 'static>(
    secs: u64,
    what: &str,
    work: impl FnOnce() -> R + Send + 'static,
) -> R {
    let (sender, receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        // The receiver is gone only if the time limit has already passed.
        let _ = sender.send(work());
    });
    match receiver.recv_timeout(Duration::from_secs(secs)) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("{what}: no result after {secs} s. A loop does not end.")
        }
        // The sender was dropped without sending: `work` panicked.
        Err(mpsc::RecvTimeoutError::Disconnected) => match worker.join() {
            Err(payload) => std::panic::resume_unwind(payload),
            Ok(()) => unreachable!("the worker sends before it returns"),
        },
    }
}
