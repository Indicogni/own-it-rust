//! A program that is slow on purpose, for the profiling lab (LABS.md, "Lab 2").
//!
//! It summarizes a generated access log again and again, so that a sampling
//! profiler has a few seconds of work to look at.
//!
//! Run: cargo run --profile profiling --manifest-path mastery/m7_perf/Cargo.toml \
//!          --example profile_target -- baseline
//! Pass `yours` instead of `baseline` to profile your capstone, and a number
//! after it to change how often the log is summarized (default 100).

use m7_perf::l08_capstone::{baseline, sample_log, summarize};
use std::io;

fn main() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    let version = args.next().unwrap_or_else(|| "baseline".to_owned());
    let rounds: usize = match args.next() {
        Some(text) => text
            .parse()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?,
        None => 100,
    };
    let log = sample_log(200_000, 1);
    for _ in 0..rounds {
        match version.as_str() {
            "baseline" => baseline::summarize(log.as_bytes(), io::sink(), 5)?,
            "yours" => summarize(log.as_bytes(), io::sink(), 5)?,
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown version {other:?}; use `baseline` or `yours`"),
                ));
            }
        }
    }
    println!("{version}: summarized {} bytes {rounds} times", log.len());
    Ok(())
}
