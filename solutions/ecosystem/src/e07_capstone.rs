//! LESSON E07 — Capstone: `jobq`, a small batch job runner
//! Run: cargo test --manifest-path ecosystem/Cargo.toml --test e07_capstone
//! Try: cargo run --manifest-path ecosystem/Cargo.toml --bin jobq -- -v run ecosystem/jobs.example.json
//!
//! PREDICT before running: first answer `p_predictions` in `tests/e07_capstone.rs`.
//! Then consider: which layer turns a missing file into exit code 66, and which layer
//! only adds the file name?
//! IMPLEMENT the `todo!()`s here and `main` in `src/bin/jobq.rs`. The integration tests
//! in `tests/e07_capstone.rs` call this library directly and also run the real binary.
//! EXPLAIN afterward: Draw the data flow from argv to exit code: clap → config (serde +
//! thiserror) → worker pool (tokio) → report (serde) → exit code (anyhow chain).
//! Mark where each crate's responsibility starts and ends.
//! JUDGMENT: Keep `main` thin. Everything testable lives in the library: parsing
//! (`Cli::try_parse_from`), loading, running, rendering, and exit-code mapping. `main`
//! only installs the global subscriber, builds the runtime, and converts the result
//! into an `ExitCode`.

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// A. The config file (JSON). Contract (same techniques as E02):
/// - `workers` defaults to 4 and `queue_capacity` to 16, `job_timeout_ms` to 1000;
/// - unknown fields are rejected everywhere (`Config`, `Job`, and inside `task`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default = "default_queue_capacity")]
    pub queue_capacity: usize,
    #[serde(default = "default_job_timeout_ms")]
    pub job_timeout_ms: u64,
    pub jobs: Vec<Job>,
}

fn default_workers() -> usize {
    4
}
fn default_queue_capacity() -> usize {
    16
}
fn default_job_timeout_ms() -> u64 {
    1000
}

/// A. `{"id": "a", "task": {"type": "sleep", "ms": 10}}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub id: String,
    pub task: Task,
}

/// A. Internally tagged by `type`, snake_case:
/// `{"type":"sleep","ms":10}`, `{"type":"checksum","data":"abc"}`, `{"type":"fail","message":"boom"}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Task {
    /// Wait `ms` milliseconds (async), then succeed with `slept {ms}ms`.
    Sleep { ms: u64 },
    /// Sum the UTF-8 bytes of `data` as u64 on the blocking pool; succeed with `checksum {sum}`.
    Checksum { data: String },
    /// Fail with `message`.
    Fail { message: String },
}

/// B. Library error for loading a config. Display messages:
/// - `Io`: `could not read config file` (source: the io::Error; `?` converts it)
/// - `Json`: `invalid config JSON` (source: the serde_json::Error; `?` converts it)
/// - `Invalid(reason)`: `invalid config: {reason}`
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read config file")]
    Io(#[from] std::io::Error),
    #[error("invalid config JSON")]
    Json(#[from] serde_json::Error),
    #[error("invalid config: {0}")]
    Invalid(String),
}

/// B. Parse and validate. Rules, checked in this order, with these exact reasons:
/// - `workers must be between 1 and 64`
/// - `queue_capacity must be at least 1`
/// - `job_timeout_ms must be at least 1`
/// - `job id must not be empty`
/// - ``duplicate job id `ID` `` (the first duplicate in file order)
pub fn parse_config(text: &str) -> Result<Config, ConfigError> {
    let config: Config = serde_json::from_str(text)?;
    let invalid = |reason: &str| Err(ConfigError::Invalid(reason.to_owned()));
    if !(1..=64).contains(&config.workers) {
        return invalid("workers must be between 1 and 64");
    }
    if config.queue_capacity < 1 {
        return invalid("queue_capacity must be at least 1");
    }
    if config.job_timeout_ms < 1 {
        return invalid("job_timeout_ms must be at least 1");
    }
    if config.jobs.iter().any(|job| job.id.is_empty()) {
        return invalid("job id must not be empty");
    }
    let mut seen = std::collections::HashSet::new();
    for job in &config.jobs {
        if !seen.insert(job.id.as_str()) {
            return Err(ConfigError::Invalid(format!(
                "duplicate job id `{}`",
                job.id
            )));
        }
    }
    Ok(config)
}

/// B. Read the file and `parse_config` it.
pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    parse_config(&std::fs::read_to_string(path)?)
}

/// How one job ended. Serialized with a `status` tag:
/// `{"status":"succeeded","output":"..."}`, `{"status":"failed","error":"..."}`,
/// `{"status":"timed_out"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome {
    Succeeded { output: String },
    Failed { error: String },
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JobReport {
    pub id: String,
    #[serde(flatten)]
    pub outcome: Outcome,
    /// Time from the job's start to its outcome (including a timeout), in milliseconds.
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// One entry per job, in config order (not completion order).
    pub jobs: Vec<JobReport>,
    pub succeeded: usize,
    pub failed: usize,
    pub timed_out: usize,
}

/// C. Run every job on a pool of `config.workers` worker tasks that pull `(index, Job)`
/// from ONE bounded `mpsc` queue of `config.queue_capacity`. (Hint: share the receiver
/// as `Arc<tokio::sync::Mutex<Receiver>>`.) Each job:
/// - runs inside a span `job` with field `id` (Display: `job{id=a}`), attached with
///   `.instrument(span)`, never with `span.enter()` held across `.await`. Create the
///   span at WARN level: a span filtered out by the level also disappears from the
///   context of the WARN events inside it;
/// - emits INFO `job started`, then `job finished` with a `status` field
///   (`succeeded`, `failed`, `timed_out`): INFO for success and WARN otherwise;
/// - is limited to `job_timeout_ms` (a timeout gives `Outcome::TimedOut`).
///
/// `Checksum` runs on `spawn_blocking`. The report lists jobs in config order with counts.
pub async fn run_jobs(config: &Config) -> Report {
    use std::sync::Arc;
    use tokio::sync::{Mutex, mpsc};
    use tracing::Instrument as _;

    let timeout = std::time::Duration::from_millis(config.job_timeout_ms);
    let (tx, rx) = mpsc::channel::<(usize, Job)>(config.queue_capacity);
    let rx = Arc::new(Mutex::new(rx));
    let mut workers = tokio::task::JoinSet::new();
    for _ in 0..config.workers {
        let rx = Arc::clone(&rx);
        workers.spawn(async move {
            let mut done = Vec::new();
            loop {
                let next = rx.lock().await.recv().await;
                let Some((index, job)) = next else { break };
                let span = tracing::warn_span!("job", id = %job.id);
                let report = run_one(job, timeout).instrument(span).await;
                done.push((index, report));
            }
            done
        });
    }
    for (index, job) in config.jobs.iter().cloned().enumerate() {
        if tx.send((index, job)).await.is_err() {
            break;
        }
    }
    drop(tx);
    let mut done: Vec<(usize, JobReport)> =
        workers.join_all().await.into_iter().flatten().collect();
    done.sort_by_key(|&(index, _)| index);
    let jobs: Vec<JobReport> = done.into_iter().map(|(_, report)| report).collect();
    let count = |want: fn(&Outcome) -> bool| jobs.iter().filter(|j| want(&j.outcome)).count();
    Report {
        succeeded: count(|o| matches!(o, Outcome::Succeeded { .. })),
        failed: count(|o| matches!(o, Outcome::Failed { .. })),
        timed_out: count(|o| matches!(o, Outcome::TimedOut)),
        jobs,
    }
}

async fn run_task(task: Task) -> Result<String, String> {
    match task {
        Task::Sleep { ms } => {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            Ok(format!("slept {ms}ms"))
        }
        Task::Checksum { data } => {
            let sum = tokio::task::spawn_blocking(move || data.bytes().map(u64::from).sum::<u64>())
                .await
                .map_err(|err| err.to_string())?;
            Ok(format!("checksum {sum}"))
        }
        Task::Fail { message } => Err(message),
    }
}

async fn run_one(job: Job, timeout: std::time::Duration) -> JobReport {
    tracing::info!("job started");
    let start = tokio::time::Instant::now();
    let outcome = match tokio::time::timeout(timeout, run_task(job.task)).await {
        Ok(Ok(output)) => Outcome::Succeeded { output },
        Ok(Err(error)) => Outcome::Failed { error },
        Err(_) => Outcome::TimedOut,
    };
    let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    match outcome {
        Outcome::Succeeded { .. } => tracing::info!(status = "succeeded", "job finished"),
        Outcome::Failed { .. } => tracing::warn!(status = "failed", "job finished"),
        Outcome::TimedOut => tracing::warn!(status = "timed_out", "job finished"),
    }
    JobReport {
        id: job.id,
        outcome,
        elapsed_ms,
    }
}

/// The `jobq` command line. (Supplied: you practiced clap in E03.)
#[derive(Debug, Parser)]
#[command(
    name = "jobq",
    version,
    about = "Run a batch of jobs from a JSON config"
)]
pub struct Cli {
    /// Log more (-v: info, -vv: debug); logs go to stderr
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {
    /// Validate a config file
    Check { config: PathBuf },
    /// Run every job in a config file
    Run {
        config: PathBuf,
        /// Override the number of workers
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=64))]
        workers: Option<u8>,
        /// Print the report as JSON
        #[arg(long)]
        json: bool,
    },
}

/// D. The application. Load the config with anyhow context `loading {path}`
/// (`Path::display`), then:
/// - `check`: write `ok: {jobs} jobs, {workers} workers` and return 0.
/// - `run`: apply `--workers`, run the jobs, write the report, and return 0 if every
///   job succeeded, else 1. With `--json`, write the `Report` as one line of JSON.
///   Otherwise write one line per job (`{id} succeeded: {output}`, `{id} failed: {error}`,
///   `{id} timed_out`), then `{s} succeeded, {f} failed, {t} timed out`.
pub async fn execute(cli: Cli, out: &mut impl Write) -> anyhow::Result<i32> {
    use anyhow::Context as _;
    let load =
        |path: &Path| load_config(path).with_context(|| format!("loading {}", path.display()));
    match cli.command {
        CliCommand::Check { config } => {
            let config = load(&config)?;
            writeln!(
                out,
                "ok: {} jobs, {} workers",
                config.jobs.len(),
                config.workers
            )?;
            Ok(0)
        }
        CliCommand::Run {
            config,
            workers,
            json,
        } => {
            let mut config = load(&config)?;
            if let Some(workers) = workers {
                config.workers = usize::from(workers);
            }
            let report = run_jobs(&config).await;
            if json {
                writeln!(out, "{}", serde_json::to_string(&report)?)?;
            } else {
                for job in &report.jobs {
                    match &job.outcome {
                        Outcome::Succeeded { output } => {
                            writeln!(out, "{} succeeded: {output}", job.id)?;
                        }
                        Outcome::Failed { error } => writeln!(out, "{} failed: {error}", job.id)?,
                        Outcome::TimedOut => writeln!(out, "{} timed_out", job.id)?,
                    }
                }
                writeln!(
                    out,
                    "{} succeeded, {} failed, {} timed out",
                    report.succeeded, report.failed, report.timed_out
                )?;
            }
            Ok(i32::from(report.succeeded != report.jobs.len()))
        }
    }
}

/// D. Exit code for an error returned by `execute`: 66 if an `io::Error` of kind
/// `NotFound` is anywhere in the chain, else 78 if a `ConfigError` is, else 1.
pub fn error_exit_code(err: &anyhow::Error) -> i32 {
    let not_found = err.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound)
    });
    if not_found {
        66
    } else if err.chain().any(|cause| cause.is::<ConfigError>()) {
        78
    } else {
        1
    }
}
